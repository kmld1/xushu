# Phase 6 报告：安全、隐私保护、数据备份与灾难恢复

- 日期：2026-11-13
- 项目：序枢 XUSHU
- 根目录：`D:\MyProject\xushu`
- 起始基线：`06899e9986ee6983a623097235429c53fd0518f8`（`main`）
- 总体状态：**代码、迁移、受限 IPC、前端、自动化与隔离 Windows Tauri/WebView2 E2E PASS；人工视觉与产品化验收 PENDING。**
- 结论：XUSHU 现在可以创建经过清单与 SQLite 完整性验证的一致性备份；可以在隔离目录中验证恢复；可以在明确确认后受控切换数据库，并在失败或崩溃后优先回退。Hook Spool、伪名化 Salt、项目 Hooks 配置与导出报告不在备份内，恢复不会自动修改 Claude Code 配置，也不会改变任务完成门禁。

## 1. Phase 6 实际实现范围

| 能力 | 实现 |
|---|---|
| 数据资产与隐私审计 | 迁移 7 登记、运行时资产清单、Hook 采集边界、保留与丢失影响；见 `docs/architecture/DATA_PROTECTION.md` |
| SQLite 一致性备份 | `rusqlite` Online Backup API；WAL 合并；单文件快照；`.partial-*` 临时目录 + 同卷原子改名 |
| Manifest 与版本 | 格式 v1；严格 JSON；备份 UUID；应用/Schema 版本；单文件相对路径、大小、SHA-256；拒绝未知字段与未来 Schema |
| 备份校验 | 清单、SHA-256、在隔离副本中打开、迁移、SQLite 完整性、外键、项目/任务/验收/验证/证据/Claude/记忆/决策/快照关系 |
| 灾难恢复 | 只读预检 → 隔离恢复 → 一次性 token + 输入“恢复” + 勾选 → 回退副本 → 受控切换 → 最终校验 |
| 失败回退 | 原库/WAL/SHM 移入 `restore-rollback`；恢复日志；启动时自动回退；回退失败明确 `needs_manual` |
| 自动备份策略 | 默认关闭；目标、间隔、保留数量；仅应用运行期间检查；只清理 automatic；失败保留旧备份 |
| 隐私与数据控制 | 数据资产页、Hook 采集/排除字段、Spool/Salt 边界、保留策略、完整性检查、应用数据目录复制 |
| 备份管理 | 创建、校验、历史、删除（确认 + 清单/哈希复验）、旧自动备份预览与清理 |
| 受限 Tauri IPC | 15 个自定义命令；无通用 SQL/Shell/FS 权限；无新 capability |
| UI | 一级“数据保护”页、创建备份 Dialog、备份策略 Dialog、校验 Sheet、7 步恢复向导、恢复历史、完整性报告、隐私资产清单 |
| 文档 | ADR-0009、DATA_PROTECTION.md、本报告；更新总计划、README、CLAUDE.md、DESIGN_SYSTEM、UI_AUDIT、DEPENDENCIES、CLAUDE_HOOKS |

## 1.1 状态标签

| 标签 | 状态 | 证据范围 |
|---|---|---|
| `BACKUP_CREATED` | **PASS** | Rust `tests/backup.rs` + 隔离 Tauri E2E 创建真实备份 |
| `BACKUP_INTEGRITY_VERIFIED` | **PASS** | 清单、SHA-256、隔离 SQLite/业务关系校验；篡改备份被拒绝 |
| `ISOLATED_RESTORE_VERIFIED` | **PASS** | 预检在隔离目录执行迁移、完整性与关系检查 |
| `RESTORE_SWITCH_VERIFIED` | **PASS** | 隔离 Tauri E2E 正式切换并最终校验通过 |
| `ROLLBACK_VERIFIED` | **PASS（Rust 自动化）** | 切换后损坏 → 回退；启动恢复未完成切换；真实 Tauri 故障注入 `NOT_RUN` |
| `DATA_PRIVACY_VERIFIED` | **PASS（自动化范围）** | 备份范围排除 Spool/Salt/项目配置；无凭据模式命中；人工隐私/视觉复核仍 PENDING |
| `E2E_VERIFIED` | **PASS（开发构建 + 隔离数据目录）** | 完整恢复、损坏拒绝、关闭重开、布局主题 |
| `VISUAL_REVIEW_PENDING` | **PENDING** | 未自动截图；等待用户手动签收 |
| `PRODUCTION_INSTALLER` | **NOT_RUN** | Phase 7 |

