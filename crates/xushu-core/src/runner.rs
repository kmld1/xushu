use std::io::{self, Read};
use std::path::PathBuf;
use std::process::{Command, Stdio};
#[cfg(windows)]
use std::sync::atomic::AtomicU32;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// 一次验证运行计划。程序与参数保持分离，不经过 Shell。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunPlan {
    pub executable: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub timeout: Duration,
    pub output_limit_bytes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionStatus {
    Completed,
    Cancelled,
    TimedOut,
    StartFailed,
    Interrupted,
}

impl ExecutionStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
            Self::TimedOut => "timed_out",
            Self::StartFailed => "start_failed",
            Self::Interrupted => "interrupted",
        }
    }
}

#[derive(Debug, Clone)]
pub struct RunOutcome<T = String> {
    pub execution_status: ExecutionStatus,
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
    pub stdout: T,
    pub stderr: T,
    pub stdout_bytes: u64,
    pub stderr_bytes: u64,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
    pub note: Option<String>,
}

impl<T: Default> RunOutcome<T> {
    pub fn start_failed(message: impl Into<String>) -> Self {
        Self {
            execution_status: ExecutionStatus::StartFailed,
            exit_code: None,
            duration_ms: 0,
            stdout: T::default(),
            stderr: T::default(),
            stdout_bytes: 0,
            stderr_bytes: 0,
            stdout_truncated: false,
            stderr_truncated: false,
            note: Some(message.into()),
        }
    }
}

/// 运行控制句柄：取消标志与本次运行的进程组或 Job。
pub struct RunControl {
    cancelled: AtomicBool,
    #[cfg(unix)]
    group: Mutex<Option<rustix::process::Pid>>,
    #[cfg(windows)]
    child_pid: AtomicU32,
    #[cfg(windows)]
    job_assigned: AtomicBool,
    #[cfg(windows)]
    job: Mutex<Option<win::JobHandle>>,
}

impl RunControl {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            cancelled: AtomicBool::new(false),
            #[cfg(unix)]
            group: Mutex::new(None),
            #[cfg(windows)]
            child_pid: AtomicU32::new(0),
            #[cfg(windows)]
            job_assigned: AtomicBool::new(false),
            #[cfg(windows)]
            job: Mutex::new(None),
        })
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        self.terminate_tree();
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    #[cfg(windows)]
    fn attach_job(&self, job: win::JobHandle) {
        let already_cancelled = self.is_cancelled();
        if let Ok(mut guard) = self.job.lock() {
            if already_cancelled {
                win::terminate(&job);
            } else {
                *guard = Some(job);
            }
        }
    }

    #[cfg(windows)]
    fn assign_process(&self, pid: u32) -> bool {
        self.child_pid.store(pid, Ordering::SeqCst);
        let guard = match self.job.lock() {
            Ok(guard) => guard,
            Err(_) => return false,
        };
        match guard.as_ref() {
            Some(job) => {
                let assigned = win::assign(job, pid);
                if assigned {
                    self.job_assigned.store(true, Ordering::SeqCst);
                }
                if self.is_cancelled() {
                    win::terminate(job);
                    if !assigned {
                        win::kill_tree_fallback(pid);
                    }
                }
                assigned
            }
            None => false,
        }
    }

    #[cfg(windows)]
    fn terminate_tree(&self) {
        if let Ok(guard) = self.job.lock() {
            if let Some(job) = guard.as_ref() {
                win::terminate(job);
            }
        }
        // Job Object 已接管进程树时由 TerminateJobObject 清理；只有 Job 分配
        // 失败或未分配时才回退到 taskkill，避免对已退出 PID 做多余操作。
        if !self.job_assigned.load(Ordering::SeqCst) {
            let pid = self.child_pid.load(Ordering::SeqCst);
            if pid != 0 {
                win::kill_tree_fallback(pid);
            }
        }
    }

    #[cfg(unix)]
    fn terminate_tree(&self) {
        let mut group = self.group.lock().unwrap();
        if group.is_some_and(|pid| terminate_group(pid).is_ok()) {
            group.take();
        }
    }

    #[cfg(unix)]
    fn spawn_group(&self, command: &mut Command) -> io::Result<std::process::Child> {
        let mut group = self.group.lock().unwrap();
        if self.is_cancelled() {
            return Err(io::Error::from(io::ErrorKind::Interrupted));
        }
        let child = command.spawn()?;
        *group = Some(rustix::process::Pid::from_child(&child));
        Ok(child)
    }

    #[cfg(unix)]
    fn finish_group(&self, leader_exited: bool) -> io::Result<()> {
        // Signal before reaping the leader, while its PID still identifies this group.
        // Remove the group under the same lock used by cancel, so a late cancel cannot signal it.
        let mut group = self.group.lock().unwrap();
        match group.take().map_or(Ok(()), terminate_group) {
            // Darwin skips zombies in killpg and reports EPERM if no process was
            // signalled. The leader's exit was observed without releasing its PID.
            Err(error)
                if cfg!(target_os = "macos")
                    && leader_exited
                    && error.kind() == io::ErrorKind::PermissionDenied =>
            {
                Ok(())
            }
            result => result,
        }
    }

    #[cfg(windows)]
    fn release_job(&self) {
        if let Ok(mut guard) = self.job.lock() {
            self.child_pid.store(0, Ordering::SeqCst);
            let job = guard.take();
            drop(guard);
            drop(job);
        }
    }

    #[cfg(not(windows))]
    fn release_job(&self) {}
}

