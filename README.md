# 序枢 XUSHU

本地优先、面向独立开发者的 AI 工作控制系统。

目标是把 AI 参与的开发工作组织成可追溯、可验证的闭环：

> 目标与验收标准 → 用户授权的执行事件 → 独立运行验证 → 证据与决策留存 → 断点恢复

XUSHU 不替代 Claude Code、编辑器或终端，也不把 AI 的“已完成”文本当作完成证据。

## 当前状态

**Phase 7（Windows x64 0.1.1 正式候选）**。宿主构建与安装包载荷核验通过，状态为 `RELEASE_PREPARATION_COMPLETE`。Phase 7A.4 仍为 `VALIDATION_PENDING`，四项来宾及原始现场验收为 `DEFERRED_BY_USER`。

**历史 Phase 7A（Windows 正式产品化、安装与生命周期）：Windows x64 Release、NSIS 安装包、`xushu-hook` sidecar、有界日志与隔离安装版 E2E PASS；生产 identifier 实机安装、代码签名、真实 Claude Code 会话与人工视觉 PENDING/NOT_RUN（2026-11-14）。** [Phase 7A 报告](docs/phases/PHASE_7A_REPORT.md) · [ADR-0010](docs/architecture/ADR/0010-windows-release-packaging.md)

**Phase 6（安全、隐私保护、数据备份与灾难恢复）：代码、迁移、受限 IPC、前端、自动化与隔离 Windows Tauri/WebView2 E2E PASS；人工视觉、真实故障注入与生产化验收 PENDING（2026-11-13）。** [Phase 6 报告](docs/phases/PHASE_6_REPORT.md) · [ADR-0009](docs/architecture/ADR/0009-backup-restore-data-protection.md) · [数据保护规范](docs/architecture/DATA_PROTECTION.md)

**Phase 5（长期记忆、开发决策与跨会话恢复）：代码、迁移、自动化与隔离桌面 E2E PASS；人工视觉与产品化验收 PENDING（2026-11-12）。** [Phase 5 报告](docs/phases/PHASE_5_REPORT.md) · [ADR-0008](docs/architecture/ADR/0008-project-memory-decisions-recovery.md)

**Phase 4（Claude Code Hooks 接入）：Windows 六种真实 Hooks 联调、已测功能和 E2E 数据隔离 PASS。Phase 4.3 补齐撤销采集与目录校验回归，整体验收仍为 PENDING。** [Phase 4.3 安全可靠性收尾](docs/phases/PHASE_4_3_SECURITY_CLOSEOUT.md)

**Phase 3（独立验收引擎）：PASS（2026-11-06）。**

已实际完成并验证：

- 导入本地 Git 项目，只读检查根目录、分支、HEAD、dirty、unborn 与 detached HEAD；
- 项目重命名、软归档与恢复；创建/编辑/归档/恢复任务，按项目、状态、关键词筛选；
- 验收条件与人工确认继续使用 Phase 2 的修订模型；
- 自动验收条件可配置“可执行程序 + 参数数组 + 显式工作目录 + 超时 + 输出上限”；
- 用户必须在命令预览、风险提示与复选框确认后批准运行；
- 后台执行真实命令，记录退出码、耗时、脱敏输出、Git HEAD 与工作区内容指纹；
- PASS / FAIL / NOT_RUN / INCONCLUSIVE / STALE 严格区分；超时、取消、启动失败不会伪造 PASS；
- 任务/条件/配置修订或代码状态变化后，旧证据自动过期；
- 完成门禁只接受当前有效的自动 PASS 与有效人工确认；
- 验收中心：待验收队列、命令预览、批准、取消、证据、历史、报告导出；
- 真实桌面 E2E：配置 → 批准 → 运行 → PASS → 完成 → 重启后仍为 done 且证据可读。

**Phase 3.5（桌面 UI/UX 产品化）：技术检查 PASS，人工视觉验收 VISUAL_REVIEW_PENDING（2026-11-06）。**

**Phase 3.6（窗口一体化与顶部融合）：技术检查 PASS，人工视觉验收 VISUAL_REVIEW_PENDING（2026-11-06）。**

