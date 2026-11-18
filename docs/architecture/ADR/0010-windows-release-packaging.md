# ADR-0010：Windows 发布打包、安装与 Hook sidecar 分发

- 日期：2026-11-14
- 状态：Accepted（Phase 7A）
- 关联：ADR-0001（Tauri 本地优先栈）、ADR-0007（Claude Code Hooks）、ADR-0009（备份恢复与数据保护）

## 背景

Phase 0–6 已形成可运行的 Tauri 2 开发构建，但分发链路缺失：

- `xushu-hook.exe` 只在开发时由 `cargo build -p xushu-hook` 生成，桌面程序从当前可执行文件同目录查找；没有安装包把它放到稳定位置。
- `tauri.conf.json` 只有最小 `bundle` 配置，没有 NSIS、WebView2 策略、发布者信息或外部二进制。
- 生产日志只写入 `eprintln!`，GUI 子系统下不可见，无法诊断安装版启动、数据库、sidecar、备份恢复失败。
- 用户数据目录（`%APPDATA%\dev.xushu.desktop`）与程序目录天然分离，但没有安装/升级/卸载验证。

Phase 7A 的目标是把开发构建变成可安装、可启动、可升级、可卸载且不丢失用户数据的 Windows x64 产品，同时不改变 Phase 3 的验收语义与 Rust 完成门禁。

## 决策

### 1. NSIS currentUser 安装，identifier 保持 `dev.xushu.desktop`

- 使用 Tauri 官方 NSIS bundler（`bundle.targets = ["nsis"]`），默认 `installMode = currentUser`，安装到 `%LOCALAPPDATA%\XUSHU`，不需要管理员权限。
- `productName = "XUSHU"`，窗口标题保持“序枢 XUSHU”，版本与 workspace/Cargo 版本一致（Phase 7A 为 `0.1.0`）。
- identifier 暂不改为新域名。现有开发与用户数据都位于 `%APPDATA%\dev.xushu.desktop`；在 V1 公开发布前若更换 identifier，必须同时提供数据迁移方案，不得静默丢弃本地数据。
- 提供 `SimpChinese` 与 `English`；不启用语言选择器，按系统语言回退到中文。

### 2. `xushu-hook` 使用 Tauri `externalBin`，启动时做版本探测

- `bundle.externalBin = ["binaries/xushu-hook"]`；`scripts/prepare-sidecar.mjs` 以 `cargo build --locked --release -p xushu-hook` 构建当前目标三元组，并把文件复制为 `binaries/xushu-hook-<target-triple>.exe`。
- Tauri 在安装目录把 sidecar 还原为 `xushu-hook.exe`，与主程序同目录；`resolve_hook_binary` 同时兼容带三元组后缀的布局。
- 主程序启动时只执行固定的 `xushu-hook --version`（参数数组、无 shell、`CREATE_NO_WINDOW`），要求输出严格等于 `xushu-hook <主程序版本>`。缺失、无法执行、非零退出或版本不一致时返回 `None`，Claude 接入状态显示 `receiver_unavailable`，并在有界日志记录 `sidecar_probe_failed` 错误码。
- 不引入 `tauri-plugin-shell` 或通用 Shell capability；前端仍不能执行任意程序。
- Hook 配置仍由用户在项目内显式预览、勾选并批准后写入；sidecar 打包成功不等于真实 Claude Code 联调通过。

### 3. 安装目录与用户数据严格分离

- 程序文件：NSIS 安装目录（currentUser 默认 `%LOCALAPPDATA%\XUSHU`）。
- 用户数据：`app_data_dir()` → `%APPDATA%\dev.xushu.desktop`，包含 `xushu.db`、WAL/SHM、`hook-inbox`、`hook-backups`、`exports`、备份/恢复工作区与 `logs/`。
- 安装器不写数据库；首次启动由 `AppState::initialize` 创建数据目录并执行迁移。
- 卸载默认不删除 `%APPDATA%` 数据。Tauri NSIS 卸载向导提供显式“删除应用数据”复选框，默认未勾选；静默 `/S` 卸载不删除数据。
- 卸载不修改项目 `.claude/settings.local.json`。残留 XUSHU Hooks 的处理方式写入 README：卸载前先在应用内卸载 Hooks，或人工删除带 `--xushu-managed` 的条目。
- 升级使用同一 NSIS 产品键原地更新，保留数据库；迁移仍由版本化迁移与 Schema 兼容检查控制，失败时不删除旧库。

### 4. 有界本地诊断日志

