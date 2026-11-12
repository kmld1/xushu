# XUSHU Phase 4 收尾审计

> 本文保留 Phase 4.2 之后的审计基线。两项 Hooks 风险的修复和当前提交门禁见 [Phase 4.3 安全可靠性收尾](PHASE_4_3_SECURITY_CLOSEOUT.md)。

审计于 2026-11-09 开始，2026-11-12 更新，时区为 Asia/Shanghai。仓库为 `D:\MyProject\xushu`，分支 `main`，审计基线 `f3ff6a8`。整体验收 **PENDING**。

## 1. 功能验收结论

| 项目 | 结论 | 证据范围 |
| --- | --- | --- |
| 六种 Claude Code Hooks、关联、去重 | **PASS（Phase 4.1 实测）** | Windows Claude Code 2.1.319 在一次性项目实际触发 SessionStart、PreToolUse、PostToolUse、PostToolUseFailure、Stop、SessionEnd。累计 4 个会话、18 条事件，实际投递重放未重复入库。本轮未启动 Claude Code。 |
| 项目级授权、修复、停用及卸载 | **PASS（已测路径）** | Phase 4.1 实测未授权请求被拒绝、授权写入及回读、停用与卸载后不再有新投递、第三方配置保留。故障下的卸载和根目录边界见第 4 节。 |
| 最小采集和事件接收 | **PASS（已测样本与故障）** | 真实样本的最小信封、会话及工具 ID 伪名、无正文；本轮回归覆盖非法输入、跨项目、并发容量、共享冲突、保留和异常顺序。缺失结束事件维持「结束未确认」。 |
| 任务完成门禁 | **PASS** | Hook 导入只写 Claude 接入表。真实 Stop 和工具成功后样本任务保持 draft、revision 1，验证运行数为 0。重跑现有回归确认有效自动 PASS 和所需人工确认才可完成，FAIL、NOT_RUN、INCONCLUSIVE、STALE 无法绕过门禁。 |
| 桌面真实数据交互 | **PASS（Phase 4.2 已测范围）** | 真实事件副本通过 Tauri IPC 和 WebView 呈现 18 行活动；已验证预览授权、键盘操作、焦点返回及布局测量。Windows 原生 150% 和 WebView 100% 仿真有实测记录。 |
| 正常库隔离 | **PASS（当前入口已测路径）** | 独立标识、数据目录和构建缓存；启动前校验及异常路径实测见第 3 节。历史入口曾写入正常库，遗留见第 2 节。 |
| 人工视觉和原生窗口验收 | **VISUAL_REVIEW_PENDING** | 见第 5 节。 |
| 生产安装包和兼容性 | **NOT_RUN** | 生产 CSP、安装包内 sidecar 定位与升级、其他 Claude Code 版本、macOS/Linux 和真实五秒 Hook 超时。 |

历史实测详见 [Phase 4.1 真实 Hooks 联调](PHASE_4_1_E2E_REPORT.md) 和 [Phase 4.2 UI 与数据隔离验收](PHASE_4_2_FINAL_AUDIT.md)。

## 2. 正常库历史数据与只读清理预览

从 `%APPDATA%\dev.xushu.desktop\xushu.db` 及同名 WAL 取得稳定副本，在临时副本上以 SQLite `mode=ro`、`query_only=ON` 查询。原库、WAL、SHM 在副本审计前后逐字节相同。完整性检查为 `ok`，外键违规为 0。正常库中共 6 个项目、9 个任务。表中的路径使用 `%TEMP%` 和仓库相对路径，六个测试 Git 根目录均存在。

