# ADR-0007：Claude Code 项目级 Hooks 接入与最小事件观察

- 状态：Accepted
- 日期：2026-11-09
- 决策者：项目负责人（用户）与主开发助手
- 相关文档：`MASTER_DEVELOPMENT_PLAN.md`、`docs/architecture/CLAUDE_HOOKS.md`、`docs/phases/PHASE_4_REPORT.md`
- 上游决策：ADR-0001（本地优先技术栈）、ADR-0002（SQLite 与受控 IPC）、ADR-0004（独立验收引擎）
- 已核对环境：Claude Code **2.1.319**；官方 Hooks reference/guide/settings 文档（2026-11-09）

## 背景

Phase 4 需要回答一个有限的问题：在用户明确授权后，XUSHU 能否观察到某个本地项目的 Claude Code 开发活动，并且**不把观察结果冒充为任务完成证据**？

约束：

- Hook 输入可能包含 Prompt、命令、文件内容、工具输出、transcript 路径和凭据；不能把完整 stdin JSON 落库、写日志或排队。
- Hook 不能阻塞 Claude Code；观察程序失败必须安静退出。
- 只能修改用户选定项目的本地设置，不能改用户级、系统级或团队共享设置。
- 不能因为收到 `Stop` 或 `PostToolUse` 就修改任务状态或生成验收证据。
- Windows 优先；不引入云服务、HTTP 服务、消息队列或后台常驻守护进程。
- 安装、停用、卸载必须可逆，且保留其他第三方 Hooks。

## 决策

### 1. 本地 CLI + 原子 spool，而不是 localhost HTTP

`xushu-hook` 是独立的 Rust 原生可执行文件。Claude Code 通过项目级 `command` Hook 启动它，stdin 传入官方 Hook JSON。它只做：

```text
有界读取 stdin → 白名单字段过滤 → 带盐 SHA-256 伪名化 → 原子写入 spool
```

spool 位于 XUSHU 应用数据目录：

```text
<app_data>/hook-inbox/
├── .xushu-salt
├── pending/<project_id>/<timestamp>-<delivery_id>.json
└── tmp/
```

桌面应用在启动、刷新接入状态、打开活动 Sheet 时导入 spool，事务写入 SQLite 后删除原文件。XUSHU 关闭时 Hook 仍可写入，下一次启动导入；这不等于“已实时接收”，UI 会显示待导入数量与最近导入时间。

未采用 localhost HTTP：增加监听端口、认证、局域网暴露和生命周期管理成本，且需要额外依赖；本阶段的事件量很小，spool 已满足离线与失败隔离要求。

### 2. 仅写入项目本地 `.claude/settings.local.json`

- 安装目标是 `<project_root>/.claude/settings.local.json`。
- 不修改 `.claude/settings.json`（团队共享）、`~/.claude/settings.json`、managed settings 或环境变量。
- 使用 `serde_json::Value` 结构化合并，保留 `permissions`、`env`、其他 `hooks`、自定义字段。
- 安装前备份原文件到应用数据目录；卸载只删除带 `args[0] == "--xushu-managed" && args[1] == <project_id>` 标记的条目。
- 安装/卸载前检测 `.claude` 或设置文件是否为符号链接/重解析点；拒绝危险路径。
- 写入采用同目录临时文件 + rename；写入后重新读取校验；数据库更新失败时尝试恢复原文件。
- 预览 token 绑定当前设置文件内容、二进制路径、项目根目录、inbox 路径与六个 Hook 计划；预览后配置变化会使安装被拒绝。

### 3. 事件白名单

仅注册六个事件：

| 事件 | 用途 | 明确不作证 |
|---|---|---|
| `SessionStart` | 会话开始/恢复 | 不代表任务开始 |
| `PreToolUse` | 工具调用开始 | 不代表权限通过或执行成功 |
| `PostToolUse` | 工具调用成功 | 不代表代码正确或验收通过 |
| `PostToolUseFailure` | 工具调用失败/中断 | 不保存错误正文 |
| `Stop` | 一轮回答结束 | **不代表整个开发任务完成** |
| `SessionEnd` | 会话结束 | 缺失时显示“结束未确认” |

只保留：事件类型、接收时间、伪名化会话/工具调用/Prompt 标识、工具名称与类别、成功/失败/未知、错误类别、耗时、会话来源/结束原因。**不保留** `tool_input`、`tool_response`、`error` 文本、`last_assistant_message`、transcript、cwd 原文、完整命令、文件路径、模型或凭据。

### 4. 伪名化与盐

- 应用首次启动生成 32 字节随机盐 `.xushu-salt`，存放在应用数据目录；不通过 IPC 暴露、不写入事件。
- `session_hash = SHA-256(salt || 0x01 || session_id)`；工具调用/Prompt 使用不同 domain 字节。
- 项目根指纹使用 canonical path + salt；导入时重新计算并比较，防止把同名不同位置的项目混淆。
- UI 只显示哈希前 12 位作为会话引用。

### 5. 数据模型（迁移 4）

新增四张表，全部使用 UUID、UTC RFC3339、外键与 CHECK：

- `claude_integrations`：项目级期望状态、安装版本/二进制路径/摘要、最近导入/错误、卸载时间。
- `claude_sessions`：伪名化会话、开始/最后事件/结束、来源与结束原因、是否观察过开始。
- `claude_tool_calls`：按会话 + `tool_use_hash` 唯一的工具调用聚合；状态 `running | succeeded | failed | unknown`；记录是否观察开始/结束与事件冲突。
- `claude_events`：最小事件账本；`delivery_id` 唯一保证 spool 导入幂等；工具事件带语义去重键；不保存任何原始 payload。

