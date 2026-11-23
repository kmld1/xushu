use std::ffi::OsStr;
use std::fs::File;
use std::io::{self, PipeReader};
use std::marker::PhantomData;
use std::mem::{size_of, size_of_val};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::path::Path;
use std::process::Command;
use std::ptr::{null, null_mut};
use std::thread;
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{
    DuplicateHandle, DUPLICATE_SAME_ACCESS, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows_sys::Win32::System::JobObjects::{
    CreateJobObjectW, IsProcessInJob, JobObjectBasicAccountingInformation,
    JobObjectExtendedLimitInformation, QueryInformationJobObject, SetInformationJobObject,
    TerminateJobObject, JOBOBJECT_BASIC_ACCOUNTING_INFORMATION,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};
use windows_sys::Win32::System::Threading::{
    CreateProcessW, DeleteProcThreadAttributeList, GetCurrentProcess, GetExitCodeProcess,
    InitializeProcThreadAttributeList, ResumeThread, TerminateProcess, UpdateProcThreadAttribute,
    WaitForSingleObject, CREATE_NO_WINDOW, CREATE_SUSPENDED, EXTENDED_STARTUPINFO_PRESENT,
    LPPROC_THREAD_ATTRIBUTE_LIST, PROCESS_INFORMATION, PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
    PROC_THREAD_ATTRIBUTE_JOB_LIST, PROC_THREAD_ATTRIBUTE_PARENT_PROCESS, STARTF_USESTDHANDLES,
    STARTUPINFOEXW,
};

pub(super) struct JobChild {
    job: OwnedHandle,
    process: OwnedHandle,
    pub stdout: Option<PipeReader>,
    pub stderr: Option<PipeReader>,
}

impl JobChild {
    pub(super) fn spawn(command: &Command) -> io::Result<Self> {
        Self::spawn_in_job(command, create_job()?)
    }

    fn spawn_in_job(command: &Command, job: OwnedHandle) -> io::Result<Self> {
        match spawn_process(command, job.as_raw_handle()) {
            Ok((process, stdout, stderr)) => Ok(Self {
                job,
                process,
                stdout: Some(stdout),
                stderr: Some(stderr),
            }),
            Err(spawn_error) => {
                // A failed CreateProcessW can still leave a process in Job
                // accounting while Windows tears it down. Confirm this private
                // job is empty before releasing it.
                terminate_and_wait(&job).map_err(|cleanup_error| {
                    io::Error::other(format!(
                        "Windows Job 启动失败: {spawn_error}; 无法确认清理: {cleanup_error}"
                    ))
                })?;
                Err(spawn_error)
            }
        }
    }

    pub(super) fn has_exited(&self) -> io::Result<bool> {
        self.wait_for_process(0)
    }

    fn wait_for_process(&self, milliseconds: u32) -> io::Result<bool> {
        match unsafe { WaitForSingleObject(self.process.as_raw_handle(), milliseconds) } {
            WAIT_OBJECT_0 => Ok(true),
            WAIT_TIMEOUT => Ok(false),
            _ => Err(io::Error::last_os_error()),
        }
    }

    pub(super) fn finish(&self) -> io::Result<Option<i32>> {
        terminate_and_wait(&self.job)?;
        // NT can decrement the Job count just before signalling the process.
        if !self.wait_for_process(250)? {
            return Err(io::Error::other("Windows Job 主进程退出尚未确认"));
        }
        let mut code = 0;
        if unsafe { GetExitCodeProcess(self.process.as_raw_handle(), &mut code) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Some(code as i32))
    }
}

fn spawn_process(
    command: &Command,
    job: HANDLE,
) -> io::Result<(OwnedHandle, PipeReader, PipeReader)> {
    // This runner inherits the application environment. Its callers supply
    // program, arguments and cwd only.
    if command.get_envs().len() != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "不支持验证环境变量覆盖",
        ));
    }
    let program = command.get_program();
    let resolved;
    let path = if Path::new(program).is_absolute() {
        program
    } else {
        resolved = program
            .to_str()
            .and_then(crate::verification::resolve_executable)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "无法定位验证程序"))?;
        resolved.as_os_str()
    };
    let application = wide_path(path)?;
    let cwd = command
        .get_current_dir()
        .map(|path| wide_path(path.as_os_str()))
        .transpose()?;
    let mut command_line = Vec::new();
    append_argument(&mut command_line, path)?;
    for argument in command.get_args() {
        command_line.push(b' ' as u16);
        append_argument(&mut command_line, argument)?;
    }
    command_line.push(0);

    let (stdout, stdout_write) = io::pipe()?;
    let (stderr, stderr_write) = io::pipe()?;
    let broker = spawn_handle_broker(job)?;
    let handles = [
        remote_inheritable_handle(&broker, &File::open("NUL")?)?,
        remote_inheritable_handle(&broker, &stdout_write)?,
        remote_inheritable_handle(&broker, &stderr_write)?,
    ];
    let parent = [broker.as_raw_handle()];
    let mut attributes = Attributes::new(2)?;
    attributes.add(PROC_THREAD_ATTRIBUTE_HANDLE_LIST, &handles)?;
    attributes.add(PROC_THREAD_ATTRIBUTE_PARENT_PROCESS, &parent)?;
    let mut startup = startup_info(&mut attributes);
    startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
    startup.StartupInfo.hStdInput = handles[0];
    startup.StartupInfo.hStdOutput = handles[1];
    startup.StartupInfo.hStdError = handles[2];
    let (process, thread) = create_suspended(
        &application,
        Some(&mut command_line),
        cwd.as_deref(),
        true,
        &startup,
    )?;
    let mut belongs_to_job = 0;
    let membership = unsafe { IsProcessInJob(process.as_raw_handle(), job, &mut belongs_to_job) };
    let membership_error = (membership == 0).then(io::Error::last_os_error);
    if belongs_to_job == 0 {
        // The suspended process is ours, even if Windows did not attach it.
        stop_suspended_process(&process)?;
        return Err(
            membership_error.unwrap_or_else(|| io::Error::other("验证程序未继承 Windows Job"))
        );
    }
    // No process in the application ever holds the inheritable pipe handles.
    // Closing the suspended broker releases its copies before the command runs.
    stop_suspended_process(&broker)?;
    if unsafe { ResumeThread(thread.as_raw_handle()) } != 1 {
        return Err(io::Error::last_os_error());
    }
    Ok((process, stdout, stderr))
}