| 历史来源 / 项目名 | 测试仓库 | 项目 ID | 当前状态 |
| --- | --- | --- | --- |
| Phase 2 桌面 E2E / Phase2 E2E 项目 20261106010141 | `%TEMP%\xushu-phase2-e2e\repo-20261106010141` | `82a5e804-9e18-485b-a295-2fbd57ff19a3` | 活跃，1 个任务 |
| Phase 3 首轮 / Phase3 E2E 项目 | `target/e2e/repo-20261106015932` | `a3b67830-2e6e-47ae-a8c9-0f1f8b62c10e` | 已归档，1 个任务 |
| Phase 3 修复复验 / Phase3 E2E 项目 | `target/e2e/repo-20261106021137` | `974add8d-917b-4e5d-adca-e2a7ee5c7b2e` | 已归档，1 个任务 |
| Phase 3 加固冒烟 / Phase3 E2E 项目 | `target/e2e/repo-20261106022214` | `27f6b98e-00cf-4371-a734-112b864475c0` | 已归档，1 个任务 |
| Phase 3 最终复验 / Phase3 E2E 项目 | `target/e2e/repo-20261106023630` | `419bb43f-41a7-4a5d-9c92-371fa94378f8` | 已归档，1 个任务 |
| Phase 3.5 桌面视觉样本 / Phase 3.5 UI 审阅 | `target/e2e/ui-repo-20261106032041` | `c3078cae-1a6f-4506-ab7d-09f5b9307710` | 活跃，4 个任务 |

| 所属项目（ID 前缀） | 任务 | 任务 ID | 状态 / 修订 |
| --- | --- | --- | --- |
| `82a5e804` | E2E 任务 20261106010141 | `569de415-4f0f-4d1f-a238-98a8ca8d7343` | done / 2 |
| `a3b67830` | Phase3 E2E 验证任务 20261106015932 | `39646a67-ed7a-4a6f-a6ec-2a042da8c77e` | done / 2 |
| `974add8d` | Phase3 E2E 修复验证 20261106021137 | `bb135f3f-4b00-4371-ad09-dc49a96c243c` | done / 2 |
| `27f6b98e` | Phase3 E2E 加固 smoke 20261106022214 | `b063844b-72dc-429c-b73a-850f6c1b5c15` | done / 2 |
| `419bb43f` | Phase3 E2E 最终复核 20261106023630 | `6f79feda-8bc6-4a91-9628-d24f86efe494` | done / 2 |
| `c3078cae` | 完成工作台与验收中心视觉重构 | `528d5634-6957-4314-9fdb-7ca5766c0f7e` | done / 3 |
| `c3078cae` | 验证失败必须在界面中明确显示 | `6c90cc27-05df-421c-a67c-249fd8fd2a1b` | awaiting_verification / 2 |
| `c3078cae` | 等待设计确认的阻塞任务 | `34a8fd87-4668-4a08-be13-17536956d14e` | blocked / 2 |
| `c3078cae` | 整理 Phase 3.5 人工视觉复核清单 | `071dab81-f311-4b93-bf20-41b117ae41d6` | draft / 1 |

### 来源与真实数据边界

- Phase 2 的旧读取脚本直接指向正常库。Phase 3 的两个旧 Tauri 覆盖配置均沿用常规标识且没有独立数据目录。四次运行 tag 与 Phase 3 报告相符。Phase 3.5 的 `ui-review-setup.mjs` 逐一匹配四个标题，后续脚本对其中一项重跑并确认完成。Phase 3.6 和 Phase 4 复用视觉样本。来源见 [Phase 2 报告](PHASE_2_REPORT.md)、[Phase 3 报告](PHASE_3_REPORT.md) 和 [Phase 3.5 UI 报告](../design/UI_REDESIGN_PHASE_3_5.md)。
- 按 `tasks.project_id → projects.id` 核对。9 条任务描述和非空阻塞/迁移原因与脚本一致。关联 9 条验收条件、38 条状态迁移、2 条无备注人工确认、6 条验证配置、7 次验证运行、7 条验证检查和 7 条证据。命令均为样本的 `git --version` 或预定失败的 `git rev-parse --verify`。项目关联的 Claude 接入、会话、工具调用、事件均为 0。已审计业务记录可归因于测试夹具，未发现真实用户业务数据混入。实际运行结果属于测试证据。
- 库内另有 1 条独立持久化探针，正常数据目录另有 1 个导出文件。两者排除在清理范围之外。Phase 4.1 的隔离证据、其他测试目录和历史 AppData 也不属于这 6 个项目的清理范围。

### 清理预览

