# Phase 4 报告：Claude Code Hooks 接入与本地事件观察

> 本文记录初次实现的验收状态。后续真实联调、隔离复跑和当前验收结论见 [Phase 4 收尾审计](PHASE_4_CLOSEOUT_AUDIT.md)。

- 日期：2026-11-09
- 项目：序枢 XUSHU
- 根目录：`D:\MyProject\xushu`
- 总体状态：**代码与技术检查 PASS；真实 Claude Code 联调 NOT_RUN；人工视觉验收 VISUAL_REVIEW_PENDING。**
- 结论：在用户明确授权后，XUSHU 可以安装项目级 Claude Code Hooks、接收白名单事件、离线导入 SQLite，并在项目详情展示会话与工具调用时间线。Hook 事件与任务完成门禁物理隔离；`Stop`、`PostToolUse` 成功均不等于验收通过。

## 1. 实际实现的功能

### 任务 A：项目级 Hooks 集成管理

- 每个项目独立接入状态：未配置、已停用、已启用、部分安装、需要修复、配置异常、接收器不可用、项目不可用、状态不一致。
- 三层状态分离：数据库期望状态、`.claude/settings.local.json` 实际安装状态、最近真实事件/导入状态。
- 安装预览：写入位置、六个 Hook、matcher、超时、接收命令、数据边界、备份说明、其他 Hook 组数量、预览 token。
- 用户必须勾选确认并点击“安装并启用”；后端再次要求 `approved: true`，且校验预览 token。
- 默认关闭；不写 `.claude/settings.json`、用户级或系统级 Claude Code 设置。
- 结构化合并保留 `permissions`、`env`、其他 `hooks` 与自定义字段；安装前备份原文件。
- 卸载只删除 `args[0] == "--xushu-managed"` 且 `args[1] == project_id` 的条目。
- 配置异常、JSON 损坏、符号链接/重解析点、外部修改、接收程序更新、二进制摘要变化均可检测并提供修复入口。

### 任务 B：真实 Hooks 事件采集

注册六个事件：`SessionStart`、`PreToolUse`、`PostToolUse`、`PostToolUseFailure`、`Stop`、`SessionEnd`。

- `Stop` 表示一轮回答结束，不表示任务完成。
- `PostToolUse` 成功不表示代码正确或验收通过。
- 只有 `PreToolUse` 时工具调用保持 `running`；`SessionEnd` 时仍未结束则变为 `unknown`、`finishObserved = false`。
- 若 `SessionEnd` 先于工具开始事件到达，工具调用按 `unknown` 记录，不伪造仍在运行或成功。
- 成功/失败相互矛盾时显示 `unknown + conflict`。
- 缺失 `SessionStart` 显示“开始未观察”；缺失 `SessionEnd` 显示“结束未确认”。

### 任务 C：最小化采集与隐私隔离

`xushu-hook` 只从官方 stdin JSON 读取白名单字段：

- 事件类型、CLI 接收时间；
- 带盐 SHA-256 的会话/工具调用/Prompt 标识；
- 工具名称与类别；
- 成功/失败/未知、错误类别、中断标志、耗时；
- 会话来源与结束原因。

明确不保存：Prompt/AI 回答正文、完整命令与参数、文件内容/路径、工具输入输出、错误正文、`last_assistant_message`、transcript、模型名、API Key/Token/环境变量。spool、SQLite、日志与 UI 均不出现原始 Hook JSON。

### 任务 D：可靠的本地事件接收

- 独立 `xushu-hook.exe`，Claude Code 以 `command` + `args` exec 形式启动，不经过 shell。
- stdin 上限 256 KiB；spool 单文件上限 16 KiB；每项目待导入上限 1000 个文件。
- 写入 `<app_data>/hook-inbox/pending/<project_id>/<timestamp>-<delivery>.json`，同目录临时文件 + rename。
- CLI 对无盐、无 inbox、无效 JSON、超限、项目不匹配、无参数等所有错误安静退出 0，不写 stdout/stderr。
- 未采用 localhost HTTP；不需要端口、认证或常驻服务。
- 桌面启动、刷新接入状态、打开活动 Sheet 时导入；XUSHU 关闭期间事件保留在 spool。
- 停用/卸载前循环导入待处理事件；仍有剩余时拒绝操作，避免静默丢弃。