- 新增 `logging` 模块，日志写入 `%APPDATA%\dev.xushu.desktop\logs\xushu.log`，超过 2 MiB 在下次启动时轮转为 `xushu.log.1`。
- 只记录固定事件名、错误码与计数（如 `database_ready schema=7 applied=0`、`sidecar_probe_failed code=version_mismatch`）；不记录路径列表、SQL、恢复令牌、Salt、Hook 原文、Prompt、transcript 或工具输入输出。
- 启动写 `app_start version=…`；正常退出写 `app_exit`；下次启动若最后一行不是 `app_exit`，记录 `previous_run_unclean_exit`。panic 只记录源文件:行号。
- 日志初始化失败不阻断应用启动；`logging::record` 在未初始化时静默返回。

### 5. WebView2 策略

- `bundle.windows.webviewInstallMode = { type: "downloadBootstrapper", silent: true }`：安装时检测 WebView2，缺失则静默运行官方 bootstrapper（需要联网）。Windows 11 通常已内置 Evergreen Runtime。
- 不使用 `fixedRuntime`/`offlineInstaller`，避免安装包增加约 127–180 MB；离线安装需求在后续发布方案中单独评估。

### 6. 签名、更新与发布边界

- Phase 7A **未配置** Windows 代码签名证书，安装包为未签名构建；状态记录 `SIGNING_NOT_CONFIGURED`，不得称为可信发行版。
- 不启用 Tauri updater。可信签名、升级元数据、回滚与发布基础设施完成前，不提供看似可用的自动更新。
- 不发布 GitHub Release，不宣称 V1.0.0 正式发布；本阶段只交付可验证的 Windows 安装包与报告。

### 7. 隔离安装 E2E

- 安装版 E2E 使用独立构建配置：identifier `dev.xushu.e2e.phase7a`、`productName = "XUSHU E2E Phase 7A"`、`appDirectoriesOverride` 指向 `target/phase7a/installed-data`，并由启动器设置 `XUSHU_E2E_DATA_DIR`。现有 Rust 启动守卫要求两者一致。
- E2E 使用独立测试项目、独立 SQLite、独立 Hook Spool 与 Salt；不打开、迁移或写入 `%APPDATA%\dev.xushu.desktop` 的正常数据库，并在前后比较 DB/WAL/SHM 指纹。
- 生产 identifier 的安装包会使用真实用户数据目录，本机无法安全隔离，因此**不自动安装生产包**；真实生产 identifier 的安装验收需独立 Windows 环境或用户明确授权。
- E2E 通过 WebView2 远程调试调用真实 Tauri IPC，覆盖项目/任务/验收/验证/记忆/决策/恢复/备份/恢复/Hook 安装/sidecar 执行/卸载；不自动截图。

## 后果

### 正面

- 安装、启动、升级、卸载路径可重复验证；sidecar 不再依赖 `target/debug` 或开发目录。
- 用户数据与程序目录分离，静默卸载不会删除数据库；升级保留任务、记忆、决策与验收记录。
- sidecar 缺失/损坏/版本漂移可见，不会静默使用。
- 生产故障有固定位置的有界日志可诊断。

### 代价与残余风险

- 未签名安装包会触发 SmartScreen 警告；正式发布前必须配置 Authenticode 与时间戳服务。
- 卸载应用后项目中的 XUSHU Hooks 不会自动移除；必须文档化并由用户处理。
- 升级后 sidecar 摘要变化会使 Claude 接入显示 `needs_repair`，用户需重新安装 Hooks；这是有意的安全行为，不是自动修复。
- 当前只有 NSIS，不提供 MSI；企业部署需求延后。
- 日志不包含路径与详细堆栈，复杂现场诊断能力有限；这是隐私边界下的取舍。
- macOS 仍需独立处理 `.app`/DMG、签名、公证、sidecar 三元组与数据目录；本 ADR 只保证代码不新增无谓的 Windows 硬编码。

## 备选方案

| 方案 | 未采用原因 |
|---|---|
| 使用 `tauri-plugin-shell` 的 `sidecar()` | 需要新增插件与 capability，扩大前端可执行的命令面；`std::process::Command` + 固定参数已满足 |
| 手工把 `target/release/xushu-hook.exe` 复制进安装目录 | 不可复现、易带入 debug 依赖，且没有目标三元组校验 |
| 把 sidecar 放在应用数据目录并在安装时复制 | 安装器无法可靠写入用户数据目录；升级/卸载语义更复杂 |
| 安装时写数据库或创建示例项目 | 违反本地优先与“不伪造数据”原则 |
| 启用自动更新 | 没有签名与发布元数据，风险高于收益 |
| 将 identifier 改为 `app.xushu.desktop` | 会改变数据目录，需要迁移方案；Phase 7A 不扩大范围 |

## 后续（Phase 7B/7C）

1. 代码签名证书、时间戳、SmartScreen 声誉与发布校验和。
2. 自动更新元数据、签名验证与回滚设计（若 V1 需要）。
3. MSI/企业部署评估。
4. macOS arm64 构建、公证、sidecar 与数据目录实机验收。
5. 真实生产 identifier 的干净 Windows 环境安装验收。