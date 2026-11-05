# 序枢 XUSHU

本地优先、面向独立开发者的 AI 工作控制系统。

目标是把 AI 参与的开发工作组织成可追溯、可验证的闭环：

> 目标与验收标准 → 用户授权的执行事件 → 独立运行验证 → 证据与决策留存 → 断点恢复

XUSHU 不替代 Claude Code、编辑器或终端，也不把 AI 的“已完成”文本当作完成证据。

## 当前状态

**Phase 1（桌面基础与本地持久化）：PASS（2026-11-06）。**

Phase 1 已实际完成并验证：

- Rust SQLite 持久化：WAL、外键、事务、完整性检查；
- 版本化迁移：`schema_migrations` 记录、失败回滚、重复执行幂等、拒绝 schema 降级；
- 受限 Tauri IPC：应用信息、数据库健康、写入/列表/按 ID 读取测试记录；
- React 桌面外壳：HashRouter、侧边导航、深浅主题、shadcn/ui 基础组件、错误边界、加载/错误/空状态；
- 真实跨进程验证：写入记录 → 完全关闭应用 → 重新启动 → 通过 UI/IPC 读回同一 ID、内容与时间。

**尚未实现（Phase 2+）：** 项目/任务/验收状态机、独立验收命令执行、Claude Code Hooks、决策记忆、备份恢复、安装包与 macOS 适配。Phase 1 只有一个明确标记的 `persistence_probes` 测试表，不是业务数据模型。

详见 [`docs/phases/PHASE_1_REPORT.md`](docs/phases/PHASE_1_REPORT.md)。

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
| Git | Rust 以参数数组只读调用本机 `git`（后续阶段） |
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

`pnpm tauri dev` 会先启动 Vite（`http://localhost:1420`），再编译并运行 `xushu-desktop.exe`。首次编译需要下载并构建 Tauri/rusqlite 依赖，耗时较长。

如果系统内存紧张，可限制 Cargo 并行编译数：

```bash
CARGO_BUILD_JOBS=2 pnpm tauri dev
```

## 本地数据

- 数据库文件：Tauri 应用数据目录下的 `xushu.db`。Windows 实测为
  `%APPDATA%\dev.xushu.desktop\xushu.db`。
- 数据库及 WAL/SHM 文件不进入 Git 仓库，界面不展示绝对路径。
- Phase 1 数据表：
  - `schema_migrations`：迁移版本记录；
  - `persistence_probes`：持久化测试记录（非 Phase 2 业务表）。
- 开启 `foreign_keys`、WAL、`synchronous=NORMAL` 和 5 秒 busy timeout。
- 迁移在事务中执行，失败回滚；重复执行幂等；数据库版本高于当前应用时拒绝降级。

## 工程结构

```text
D:\MyProject\xushu\
├── apps/desktop/
│   ├── src/
│   │   ├── app/                    布局、路由 Providers、主题同步
│   │   ├── components/             错误边界、主题控件、shadcn/ui 组件
│   │   ├── features/
│   │   │   ├── persistence/        测试记录 API、查询、表单、列表
│   │   │   └── system/             应用信息与数据库健康查询
│   │   ├── lib/                    IPC 包装、类名工具
│   │   ├── pages/                  概览、本地数据、设置、404
│   │   ├── stores/                 主题 Zustand store
│   │   └── test/                   Vitest 设置
│   ├── src-tauri/
│   │   ├── src/commands.rs         五个 IPC 命令
│   │   ├── src/error.rs            结构化 IPC 错误
│   │   ├── src/state.rs            应用状态与数据库初始化
│   │   ├── src/lib.rs              Tauri Builder
│   │   ├── capabilities/default.json
│   │   └── tauri.conf.json         CSP / devCsp / 窗口配置
│   └── components.json             shadcn/ui 配置
├── crates/xushu-core/
│   ├── migrations/                 版本化 SQL 迁移
│   ├── src/db/                     数据库、PRAGMA、迁移、事务、健康检查
│   ├── src/probe.rs                测试记录校验与持久化
│   ├── src/error.rs                CoreError
│   └── tests/persistence.rs        12 个持久化集成测试
├── docs/
│   ├── architecture/
│   │   ├── ADR/                    0001 技术栈、0002 SQLite/IPC
│   │   └── DEPENDENCIES.md
│   └── phases/
│       ├── PHASE_0_REPORT.md
│       └── PHASE_1_REPORT.md
├── Cargo.toml / Cargo.lock
├── package.json / pnpm-workspace.yaml / pnpm-lock.yaml
└── rust-toolchain.toml
```

## 安全与隐私基线

- 前端只调用显式注册的五个 Tauri Commands；没有任意 SQL、Shell 或通用文件系统权限。
- Tauri capabilities 仅授予 `core:default`。
- Rust 对写入输入再次校验；IPC 错误返回 `{ code, message }`，不返回绝对路径、SQL 或凭据。
- CSP 已配置生产与开发两套策略：生产 `script-src 'self'`；开发额外允许 Vite HMR。
- 不读取或输出 API Key、`.env`、SSH 私钥、token 等敏感信息。
- Claude Code Hooks 尚未实现；V1 不依赖云服务、登录系统或远程数据库。
- 生产 CSP、安装包与 macOS 尚未实机验证，不能把当前 PASS 视为最终发布验证。

## 文档

- [MASTER_DEVELOPMENT_PLAN.md](MASTER_DEVELOPMENT_PLAN.md)
- [CLAUDE.md](CLAUDE.md)
- [依赖与工具链基线](docs/architecture/DEPENDENCIES.md)
- [ADR-0001](docs/architecture/ADR/0001-local-first-tauri-stack.md)
- [ADR-0002](docs/architecture/ADR/0002-sqlite-persistence-and-ipc.md)
- [Phase 0 报告](docs/phases/PHASE_0_REPORT.md)
- [Phase 1 报告](docs/phases/PHASE_1_REPORT.md)