fn spawn_handle_broker(job: HANDLE) -> io::Result<OwnedHandle> {
    let executable = std::env::current_exe()?;
    let application = wide_path(executable.as_os_str())?;
    let jobs = [job];
    let mut attributes = Attributes::new(1)?;
    attributes.add(PROC_THREAD_ATTRIBUTE_JOB_LIST, &jobs)?;
    let startup = startup_info(&mut attributes);
    // Windows attaches the broker to the Job at creation. Its initial thread
    // is never resumed and there is no fallback for failed Job attachment.
    let (broker, _thread) = create_suspended(&application, None, None, false, &startup)?;
    Ok(broker)
}

fn startup_info(attributes: &mut Attributes<'_>) -> STARTUPINFOEXW {
    let mut startup = STARTUPINFOEXW::default();
    startup.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
    startup.lpAttributeList = attributes.as_ptr();
    startup
}

fn create_suspended(
    application: &[u16],
    command_line: Option<&mut [u16]>,
    cwd: Option<&[u16]>,
    inherit_handles: bool,
    startup: &STARTUPINFOEXW,
) -> io::Result<(OwnedHandle, OwnedHandle)> {
    let mut information = PROCESS_INFORMATION::default();
    if unsafe {
        CreateProcessW(
            application.as_ptr(),
            command_line.map_or(null_mut(), |value| value.as_mut_ptr()),
            null(),
            null(),
            i32::from(inherit_handles),
            CREATE_NO_WINDOW | CREATE_SUSPENDED | EXTENDED_STARTUPINFO_PRESENT,
            null(),
            cwd.map_or(null(), |path| path.as_ptr()),
            &startup.StartupInfo,
            &mut information,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    // On success both handles are newly owned by the caller.
    let process = unsafe { OwnedHandle::from_raw_handle(information.hProcess) };
    let initial_thread = unsafe { OwnedHandle::from_raw_handle(information.hThread) };
    Ok((process, initial_thread))
}

fn stop_suspended_process(process: &OwnedHandle) -> io::Result<()> {
    let handle = process.as_raw_handle();
    if unsafe { TerminateProcess(handle, 1) } == 0 {
        return Err(io::Error::last_os_error());
    }
    match unsafe { WaitForSingleObject(handle, 1000) } {
        WAIT_OBJECT_0 => Ok(()),
        WAIT_TIMEOUT => Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "Windows 启动辅助进程退出超时",
        )),
        _ => Err(io::Error::last_os_error()),
    }
}

fn terminate_and_wait(job: &OwnedHandle) -> io::Result<()> {
    if active_processes(job)? == 0 {
        return Ok(());
    }
    if unsafe { TerminateJobObject(job.as_raw_handle(), 1) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let start = Instant::now();
    while active_processes(job)? != 0 {
        if start.elapsed() >= Duration::from_secs(3) {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "Windows Job 仍有进程未退出",
            ));
        }
        thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}

