# 序枢 XUSHU

本地优先、面向独立开发者的 AI 工作控制系统。

目标是把 AI 参与的开发工作组织成可追溯、可验证的闭环：

> 目标与验收标准 → 用户授权的执行事件 → 独立运行验证 → 证据与决策留存 → 断点恢复

XUSHU 不替代 Claude Code、编辑器或终端，也不把 AI 的“已完成”文本当作完成证据。

## 当前状态

**Phase 2（项目管理、任务系统与状态机）：PASS（2026-11-06）。**

已实际完成并验证：

- 导入本地 Git 项目，只读检查根目录、分支、HEAD、dirty、unborn 与 detached HEAD；
- 项目重命名、软归档与恢复；不删除磁盘项目或任务数据；
- 创建/编辑/归档/恢复任务，按项目、状态、关键词筛选；
- 验收条件新增/编辑/删除，自动/人工类型与必需/可选；
- Rust 统一状态机：draft / ready / running / awaiting_verification / done / blocked / cancelled；
- 人工确认记录与修订绑定；编辑任务/条件或返工后旧确认失效；
- 完成门禁：必需条件未满足时禁止 done；Phase 2 没有自动验证引擎，必需自动条件不会伪造成 PASS；
- 真实桌面 E2E：导入项目 → 创建任务 → 添加人工验收 → 推进状态 → 确认 → 完成 → 重启后仍为 done 且确认保留。

**尚未实现（Phase 3+）：** 自动验证命令执行器、真实自动验收证据、证据过期判定、Claude Code Hooks、决策记忆、备份恢复、安装包与 macOS 适配。

详见 [`docs/phases/PHASE_2_REPORT.md`](docs/phases/PHASE_2_REPORT.md)。

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
| Claude Code 接入 | 项目级 Hooks（Phase 4，尚未实现） |
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

