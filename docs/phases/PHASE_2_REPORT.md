# Phase 2 报告：项目管理、任务系统与状态机

- 日期：2026-11-06
- 项目：序枢 XUSHU
- 根目录：`D:\MyProject\xushu`
- 总体状态：**PASS**
- 结论：XUSHU 已成为可管理本地 Git 项目、任务、验收条件与状态历史的桌面应用。Rust 领域层统一执行输入校验、乐观并发、状态机与完成门禁；真实桌面 E2E 已完成导入 → 建任务 → 加人工验收 → 推进状态 → 确认 → 完成 → 完全关闭 → 重启恢复。
- UI 视觉状态：**VISUAL_REVIEW_PENDING**（自动化检查通过，等待用户手动截图验收）。

## 1. 实际完成的业务功能

### 项目

- 通过用户输入的本地路径导入 Git 工作树；
- Rust 侧检查路径存在、目录、Git 工作树；
- 读取 Git 根目录、分支、HEAD、dirty、detached/unborn；
- 保存项目名称、路径、创建/更新时间；
- 同一 Git 根目录唯一，防止重复导入（含子目录解析到同一根）；
- 列表、详情、重命名、软归档与恢复；
- 归档不删除磁盘文件、Git 仓库或任务数据；
- Git 只读，不执行任何写命令；不读取源码内容。

### 任务

- 创建、编辑标题/描述/优先级；
- 列表、详情、按项目/状态/关键词筛选、包含归档；
- 软归档与恢复；
- 乐观并发：旧 `revision` 编辑返回冲突，不覆盖新数据；
- 任务创建时写入初始状态历史；
- 阻塞原因、取消原因、重新开启原因；
- 状态历史（from/to、原因、任务修订、时间）。

### 验收条件与人工确认

- 验收条件新增、编辑、删除；
- 类型：自动 / 人工；必需 / 可选；
- 每条条件独立 revision；
- 人工确认绑定 `criterion_revision` 与 `task_revision`；
- 只能对 `awaiting_verification` 状态的人工条件确认；
- 编辑任务、增删改条件、返工/取消/重新开启会使旧确认失效；
- 确认记录保留，用于追溯，不静默删除历史。

### 工作台与 UI

- 真实项目数、任务状态计数、最近任务、阻塞任务、待验证任务；
- 当前项目切换与真实 Git 状态；
- 项目/任务/验收条件/状态历史页面；
- 空、加载、错误、成功/Toast 反馈；
- 深/浅/跟随系统主题；
- 统一设计系统与组件；
- 不展示假数据。

### 未做（有意）

- 自动验证命令执行器；
- 自动验收 PASS/FAIL 证据；
- 证据过期判定；
- Claude Code Hooks；
- 云服务、额外数据库、AI 接口；
- Phase 3 及以后功能。

## 2. 项目管理数据结构与 API

### 2.1 数据结构

`projects`：

| 字段 | 类型 | 说明 |
|---|---|---|
| `id` | TEXT PK | UUID |
| `name` | TEXT | 显示名称，1–120 字符 |
| `root_path` | TEXT | 规范化 Git 根目录 |
| `root_path_key` | TEXT UNIQUE | 路径去重键（Windows 小写/反斜杠规范化） |
| `created_at` | TEXT | UTC RFC3339 |
| `updated_at` | TEXT | UTC RFC3339 |
| `archived_at` | TEXT NULL | 软归档时间 |

`root_path_key` 是需求字段之外的实现字段，用于防止 Windows 大小写/斜杠差异导致重复导入；在 ADR-0003 中记录。

### 2.2 服务 API

`ProjectService`：

- `import(ImportProjectInput) -> Project`
- `list(include_archived) -> Vec<Project>`
- `get(id) -> Project`
- `rename(UpdateProjectInput) -> Project`
- `archive(id) -> Project`
- `restore(id) -> Project`
- `git_status(id) -> ProjectGitStatus`

`ProjectGitStatus`：分支、HEAD、dirty、detached、unborn、路径、检查时间。

### 2.3 Git 实现

