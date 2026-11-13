# Phase 5 报告：长期记忆、开发决策与跨会话恢复

- 日期：2026-11-12
- 项目：序枢 XUSHU
- 根目录：`D:\MyProject\xushu`
- 起始基线：`1bca890e65d9f23880dc451822aaa763b1106447`（`main`）
- 总体状态：**代码、迁移、受限 IPC、前端、自动化与隔离桌面 E2E PASS；人工视觉与产品化验收 PENDING。**
- 结论：XUSHU 现在可以在项目级保存可修订的长期记忆与追加式开发决策，并从真实数据库、只读 Git 与独立验收结果确定性生成跨会话恢复包。记忆、决策、Hook 事件、恢复包与 AI 文本都不能替代任务完成门禁。

## 1. Phase 5 已实现功能

### 1.1 项目长期记忆

- 类别：项目目标、开发约束、技术架构决策、开发规范、已确认事实、风险与已知问题、后续待处理、项目知识。
- 每条记忆包含：UUID、项目、类别、标题、内容、来源类型、确认状态、有效性、关联任务/证据、修订号、创建/更新/确认/归档时间。
- 支持创建、查看、编辑、确认、归档、恢复；每次变更写入 `project_memory_revisions`，旧版本可追溯。
- 已归档记忆默认不进入列表与恢复包；未确认记忆不进入“已确认”上下文。
- AI 推断来源默认只能保存为未确认，且禁止直接以已确认状态创建。
- 所有读写都要求项目 ID，并校验任务/证据同项目归属；跨项目访问返回 `NOT_FOUND` 或校验错误。

### 1.2 开发决策日志

- 记录问题、背景与约束、备选方案、最终方案、采用原因、已知代价与风险、关联任务、状态、取代关系与时间。
- 状态：草稿 → 已确认 → 已被取代 / 已废弃。
- 草稿可编辑；已确认决策不可原地改写。需要变化时创建新决策，确认时在同一事务内把旧决策标记为已被取代，并双向保留引用。
- 每次状态/内容变化写入 `decision_revisions`；废弃必须填写原因。
- 决策操作不修改任务状态、验收条件、证据或完成门禁。

### 1.3 跨会话恢复包

- 确定性生成，不依赖 AI Provider。
- 包含：项目身份、实时 Git 分支/HEAD/dirty/代码指纹、任务状态与 blockers、独立验证结果与过期原因、已导入的 Claude Code 元数据、已确认记忆/决策、未确认/已取代提示、下一步、建议读取文件、结构化冲突与缺失信息、数据来源与排除项。
- Git 读取失败显示 `UNKNOWN`，不使用旧数据冒充实时状态。
- 快照保存生成时间、指纹、Markdown 与结构化包 JSON；最近 20 份保留。
- `get_recovery_overview` 重新构建当前包并比较指纹，返回 fresh / stale / no_snapshot。
- 历史快照与过期快照在 UI 中明确区分；复制时剪贴板文本开头加入对应警告。

### 1.4 用户入口与工作流联动

- 新增一级导航“恢复”。
- 项目详情新增“项目记忆与决策”strip；项目子页面 `/projects/:id/knowledge` 管理记忆与决策。
- 任务详情新增“相关决策与记忆”，可按任务过滤。
- 工作台“下一步”焦点面板增加“查看恢复上下文”。
- 恢复页支持预览完整指令、复制、重新生成、检查过期、查看历史快照、指定/清除下一步任务。
- 项目归档后记忆/决策只读；恢复包仍可生成用于只读交接。

## 2. 技术架构及数据流

```text
React 页面
  │ invoke（强类型参数；前端 Zod 仅改善体验）
  ▼
Tauri Commands（knowledge_commands / recovery_commands）
  │ 项目 ID、输入与归属校验
  ▼
Rust 领域服务
  ├── MemoryService ── SQLite（记忆 + 修订，事务）
  ├── DecisionService ─ SQLite（决策 + 修订 + 取代，事务）
  └── RecoveryService ─ 读取项目/任务/验证/Claude/记忆/决策
                         + 只读 Git / code_state
                         + 确定性 Markdown + 指纹
                         + 快照插入与保留裁剪（事务）
```

