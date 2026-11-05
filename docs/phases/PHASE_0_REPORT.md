# Phase 0 报告：仓库体检与工程启动

- 日期：2026-11-05
- 项目：序枢 XUSHU
- 根目录：`D:\MyProject\xushu`
- 首轮总体状态：**BLOCKED**（历史记录，已由第二轮结果取代）
- 结论：只读体检、官方版本核对、仓库文档初始化已完成；Tauri/React 脚手架与所有编译/类型/启动验证未执行，原因是缺少 Rust/Cargo、MSVC C++ Build Tools、pnpm。未伪造任何 PASS。

> **第二轮修订（2026-11-05 晚间）：** 用户已补齐 Rust/Cargo、MSVC Build Tools、Windows SDK 与 pnpm；本轮重新执行 Phase 0，脚手架、锁文件、全部质量门禁与桌面启动均已实际验证，最终状态为 **PASS**。
>
> 第 1–13 节保留首轮 BLOCKED 的原始记录，不修改历史；第二轮的实测、变更、证据与最终验收见第 14–23 节。

## 1. 本轮目标与边界

**目标：** 只执行 Phase 0 —— 仓库体检、工具链检查、版本核对、在不覆盖前提下初始化工程基础、执行可执行的验证、生成报告。

**边界：**

- 不进入 Phase 1；
- 不安装 Hooks、不注册系统服务、不修改全局 Claude/DeepSeek 配置；
- 不执行需要管理员权限的安装；
- 不使用子 Agent / 多 Agent；
- 不截图 / 录屏；
- 未验证项不得标记 PASS。

## 2. 启动前只读检查

### 2.1 目录内容

命令：`ls -la D:/MyProject/xushu`

结果：退出码 0。仅存在：

- `CLAUDE.md`（8,372 bytes，mtime 2026-11-05 18:47:18 +0800）
- `MASTER_DEVELOPMENT_PLAN.md`（32,474 bytes，mtime 2026-11-05 18:47:42 +0800）

未发现代码、构建产物、隐藏配置或第三方文件。

### 2.2 Git 状态

命令：`git -C D:/MyProject/xushu status --short --branch`

结果：退出码 128，`fatal: not a git repository (or any of the parent directories): .git`。

结论：初始状态不是 Git 仓库。本轮在文件初始化后执行了 `git init -b main`（见第 5 节），未提交、未配置 remote、未修改全局 Git 配置。

## 3. 工具链检查