- `std::process::Command` + 参数数组；不经过 shell；
- 每条命令 5 秒超时；
- stdout/stderr 独立线程读取；
- 只调用 `rev-parse`、`symbolic-ref`、`status`；
- 支持无提交（unborn）、detached HEAD、worktree；
- 不返回文件列表或源码内容。

## 3. 任务状态机设计与合法转换

### 3.1 状态

`draft`、`ready`、`running`、`awaiting_verification`、`done`、`blocked`、`cancelled`。

### 3.2 转换图

| 从 | 合法到 |
|---|---|
| draft | ready, cancelled |
| ready | running, blocked, cancelled |
| running | awaiting_verification, blocked, cancelled |
| awaiting_verification | running, done, blocked, cancelled |
| blocked | ready, running, cancelled |
| done | 无 |
| cancelled | 无（仅独立 reopen） |

### 3.3 门禁与规则

- `draft → ready`：至少一个必需验收条件。
- `blocked`：必须填写原因；离开 blocked 清空当前阻塞原因。
- `cancelled`：终止状态；重新开启必须调用 `reopen_task`，填写原因，回到 draft 并增加 revision。
- `awaiting_verification → done`：
  - 至少一个必需验收条件；
  - 所有必需人工条件有匹配当前 `criterion_revision` 与 `task_revision` 的确认；
  - 必需自动条件在 Phase 2 永远不满足（没有验证引擎）；
  - 无豁免/跳过路径；
  - 任一失败返回 `TASK_NOT_COMPLETABLE` 与具体 blockers。
- 编辑任务/条件、删除条件、从待验证返工/阻塞/取消、重新开启 → revision 增加，旧人工确认失效。
- 普通 draft→ready→running→awaiting→done 不增加 revision，因此人工确认可跨到 done。
- 所有转换在事务中执行并写 `task_transitions`。
- 前端不自行判断状态合法性；使用后端 `allowedTransitions` 与 `completion.blockers`。

### 3.4 测试覆盖

- 所有 7×7 状态对（合法/非法）；
- 终态无转换；
- done 只能从 awaiting 到达；
- ready 缺必需条件；
- 自动条件阻断 done 且不能被人工确认；
- 人工确认流程；
- 任务/条件编辑、返工、重新开启导致确认失效；
- 阻塞原因必填；
- 乐观并发；
- 归档任务不可转换；
- 工作台计数。

## 4. SQLite Migration 记录

### 4.1 迁移列表

| 版本 | 名称 | 内容 |
|---|---|---|
| 1 | `0001_create_persistence_probes` | Phase 1 持久化探针 |
| 2 | `0002_projects_tasks` | 项目、任务、验收条件、状态历史、人工确认 |

迁移 1 保持原样；迁移 2 只追加。迁移仍使用 ADR-0002 的版本表、单迁移事务、幂等与拒绝 schema 降级。

### 4.2 迁移 2 表

- `projects`
- `tasks`
- `acceptance_criteria`
- `task_transitions`
- `manual_confirmations`

关键约束：

- UUID 主键；UTC RFC3339 时间；
- `projects.root_path_key` 唯一；
- `tasks.project_id` → `projects.id`；
- `acceptance_criteria.task_id` → `tasks.id`；
- `task_transitions.task_id` → `tasks.id`；
- `manual_confirmations.criterion_id` → `acceptance_criteria.id`；
- `manual_confirmations(criterion_id, criterion_revision, task_revision)` 唯一；
- 状态/优先级/条件类型 CHECK；
- 项目/任务 `archived_at` 软归档；
- 保留 `persistence_probes`。

### 4.3 持久化实测

数据库位置：`%APPDATA%\dev.xushu.desktop\xushu.db`（不进入仓库）。

E2E 后独立只读查询：

```json
{
  "integrity": "ok",
  "journal_mode": "wal",
  "schema_version": 2,
  "tables": [
    "acceptance_criteria",
    "manual_confirmations",
    "persistence_probes",
    "projects",
    "schema_migrations",
    "task_transitions",
    "tasks"
  ],
  "task_status": "done",
  "task_revision": 2,
  "criterion_kind": "manual",
  "criterion_required": 1,
  "confirmation_task_revision": 2,
  "confirmation_criterion_revision": 1
}
```