来源优先级：

1. 真实任务状态、验收证据与 Git 状态。
2. 已确认记忆与已确认决策（开发上下文）。
3. 未确认记忆、草稿决策、系统建议（明确标记，不作为事实）。
4. Hook 事件、AI 文本（观察/辅助，不是完成证据）。

恢复页与 Markdown 都明确写出该优先级；结构化冲突进入 `conflicts`，缺失信息进入 `missing_information`。

## 3. 数据库迁移与新增表

迁移：`crates/xushu-core/migrations/0006_project_memory_and_recovery.sql`（版本 6）。

| 表 | 用途 |
|---|---|
| `project_memories` | 当前记忆版本、来源、确认、有效性、关联任务/证据、修订 |
| `project_memory_revisions` | 每次记忆变更的不可变快照 |
| `decisions` | 当前决策、状态、取代关系、修订 |
| `decision_revisions` | 每次决策变更的不可变快照 |
| `project_recovery_preferences` | 用户指定的下一步任务（每项目一行） |
| `recovery_snapshots` | 恢复包 Markdown、结构化 JSON、指纹与计数 |

约束与策略：

- UUID、UTC RFC3339、外键、CHECK、事务。
- 记忆确认状态与 `confirmed_at` 一致性；有效性归档与 `archived_at` 一致性。
- 决策 `draft/confirmed/superseded/deprecated` 状态机约束；`superseded` 必须有 `superseded_by_id`；已确认决策保留 `confirmed_at`。
- 记忆/决策修订表 `(memory_id, revision)` / `(decision_id, revision)` 唯一。
- 索引覆盖项目 + 更新时间、项目 + 状态/类别、关联任务、取代关系、快照项目 + 生成时间。
- 恢复快照每项目保留最近 20 份；长文本在包内最多 4000 字符；Markdown ≤ 4 MiB、结构化包 ≤ 8 MiB。
- 迁移 5→6 为纯追加表；测试验证旧项目/任务保留。

## 4. 新增 Rust 领域服务

| 文件 | 内容 |
|---|---|
| `crates/xushu-core/src/memory.rs` | `MemoryService`、类别/来源/确认/有效性枚举、输入/视图/修订类型、项目隔离与事务 |
| `crates/xushu-core/src/decision.rs` | `DecisionService`、状态机、取代事务、修订历史、项目隔离 |
| `crates/xushu-core/src/recovery.rs` | `RecoveryService`、确定性包、Git/代码指纹、验证过期、Claude 元数据、下一步、冲突/缺失信息、快照与新鲜度 |
| `crates/xushu-core/src/task.rs` | 新增 `recovery_candidates`；将 `criteria_with_completion` 复用于恢复评估（未改变门禁语义） |
| `crates/xushu-core/src/error.rs` | 新增 `RECOVERY_ERROR` 代码 |
| `crates/xushu-core/src/db/migrations.rs` | 注册迁移 6 |

## 5. 新增 Tauri IPC

| 命令 | 作用 | 写入边界 |
|---|---|---|
| `list_project_memories` | 按项目/类别/归档/任务筛选 | 只读 |
| `get_project_memory` | 记忆详情 + 修订历史 | 只读 |
| `create_project_memory` | 创建记忆 | 事务写记忆 + 修订 |
| `update_project_memory` | 乐观修订更新 | 事务写记忆 + 修订 |
| `confirm_project_memory` | 人工确认未确认记忆 | 事务写记忆 + 修订 |
| `archive_project_memory` | 归档记忆 | 事务写记忆 + 修订 |
| `restore_project_memory` | 恢复记忆 | 事务写记忆 + 修订 |
| `list_project_decisions` | 按项目/状态/任务筛选 | 只读 |
| `get_project_decision` | 决策详情 + 修订历史 | 只读 |
| `create_project_decision` | 创建草稿（可预设取代对象） | 事务写决策 + 修订 |
| `update_project_decision` | 仅草稿可更新 | 事务写决策 + 修订 |
| `confirm_project_decision` | 确认草稿；必要时取代旧决策 | 单事务更新两个决策 + 两条修订 |
| `deprecate_project_decision` | 废弃草稿/已确认决策 | 事务写决策 + 修订 |
| `get_recovery_overview` | 最近快照 + 当前新鲜度 | 只读（重新计算） |
| `generate_recovery_package` | 生成并保存快照 | 事务插入快照 + 保留裁剪 |
| `list_recovery_snapshots` | 最近 20 份快照摘要 | 只读 |
| `get_recovery_snapshot` | 按项目 + 快照 ID 读取 | 只读 |
| `set_recovery_next_task` | 指定/清除下一步任务 | 事务 UPSERT 偏好 |

