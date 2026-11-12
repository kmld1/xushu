# XUSHU Phase 4.1 独立验收

日期为 2026-11-09，时区为 Asia/Shanghai。整体验收 **INCOMPLETE**，人工视觉待复核，原 Claude Code 用户配置目录的运行状态变动单独记录于第 5 节。

| 验收项 | 状态 | 实测范围 |
| --- | --- | --- |
| `CODE_VERIFIED` | **PASS** | 10 项质量命令退出 0，前端 48 项、Rust 109 项通过 |
| `REAL_CLAUDE_HOOKS_E2E` | **PASS** | Windows 上 Claude Code 2.1.319，六种事件均由真实进程触发并经 Spool 导入 SQLite |
| `INSTALL_UNINSTALL_VERIFIED` | **PASS** | 一次性项目的授权安装、配置回读、修复、停用、卸载和重复卸载 |
| `PRIVACY_VERIFIED` | **PASS（限测试样本）** | XUSHU Spool、SQLite 及 WAL 的白名单、伪名和正文标记检查 |
| `VISUAL_REVIEW_PENDING` | **PENDING** | 无截图的桌面功能测试已执行，人工视觉复核待用户截图 |

## 1. 环境与验收矩阵

- Windows 11 企业版 build 26200，x64。Tauri 2.12.5 开发桌面、WebView2、SQLite 迁移 1–4，实机 DPI 144。Node 24.20.0、pnpm 12.16.1、Rust/Cargo 1.99.1。
- Claude Code 版本命令返回 2.1.319。原生认证状态命令退出 0 且返回已登录。真实模型回合、工具结果和正常退出用于验证模型可用性。最终复跑使用仅对子进程生效的一次性配置目录，并将现有用户设置文件作为 Claude Code 的只读启动输入。测试使用默认权限模式，仅允许指定文本读取和一次固定的失败命令，并关闭测试会话持久化。
- Git 测试根目录为 `D:\MyProject\xushu-hook-e2e-test-a6b35d18\项目 & Hooks`。隔离桌面标识为 `dev.xushu.phase41e2e.a6b35d18`。接收器使用本轮 release 构建的 `xushu-hook.exe`，放在同一个 D 盘测试目录内带空格、中文和 `&` 的路径。测试仓库由 `git init` 创建，包含一个普通文本文件和测试专用 Claude 配置。
- 核对官方六事件、`command + args`、`tool_use_id`、工具失败语义和 SessionEnd 预算。Windows 直接执行 `.exe` 的路径和参数行为由本机实测补充。 [Hooks 协议](https://code.claude.com/docs/en/hooks) · [设置作用域](https://code.claude.com/docs/en/settings) · [CLI 参考](https://code.claude.com/docs/en/cli-reference)

| 预先设定的门槛 | 核验证据 |
| --- | --- |
| 项目边界、默认停用、授权门禁、预览六项 | Git 根路径、Tauri IPC、WebView DOM、设置文件 |
| 六事件逐项触发、工具状态、任务门禁 | Claude Code 进程、实际 Spool、SQLite、活动面板 |
| 伪名、字段最小化、去重、时间及容量 | 实际 Spool/SQLite 查询、同一实际投递回放、边界回归 |
| 错误提示、停用无事件、卸载无损、再次启动 | 故障注入、实际桌面操作、设置结构比较、Claude Code 复跑 |
| 全量质量、原生窗口、人工视觉 | 退出码、原生窗口状态及 DOM、待用户截图 |

## 2. 授权安装与恢复

1. 隔离桌面数据库初始为空。经实际 Tauri IPC 导入一次性 Git 仓库，核对返回根路径后才安装。详情状态为 `not_configured`、`desiredEnabled=false`、0/6，已有两个第三方 Hook 组。
2. 安装 Dialog 显示 `.claude/settings.local.json` 和六条配置。五个可匹配事件使用 `*`，Stop 无 matcher，每项 5 秒。授权框未勾选时按钮禁用。直接发出 `approved=false` 的真实 IPC 请求得到 `APPROVAL_REQUIRED`，设置文件哈希与安装前相同。
3. 经 Dialog 勾选并确认后状态为 `installed`、6/6。独立解析落盘文件，逐一比较事件名、matcher、可执行文件、参数和 timeout，6 条均与预览相符。安装差异仅在 `hooks` 中新增六个 XUSHU 处理器。既有 `permissions`、测试 `env`、两个第三方组及其顺序保留。
4. 测试专用配置注入无效 JSON 后，桌面显示 `configuration_error`，阻止预览并提示原因。改回合法 JSON 后，测试专用 matcher 漂移及接收器摘要更新显示 `needs_repair`，用桌面修复预览和授权恢复为 6/6。matcher 与失败策略分别有回归测试。
5. 临时移开测试接收器时，桌面报告 `receiver_unavailable` 且阻止预览。真实 Claude Code 报告 SessionStart Hook 执行错误，进程仍完成模型回复并退出 0。随后将该测试接收器恢复到原位置。
6. 用桌面停用后，`desiredEnabled=false`、管理项 0，历史 18 条保留。新 Claude Code 进程正常回复并退出 0，实际新增 Spool 和事件均为 0。用桌面卸载后，再次卸载成功，文件字节保持停用时的内容，数据库卸载时间随操作刷新。安装前、停用后、卸载后 JSON 结构相等。安装前原始格式与应用重写后的格式可不同。
7. 卸载后独立启动 Claude Code，模型回复成功且退出 0，无新增 Spool 和事件。最终 Spool 待处理 0，三个测试配置备份保留。测试仓库的 `.claude` 目录保留。

## 3. 六事件与 SQLite 关联

下表取最终构建的 `enabled-final` 会话。Claude Code 进程于北京时间 01:00:48 启动，01:01:05 退出 0。模型完成三次交互，成功读出测试文件内容，Bash 的受控失败由工具结果标为错误，模型完成正常回复。表中时间为接收器捕获时间，时区为北京时间。

| Claude Code 事件 | 状态 | 真实证据 |
| --- | --- | --- |
| SessionStart | **PASS** | 01:00:49.478，CLI 返回成功，实际项目 Spool 518 B，SQLite 会话 `start_observed=1` |
| PreToolUse | **PASS** | 01:00:55.134 的 Read 及 01:01:01.997 的 Bash。用测试项目的第三方 Read Hook 提供 5 秒观察窗口，期间活动面板显示 Read 执行中，`startObserved=true`、`finishObserved=false` |
| PostToolUse | **PASS** | 01:01:00.277，Read 的开始和成功引用同一工具调用，类别 `file_read`，最终 `succeeded`，完成标志为真 |
| PostToolUseFailure | **PASS** | 01:01:02.841，Claude Code 实际运行 `exit 7` 后生成失败信封。开始和失败引用同一 Bash 调用，类别 `shell`、最终 `failed`、原因 `exit_code_nonzero` |
| Stop | **PASS** | 01:01:04.702，模型完成回复，桌面时间线显示“一轮回答结束”，任务仍为 `draft`、revision 1 |
| SessionEnd | **PASS** | 01:01:04.767，正常退出前写入，SQLite 写入 `ended_at`、`end_reason=other`，活动面板显示已结束 |

- 最终会话共 7 个实际投递、1 个会话、2 条工具调用。两次完整模型回合及两次前置启动探针累计 4 个会话、18 条事件，按类型分布为 4 次开始、4 次工具开始、2 次工具成功、2 次工具失败、2 次 Stop、4 次会话结束。所有事件属于同一个隔离项目，外键检查无异常。
- 接收时间先于导入时间。全样本导入延迟约 0.087–38.946 秒，包含观察 Spool 后手动刷新的等待。官方输入未提供统一的事件发生时间，产品使用接收器时间。
- 重放一份由 Claude Code 实际触发、此前已入库的原始最小信封，刷新返回 `duplicates=1`、`imported=0`。再次空刷新不增加记录，事件总数仍为 18。回放单列为去重验证。
- SQLite 中测试任务仍为 `draft`、revision 1，`verification_runs=0`。已结束会话和工具结果没有生成任务完成记录。Phase 1–3 原有验收引擎回归一并通过。

## 4. 缺陷与修复

| 严重度 | 可复现条件和实际结果 | 修复及验证 |
| --- | --- | --- |
| 高 | 首次在本机隔离 AppData 初始化，Windows 对加密目录内同卷重命名返回错误 17，桌面报 `IO_ERROR`。CLI 信封发布也失败 | 盐初始化与读取使用文件锁。Spool 常规路径保留重命名，错误 17 时独占创建并写完再释放。导入遇共享冲突则重试，对崩溃残留的无效信封仍做验证。两项 AppData 测试均由 101 转为 0，真实会话复测通过 |
| 高 | 999 个待处理文件时启动 12 个接收进程，实际增至 1002 条 | 按项目持锁执行计数和发布。并发回归达到 1000 条上限，超过上限的接收器仍安静退出 |
| 高 | 停用或卸载已写设置文件，SQLite 状态更新随后被测试触发器拒绝，数据库与设置不同步 | 仅在磁盘仍等于本次写入结果时恢复原设置，遇外部变动或恢复失败返回错误。停用及卸载故障注入回归均通过 |
| 中 | 外部更改已安装处理器的 matcher 或 `onFailure`，状态仍可能显示已安装 | 比较 matcher 与完整托管处理器。状态转为需要修复，重新授权修复后恢复 |
| 中 | 读取工具完成后，历史 PreToolUse 行显示后来产生的“成功”或“失败” | 历史开始行显示“已观察”。工具的实时状态仍在工具调用记录中单独更新。实际桌面复测与回归通过 |
| 中 | 停用后空刷新时，测试数据库中 5002 条含过期数据的事件未裁剪 | 启动及刷新处理已注册接入的保留策略。停用后空刷新回归裁剪至 5000 条并移除过期事件 |
| 中 | 合并或移除处理器会清掉原有的空第三方 matcher 组和无关空事件数组 | 仅移除确实变空的托管组或事件，保留未触及的空组和无关事件数组，回归通过 |
| 中 | 导入器原先会跟随被重定向的项目待导入目录及文件。读取前的大小检查与读操作也分离 | 导入前拒绝检测到的目录重定向和非普通信封，改为打开后有界读取。接收器检查盐和待处理路径，启动只清理超过 60 秒的临时文件，避免删除活跃接收器文件 |

上述功能回归中的构造数据、SQLite 触发器和文件故障注入仅用于边界测试。真实事件结果来自第 3 节的 Claude Code 进程和最小信封。

## 5. 安全、隐私与剩余边界

- 检查了 15 个来自真实进程的 Spool 样本，字段集合与白名单相同，会话哈希为 64 位十六进制。对样本、SQLite 与 WAL 检索测试 Prompt、回复、文件内容、工具参数、环境值、原始字段名和凭据字段标记，命中 0。对最终会话实际产生的原始会话与工具 ID 搜索命中 0。没有读取用户凭据或 transcript，测试仅在内存中提取结果类别和标记命中情况。
- 代码检查与测试覆盖 256 KiB stdin、16 KiB 信封、1000 条 Spool、30 天和 5000 条保留、项目根指纹、投递和语义去重、损坏设置、目录 Junction、丢失结束事件、成功与失败事件冲突及数据库事务。保留检查依赖应用运行且接入路径可读。
- 项目设置更新使用同目录临时文件及重命名，写入前重新比较设置内容。禁用和卸载使用已验证的项目 UUID、明确授权及产品 IPC。写入错误有设置备份和恢复路径。配置备份可能包含该项目原本保存的环境值，仅存应用数据区。Windows 的替代发布路径用于事件 Spool，项目设置仍要求文件系统支持安全替换。
- 桌面只有受控 IPC，前端没有通用文件系统写、Shell 或 SQL 权限。检测到的符号链接和重解析点会被拒绝。路径检查和最终打开之间仍存在本机同用户并发替换的窗口，本轮未进行对抗性竞争测试。强杀会话可能缺少 SessionEnd，状态保留为“结束未确认”。
- XUSHU 本轮只写测试仓库的 `.claude/settings.local.json`，没有直接写入 Claude Code 用户或系统设置。前置模型探针使用本机默认的 Claude Code 配置目录时，Claude Code 自身的 `.claude.json` 在正常退出时更新了元数据。最终复跑改用 D 盘子进程配置目录，原 `.claude.json`、用户 `settings.json` 和检查到的受管设置未再变化。原提示历史文件在两次隔离调用之间出现时间戳变化，与测试进程运行窗口不重合，来源未核实。只检查了文件元数据。

## 6. 质量命令与桌面验证

| 实际命令 | 退出码 | 结果 |
| --- | --- | --- |
| `pnpm typecheck` | 0 | 通过 |
| `pnpm lint` | 0 | 通过 |
| `pnpm test` | 0 | 8 个测试文件，48 项通过 |
| `pnpm build` | 0 | 通过，仍有 Vite 大于 500 kB 的 chunk 提示 |
| `pnpm format` | 0 | 通过 |
| `cargo fmt --all -- --check` | 0 | 通过 |
| `cargo check --workspace --all-targets --locked` | 0 | 通过 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | 通过 |
| `cargo test --workspace --locked` | 0 | 109 项通过，其中 Claude 核心集成 26 项、CLI 8 项、协议 5 项 |
| `cargo build -p xushu-hook --release` | 0 | 通过，真实 Hook 使用此 release 构建 |
| `cargo build -p xushu-desktop --locked` | 0 | 测试结束后重新生成常规标识的桌面开发构建 |
| 隔离 `pnpm tauri dev --no-watch --config …` | 0 | 真实窗口经关闭按钮退出，Vite 子进程在退出清理时报告 -1，复查监听端口为 0 |
| AppData 两项定向回归 | 0 / 0 | 同一隔离 AppData 环境中的首次初始化和信封发布 |
| Windows 原生回归脚本 | 0 | DPI 144、Snap 命中 `HTMAXBUTTON=9`，最小化、最大化和还原有效 |
| `git diff --check` | 0 | 已跟踪变更无空白错误 |

实际桌面验证了未配置、预览、授权安装、活动 Sheet、执行中、正常结束、工具失败、配置异常、修复、停用及卸载反馈。在 1366×768 和 500×480 逻辑外窗测得 WebView 视口分别为 1352×760 和 486×472，页面及主内容均无横向溢出，窗口控制组高度为 44 CSS px。本轮变更集中于 Rust 后端和测试，前端组件与窗口层没有改动，因此未启用 `frontend-design` Skill。

## 7. 未运行与人工视觉

- **Phase 3.6** 待复核深浅主题、工作带层级、焦点/失焦、长文本、实机鼠标拖动及双击、边缘缩放、系统 Snap 悬停菜单与半屏/三栏选择、多显示器和其他 DPI。
- **Phase 4** 待复核接入区块、六项预览、授权与错误提示、活动 Sheet 空态和长文本、工具失败与运行中状态、窄窗、Tab/Escape 和关闭后的焦点返回。参见 [UI 审计](../design/UI_AUDIT.md)。无自动截图或录屏。
- **NOT_RUN** 包括生产安装包与 sidecar 路径、其他 Claude Code 版本、macOS/Linux、正常运行中的强杀故障注入、真实五秒 Hook 超时实测。故障与权限分支的代码审查、现有自动化或官方协议覆盖范围已分别标注。

## 8. 工作区、证据与下一步

- 源码分支 `main`，HEAD `f3ff6a8`。开始时 `git status --short` 为 79 项，最终为 80 项，两条已暂存重命名保留。现有 Phase 3.5、3.6、4 改动保留。Phase 4.1 修改 Claude 核心、Hook CLI、协议及回归测试，更新接入说明并新增本报告。未提交或推送。
- D 盘测试仓库和相邻 `evidence` 目录保留。其中有验收矩阵、测试设置副本、脱敏进程摘要、最小信封、窗口测量与 SQLite Backup API 创建的一致性副本 `xushu.phase41.sqlite`。备份 `integrity_check=ok`，保留文件内的路径均为本地证据。
- 本轮创建的隔离 AppData 约 1.64 MiB，D 盘 WebView 缓存约 139 MiB，隔离 Claude 配置目录约 28 KiB。自动审批策略拒绝了对这三个本轮专用目录的递归清理命令，工具给出的原因为策略阻止，因此保留目录。复查没有活动桌面、接收器、测试 WebView 进程或测试监听端口，Spool 待导入为 0。未使用系统 Temp 作为一次性项目位置。
- 下一步由用户提供 Phase 3.6 和 Phase 4 的手动截图以复核视觉条目。发布包内接收器路径和其他平台兼容性另行验收。