## 2. 数据资产清单（摘要）

完整清单见 `docs/architecture/DATA_PROTECTION.md` 第 1 节和数据保护页“隐私与本地数据”。

- 进入备份：项目、任务、验收条件与历史、验证运行/检查/证据摘要、Claude 集成状态与事件元数据、长期记忆与修订、开发决策与修订、恢复快照、备份登记与恢复审计。
- 不进入备份：Hook Spool、伪名化 Salt、项目 `.claude/settings.local.json`、`exports/` 验收报告、WebView 本地偏好、日志。
- 备份是未加密的数据库副本；包含本地路径、用户录入内容与脱敏后的测试输出摘要，必须保存在受控位置。
- Salt 丢失不破坏历史事件读取，但新事件无法与旧会话伪名关联；Spool 未导入事件不会因恢复而恢复。

## 3. 威胁模型（摘要）

完整表见 `docs/architecture/DATA_PROTECTION.md` 第 2 节。

| 威胁 | 主要缓解 | 残余风险 |
|---|---|---|
| WAL/并发写导致备份不一致 | Online Backup API、应用互斥、SQLite 锁重试、快照完整性检查 | 外部进程绕过应用互斥时仍依赖 SQLite |
| 恶意/损坏/截断备份 | 严格清单、单文件路径、SHA-256、隔离 SQLite + 业务关系检查、容量上限 | 校验和不证明来源可信 |
| 将来 Schema 降级或旧 Schema 不兼容 | 未来版本拒绝；旧版本隔离迁移 | 无迁移路径的版本拒绝 |
| 恢复覆盖当前数据 | 只读预检、隔离验证、输入“恢复”+勾选、回退副本 | 用户确认后仍会丢失备份后的变更 |
| 切换中断 | 恢复日志、原库/WAL/SHM 保留、启动回退 | 回退也可能失败，需人工 |
| Junction/符号链接/UNC | 句柄 + 重解析点校验、绝对路径、文件不跟随链接 | 本地进程极小窗口内的 TOCTOU |
| Salt/Spool/项目配置泄露或误改 | 不进入备份、不导出、不自动修改项目配置 | 本机高权限进程可读取应用数据目录 |
| 自动清理误删手动备份 | 仅 automatic、登记匹配、清单匹配、仅已知文件 | 手动删除仍需用户确认 |
| 敏感日志 | 稳定错误码/固定文案、无路径/SQL/原文 | 本地 UI 显示路径供用户定位 |

## 4. 备份格式与版本

格式 v1，目录固定为：

```text
xushu-backup-<UTC>-<short-id>/
├── manifest.json
└── xushu.db
```

Manifest 包含：`formatVersion=1`、`product=XUSHU`、`backupId`（UUID）、`appVersion`、`schemaVersion`、`createdAt`、`origin`（manual/automatic）、`scope=database`、`encrypted=false`、`complete=true`、单文件 `path=xushu.db`、大小、SHA-256。

边界：

- 只允许一个相对路径 `xushu.db`；拒绝绝对路径、`..`、嵌套路径与重复文件。
- 清单 ≤ 256 KiB；数据文件 ≤ 2 GiB。
- 拒绝未知字段、未知格式版本、未来 Schema、未完成标记、加密标记。
- 旧 Schema（v1–7）允许在隔离副本中升级；不静默丢表/丢字段。
- 不使用可执行代码格式；不使用 ZIP 路径解析。
- API 核对：`rusqlite 0.41.1` 的 `backup` feature/`Backup` API 按本机锁定源码核对；SQLite Backup/WAL 按官方文档；Windows 文件 API 使用锁定的 `windows-sys 0.61.2`；Tauri 2.12 使用现有命令/`State`/`spawn_blocking` 模式并实际编译与 E2E。未使用联网 Context7。

## 5. 一致性备份机制

1. `BackupService::create_with_target` 锁定 `backup_mutation`，准备并校验目标目录。
2. 检查目标空间 ≥ 主库 + WAL + 8 MiB。
3. 在目标内创建 `.partial-<uuid>/`；调用 `Database::backup_to_with_cancel` 使用 SQLite Online Backup API 分页复制。
4. Busy/Locked 时重试；30 秒持续锁定时返回明确错误；支持用户取消。
5. 打开快照，`compact_for_backup` 合并 WAL 并切换到 DELETE，使备份成为单文件。
6. 执行 `DataIntegrityService`：SQLite `integrity_check`、外键、Schema、项目/任务/验收/验证/证据/Claude/记忆/决策/快照归属与修订历史。
7. 计算 SHA-256，写清单，重新读取清单并再次校验文件大小与哈希。
8. 同卷原子改名到最终目录；登记 `backup_records`；登记失败则删除刚创建的自有备份目录。
9. 失败或取消：删除 partial 目录；Windows 目录句柄在删除前释放（本轮修复）。

