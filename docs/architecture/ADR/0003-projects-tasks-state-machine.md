# ADR-0003：项目、任务、验收修订与可信完成门禁

- 状态：Accepted
- 日期：2026-11-06
- 决策者：项目负责人（用户）与主开发助手
- 相关文档：`MASTER_DEVELOPMENT_PLAN.md`、`docs/phases/PHASE_2_REPORT.md`、`docs/architecture/DEPENDENCIES.md`
- 上游决策：ADR-0001（本地优先技术栈）、ADR-0002（SQLite 与受控 IPC）

## 背景

Phase 2 要让 XUSHU 从工程骨架变成真正可用的项目/任务管理系统：

- 导入本地 Git 项目并只读检查真实状态；
- 创建任务、验收条件与状态历史；
- 由 Rust 统一管理状态机；
- 在 Phase 3 自动验证引擎尚未存在时，仍然保证“完成”可信；
- 所有业务状态持久化到 SQLite，并在重启后恢复。

本阶段不执行验证命令、不接入 Claude Code Hooks、不调用额外 AI 服务。

## 决策

### 1. Git 只读检查

- 使用 `std::process::Command` 以**可执行文件 + 参数数组**调用 `git`，不经过 shell 字符串。
- 每条 Git 命令 5 秒超时；stdout/stderr 由独立线程读取，避免管道阻塞。
- 只读取：
  - `rev-parse --is-inside-work-tree`
  - `rev-parse --show-toplevel`
  - `symbolic-ref --quiet --short HEAD`
  - `rev-parse --verify HEAD`
  - `status --porcelain=v1 --untracked-files=normal`
- 不支持、也不执行任何写命令（checkout/reset/clean/add/commit 等）。
- 支持空仓库（unborn）、detached HEAD 与 worktree；不读取源码内容或文件列表。
- 导入时保存 Git 根目录（canonical path），同一根目录通过规范化 key 唯一约束防止重复导入。

### 2. 项目生命周期

- `projects` 保存名称、根路径、根路径 key、创建/更新时间与 `archived_at`。
- “移除”采用**软归档**，不删除磁盘项目、Git 仓库或任务数据；可恢复。
- 归档项目默认从活动项目与工作台统计中排除，任务仍保留。
- 路径检查、Git 检查与唯一性检查全部在 Rust 侧完成。
- 项目重命名只修改 XUSHU 显示名，不修改磁盘目录或 Git 配置。

### 3. 任务与验收修订模型

- `tasks.revision` 表示任务内容/验收定义修订号，不表示状态版本。
- 以下操作增加任务 revision，并使旧人工确认失效：
  - 编辑标题、描述、优先级；
  - 新增、修改、删除验收条件；
  - 从待验证状态返工/阻塞/取消（离开待验证）；
  - 重新开启已取消任务。
- 普通状态推进（ready/running/awaiting/done）不增加 revision；因此人工确认可以跨 `awaiting_verification → done` 保持有效。
- `acceptance_criteria.revision` 表示单条验收条件的修订号。
- `manual_confirmations` 记录 `criterion_revision` 与 `task_revision`；只有两者都匹配当前修订时才视为有效。
- 所有编辑使用 `expected_revision` 做乐观并发检查；旧编辑返回 `CONFLICT`，不覆盖新数据。

### 4. 状态机

状态：`draft → ready → running → awaiting_verification → done`，另有
`blocked` 与 `cancelled`。

合法转换：

| 从 | 到 |
|---|---|
| draft | ready, cancelled |
| ready | running, blocked, cancelled |
| running | awaiting_verification, blocked, cancelled |
| awaiting_verification | running, done, blocked, cancelled |
| blocked | ready, running, cancelled |
| done | 无 |
| cancelled | 无（仅独立 `reopen_task`） |

规则：

- `draft → ready` 要求至少一个必需验收条件。
- `blocked` 必须填写原因；解除后清空当前阻塞原因。
- `cancelled` 是终止状态；重新开启必须调用独立 `reopen_task`，填写原因，回到 draft 并增加 revision。
- 从 `awaiting_verification` 离开（除直接完成外）增加 revision，旧人工确认失效。
- 所有转换由 Rust 校验并写入 `task_transitions`；前端不得自行修改状态。
- 转换在事务中完成。

### 5. 完成门禁

任务只能从 `awaiting_verification` 进入 `done`，且必须：

1. 至少有一个**必需**验收条件；
2. 所有必需条件满足：
   - 人工条件：存在匹配当前 `criterion_revision` 与 `task_revision` 的确认记录；
   - 自动条件：Phase 2 **永远不满足**，因为没有验证运行器。
3. 没有自动豁免。Phase 2 不提供“跳过/豁免”路径，避免绕过验收。

因此：

