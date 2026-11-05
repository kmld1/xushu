# 序枢 XUSHU

本地优先、面向独立开发者的 AI 工作控制系统。

目标是把 AI 参与的开发工作组织成可追溯、可验证的闭环：

> 目标与验收标准 → 用户授权的执行事件 → 独立运行验证 → 证据与决策留存 → 断点恢复

XUSHU 不替代 Claude Code、编辑器或终端，也不把 AI 的“已完成”文本当作完成证据。

## 当前状态

**Phase 0（仓库体检与工程启动）：PASS（2026-11-05）。**

本轮已完成并实测：

- Tauri 2 + React + TypeScript + Vite 最小桌面工程可编译、可启动；
- pnpm workspace 与 Cargo workspace 可独立复现安装；
- `pnpm-lock.yaml` 与 `Cargo.lock` 已生成，冻结锁文件检查通过；
- TypeScript 类型检查、ESLint、Prettier、Vitest、Vite 生产构建全部通过；
- `cargo check --locked`、`cargo fmt --check`、`cargo clippy -D warnings`、`cargo test` 全部通过；
- 实际启动桌面程序，确认主窗口与 WebView2 正常创建。

Phase 0 只包含工程骨架，**尚未实现**数据库、IPC、项目/任务管理、验收引擎、Hooks、路由与产品页面。详见 [`docs/phases/PHASE_0_REPORT.md`](docs/phases/PHASE_0_REPORT.md)。

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
| Claude Code 接入 | 项目级 Hooks（用户显式授权）+ `xushu-hook` CLI |
| 测试 | Vitest + React Testing Library + `cargo test` |
| 包管理 | pnpm + Cargo workspace |

> 上表是 V1 的冻结目标。Phase 0 只安装了实际用到的 React/Vite/Tailwind/Tauri 与质量工具；Router、Query、Zustand、RHF、Zod、shadcn/ui、rusqlite 等将在对应阶段引入。

## 环境先决条件

| 组件 | 要求 | 当前实测 |
|---|---|---|
| Windows | Windows 11 x64 | 10.0.26200 ✅ |
| Node.js | `^22.22.2` 或 `^24.15.0`（jsdom 31 / Vite 8） | v24.20.0 ✅ |
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

`pnpm tauri dev` 会先启动 Vite（`http://localhost:1420`），再编译并运行 `xushu-desktop.exe`。首次编译需要下载并构建 Tauri 依赖，耗时较长。

## 工程结构

```text
D:\MyProject\xushu\
├── apps/
│   └── desktop/
│       ├── src/                      React 入口、页面骨架、Vitest 测试
│       ├── src-tauri/                Tauri Rust 应用
│       │   ├── capabilities/         最小能力集（core:default）
│       │   ├── icons/                桌面图标
│       │   ├── src/                  main.rs / lib.rs
│       │   ├── build.rs
│       │   └── tauri.conf.json
│       ├── index.html
│       ├── vite.config.ts
│       ├── vitest.config.ts
│       ├── eslint.config.js
│       └── tsconfig.json
├── crates/
│   └── xushu-core/                   核心领域库骨架（Phase 0 仅常量与测试）
├── docs/
│   ├── architecture/
│   │   ├── ADR/                      架构决策
│   │   └── DEPENDENCIES.md           实测工具链与锁定版本
│   └── phases/
│       └── PHASE_0_REPORT.md         Phase 0 验收报告（含首轮阻塞历史）
├── Cargo.toml                        Cargo workspace
├── Cargo.lock
├── package.json                      pnpm 根工具链
├── pnpm-workspace.yaml
├── pnpm-lock.yaml
└── rust-toolchain.toml
```

`apps/desktop/src-tauri/gen/` 为 Tauri 构建期生成物，已被 `.gitignore` 排除。移动端图标未生成；当前目标是 Windows 桌面，macOS 后续适配。

## 文档

- [MASTER_DEVELOPMENT_PLAN.md](MASTER_DEVELOPMENT_PLAN.md)
- [CLAUDE.md](CLAUDE.md)
- [依赖与工具链基线](docs/architecture/DEPENDENCIES.md)
- [ADR-0001](docs/architecture/ADR/0001-local-first-tauri-stack.md)
- [Phase 0 报告](docs/phases/PHASE_0_REPORT.md)

## 安全与隐私基线

- 不读取或输出 API Key、`.env`、SSH 私钥、token 等敏感信息。
- 前端只调用受限 Tauri Commands；数据库、路径与命令权限在 Rust 侧校验。
- 验证命令使用可执行文件 + 参数数组，禁止 shell 字符串拼接；执行前需用户确认。
- Claude Code Hook 默认只采集白名单元数据，不采集 prompt 原文、工具完整输入输出、transcript 或源码。
- 本地优先：V1 不依赖云服务、登录系统或远程数据库。
- Phase 0 的 Tauri CSP 仍为 `null`；CSP 收紧属于后续安全加固任务，不能把当前状态视为最终安全配置。