## 6. Manifest 与完整性验证

- `BackupService::verify` 读取登记、清单与数据文件；校验登记与清单一致、SHA-256、然后复制到应用数据目录 `verify-workspaces` 隔离打开，执行迁移与业务关系检查。
- 校验结果更新 `integrity_status`（verified/failed/unverified/missing）、`verified_at` 与简短 `last_error`。
- 未校验或校验失败的备份在 UI 中不会显示为“已验证”。
- 只有通过完整校验的备份才显示可供恢复；损坏备份在预检阶段被拒绝，不进入正式恢复。
- SHA-256 只证明内容与清单一致，不证明来源可信；UI 与文档均明确。

## 7. 隔离恢复与正式恢复

### 7.1 预检

- 只读读取清单与数据文件；校验 SHA-256、格式、Schema、目标目录与容量。
- 复制到应用数据目录 `restore-workspaces/<token>/`，再次校验副本哈希。
- 打开隔离副本，执行迁移、SQLite 完整性、外键、项目归属、验证/证据关系、记忆/决策修订与取代关系检查。
- 返回当前/备份数据计数、项目名称（最多 5 个）、缺失本地路径数量、运行中验证数量、**备份之后当前数据库的时间戳变更计数**（任务/项目/验收/验证/记忆/决策/快照/Claude 事件）、Hooks 复核提示、影响范围、警告与 15 分钟一次性 token。
- 影响范围与确认阶段明确显示“这些变更不在备份中，恢复后仅保留在回退副本”，避免用户只看到总数而不知道会丢失哪些类别的当前变更。
- 预检未通过时 `ready=false`、无 token，不能进入正式恢复。

### 7.2 明确确认

- 必须输入中文“恢复”并勾选风险确认；后端再次校验。
- 预检 token 单次使用；开始执行时作废其他预检并清理其工作区。
- 存在运行中的验证运行或应用内运行句柄时拒绝恢复。

### 7.3 正式切换

1. 重新校验清单、数据文件与隔离副本哈希。
2. 检查正式恢复峰值空间：2 × 当前数据库 + 2 × 备份数据文件 + 16 MiB。
3. 为当前数据库创建一致性回退副本，并执行完整性检查。
4. checkpoint 当前数据库，关闭连接。
5. 写恢复日志（prepared → switching）。
6. 将当前 DB/WAL/SHM 移动到 `restore-rollback/<operation>/`。
7. 复制隔离副本到正式路径，校验 SHA-256。
8. 重新打开数据库，执行最终完整性/业务关系检查。
9. 通过后记录 `restore_operations.completed`、归档日志为 completed；失败则自动回退。

## 8. 失败回退机制

- 原数据库、WAL、SHM 不删除；移动到回退目录。
- 回退时把切换后的数据库保存为 `interrupted-*-xushu.db`，再恢复原文件。
- 回退后重新执行完整性检查；通过才报告 `rolled_back`。
- 回退失败报告 `rollback_failed`，保留全部文件，提示人工检查。
- 启动时读取恢复日志：
  - prepared → 原库不动，归档 abandoned；
  - switching/switched/未完成 → 自动回退并验证；
  - completed → 校验当前数据库；失败则回退；
  - 回退失败 → needs_manual。
- 本轮通过 Rust 测试真实模拟“切换后数据库损坏 → 回退 → 重开原库”和“启动时回退未完成切换”。

## 9. Hooks、记忆与恢复边界

- 备份只包含 SQLite；Hook Spool、Salt、项目 `.claude` 配置、导出报告与 WebView 偏好不在备份内。
- 恢复数据库不会自动安装、停用、启用或卸载任何 Claude Code Hooks；预检与结果均提示人工复核。
- 恢复不会修改项目 `.claude/settings.local.json`；E2E 夹具仓库验证未写入 `.claude`。
- 备份中的 Claude 事件只是已导入的白名单元数据；不是原始 Hook JSON、Prompt、transcript 或工具输出。
- 记忆、决策、恢复快照按备份时点恢复，包含修订历史；不改变任务完成门禁。
- 备份时存在 running 验证运行时，预检会提示；恢复后下次启动会按 Phase 3 逻辑标记为中断。