# 3. Rust 质量门禁
cargo check --workspace --all-targets --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# 4. 启动桌面程序（开发模式）
pnpm tauri dev
```

首次编译需要下载并构建 Tauri/rusqlite 依赖，耗时较长。`pnpm build` 当前会产生约
719 KB（gzip 220 KB）的单 chunk 及 >500 KB 提示；这是已知优化项，不影响功能。

## 使用流程

1. **项目**：输入本地 Git 工作树路径导入；XUSHU 只读检查，不修改仓库。
2. **任务**：在当前项目中创建任务；初始为草稿。
3. **验收条件**：至少添加一个必需条件，才能将任务推进到就绪。
4. **状态**：按 Rust 状态机推进；阻塞/取消必须填写原因。
5. **人工确认**：任务进入 `待验证` 后，人工确认必需的人工验收条件。
6. **完成**：所有必需条件满足后，`标记完成` 才可用；含必需自动条件的任务在 Phase 3 前不能完成。
7. **恢复**：重启应用后，项目、任务、条件、确认与状态历史均保留。

## 本地数据

- 数据库文件：Tauri 应用数据目录下的 `xushu.db`。Windows 实测为
  `%APPDATA%\dev.xushu.desktop\xushu.db`。
- 数据库及 WAL/SHM 文件不进入 Git 仓库，界面不展示绝对路径。
- Schema 版本 2：
  - `schema_migrations`
  - `persistence_probes`（Phase 1 诊断）
  - `projects`
  - `tasks`
  - `acceptance_criteria`
  - `task_transitions`
  - `manual_confirmations`
- 开启 `foreign_keys`、WAL、`synchronous=NORMAL` 和 5 秒 busy timeout。
- 迁移在事务中执行，失败回滚；重复执行幂等；数据库版本高于当前应用时拒绝降级。
- 项目/任务归档为软归档；移除不删除磁盘文件或历史数据。

## 工程结构

```text
D:\MyProject\xushu\
├── apps/desktop/
│   ├── src/
│   │   ├── app/                     App Shell、Providers、主题同步
│   │   ├── components/              PageHeader、状态/状态轨道/历史、shadcn/ui
│   │   ├── features/
│   │   │   ├── projects/            项目 API、导入、Git 状态、操作菜单
│   │   │   ├── tasks/               任务 API、表单、验收条件、状态操作
│   │   │   ├── workspace/           当前项目、Signal Rail
│   │   │   ├── system/              应用信息与数据库健康
│   │   │   └── persistence/         Phase 1 持久化探针
│   │   ├── pages/                   工作台、项目、项目详情、任务、任务详情、诊断、设置
│   │   ├── lib/                     IPC、领域类型、格式化、Zod 辅助
│   │   └── stores/                  主题、当前项目 Zustand
│   ├── src-tauri/
│   │   ├── src/commands.rs          Phase 1 五个命令
│   │   ├── src/business_commands.rs 项目/任务/验收/工作台命令
│   │   ├── src/state.rs             数据库状态
│   │   └── capabilities/default.json
│   └── components.json
├── crates/xushu-core/
│   ├── migrations/                  迁移 1（探针）+ 迁移 2（业务表）
│   ├── src/db/                      数据库、PRAGMA、迁移、事务、健康检查
│   ├── src/git.rs                   只读 Git 检查（参数数组、超时）
│   ├── src/project.rs               项目 Repository/Service
│   ├── src/task.rs                  任务/验收/确认 Repository/Service
│   ├── src/state_machine.rs         状态转换图
│   ├── src/models.rs                领域模型
│   └── tests/                       持久化 + 业务集成测试
├── docs/
│   ├── architecture/ADR/            0001 技术栈、0002 SQLite/IPC、0003 项目/任务/状态机
│   ├── design/DESIGN_SYSTEM.md      XUSHU 设计系统
│   ├── design/UI_AUDIT.md           UI 审计与人工视觉验收状态
│   └── phases/                      Phase 0/1/2 报告
├── Cargo.toml / Cargo.lock
├── package.json / pnpm-workspace.yaml / pnpm-lock.yaml
└── rust-toolchain.toml
```

## 设计系统与视觉验收

- 统一设计系统：[`docs/design/DESIGN_SYSTEM.md`](docs/design/DESIGN_SYSTEM.md)
- UI 审计：[`docs/design/UI_AUDIT.md`](docs/design/UI_AUDIT.md)
- 使用 `frontend-design` Skill 制定并执行统一视觉语言；冷石墨 + 氧化青 + 黄铜色板，状态轨道/历史作为“序枢”识别元素。
- **视觉状态：VISUAL_REVIEW_PENDING。** 自动化类型/Lint/测试/构建通过不等于视觉验收通过；需要用户手动截图确认深浅主题、900/1400px 窗口、空/错误/待验证状态。

## 安全与隐私基线

- 前端只调用显式注册的 Tauri Commands；没有任意 SQL、Shell 或通用文件系统权限。
- Tauri capabilities 仅 `core:default`。
- Git 只读检查使用可执行文件 + 参数数组，5 秒超时；不执行任何 Git 写操作。
- Rust 对项目/任务/验收输入再次校验；IPC 错误返回 `{ code, message }`，不返回绝对路径、SQL 或凭据。
- 状态机与完成门禁只在 Rust 实现；前端只展示后端返回的允许转换与 blockers。
- CSP 保持生产/开发两套策略；生产 `script-src 'self'`。
- 不读取或输出 API Key、`.env`、SSH 私钥、token 等敏感信息。
- 不展示虚假项目、任务、KPI 或自动验收 PASS。

## 文档

- [MASTER_DEVELOPMENT_PLAN.md](MASTER_DEVELOPMENT_PLAN.md)
- [CLAUDE.md](CLAUDE.md)
- [依赖与工具链基线](docs/architecture/DEPENDENCIES.md)
- [ADR-0001](docs/architecture/ADR/0001-local-first-tauri-stack.md)
- [ADR-0002](docs/architecture/ADR/0002-sqlite-persistence-and-ipc.md)
- [ADR-0003](docs/architecture/ADR/0003-projects-tasks-state-machine.md)
- [Phase 0 报告](docs/phases/PHASE_0_REPORT.md)
- [Phase 1 报告](docs/phases/PHASE_1_REPORT.md)
- [Phase 2 报告](docs/phases/PHASE_2_REPORT.md)
- [设计系统](docs/design/DESIGN_SYSTEM.md)
- [UI 审计](docs/design/UI_AUDIT.md)