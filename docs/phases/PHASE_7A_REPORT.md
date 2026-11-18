# Phase 7A 报告：Windows 正式产品化、安装与生命周期

- 日期：2026-11-14
- 项目：序枢 XUSHU
- 根目录：`D:\MyProject\xushu`
- 起始基线：`cd6e63b7b8b1e1214d69bbfe8421a2fdae767c75`（`main`，`chore(xushu): checkpoint phase 6 (validation pending)`）
- 结论：Windows x64 Release、NSIS 安装包、`xushu-hook` sidecar、有界日志与**隔离安装版 E2E** 已完成并通过；**生产 identifier 的实机安装、代码签名、真实 Claude Code 会话与人工视觉仍为 NOT_RUN/PENDING**。本报告不把历史 PENDING 改为 PASS，也不宣称 V1.0.0 已发布。

## 0. 状态判定

| 标签 | 状态 | 证据与范围 |
|---|---|---|
| `RELEASE_BUILD` | **PASS** | `pnpm release:windows` 退出码 0；`target/release/xushu-desktop.exe` ProductVersion/FileVersion = 0.1.0 |
| `WINDOWS_INSTALLER_BUILT` | **PASS** | Tauri NSIS；`XUSHU_0.1.0_x64-setup.exe`，3,303,953 B，SHA-256 `3bf2143f…` |
| `WINDOWS_INSTALLER_TESTED` | **PASS（隔离 identifier）** | `dev.xushu.e2e.phase7a` 测试安装包真实静默安装/启动/升级/卸载；生产 identifier 包未安装（`NOT_RUN`） |
| `WEBVIEW2_RUNTIME_VERIFIED` | **PASS（当前环境）** | WebView2 157.0.4341.57；安装包配置 `downloadBootstrapper`；缺失运行时的实际 bootstrapper 路径 `NOT_RUN` |
| `SIDECAR_BUNDLED` | **PASS** | 安装目录含 `xushu-hook.exe`（249,856 B），与主程序同目录；非 `target/debug` |
| `SIDECAR_RUNTIME_VERIFIED` | **PASS（隔离构造输入）** | 安装版 Hook 配置指向安装目录 sidecar；执行真实 Hook JSON → spool → 重启导入；真实 Claude Code 会话联调 `NOT_RUN` |
| `INSTALLATION_VERIFIED` | **PASS（隔离 identifier）** | 安装到含空格与中文的目录，首启创建 SQLite；生产 identifier `NOT_RUN` |
| `FIRST_LAUNCH_VERIFIED` | **PASS** | 安装版首次启动创建 `xushu.db`、`hook-inbox`、`EBWebView`、`logs` |
| `DATA_PERSISTENCE_VERIFIED` | **PASS** | 关闭重开后任务 `done`、记忆、决策、恢复快照、备份记录、Hook 事件均保留 |
| `UPGRADE_VERIFIED` | **PASS（0.1.0→0.1.1）** | 原地升级后版本 0.1.1；任务/记忆/决策/恢复/备份保留；sidecar 摘要变化显示 `needs_repair` |
| `UNINSTALL_DATA_SAFETY_VERIFIED` | **PASS（静默卸载）** | 程序目录与开始菜单快捷方式移除；数据、DB、项目、Claude 设置保留；GUI 复选框默认状态由生成脚本核对 |
| `BACKUP_RESTORE_REGRESSION` | **PASS** | 安装版完整备份 → 校验 → 修改 → 预检 → 损坏拒绝 → 正式恢复 → 最终校验 → 重开；升级后备份记录仍在 |
| `WINDOWS_NATIVE_WINDOW_VERIFIED` | **PASS（自动化范围）**；人工视觉另见 `VISUAL_REVIEW_PENDING` | 最大化/还原/最小化/关闭、Snap 命中 `HTMAXBUTTON`、500×480、DPI 144；实际 Snap 悬停菜单/拖拽/多显示器/150% 视觉未测 |
| `VISUAL_REVIEW_PENDING` | **PENDING** | 未自动截图/录屏；需用户手动签收 |
| `MACOS_PENDING` | **PENDING** | 未构建、未签名、未实机验证 |
| `GITHUB_RELEASE_PENDING` | **PENDING** | 未发布、未上传 |
| `SIGNING_NOT_CONFIGURED` | **明确记录** | 安装包与 EXE 均未签名（.NET 证书读取返回未签名） |
| `AUTO_UPDATE` | **NOT_RUN / 禁用** | 无可信签名与升级元数据；未启用 updater |
| `MSI` | **NOT_RUN** | 本阶段只交付 NSIS |
| `PRODUCTION_IDENTIFIER_INSTALL` | **NOT_RUN** | 会使用真实 `%APPDATA%\dev.xushu.desktop`；本机无法安全隔离，需独立 Windows 环境或用户明确授权 |