## 10. 用户隐私控制

数据保护页实际展示：

- 当前数据库健康、恢复操作历史、完整性检查、备份列表与校验状态。
- 备份策略、自动备份到期检查、自动备份清理预览与执行。
- 数据资产清单：位置、权威性、可重建性、敏感性、是否进入备份、导出说明、保留、丢失影响。
- Hook 采集字段/排除字段、事件与会话计数、待导入 Spool、Salt 存在性与大小（不显示内容）、保留策略。
- 未加密提示、应用数据目录显示与用户主动复制。
- 没有“清空全部数据”按钮；备份删除需要确认并重新校验清单与哈希。

## 11. 数据库迁移与兼容性

迁移 7（`0007_backup_restore_privacy.sql`）只新增：

| 表 | 用途 |
|---|---|
| `backup_records` | 备份登记、路径、格式/Schema、来源、大小、哈希、完整性状态、校验时间、错误 |
| `backup_settings` | 单行策略：自动开关、目标、间隔、保留、最近自动备份与错误 |
| `restore_operations` | 恢复操作审计：备份、目录、阶段、结果、切换校验、回退状态、明细 JSON |

- 迁移 1–6 未修改；旧数据保留；`backup_settings` 初始化为默认关闭。
- 迁移测试更新为 schema 7；Phase 4 数据经过 5→6→7 保留；Phase 5 数据与备份表共存。
- 旧 Schema 备份在隔离副本中升级到 7；未来 Schema 明确拒绝。
- 回滚应用版本会因 schema 过高而拒绝打开，避免静默降级。

## 12. Rust 领域服务与 Tauri IPC

| 命令 | 作用 | 写入边界 |
|---|---|---|
| `get_data_protection_overview` | 数据保护总览、资产、备份、策略、恢复历史、完整性健康 | 只读 |
| `dismiss_restore_notice` | 隐藏本次运行启动恢复提示 | 内存状态 |
| `create_backup` | 创建一致性备份 | 目标目录 + 登记 |
| `cancel_backup` | 请求取消当前备份 | 内存标志 |
| `list_backups` | 备份列表 | 只读 |
| `verify_backup` | 完整校验（含隔离打开） | 更新登记状态 |
| `delete_backup` | 确认后删除登记备份 | 删除自有备份目录 + 登记 |
| `get_backup_policy` | 读取策略 | 只读 |
| `update_backup_policy` | 保存策略 | `backup_settings` |
| `preview_backup_cleanup` | 预览旧 automatic 备份 | 只读 |
| `apply_backup_cleanup` | 确认后清理 | 删除自有 automatic 备份 |
| `run_due_automatic_backup` | 到期自动备份 + 保留清理 | 目标目录 + 登记 + 策略时间 |
| `check_data_integrity` | 结构与业务关系检查 | 只读 |
| `preflight_restore` | 只读预检 + 隔离恢复 | 应用自有临时工作区 |
| `perform_restore` | 正式恢复 | 回退目录 + 正式数据库 + 审计表 |

- 新增 `data_protection_commands.rs`；`AppState` 管理备份/恢复互斥、取消标志、预检 token 与启动通知。
- 未新增 capability、插件、通用 FS/SQL/Shell 权限；前端仍只能调用固定命令。
- 备份/恢复命令使用 `spawn_blocking`，避免阻塞 UI 线程；取消命令仍可到达。
- 恢复期间持有数据库槽位锁，其他数据库命令等待；运行中验证被拒绝。

## 13. UI 页面与交互

