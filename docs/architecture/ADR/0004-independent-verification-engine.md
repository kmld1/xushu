# ADR-0004：独立验收引擎与证据可信规则

- 状态：Accepted
- 日期：2026-11-06
- 决策者：项目负责人（用户）与主开发助手
- 相关文档：`MASTER_DEVELOPMENT_PLAN.md`、`docs/phases/PHASE_2_REPORT.md`、`docs/phases/PHASE_3_REPORT.md`、`docs/architecture/ADR/0003-projects-tasks-state-machine.md`
- 上游决策：ADR-0001（本地优先技术栈）、ADR-0002（SQLite 与受控 IPC）、ADR-0003（任务状态机与完成门禁）

## 背景

Phase 2 的完成门禁在 Rust 侧强制执行，但自动验收条件永远不满足：当时没有验证运行器，也没有证据模型。Phase 3 必须补齐“任务验收条件 → 验证配置 → 用户授权 → 真实执行 → 证据 → 判定 → 完成门禁”的闭环。

关键约束：

- AI 文本、前端状态、Git 改动、Hook 事件均不能替代真实验证；
- 不能把 AI 生成的任意 Shell 字符串直接执行；
- 验证结果必须与任务修订、验收条件修订、验证配置修订和代码状态绑定；
- 旧结果不得被删除，但必须能被判定为过期；
- Windows 下取消/超时必须清理子进程树，不留下无人管理的后台进程；
- 验证执行不得阻塞 Tauri UI。

## 决策

### 1. 数据模型（迁移 3）

新增四张表，保留 Phase 1/2 全部数据：

- `verification_specs`：每条自动验收条件最多一个验证配置；保存可执行程序、参数数组 JSON、显式工作目录、超时、输出上限、成功规则、配置修订号。
- `verification_runs`：一次经用户批准的运行；保存任务/条件/配置修订快照、程序参数快照、开始/结束代码状态、退出码、耗时、执行状态、脱敏输出摘要与错误。
- `verification_checks`：运行产生的检查判定（`pass | fail | not_run | inconclusive`）及理由。
- `evidence`：与运行关联的证据（摘要、可选脱敏输出、内容摘要哈希、敏感性标记）。

关系：

- `task_id → tasks(id) ON DELETE RESTRICT`（任务不硬删除）；
- `criterion_id → acceptance_criteria(id) ON DELETE SET NULL`，同时保存条件描述快照，删除条件不丢历史；
- `spec_id → verification_specs(id) ON DELETE SET NULL`，同时保存配置快照；
- `verification_checks.evidence_id → evidence(id) ON DELETE SET NULL`。

### 2. 三层状态严格分离

- **执行状态** `execution_status`：`running | completed | cancelled | timed_out | start_failed | interrupted`；
- **检查结果** `result`：`pass | fail | not_run | inconclusive`；
- **证据有效性**：派生状态 `valid | stale`，不修改历史行。

`STALE` 不是写入的检查结果，而是查询/完成门禁时根据当前任务修订、条件修订、配置修订与当前代码状态计算出的结论。历史 `pass/fail` 原样保留。

### 3. 结果判定规则

首版唯一成功规则为 `exit_code_zero`，并在配置中显式保存、在批准界面展示：

| 情况 | 执行状态 | 检查结果 |
|---|---|---|
| 正常退出，退出码 0 | `completed` | `pass` |
| 正常退出，退出码非 0 | `completed` | `fail` |
| 超时 | `timed_out` | `fail` |
| 用户取消 | `cancelled` | `inconclusive` |
| 程序不存在/工作目录无效/启动失败 | `start_failed` | `not_run` |
| 应用重启导致运行中断 | `interrupted` | `inconclusive` |
| 运行期间代码变化、指纹不完整、内部异常 | 对应状态 | `inconclusive` |

超时视为不满足验收条件，绝不产生 PASS；取消与中断不伪造 NOT_RUN，也不伪造 PASS。

### 4. 命令安全边界

- 程序与参数数组分离，使用 `std::process::Command`，不经过 Shell；
- 拒绝 `cmd`、`powershell`、`pwsh`、`sh`、`bash`、`zsh`、`wsl` 等 Shell 可执行文件；
- Windows 首版还拒绝 `.bat/.cmd/.ps1/.vbs/.js/.wsf` 等包装器，PATH 解析只接受 `.exe/.com`；
- 工作目录必须是项目根目录或其子目录，canonicalize 后校验边界；
- 前端不能在执行时提交任意命令：只提交 `criterionId`、期望修订号和 `approved=true`；Rust 从数据库读取已保存配置；
- 未经批准（`approved != true`）一律拒绝；
- 只有任务处于 `awaiting_verification` 时才允许启动验证，避免在未声明执行完成时生成“完成证据”；
- 同一任务同一时间只允许一个运行；
- 明确提示：即使无 Shell 拼接，测试脚本本身仍可能执行任意代码；用户必须确认项目可信。

### 5. 异步执行与进程树

- `start_verification` 只做校验、写入 `running` 运行记录并启动后台线程，立即返回；
- 子进程在后台线程执行，不持有数据库锁；
- Windows 使用 Job Object（`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`）绑定子进程树；取消/超时调用 `TerminateJobObject`，应用退出时句柄关闭也会终止整棵树；若 Job 分配失败，取消/超时时回退到 `taskkill /PID <pid> /T /F`；
- 非 Windows 平台首版使用主进程 `kill`，并明确标注未做进程组验证；
- 输出由独立线程读取，按配置上限截断并继续排空管道，避免死锁与无界内存；
- 取消通过共享 `RunControl` 标志与 Job 句柄完成；取消命令不直接信任前端状态。

### 6. 代码状态与证据失效