未新增 capabilities；前端仍无通用 SQL、文件系统写入或 shell 权限。IPC 不返回 Claude 原始 JSON、Prompt、transcript、密钥或数据库绝对路径。

## 6. 前端新增页面与组件

- 路由：`/projects/:projectId/knowledge`、`/recovery`。
- 页面：`project-knowledge-page.tsx`、`recovery-page.tsx`。
- 组件：
  - `features/knowledge/`：API/queries、Zod schema、类别/状态显示、Badge、记忆/决策 Form Dialog、Detail Sheet、项目摘要 strip、任务关联 strip。
  - `features/recovery/`：API/queries、Badge、显示映射、剪贴板文本构建、完整指令 Sheet、历史快照 Sheet。
- 导航：侧栏新增“恢复”；顶部工作带页面名称支持“恢复”“记忆与决策”；工作台增加恢复入口。
- 设计系统：复用现有 Tokens、Section、Dialog、Sheet、EmptyState、LoadingState、ErrorState、Alert、ConfirmationDialog；`SheetFooter` 增加 `flex-wrap`。
- 未新增一级菜单承载每个子功能；记忆/决策作为项目上下文子页面。

## 7. 长期记忆的真实性与来源控制

- `source_type` 是来源标签，不是自动可信声明；恢复包只纳入“已确认 + 有效”的记忆。
- 用户通过 UI 创建的记忆标记为 `user` 且默认已确认；AI 推断必须保持未确认并由人工确认。
- 未确认记忆在恢复页单独展示为“不是事实”，不得作为权威上下文。
- 记忆关联任务/证据时校验同项目归属；不读取任务文件或证据正文之外的敏感内容。
- 记忆编辑保留修订历史；归档不删除；恢复不重写历史。
- 恢复包 Markdown 明确“记忆与决策不能替代任务状态、验收证据或完成门禁”。

## 8. 决策历史及有效性管理

- 已确认决策不可原地编辑；测试覆盖“直接编辑已确认决策被拒绝”。
- 取代流程：新决策草稿可指向已确认旧决策；确认新决策的事务内同时写旧决策 `superseded` + `superseded_by_id` 与新决策 `supersedes_id`。
- 已被取代/废弃决策保留原 `confirmed_at` 与修订历史；旧决策不能被再次确认或再次取代。
- 废弃必须填写原因；草稿与已确认决策均可废弃，已取代决策不可废弃。
- 决策操作前后测试断言任务状态与修订不变。
- 恢复包只纳入已确认决策；草稿、取代/废弃提示分开呈现。

## 9. 恢复包生成机制

1. 校验项目 ID 并读取项目。
2. 只读采集 Git 分支、HEAD、dirty 与工作区代码指纹；失败记录 `UNKNOWN` 与原因。
3. 在数据库锁内读取任务候选、评估当前验收 blockers、验证运行、Claude 元数据、记忆、决策、下一步偏好与计数。
4. 只检查白名单协作文件是否存在（不读取内容），并列出最近 2 份 `docs/phases` 报告文件名。
5. 构建确定性结构化包，计算来源指纹（排序后的任务/记忆/决策/验证/Claude/文件状态 + Git 状态）。
6. 生成 Markdown；过长文本在包内截断到 4000 字符并记录上限说明。
7. 事务插入快照并按项目保留最近 20 份。
8. `get_recovery_overview` 重新构建当前包；指纹相同为 fresh，不同为 stale 并列出变化类别。