- 已实际调用 `frontend-design` Skill（当前主会话，2026-11-13），未使用子 Agent，未自动截图/录屏；复用现有设计系统，不重新设计整个应用。
- 新增一级导航“数据保护”（`/data-protection`），顶部工作带显示页面名称；不改变 44px 工作带、240/52px 侧栏与窗口控制。
- 页面结构：启动提示 → 状态带 → 备份与恢复 → 恢复历史 → 数据完整性 → 隐私与本地数据 → 数据目录。
- 创建备份 Dialog：目标目录、范围、预计大小、未加密警告、执行/取消、结果与失败原因。
- 备份策略 Dialog：自动开关、目标、间隔、保留、最近自动备份、旧 automatic 备份预览与两步确认清理；备份区显示策略目标可用空间与备份总占用。
- 校验 Sheet：清单/哈希/隔离完整性检查项与结果。
- 恢复向导 Sheet：7 步序列（选择备份、兼容性、影响范围、隔离验证、明确确认、执行恢复、验证结果）；当前/备份计数对比、检查项、警告、影响、Hooks 提示、输入“恢复”+勾选、执行中、结果（完成/回退/回退失败）。
- 恢复历史、完整性报告与隐私资产清单使用 `divide-y` 日志式行，不套卡片墙；状态使用图标 + 中文 + 颜色；命令/路径/哈希使用 mono。
- 未新增伪造进度百分比；只有真实加载状态与旋转图标。

## 14. 测试数量与真实退出码

### 14.1 前端

| 命令 | 退出码 | 结果 |
|---|---|---|
| `pnpm typecheck` | 0 | `tsc --noEmit` 通过 |
| `pnpm lint` | 0 | ESLint 10，`--max-warnings 0` |
| `pnpm test` | 0 | 11 个测试文件、**83** 项通过 |
| `pnpm build` | 0 | Vite 通过；JS **980.05 kB / gzip 280.77 kB**；仍有原有 >500 kB 提示 |
| `pnpm format` | 0 | Prettier 通过 |

### 14.2 Rust

| 命令 | 退出码 | 结果 |
|---|---|---|
| `cargo fmt --all -- --check` | 0 | 通过 |
| `cargo check --workspace --all-targets --locked` | 0 | 通过 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | 无警告 |
| `cargo test --workspace --locked` | 0 | **186** 项通过，0 失败，0 ignored |
| `git diff --check` | 0 | 无空白错误 |

Rust 测试构成：

| 测试组 | 数量 |
|---|---:|
| `xushu-core` lib（含 backup 模块单元测试） | 31 |
| `tests/backup.rs`（新增） | 15 |
| `tests/business.rs` | 19 |
| `tests/claude.rs` | 37 |
| `tests/decisions.rs` | 8 |
| `tests/memory.rs` | 8 |
| `tests/persistence.rs`（迁移 5→6→7） | 13 |
| `tests/recovery.rs` | 14 |
| `tests/verification.rs` | 26 |
| `xushu-desktop` lib | 2 |
| `xushu-hook` CLI | 8 |
| `xushu-hook-protocol` lib | 5 |
| **合计** | **186** |

Phase 6 覆盖：一致快照、清单/哈希、截断/损坏、未来/旧 Schema、取消、并发写、自动清理、Junction 路径、隔离恢复、受控切换、回退、启动恢复、运行中验证阻断、备份/恢复 UI 确认门禁、显示辅助函数。

## 15. Windows 桌面 E2E 证据

环境：Windows 11、Tauri 2.12.5、WebView2、隔离 app 数据目录与隔离 SQLite；无自动截图/录屏。

### 15.1 完整恢复与损坏备份拒绝

- 命令：`python scripts/desktop-e2e.py --scenario target/phase6-audit/phase6-e2e.mjs`
- 退出码：0；输出：`PASS: project/task in isolated SQLite, normal database unchanged, run directory removed`
- 证据：`target/phase6-audit/phase6-e2e-result.json`
- 实测：
  - 创建真实备份 `952ecfba-…`，`manifest.formatVersion=1`、`schemaVersion=7`、`payloadSize=413696`、`encrypted=false`；清单只含 `xushu.db`。
  - `verify_backup` 通过（清单 + SHA-256 + 隔离 SQLite/业务关系）。
  - 修改数据库（新增任务、修改记忆标题）后预检 `ready=true`，当前数据库仍有修改后的任务与记忆；预检的 `changesSinceBackup` 为 `[任务更新 1, 记忆更新 1]`。
  - 复制并篡改备份后，预检被拒绝：`BACKUP_ERROR / 备份数据文件大小与清单不一致`。
  - 正式恢复：`status=completed`、`switchVerified=true`、`finalReport.ok=true`。
  - 恢复后任务数回到 1（备份后的任务消失）、记忆标题回到“E2E 恢复记忆”、决策仍为 confirmed、恢复快照仍存在、恢复审计 1 条。
  - UI：数据保护页、备份、已验证、恢复历史、恢复完成、隐私清单、完整性区可见；点击“恢复”打开向导。
  - 正常数据库 DB/WAL/SHM 在启动前后与结束后的指纹一致；E2E 不打开正常数据库。