- 切换为 Tauri 自定义标题栏（`decorations: false`），建立 44px 单一顶部工作带；
- 品牌、当前项目、branch/HEAD/dirty、页面名称、主题与自绘窗口控制位于同一表面；
- 原生 Windows 11 Snap Layouts 通过 `HTMAXBUTTON` 覆盖层恢复；拖动、双击、最小化/最大化/还原/关闭均实测；
- 工作台首屏改为状态条 → 焦点任务 → 最近验证，不再以大标题开场；
- 一级页面移除独立页头，主操作进入上下文工具栏；
- 窗口最小尺寸 500×480；<760px 侧栏收为 52px 图标轨；
- 已测 1366×768、1600×900、1920×1080、500×480 与 100%/150% 缩放无页面级横向溢出；
- Snap 悬停菜单视觉、实机鼠标双击、多显示器与其它 DPI 仍需人工复核。

**Phase 4 已完成的技术能力：**

- 项目级 Claude Code 接入，默认关闭；只写项目 `.claude/settings.local.json`，不碰团队/用户/系统设置；
- 安装前预览六个 Hooks、写入位置、数据边界与备份说明；用户勾选并明确批准后才写入；
- `xushu-hook` CLI 有界读取官方 Hook JSON，只保留白名单元数据；原始 Prompt、命令、文件内容、工具输出、transcript 与凭据不进入 spool、SQLite 或日志；
- 离线 spool、原子写入、上限/保留策略、项目根指纹、投递幂等与工具事件语义去重；
- 项目详情显示接入状态、最近接收、待导入、未结束工具调用与最近活动；活动 Sheet 显示会话与时间线；
- 停用/卸载可逆，只删除 XUSHU 自己创建的条目；配置异常、路径重定向、外部修改与接收器更新可检测并提示修复；
- Hook 事件与任务完成门禁物理隔离；`Stop`、`PostToolUse` 成功都不等于验收通过。

**Phase 5 已完成的技术能力：**

- 项目长期记忆：目标、约束、架构、规范、事实、风险、后续事项与项目知识；来源、确认状态、有效性、任务/证据关联与完整修订历史；
- 开发决策：草稿 → 已确认 → 已被取代/已废弃；已确认决策不可原地改写，取代关系双向可追溯，历史不被覆盖；
- 确定性跨会话恢复包：项目/Git/任务/阻塞/验收 blockers/独立验证/Claude 元数据/已确认记忆与决策/下一步/待读文件/冲突与缺失信息；
- 快照指纹与当前/过期判定；历史快照保留最近 20 份；过期或历史快照复制时明确添加警告；
- 项目记忆与决策页面、项目详情摘要、任务详情关联、一级“恢复”页面；恢复指令预览与用户主动复制；
- 迁移 6 纯追加表，旧 Phase 4 数据保留；记忆/决策/快照事务与项目隔离；重启后数据与快照一致；
- 恢复包不依赖 AI，不读取 transcript/Prompt/工具输出，不扫描项目文件内容。

**Phase 6 已完成的技术能力：**

- 一致性备份：SQLite Online Backup API、WAL 合并、单文件快照、`.partial-*` 临时目录 + 同卷原子改名、失败/取消清理；
- 备份格式 v1：`manifest.json` + `xushu.db`，应用/Schema/格式版本、备份 UUID、来源、大小、SHA-256；拒绝未知字段、路径穿越、未来 Schema、截断/损坏文件；
- 完整校验：清单、SHA-256、隔离副本迁移、SQLite 完整性、外键、项目/任务/验收/验证/证据/Claude/记忆/决策/恢复快照关系；
- 灾难恢复：只读预检 → 隔离恢复 → 15 分钟一次性 token + 输入“恢复”+ 勾选 → 回退副本 → 受控切换 → 最终校验；失败自动回退；
- 恢复日志与启动恢复：prepared/switching/switched/completed；崩溃后优先回退原数据库；回退失败明确 `needs_manual`；
- 备份策略：自动备份默认关闭；只在应用运行期间检查；自动保留只清理 automatic；手动备份永不自动删除；
- 数据保护中心：备份历史、校验、恢复历史、完整性检查、数据资产清单、Hook 采集边界、Spool/Salt 边界、未加密提示、应用数据目录；
- 路径安全：拒绝 UNC、相对路径、Junction、符号链接与重解析点祖先；文件打开不跟随链接；大小/容量上限；
- 恢复不包含 Hook Spool、Salt、项目 `.claude` 配置、导出报告或 WebView 偏好；不会自动安装/停用 Hooks，不改变任务完成门禁。