保留策略：30 天、每项目最多 5000 条事件与 500 个会话；spool 每项目最多 1000 个事件文件（16 KiB/文件）。清理在导入事务后执行。

### 6. 三层状态严格分离

- **期望状态**：`claude_integrations.desired_enabled`。
- **实际安装状态**：每次从项目设置文件重新读取，与当前二进制路径/超时/参数比较；不信任数据库布尔值。
- **真实接收状态**：spool、事件表、最近导入时间与错误类别。

派生状态包括：未配置、已停用、已启用、部分安装、需要修复、配置异常、接收器不可用、项目不可用、状态不一致。UI 必须同时显示状态文字与原因。

### 7. 不修改任务状态

`xushu-core::claude` 不引用 `TaskService`、验收条件、证据或完成门禁。Hook 事件只能出现在 Claude 活动时间线；任务 `done` 仍只能由当前有效的独立验收证据与人工确认满足。测试会显式断言导入前后任务状态/修订不变，且不会生成 `verification_runs`。

### 8. 安全与失败行为

- Hook CLI 对所有异常（无盐、无 inbox、无效 JSON、超上限、项目不匹配、无参数）安静退出 0，不写 stdout/stderr。
- 单文件 16 KiB、原始 stdin 256 KiB、每项目 1000 个待导入文件；超限安全丢弃。
- 导入器使用 `deny_unknown_fields` 校验信封；项目 ID 与根指纹必须匹配；未授权的项目事件被拒绝并删除。
- 停用/卸载前先循环导入待处理事件，避免把启用期间的事件静默丢弃；若仍有剩余则拒绝停用并提示先刷新。
- 事件导入失败不会影响 Claude Code，也不会影响任务。

### 9. IPC 与 UI

新增只读/受控命令：状态、活动、预览、安装、停用、卸载、刷新导入；所有写入命令要求 `approved: true`。前端不接触文件系统、SQL 或原始 Hook JSON。项目详情页增加“Claude Code 接入”区块，活动使用右侧 Sheet；不新增一级导航。

## 备选方案与取舍

| 备选 | 未采用原因 |
|---|---|
| localhost HTTP 接收服务 | 需要端口、认证、局域网边界和常驻生命周期；本阶段无必要。 |
| 直接把 stdin JSON 存 SQLite | 违反最小采集；会保存 Prompt、命令、文件内容、凭据。 |
| 只存原始 session_id / tool_use_id | 可与 transcript、日志和外部数据关联；改用本地盐伪名化。 |
| 修改 `.claude/settings.json` | 会进入团队共享配置；只允许 `settings.local.json`。 |
| 修改用户级/系统级设置 | 违反最小权限与授权范围。 |
| 使用 shell 包装 Hook 命令 | 增加注入与路径解析风险；使用 `command` + `args` 的 exec 形式。 |
| 将 `Stop` 或 `PostToolUse` 视为完成 | 违反产品核心价值；事件不是验收证据。 |
| 在 XUSHU 未启动时假装实时接收 | 使用有界 spool 与“待导入/最近导入”状态，明确延迟。 |
| 自动异步 Hook 全部事件 | 异步进程可能在会话结束时被截断；当前同步 CLI 实测约 14.5 ms/次，可接受。 |

## 后果

**正面：**

- 用户授权后可在项目详情看到真实的会话、工具调用与结束事件；
- 原始敏感内容不进入 spool、SQLite、日志或 UI；
- 安装/停用/卸载可逆，其他 Hooks 保留，配置变化可检测并修复；
- 事件与任务完成门禁物理隔离；
- XUSHU 关闭期间事件可在下次启动导入，不伪造实时成功。

**负面 / 风险：**

- 生产安装包尚未把 `xushu-hook.exe` 作为 sidecar 打包；当前开发构建依赖 `target/debug` 同目录的二进制。Phase 7 需完成发布包路径与升级验证。
- Hook 是尽力观察：会话结束事件缺失时只能显示“结束未确认”，不能推断成功。
- 同步 Hook 仍会为每次事件启动一个短进程；本机实测平均约 14.5 ms，但防病毒/慢磁盘环境可能更高。
- 设置文件备份可能包含用户自己的 `env` 值；备份只存放在应用数据目录，不导出、不记录日志，但仍是同用户可读副本。
- 项目归档不会自动停用已安装 Hooks；用户需在项目详情停用/卸载。该行为已记录为限制。
- 目前只在 Windows 11 与 Claude Code 2.1.319 上核对；macOS 与旧版本未验证。

## 回滚

1. 在项目详情执行“停用事件采集”或“卸载接入”，只删除 XUSHU 创建的 Hook 条目；其他配置保持不变。
2. 如 UI 不可用，可手工删除 `.claude/settings.local.json` 中带 `--xushu-managed` 的处理器；不得删除其他 Hooks。
3. 代码回滚后保留迁移 4；旧应用会因 `SchemaTooNew` 拒绝打开新数据库，需先导出或升级应用。
4. spool/备份目录可手工清理；不会影响项目源码或任务数据。

## 参考

- <https://code.claude.com/docs/en/hooks>
- <https://code.claude.com/docs/en/hooks-guide>
- <https://code.claude.com/docs/en/settings>
- <https://code.claude.com/docs/en/settings-reference>
- <https://doc.rust-lang.org/std/fs/fn.rename.html>
- <https://www.sqlite.org/foreignkeys.html>