#[cfg(unix)]
fn terminate_group(group: rustix::process::Pid) -> io::Result<()> {
    use rustix::process::{kill_process_group, Signal};
    match kill_process_group(group, Signal::KILL) {
        Ok(()) | Err(rustix::io::Errno::SRCH) => Ok(()),
        Err(error) => Err(error.into()),
    }
}

#[cfg(unix)]
fn has_exited(child: &mut std::process::Child) -> io::Result<bool> {
    use rustix::process::{waitid, Pid, WaitId, WaitIdOptions};
    match waitid(
        WaitId::Pid(Pid::from_child(child)),
        WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT,
    ) {
        Ok(status) => Ok(status.is_some()),
        Err(rustix::io::Errno::INTR) => Ok(false),
        Err(error) => Err(error.into()),
    }
}

#[cfg(windows)]
fn has_exited(child: &mut std::process::Child) -> io::Result<bool> {
    child.try_wait().map(|status| status.is_some())
}

/// 执行验证计划，清理本次运行的进程组或 Job，并有界地排空输出。
pub fn execute(plan: &RunPlan, control: &Arc<RunControl>) -> RunOutcome {
    let mut command = Command::new(&plan.executable);
    command.args(&plan.args).current_dir(&plan.cwd);
    let result = execute_command(command, plan.timeout, plan.output_limit_bytes, control);
    RunOutcome {
        execution_status: result.execution_status,
        exit_code: result.exit_code,
        duration_ms: result.duration_ms,
        stdout: String::from_utf8_lossy(&result.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&result.stderr).into_owned(),
        stdout_bytes: result.stdout_bytes,
        stderr_bytes: result.stderr_bytes,
        stdout_truncated: result.stdout_truncated,
        stderr_truncated: result.stderr_truncated,
        note: result.note,
    }
}