**Phase 6 待验收项**：数据保护页/创建备份/策略 Dialog/校验 Sheet/7 步恢复向导的人工视觉与键盘焦点；真实故障注入（切换中杀进程、磁盘满、权限拒绝、回退失败）；生产 identifier 实机安装与签名；macOS/Linux；真实 Claude Code 恢复后复核。备份未加密，审计时不得把未校验备份或回退显示为成功恢复。

**Phase 5 待验收项**：知识页/恢复页的浅色与深色人工视觉、500×480/1366×768/1600×900 实际截图、长文本与窄窗交互、键盘焦点；生产安装包与跨平台验证仍待后续阶段。

详见 [`docs/phases/PHASE_5_REPORT.md`](docs/phases/PHASE_5_REPORT.md)、[`docs/phases/PHASE_4_REPORT.md`](docs/phases/PHASE_4_REPORT.md)、[`docs/phases/PHASE_3_REPORT.md`](docs/phases/PHASE_3_REPORT.md)、[`docs/design/UI_REDESIGN_PHASE_3_5.md`](docs/design/UI_REDESIGN_PHASE_3_5.md) 与 [`docs/design/UI_REDESIGN_PHASE_3_6.md`](docs/design/UI_REDESIGN_PHASE_3_6.md)。

## 冻结技术栈

| 层 | 技术 |
|---|---|
| 桌面壳 | Tauri 2.x |
| 前端 | React + TypeScript strict + Vite |
| UI | Tailwind CSS + shadcn/ui + Lucide |
| 路由 / 异步状态 / UI 状态 | React Router / TanStack Query / Zustand |
| 表单与校验 | React Hook Form + Zod |
| 核心服务 | Rust（Tauri Commands + 领域服务） |
| 数据库 | SQLite + rusqlite + 版本化迁移 |
| Git | Rust 以参数数组只读调用本机 `git` |
| Claude Code 接入 | 项目级 Hooks + 本地 `xushu-hook` CLI + 有界 spool |
| 测试 | Vitest + React Testing Library + `cargo test` |
| 包管理 | pnpm + Cargo workspace |

## 环境先决条件

| 组件 | 要求 | 当前实测 |
|---|---|---|
| Windows | Windows 11 x64 | 10.0.26200 ✅ |
| Node.js | `^22.22.2` 或 `^24.15.0` | v24.20.0 ✅ |
| pnpm | 12.16.1（`packageManager` 固定） | 12.16.1 ✅ |
| Git | 可用 | 2.56.0.windows.2 ✅ |
| Rust | stable MSVC，Tauri 2.12 MSRV 1.90 | 1.99.1 ✅ |
| MSVC | C++ Build Tools，含“使用 C++ 的桌面开发” | VS Build Tools 2026 18.11.3 ✅ |
| Windows SDK | 与 MSVC 匹配 | 10.0.26100.0 ✅ |
| WebView2 | Evergreen Runtime | 156.0.4314.58 ✅ |

## 快速开始

```bash
# 1. 按锁文件安装前端依赖
pnpm install --frozen-lockfile

# 2. 前端质量门禁
pnpm typecheck
pnpm lint
pnpm test
pnpm build
pnpm format

# 3. 构建 Hook 接收 CLI（开发模式；桌面程序会在同目录查找）
cargo build -p xushu-hook

# 4. Rust 质量门禁
cargo check --workspace --all-targets --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# 5. 启动桌面程序（开发模式）
pnpm tauri dev
```

首次编译需要下载并构建 Tauri/rusqlite 依赖，耗时较长。`pnpm build` 当前会产生约
979 KB（gzip 280 KB）的单 chunk 及 >500 KB 提示；这是已知优化项，不影响功能。