恢复包不会把“未观察到失败事件”当作成功；Claude 会话结束未确认会进入缺失信息。验证结果按生成时任务修订与代码状态计算，过期显示 STALE。

## 10. Claude Code 交接流程

1. 用户在恢复页查看摘要。
2. 可打开“完整恢复指令”Sheet，查看包含本地路径、任务/验证状态与已确认记忆的完整 Markdown。
3. 用户主动点击“复制恢复指令”；若快照过期，先二次确认。
4. 剪贴板文本为生成时 Markdown；过期/历史快照在开头加入警告。
5. 用户自行在新 Claude Code 会话中粘贴；XUSHU 不自动启动会话、不写全局配置、不注入 Prompt。
6. 恢复包明确声明“不是 Claude Code 自身会话记忆”；新会话仍需读取建议文件并重新核验。

## 11. 安全与隐私边界

- 所有记忆/决策/快照命令要求项目 ID 并校验归属；跨项目读取/写入被拒绝。
- 不开放通用 SQL、FS 写入或 shell；业务校验在 Rust。
- 恢复包不读取 Claude Code transcript、Prompt、工具输入输出、密钥；只使用 Phase 4 已导入的元数据。
- 恢复包不扫描项目文件内容；只检查白名单文件是否存在。
- 恢复包包含本地路径与用户记忆，UI 在预览/复制处提示；不会自动上传。
- 长文本与快照大小有上限；快照每项目保留 20 份。
- 测试与桌面 E2E 使用隔离数据库/目录；正常数据库指纹不变。
- 未发现真实密钥被写入代码、测试夹具、日志或报告；测试中的 `API_KEY=...`/`PROMPT...` 为合成的脱敏测试样本。

## 12. 质量检查和测试数量

| 命令 | 退出码 | 结果 |
|---|---|---|
| `pnpm typecheck` | 0 | `tsc --noEmit` 通过 |
| `pnpm lint` | 0 | ESLint 10，`--max-warnings 0` |
| `pnpm test` | 0 | 9 个测试文件、**66** 个测试通过 |
| `pnpm build` | 0 | Vite 通过；JS 917.84 kB / gzip 266.03 kB；仍有原有 >500 kB 提示 |
| `pnpm format` | 0 | Prettier 通过 |
| `cargo fmt --all -- --check` | 0 | rustfmt 通过 |
| `cargo check --workspace --all-targets --locked` | 0 | Rust workspace 通过 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | 无警告 |
| `cargo test --workspace --locked` | 0 | **152** 项通过 |
| `git diff --check` | 0 | 无空白错误 |

Rust 测试构成：

| 测试组 | 数量 |
|---|---|
| `xushu-core` lib | 13 |
| `tests/business.rs` | 19 |
| `tests/claude.rs` | 37 |
| `tests/decisions.rs`（新增） | 8 |
| `tests/memory.rs`（新增） | 8 |
| `tests/persistence.rs`（含迁移 5→6） | 13 |
| `tests/recovery.rs`（新增） | 13 |
| `tests/verification.rs` | 26 |
| `xushu-desktop` lib | 2 |
| `xushu-hook` CLI | 8 |
| `xushu-hook-protocol` lib | 5 |
| **合计** | **152** |

Phase 5 覆盖：创建/修改/确认/归档/恢复/修订历史、来源与未确认规则、项目隔离、输入上限、事务回滚、决策状态与取代、废弃、任务状态隔离、恢复包完整生成、Git 失败、无任务、验证过期、冲突、Claude 结束未确认、下一步偏好、快照隔离/保留/重开、迁移保留、快照插入失败回滚。

## 13. Windows Tauri E2E 结果