- 边界：这是开发构建 + 隔离数据目录，不等于生产安装包验收。

### 15.2 关闭重开一致性

- 命令：`python scripts/desktop-e2e.py --snapshot target/phase6-audit/phase6-restored.db --scenario target/phase6-audit/phase6-e2e.mjs`
- 退出码：0；输出同上 PASS。
- 证据：`target/phase6-audit/phase6-reopen-result.json`
- 实测：新应用数据目录从恢复后的单文件快照启动；schema 7、完整性正常；任务 1 个、记忆标题正确、决策 confirmed、快照存在、恢复审计 completed；数据保护页可渲染。正常数据库指纹不变。

### 15.3 布局与主题

- 命令：`python scripts/desktop-e2e.py --scenario target/phase6-audit/phase6-layout.mjs`
- 退出码：0；证据：`target/phase6-audit/phase6-layout.json`
- 视口：500×480、1366×768、1600×900、1920×1080；浅色/深色；数据保护页 + 恢复向导。
- 结果：
  - 顶部工作带均为 44px；<760px 侧栏 52px，≥760px 240px。
  - 页面与主内容无横向溢出；可见按钮宽度均 ≥20px。
  - 500×480 恢复向导宽 460px、正文可滚动（365 > 324）；1366/1600/1920 宽 760px。
  - 该测量不代替人工视觉截图。

### 15.4 正常数据库指纹说明

- 本轮 E2E 在每次运行内对正常数据库 `xushu.db`、`xushu.db-wal`、`xushu.db-shm` 三件套做启动前/启动后/结束后的 SHA-256 比较，均一致。
- 当前主文件指纹与 Phase 5.1 记录一致（DB 4096 B，`89A2318C…`）；当前 WAL/SHM 指纹与 Phase 5.1 历史记录不同（本轮多次读取均为 WAL 2463792 B `096E2C71…`、SHM 32768 B `ADD4EBAB…`），原因未在本轮确定，可能是用户在两次审计之间的正常应用使用。本轮没有打开、迁移或写入正常数据库；该差异不作为 Phase 6 测试写入的证据，也不被掩盖。
- Phase 6 会话内多次读取的 WAL/SHM 指纹一致，说明本轮测试没有改变正常数据库；完整指纹记录：`target/phase6-audit/normal-db-after.json`。

## 16. 数据隔离与敏感数据审计

- 所有 Rust 备份/恢复测试使用 `tempfile::TempDir` 隔离数据库；E2E 使用 Tauri 隔离 identifier、`XUSHU_E2E_DATA_DIR` 与独立 app 数据目录。
- E2E 不打开正常数据库；只做只读 SHA-256 指纹。
- 备份 Manifest 不包含绝对路径、Salt、Spool、Prompt、transcript、工具输入输出或密钥。
- 数据保护总览只包含计数、项目名称（最多 5 个）、缺失路径数量、Salt 存在性/大小、Spool 计数/字节数，不含 Salt 内容或事件正文。
- 错误映射：IO 错误不显示路径；备份/恢复错误使用固定文案；数据库内部错误只记录错误码。
- 对新增文件做凭据模式扫描：未发现 `sk-`、`ghp_`、`AKIA`、私钥块或明文密码；`overview.rs` 中的 `transcript` 是“不采集字段”的文案，不是数据。
- 未读取或输出任何真实 API Key、`.env`、SSH 私钥、Token 或 Claude Code transcript。

## 17. 发现并修复的缺陷