fn active_processes(job: &OwnedHandle) -> io::Result<u32> {
    let mut info = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();
    if unsafe {
        QueryInformationJobObject(
            job.as_raw_handle(),
            JobObjectBasicAccountingInformation,
            (&mut info as *mut JOBOBJECT_BASIC_ACCOUNTING_INFORMATION).cast(),
            size_of_val(&info) as u32,
            null_mut(),
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(info.ActiveProcesses)
}

fn create_job() -> io::Result<OwnedHandle> {
    let handle = unsafe { CreateJobObjectW(null(), null()) };
    if handle.is_null() {
        return Err(io::Error::last_os_error());
    }
    let job = unsafe { OwnedHandle::from_raw_handle(handle) };
    let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    if unsafe {
        SetInformationJobObject(
            job.as_raw_handle(),
            JobObjectExtendedLimitInformation,
            (&info as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
            size_of_val(&info) as u32,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(job)
}

fn remote_inheritable_handle(
    broker: &OwnedHandle,
    source: &impl AsRawHandle,
) -> io::Result<HANDLE> {
    let process = unsafe { GetCurrentProcess() };
    let mut handle = null_mut();
    if unsafe {
        DuplicateHandle(
            process,
            source.as_raw_handle(),
            broker.as_raw_handle(),
            &mut handle,
            0,
            1,
            DUPLICATE_SAME_ACCESS,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    // The returned handle belongs to the broker process, never to this one.
    Ok(handle)
}

struct Attributes<'a>(Vec<usize>, PhantomData<&'a [HANDLE]>);

impl<'a> Attributes<'a> {
    fn new(count: u32) -> io::Result<Self> {
        let mut bytes = 0;
        unsafe { InitializeProcThreadAttributeList(null_mut(), count, 0, &mut bytes) };
        if bytes == 0 {
            return Err(io::Error::last_os_error());
        }
        let mut storage = vec![0_usize; bytes.div_ceil(size_of::<usize>())];
        if unsafe {
            InitializeProcThreadAttributeList(storage.as_mut_ptr().cast(), count, 0, &mut bytes)
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(storage, PhantomData))
    }

    fn as_ptr(&mut self) -> LPPROC_THREAD_ATTRIBUTE_LIST {
        self.0.as_mut_ptr().cast()
    }

    fn add(&mut self, key: u32, handles: &'a [HANDLE]) -> io::Result<()> {
        if unsafe {
            UpdateProcThreadAttribute(
                self.as_ptr(),
                0,
                key as usize,
                handles.as_ptr().cast(),
                size_of_val(handles),
                null_mut(),
                null_mut(),
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

impl Drop for Attributes<'_> {
    fn drop(&mut self) {
        unsafe { DeleteProcThreadAttributeList(self.as_ptr()) };
    }
}

fn wide_path(path: &OsStr) -> io::Result<Vec<u16>> {
    let mut wide: Vec<_> = path.encode_wide().collect();
    if wide.contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "路径包含空字符",
        ));
    }
    wide.push(0);
    Ok(wide)
}

fn append_argument(line: &mut Vec<u16>, argument: &OsStr) -> io::Result<()> {
    const QUOTE: u16 = b'"' as u16;
    const SLASH: u16 = b'\\' as u16;
    // CRT argv rules: a quote or the closing delimiter needs escaped slashes.
    line.push(QUOTE);
    let mut slashes = 0;
    for character in argument.encode_wide() {
        if character == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "参数包含空字符",
            ));
        }
        if character == SLASH {
            slashes += 1;
            continue;
        }
        if character == QUOTE {
            slashes = slashes * 2 + 1;
        }
        line.extend(std::iter::repeat_n(SLASH, slashes));
        line.push(character);
        slashes = 0;
    }
    line.extend(std::iter::repeat_n(SLASH, slashes * 2));
    line.push(QUOTE);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    const JOB_OBJECT_QUERY: u32 = 0x0004;

    #[test]
    fn inheriting_process_in_the_application_cannot_hold_broker_pipes() {
        use std::process::Stdio;
        let job = create_job().unwrap();
        let broker = spawn_handle_broker(job.as_raw_handle()).unwrap();
        let (mut reader, writer) = io::pipe().unwrap();
        remote_inheritable_handle(&broker, &writer).unwrap();
        drop(writer);
        let mut unrelated = Command::new("node")
            .args(["-e", "setTimeout(() => {}, 10000)"])
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        stop_suspended_process(&broker).unwrap();
        let read = super::super::read_pipe(&mut reader, &mut [0; 1]);
        let alive = unrelated.try_wait().unwrap().is_none();
        unrelated.kill().unwrap();
        unrelated.wait().unwrap();
        assert_eq!(read.unwrap(), 0, "unrelated process inherited the pipe");
        assert!(
            alive,
            "test must observe EOF while the other process is alive"
        );
    }

    #[test]
    fn rejected_job_assignment_never_runs_the_program() {
        let fixture = tempfile::tempdir().unwrap();
        let marker = fixture.path().join("should-not-exist.txt");
        let mut occupy = Command::new("node");
        occupy.args(["-e", "setTimeout(() => {}, 10000)"]);
        let job = create_job().unwrap();
        let unrelated = JobChild::spawn(&occupy).unwrap();
        assert!(!unrelated.has_exited().unwrap());

        let process = unsafe { GetCurrentProcess() };
        let mut query_only_job = null_mut();
        assert_ne!(
            unsafe {
                DuplicateHandle(
                    process,
                    job.as_raw_handle(),
                    process,
                    &mut query_only_job,
                    JOB_OBJECT_QUERY,
                    0,
                    0,
                )
            },
            0
        );
        let query_only_job = unsafe { OwnedHandle::from_raw_handle(query_only_job) };
        let mut rejected = Command::new("node");
        rejected
            .args(["-e", "require('fs').writeFileSync(process.argv[1], 'ran')"])
            .arg(&marker);
        let error = match spawn_process(&rejected, query_only_job.as_raw_handle()) {
            Ok(_) => panic!("Job handle without assignment permission accepted the process"),
            Err(error) => error,
        };
        assert!(error.raw_os_error().is_some(), "{error}");
        terminate_and_wait(&job).unwrap();
        assert_eq!(active_processes(&job).unwrap(), 0);
        assert!(!unrelated.has_exited().unwrap());
        unrelated.finish().unwrap();
        assert!(!marker.exists(), "program ran despite rejection: {error}");
    }

    #[test]
    fn native_argv_preserves_empty_unicode_quotes_and_trailing_slashes() {
        let directory = tempfile::tempdir().unwrap();
        let arguments = [
            "",
            "目录 文档",
            "\"inside\"",
            "end\\",
            "before\\\"quote",
            "a\tb",
        ];
        let mut command = Command::new("node");
        command
            .current_dir(directory.path())
            .args([
                "-e",
                "console.log(JSON.stringify(process.argv.slice(1)))",
                "--",
            ])
            .args(arguments);
        let result = super::super::execute_command(
            command,
            Duration::from_secs(5),
            4096,
            &super::super::RunControl::new(),
        );
        assert_eq!(
            result.execution_status,
            super::super::ExecutionStatus::Completed,
            "{result:?}"
        );
        let actual: Vec<String> = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(actual, arguments);
    }

    #[test]
    fn windows_job_failure_is_not_completed() {
        let result = super::super::execute_command(
            Command::new("xushu-nonexistent-test-executable"),
            Duration::from_secs(5),
            4096,
            &super::super::RunControl::new(),
        );
        assert_eq!(
            result.execution_status,
            super::super::ExecutionStatus::StartFailed
        );
        assert_eq!(result.exit_code, None);
    }

    #[test]
    fn windows_job_is_reaped_after_the_runner_process_is_killed() {
        use std::fs;
        use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_SYNCHRONIZE};
        const FIXTURE: &str = "XUSHU_WINDOWS_JOB_RUNNER_EXIT_FIXTURE";
        if let Some(marker) = std::env::var_os(FIXTURE) {
            let mut command = Command::new("node");
            command
                .args(["-e", "const fs = require('node:fs'); const p = process.argv[1]; fs.writeFileSync(p, String(process.pid)); fs.writeFileSync(p + '.ready', '1'); setTimeout(() => {}, 15000);"])
                .arg(marker);
            let result = super::super::execute_command(
                command,
                Duration::from_secs(10),
                4096,
                &super::super::RunControl::new(),
            );
            panic!("fixture runner should have been terminated: {result:?}");
        }

        let directory = tempfile::tempdir().unwrap();
        let marker = directory.path().join("pid");
        let mut runner = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "runner::win::tests::windows_job_is_reaped_after_the_runner_process_is_killed",
            ])
            .env(FIXTURE, &marker)
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(4);
        while !marker.with_extension("ready").is_file() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        let process = fs::read_to_string(&marker).ok().map(|pid| {
            let handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, pid.parse().unwrap()) };
            assert!(!handle.is_null());
            unsafe { OwnedHandle::from_raw_handle(handle) }
        });
        runner.kill().unwrap();
        assert!(!runner.wait().unwrap().success());
        let process = process.expect("fixture failed to publish its process handle");
        assert_eq!(
            unsafe { WaitForSingleObject(process.as_raw_handle(), 3000) },
            WAIT_OBJECT_0
        );
    }
}