## 1. 生产构建架构审计与实施范围

审计发现：

- `tauri.conf.json` 只有最小 bundle 配置，`targets = "all"`，没有 NSIS、WebView2 策略、发布者信息或外部二进制。
- `xushu-hook.exe` 只在开发时由 `cargo build -p xushu-hook` 生成；`resolve_hook_binary` 只从当前可执行文件同目录查找，安装包没有把它放到稳定位置。
- 生产日志仅 `eprintln!`，GUI 子系统下不可见，无法诊断安装版启动、数据库、sidecar、备份恢复失败。
- 应用数据目录 `%APPDATA%\dev.xushu.desktop` 与程序目录天然分离，但没有安装/升级/卸载验证。
- 没有开发路径硬编码进入领域层；窗口 Snap 与部分原生文件操作使用 `#[cfg(windows)]` 隔离。
- 没有 Debug 专属运行逻辑会阻止 Release；`windows_subsystem = "windows"` 只在非 debug 生效。
- Tauri 2.12.5 / tauri-build 2.7.4 / @tauri-apps/cli 2.12.4 与当前工具链兼容；官方 externalBin、Windows installer 文档与本地 2.12.4 `config.schema.json` 已核对（2026-11-14）。Context7 需要认证、本轮未使用；未据旧版 API 猜测配置。

本轮实施：

1. `scripts/prepare-sidecar.mjs`：以 `cargo build --locked --release -p xushu-hook` 构建当前目标三元组，按 Tauri 要求复制为 `binaries/xushu-hook-<triple>.exe`。
2. `tauri.conf.json`：`externalBin`、NSIS currentUser、SimpChinese/English、WebView2 downloadBootstrapper、发布者/版权/分类/描述、图标。
3. `resolve_hook_binary`：启动时执行固定 `xushu-hook --version`，严格匹配主程序版本；缺失、损坏、版本不一致均返回接收器不可用。
4. `logging`：应用数据目录内有界日志、正常/非正常退出检测、panic 位置记录；只记录固定事件与错误码。
5. Release profile：`codegen-units=1`、`lto=true`、`opt-level="s"`、`strip=true`。
6. 隔离安装版 E2E 脚本：真实 NSIS 安装、WebView2 CDP 驱动真实 Tauri IPC、升级、sidecar 异常矩阵、窗口/Snap、卸载数据安全；不打开正常数据库。

## 2. Windows Release 架构

- 构建入口：`pnpm release:windows`（脚本定义）→ `@tauri-apps/desktop tauri build --bundles nsis`。本轮实际执行的是后者，退出码 0。
- `beforeBuildCommand`：`pnpm build && pnpm sidecar:prepare`；前端先完成 `tsc --noEmit` 与 Vite 生产构建，再构建 sidecar。
- Rust：workspace + `Cargo.lock`；桌面、Hook CLI、协议、领域 crate 全部 Release 编译。
- 不依赖 Vite Dev Server、`target/debug`、开发目录临时文件或用户安装 Node/Rust/pnpm。
- 安装包只包含：`xushu-desktop.exe`、`xushu-hook.exe`、`uninstall.exe`；不包含数据库、Salt、Spool、私钥、测试夹具或真实用户文件。
- 主程序约 10.9 MB；sidecar 249,856 B；NSIS 安装包 3.15 MiB（LZMA 默认）。

## 3. Bundle 配置与安装包