`xushu-hook.exe` 在开发模式下由 `cargo build -p xushu-hook` 生成；桌面程序从当前可执行文件同目录查找它。正式构建会由 `pnpm sidecar:prepare` 自动编译并按 Tauri `externalBin` 规则打包，安装后位于主程序同目录。

## Windows 正式构建与安装（0.1.1 候选）

```bash
pnpm install --frozen-lockfile
pnpm release:windows
```

使用默认 Cargo 输出目录时，产物路径如下。

```text
target/release/xushu-desktop.exe
target/release/bundle/nsis/XUSHU_0.1.1_x64-setup.exe
```

- 安装器使用 Tauri 官方 NSIS，默认 **currentUser** 安装（无需管理员权限），默认目录 `%LOCALAPPDATA%\XUSHU`。
- 安装包包含主程序与 `xushu-hook.exe`；应用启动时会执行固定的 `xushu-hook --version` 探测，sidecar 缺失、损坏或版本与主程序不一致时，Claude 接入状态显示“接收程序不可用”，不会静默使用不可信接收器。
- 用户数据保存在 `%APPDATA%\dev.xushu.desktop`（SQLite、Hook Spool、Salt、导出、备份工作区等），与安装目录分离。日志位于 `%APPDATA%\dev.xushu.desktop\logs\xushu.log`，有界轮转且只记录固定事件与错误码。
- 升级安装保留用户数据与数据库迁移；升级后 sidecar 摘要或路径变化会显示需要重新安装 Hooks，不会自动修改项目配置。
- 卸载默认保留用户数据库、备份、日志与项目目录。卸载向导中的“删除应用数据”复选框默认不勾选；静默卸载不会删除 `%APPDATA%` 数据。
- 卸载不会删除各项目 `.claude/settings.local.json` 中的 XUSHU Hooks。请在卸载前先在应用内逐项目“卸载 Hooks”；若应用已卸载，需人工从项目设置中删除带 `--xushu-managed` 的条目（先备份该文件）。
- 当前未配置 Windows 代码签名证书（`SIGNING_NOT_CONFIGURED`），安装包不是已签名发行版；未启用自动更新。

历史 Phase 7A 的隔离安装 E2E 可用以下命令运行。

```bash
python scripts/phase7a-e2e.py --installer "<0.1.0 隔离 NSIS 安装包>"
```

该脚本只接受使用 `dev.xushu.e2e.phase7a` identifier 与隔离 `appDirectoriesOverride` 构建的测试安装包，使用独立数据目录，不打开或写入正常用户数据库。 `--installer` 使用 `0.1.0` 基线包，可选的 `--upgrade-installer` 使用 `0.1.1` 隔离升级包。

## 使用流程

1. **项目**：输入本地 Git 工作树路径导入；XUSHU 只读检查，不修改仓库。
2. **任务**：在当前项目中创建任务；初始为草稿。
3. **验收条件**：至少添加一个必需条件，才能将任务推进到就绪。
4. **状态**：按 Rust 状态机推进；阻塞/取消必须填写原因。
5. **人工确认**：任务进入 `待验证` 后，人工确认必需的人工验收条件。
6. **自动验证**：为自动条件配置命令；在验收中心或任务详情预览并批准运行；只有当前有效的 PASS 证据才满足条件。
7. **完成**：所有必需条件满足后，`标记完成` 才可用；失败、未运行、无法判定或过期证据都会阻止完成。
8. **恢复**：重启应用后，项目、任务、条件、确认、验证运行与证据均保留。
9. **Claude Code 接入（可选）**：项目详情 → “Claude Code 接入” → 查看预览并勾选确认 → 安装 Hooks。只写项目本地设置；事件只形成活动时间线，不改变任务状态。停用/卸载只删除 XUSHU 条目。真实联调请使用一次性测试项目，不要自动启用正在使用的项目。
10. **数据保护**：一级导航“数据保护” → 创建备份（用户选择的本地目录）→ 校验备份 → 查看恢复历史与完整性。恢复必须先预检并查看影响范围；输入“恢复”并勾选后才能执行。备份未加密；恢复不包含 Hook Spool、Salt、项目配置或导出报告，也不会自动修改 Hooks。