状态历史完整：NULL→draft→ready→running→awaiting_verification→done。

## 5. 新增及修改文件

### 5.1 新增：Rust 领域与数据

- `crates/xushu-core/migrations/0002_projects_tasks.sql`
- `crates/xushu-core/src/git.rs`
- `crates/xushu-core/src/models.rs`
- `crates/xushu-core/src/project.rs`
- `crates/xushu-core/src/task.rs`
- `crates/xushu-core/src/state_machine.rs`
- `crates/xushu-core/src/validation.rs`
- `crates/xushu-core/tests/business.rs`

### 5.2 新增：Tauri

- `apps/desktop/src-tauri/src/business_commands.rs`
- 修改 `src/lib.rs` 注册 21 个业务命令
- 修改 `src/error.rs` 增加领域错误映射
- `src/commands.rs` 阶段字段改为 Phase 2

### 5.3 新增：前端数据层

- `src/lib/domain.ts`
- `src/lib/format.ts`
- `src/lib/task-display.ts`
- `src/lib/zod-helpers.ts`
- `src/stores/workspace-store.ts`
- `src/features/projects/{api,queries,schema}.ts`
- `src/features/tasks/{api,queries,schema}.ts`
- `src/features/workspace/use-current-project.ts`
- `src/features/workspace/signal-rail.tsx`
- `src/features/system/system-health-card.tsx`

### 5.4 新增：UI 组件与页面

- `src/components/status-badge.tsx`
- `src/components/state-track.tsx`
- `src/components/state-spine.tsx`
- `src/components/empty-state.tsx`
- `src/components/loading-state.tsx`
- `src/components/error-state.tsx`
- `src/components/confirmation-dialog.tsx`
- `src/components/ui/{dialog,dropdown-menu,select,tabs,table,tooltip,sonner}.tsx`
- `src/features/projects/{import-project-dialog,rename-project-dialog,project-actions-menu,project-git-summary}.tsx`
- `src/features/tasks/{create-task-dialog,edit-task-dialog,criterion-dialog,confirm-manual-dialog,reason-dialog,criterion-list,transition-panel,task-table}.tsx`
- `src/pages/{projects-page,project-detail-page,tasks-page,task-detail-page}.tsx`

### 5.5 修改

- `crates/xushu-core/src/{lib,error}.rs`、`src/db/migrations.rs`、`tests/persistence.rs`
- `apps/desktop/src-tauri/{src/lib.rs,src/error.rs,src/commands.rs}`
  （`Cargo.toml`、`tauri.conf.json` 经提交前核对未修改）
- `apps/desktop/src/{App.tsx,App.test.tsx,index.css}`
- `apps/desktop/src/app/{app-layout,providers}.tsx`
- `apps/desktop/src/components/{page-header,ui/*}.tsx`
- `apps/desktop/src/pages/{overview,persistence,settings}-page.tsx`
- `apps/desktop/package.json`、`pnpm-lock.yaml`
  （`pnpm-workspace.yaml` 经提交前核对未修改）
- `CLAUDE.md` 追加 UI/UX 设计系统规范
- `README.md`、`docs/architecture/DEPENDENCIES.md`

### 5.6 新增文档

- `docs/architecture/ADR/0003-projects-tasks-state-machine.md`
- `docs/design/DESIGN_SYSTEM.md`
- `docs/design/UI_AUDIT.md`
- `docs/phases/PHASE_2_REPORT.md`

### 5.7 Git

- Phase 1 基线：`85ba249 feat: 完成 Phase 1 本地持久化与桌面外壳`（本地提交，未推送）。
- Phase 2 变更：**未提交**，等待用户审查。
- 未修改历史提交，未 push。

## 6. Tauri IPC 命令清单

### Phase 1 保留（5）

`get_app_info`、`get_database_health`、`create_probe_record`、`list_probe_records`、`get_probe_record`。

### 项目（7）

`import_project`、`list_projects`、`get_project`、`update_project`、`archive_project`、`restore_project`、`get_project_git_status`。

### 任务（9）

