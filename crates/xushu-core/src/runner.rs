use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
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
pub struct RunOutcome {
    pub execution_status: ExecutionStatus,
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
    pub stdout: String,
    pub stderr: String,
    pub stdout_bytes: u64,
    pub stderr_bytes: u64,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
    pub note: Option<String>,
}

impl RunOutcome {
    pub fn start_failed(message: impl Into<String>) -> Self {
        Self {
            execution_status: ExecutionStatus::StartFailed,
            exit_code: None,
            duration_ms: 0,
            stdout: String::new(),
            stderr: String::new(),
            stdout_bytes: 0,
            stderr_bytes: 0,
            stdout_truncated: false,
            stderr_truncated: false,
            note: Some(message.into()),
        }
    }
}

/// 运行控制句柄：取消标志 + Windows Job 句柄。
pub struct RunControl {
    cancelled: AtomicBool,
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

    #[cfg(not(windows))]
    fn terminate_tree(&self) {
        // 非 Windows 首版只终止主进程；进程组清理未在本阶段验证。
    }

    #[cfg(windows)]
    fn release_job(&self) {
        if let Ok(mut guard) = self.job.lock() {
            let job = guard.take();
            drop(guard);
            drop(job);
        }
    }

    #[cfg(not(windows))]
    fn release_job(&self) {}
}

/// 执行验证计划。返回前保证输出线程已结束、子进程树已终止。
pub fn execute(plan: &RunPlan, control: &Arc<RunControl>) -> RunOutcome {
    if control.is_cancelled() {
        return RunOutcome {
            execution_status: ExecutionStatus::Cancelled,
            exit_code: None,
            duration_ms: 0,
            stdout: String::new(),
            stderr: String::new(),
            stdout_bytes: 0,
            stderr_bytes: 0,
            stdout_truncated: false,
            stderr_truncated: false,
            note: Some("运行开始前已被取消".to_owned()),
        };
    }

    #[cfg(windows)]
    let job = win::create_job();
    #[cfg(windows)]
    if let Some(job) = job {
        control.attach_job(job);
    }

    let mut command = Command::new(&plan.executable);
    command
        .args(&plan.args)
        .current_dir(&plan.cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let started = Instant::now();
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            control.release_job();
            return RunOutcome {
                execution_status: ExecutionStatus::StartFailed,
                exit_code: None,
                duration_ms: started.elapsed().as_millis() as u64,
                stdout: String::new(),
                stderr: String::new(),
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

    let stdout_handle = stdout_pipe.map(|pipe| {
        let limit = plan.output_limit_bytes;
        thread::spawn(move || read_limited(pipe, limit))
    });
    let stderr_handle = stderr_pipe.map(|pipe| {
        let limit = plan.output_limit_bytes;
        thread::spawn(move || read_limited(pipe, limit))
    });

    let deadline = started + plan.timeout;
    let mut note = None;
    let mut execution_status = ExecutionStatus::Completed;

    loop {
        if control.is_cancelled() {
            execution_status = ExecutionStatus::Cancelled;
            note = Some("用户取消验证运行".to_owned());
            let _ = child.kill();
            break;
        }
        if Instant::now() >= deadline {
            execution_status = ExecutionStatus::TimedOut;
            note = Some(format!("验证运行超过 {} 秒上限", plan.timeout.as_secs()));
            let _ = child.kill();
            break;
        }
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => thread::sleep(Duration::from_millis(20)),
            Err(error) => {
                execution_status = ExecutionStatus::Interrupted;
                note = Some(format!("等待验证进程失败: {error}"));
                let _ = child.kill();
                break;
            }
        }
    }

    // 正常退出后也终止可能残留的子孙进程，并关闭继承的管道，避免读取线程
    // 被后台守护进程永久占用。非 Windows 首版只终止主进程。
    #[cfg(windows)]
    control.terminate_tree();

    let exit_code = match child.wait() {
        Ok(status) => status.code(),
        Err(_) => None,
    };

    let (stdout, stdout_bytes, stdout_truncated) = stdout_handle
        .and_then(|handle| handle.join().ok())
        .unwrap_or_default();
    let (stderr, stderr_bytes, stderr_truncated) = stderr_handle
        .and_then(|handle| handle.join().ok())
        .unwrap_or_default();

    #[cfg(windows)]
    control.release_job();

    RunOutcome {
        execution_status,
        exit_code,
        duration_ms: started.elapsed().as_millis() as u64,
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
        stdout_bytes,
        stderr_bytes,
        stdout_truncated,
        stderr_truncated,
        note,
    }
}

fn read_limited(mut pipe: impl Read, max_bytes: usize) -> (Vec<u8>, u64, bool) {
    let mut kept = Vec::new();
    let mut total = 0_u64;
    let mut buffer = [0_u8; 16 * 1024];

    loop {
        match pipe.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => {
                total += read as u64;
                if kept.len() < max_bytes {
                    let remaining = max_bytes - kept.len();
                    let take = remaining.min(read);
                    kept.extend_from_slice(&buffer[..take]);
                }
            }
            Err(_) => break,
        }
    }

    (kept, total, total > max_bytes as u64)
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