> 首版不通过 Shell，且拒绝 `cmd`、`powershell`、`sh` 与 `.cmd/.bat/.ps1` 包装器。
> 测试脚本仍以当前用户权限运行，请只对可信项目配置验证。

## 本地数据

- 数据库文件：Tauri 应用数据目录下的 `xushu.db`。Windows 实测为
  `%APPDATA%\dev.xushu.desktop\xushu.db`。
- 数据库及 WAL/SHM 文件不进入 Git 仓库，界面不展示绝对路径。
- Schema 版本 7（迁移 1–7）：
  - `schema_migrations`
  - `persistence_probes`（Phase 1 诊断）
  - `projects`、`tasks`、`acceptance_criteria`、`task_transitions`、`manual_confirmations`
  - `verification_specs`、`verification_runs`、`verification_checks`、`evidence`
  - `claude_integrations`、`claude_sessions`、`claude_tool_calls`、`claude_events`（Phase 4/5；无原始 payload）
  - `project_memories`、`project_memory_revisions`、`decisions`、`decision_revisions`、`project_recovery_preferences`、`recovery_snapshots`（Phase 5）
  - `backup_records`、`backup_settings`、`restore_operations`（Phase 6）
- 开启 `foreign_keys`、WAL、`synchronous=NORMAL` 和 5 秒 busy timeout。
- 迁移在事务中执行，失败回滚；重复执行幂等；数据库版本高于当前应用时拒绝降级。
- 项目/任务归档为软归档；移除不删除磁盘文件或历史数据。
- Claude Hook spool、盐与设置备份位于应用数据目录 `hook-inbox/`、`hook-backups/`；不进入仓库，不通过 IPC 暴露盐或原始 Hook JSON。
- Hook 事件保留 30 天、每项目最多 5000 条；待导入文件每项目上限 1000 条。
- 备份目录由用户选择；每个备份只含 `manifest.json` 与 `xushu.db`。当前备份**未加密**，不包含 Hook Spool、Salt、项目 `.claude` 配置、`exports/` 或 WebView 偏好。
- 恢复回退目录 `restore-rollback/`、恢复日志 `restore-journal.json` 与隔离工作区 `restore-workspaces/` 位于应用数据目录；回退文件不自动删除。

## 工程结构