### 任务 E：SQLite 数据模型

迁移 4：`crates/xushu-core/migrations/0004_claude_hooks.sql`。

新增表：

- `claude_integrations`：项目级期望状态、安装版本、二进制路径/摘要、最近导入/错误、卸载时间。
- `claude_sessions`：伪名化会话、开始/最后事件/结束、来源、结束原因、是否观察开始。
- `claude_tool_calls`：会话 + 工具哈希唯一，状态/失败类别/耗时/开始结束观察/冲突标志。
- `claude_events`：最小事件账本；`delivery_id` 唯一；工具事件语义去重键；无原始 payload。

约束与策略：

- UUID、UTC RFC3339、外键、CHECK、事务。
- 事件保留 30 天，每项目最多 5000 条；会话最多 500 个。
- 不引用 `tasks`、验收条件、证据或完成门禁；测试显式断言导入前后任务状态/修订不变。

### 任务 F：Tauri IPC 与前端

新增 7 个 IPC：

- `get_claude_integration`
- `get_claude_activity`
- `preview_claude_hooks`
- `install_claude_hooks`
- `disable_claude_hooks`
- `uninstall_claude_hooks`
- `refresh_claude_events`

前端：

- 项目详情新增“Claude Code 接入”strip；
- 安装预览 Dialog：写入位置、六个 Hook 表、命令 details、会采集/不会采集、警告、复选框授权；
- 活动 Sheet：最近会话、未结束/未确认工具调用、事件时间线、刷新；
- 状态使用图标 + 中文 + 颜色；不新增一级导航；不自动截图；
- 空状态、加载态、错误态、配置异常、需要修复、停用/卸载确认均已实现。

## 2. 新增与修改的重要文件

### 新增

- `crates/xushu-hook-protocol/`：白名单协议、字段校验、带盐哈希、信封。
- `crates/xushu-hook/`：独立 Hook CLI 与真实进程测试。
- `crates/xushu-core/src/claude/`：配置合并、spool 导入、会话/工具状态、活动查询。
- `crates/xushu-core/migrations/0004_claude_hooks.sql`
- `crates/xushu-core/tests/claude.rs`（19 项）
- `apps/desktop/src-tauri/src/claude_commands.rs`
- `apps/desktop/src/features/claude/`（API、queries、状态、安装 Dialog、活动 Sheet、接入 strip、测试）
- `docs/architecture/ADR/0007-claude-code-hooks-integration.md`
- `docs/architecture/CLAUDE_HOOKS.md`
- `docs/phases/PHASE_4_REPORT.md`

### 修改

- `Cargo.toml`：加入两个新 crate；`crates/xushu-core/Cargo.toml` 增加协议依赖；`apps/desktop/src-tauri/Cargo.toml` 增加测试 dev-dependency。
- `crates/xushu-core/src/lib.rs`、`error.rs`、`db/migrations.rs`。
- `apps/desktop/src-tauri/src/state.rs`、`error.rs`、`lib.rs`、`commands.rs`。
- `apps/desktop/src/lib/domain.ts`、`pages/project-detail-page.tsx`、`App.test.tsx`、`app/app-layout.tsx`。
- `MASTER_DEVELOPMENT_PLAN.md`、`CLAUDE.md`、`README.md`。
- `docs/architecture/DEPENDENCIES.md`、`docs/design/DESIGN_SYSTEM.md`、`docs/design/UI_AUDIT.md`。
- `crates/xushu-core/tests/persistence.rs`（schema version 4）。