| 检查 | 命令 | 结果 | 判定 |
|---|---|---|---|
| Node.js | `node --version` | `v24.20.0`，退出码 0 | PASS |
| npm | `npm --version` | `11.19.0`，退出码 0 | PASS（辅助） |
| Corepack | `corepack --version` | `0.35.0`，退出码 0 | PASS |
| pnpm | `pnpm --version` | `command not found`，退出码 127 | MISSING |
| Git | `git --version` | `2.56.0.windows.2`，退出码 0 | PASS |
| rustc | `rustc --version` | `command not found`，退出码 127 | MISSING |
| cargo | `cargo --version` | `command not found`，退出码 127 | MISSING |
| rustup | `rustup show` | `command not found`，退出码 127 | MISSING |
| MSVC C++ Build Tools | vswhere / VS 目录 / `cl.exe` / VS7 注册表 | vswhere 不存在；VS 安装目录不存在；`cl.exe` 不在 PATH；VS7 键不存在 | MISSING |
| Windows SDK | 注册表 + 目录 | `KitsRoot10` 指向 `C:\Program Files\Windows Kits\10\`，但 `Include` / `Lib` / `bin` 不存在，仅有 `Catalogs` | MISSING（不可用） |
| WebView2 | `HKLM\...\EdgeUpdate\Clients\{F3017226-...}` 的 `pv` | `156.0.4314.58` | PASS |

补充：

- Node 安装路径：`D:\nodejs\node.exe`；npm：`D:\nodejs\npm.cmd`。
- `where.exe cargo/rustc/pnpm` 均未找到。
- `%USERPROFILE%\.cargo\bin` 不存在。
- WebView2 用户级注册表键不存在，但机器级 Runtime 已安装。

## 4. 官方版本与兼容性核对

使用 Tavily MCP 检索/读取官方资料（2026-11-05）：

1. Tauri 官方 Windows 先决条件：Microsoft C++ Build Tools（勾选 “Desktop development with C++”）、WebView2、Rust。
2. Tauri 2.12 发布说明：MSRV 提高到 **Rust 1.90**；不再官方支持 Windows 7（本项目目标 Windows 11，不受影响）。
3. 官方 npm registry：
   - `@tauri-apps/cli` / `@tauri-apps/api`：`2.12.4`。
   - `vite`：`engines.node = ^20.19.0 || >=22.12.0`。
   - `typescript-eslint`：`8.72.1`，peer `typescript >=4.8.4 <6.1.0`、`eslint ^8.57 || ^9 || ^10`。
   - `typescript@6.0.2` 存在；`@tauri-apps/api` 的开发依赖使用 `typescript ^6.0.3`。
4. Rust 官方 stable channel manifest（2026-10-15）：`cargo 1.99.1`，即当前 stable 为 **1.99.1**。
5. TypeScript 7.0 兼容性：TS 7.0 为原生编译器，7.0 不提供编译器 API；typescript-eslint 8.72.1 不支持，直接使用会 `ERESOLVE` 或运行时崩溃。因此候选基线选择 TypeScript 6.0.x。

详细版本表见 `docs/architecture/DEPENDENCIES.md`。

## 5. 本轮实际变更

| 文件 | 操作 | 说明 |
|---|---|---|
| `README.md` | 新建 | 项目定位、当前 BLOCKED 状态、先决条件、解除阻塞步骤、计划结构 |
| `.gitignore` | 新建 | 排除 Node/Rust/Tauri 产物、SQLite 运行时、日志、密钥、安装包、OS/编辑器文件 |
| `docs/architecture/DEPENDENCIES.md` | 新建 | 工具链实测、候选版本、兼容性结论、锁定策略、官方参考 |
| `docs/architecture/ADR/0001-local-first-tauri-stack.md` | 新建 | 首份 ADR：Tauri 2 + React + Rust/SQLite 本地优先架构 |
| `docs/phases/PHASE_0_REPORT.md` | 新建 | 本报告 |
| `node_modules/`、`package.json`、`pnpm-lock.yaml`、`Cargo.toml`、`Cargo.lock`、`apps/`、`crates/` | 未创建 | 工具链缺失，按 `CLAUDE.md` 第五节第 4 条不生成无法验证的脚手架 |

**未修改/覆盖的文件：**

- `CLAUDE.md`（mtime 保持 2026-11-05 18:47:18 +0800）
- `MASTER_DEVELOPMENT_PLAN.md`（mtime 保持 2026-11-05 18:47:42 +0800）

**Git：**

- 执行 `git init -b main`，退出码 0。
- 未执行 `git add`、`git commit`、`git remote`；仓库当前全部文件为未跟踪状态。
- 未修改全局 Git 配置。

## 6. 实际执行的命令与结果

| 命令 | 退出码 | 结果 |
|---|---|---|
| `ls -la D:/MyProject/xushu` | 0 | 仅两个计划/规则文件 |
| `git -C ... status --short --branch` | 128 | 初始非 Git 仓库 |
| `node --version` | 0 | v24.20.0 |
| `npm --version` | 0 | 11.19.0 |
| `corepack --version` | 0 | 0.35.0 |
| `pnpm --version` | 127 | command not found |
| `git --version` | 0 | 2.56.0.windows.2 |
| `rustc --version` | 127 | command not found |
| `cargo --version` | 127 | command not found |
| `rustup show` | 127 | command not found |
| vswhere 查询 | — | vswhere 不存在 |
| WebView2 注册表查询 | 0 | 156.0.4314.58 |
| Windows Kits 目录检查 | 0 | 仅有 `Catalogs` |
| `curl .../channel-rust-stable.toml` | 0 | date 2026-10-15；cargo 1.99.1 |
| `git init -b main` | 0 | 初始化空仓库（main） |
| `git status --short` | 0 | 列出新建的未跟踪文件 |

## 7. 测试矩阵

| 项目 | 结果 | 说明 |
|---|---|---|
| 目录只读检查 | PASS | 目录存在，仅两个既有文件 |
| 已有文件未被覆盖 | PASS | 本轮未写入 `CLAUDE.md` / `MASTER_DEVELOPMENT_PLAN.md`；mtime 未变 |
| Node.js 可用 | PASS | v24.20.0 |
| Git 可用 | PASS | 2.56.0.windows.2 |
| WebView2 可用 | PASS | 156.0.4314.58 |
| pnpm 可用 | FAIL（环境） | 未安装/未启用 |
| Rust/Cargo 可用 | FAIL（环境） | 未安装 |
| MSVC C++ Build Tools | FAIL（环境） | 未安装 |
| Windows SDK | FAIL（环境） | 目录不完整 |
| `pnpm install --frozen-lockfile` | NOT_RUN | 无 `package.json`/lockfile，且 pnpm 缺失 |
| `pnpm typecheck` / `tsc --noEmit` | NOT_RUN | 前端脚手架未创建 |
| `cargo check` | NOT_RUN | Cargo workspace 未创建，cargo 缺失 |
| `cargo fmt --check` | NOT_RUN | 同上 |
| `cargo clippy` | NOT_RUN | 同上 |
| Vitest / RTL | NOT_RUN | 前端未创建 |
| Tauri 桌面启动 | NOT_RUN | 脚手架与工具链均缺失 |
| ADR 存在 | PASS | `docs/architecture/ADR/0001-...md` |
| 阶段报告存在 | PASS | 本文件 |

## 8. 安全 / 隐私 / 权限检查

- 未读取或输出 API Key、`.env`、SSH 私钥、token、credential store。
- 未修改全局 Claude Code / DeepSeek 配置、模型、endpoint、API Key、Shell 环境变量、账单设置。
- 未执行管理员权限安装；未安装 pnpm、Rust、MSVC 或任何系统依赖。
- 未安装 Claude Code Hooks、未注册系统服务、未修改其他项目。
- 未使用子 Agent / 多 Agent；未截图/录屏。
- 未创建数据库、未执行任何 SQL、未写入敏感数据。
- 新增文档不包含真实密钥或个人信息。
- `.gitignore` 已排除 `.env*`、密钥、日志、SQLite 运行时文件、构建产物与安装包。

## 9. 阻塞与解除方法

### 阻塞 1：Rust/Cargo 缺失

**影响：** 无法创建/验证 Cargo workspace、`cargo check`、Tauri Rust 后端、SQLite 层。

**最小人工步骤：**

1. 从 <https://rustup.rs/> 下载 `rustup-init.exe`。
2. 使用默认 host `x86_64-pc-windows-msvc` 安装。
3. 重开终端，验证：

   ```bash
   rustc --version
   cargo --version
   rustup show
   ```

### 阻塞 2：MSVC C++ Build Tools 缺失

**影响：** Rust MSVC 目标无法链接；Tauri Windows 构建无法完成。

**最小人工步骤：**

1. 从 <https://visualstudio.microsoft.com/visual-cpp-build-tools/> 下载 Build Tools。
2. 勾选 **“使用 C++ 的桌面开发”**（Desktop development with C++）。
3. 安装后重开终端，确认 `cl.exe` 可从 Developer Command Prompt/PowerShell 找到；或由 Rust 构建自动定位。

### 阻塞 3：pnpm 未启用

**影响：** 无法执行 `pnpm install`、生成 `pnpm-lock.yaml`、运行前端脚本。

**最小人工步骤（需用户确认允许修改 Node 工具目录/缓存）：**

```bash
corepack enable pnpm
corepack pnpm --version
```

或使用用户认可的 pnpm 安装方式。

### 阻塞 4：Windows SDK 不完整

**影响：** 即使安装 MSVC，若 SDK 头文件/库缺失，Windows 目标仍可能链接失败。当前注册表存在 `KitsRoot10`，但 `Include` / `Lib` / `bin` 不存在。

**建议：** 安装 MSVC Build Tools 时勾选 Windows SDK；安装后验证：

```powershell
Test-Path "C:\Program Files\Windows Kits\10\Include"
Test-Path "C:\Program Files\Windows Kits\10\Lib"
```

## 10. 明确未完成 / 未验证

- 未创建 Tauri + React + TypeScript 最小桌面脚手架。
- 未创建 pnpm/Cargo workspace。
- 未生成 `pnpm-lock.yaml` / `Cargo.lock`。
- 未安装任何 npm/Cargo 依赖。
- 未执行 `pnpm install --frozen-lockfile`、`tsc`、ESLint、Prettier、Vitest、`cargo check`、`cargo fmt`、`cargo clippy`、Tauri 启动。
- 未进行 Windows 桌面窗口实机启动验证。
- 未验证 macOS 兼容性（不在本轮范围）。
- Phase 0 验收门禁未满足，因此**不能**标记 Phase 0 完成。

## 11. 下一步（仅当前阶段的最小任务）

1. 用户安装/授权安装 Rust、MSVC C++ Build Tools、pnpm（见第 9 节）。
2. 重开终端，复跑第 3 节工具链检查；全部满足后，重新进入 Phase 0 的脚手架步骤：
   - 初始化 pnpm + Cargo workspace；
   - 创建 `apps/desktop`（Tauri 2 + React + TS + Vite）与 `crates/xushu-core` 最小骨架；
   - 生成并提交 lockfile；
   - 执行 `pnpm install --frozen-lockfile`、`tsc --noEmit`、`cargo check`、`cargo fmt --check`；
   - 更新本报告与 `DEPENDENCIES.md`，把候选版本改为实测锁定版本。
3. 只有在上述验证通过且用户确认后，才进入 Phase 1。本轮不进入 Phase 1。

## 12. 标准汇报摘要

```text
当前阶段 / 任务：Phase 0 — 仓库体检与工程启动
实施摘要：完成目录与工具链只读检查、官方版本核对、README/.gitignore/DEPENDENCIES/ADR/Phase 0 报告初始化；因 Rust/Cargo、MSVC C++ Build Tools、pnpm 缺失，未创建脚手架、未安装依赖、未执行编译/类型检查。
修改文件：README.md、.gitignore、docs/architecture/DEPENDENCIES.md、docs/architecture/ADR/0001-local-first-tauri-stack.md、docs/phases/PHASE_0_REPORT.md；git init -b main（未提交）。
实际执行的命令与退出码：见第 6 节。
测试：PASS — 目录检查、Node/Git/WebView2、文档存在；FAIL（环境）— pnpm/Rust/MSVC/Windows SDK；NOT_RUN — pnpm install、tsc、cargo check、Vitest、Tauri 启动。
安全/隐私检查：未读取/输出密钥；未修改全局配置；未执行管理员安装；未安装 Hooks；未使用子 Agent；未截图。
明确未完成或尚未验证：脚手架、依赖安装、lockfile、编译、类型检查、桌面启动、Phase 0 验收门禁。
下一步：用户安装/授权安装 Rust + MSVC Build Tools + pnpm；复跑工具链检查后继续 Phase 0 脚手架与验证。
```

## 13. 首轮结论（历史）

Phase 0 **未完成，状态 BLOCKED**。本轮没有伪造 PASS，也没有提前进入 Phase 1。仓库文档基础已建立；完成工具链安装后，Phase 0 可在最小范围内继续。

---

# 第二轮执行记录（2026-11-05 晚间）

## 14. 触发条件与执行边界

用户已手动安装并授权使用以下环境：Node.js v24.20.0、pnpm 12.16.1、Rust/Cargo 1.99.1、MSVC Build Tools 2026、Windows SDK。本轮目标是在不进入 Phase 1 的前提下，真正完成 Phase 0 的工程初始化与验证。

边界：

- 只执行 Phase 0；未实现 SQLite、IPC、项目/任务、验收引擎、Hooks 或产品页面；
- 未安装 Claude Code Hooks、未注册系统服务、未修改全局 Claude/DeepSeek/Git/Shell 配置；
- 未使用子 Agent / 多 Agent；未截图或录屏；
- 未执行 `git add`、`git commit`、`git push`；
- 保留首轮已有文件，未覆盖 `CLAUDE.md`、`MASTER_DEVELOPMENT_PLAN.md` 与首轮报告内容。

## 15. 工具链复核与真实编译验证

### 15.1 命令级复核

| 检查 | 命令 | 结果 | 判定 |
|---|---|---|---|
| Node.js | `node --version` | v24.20.0，退出码 0 | PASS |
| pnpm | `pnpm --version` | 12.16.1，退出码 0 | PASS |
| Git | `git --version` | 2.56.0.windows.2，退出码 0 | PASS |
| rustc | `rustc --version` | 1.99.1 (58cf8f9c3 2026-10-13)，退出码 0 | PASS |
| cargo | `cargo --version` | 1.99.1 (5f94df478 2026-08-27)，退出码 0 | PASS |
| rustup | `rustup show` | stable-x86_64-pc-windows-msvc，active/default | PASS |
| rustfmt | `rustfmt --version` | 1.10.0-stable | PASS |
| Clippy | `cargo clippy --version` | 0.1.99 | PASS |
| MSVC 安装 | `vswhere -all -products * -format json` | VS Build Tools 2026 18.11.12328.437，`isComplete=true`、`isLaunchable=true` | PASS |
| Windows SDK | 文件存在性检查 | `windows.h`、`user32.lib`、`rc.exe`（10.0.26100.0）均存在 | PASS |
| WebView2 | 注册表 `pv` | 156.0.4314.58 | PASS |

### 15.2 编译器 / 链接器真实测试

本轮没有只检查安装路径，而是在 MSVC 开发者环境中实际编译、链接并运行了最小程序：

- C 程序：`cl /nologo /W3 envtest.c /Fe:envtest.exe`，`CL_EXIT=0`；运行 `envtest.exe` 输出 `msvc-ok`，`RUN_EXIT=0`。
- Rust 程序：`rustc -O rs.rs -o rs.exe`，`rustc_exit=0`；运行输出 `rust-ok`，`run_exit=0`。这同时验证了 Rust 对 MSVC 链接器的探测。
- `cl.exe` / `link.exe` 实际位于 `D:\Microsoft Visual Studio\18\BuildTools\VC\Tools\MSVC\14.51.36231\bin\Hostx64\x64\`。
- 备注：`vcvars64.bat` 输出一条 `vswhere.exe` 不在 PATH 的提示，但 `cl.exe`/`link.exe` 随后正常定位，编译链接成功；`pnpm tauri info` 也能正确识别 MSVC。

### 15.3 官方资料核对（2026-11-05）

- Tauri 官方 Windows 先决条件：Microsoft C++ Build Tools（“使用 C++ 的桌面开发”）、WebView2、Rust MSVC 工具链、Node.js（JavaScript 前端）。
- Tauri 2.12 发布说明（2026-09-26）：MSRV 提升到 **Rust 1.90**；不再官方支持 Windows 7。本项目目标 Windows 11，不受影响。
- crates.io：`tauri` 稳定版 **2.12.5**；`tauri-build` 稳定版 **2.7.4**。
- npm registry：`@tauri-apps/cli` **2.12.4**、`vite` **8.3.6**、`react`/`react-dom` **19.3.1**、`typescript` **6.0.3**、`typescript-eslint` **8.72.1**、`eslint` **10.14.0**、`tailwindcss` **4.3.4**、`vitest` **5.1.0**。
- TypeScript 当前 latest 为 7.0.2，但 `typescript-eslint@8.72.1` 的 peer 范围为 `>=4.8.4 <6.1.0`，因此选择 6.0.3；未关闭类型检查或放宽 Lint 规则。

详细版本矩阵与兼容性依据见 `docs/architecture/DEPENDENCIES.md`。

## 16. 本轮工程变更

### 16.1 新增文件

| 路径 | 说明 |
|---|---|
| `package.json` | pnpm 根工具链、统一脚本、`packageManager: pnpm@12.16.1` |
| `pnpm-workspace.yaml` | workspace 成员 `apps/*`；含 pnpm 12 自动写入的供应链策略豁免记录 |
| `.npmrc` | 固定官方 npm registry，`save-exact=true` |
| `Cargo.toml` | Cargo workspace，成员为 Tauri 应用与 `xushu-core` |
| `rust-toolchain.toml` | stable MSVC、rustfmt、clippy、Windows target |
| `.prettierrc.json` / `.prettierignore` | 前端与配置格式规范；避免改写既有中文文档 |
| `pnpm-lock.yaml` | 实际解析的 238 个前端包 |
| `Cargo.lock` | 实际解析的 Rust 依赖 |
| `apps/desktop/package.json` | React/Vite/Tailwind/Tauri CLI/ESLint/Vitest 精确版本 |
| `apps/desktop/index.html` | `lang="zh-CN"` 的 Vite 入口 |
| `apps/desktop/tsconfig.json` | strict、noEmit、bundler、react-jsx |
| `apps/desktop/vite.config.ts` | React + Tailwind 插件、Tauri dev server 端口 1420 |
| `apps/desktop/vitest.config.ts` | jsdom + Setup 文件 |
| `apps/desktop/eslint.config.js` | ESLint 10 flat config + TS/React Hooks 规则 |
| `apps/desktop/src/main.tsx` | React 根挂载，含缺失 `#root` 的显式错误 |
| `apps/desktop/src/App.tsx` | Phase 0 占位界面，明确声明不代表产品功能 |
| `apps/desktop/src/index.css` | Tailwind 4 入口与基础字体 |
| `apps/desktop/src/App.test.tsx` | RTL 冒烟测试 |
| `apps/desktop/src/test/setup.ts` | jest-dom 与 `cleanup` |
| `apps/desktop/src/vite-env.d.ts` | Vite 类型引用 |
| `apps/desktop/src-tauri/Cargo.toml` | `tauri 2.12.5`、`tauri-build 2.7.4` |
| `apps/desktop/src-tauri/build.rs` | Tauri 构建脚本 |
| `apps/desktop/src-tauri/src/main.rs` | Windows 桌面入口 |
| `apps/desktop/src-tauri/src/lib.rs` | Tauri Builder 启动，无业务命令 |
| `apps/desktop/src-tauri/tauri.conf.json` | 产品名、窗口、devUrl、frontendDist、bundle |
| `apps/desktop/src-tauri/capabilities/default.json` | 仅 `core:default` |
| `apps/desktop/src-tauri/app-icon.svg` | 图标源文件 |
| `apps/desktop/src-tauri/icons/*` | `tauri icon` 生成的桌面图标（已移除未使用的 android/ios 目录） |
| `crates/xushu-core/Cargo.toml` | 核心库 workspace crate |
| `crates/xushu-core/src/lib.rs` | Phase 0 常量与一个单元测试 |

### 16.2 更新文件

| 路径 | 说明 |
|---|---|
| `README.md` | 状态改为 Phase 0 PASS；补充实际环境、命令与结构 |
| `docs/architecture/DEPENDENCIES.md` | 从“候选版本”改为实测锁定版本与验证结果 |
| `docs/phases/PHASE_0_REPORT.md` | 保留首轮 BLOCKED 历史，追加本轮结果 |
| `docs/architecture/ADR/0001-local-first-tauri-stack.md` | 追加环境状态修订说明，不改原决策 |

### 16.3 未修改 / 未提交

- `CLAUDE.md`、`MASTER_DEVELOPMENT_PLAN.md` 未修改。
- 首轮 `.gitignore` 保持原内容。
- 未执行任何 Git 写操作；仓库仍全部为未跟踪文件。两个 lockfile 已生成在项目目录，但不是 Git 提交。

## 17. 实际执行的命令与退出码

| 命令 | 退出码 | 关键结果 |
|---|---|---|
| `pnpm install` | 0 | 2 个项目；新增 238 个包；2m36s |
| `pnpm tauri icon src-tauri/app-icon.svg` | 0 | 生成桌面图标；警告一条系统字体 malformed，不影响产物 |
| `pnpm typecheck` | 0 | `tsc --noEmit` 通过 |
| `pnpm lint` | 0 | ESLint 通过，`--max-warnings 0` |
| `pnpm test` | 0 | 1 个测试文件、1 个测试通过 |
| `pnpm build` | 0 | Vite 8 生产构建通过；JS 220.85 kB / gzip 69.27 kB |
| `pnpm format`（首次） | 1 | 16 个新文件格式不一致 |
| `pnpm format:write` | 0 | 仅格式化本轮新增文件 |
| `pnpm format`（复跑） | 0 | 全部匹配 |
| `pnpm install --frozen-lockfile` | 0 | 锁文件与 workspace 一致，64ms |
| `cargo check --workspace --all-targets` | 0 | 首次 2m00s；Tauri 2.12.5 编译通过 |
| `cargo check --workspace --all-targets --locked` | 0 | 1.34s；锁文件复现通过 |
| `cargo fmt --all -- --check`（首次） | 1 | 4 个 Rust 文件缺文件末尾换行 |
| `cargo fmt --all` | 0 | 仅格式化本轮新增 Rust 文件 |
| `cargo fmt --all -- --check`（复跑） | 0 | 格式通过 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | 3.64s，无警告 |
| `cargo test --workspace` | 0 | xushu-core 1 个测试通过；其他 target 0 个测试 |
| `cargo test -p xushu-core` | 0 | `app_name_is_not_empty ... ok`，1 passed |
| `pnpm tauri info` | 0 | 正确识别 MSVC、WebView2、Rust、Node、pnpm 与依赖版本 |
| `pnpm tauri dev`（后台） | 启动成功 | Vite ready；cargo 编译完成；`xushu-desktop.exe` 运行 |
| `curl http://localhost:1420/` | 0 | HTTP 200，返回 `lang="zh-CN"` 的 HTML |
| `taskkill /F /T /PID 26284`（清理） | 0 | 结束本次启动的 Tauri/Vite/cargo/WebView2 进程树；端口 1420 释放 |

说明：`pnpm format` 与 `cargo fmt --check` 的首次失败均发生在本轮新增文件的格式/换行上，已通过格式化修复并复跑通过；不是产品逻辑或依赖兼容失败。

## 18. 桌面启动验证（未截图）

启动命令：`pnpm tauri dev`（后台运行，日志写入临时目录）。

关键日志：

```text
VITE v8.3.6  ready in 577 ms
Local: http://localhost:1420/
Compiling xushu-desktop v0.1.0 (D:\MyProject\xushu\apps\desktop\src-tauri)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.39s
Running `D:\MyProject\xushu\target\debug\xushu-desktop.exe`
```

进程核验（PowerShell）：

```json
{
  "Id": 31252,
  "ProcessName": "xushu-desktop",
  "MainWindowTitle": "序枢 XUSHU",
  "MainWindowHandle": 18418040
}
```

WebView2 子进程：

```json
{
  "ProcessId": 25108,
  "Name": "msedgewebview2.exe",
  "CommandLine": "... --webview-exe-name=xushu-desktop.exe --webview-exe-version=0.1.0 --user-data-dir=C:\Users\kmld\AppData\Local\dev.xushu.desktop\EBWebView ..."
}
```

同时 `curl --noproxy '*' http://localhost:1420/` 返回 HTTP 200。

**关于后台任务被终止的说明：** 启动验证完成后，Claude Code 因系统内存不足终止了后台 `pnpm tauri dev` 任务；这发生在窗口与 WebView2 已验证存在之后，不是应用启动失败。随后本轮主动清理了残留的 Tauri/Vite/cargo/WebView2 进程树，并确认端口 1420 已不可连接。未进行任何截图或录屏。

## 19. 安全 / 隐私 / 权限检查

- 未读取或输出 API Key、`.env`、SSH 私钥、token、credential store。
- 未修改全局 Claude Code / DeepSeek 配置、模型、endpoint、API Key、Shell 环境变量、账单设置。
- 未执行管理员安装；Rust、MSVC、pnpm 由用户提前安装。
- 未安装 Claude Code Hooks、未注册系统服务、未修改其他项目。
- 未使用子 Agent / 多 Agent；未截图/录屏。
- Tauri capabilities 仅授予 `core:default`；没有自定义 IPC 命令、没有文件系统/Shell/SQL 权限。
- 依赖下载使用项目级 `.npmrc` 固定的官方 npm registry；未修改用户全局 registry。
- 未创建数据库、未执行 SQL、未写入敏感数据。
- 启动的桌面进程与 WebView2 数据目录仅用于本轮验证，验证后已终止进程；未清理用户级 WebView2 数据目录。

## 20. 明确未完成 / 未验证 / 限制

1. **未执行 `pnpm tauri build`**：没有生成 Windows 安装包或 release 二进制；bundle/安装器验证属于 Phase 7。
2. **未进行 macOS 验证**：当前仅 Windows 11 x64 实机验证；macOS 真机不在本轮范围。
3. **未实现任何产品功能**：无 SQLite、迁移、IPC 命令、项目/任务状态机、验收引擎、Hooks、路由、主题切换、错误边界、shadcn/ui 或 Lucide。
4. **CSP 仍为 `null`**：这是 Tauri 默认脚手架状态，不能视为最终安全配置；需在后续阶段收紧并测试。
5. **测试覆盖有限**：前端仅 1 个冒烟测试，Rust 仅 1 个常量测试；Phase 0 只验证工具链与脚手架，不代表业务正确性。
6. **未做 Git 提交**：按项目规则等待用户授权；所有文件仍是未跟踪状态。
7. **桌面启动由后台任务执行**：启动证据完整，但未进行人工视觉检查；按要求未自动截图。

## 21. Phase 0 最终验收

| 验收门禁 | 结果 | 证据 |
|---|---|---|
| 架构决策文件存在 | PASS | `docs/architecture/ADR/0001-local-first-tauri-stack.md` |
| 仓库结构正确 | PASS | pnpm + Cargo workspace；`apps/desktop`、`crates/xushu-core` 可达 |
| 依赖安装可复现 | PASS | `pnpm install --frozen-lockfile` 退出码 0；`cargo check --locked` 退出码 0 |
| `pnpm-lock.yaml` / `Cargo.lock` 存在 | PASS | 92 KB / 104 KB，实际解析版本已记录 |
| 前端类型检查 | PASS | `pnpm typecheck` 退出码 0 |
| 前端构建 | PASS | `pnpm build` 退出码 0 |
| Rust 编译检查 | PASS | `cargo check --workspace --all-targets --locked` 退出码 0 |
| Rust 格式 / Clippy / 测试 | PASS | 三条命令均退出码 0 |
| Lint / Prettier / Vitest | PASS | 三条命令均退出码 0 |
| Tauri 桌面实际启动 | PASS | 窗口标题「序枢 XUSHU」、主窗口句柄非零、WebView2 子进程存在 |
| 未伪造 PASS / 未跳过检查 | PASS | 首次格式失败已记录并按规则修复复跑 |
| 未提前进入 Phase 1 | PASS | 无业务命令、数据库、Hooks 或产品页面 |

**Phase 0 最终状态：PASS。** 该结论仅覆盖工程初始化与工具链验证，不代表 V1 产品功能完成。

## 22. 下一步（仅建议，尚未执行）

Phase 1 建议从最小闭环开始，保持本阶段门禁全绿：

1. 在 `crates/xushu-core` 引入 `rusqlite` 与版本化迁移骨架，建立 `schema_migrations` 表；用临时数据库做迁移成功、失败回滚与重复执行测试。
2. 增加最小 Tauri IPC：由 Rust 解析应用数据目录、初始化数据库连接，提供只读健康检查命令；前端只调用显式允许的命令。
3. 建立 UI 外壳：React Router、主题令牌、错误边界、加载/错误/空状态；暂不实现虚假的业务 KPI。
4. 实现“写入一条测试记录 → 退出 → 重启 → 读回”的持久化验收，并记录真实命令与退出码。

在上述内容完成并通过验收前，不进入 Phase 2。

## 23. 标准汇报摘要

```text
当前阶段 / 任务：Phase 0 — 仓库体检与工程启动（第二轮，环境补齐后）
实施摘要：搭建 pnpm + Cargo workspace；创建 Tauri 2.12.5 + React 19 + TS 6 + Vite 8 + Tailwind 4 最小桌面工程；生成 lockfile；配置 ESLint/Prettier/Vitest/rustfmt/Clippy；实际编译、测试并启动桌面程序。
修改文件：见第 16 节；新增 apps/desktop、crates/xushu-core、根配置、pnpm-lock.yaml、Cargo.lock；更新 README、DEPENDENCIES、PHASE_0_REPORT、ADR-0001 修订说明。
实际执行的命令与退出码：见第 17 节。
测试：PASS — 环境、MSVC 编译链接、rustc 链接、冻结安装、typecheck、ESLint、Prettier、Vitest、Vite build、cargo check --locked、cargo fmt、cargo clippy、cargo test、Tauri 启动；NOT_RUN — 安装包构建（tauri build）、macOS、Phase 1 功能。
安全/隐私检查：未读取/输出密钥；未修改全局配置；未执行管理员安装；未安装 Hooks；未使用子 Agent；未截图。
明确未完成或尚未验证：SQLite/IPC/业务功能、安装包、macOS、CSP 收紧、完整测试覆盖。
下一步：Phase 1 从 xushu-core 的 SQLite 迁移与持久化最小闭环开始；等待用户确认。
```