```text
D:\MyProject\xushu\
├── apps/desktop/
│   ├── src/
│   │   ├── app/                     App Shell、顶部工作带、Providers、主题同步
│   │   ├── components/              WindowControls、PageHeader、状态/状态轨道/历史、shadcn/ui
│   │   ├── features/
│   │   │   ├── projects/            项目 API、导入、Git 状态、操作菜单
│   │   │   ├── tasks/               任务 API、表单、验收条件、状态操作
│   │   │   ├── verification/        验证配置、批准、运行状态、证据、历史、报告
│   │   │   ├── claude/              Claude Code 接入状态、安装预览、活动时间线
│   │   │   ├── knowledge/           项目记忆、开发决策、修订历史
│   │   │   ├── recovery/            跨会话恢复包、复制指令、历史快照
│   │   │   ├── data-protection/     数据保护、创建备份、策略、校验、恢复向导、隐私清单
│   │   │   ├── workspace/           当前项目、Signal Rail
│   │   │   ├── system/              应用信息与数据库健康
│   │   │   └── persistence/         Phase 1 持久化探针
│   │   ├── pages/                   工作台、项目、任务、验收中心、恢复、数据保护、诊断、设置
│   │   ├── lib/                     IPC、领域类型、格式化、验证展示、Zod 辅助
│   │   └── stores/                  主题、当前项目 Zustand
│   ├── src-tauri/
│   │   ├── src/commands.rs          Phase 1 五个命令
│   │   ├── src/business_commands.rs 项目/任务/验收/验证/工作台命令
│   │   ├── src/claude_commands.rs   Claude Code 接入/活动/安装/停用命令
│   │   ├── src/knowledge_commands.rs 记忆/决策命令
│   │   ├── src/recovery_commands.rs 恢复包命令
│   │   ├── src/data_protection_commands.rs 备份/恢复/隐私命令
│   │   ├── src/state.rs             数据库、后台验证线程、Claude 运行时、备份/恢复互斥与预检令牌
│   │   ├── src/window_chrome.rs     自定义窗口边缘与 Snap 覆盖层（Windows）
│   │   └── capabilities/default.json
│   └── components.json
├── crates/xushu-core/
│   ├── migrations/                  迁移 1–7（探针、业务、验证、Hooks、记忆/恢复、备份/恢复）
│   ├── src/db/                      数据库、PRAGMA、迁移、事务、健康检查、Online Backup API
│   ├── src/git.rs                   只读 Git 检查（参数数组、超时、输出上限）
│   ├── src/code_state.rs            HEAD/分支/工作区内容指纹
│   ├── src/runner.rs                后台进程、超时、取消、Windows Job Object
│   ├── src/redaction.rs             输出模式化脱敏
│   ├── src/project.rs               项目 Repository/Service
│   ├── src/task.rs                  任务/验收/确认 Repository/Service
│   ├── src/verification.rs          验证配置、运行、证据、过期与报告
│   ├── src/state_machine.rs         状态转换图
│   ├── src/models.rs                领域模型
│   ├── src/claude/                  Hooks 配置、spool 导入、会话/工具状态与活动查询
│   ├── src/backup/                  清单、路径安全、完整性、备份服务、恢复/回退、数据保护总览
│   └── tests/                       持久化 + 业务 + 验证 + Claude + 记忆/决策/恢复 + 备份/恢复测试
├── crates/xushu-hook-protocol/      Hook 白名单协议、带盐哈希与信封校验
├── crates/xushu-hook/               本地 Hook CLI（独立可执行文件）
├── docs/
│   ├── architecture/ADR/            0001 技术栈 … 0007 Claude Hooks、0008 记忆/决策/恢复、0009 备份/恢复/数据保护
│   ├── architecture/DATA_PROTECTION.md 数据资产、威胁模型、备份格式与恢复规范
│   ├── design/DESIGN_SYSTEM.md      XUSHU 设计系统
│   ├── design/UI_AUDIT.md           UI 审计与人工视觉验收状态
│   └── phases/                      Phase 0–6 报告与独立审计
├── Cargo.toml / Cargo.lock
├── package.json / pnpm-workspace.yaml / pnpm-lock.yaml
└── rust-toolchain.toml
```

## 设计系统与视觉验收

- 统一设计系统：[`docs/design/DESIGN_SYSTEM.md`](docs/design/DESIGN_SYSTEM.md)（v2.4 / Phase 6 数据保护）
- UI 审计：[`docs/design/UI_AUDIT.md`](docs/design/UI_AUDIT.md)
- Phase 3.5 重设计报告：[`docs/design/UI_REDESIGN_PHASE_3_5.md`](docs/design/UI_REDESIGN_PHASE_3_5.md)
- Phase 3.6 窗口一体化报告：[`docs/design/UI_REDESIGN_PHASE_3_6.md`](docs/design/UI_REDESIGN_PHASE_3_6.md)
- 使用 `frontend-design` Skill 制定并执行统一视觉语言；冷石墨 + 氧化青 + 黄铜色板，顶部工作带、项目上下文、状态序列与证据终端作为“序枢”识别元素。
- 自定义标题栏：44px 顶部工作带 + 46×44px 窗口控制 + Windows Snap `HTMAXBUTTON` 覆盖层。窗口取舍见 [`ADR-0006`](docs/architecture/ADR/0006-custom-titlebar-window-chrome.md)（取代 ADR-0005）。
- **视觉状态：VISUAL_REVIEW_PENDING。** 自动化类型/Lint/测试/构建与窗口尺寸测量通过不等于视觉验收通过；需要用户手动截图确认深浅主题、Snap 悬停菜单、1366/1600/1920/500 窗口、空/错误/待验证/FAIL/STALE 状态。

## 安全与隐私基线