未删除、未覆盖 Phase 3.5/3.6 未提交改动。

## 3. 数据库迁移编号与新增表

- 迁移编号：**4**
- 文件：`crates/xushu-core/migrations/0004_claude_hooks.sql`
- 新表：`claude_integrations`、`claude_sessions`、`claude_tool_calls`、`claude_events`
- 旧表与数据全部保留；迁移在事务中执行；重复执行幂等；schema 版本高于应用时拒绝降级。
- 迁移 1–3 未修改。

## 4. 新增 Tauri IPC

| 命令 | 作用 | 写入/授权 |
|---|---|---|
| `get_claude_integration` | 导入待处理事件并返回三层状态 | 导入可写事件表；不写项目设置 |
| `get_claude_activity` | 导入并返回会话/事件/工具调用 | 导入可写事件表 |
| `preview_claude_hooks` | 只读生成安装预览与 token | 只读 |
| `install_claude_hooks` | 安装/修复 | `approved=true` + 预览 token |
| `disable_claude_hooks` | 删除 XUSHU Hooks，保留记录/历史 | `approved=true` |
| `uninstall_claude_hooks` | 删除 Hooks 并标记卸载 | `approved=true` |
| `refresh_claude_events` | 手动导入 spool | 只写事件表 |

Capabilities 未新增权限；前端仍无通用 FS/Shell/SQL。IPC 不返回盐、原始 Hook JSON、transcript 或设置文件内容。

## 5. Hooks 适配器执行方式与权限边界

- 可执行文件：`xushu-hook.exe`；开发模式从桌面程序同目录查找，或使用 `XUSHU_HOOK_BIN`。
- Hook 形式：`type: command`，`command` + `args` exec 形式，不经过 shell；超时 5 秒。
- 入口：Claude Code 官方 `command` Hook stdin。
- 失败策略：所有异常安静退出 0，不阻塞 Claude Code，不改变工具调用/权限/结果。
- 数据路径：`<app_data>/hook-inbox/`；盐 `.xushu-salt`；备份 `hook-backups/`。
- 不监听端口、不暴露局域网、不读取 transcript、不扫描其他项目。
- 权限边界：只改用户选定项目的 `.claude/settings.local.json`；不写团队/用户/系统设置；不修改任务、验收或证据。
- 性能：release 构建 30 次调用平均 **14.46 ms**；总耗时 433.8 ms；每次均写入白名单信封；无原始敏感内容。

## 6. 启用、停用、卸载实际测试结果

自动化（`crates/xushu-core/tests/claude.rs`，19 项）：

- 无 `.claude` 目录：预览显示将新建；安装创建 `settings.local.json`。
- 已有合法 `settings.local.json`：`permissions`、`env`、`custom`、其他 `PreToolUse` Hook 组在安装/停用/卸载后保持语义一致。
- 重复安装：已精确安装时不重写文件（字节级比较）。
- 重复卸载：幂等成功。
- JSON 损坏/`hooks` 结构非法：预览受阻；安装拒绝；原文件字节不变。
- 预览 token 过期/错误：安装拒绝（CONFLICT）。
- 未批准安装：APPROVAL_REQUIRED，且不创建文件。
- 归档项目：预览受阻；安装拒绝。
- `.claude` 为普通文件：拒绝修改。
- `.claude` 为 Junction（Windows 重解析点）：拒绝修改；目标目录未写入。
- 外部删除部分 Hook：状态变为 `partial`；预览/修复后恢复 `installed`。
- 接收程序二进制更新：状态变为 `needs_repair`（`receiver_changed=true`）；重新安装后恢复。
- 接收程序缺失：状态 `receiver_unavailable`；预览受阻。
- 停用后导入：新事件 `ignored`，不进入事件表；待导入清空。
- 卸载后其他 Hooks 保持原样；备份目录至少有一份原文件备份。

桌面冒烟（未安装真实项目 Hooks）：