```json
{
  "targets": ["nsis"],
  "publisher": "XUSHU",
  "category": "DeveloperTool",
  "externalBin": ["binaries/xushu-hook"],
  "windows": {
    "webviewInstallMode": { "type": "downloadBootstrapper", "silent": true },
    "nsis": {
      "installMode": "currentUser",
      "languages": ["SimpChinese", "English"],
      "displayLanguageSelector": false,
      "installerIcon": "icons/icon.ico",
      "uninstallerIcon": "icons/icon.ico"
    }
  }
}
```

- 安装包格式：NSIS `-setup.exe`；默认安装到 `%LOCALAPPDATA%\XUSHU`，无需管理员权限。
- 开始菜单快捷方式：`XUSHU.lnk`（测试包为 `XUSHU E2E Phase 7A.lnk`）；E2E 已验证创建与卸载移除。
- 安装包/卸载器使用应用图标；安装/卸载向导支持简体中文与英文。
- MSI 未构建：MSI 需要 WiX/VBSCRIPT 且企业部署需求未确认；不为生成两种包牺牲稳定性。

## 4. Hook sidecar 打包与定位

- `bundle.externalBin = ["binaries/xushu-hook"]`；Tauri 在安装目录把 sidecar 还原为 `xushu-hook.exe`，与主程序同目录（生成脚本 `installer.nsi` 实际包含 `File /oname=xushu-hook.exe …`）。
- `resolve_hook_binary` 优先 `XUSHU_HOOK_BIN`（开发/测试显式覆盖），否则查找当前可执行文件同目录的 `xushu-hook.exe` 或带三元组后缀名称。
- 版本探测：只执行 `--version`，参数数组、无 shell、Windows `CREATE_NO_WINDOW`；输出必须严格为 `xushu-hook <主程序版本>`。
- 探测失败日志：`sidecar_probe_failed code=spawn_failed|nonzero_exit|version_mismatch`；显式 `XUSHU_HOOK_BIN` 指向不存在文件时记录 `missing`；自动查找未找到 sidecar 时直接返回不可用、不执行探测。UI 状态为 `receiver_unavailable`，不会静默使用。
- E2E 真实验证：
  - 安装目录路径含空格与中文；Hook 配置 command 指向安装目录 `xushu-hook.exe`；构造 PreToolUse JSON 执行成功，spool 写入；重启后导入 1 条事件。
  - 升级到 0.1.1 后事件计数从 1 增至 2；sidecar 摘要变化使状态为 `needs_repair`/`receiverChanged=true`，未自动修改项目配置。
  - sidecar 缺失、替换为文本、替换为版本 9.9.9 的假程序，三种情况均返回 `receiver_unavailable`，日志分别出现 `spawn_failed`/`version_mismatch`（缺失时不执行探测）。
- 未向前端授予通用 Shell；未新增 capability；Hooks 安装/停用/修复/卸载门禁保持不变。
- 真实 Claude Code 会话未启用（需用户明确授权的一次性项目）；构造输入测试不等同于真实联调。

## 5. 应用版本与标识符

- `productName = "XUSHU"`；窗口标题“序枢 XUSHU”；`version = 0.1.0`。
- identifier 保持 `dev.xushu.desktop`。原因：现有用户数据位于 `%APPDATA%\dev.xushu.desktop`；V1 公开发布前若更换 identifier，必须同时提供数据迁移方案，不得静默丢弃本地数据（见 ADR-0010）。
- 设置页“阶段”值由 Rust `get_app_info` 更新为 `Phase 7A`；未改变页面布局。
- 升级 E2E 临时使用 0.1.1 构建；完成后已恢复 workspace `Cargo.toml` / `Cargo.lock` 到 0.1.0，并重新通过 `cargo check --locked`。

## 6. WebView2 运行时

- 当前环境 WebView2 157.0.4341.57；安装版 E2E 实际启动 WebView2 并通过 CDP 调用 IPC。
- 安装包配置 `downloadBootstrapper`（silent）：安装时检测，缺失则运行官方 bootstrapper，需要联网。
- 未使用固定运行时（约 +180 MB）或离线安装器（约 +127 MB）。
- 真实“没有 WebView2 的干净系统”未测试：`NOT_RUN`；不通过卸载本机 WebView2 制造风险。