| 对象 | 预览动作 | 影响 |
| --- | --- | --- |
| 4 个已归档 Phase 3 项目 | 保持归档 | 4 个任务及验证记录保留。 |
| 活跃 Phase 2 项目 `82a5e804…` | 经用户确认后可在项目页软归档 | 1 个项目移出活跃项目列表，任务、人工确认、目录均保留，可恢复。 |
| 活跃 Phase 3.5 项目 `c3078cae…` | 保留用于第 5 节，人工视觉复核后再确认是否软归档 | 4 个任务、三次验证运行、状态和证据均可继续用于复核。 |
| 9 个任务、依赖行和磁盘目录 | 保留 | 直接硬删除会触及外键限制或级联关系，也会移除验收历史。 |

未来执行前按完整项目 ID、路径、任务 ID 与依赖计数重新核对，若有新增或变化，重新预览。此次执行的 SQLite 查询均为只读，归档和删除均为 **NOT_RUN**。

## 3. E2E 数据隔离

- `scripts/desktop-e2e.py` 每轮在 `target` 下创建新的运行目录，使用不同的 `dev.xushu.e2e.*` 标识、`appDirectoriesOverride`、本机独立调试端口及 WebView2 profile。工作目录、SQLite、Hook 收件箱及夹具仓库均按运行隔离。构建产物独立存放在 `target/desktop-e2e-build/`，锁阻止并发覆盖该构建。
- 修复前只在进程启动后检查落盘路径。此时 Tauri 的 `AppState::initialize` 已可能执行迁移、恢复运行及导入。现由启动器只给隔离桌面子进程传入预期数据目录，应用在初始化前核对标识与实际解析目录。隔离构建缺少预期目录、目录不符，或常规标识收到隔离启动参数时拒绝初始化。启动器保留实际落盘和正常库指纹检查。
- 失败和 Python 异常经 `finally` 关闭本轮桌面和 Vite，并在共享冲突时有界重试清理本轮目录。历史目录未列入清理。外部强杀 Python 或整机掉电时 `finally` 无法运行，可能遗留隔离目录或子进程。数据路径在 SQLite 启动前已限定。硬杀启动器及整机掉电为 **NOT_RUN**。自定义 `--scenario` 是本地可执行代码，隔离结论覆盖已审查的测试场景。

| 实测路径 | 退出及观察 |
| --- | --- |
| `python scripts/desktop-e2e.py` | 0。真实 IPC 写入 1 个夹具项目和 1 个关联任务，隔离库完整性检查通过，正常库 DB/WAL 指纹一致，本轮目录清理。 |
| `python scripts/desktop-e2e.py --snapshot <Phase 4.1 证据库>` | 0。使用证据副本启动和写入夹具成功。正常库与证据原库的 DB/WAL/SHM 指纹一致，本轮目录清理。 |
| 故意向隔离桌面子进程传入错误目录 | 修复前探针退出 1，进程未拒绝启动。修复后桌面非零退出，两处 SQLite 均未创建，探针断言通过。 |
| `--scenario target/phase42-audit/failure.mjs` | Node 在夹具写入后按预期退出 1，启动器报告该失败。 |
| 写入夹具后注入 Python `KeyboardInterrupt` | 捕获预期中断，关闭并清理本轮进程和目录。 |
| 写入夹具后强制终止本轮桌面 PID | 启动器检测非零退出，按预期报错并清理。 |

错误目录、场景失败、模拟 Python 中断、强制终止桌面四个探针还逐字节比较了正常 DB/WAL/SHM，结果一致。运行前存在的 `target/run-*` 集合保持一致。

## 4. Hooks 风险与本轮修复