- 命令：`python scripts/desktop-e2e.py --scenario target/phase5-audit/phase5-e2e.mjs`
- 退出码：0；输出：`PASS: project/task in isolated SQLite, normal database unchanged, run directory removed`
- 环境：Windows 11、Tauri 2.12.5、WebView2、隔离 app 数据目录与隔离 SQLite；无截图/录屏。
- 实测：
  - 创建记忆、创建并确认决策、指定下一步、生成恢复包。
  - 恢复包包含已确认记忆/决策、用户指定的下一步、Markdown 与指纹。
  - 更新记忆后恢复包变 `stale`；重新生成后变 `fresh`。
  - 记忆/决策/恢复操作前后任务仍为 `draft`、revision 1。
  - 知识页记忆标题、决策标题、恢复页“下一步”与更新后记忆均可见。
  - 点击记忆/决策标题可打开真实详情 Sheet（真实调用 `get_project_memory` / `get_project_decision`）。
  - 快照保留 2 份；正常数据库指纹前后一致；隔离 SQLite `integrity_check=ok`。
- 证据（已忽略目录）：`target/phase5-audit/phase5-e2e-result.json`
  - 指纹：`sha256:82151b528a2cef9216eca1f5a61e1b2a787f068b87b2ebbdc76d2b25fe73546e`
  - Markdown 长度：2387；freshness：fresh → stale → fresh。
- 边界：本 E2E 通过真实 IPC 建立记忆/决策/快照并断言页面渲染；表单 Dialog 的完整桌面点击路径由 Vitest 覆盖，未在 E2E 中逐步填写，人工验收应补测。
- 这是开发构建 + 隔离数据目录的桌面 E2E，不等于生产安装包验收。

### 13.1 布局测量（无截图）

- 命令：`python scripts/desktop-e2e.py --scenario target/phase5-audit/phase5-layout.mjs`
- 退出码：0；证据：`target/phase5-audit/phase5-layout.json`。
- 视口：500×480、1366×768、1600×900；页面：知识页（记忆/决策）与恢复页，共 9 组。
- 结果：
  - 顶部工作带均为 **44px**；<760px 侧栏为 **52px**，≥760px 为 **240px**。
  - `document` 与 `main` 的 `scrollWidth == clientWidth`，无页面级或主区横向溢出。
  - 可见按钮宽度均 ≥20px；未发现不可访问的极小操作。
  - 500×480 下知识页/恢复页按钮换行、主内容宽度 438px；1366/1600 下主内容宽度 1116/1350px。
- 该测量不代替人工视觉截图；深浅主题、实际窗口 DPI 与焦点行为仍需用户复核。

## 14. 发现并修复的问题

1. **迁移 6 决策 CHECK 约束过严**：不允许“已确认决策废弃后保留 `confirmed_at`”。在迁移未发布前修正约束并补测。
2. **恢复日志成功但复制未触发**：测试发现 jsdom 已提供 `navigator.clipboard.writeText`，原 `Object.defineProperty` 未覆盖；改用 `vi.spyOn`，并验证复制文本精确。
3. **表单 reset 的 React Hook 依赖告警**：改用 `useMemo` 初始化值，避免对话框打开时丢失输入或引入 lint 抑制。
4. **建议下一步无法一键确认**：原按钮在选中建议任务时被禁用；改为仅当该任务已是“用户指定”时禁用。
5. **窄窗口 Sheet Footer 可能溢出**：共享 `SheetFooter` 增加 `flex-wrap`。
6. **恢复包长文本/快照容量无上限**：增加 4000 字符包内截断与 4 MiB / 8 MiB 快照上限，并在来源说明中披露。
7. **桌面 E2E 首次断言写错**：UI 检查仍在找更新前标题；修正后 PASS。
8. **任务/项目/验证变更未刷新恢复查询**：相关 mutation 增加 `["recovery"]` 失效；恢复 overview 改为进入页面即重新计算。

## 15. 未完成或未验证事项