- 项目详情显示“Claude Code 接入”、状态与主操作。
- 点击“查看并安装”打开预览；六个 Hook 表可见；未勾选时“安装并启用”禁用。
- 未点击安装；测试项目 `.claude` 目录未创建；未修改任何 Claude Code 配置。

## 7. 前端与 Rust 测试数量

| 层 | 测试文件/组 | 通过数 |
|---|---|---|
| 前端 Vitest | 8 个测试文件 | **48** |
| Rust `xushu-core` lib | 状态/模型等 | 11 |
| Rust 业务集成 | `tests/business.rs` | 19 |
| Rust Claude 集成 | `tests/claude.rs` | 19 |
| Rust 持久化 | `tests/persistence.rs` | 12 |
| Rust 验证引擎 | `tests/verification.rs` | 26 |
| Rust 桌面 lib | 窗口/数据库失败路径 | 2 |
| Rust Hook CLI | `crates/xushu-hook/tests/cli.rs` | 6 |
| Rust Hook 协议 | `xushu-hook-protocol` lib | 5 |
| **Rust 合计** | — | **100** |

## 8. 质量检查真实退出码

| 命令 | 退出码 | 结果 |
|---|---|---|
| `pnpm typecheck` | 0 | `tsc --noEmit` 通过 |
| `pnpm lint` | 0 | ESLint 10，`--max-warnings 0` |
| `pnpm test` | 0 | 8 个文件、48 个测试通过 |
| `pnpm build` | 0 | Vite 通过；JS 827.13 kB / gzip 246.40 kB；仍有原有 >500 kB 提示 |
| `pnpm format` | 0 | Prettier 通过（首次检查发现 2 个本次修改文件未格式化，格式化后复跑通过） |
| `cargo fmt --all -- --check` | 0 | rustfmt 通过 |
| `cargo check --workspace --all-targets --locked` | 0 | Rust workspace 通过 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | 无警告 |
| `cargo test --workspace --locked` | 0 | 100 项通过 |
| `cargo build -p xushu-hook --release` | 0 | 302 KB release 可执行文件 |
| Hook CLI 计时（30 次） | 0 | 平均 14.46 ms；spool 30 个；无敏感字段 |
| 桌面无截图冒烟 | 0 | 接入区块/预览/布局/工作台回归通过；未安装 Hooks |
| 真实 Claude Code 联调 | **NOT_RUN** | 需用户授权测试项目并手动启动 Claude Code |
| 人工视觉复核 | **PENDING** | 未自动截图 |

> 命令日志与桌面冒烟 JSON 位于已忽略的 `target/phase4-audit/`，不进入仓库。

## 9. 桌面端测试结果

- 使用真实 Tauri/WebView2 桌面程序与既有 `Phase 3.5 UI 审阅` 测试数据；未创建或删除业务记录。
- 项目详情接入区块可见；点击“查看并安装”后预览包含六个 Hook 表。
- 未勾选授权复选框时，安装按钮为禁用状态。
- 1366×768：接入 Section 与 `main` 均无横向溢出（`scrollWidth == clientWidth`）。
- 500×480 CDP 视口：接入 Section 与 `main` 均无横向溢出。
- 返回工作台后，“下一步”与“最近验证”仍正常渲染。
- 未安装任何真实项目 Hooks；测试项目未生成 `.claude` 目录。
- 测试结束后通过自绘关闭按钮退出桌面进程，Vite 与 Tauri 进程均结束。
- 未自动截图/录屏；视觉细节仍需用户人工截图复核。

## 10. 尚未验证或未实现的事项