`create_task`、`list_tasks`、`get_task`、`update_task`、`archive_task`、`restore_task`、`transition_task_status`、`reopen_task`、`get_workspace_summary`。

### 验收条件（5）

`create_acceptance_criterion`、`list_acceptance_criteria`、`update_acceptance_criterion`、`remove_acceptance_criterion`、`confirm_manual_criterion`。

### 权限与错误

- capabilities 仅 `core:default`；无 Shell/FS/SQL 插件。
- 前端不接收 SQL 或数据库路径。
- 错误代码：`VALIDATION_ERROR`、`NOT_FOUND`、`CONFLICT`、`INVALID_TRANSITION`、`TASK_NOT_COMPLETABLE`、`GIT_ERROR`、`DATABASE_*`、`SCHEMA_TOO_NEW`。
- 内部错误不返回 SQL、绝对路径或凭据。

## 7. 前端页面、交互与设计系统

### 7.1 页面

| 页面 | 路由 | 内容 |
|---|---|---|
| 工作台 | `/` | Signal Rail、当前项目、需要关注、最近任务、系统健康 |
| 项目 | `/projects` | 导入、活动/归档 Tabs、Git 状态、行操作 |
| 项目详情 | `/projects/:projectId` | Git 状态、元数据、项目任务、新建任务 |
| 任务 | `/tasks` | 项目/状态/搜索筛选、任务表格、新建任务 |
| 任务详情 | `/tasks/:taskId` | StateTrack、目标、验收条件、完成门禁、状态操作、StateSpine、元数据 |
| 本地数据与健康 | `/diagnostics` | 健康摘要 + Phase 1 探针 |
| 设置 | `/settings` | 外观、关于、本地数据与隐私 |
| 404 | `*` | 统一空状态 |

### 7.2 Design System

- 已调用 `frontend-design` Skill（当前主会话，未使用子 Agent）。
- `docs/design/DESIGN_SYSTEM.md`：品牌定位、冷石墨 + 氧化青 + 黄铜色板、Typography/Spacing/Radius/Shadow/Motion/Z-index Tokens、App Shell、组件、状态表达、页面规范、无障碍。
- 共享组件：Button/Input/Textarea/Select/Dialog/DropdownMenu/Tabs/Table/Tooltip/Toast/Card/Badge/StatusBadge/StateTrack/StateSpine/EmptyState/LoadingState/ErrorState/ConfirmationDialog。
- 状态使用图标 + 中文 + 颜色；未来 PASS/FAIL/NOT_RUN/INCONCLUSIVE/STALE 已预留 Token/组件。
- App Shell：252px 侧栏、56px 顶栏、1240px 内容最大宽度；当前项目切换与只读 Git 状态在顶部/侧栏。
- 主题：浅色/深色/跟随系统；Zustand 持久化；原生 radio 与键盘焦点。
- 不展示假数据；空状态明确邀请下一步操作。
- **视觉验收未完成**：`docs/design/UI_AUDIT.md` 标记 `VISUAL_REVIEW_PENDING`，等待用户手动截图。

### 7.3 关键交互

- 导入项目：Dialog 输入路径/名称，说明只读 Git；成功后设为当前项目。
- 项目行操作：Dropdown → 设为当前/重命名/归档/恢复；归档需确认。
- 新建/编辑任务：RHF + Zod；失败保留表单错误与 Toast。
- 验收条件：新增/编辑/删除；自动条件提示 Phase 3 限制；删除需确认。
- 状态操作：仅展示后端 `allowedTransitions`；blocked/cancelled 必须填写原因；done 在 `canComplete=false` 时禁用并显示 blockers。
- 人工确认：仅待验证状态显示；Dialog 说明确认绑定修订；成功后刷新门禁。
- 归档/恢复：软归档有确认；任务详情区分只读/终态。

## 8. 实际测试命令与退出码

最终复跑（2026-11-06）：

