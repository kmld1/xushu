# 依赖与工具链基线

> 状态：**Phase 0 PASS**（2026-11-05 第二轮，环境补齐后实测）。
>
> 本文件只记录实际安装、实际锁定并通过检查的版本。Phase 0 采用最小依赖；未安装的既有技术栈组件在文末明确列出，避免把“计划使用”写成“已安装”。

## 1. 本机工具链实测

| 组件 | 实测结果 | 判定 | 证据 |
|---|---|---|---|
| Windows | 11 Enterprise 10.0.26200 x64 | PASS | 系统环境；`cmd /c ver` 退出码 0 |
| Node.js | v24.20.0 | PASS | `node --version` 退出码 0 |
| npm | 11.19.0 | PASS（辅助） | `npm --version` 退出码 0 |
| Corepack | 0.35.0 | PASS | `corepack --version` 退出码 0 |
| pnpm | 12.16.1 | PASS | `pnpm --version` 退出码 0；`D:\nodejs\pnpm.CMD` |
| Git | 2.56.0.windows.2 | PASS | `git --version` 退出码 0 |
| rustc | 1.99.1 (58cf8f9c3 2026-10-13) | PASS | `rustc --version` 退出码 0 |
| cargo | 1.99.1 (5f94df478 2026-08-27) | PASS | `cargo --version` 退出码 0 |
| rustup | 1.29.3 (edbc831ce 2026-11-02) | PASS | `rustup show`、`pnpm tauri info` 退出码 0 |
| Rust toolchain | stable-x86_64-pc-windows-msvc | PASS | 已安装且为默认；`rustup show` |
| rustfmt | 1.10.0-stable | PASS | `rustfmt --version` 退出码 0 |
| Clippy | 0.1.99 | PASS | `cargo clippy --version` 退出码 0 |
| MSVC Build Tools | Visual Studio 生成工具 2026，18.11.12328.437；VC Tools 14.51.36231 | PASS | `vswhere -all -products *`；`cl.exe`、`link.exe` 实际编译链接成功 |
| Windows SDK | 10.0.26100.0 | PASS | `Include\10.0.26100.0\um\windows.h`、`Lib\10.0.26100.0\um\x64\user32.lib`、`bin\10.0.26100.0\x64\rc.exe` 均存在 |
| WebView2 Runtime | 156.0.4314.58 | PASS | 注册表 `HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}` 的 `pv`；Tauri 启动时实际创建 `msedgewebview2.exe` 子进程 |

### 1.1 MSVC 编译器 / 链接器真实验证

仅检查安装路径不足以证明可用，本轮执行了真实编译、链接和运行：

- `vswhere.exe -all -products * -format json`：退出码 0；`installationPath = D:\Microsoft Visual Studio\18\BuildTools`，`isComplete = true`，`isLaunchable = true`。
- 实际路径：`D:\Microsoft Visual Studio\18\BuildTools\VC\Tools\MSVC\14.51.36231\bin\Hostx64\x64\cl.exe` 与 `link.exe`。
- C 测试程序：`cl /nologo /W3 envtest.c /Fe:envtest.exe`，`CL_EXIT=0`；运行 `envtest.exe` 输出 `msvc-ok`，`RUN_EXIT=0`。
- Rust 测试程序：`rustc -O rs.rs -o rs.exe`，`rustc_exit=0`；运行输出 `rust-ok`，`run_exit=0`。该步骤同时验证了 Rust 对 MSVC 链接器的自动探测。
- 备注：直接调用 `vcvars64.bat` 时会输出一条 `vswhere.exe` 不在 PATH 的提示，但随后 `cl.exe`/`link.exe` 均能定位，实际编译链接成功；`pnpm tauri info` 也正确识别 MSVC。

## 2. 实际锁定的前端依赖

锁文件：`pnpm-lock.yaml`（`lockfileVersion: '9.0'`，92 KB），由 `pnpm install` 生成；随后 `pnpm install --frozen-lockfile` 退出码 0，确认可复现。