pub(crate) fn execute_command(
    mut command: Command,
    timeout: Duration,
    output_limit_bytes: usize,
    control: &RunControl,
) -> RunOutcome<Vec<u8>> {
    fn cancelled_before_spawn() -> RunOutcome<Vec<u8>> {
        RunOutcome {
            execution_status: ExecutionStatus::Cancelled,
            exit_code: None,
            duration_ms: 0,
            stdout: Vec::new(),
            stderr: Vec::new(),
            stdout_bytes: 0,
            stderr_bytes: 0,
            stdout_truncated: false,
            stderr_truncated: false,
            note: Some("运行开始前已被取消".to_owned()),
        }
    }
    if control.is_cancelled() {
        return cancelled_before_spawn();
    }

    #[cfg(windows)]
    let job = win::create_job();
    #[cfg(windows)]
    if let Some(job) = job {
        control.attach_job(job);
    }

    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let started = Instant::now();
    #[cfg(unix)]
    let spawned = control.spawn_group(&mut command);
    #[cfg(windows)]
    let spawned = command.spawn();
    let mut child = match spawned {
        Ok(child) => child,
        Err(error) => {
            control.release_job();
            if control.is_cancelled() {
                return cancelled_before_spawn();
            }
            return RunOutcome {
                execution_status: ExecutionStatus::StartFailed,
                exit_code: None,
                duration_ms: started.elapsed().as_millis() as u64,
                stdout: Vec::new(),
                stderr: Vec::new(),
                stdout_bytes: 0,
                stderr_bytes: 0,
                stdout_truncated: false,
                stderr_truncated: false,
                note: Some(format!("无法启动验证程序: {error}")),
            };
        }
    };

    #[cfg(windows)]
    {
        let _ = control.assign_process(child.id());
    }

    let stdout_pipe = child.stdout.take();
    let stderr_pipe = child.stderr.take();
    let stop_capture = Arc::new(AtomicBool::new(false));

    let stdout_handle = stdout_pipe.map(|pipe| {
        let stop = stop_capture.clone();
        thread::spawn(move || read_limited(pipe, output_limit_bytes, &stop))
    });
    let stderr_handle = stderr_pipe.map(|pipe| {
        let stop = stop_capture.clone();
        thread::spawn(move || read_limited(pipe, output_limit_bytes, &stop))
    });

    let mut note = None;
    let mut execution_status = ExecutionStatus::Completed;

    loop {
        if control.is_cancelled() {
            execution_status = ExecutionStatus::Cancelled;
            note = Some("用户取消验证运行".to_owned());
            break;
        }
        if started.elapsed() >= timeout {
            execution_status = ExecutionStatus::TimedOut;
            note = Some(format!("验证运行超过 {} 秒上限", timeout.as_secs()));
            break;
        }
        match has_exited(&mut child) {
            Ok(true) => break,
            Ok(false) => thread::sleep(Duration::from_millis(20)),
            Err(error) => {
                execution_status = ExecutionStatus::Interrupted;
                note = Some(format!("等待验证进程失败: {error}"));
                break;
            }
        }
    }

    #[cfg(unix)]
    if let Err(error) = control.finish_group(execution_status == ExecutionStatus::Completed) {
        execution_status = ExecutionStatus::Interrupted;
        note = Some(format!("无法清理验证进程组: {error}"));
    }
    #[cfg(windows)]
    control.terminate_tree();
    if execution_status != ExecutionStatus::Completed {
        let _ = child.kill();
    }

    let exit_code = match child.wait() {
        Ok(status) => status.code(),
        Err(error) => {
            execution_status = ExecutionStatus::Interrupted;
            note = Some(format!("无法回收验证进程: {error}"));
            None
        }
    };

    #[cfg(windows)]
    control.release_job();
    stop_capture.store(true, Ordering::Release);

    let stdout = stdout_handle
        .and_then(|handle| handle.join().ok())
        .unwrap_or_default();
    let stderr = stderr_handle
        .and_then(|handle| handle.join().ok())
        .unwrap_or_default();

    if execution_status == ExecutionStatus::Completed && (!stdout.eof || !stderr.eof) {
        execution_status = ExecutionStatus::Interrupted;
        note = Some("验证输出管道未完整关闭".to_owned());
    }

    RunOutcome {
        execution_status,
        exit_code,
        duration_ms: started.elapsed().as_millis() as u64,
        stdout: stdout.kept,
        stderr: stderr.kept,
        stdout_bytes: stdout.total,
        stderr_bytes: stderr.total,
        stdout_truncated: stdout.total > output_limit_bytes as u64 || !stdout.eof,
        stderr_truncated: stderr.total > output_limit_bytes as u64 || !stderr.eof,
        note,
    }
}

#[derive(Default)]
struct CapturedOutput {
    kept: Vec<u8>,
    total: u64,
    eof: bool,
}

#[cfg(unix)]
use std::os::fd::AsFd as PipeHandle;
#[cfg(windows)]
use std::os::windows::io::AsRawHandle as PipeHandle;

