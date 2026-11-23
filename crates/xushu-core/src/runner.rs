use std::io::{self, Read};
use std::path::PathBuf;
use std::process::Command;
#[cfg(unix)]
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
#[cfg(unix)]
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

#[cfg(windows)]
mod win;

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

/// 运行控制句柄。Windows runner 在自己的线程中持有 Job 并轮询取消标志。
pub struct RunControl {
    cancelled: AtomicBool,
    #[cfg(unix)]
    group: Mutex<Option<rustix::process::Pid>>,
}

impl RunControl {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            cancelled: AtomicBool::new(false),
            #[cfg(unix)]
            group: Mutex::new(None),
        })
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        #[cfg(unix)]
        self.terminate_tree();
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
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
    command: Command,
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

    let started = Instant::now();
    #[cfg(unix)]
    let spawned = {
        use std::os::unix::process::CommandExt;
        let mut command = command;
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0);
        control.spawn_group(&mut command)
    };
    #[cfg(windows)]
    let spawned = win::JobChild::spawn(&command);
    let mut child = match spawned {
        Ok(child) => child,
        Err(error) => {
            #[cfg(unix)]
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
        #[cfg(unix)]
        let exited = has_exited(&mut child);
        #[cfg(windows)]
        let exited = child.has_exited();
        match exited {
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
    let cleanup = {
        if let Err(error) = control.finish_group(execution_status == ExecutionStatus::Completed) {
            execution_status = ExecutionStatus::Interrupted;
            note = Some(format!("无法清理验证进程组: {error}"));
        }
        if execution_status != ExecutionStatus::Completed {
            let _ = child.kill();
        }
        child.wait().map(|status| status.code())
    };
    #[cfg(windows)]
    let cleanup = child.finish();

    let exit_code = match cleanup {
        Ok(code) => code,
        Err(error) => {
            execution_status = ExecutionStatus::Interrupted;
            note = Some(format!("无法确认验证进程清理完成: {error}"));
            None
        }
    };

    drop(child);
    stop_capture.store(true, Ordering::Release);

    let stdout = stdout_handle
        .and_then(|handle| handle.join().ok())
        .unwrap_or_default();
    let stderr = stderr_handle
        .and_then(|handle| handle.join().ok())
        .unwrap_or_default();

    if (!stdout.eof || !stderr.eof)
        && (execution_status == ExecutionStatus::Completed
            || cfg!(windows) && execution_status != ExecutionStatus::Interrupted)
    {
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
    #[cfg(windows)]
    let drain_limit = Duration::from_millis(750);
    #[cfg(unix)]
    let drain_limit = Duration::from_millis(150);

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
            && drain_started.get_or_insert_with(Instant::now).elapsed() >= drain_limit
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