- **真实 Claude Code 联调 NOT_RUN**：尚未在真实 Claude Code 会话中触发六个 Hook 并验证入库。需用户按 `docs/architecture/CLAUDE_HOOKS.md` 第 8 节在明确授权的测试项目执行。
- **生产安装包 sidecar**：尚未把 `xushu-hook.exe` 作为 Tauri sidecar 打包；当前开发构建依赖同目录二进制或 `XUSHU_HOOK_BIN`。
- **人工视觉复核**：安装预览、活动 Sheet、深浅主题、500×480、键盘焦点仍为 PENDING；未自动截图。
- **旧版 Claude Code / macOS**：仅在 Windows 11 + Claude Code 2.1.319 核对；旧版本字段兼容性、macOS/Linux 未验证。
- **项目归档**：不会自动停用已安装 Hooks；需手动停用/卸载。
- **会话结束缺失**：只能显示“结束未确认”，无法恢复丢失事件。
- **设置备份内容**：可能包含用户自己的 `env` 值；只存应用数据目录，未做加密。
- **生产数据库**：迁移 4 已在临时数据库与真实应用数据目录测试；未做大规模历史数据性能测试。
- **前端 bundle**：仍为单 chunk（827.13 kB），有 >500 kB 提示；未做构建拆分。

## 11. 是否修改任何 Claude Code 配置

**没有修改任何项目的 `.claude/settings*.json`、用户级 `~/.claude/settings.json` 或系统级配置。**

本轮实际发生的本地变化：

- 桌面应用启动时在应用数据目录创建/确认 `hook-inbox/`、`hook-backups/` 与 `.xushu-salt`；这是 XUSHU 运行时目录，不是 Claude Code 配置。
- 桌面冒烟只打开安装预览，未点击“安装并启用”。
- 测试项目 `target/e2e/ui-repo-20261106032041` 未生成 `.claude` 目录。
- 未启用任何真实项目 Hooks；未修改全局环境变量、PATH、TEMP/TMP。

## 12. 当前 Git 状态

- 分支：`main`
- 本轮**未提交、未推送、未创建远程仓库**。
- 工作区包含 Phase 3.5/3.6 既有未提交改动，全部保留；本轮新增/修改的文件见第 2 节。
- `git status --short` 共 79 条变更（含既有 UI 改动与本轮新增文件）；未执行 `git reset --hard`、checkout、clean 或批量删除。
- 关键新增未跟踪文件：
  - `crates/xushu-hook/`
  - `crates/xushu-hook-protocol/`
  - `crates/xushu-core/src/claude/`
  - `crates/xushu-core/migrations/0004_claude_hooks.sql`
  - `crates/xushu-core/tests/claude.rs`
  - `apps/desktop/src-tauri/src/claude_commands.rs`
  - `apps/desktop/src/features/claude/`
  - `docs/architecture/ADR/0007-claude-code-hooks-integration.md`
  - `docs/architecture/CLAUDE_HOOKS.md`

## 13. 下一步所需的人工操作

1. 审查本轮差异与 `docs/architecture/ADR/0007-claude-code-hooks-integration.md`、`docs/architecture/CLAUDE_HOOKS.md`。
2. 若要验证真实接入，创建/选择一个**一次性测试 Git 项目**，导入 XUSHU，按 `CLAUDE_HOOKS.md` 第 8 节执行安装 → 运行 `claude` → 触发工具 → 结束 → 查看活动 → 停用 → 卸载；记录真实结果。
3. 人工截图复核项目详情接入 strip、安装预览、活动 Sheet、空/错误状态、深色/浅色与 500×480；完成后更新 `docs/design/UI_AUDIT.md`。
4. 如需 Phase 5，请用户明确批准；不得把本报告 PASS 解释为真实 Claude Code 联调或视觉验收已完成。
5. 生产分发前完成 `xushu-hook` sidecar 打包、升级路径与安装包验证（Phase 7）。

---

**明确结论：** 本轮完成的是 Phase 4 的代码、数据层、IPC、UI 与自动化/桌面冒烟验证；**真实 Claude Code 联调与人工视觉验收尚未完成**，必须以 `NOT_RUN` / `PENDING` 记录，不能宣称整个 Phase 4 已通过真实端到端验收。