| 状态 | 审计发现 |
| --- | --- |
| **已修复并回归** | 安装及校验失败曾无条件恢复原设置。文件在写入后若被用户编辑，回滚可能覆盖新内容，甚至删除在本次安装后新编辑的文件。统一回滚前核对当前路径及原始字节等于本次写入的内容，发现变化则报冲突。安装原本已精确匹配且没有写文件时跳过回滚。新增回归先失败后通过，既有数据库写入失败后的停用/卸载回滚测试通过。 |
| **高风险 · PENDING** | `remove_hooks` 先要求 `drain_pending` 成功，再移除配置。若待处理信封持续被占用、积压无法推进或导入发生数据库错误，停用/卸载可能在触碰设置前失败，原 Hook 仍可继续运行。正常停用/卸载已验证，持久故障下可可靠撤销采集的验收尚未关闭。 |
| **高风险 · PENDING** | `read_snapshot` 将登记的项目根路径重新 canonicalize 后直接使用解析结果。已有检查拒绝 `.claude` 或设置文件的重定向，但登记根路径或其上级被替换为 junction 时，重新预览仍可能面向另一处目录。当前项目身份与写入边界在此条件下的故障回归为 NOT_RUN。 |
| **剩余边界 · PENDING** | 文件替换与 SQLite 更新之间无法用同一个事务覆盖。整进程中止可能留下文件和期望状态不一致。回读与回滚的比较可识别已观察到的外部变动，路径检查与最终写入仍有竞争窗口。配置备份保存原始项目设置，可能包含已有环境值。需在共享或迁移时作为私有文件处理。 |

独立完成门禁在本轮回归范围内未发现新增高风险绕过。现有工作区的任务状态机和验证引擎相对于 Phase 3 基线没有源码修改。

## 5. 人工视觉与原生行为

下列条目全部 **PENDING**。Phase 3.6 早期浅色截图及 Phase 4.2 程序化测量按原报告保留，当前界面的人工复核仍待完成。

- **工作区与主题。** 浅色、深色和跟随系统下检查 44px 工作带与侧栏连续性、品牌/项目选择器/分支/HEAD/dirty/页名层次、焦点和失焦、圆角/阴影、悬停、禁用、菜单、Dialog、Sheet、Toast、滚动条、对比度及命令块。以真实长项目名、分支名和阻塞原因复核截断、完整提示与换行。
- **窗口。** 在 1366×768、1600×900、1920×1080 及 500×480 检查排版、52px 导航轨、窗口控件与主题入口、页面横向滚动和长内容可读性。实机操作最小化、最大化、还原、关闭、空白区拖动和双击、边缘缩放、激活/失焦、跨屏及跨 DPI。验证 Windows 原生 100%、125%、175%、200% 与 150% 观感。实测 Snap 悬停菜单及半屏/三栏选择，`HTMAXBUTTON=9` 命中已有程序化证据。
- **业务页面。** 工作台有/无项目、状态条、焦点任务、最近验证与待处理。项目列表、导入、选中、Git 行与菜单。项目详情仓库条、任务表和侧栏。任务列表筛选和行操作，详情的待验证/完成/阻塞/取消、StateTrack、阻塞原因及完成门禁。诊断、设置与 404 的空态、反馈和层级。
- **验收流。** 验收中心空队列/待验收和自动/人工来源，PASS、FAIL、NOT_RUN、INCONCLUSIVE、STALE 表达，验证配置和批准、运行及取消，证据/历史/报告 Sheet 的长输出、滚动和复制。
- **Claude Code 接入。** 项目详情未配置、已启用、已停用、配置异常、需修复及错误原因。六条预览表、命令展开、授权复选框、禁用和悬停、停用/卸载确认与反馈。活动 Sheet 的空态、待导入、会话、工具执行中/成功/失败/未知、缺失结束、长工具名、状态图标和窄窗滚动。
- **真实键盘与辅助技术。** Tab/Shift+Tab、Space/Enter、Escape、焦点可见及关闭后返回，主题单选、窗口控制、行操作、复选框、预览和活动滚动。屏幕阅读器的名称、状态与朗读顺序。Phase 4.2 已有自动键盘和无障碍树证据。

视觉验收需要用户手动操作或提供截图。本轮未修改 UI 源码或窗口控件。

## 6. Git 差异与敏感文件