运行时记录：

- Git HEAD、分支、detached、unborn、dirty；
- 工作区内容指纹：对 `git status --porcelain=v1 -z --untracked-files=all` 列出的变更/未跟踪文件，在项目根目录内读取字节计算 SHA-256，连同路径与状态排序后汇总；
- 指纹完整性标记；超过文件数/总字节/单文件上限时标记不完整；
- 运行开始与结束各采集一次，两者必须一致。

完成门禁要求：

1. 最新运行必须 `completed` 且 `pass`；
2. `task_revision`、`criterion_revision`、`spec_revision` 与当前一致；
3. 当前代码状态与运行结束时一致（HEAD、分支、detached 与内容指纹）；
4. 指纹必须完整，运行前后必须一致。

任一不满足：

- 历史结果仍在，但视图返回 `STALE`；
- 无法可靠采集当前代码状态时返回 `INCONCLUSIVE`；
- 均不能作为完成证据。

不为了实现失效判断修改用户 Git 仓库；只读执行 `rev-parse`、`symbolic-ref`、`status`。

### 7. 完成门禁

修改 Phase 2 的 `criteria_with_completion`：

- 人工条件仍要求匹配当前 `criterion_revision` 与 `task_revision` 的确认；
- 自动条件要求存在当前有效 PASS 证据；
- 二者来源严格分开展示；
- 自动条件没有配置、未运行、失败、无法判定、过期时均产生 blocker；
- 不新增豁免路径；
- 前端仍只展示 Rust 计算的 `completion.blockers`。

### 8. IPC

新增验证配置、模板查询、预览、批准启动、状态查询、取消、历史、检查、证据、报告导出命令。所有命令经 `VerificationService` 与 `TaskService`，不向前端开放通用 Shell、文件系统或 SQL。`capabilities` 仍只有 `core:default`。

验收中心队列由 `TaskService::verification_queue` 返回；Git 指纹在数据库锁外采集，避免与后台 finalize 写入相互等待。报告写入应用数据目录的 `exports/`，IPC 只返回文件名与 Markdown，不返回绝对路径。

### 9. 脱敏与证据

- 输出按行/模式脱敏：凭据关键词赋值、Bearer Token、私钥块、常见 API Key 前缀；
- 保存脱敏后的输出与敏感性标记；原始敏感输出不落库；
- 证据内容哈希覆盖存储的脱敏内容与关键元数据；
- 明确记录：正则脱敏不保证 100% 防泄漏；用户可选择仅保存元数据（输出上限设为最小并保留截断标记）。
- 报告导出为本地 Markdown 文件，写入应用数据目录的 `exports/`，不返回绝对路径。

### 10. UI

- 使用 `frontend-design` Skill，复用 `docs/design/DESIGN_SYSTEM.md` 的 Tokens 与共享组件；
- 新增验收中心：待验收任务、条件状态、配置、命令预览、风险提示、批准、运行进度、取消、结果、证据、历史、过期提示、报告导出；
- 状态使用图标 + 中文 + 颜色，不只靠颜色；
- 不自动截图；人工视觉验收保持 `VISUAL_REVIEW_PENDING`。

## 备选方案与取舍

| 备选 | 未采用原因 |
|---|---|
| 执行 AI 生成的 Shell 字符串 | 注入与不可审计；违反最小权限与可复核原则。 |
| 前端在批准时提交 executable/args | 可绕过配置审查；批准应针对已保存的明确配置。 |
| 用 `cmd /c` / `powershell -Command` 运行 | 首版明确禁止；增加不可控解释层。 |
| 仅比较 Git HEAD 判断代码未变 | 未提交修改同样影响结果；不可靠。 |
| 仅比较 dirty 布尔值 | 两个不同工作区都可能 dirty=true。 |
| 取消只杀主进程 | 测试脚本常派生子进程；会留下后台进程。 |
| 取消/超时之后伪造 NOT_RUN | 掩盖真实执行尝试；采用 `inconclusive`/`fail` 并保留原因。 |
| 过期时删除旧结果 | 破坏可追溯性；过期是派生状态。 |
| 自动把失败改写成通过 | 违反产品核心价值。 |
| 引入云验证服务 | 违反本地优先与隐私边界。 |

## 后果

**正面：**

- 自动验收条件第一次拥有真实、可追溯的证据；
- 完成门禁无法被前端或 AI 文本绕过；
- 代码、任务、条件或配置变化后旧证据自动不可用于完成；
- 取消/超时不会遗留无人管理的子进程；
- 历史运行与证据保留，可导出报告。

**负面 / 风险：**

- 代码指纹需要读取变更文件内容并在 Rust 内哈希；超大工作区会标记不完整并阻止自动 PASS；
- Windows Job Object 之外的平台未做真实进程组验证；
- 测试脚本仍然拥有用户权限，可信项目确认不能替代沙箱；
- 脱敏是模式匹配，不能保证覆盖所有密钥格式；
- 单连接 SQLite 在高频运行时查询下可能短暂排队；
- 生产 CSP、安装包与 macOS 仍未在本阶段验证。

## 回滚

若需调整判定或过期规则，必须新增 ADR；不得删除已有运行/检查/证据记录，不得让历史 PASS 在新规则下被静默视为有效。代码回滚后，数据库迁移 3 保留；旧应用会因 `SchemaTooNew` 拒绝打开新数据库，需先导出数据或升级应用。

## 参考

- <https://doc.rust-lang.org/std/process/struct.Command.html>
- <https://learn.microsoft.com/windows/win32/procthread/job-objects>
- <https://git-scm.com/docs/git-status>
- <https://www.sqlite.org/foreignkeys.html>
- <https://v2.tauri.app/security/capabilities/>