| 类别 | 包 | 锁定版本 |
|---|---|---|
| 运行时 | `react` | 19.3.1 |
| 运行时 | `react-dom` | 19.3.1 |
| 构建 | `vite` | 8.3.6 |
| 构建 | `@vitejs/plugin-react` | 6.2.1 |
| 构建 | `typescript` | 6.0.3 |
| 样式 | `tailwindcss` | 4.3.4 |
| 样式 | `@tailwindcss/vite` | 4.3.4 |
| Tauri CLI | `@tauri-apps/cli` | 2.12.4 |
| 静态检查 | `eslint` | 10.14.0 |
| 静态检查 | `@eslint/js` | 10.0.1 |
| 静态检查 | `typescript-eslint` | 8.72.1 |
| 静态检查 | `eslint-plugin-react-hooks` | 7.1.1 |
| 静态检查 | `eslint-plugin-react-refresh` | 0.5.7 |
| 静态检查 | `eslint-config-prettier` | 10.1.8 |
| 静态检查 | `globals` | 17.14.0 |
| 格式 | `prettier` | 3.9.10 |
| 测试 | `vitest` | 5.1.0 |
| 测试 | `jsdom` | 31.0.0 |
| 测试 | `@testing-library/react` | 16.3.3 |
| 测试 | `@testing-library/dom` | 10.4.2 |
| 测试 | `@testing-library/jest-dom` | 7.0.1 |
| 类型 | `@types/node` | 24.19.2 |
| 类型 | `@types/react` | 19.3.0 |
| 类型 | `@types/react-dom` | 19.3.0 |

说明：

- 根 `package.json` 写入 `packageManager: pnpm@12.16.1`，并使用精确版本（无 `^`），减少重新解析时的漂移。
- 项目级 `.npmrc` 固定官方 registry：`https://registry.npmjs.org/`，并设置 `save-exact=true`。
- pnpm 12 在首次安装时向 `pnpm-workspace.yaml` 自动写入 `minimumReleaseAgeExclude`（`jsdom@31.0.0`、`vite@8.3.6`），这是 pnpm 供应链策略对本次选定新版本的显式豁免记录；保留该状态，后续升级需重新评估。

## 3. 实际锁定的 Rust 依赖

锁文件：`Cargo.lock`（104 KB），由首次 `cargo check` 生成；`cargo check --workspace --all-targets --locked` 退出码 0。

| 包 | 锁定版本 | 说明 |
|---|---|---|
| `tauri` | 2.12.5 | Tauri 2.x 稳定版 |
| `tauri-build` | 2.7.4 | Tauri 构建期依赖的独立版本线 |
| `wry` | 0.57.1 | Tauri 使用的 WebView 绑定 |
| `tao` | 0.37.2 | Tauri 使用的窗口库 |
| `xushu-core` | 0.1.0 | 本地 workspace crate |
| `xushu-desktop` | 0.1.0 | 本地 Tauri 应用 crate |

- Rust edition：2021（workspace 统一）；`rust-version = "1.90"`（Tauri 2.12 MSRV）。
- `rust-toolchain.toml` 使用 `channel = "stable"`，并要求 `rustfmt`、`clippy` 与 `x86_64-pc-windows-msvc` target；本机 stable 实测为 1.99.1。
- `pnpm tauri info` 确认：`tauri 2.12.5`、`tauri-build 2.7.4`、`wry 0.57.1`、`tao 0.37.2`、`@tauri-apps/cli 2.12.4`，MSVC/WebView2/Rust/Node/pnpm 均被识别。

## 4. 关键兼容性结论

1. **Tauri 2.12 要求 Rust 1.90+**；本机 1.99.1 满足。Tauri 2.12 已移除官方 Windows 7 支持，本项目目标为 Windows 11 x64，不受影响。
2. **Vite 8.3.6** 的 engines 为 `^20.19.0 || >=22.12.0`；Node 24.20.0 满足。
3. **TypeScript 7.0.2 是当前 npm latest，但本轮不采用**：`typescript-eslint@8.72.1` 的 peer 范围为 `typescript >=4.8.4 <6.1.0`，TS 7.0 不提供其依赖的编译器 API。选用 **TypeScript 6.0.3**，并通过实际 `tsc --noEmit` 与 ESLint 验证。
4. **React 19.3.1 + `@vitejs/plugin-react@6.2.1`** 可正常构建；React 19 类型包为 19.3.0。
5. **Tailwind CSS 4.3.4** 通过 `@tailwindcss/vite` 接入；其 peer 支持 Vite `^5.2 || ^6 || ^7 || ^8`，与 Vite 8.3.6 兼容。
6. **Vitest 5.1.0** 支持 Vite `^6.4 || ^7 || ^8`、Node `^24`；**jsdom 31.0.0** 要求 Node `^22.22.2 || ^24.15.0 || >=26`；本机 Node 24.20.0 满足两者。
7. **ESLint 10.14.0 + typescript-eslint 8.72.1 + eslint-plugin-react-hooks 7.1.1 + eslint-plugin-react-refresh 0.5.7** 的 peer 范围互相兼容；`pnpm lint` 实际通过。
8. **Tauri CLI 2.12.4 与 tauri crate 2.12.5** 已在 Windows 实际编译、启动并创建 WebView2 窗口，版本组合可用。