- 审计起点为 98 条展开记录。58 个仅工作区修改、2 个索引中已暂存且工作区继续修改的重命名、38 个未跟踪文件。加入本报告后为 99 条，未跟踪文件增至 39 个。两条暂存重命名是验证证据和验证历史的 `*-dialog.tsx → *-sheet.tsx`，索引为 R100。只提交当前索引会遗漏工作区内容。
- **Phase 3.5 / 3.6。** 页面、主题令牌、应用布局及复用组件是预期范围。未跟踪的 `workspace-chrome.tsx`、`window-controls.tsx`、`window_chrome.rs`、`section.tsx`、`sheet.tsx`、`command-block.tsx`、ADR 0005/0006 和两份 UI 报告是对应交付的一部分。
- **Phase 4 / 4.1。** 工作区 Cargo 清单与锁文件、Tauri 入口/状态、前端项目详情和领域类型为预期变更。必须纳入的未跟踪源码包括 `claude_commands.rs`、`features/claude/`、`crates/xushu-core/src/claude/`、迁移 `0004_claude_hooks.sql`、核心集成测试及 `crates/xushu-hook{,-protocol}/` 的清单、源码和测试。ADR 0007、接入说明和阶段报告也尚未跟踪。
- **Phase 4.2。** 接入预览和活动组件、确认焦点与测试、`scripts/desktop-e2e.py`、`scripts/desktop-e2e.mjs` 及验收报告均属于阶段交付。38 个初始未跟踪文件均归入上述阶段，未发现无关可见文件。此前阶段混在同一工作区，仅能按功能归属审查。
- **临时产物。** `target/e2e/`、`target/phase3.6-audit/`、`target/phase4-audit/`、`target/phase42-audit/`、`target/desktop-e2e-build/`、`target/run-ncvqwdjf/`、依赖、Vite 输出及 Tauri 生成 schema 均已忽略。旧测试仓库、证据和缓存保留。当前阶段提交清单应排除这些产物。
- **敏感检查。** 对 Git 索引与可提交工作区查验路径、SQLite/WAL 文件头、常见凭据格式及配置赋值模式，仅记录命中位置。未发现数据库、用户 Claude 私有设置、私钥、实际凭据或备份进入候选集。唯一凭据格式命中位于基线脱敏测试，其他标记来自测试样本或规则说明。已提交历史另做了敏感文件名检查。报告仍含本地运行环境信息，公开分享前需核对可披露范围。
- **忽略规则缺口。** `.db`、`.db-wal`、`.db-shm`、`.sqlite`、`.sqlite3`、`.env` 及常见私钥已覆盖。若将 `.claude/settings.local.json`、私有 `.claude.json`、`.sqlite` 或 `.sqlite3` 的 WAL/SHM、`hook-backups/` 或 `hook-inbox/` 拷到可跟踪路径，现有规则可能放行。正常 AppData 和 Phase 4.1 证据原目录位于仓库外。审计按 Git 候选路径检查。

## 7. 本轮验证与提交结论

| 命令 | 退出码 | 结果 |
| --- | --- | --- |
| `pnpm typecheck`、`pnpm lint`、`pnpm test`、`pnpm build` | 0 / 0 / 0 / 0 | 8 个前端测试文件、54 项通过。构建有既有大于 500 kB 的 chunk 提示。 |
| `pnpm format`、`git diff --check`、`git diff --cached --check` | 0 / 0 / 0 | 仓库配置范围内的格式检查通过，工作区和索引的已跟踪差异无空白错误。另核对本报告的空白、15 个标识和文档链接。 |
| `cargo fmt --all -- --check`、`cargo check --workspace --all-targets --locked`、`cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 / 0 / 0 | 格式、编译与 lint 通过。 |
| `cargo test -p xushu-core --locked rollback_preserves_settings_changed_after_our_write` | 修复前 1，修复后 0 | 复现原回滚没有拒绝外部修改。修复后定向和完整套件均通过。 |
| `cargo test --workspace --locked` | 0 | 110 项通过。 |
| E2E 正常、失败及异常退出 | 见第 3 节 | 已测隔离路径通过。 |

**正式 Phase 4 验收提交条件尚未满足。** 第 4 节两个高风险场景和第 5 节人工视觉仍待关闭。当前已验证成果可以按明确文件清单整理为保留 PENDING 状态的审计检查点，提交范围待用户确认。工作区与索引保留，未创建提交或推送。历史数据处置独立等待确认。