## 7. 用户数据生命周期

| 资产 | 位置 | 安装/升级/卸载行为 |
|---|---|---|
| 程序文件 | NSIS 安装目录（默认 `%LOCALAPPDATA%\XUSHU`） | 安装创建；升级原地替换；卸载删除 |
| 主数据库/WAL/SHM | `%APPDATA%\dev.xushu.desktop\xushu.db*` | 安装不写；首启创建；升级保留；卸载默认保留 |
| Hook Spool | `%APPDATA%\dev.xushu.desktop\hook-inbox\` | 不进入安装包；升级保留；卸载不删除 |
| 伪名化 Salt | `…\hook-inbox\.xushu-salt` | 不进入安装包/备份；升级保留；卸载不删除 |
| Hook 备份 | `…\hook-backups\` | 升级保留；卸载不删除 |
| 备份/恢复工作区 | `…\`（用户选择目标 + 应用内工作区） | 升级保留；卸载不删除 |
| 日志 | `…\logs\xushu.log` | 有界轮转；升级保留；卸载不删除 |
| 项目 `.claude/settings.local.json` | 用户项目目录 | 卸载器不修改；XUSHU Hooks 需用户显式清理 |

- 安装器不把数据库放入程序目录；程序目录可被升级/卸载替换。
- E2E 用独立数据目录 `target/phase7a/installed-data`，不打开正常数据库；正常 DB/WAL/SHM 指纹前后一致。
- 日志内容仅固定事件/错误码；E2E 扫描 `sk-`、`ghp_`、`AKIA`、私钥头均无命中。

## 8. 数据库升级与回退

- Schema 仍为 7；0.1.0→0.1.1 升级不新增迁移，启动执行幂等迁移检查（`applied=0`）。
- E2E 升级前数据：任务 `done`、2 条验收条件、2 次验证、2 条证据、记忆、已确认决策、恢复快照、备份记录、1 条 Claude 事件。
- 升级后：任务仍 `done`，记忆标题“安装版恢复记忆”，决策 `confirmed`，恢复快照与备份记录仍在；未伪造验证记录，未改变完成状态。
- 旧 Schema → 7 的迁移由 `cargo test` 的 persistence/backup 测试覆盖；生产安装版未构造旧 Schema 数据库，标记为测试范围限制。
- 迁移失败安全停止、升级中断/回退未在安装版 E2E 注入（`NOT_RUN`）；Rust 层迁移失败回滚与恢复回退由既有测试覆盖，不把历史结果冒充本轮结果。

## 9. 安装、升级与卸载

### 9.1 安装

- 真实 NSIS 静默安装到 `target\phase7a\install 目录\序枢 XUSHU`（含空格/中文），退出码 0。
- 安装目录包含 `xushu-desktop.exe`、`xushu-hook.exe`、`uninstall.exe`；无测试数据库/夹具。
- 首次启动创建隔离数据目录与 SQLite；未依赖开发服务器。

### 9.2 升级

- 0.1.0 → 0.1.1 原地 NSIS 升级，退出码 0；安装版 ProductVersion/FileVersion 变为 0.1.1。
- 数据保留；Hook 状态变为 `needs_repair`（`receiver_changed=true`），符合“sidecar 已更新，需要重新安装 Hooks”的安全语义；未自动启用/修复任何 Hooks。
- 升级后新 sidecar 仍可执行并写入 spool；重启导入第 2 条事件。
- 完整备份→预检→正式恢复回归在 0.1.0 安装版执行；升级后只验证备份记录、恢复历史与数据保留，未再次执行正式恢复。

### 9.3 卸载

- 静默卸载退出码 0；安装目录清空/移除；开始菜单快捷方式移除。
- 数据目录、`xushu.db`、测试项目、项目 `.claude/settings.local.json` 均保留；卸载前数据目录有 297 个条目，卸载后目录与主库仍存在。
- Tauri NSIS 卸载向导含“删除应用数据”复选框；生成脚本核对为默认未勾选，静默卸载跳过确认页且状态为 0。GUI 勾选删除数据的真实交互未测试（`NOT_RUN`）。
- 卸载不会清理项目中的 XUSHU Hooks；README/ADR/本报告均给出处理步骤：卸载前在应用内逐项目“卸载 Hooks”，或人工删除带 `--xushu-managed` 的条目并先备份。

## 10. 安全与隐私检查

- 前端仍只能调用固定 Tauri Commands；capability 未扩大；未启用 `withGlobalTauri`；未引入 shell/fs/sql 插件。
- sidecar 只执行固定 `--version` 参数，不执行 AI 文本、用户输入或 Hook 内容；Hook CLI 仍只写白名单元数据 spool。
- 日志不含 API Key、Token、私钥、transcript、完整 Prompt、工具输入输出、Salt 或原始恢复授权 token；错误只记录稳定码与计数。
- 安装包不含测试数据库、Salt、Spool、私钥或真实用户文件。
- E2E 使用隔离 identifier/数据目录；正常数据库指纹未变化。
- 未读取或输出任何真实密钥/凭据；新增文件凭据模式扫描无命中。
- 多实例：应用数据目录锁使第二个实例返回 `DATABASE_IN_USE`，不会两个进程同时写库；未引入进程级单实例插件或窗口重定向。Rust 测试 `second_app_state_cannot_open_the_same_database` 通过。
- 版本不兼容：数据库迁移/Schema 错误以稳定错误码记录（`database_migration_failed` / `database_initialization_failed`），不会静默降级；启动恢复日志失败记录 `restore_journal_recovery_failed`，不自动删除回退证据。
- 权限：安装为 currentUser，不请求管理员权限；日志与数据库位于当前用户 `%APPDATA%`，不写系统目录或注册表业务数据；未修改系统环境变量。
- 未签名安装包会触发 SmartScreen；已明确记录，不得称为可信发布版。

## 11. 实际构建产物与 SHA-256

| 产物 | 大小 | SHA-256 |
|---|---:|---|
| `target/phase7a/production/XUSHU_0.1.0_x64-setup.exe` | 3,303,953 B | `3bf2143fb7f5dd538ccfd554951e32547b2823c125395f634302f3593df2972c` |
| `target/phase7a/production/xushu-desktop.exe` | 10,931,200 B | `fb2d23ef49d12be5e3627065b76d25704ff8996fe647ef8d719081e1d99eec2f` |
| 隔离安装包 0.1.0 | 3,303,653 B | `e6f771e6cbfe71a8b11792c24b5e2d6ba827de38205a29d0153e6fca87f5a84b` |
| 隔离安装包 0.1.1 | 3,302,913 B | `f675e46557338f830f713d1436ea841d43aec7c9308b9ea1e5f7a13911beb16c` |
| 已安装 sidecar（0.1.1） | 249,856 B | `3b9b7491e1e5d36b31ea11be2a89cdac60403c28f317df814af92abd26ae18bc` |
| 工作区暂存 sidecar（0.1.0） | 249,856 B | `1f68f5b66865feb9324f4e91aa0cc9f1e989660595ff80baa2414d6e9cba01db` |

- 校验文件：`target/phase7a/production/SHA256SUMS.txt`、`target/phase7a/e2e/SHA256SUMS.txt`。
- 产物在 `.gitignore` 覆盖的 `target/` 下，不进入仓库。

## 12. 真实安装 E2E 结果

环境：Windows 11 Enterprise 10.0.26200 x64；DPI 144；显示器 1 台；WebView2 157.0.4341.57；Python 3.14.7 仅作测试编排；未自动截图/录屏。

隔离方式：

- 安装包 identifier `dev.xushu.e2e.phase7a`；`productName = "XUSHU E2E Phase 7A"`；`appDirectoriesOverride` + `XUSHU_E2E_DATA_DIR` 指向 `target/phase7a/installed-data`；现有启动守卫要求两者一致。E2E 编排脚本在安装后读取主程序内嵌 identifier，发现不是隔离构建即拒绝启动。
- 正常数据库 `%APPDATA%\dev.xushu.desktop\xushu.db` 在前后做 DB/WAL/SHM 指纹比较，均未变化。

命令与退出码：

```text
python scripts/phase7a-e2e.py --installer "target/phase7a/e2e/XUSHU E2E Phase 7A_0.1.0_x64-setup.exe" --upgrade-installer "target/phase7a/e2e/XUSHU E2E Phase 7A_0.1.1_x64-setup.exe"
```

退出码 **0**。实测结果：

1. 真实 NSIS 安装到含空格/中文目录；从已安装路径启动，不依赖 Vite。
2. 创建隔离 Git 项目、任务、自动/人工验收条件；推进到待验证。
3. 失败命令 `git rev-parse --verify refs/heads/phase7a-missing-branch`：exit 128，检查结果 `fail`；尝试 `done` 被拒绝（`TASK_NOT_COMPLETABLE`）。
4. 通过命令 `git rev-parse --git-dir`：exit 0，检查结果 `pass`；人工确认后 `done` 成功。
5. 创建项目记忆、决策并确认、恢复包。
6. 创建真实一致性备份并通过校验；修改记忆后预检 `ready=true`，正式恢复 `completed`、`switchVerified=true`、`finalReport.ok=true`；恢复后任务/记忆/决策/快照回到备份时点；恢复后再创建并校验一份备份。
7. 安装 Hooks（6/6）；关闭应用后执行安装版 sidecar（构造 PreToolUse JSON），重启导入 1 条事件；活动时间线 1 条。
8. 关闭重开：任务 `done`、记忆、决策、快照、备份记录、恢复历史、事件均保留；重开后 `get_database_health` 为 `ok=true`、`schemaVersion=7`、`journalMode=wal`，退出后的 WAL 可被正常恢复。
9. 强杀进程后重启：日志检测到 `previous_run_unclean_exit`；随后正常关闭记录 `app_exit`。
10. 原地升级 0.1.1：版本元数据 0.1.1；任务/记忆/决策/快照/备份保留；Hook 状态 `needs_repair`（`receiver_changed=true`）；升级后 sidecar 再写入 1 条事件，重开后事件计数 2。
11. 迁移到另一含空格/中文的目录后启动：sidecar 仍定位正确；窗口 500×480 测量工作带 44px、侧栏 52px、无横向溢出；最大化→还原→最小化→关闭通过；Snap Overlay `WM_NCHITTEST = HTMAXBUTTON (9)`。
12. sidecar 缺失/文本损坏/伪版本 9.9.9：三种情况均为 `receiver_unavailable`。
13. 卸载：开始菜单快捷方式与安装目录移除；数据、数据库、测试项目、Claude 设置保留；正常数据库指纹不变。

证据（已忽略目录）：

- `target/phase7a/installed-e2e-report.json`
- `target/phase7a/installed-full-result.json`
- `target/phase7a/installed-reopen-result.json`
- `target/phase7a/installed-upgrade-result.json`
- `target/phase7a/installed-upgrade-reopen-result.json`
- `target/phase7a/installed-window-result.json`
- `target/phase7a/installed-sidecar-unavailable-result.json`（三次运行共用文件名，最后一次为伪版本；三次状态见 `e2e-combined-run.log`）
- `target/phase7a/e2e-combined-run.log`
- `target/phase7a/installed-data/logs/xushu.log`（1,853 B，含正常/非正常退出与 sidecar 探测码，无敏感模式）

## 13. 自动化测试与退出码

| 命令 | 退出码 | 结果 |
|---|---|---|
| `pnpm install --frozen-lockfile` | 0 | 锁文件一致，供应链策略通过 |
| `pnpm typecheck` | 0 | `tsc --noEmit` |
| `pnpm lint` | 0 | ESLint 10，`--max-warnings 0` |
| `pnpm test` | 0 | 11 文件、**83** 项 |
| `pnpm build` | 0 | Vite；980.05 kB / gzip 280.77 kB；原有 >500 kB 提示 |
| `pnpm format` | 0 | Prettier 全部通过 |
| `cargo fmt --all -- --check` | 0 | 通过 |
| `cargo check --workspace --all-targets --locked` | 0 | 通过 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | 无警告 |
| `cargo test --workspace --locked` | 0 | **207** 项通过，0 失败，0 ignored |
| `pnpm --filter @xushu/desktop tauri build --bundles nsis`（等价 `pnpm release:windows`） | 0 | Release + NSIS |
| 安装版 E2E（含升级） | 0 | 见第 12 节 |
| `git diff --check` | 0 | 无空白错误 |

Rust 测试构成：core lib 33、backup 22、business 19、claude 37、decisions 8、memory 8、persistence 13、recovery 14、verification 26、desktop lib 14、hook CLI 8、hook protocol 5。

本轮新增测试：

- `state::tests::sidecar_version_probe_accepts_only_the_exact_packaged_version`
- `logging::tests::clean_exit_detection_ignores_blank_lines`
- `logging::tests::log_rotation_triggers_only_at_the_bound`
- `commands::tests::app_info_reports_the_current_productization_phase`

## 14. Windows 原生窗口与 Snap

- 安装版 1100×720：工作带 44px、侧栏 240px、无页面级横向溢出。
- 安装版 500×480：工作带 44px、侧栏 52px、无横向溢出；`data-tauri-drag-region` 存在。
- 真实安装版 IPC：最大化 → `isMaximized=true`；还原 → `false`；最小化 → 原生 `IsIconic=true`；关闭 → 进程退出码 0。
- Snap Overlay 原生子窗口类 `XushuSnapOverlay` 存在；`WM_NCHITTEST` 返回 `HTMAXBUTTON (9)`。
- 当前 DPI 144、显示器 1 台；未在 100%/150% 多显示器环境实测。
- 以下仍为 **VISUAL_REVIEW_PENDING**：实际 Snap 悬停菜单、标题栏拖拽手感、双击最大化、多显示器、其他 DPI、1366/1600/1920 与深浅主题人工截图。未自动截图/录屏。

## 15. macOS 兼容性风险

- NSIS、Windows 窗口 Snap 覆盖层、`windows-sys` 窗口/文件操作、currentUser 安装与卸载器均为 Windows 专属；macOS 需 `.app`/DMG、签名/公证、独立 sidecar 三元组与卸载语义。
- `resolve_hook_binary` 使用 `current_exe` 同目录，具备跨平台基础；但 macOS `.app` 内 sidecar 位置、Gatekeeper quarantine、universal/arm64 打包未验证。
- 数据目录、日志、数据库路径使用 Tauri 路径 API 与相对路径，未新增 `D:\` 硬编码；NSIS/E2E 脚本属于 Windows 测试工具，不进入运行时。
- 本阶段未构建 macOS，`MACOS_PENDING`；不得声称兼容已验证。

## 16. 未验证与延期项目

| 项目 | 状态 | 说明 |
|---|---|---|
| 生产 identifier 真实安装 | **NOT_RUN** | 会触碰真实数据目录；需独立 Windows 环境/账户或用户明确授权 |
| Windows 代码签名 | **NOT_RUN** | 无证书；`SIGNING_NOT_CONFIGURED` |
| 真实 Claude Code 会话联调 | **NOT_RUN** | 未自动启用真实项目 Hooks；需用户授权的一次性项目 |
| WebView2 缺失系统安装 | **NOT_RUN** | 未卸载本机运行时 |
| GUI 卸载勾选“删除应用数据” | **NOT_RUN** | 只核对生成脚本默认未勾选；未点击 GUI |
| 升级中断/回滚 | **NOT_RUN** | 未在安装器中途杀进程或制造磁盘满/权限拒绝 |
| 旧 Schema 数据库在安装版上升级 | **NOT_RUN** | Rust 迁移测试覆盖；安装版只测同 Schema 版本升级 |
| 多显示器/其他 DPI/150% 视觉 | **PENDING** | 需人工 |
| Snap 实际悬停菜单/拖拽 | **PENDING** | 需人工 |
| 屏幕阅读器/真实剪贴板 | **NOT_RUN** | 沿用历史未验证 |
| macOS / DMG / 公证 | **PENDING** | Phase 7B |
| 自动更新 / MSI / GitHub Release | **NOT_RUN** | 未启用/未构建/未发布 |

## 17. Git 工作区状态

- 分支：`main`；HEAD：`cd6e63b7b8b1e1214d69bbfe8421a2fdae767c75`（未变化）。
- 未提交、未推送、未 reset、未 clean、未进入 Phase 7B。
- 未跟踪的 `docs/phases/PHASE_4_REAL_HOOKS_REGRESSION.md` 原样保留，未修改、未提交。
- 本轮修改（已跟踪）：`.gitignore`、`CLAUDE.md`、`Cargo.toml`、`README.md`、`apps/desktop/package.json`、`apps/desktop/src-tauri/src/commands.rs`、`lib.rs`、`state.rs`、`tauri.conf.json`、`package.json`、`MASTER_DEVELOPMENT_PLAN.md`、`docs/architecture/CLAUDE_HOOKS.md`、`docs/architecture/DEPENDENCIES.md`、`docs/design/UI_AUDIT.md`。
- 本轮新增（未跟踪）：`apps/desktop/src-tauri/src/logging.rs`、`scripts/prepare-sidecar.mjs`、`scripts/phase7a-e2e.py`、`scripts/phase7a-installed-e2e.mjs`、`docs/architecture/ADR/0010-windows-release-packaging.md`、`docs/phases/PHASE_7A_REPORT.md`。
- `Cargo.lock` 曾临时改到 0.1.1 用于升级构建，已恢复并通过 `cargo check --locked`；`apps/desktop/src-tauri/binaries/` 被 `.gitignore` 排除。

## 18. 人工验收清单

1. 在独立 Windows 11 x64 环境或第二个用户账户安装**生产 identifier** `XUSHU_0.1.0_x64-setup.exe`，确认无需管理员权限、开始菜单入口、首次启动创建 `%APPDATA%\dev.xushu.desktop\xushu.db`。
2. 检查 Windows SmartScreen/签名状态，确认记录为未签名；正式发布前配置 Authenticode 与时间戳。
3. 在 GUI 卸载器中确认“删除应用数据”默认未勾选；先不勾选卸载，确认数据保留；再用一次性数据测试勾选删除。
4. 手动检查 Snap 悬停菜单、标题栏拖拽、双击最大化/还原、500×480/1366×768/1600×900/1920×1080、100%/150% 缩放、多显示器、深浅主题；由用户手动截图，不由 AI 自动截图。
5. 在用户明确授权的一次性项目中安装 Hooks，运行真实 Claude Code 会话，确认六类事件、去重、结束缺失提示；不要对正在使用的项目自动启用。
6. 在一次性数据目录中验证旧 Schema 备份恢复、升级中断、磁盘满/权限拒绝与回退提示。
7. 复核日志实际内容，确认无路径列表、Prompt、transcript、工具输出、Salt、Token。
8. 确认卸载后的残留 Hooks 说明可被普通用户理解，并实际清理一个测试项目。
9. 将本报告中的 PENDING/NOT_RUN 逐项更新；不得把自动化测量或隔离安装包写成生产实机 PASS。

## 19. 是否具备独立验收条件

- **代码、Release 构建、NSIS 打包、sidecar 打包/定位、隔离安装/重开/升级/卸载 E2E、备份恢复回归、窗口自动化**：具备独立复核条件，证据与哈希已记录。
- **生产 identifier 实机安装、签名、真实 Claude Code 会话、GUI 视觉/卸载勾选、多显示器/其他 DPI**：尚不具备本机安全执行条件，需用户提供独立环境或明确授权。
- 独立验收应重新执行第 13 节门禁，并重点复核第 16 节 NOT_RUN/PENDING 项。**当前不得判定 V1.0.0 正式发布。**

---

## 附：本轮主要命令

```text
pnpm install --frozen-lockfile
pnpm typecheck
pnpm lint
pnpm test
pnpm build
pnpm format
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
pnpm release:windows
python scripts/phase7a-e2e.py --installer <0.1.0 隔离包> --upgrade-installer <0.1.1 隔离包>
git diff --check
```