- 前端只调用显式注册的 Tauri Commands；没有任意 SQL、Shell 或通用文件系统权限。
- Tauri capabilities 为 `core:default` + 最小窗口控制权限（`set-theme`、`start-dragging`、`internal-toggle-maximize`、`minimize`、`toggle-maximize`、`close`、`is-maximized`）；未启用 `withGlobalTauri`。
- Git 只读检查使用可执行文件 + 参数数组，5 秒超时；不执行任何 Git 写操作。
- 验证命令同样使用可执行程序 + 参数数组，不经过 Shell；拒绝 shell 与 .cmd/.bat/.ps1 包装器。
- 验证必须由用户在执行前预览命令、风险与工作目录，并显式批准；前端不能在执行时提交任意命令。
- Windows 使用 Job Object 绑定子进程树；取消、超时和应用退出都会清理子进程。
- 输出按上限截断并在落库前做模式化脱敏；脱敏不是 100% 保证，证据仍属本地敏感数据。
- Rust 对项目/任务/验收/验证输入再次校验；IPC 错误返回 `{ code, message }`，不返回绝对路径、SQL 或凭据。
- 状态机、证据有效性与完成门禁只在 Rust 实现；前端只展示后端返回的允许转换与 blockers。
- CSP 保持生产/开发两套策略；生产 `script-src 'self'`。
- 不读取或输出 API Key、`.env`、SSH 私钥、token 等敏感信息。
- 备份使用 SQLite Online Backup API；运行时不复制 `.db`；清单只允许一个 `xushu.db` 相对路径；拒绝 UNC、Junction、符号链接、路径穿越与未来 Schema。
- 恢复必须经过只读预检、隔离验证、一次性 token、输入“恢复”、勾选确认、回退副本、受控切换与最终校验；失败优先回退，启动时检查恢复日志。
- 备份未加密；不包含 Hook Spool、Salt、项目 `.claude` 配置、导出报告或 WebView 偏好；恢复不自动修改 Hooks。
- 不展示虚假项目、任务、KPI 或自动验收 PASS。

## 文档

- [MASTER_DEVELOPMENT_PLAN.md](MASTER_DEVELOPMENT_PLAN.md)
- [CLAUDE.md](CLAUDE.md)
- [依赖与工具链基线](docs/architecture/DEPENDENCIES.md)
- [ADR-0001](docs/architecture/ADR/0001-local-first-tauri-stack.md)
- [ADR-0002](docs/architecture/ADR/0002-sqlite-persistence-and-ipc.md)
- [ADR-0003](docs/architecture/ADR/0003-projects-tasks-state-machine.md)
- [ADR-0004](docs/architecture/ADR/0004-independent-verification-engine.md)
- [ADR-0005](docs/architecture/ADR/0005-native-window-integrated-shell.md)（已被 ADR-0006 取代）
- [ADR-0006](docs/architecture/ADR/0006-custom-titlebar-window-chrome.md)
- [ADR-0007](docs/architecture/ADR/0007-claude-code-hooks-integration.md)
- [ADR-0008](docs/architecture/ADR/0008-project-memory-decisions-recovery.md)
- [ADR-0009](docs/architecture/ADR/0009-backup-restore-data-protection.md)
- [数据保护规范](docs/architecture/DATA_PROTECTION.md)
- [Phase 0 报告](docs/phases/PHASE_0_REPORT.md)
- [Phase 1 报告](docs/phases/PHASE_1_REPORT.md)
- [Phase 2 报告](docs/phases/PHASE_2_REPORT.md)
- [Phase 3 报告](docs/phases/PHASE_3_REPORT.md)
- [Phase 4 报告](docs/phases/PHASE_4_REPORT.md)
- [Phase 5 报告](docs/phases/PHASE_5_REPORT.md)
- [Phase 6 报告](docs/phases/PHASE_6_REPORT.md)
- [设计系统](docs/design/DESIGN_SYSTEM.md)
- [UI 审计](docs/design/UI_AUDIT.md)
- [Phase 3.5 UI 重设计报告](docs/design/UI_REDESIGN_PHASE_3_5.md)
- [Phase 3.6 窗口一体化报告](docs/design/UI_REDESIGN_PHASE_3_6.md)