| 命令 | 退出码 | 结果 |
|---|---|---|
| `pnpm install --frozen-lockfile` | 0 | 锁文件一致 |
| `pnpm typecheck` | 0 | `tsc --noEmit` 通过 |
| `pnpm lint` | 0 | ESLint 10，`--max-warnings 0` |
| `pnpm test` | 0 | 7 个测试文件、39 个测试通过 |
| `pnpm build` | 0 | Vite 构建通过；JS 719.50 kB / gzip 220.27 kB |
| `pnpm format` | 0 | Prettier 通过 |
| `cargo check --workspace --all-targets --locked` | 0 | Rust workspace 通过 |
| `cargo fmt --all -- --check` | 0 | rustfmt 通过 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | 无警告 |
| `cargo test --workspace` | 0 | 4 lib + 19 业务 + 12 持久化测试 |
| `pnpm tauri info` | 0 | 环境/CSP/依赖识别正确 |
| 桌面业务流程 E2E | 0 | 导入/建任务/验收/状态/确认/完成/重启全部通过 |

开发中发现并修复的真实问题（最终均复跑通过）：

1. `DropdownMenuSeparator` 常量与组件同名重复；
2. Zod 表单 schema 结构错误导致 RHF resolver 类型不匹配；
3. 展示元数据放在组件文件触发 react-refresh 警告，拆到 `lib/task-display.ts`；
4. 工作台/项目页测试的异步查询与重复元素断言需按真实 UI 调整；
5. 项目页当前项目标记改为使用后端/选中态派生值；
6. 验收条件创建按钮文案与“添加条件”冲突，改为“创建条件”以便清晰操作与自动化。

**NOT_RUN：**

- `pnpm tauri build` / Windows 安装包；
- 生产 CSP 下的成品运行验证；
- macOS 实机；
- 自动验收命令执行与证据（Phase 3）；
- Claude Code Hooks（Phase 4）。

## 9. SQLite 持久化与事务测试结果

### Rust 测试

- **4 个 lib 测试**：状态机图（7×7）、终态、done 来源等。
- **19 个业务集成测试**：
  - 真实 Git 导入与只读状态；
  - 重复导入（含子目录）；
  - 缺失/非 Git/非目录路径；
  - unborn 与 detached HEAD；
  - 项目重命名/归档/恢复并保留任务；
  - 任务创建/编辑/非法输入；
  - 乐观并发冲突；
  - 非法状态转换；
  - ready 必需条件；
  - 自动条件阻止 done；
  - 人工确认流程；
  - 确认只能在待验证；
  - 任务/条件编辑使确认失效；
  - 离开待验证使确认失效；
  - 阻塞原因；
  - 取消与显式重新开启；
  - 验收条件 CRUD 与 revision；
  - 任务归档/恢复；
  - 搜索/状态筛选；
  - 工作台真实计数。
- **12 个持久化测试**：WAL、外键、迁移、失败回滚、幂等、schema 降级、Probe 回归等。

### 事务

- 项目/任务/验收/确认/状态转换全部在 Rust 事务中执行；
- 旧编辑/旧状态转换返回冲突，不写入；
- 迁移失败整段回滚；
- 人工确认唯一约束防止重复；
- E2E 后独立只读 SQLite 查询 integrity=`ok`、journal=`wal`、schema=2。

## 10. 桌面程序实际运行结果

- 使用已编译 `target/debug/xushu-desktop.exe` + Vite 开发服务器启动；
- 真实窗口「序枢 XUSHU」，1500×1000，WebView2 156.0.4314.58；
- E2E 第一进程 PID 17144：
  - 导入项目 `Phase2 E2E 项目 20261106010141`；
  - 创建任务 `E2E 任务 20261106010141`；
  - 添加必需人工条件 `人工确认 E2E 结果 20261106010141`；
  - draft → ready → running → awaiting_verification → 人工确认 → done；
- 完全关闭应用；
- 重启第二进程 PID 34716：
  - 任务仍存在，状态 done；
  - 验收条件仍在，且已确认；
  - `completionSatisfied=true`；
  - `statusDone=true`、`confirmed=true`；
- 验证后关闭应用并停止 Vite；无 xushu-desktop 进程、1420/1421/9222 无监听；
- 未截图/录屏。

E2E 证据：

- `C:\Users\kmld\AppData\Local\Temp\xushu-phase2-e2e\business-e2e-write.json`
- `C:\Users\kmld\AppData\Local\Temp\xushu-phase2-e2e\business-e2e-evidence.json`

