//! `xushu-hook`：Claude Code 项目级 Hook 的本地接收程序。
//!
//! 它只做三件事：
//! 1. 有界读取 stdin 中的 Hook JSON；
//! 2. 用 `xushu-hook-protocol` 白名单过滤出最小事件；
//! 3. 原子写入 XUSHU 本地 spool，等待桌面应用导入。
//!
//! 任何失败都必须安静地退出 0，绝不能阻塞或改变 Claude Code 的正常执行。

use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use xushu_hook_protocol::{
    filter_hook_input, is_link_or_reparse, read_salt, FilterContext, HookEnvelope,
    MAX_HOOK_INPUT_BYTES, MAX_PENDING_EVENTS_PER_PROJECT, MAX_SPOOL_FILE_BYTES,
};

const MANAGED_ARG: &str = "--xushu-managed";
const PROJECT_ROOT_ARG: &str = "--project-root";
const INBOX_ARG: &str = "--inbox";

struct Arguments {
    project_id: String,
    project_root: PathBuf,
    inbox_dir: PathBuf,
    capture_generation: i64,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.iter().any(|argument| argument == "--version") {
        println!("xushu-hook {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }

    // Hook 失败不能阻塞 Claude Code；不向 stdout/stderr 写入任何内容。
    let _ = run(&args);
    ExitCode::SUCCESS
}

fn run(args: &[String]) -> Option<()> {
    let arguments = parse_arguments(args)?;
    let salt = read_salt(&arguments.inbox_dir).ok()?;

    let mut input = Vec::with_capacity(8 * 1024);
    let stdin = std::io::stdin();
    stdin
        .lock()
        .take((MAX_HOOK_INPUT_BYTES + 1) as u64)
        .read_to_end(&mut input)
        .ok()?;
    if input.is_empty() || input.len() > MAX_HOOK_INPUT_BYTES {
        return None;
    }

    let context = FilterContext {
        project_id: &arguments.project_id,
        project_root: &arguments.project_root,
        salt: &salt,
    };
    let mut envelope = filter_hook_input(&input, &context).ok()?;
    envelope.capture_generation = arguments.capture_generation;
    write_spool_file(&arguments.inbox_dir, &envelope).ok()
}

fn parse_arguments(args: &[String]) -> Option<Arguments> {
    let mut project_id = None;
    let mut project_root = None;
    let mut inbox_dir = None;
    let mut capture_generation = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            MANAGED_ARG => {
                project_id = args.get(index + 1).cloned();
                index += 2;
            }
            PROJECT_ROOT_ARG => {
                project_root = args.get(index + 1).map(PathBuf::from);
                index += 2;
            }
            INBOX_ARG => {
                inbox_dir = args.get(index + 1).map(PathBuf::from);
                index += 2;
            }
            "--capture-generation" if capture_generation.is_none() => {
                let generation: i64 = args.get(index + 1)?.parse().ok()?;
                if generation <= 0 {
                    return None;
                }
                capture_generation = Some(generation);
                index += 2;
            }
            _ => return None,
        }
    }

    Some(Arguments {
        project_id: project_id?,
        project_root: project_root?,
        inbox_dir: inbox_dir?,
        capture_generation: capture_generation.unwrap_or_default(),
    })
}

fn write_spool_file(inbox_dir: &Path, envelope: &HookEnvelope) -> std::io::Result<()> {
    if is_link_or_reparse(inbox_dir) {
        return Ok(());
    }
    let pending_root = inbox_dir.join("pending");
    let project_dir = pending_root.join(&envelope.project_id);
    let temporary_dir = inbox_dir.join("tmp");

    if is_link_or_reparse(&pending_root)
        || is_link_or_reparse(&temporary_dir)
        || is_link_or_reparse(&project_dir)
    {
        return Ok(());
    }

    fs::create_dir_all(&project_dir)?;
    fs::create_dir_all(&temporary_dir)?;
    if is_link_or_reparse(&project_dir) || is_link_or_reparse(&temporary_dir) {
        return Ok(());
    }

    let lock_path = project_dir.join(".write.lock");
    if is_link_or_reparse(&lock_path) {
        return Ok(());
    }
    let mut lock_options = OpenOptions::new();
    lock_options.write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        lock_options.mode(0o600);
    }
    let capacity_lock = lock_options.open(lock_path)?;
    capacity_lock.lock()?;
    let pending_count = fs::read_dir(&project_dir)?
        .flatten()
        .filter(|entry| {
            Path::new(&entry.file_name())
                .extension()
                .is_some_and(|ext| ext == "json")
        })
        .take(MAX_PENDING_EVENTS_PER_PROJECT)
        .count();
    if pending_count >= MAX_PENDING_EVENTS_PER_PROJECT {
        return Ok(());
    }

    let serialized = serde_json::to_vec(envelope)?;
    if serialized.len() as u64 > MAX_SPOOL_FILE_BYTES {
        return Ok(());
    }

    let temporary_path = temporary_dir.join(format!("{}.tmp", envelope.delivery_id));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary_path)?;
    file.write_all(&serialized)?;
    file.flush()?;
    drop(file);

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let final_path = project_dir.join(format!("{timestamp:020}-{}.json", envelope.delivery_id));

    let result = fs::rename(&temporary_path, &final_path);
    if result.is_ok() {
        return result;
    }
    let _ = fs::remove_file(&temporary_path);
    #[cfg(windows)]
    if result
        .as_ref()
        .is_err_and(|error| error.kind() == std::io::ErrorKind::CrossesDevices)
    {
        return write_exclusive(&final_path, &serialized);
    }
    result
}

#[cfg(windows)]
fn write_exclusive(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::os::windows::fs::OpenOptionsExt;

    // Some Windows profiles reject even same-volume rename. The importer
    // retries a sharing violation and can only read once this handle closes.
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .share_mode(0)
        .open(path)?;
    let result = file.write_all(bytes).and_then(|()| file.sync_all());
    drop(file);
    if result.is_err() {
        let _ = fs::remove_file(path);
    }
    result
}