## 5. 锁文件与复现策略

- 前端：提交 `pnpm-lock.yaml`；验收命令 `pnpm install --frozen-lockfile`（本轮退出码 0）。
- Rust：提交 `Cargo.lock`；验收命令 `cargo check --workspace --all-targets --locked`（本轮退出码 0）。
- 不在安装命令中使用 `latest` 或无锁定的补丁升级；升级依赖时重新生成两个锁文件，并复跑第 6 节全部门禁。
- `.gitignore` 已排除 `node_modules/`、`dist/`、`target/`、SQLite 运行时文件、日志、密钥与安装包；两个锁文件不被忽略。
- `src-tauri/gen/schemas/` 为 Tauri 构建期生成物，已被 `.gitignore` 排除。

## 6. 本轮实际通过的质量门禁

| 命令 | 退出码 | 结果 |
|---|---|---|
| `pnpm install --frozen-lockfile` | 0 | 锁文件与 workspace 一致，秒级完成 |
| `pnpm typecheck` | 0 | `tsc --noEmit` 通过 |
| `pnpm lint` | 0 | ESLint 10 flat config，`--max-warnings 0` |
| `pnpm test` | 0 | Vitest 5：1 个测试文件、1 个测试通过 |
| `pnpm build` | 0 | Vite 生产构建通过 |
| `pnpm format` | 0 | Prettier 检查通过 |
| `cargo check --workspace --all-targets --locked` | 0 | Rust workspace 通过 |
| `cargo fmt --all -- --check` | 0 | rustfmt 通过 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | Clippy 通过 |
| `cargo test --workspace` | 0 | xushu-core 1 个单元测试通过；其余 target 0 个测试 |
| `pnpm tauri info` | 0 | 环境与依赖版本被正确识别 |
| `pnpm tauri dev` | 启动成功 | Vite ready、cargo 运行、窗口进程与 WebView2 子进程均存在 |

## 7. 已规划但本阶段未安装

以下组件属于冻结技术栈，但 Phase 0 没有使用场景，因此未安装、未锁定，不能视为已完成：

- `@tauri-apps/api`（Phase 1 引入 IPC 时安装）
- React Router、TanStack Query、Zustand、React Hook Form、Zod
- shadcn/ui、Lucide
- `rusqlite`、SQLite 迁移与备份相关依赖
- `xushu-hook`（Phase 4 创建）

## 8. 官方参考（核对日期：2026-11-05）

- Tauri prerequisites：<https://v2.tauri.app/start/prerequisites/>（Windows 需要 Microsoft C++ Build Tools 的“使用 C++ 的桌面开发”、WebView2 与带 MSVC 工具链的 Rust）
- Tauri 2.12 发布说明：<https://v2.tauri.app/blog/tauri-2.12/>（2026-09-26；MSRV 提升到 1.90；移除 Windows 7 支持）
- crates.io API：<https://crates.io/api/v1/crates/tauri>（`max_stable_version = 2.12.5`）
- crates.io API：<https://crates.io/api/v1/crates/tauri-build>（`max_stable_version = 2.7.4`）
- npm registry：<https://registry.npmjs.org/>（各包 `version` / `engines` / `peerDependencies` 实际查询）
- Rust stable channel：<https://static.rust-lang.org/dist/channel-rust-stable.toml>
- Vite 指南：<https://vite.dev/guide/>
- TypeScript 7 / typescript-eslint 兼容性议题：<https://github.com/typescript-eslint/typescript-eslint/issues/12518>