证据中的 `checks`：

```json
{
  "taskTitlePresent": true,
  "criterionPresent": true,
  "doneLabelCount": 3,
  "statusDone": true,
  "confirmed": true,
  "completionSatisfied": true
}
```

E2E 使用的测试项目、任务与确认仍保留在本地数据库中，名称包含运行 tag
`20261106010141`；临时 Git 仓库位于
`%TEMP%\xushu-phase2-e2e\repo-20261106010141`，未删除任何用户数据。测试项目可在 UI
中归档/恢复。

## 11. 安全与权限检查

- 未读取/输出 API Key、`.env`、SSH 私钥、token；`find` 敏感文件扫描为空。
- 未修改全局 Claude Code / DeepSeek / Git / Shell / 注册表配置。
- 未执行管理员安装；未安装系统软件。
- 未安装 Claude Code Hooks；未注册系统服务；未修改其他项目。
- 未使用子 Agent / 多 Agent；未截图/录屏。
- Tauri capabilities 仅 `core:default`；没有 Shell/FS/SQL 插件权限。
- Git 只读、参数数组、5 秒超时、无 shell 拼接；不读取源码内容。
- 项目移除采用软归档，不删除磁盘文件；归档/删除条件有确认。
- 数据库位于系统应用数据目录，不入 Git；`.gitignore` 覆盖 DB/WAL/日志/密钥/产物。
- IPC 错误不返回 SQL、绝对路径或凭据。
- 完成门禁在 Rust；前端不能通过按钮或状态文本绕过。
- Phase 2 变更未提交、未推送；Phase 1 基线为本地提交。

## 12. 遗留风险与未验证事项

1. **必需自动条件在 Phase 3 前无法完成**：这是有意的安全策略，不是 bug；UI 明确说明。
2. **人工确认只能在待验证状态**；编辑/返工/重新开启会使确认失效，需要重新确认。
3. **生产 CSP 未用 `tauri build` 成品运行验证**；仅开发环境与配置检查。
4. **未构建安装包**；未做干净环境安装/卸载。
5. **macOS 未验证**；路径 key 的小写规范化按 Windows 设计，macOS 需重新评估。
6. **E2E 为 Windows UI Automation**，不是跨平台自动化套件；临时脚本未入仓。
7. **前端单 chunk 719.50 kB**（gzip 220.27 kB），Vite 提示 >500 kB；后续可路由级拆包。
8. **SQLite 单连接串行化**；当前单用户桌面场景足够，长查询/并发写入需后续评估。
9. **项目导入使用路径输入框**，未引入原生文件夹选择器以保持最小权限；路径粘贴/输入是当前方式。
10. **软归档不释放路径去重**：同一根目录归档后需恢复，不能重新导入。
11. **无硬删除/保留策略**：项目、任务、条件与历史会持续保留；后续需设计导出/清理。
12. **视觉验收未完成**：`VISUAL_REVIEW_PENDING`，需用户手动截图确认深浅主题、窗口缩放、空/错误/待验证状态。
13. 项目列表对每个项目执行 Git 状态查询；大量项目时需优化批量/懒加载。

## 13. Phase 2 最终验收

| 验收目标 | 结果 | 证据 |
|---|---|---|
| 导入本地 Git 项目 | PASS | 真实临时仓库导入；分支/HEAD/status 测试 |
| 重复/缺失/非 Git 路径处理 | PASS | 19 个业务测试 |
| 项目重命名/归档/恢复 | PASS | 测试 + UI 操作 |
| 创建/编辑/筛选/搜索任务 | PASS | 业务测试 + 页面测试 |
| 验收条件 CRUD | PASS | 业务测试 + UI |
| 状态机合法/非法转换 | PASS | 7×7 测试 + 业务测试 |
| 必需条件门禁 | PASS | 手动/自动条件测试 |
| 自动条件不伪造 PASS | PASS | `TASK_NOT_COMPLETABLE`，自动条件永不满足 |
| 人工确认与失效规则 | PASS | 编辑/返工/重开测试 |
| 乐观并发 | PASS | 旧 revision 返回 CONFLICT |
| 事务与回滚 | PASS | 持久化/迁移/业务测试 |
| 迁移 2 / 保留 Probe | PASS | schema 2、12 个持久化测试 |
| Tauri IPC 权限 | PASS | capabilities `core:default`，无 Shell/FS/SQL |
| 前端统一设计系统 | PASS（视觉待人工） | DESIGN_SYSTEM + UI_AUDIT；`VISUAL_REVIEW_PENDING` |
| 全部质量门禁 | PASS | 10 条命令退出码 0 |
| 桌面业务流程 E2E | PASS | 导入 → 任务 → 验收 → 完成 → 重启读取 |
| 不提前进入 Phase 3 | PASS | 无验证命令执行/自动证据/Hooks |