1. **Windows canonical 路径 `\\?\` 前缀进入备份登记与 UI**：E2E 断言发现备份目录不以用户目标路径开头；增加 `display_path`，展示与登记去掉 verbatim 前缀，文件操作仍用 canonical 路径。
2. **取消备份后残留空 partial 目录**：Windows 目录句柄阻止删除；失败清理前显式释放 `Directories` 句柄。回归测试 `cancelled_backup_leaves_no_valid_backup_directory`。
3. **回退副本压缩前后哈希比较必然失败**：改为压缩后校验非空，再做完整完整性检查。
4. **预检 Hooks 复核使用当前库而非备份库计数**：改为使用隔离恢复后的 incoming summary。
5. **正式恢复只检查隔离空间，未估算回退/切换峰值**：新增 `check_formal_restore_capacity`，恢复前检查 2×当前 + 2×备份 + 余量。
6. **开始恢复时其他预检仍可复用旧状态**：恢复开始时作废并清理其他预检工作区。
7. **清理应用自有工作区前未校验目录类型**：删除前先 `secure_directory`，拒绝被替换为 Junction/符号链接的目录。
8. **E2E 断言使用 `get_task` 顶层字段**：实际返回 `TaskDetail`，修正为 `.task.id`/`.task.status`（测试缺陷，非产品缺陷）。

## 18. 未实现的功能

- 备份加密、密钥管理、密钥轮换与丢失恢复。
- 常驻自动备份调度（后台服务/系统计划任务）；当前只在应用运行时检查。
- 恢复 Hook Spool、Salt 或项目 `.claude` 配置；当前明确排除。
- 恢复后自动重定位失效项目路径。
- 通用数据删除/清理 UI（记忆归档、快照清理、Spool 清理等）；本轮只有备份删除与 automatic 保留清理。
- 备份签名/来源认证；SHA-256 不防伪造来源。
- 生产安装包 sidecar、自动更新、macOS/Linux 真机验证。

## 19. NOT_RUN / PENDING / BLOCKED

| 项目 | 状态 | 说明 |
|---|---|---|
| 人工视觉复核 | **VISUAL_REVIEW_PENDING** | 未自动截图；需用户手动确认数据保护页、Dialog/Sheet、深浅主题、500×480/1366×768/1600×900/1920×1080、150% 缩放、键盘焦点 |
| Tauri E2E 中的真实回退故障注入 | **NOT_RUN** | 回退、启动回退由 Rust 测试真实执行；E2E 只验证成功切换 |
| 实际磁盘写满/权限拒绝 | **NOT_RUN** | 容量规则与错误映射已实现；未制造真实满盘/拒绝访问 |
| 备份目标最终目录冲突/目标被占用 | **NOT_RUN** | 唯一目录名与 IO 错误映射已实现；未单独制造冲突 |
| 真实独占锁 30 秒超时 | **NOT_RUN** | 代码路径已实现；未构造持续独占锁并等待 30 秒 |
| >2 GiB 真实大备份拒绝 | **NOT_RUN** | 清单大小上限有单元测试；未生成真实超大文件 |
| 回退失败真实故障注入 | **NOT_RUN** | 回退成功与启动回退已由 Rust 测试覆盖；未强制制造回退失败 |
| 并发恢复/重复 token 压力测试 | **NOT_RUN** | 单一互斥与单次 token 已实现；未做并发压力测试 |
| 不同 Windows 用户环境 | **NOT_RUN** | 重开使用新应用数据目录；未使用第二个 Windows 账户 |
| 跨卷恢复/跨卷切换 | **NOT_RUN** | 复制路径不使用跨卷 rename 假设；未在真实跨卷环境验证 |
| 恢复后真实 Claude Code 联调复核 | **NOT_RUN** | 未启用任何真实项目 Hooks；需要用户明确授权的一次性项目 |
| 生产安装包与 sidecar | **NOT_RUN** | Phase 7 |
| macOS/Linux | **NOT_RUN** | 未构建、未实机验证 |
| 屏幕阅读器/真实 Windows 剪贴板 | **NOT_RUN** | 本轮 E2E 在隔离 WebView 内操作 |
| Windows Snap 实际悬停菜单 | **NOT_RUN** | 沿用 Phase 3.6 历史状态，未在本轮改为 PASS |
| Phase 4/5 人工视觉 | **PENDING** | 本轮未将历史 PENDING 改为 PASS |

## 20. Git 工作区状态

- 分支：`main`
- 起始 HEAD：`06899e9986ee6983a623097235429c53fd0518f8`
- 本轮**未提交、未推送、未 reset、未 clean**。
- 工作区共 39 项状态行：28 个已跟踪修改 + 11 个未跟踪条目（含 `docs/phases/PHASE_4_REAL_HOOKS_REGRESSION.md`）。
- 未跟踪的 `docs/phases/PHASE_4_REAL_HOOKS_REGRESSION.md` 原样保留，未覆盖、未修改、未提交。
- 主要新增：
  - 迁移 `crates/xushu-core/migrations/0007_backup_restore_privacy.sql`
  - 领域 `crates/xushu-core/src/backup/{mod,manifest,paths,integrity,service,restore,overview}.rs`
  - 测试 `crates/xushu-core/tests/backup.rs`
  - IPC `apps/desktop/src-tauri/src/data_protection_commands.rs`
  - 前端 `apps/desktop/src/features/data-protection/`、`pages/data-protection-page.tsx`、测试
  - 文档 `docs/architecture/ADR/0009-...md`、`docs/architecture/DATA_PROTECTION.md`、本报告
- 主要修改：迁移注册、Database 备份 API、Claude 隐私快照、path_guard 可见性、错误映射、AppState、路由/导航、domain 类型、E2E 三件套指纹、设计/依赖/总计划/README/CLAUDE.md。
- E2E 证据位于已忽略的 `target/phase6-audit/`，不进入仓库。

## 21. Phase 7 前必须完成的工作

1. 用户手动视觉签收数据保护页、创建/策略 Dialog、校验 Sheet、恢复向导在 500×480、1366×768、1600×900、1920×1080、深浅主题、150% 缩放与键盘焦点下的表现。
2. 在一次性隔离 Tauri 环境中做真实故障注入：切换中杀进程、磁盘满、权限拒绝、回退失败，确认启动提示与文件保留。
3. 生产安装包与 `xushu-hook` sidecar 打包；用安装版验证备份/恢复，不只开发构建。
4. 明确备份加密与密钥管理是否进入 V1；若进入，新增 ADR 与成熟加密实现。
5. 明确自动备份是否需要常驻调度；若需要，设计合法的 Windows 生命周期方案并在 UI 如实说明。
6. 在用户明确授权的一次性项目中复核恢复后的 Claude Code Hooks 状态；不得自动修改真实项目配置。
7. 屏幕阅读器、真实剪贴板、多显示器/DPI、Windows Snap 实际菜单。
8. 将本报告的 PENDING 项逐项更新；不得把人工视觉或生产安装包写成 PASS。

## 22. 人工验收清单

1. 打开“数据保护”，确认状态带显示真实数据库版本/完整性、最近备份、策略与未加密提示；没有备份时显示明确空状态。
2. 创建备份到 D 盘测试目录；观察“创建一致性快照并校验”真实状态；完成后检查路径、时间、大小、Schema、格式、SHA-256 与“已验证”。
3. 打开备份目录，确认只有 `manifest.json` 与 `xushu.db`；复制一份并篡改 `xushu.db`，在恢复向导中确认预检拒绝且当前数据库不变。
4. 修改一个任务或记忆；用原备份预检，确认兼容性/影响范围/隔离验证/警告可见；输入“恢复”并勾选前“执行恢复”不可用。
5. 执行恢复，确认结果为“恢复完成”且最终校验通过；检查任务/记忆/决策/快照回到备份时点，恢复历史出现记录。
6. 在 Rust 测试或一次性构建中验证回退/启动恢复；不要对真实用户数据库做破坏性测试。
7. 校验备份；删除一份备份，确认二次确认、清单/哈希复验与“仅已知文件”保护。
8. 打开备份策略：启用/禁用自动备份、修改间隔与保留；预览旧 automatic 备份并确认手动备份不进入自动清理。
9. 恢复一个带 Hooks 期望状态的备份；确认项目 `.claude/settings.local.json` 未被修改，UI 提示需要重新检查接入状态。
10. 检查隐私清单：Spool、Salt、项目配置、导出报告、WebView 偏好明确“不在备份”；Salt 只显示存在性/大小。
11. 在 500×480、1366×768、1600×900、1920×1080、深浅主题与 150% 缩放下检查页面、Dialog、Sheet、按钮换行、滚动与焦点返回；由用户手动截图，不由 AI 自动截图。
12. 确认没有伪造进度、没有把未校验备份显示为成功、没有把回退显示为成功恢复。

---

**明确结论：** Phase 6 的代码、迁移、IPC、前端、自动化与隔离 Windows Tauri/WebView2 E2E 已完成并记录；**人工视觉、真实故障注入、生产安装包、跨平台与真实 Claude Hooks 复核仍未完成**。Phase 6 整体验收保持 **PENDING**，等待用户委派独立验收。不得把 `BACKUP_CREATED` 直接视为整个备份恢复系统 PASS；本轮分别记录了 `BACKUP_CREATED`、`BACKUP_INTEGRITY_VERIFIED`、`ISOLATED_RESTORE_VERIFIED`、`RESTORE_SWITCH_VERIFIED`、`ROLLBACK_VERIFIED`、`DATA_PRIVACY_VERIFIED`、`E2E_VERIFIED` 与 `VISUAL_REVIEW_PENDING` 的范围。