- 仅有可选条件的任务不能进入 ready，也不能完成；
- 含必需自动条件的任务可以执行到 `awaiting_verification`，但完成后门禁会明确阻止；
- 不会把 AI 文本、Hook 事件、前端状态或“用户点击完成”当作 PASS 证据；
- UI 必须展示具体 blockers，而不是只显示“不能完成”。

Phase 3 只有接入真实验证证据后，才能让自动条件满足；本阶段未伪造任何自动 PASS。

### 6. 数据模型与迁移

迁移 2 `0002_projects_tasks.sql` 新增：

- `projects`
- `tasks`
- `acceptance_criteria`
- `task_transitions`
- `manual_confirmations`

约束：

- 所有 ID 为 TEXT UUID；
- 时间 UTC RFC3339；
- 外键：tasks → projects、criteria → tasks、transitions → tasks、confirmations → criteria；
- 枚举使用 CHECK；
- `projects(root_path_key)` 唯一；人工确认 `(criterion_id, criterion_revision, task_revision)` 唯一；
- 任务/项目归档为 `archived_at`，不硬删除；
- 保留 Phase 1 `persistence_probes`；
- 迁移仍沿用 ADR-0002 的版本表、单迁移事务与拒绝降级机制。

### 7. IPC 与前端边界

- 新增项目/任务/验收/工作台命令，全部通过 `TaskService`/`ProjectService` 调用；
- 不向前端开放 SQL、Shell、文件系统或数据库路径；
- capabilities 仍只有 `core:default`；
- 错误返回 `{ code, message }`，新增 `CONFLICT`、`INVALID_TRANSITION`、`TASK_NOT_COMPLETABLE`、`GIT_ERROR`；
- 前端只展示后端返回的 `allowedTransitions`、`completion.blockers`，不自行推导状态合法性。

### 8. UI 设计系统

- 使用 `frontend-design` Skill 先建立 `docs/design/DESIGN_SYSTEM.md`；
- 统一 App Shell、Tokens、组件（Button/Input/Select/Dialog/Dropdown/Tabs/Table/Tooltip/Toast/Status/Empty/Loading/Error）；
- 状态必须图标 + 中文 + 颜色；
- 使用 HashRouter、TanStack Query、Zustand、RHF + Zod；
- 不展示假数据；空/错误/加载状态统一；
- 视觉检查由用户手动截图，未完成前标记 `VISUAL_REVIEW_PENDING`。

## 备选方案与取舍

| 备选 | 未采用原因 |
|---|---|
| 前端维护状态机或直接改 DB | 可绕过完成门禁；违反可信边界。 |
| 删除项目时级联删除任务 | 破坏可追溯性与恢复能力；采用软归档。 |
| 任务状态转换也增加 revision | 会使待验证阶段的人工确认立即失效，流程不可用；采用内容修订号 + 离开待验证失效。 |
| Phase 2 伪造自动验收 PASS | 违反“AI 文本/前端状态不是证据”；自动条件保持待验证。 |
| 允许豁免/跳过必需条件 | 会形成完成漏洞；Phase 2 明确不做。 |
| 以 shell 字符串调用 git | 注入与路径风险；采用参数数组 + 超时。 |
| 用前端计算完成状态 | 不可信；完成门禁在 Rust。 |

## 后果

**正面：**

- 项目、任务、验收与历史成为真实的本地业务数据；
- 完成必须满足 Rust 计算的门禁，不能靠点击或 AI 声明绕过；
- 自动验收尚未实现时行为明确、可解释，而不是假通过；
- Git 检查只读、可超时、支持合理边界情况；
- UI 有统一设计系统与共享组件。

**负面 / 风险：**

- 含必需自动条件的任务在 Phase 3 前无法完成——这是有意的安全行为，但可能让部分用户觉得受限；
- 软归档不会释放路径去重；同一 Git 根目录归档后需恢复而非重新导入；
- 人工确认只能在待验证状态创建；编辑/返工会使其失效，需要重新确认；
- Windows 的路径 key 使用小写规范化；macOS 大小写敏感文件系统后续需重新评估；
- E2E 依赖 Windows UI Automation 作为开发测试，不是跨平台自动化套件；
- 生产 CSP、安装包、macOS 未在本阶段实机验证；
- 前端 bundle 约 719 KB（gzip 220 KB），后续可做路由级拆包。

## 回滚

若未来改变状态机或完成门禁，必须新增 ADR，并保留 `task_transitions`、`manual_confirmations` 历史与迁移记录；
不得删除用户任务/项目数据，也不得让前端绕过新的 Rust 门禁。回滚代码时需评估已有 done 任务是否仍满足旧规则。

## 参考

- <https://git-scm.com/docs/git-rev-parse>
- <https://git-scm.com/docs/git-status>
- <https://www.sqlite.org/foreignkeys.html>
- <https://www.sqlite.org/lang_transaction.html>
- <https://v2.tauri.app/security/capabilities/>
- `docs/design/DESIGN_SYSTEM.md`