**Phase 2 最终状态：PASS。**

该 PASS 覆盖项目/任务/验收/状态机、SQLite 持久化、受控 IPC、统一 UI 与真实桌面 E2E；**不覆盖**自动验收引擎、安装包、生产 CSP 成品、macOS 与人工视觉验收。

## 14. Phase 3 建议起点

Phase 3 的核心是**真实自动验收证据**，不是 UI 装饰：

1. 在 `xushu-core` 定义验证模板/运行输入：可执行文件 + 参数数组 + 显式 cwd + 超时 + 输出上限；禁止 shell 字符串。
2. 用户显式批准后执行；记录退出码、耗时、stdout/stderr 脱敏摘要、Git HEAD 与工作区 dirty 状态。
3. 新增 `verification_runs` / `verification_checks` / `evidence` 迁移与 Rust 服务；把自动验收条件与当前任务 revision 关联。
4. 只有真实验证通过才能让自动条件满足；失败/未运行/无法判定/过期都不得变成 PASS。
5. 源码/验收条件/任务 revision 变化后，旧证据标为 stale；任务完成门禁读取有效证据。
6. UI 增加验收中心：命令预览、风险提示、用户批准、运行状态、取消、结果与证据链接。
7. 保留 Phase 2 的人工确认模型；自动证据与人工确认分别来源、分别展示，不混淆。
8. 完成后再评估 Hooks（Phase 4）；不要提前接入。

当前不自动进入 Phase 3，等待用户确认。

## 15. 标准汇报摘要

```text
当前阶段 / 任务：Phase 2 — 项目管理、任务系统与状态机
实施摘要：新增迁移 2 与 Project/Task/Acceptance/Transition/ManualConfirmation 领域；Rust 状态机与完成门禁、只读 Git 检查、21 个业务 IPC；统一设计系统与工作台/项目/任务/详情/诊断/设置页面；完成真实桌面业务 E2E。
修改文件：见第 5 节；Phase 1 基线 85ba249，Phase 2 变更未提交。
实际执行的命令与退出码：见第 8 节；10 条质量门禁均 0。
测试：PASS — 前端 39 测试、Rust 35 测试、桌面 E2E（导入/任务/验收/完成/重启）；NOT_RUN — tauri build 安装包、生产 CSP 成品、macOS、Phase 3 自动验证、Hooks。
安全/隐私检查：未读取/输出密钥；未修改全局配置；未管理员安装；未安装 Hooks；未使用子 Agent；未截图；capabilities 仅 core:default；Git 只读。
明确未完成或尚未验证：自动验收引擎、安装包、生产 CSP、macOS、人工视觉验收。
下一步：等待用户审查 Phase 2 与 UI 截图；确认后再规划 Phase 3。
```

## 16. 结论

Phase 2 已在 2026-11-06 实际完成并通过验收。XUSHU 能管理本地 Git 项目、任务、验收条件、人工确认与状态历史；完成必须经过 Rust 门禁，自动条件在 Phase 3 前不会被伪造为通过。全部可用质量门禁与真实桌面 E2E 均通过。

UI 自动化与业务 E2E 通过不等于视觉验收通过；在用户提供截图前，视觉状态保持 `VISUAL_REVIEW_PENDING`。

**停止于 Phase 2，等待用户确认是否进入 Phase 3。**