- **人工视觉复核 PENDING**：知识页、恢复页、Dialog/Sheet、长文本、500×480、1366×768、1600×900、深浅主题、150% 缩放与键盘焦点均需用户手动截图/操作确认；未自动截图。
- **Phase 4 人工视觉仍 PENDING**：本轮没有把 Phase 4 的视觉、Snap 菜单或真实 Claude Code 新会话改为 PASS。
- **生产安装包与 sidecar**：未验证 `tauri build` 成品、`xushu-hook` sidecar 打包与升级路径（Phase 7）。
- **macOS/Linux**：未构建、未实机验证。
- **Windows Snap 实际悬停菜单**：历史 NOT_RUN，仍需人工确认。
- **自由文本语义冲突检测**：未实现；只呈现结构化冲突（如已完成任务证据过期）与来源优先级，不假装能自动理解任意记忆文本。
- **AI 摘要**：未引入；恢复包完全确定性。未来若增加，只能作为可选增强。
- **大数据/性能**：未对大量记忆/决策/快照做压力测试；包内已有数量与大小上限。
- **多项目桌面 UI 隔离**：Rust 测试覆盖项目隔离；桌面 E2E 只使用一个隔离项目。
- **真实 Claude Code 联调**：本轮未重新运行；沿用 Phase 4.3/真实回归历史结论，但未在本轮重新验证。

## 16. Git 工作区状态

- 分支：`main`
- 起始 HEAD：`1bca890e65d9f23880dc451822aaa763b1106447`
- 本轮**未提交、未推送、未 reset、未 clean**。
- 工作区共 41 项变更（以 `git status --short` 为准）；既有 Phase 3.5/3.6/4 未提交改动保留。
- 未跟踪的 `docs/phases/PHASE_4_REAL_HOOKS_REGRESSION.md` 原样保留，未覆盖。
- 新增/修改的主要文件：
  - 迁移：`crates/xushu-core/migrations/0006_project_memory_and_recovery.sql`
  - 领域：`crates/xushu-core/src/memory.rs`、`decision.rs`、`recovery.rs`
  - IPC：`apps/desktop/src-tauri/src/knowledge_commands.rs`、`recovery_commands.rs`
  - 前端：`features/knowledge/`、`features/recovery/`、`pages/project-knowledge-page.tsx`、`pages/recovery-page.tsx`
  - 文档：`docs/architecture/ADR/0008-project-memory-decisions-recovery.md`、本报告、`DESIGN_SYSTEM.md`、`UI_AUDIT.md`、`MASTER_DEVELOPMENT_PLAN.md`、`README.md`、`CLAUDE.md`
  - 测试：`tests/memory.rs`、`tests/decisions.rs`、`tests/recovery.rs`、`tests/persistence.rs`、`App.test.tsx`、`copy-recovery.test.ts`
- 审计证据位于已忽略的 `target/phase5-audit/`，不进入仓库。

## 17. 下一步人工验收清单

1. 使用 `pnpm tauri dev`（或现有开发启动方式）打开桌面应用，选择测试项目。
2. 项目详情确认“项目记忆与决策”strip、Claude 接入 strip、仓库 strip 的层级与操作位置。
3. 进入“记忆与决策”：新建已确认记忆、编辑、查看修订、归档/恢复；新建 AI 推断（如通过测试接口）确认保持未确认。
4. 新建决策草稿、编辑、确认；再创建取代决策并确认，检查双向取代提示与旧决策历史。
5. 进入“恢复”：生成恢复包；检查恢复带、下一步、Git、任务、验证、Claude、记忆、决策、文件、冲突与来源 details。
6. 打开完整恢复指令 Sheet，检查深色 Markdown、本地路径警告、复制内容；更新一条记忆后重新打开恢复页，确认“已过期”与复制前二次确认。
7. 打开历史快照，确认“历史”标记与“这是历史快照”警告。
8. 在 500×480、1366×768、1600×900 与深浅主题下检查知识页、恢复页、Dialog/Sheet、按钮换行与滚动。
9. 用键盘完成：Tab 到新建/查看/确认/复制/历史、Escape 关闭、关闭后焦点返回。
10. 人工视觉通过后，再更新 `UI_AUDIT.md` 的 PENDING 项；本轮不代替用户宣布视觉验收通过。

---

**明确结论：** Phase 5 的代码、迁移、IPC、前端、自动化与隔离桌面 E2E 已完成并记录；**人工视觉与生产化验收尚未完成**，Phase 5 整体验收保持 **PENDING**，不得解释为已全部通过。