fn read_limited(
    mut pipe: impl Read + PipeHandle,
    max_bytes: usize,
    stop: &AtomicBool,
) -> CapturedOutput {
    let mut output = CapturedOutput::default();
    let mut buffer = [0_u8; 16 * 1024];
    let mut drain_started = None;

    #[cfg(unix)]
    {
        use rustix::fs::{fcntl_getfl, fcntl_setfl, OFlags};
        if fcntl_getfl(&pipe)
            .and_then(|flags| fcntl_setfl(&pipe, flags | OFlags::NONBLOCK))
            .is_err()
        {
            return output;
        }
    }
    loop {
        if stop.load(Ordering::Acquire)
            && drain_started.get_or_insert_with(Instant::now).elapsed()
                >= Duration::from_millis(150)
        {
            break;
        }
        match read_pipe(&mut pipe, &mut buffer) {
            Ok(0) => {
                output.eof = true;
                break;
            }
            Ok(read) => {
                output.total += read as u64;
                let take = (max_bytes - output.kept.len()).min(read);
                output.kept.extend_from_slice(&buffer[..take]);
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(5));
            }
            Err(_) => break,
        }
    }

    output
}

#[cfg(unix)]
fn read_pipe(pipe: &mut impl Read, buffer: &mut [u8]) -> io::Result<usize> {
    pipe.read(buffer)
}

#[cfg(windows)]
fn read_pipe(pipe: &mut (impl Read + PipeHandle), buffer: &mut [u8]) -> io::Result<usize> {
    use windows_sys::Win32::{Foundation::ERROR_BROKEN_PIPE, System::Pipes::PeekNamedPipe};

    let mut available = 0;
    // This thread is the only reader. Bytes reported by PeekNamedPipe cannot be
    // consumed by another reader between the peek and the bounded read.
    let ok = unsafe {
        PeekNamedPipe(
            pipe.as_raw_handle(),
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            &mut available,
            std::ptr::null_mut(),
        )
    };
    if ok == 0 {
        let error = io::Error::last_os_error();
        return if error.raw_os_error() == Some(ERROR_BROKEN_PIPE as i32) {
            Ok(0)
        } else {
            Err(error)
        };
    }
    if available == 0 {
        return Err(io::Error::from(io::ErrorKind::WouldBlock));
    }
    let length = buffer.len().min(available as usize);
    pipe.read(&mut buffer[..length])
}

#[cfg(windows)]
mod win {
    use std::path::PathBuf;
    use std::process::{Command, Stdio};

    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE,
    };

    pub struct JobHandle(HANDLE);

    unsafe impl Send for JobHandle {}
    unsafe impl Sync for JobHandle {}

    impl Drop for JobHandle {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }

    pub fn create_job() -> Option<JobHandle> {
        let handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if handle.is_null() {
            return None;
        }

        let job = JobHandle(handle);
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

        let configured = unsafe {
            SetInformationJobObject(
                job.0,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const std::ffi::c_void,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };

        if configured == 0 {
            None
        } else {
            Some(job)
        }
    }

    pub fn assign(job: &JobHandle, pid: u32) -> bool {
        let process = unsafe { OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, 0, pid) };
        if process.is_null() {
            return false;
        }
        let assigned = unsafe { AssignProcessToJobObject(job.0, process) } != 0;
        unsafe {
            CloseHandle(process);
        }
        assigned
    }

    pub fn terminate(job: &JobHandle) {
        unsafe {
            TerminateJobObject(job.0, 1);
        }
    }

    /// Job Object 分配失败时的有界兜底：按 PID 树终止。
    ///
    /// 固定从 `%SystemRoot%\System32\taskkill.exe` 启动，避免 PATH 劫持。
    pub fn kill_tree_fallback(pid: u32) {
        let taskkill = std::env::var_os("SystemRoot")
            .map(|root| PathBuf::from(root).join("System32").join("taskkill.exe"))
            .filter(|path| path.is_file())
            .unwrap_or_else(|| PathBuf::from("taskkill"));

        let _ = Command::new(taskkill)
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}
