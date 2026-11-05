# 依赖与工具链基线

> 状态：**Phase 1 PASS**（2026-11-06）。
>
> 本文件只记录实际安装、实际锁定并通过检查的版本。依赖升级必须重新生成
> `pnpm-lock.yaml` / `Cargo.lock`，并复跑文末质量门禁。

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
| rustup | 1.29.3 (edbc831ce 2026-11-02) | PASS | `rustup show`、`pnpm tauri info` |
| Rust toolchain | stable-x86_64-pc-windows-msvc | PASS | 由 `rust-toolchain.toml` 覆盖为 stable |
| rustfmt | 1.10.0-stable | PASS | `cargo fmt --check` 退出码 0 |
| Clippy | 0.1.99 | PASS | `cargo clippy ... -D warnings` 退出码 0 |
| MSVC Build Tools | Visual Studio 生成工具 2026，18.11.12328.437；VC Tools 14.51.36231 | PASS | `cl.exe` 编译、`link.exe` 链接、exe 运行成功 |
| Windows SDK | 10.0.26100.0 | PASS | `windows.h`、`user32.lib`、`rc.exe` 存在 |
| WebView2 Runtime | 156.0.4314.58 | PASS | Tauri 启动时创建 `msedgewebview2.exe` 子进程 |
| Python（仅测试辅助） | 3.14.7 | PASS（辅助） | 独立只读 `sqlite3` 核验数据库；不是应用依赖 |

## 2. 前端直接依赖

锁文件：`pnpm-lock.yaml`（104 KB）；`pnpm install --frozen-lockfile` 退出码 0。

### 2.1 运行时

| 包 | 锁定版本 | 用途 |
|---|---|---|
| `react` / `react-dom` | 19.3.1 | UI 运行时 |
| `@tauri-apps/api` | 2.12.4 | Tauri IPC 前端调用 |
| `react-router` | 8.5.1 | HashRouter 路由 |
| `@tanstack/react-query` | 5.106.1 | IPC 异步状态与缓存 |
| `zustand` | 5.0.16 | 主题等纯 UI 状态 |
| `react-hook-form` | 7.91.0 | 表单状态 |
| `zod` | 4.6.7 | 前端输入校验（Rust 仍二次校验） |
| `@hookform/resolvers` | 7.0.3 | RHF + Zod 适配 |
| `lucide-react` | 1.69.0 | 图标 |
| `class-variance-authority` | 0.7.1 | shadcn/ui 变体 |
| `clsx` | 2.1.1 | 类名组合 |
| `tailwind-merge` | 3.7.0 | Tailwind 类名合并 |
| `@radix-ui/react-slot` | 1.5.0 | shadcn/ui Button asChild |
| `@radix-ui/react-label` | 2.1.17 | shadcn/ui Label |

### 2.2 构建、检查与测试

| 包 | 锁定版本 |
|---|---|
| `vite` | 8.3.6 |
| `@vitejs/plugin-react` | 6.2.1 |
| `typescript` | 6.0.3 |
| `tailwindcss` / `@tailwindcss/vite` | 4.3.4 |
| `@tauri-apps/cli` | 2.12.4 |
| `eslint` | 10.14.0 |
| `@eslint/js` | 10.0.1 |
| `typescript-eslint` | 8.72.1 |
| `eslint-plugin-react-hooks` | 7.1.1 |
| `eslint-plugin-react-refresh` | 0.5.7 |
| `eslint-config-prettier` | 10.1.8 |
| `globals` | 17.14.0 |
| `prettier` | 3.9.10 |
| `vitest` | 5.1.0 |
| `jsdom` | 31.0.0 |
| `@testing-library/react` | 16.3.3 |
| `@testing-library/dom` | 10.4.2 |
| `@testing-library/jest-dom` | 7.0.1 |
| `@testing-library/user-event` | 14.6.8 |
| `@types/node` | 24.19.2 |
| `@types/react` / `@types/react-dom` | 19.3.0 |

## 3. Rust 依赖

锁文件：`Cargo.lock`（108 KB）；`cargo check --workspace --all-targets --locked` 退出码 0。

| 包 | 锁定版本 | 说明 |
|---|---|---|
| `tauri` | 2.12.5 | 桌面壳与 IPC |
| `tauri-build` | 2.7.4 | 构建期依赖（独立版本线） |
| `wry` | 0.57.1 | WebView 绑定 |
| `tao` | 0.37.2 | 窗口库 |
| `serde` | 1.0.229 | DTO/命令序列化 |
| `rusqlite` | 0.41.1 | SQLite 访问；启用 `bundled` |
| `libsqlite3-sys` | 0.39.1 | 随 rusqlite 锁定的 SQLite FFI |
| `uuid` | 1.29.0 | v4 记录 ID |
| `chrono` | 0.4.45 | UTC RFC3339 时间 |
| `thiserror` | 2.0.21 | 核心错误类型（传递依赖另有 1.0.69） |
| `tempfile` | 3.27.0 | 仅 dev/test 临时数据库 |

- Rust edition：2021；workspace `rust-version = "1.90"`。
- `rust-toolchain.toml`：`channel = "stable"`、`rustfmt`、`clippy`、
  `x86_64-pc-windows-msvc`。

## 4. 关键兼容性结论

1. **Tauri 2.12 MSRV 1.90**；Rust 1.99.1 满足。Tauri 2.12 移除 Windows 7 支持，本项目目标 Windows 11。
2. **Vite 8.3.6** 要求 Node `^20.19.0 || >=22.12.0`；Node 24.20.0 满足。
3. **TypeScript 6.0.3**：typescript-eslint 8.72.1 的 peer 为 `>=4.8.4 <6.1.0`，因此不使用 TS 7.0.2。
4. **React Router 8.5.1** 要求 Node `>=22.22.0`、React `>=19.2.7`；当前 Node 24.20.0、React 19.3.1 满足。
5. **TanStack Query 5.106.1** peer 支持 React 18/19；**Zustand 5.0.16** peer 支持 React >=18。
6. **React Hook Form 7.91.0 + Zod 4.6.7 + @hookform/resolvers 7.0.3**：resolvers 的 zod peer 为 `^3.25.0 || ^4.0.0`，组合已验证通过类型检查与测试。
7. **Tailwind CSS 4.3.4** 通过 `@tailwindcss/vite` 接入；peer 支持 Vite 8。
8. **Vitest 5.1.0 / jsdom 31.0.0** 与 Node 24.20.0、Vite 8.3.6 兼容；13 个前端测试通过。
9. **rusqlite 0.41.1 bundled** 在 MSVC 14.51 + Windows SDK 10.0.26100 下实际编译、链接并运行；WAL、外键与迁移测试通过。
10. `@tauri-apps/cli 2.12.4` 与 `tauri 2.12.5` 已在本机实际编译并启动。

## 5. 锁文件与复现策略

- 根 `package.json` 固定 `packageManager: pnpm@12.16.1`；项目级 `.npmrc` 固定官方 registry
  `https://registry.npmjs.org/` 与 `save-exact=true`。
- 前端验收：`pnpm install --frozen-lockfile`（2026-11-06 退出码 0）。
- Rust 验收：`cargo check --workspace --all-targets --locked`（退出码 0）。
- pnpm 12 的供应链策略在 `pnpm-workspace.yaml` 中记录了 `minimumReleaseAgeExclude`：
  `jsdom@31.0.0`、`vite@8.3.6`、`@hookform/resolvers@7.0.3`、
  `@tanstack/query-core@5.106.1`、`@tanstack/react-query@5.106.1`、`lucide-react@1.69.0`。
  这是对本次选定新版本进入安装的显式豁免记录；升级时需重新评估。
- 构建产物 `node_modules/`、`dist/`、`target/`、`src-tauri/gen/`、SQLite 运行时文件、
  日志与密钥均被 `.gitignore` 排除；两个 lockfile 不被忽略。

## 6. 数据库与安全配置基线

- 数据库：Tauri `app_data_dir()` 下的 `xushu.db`；Windows 实测
  `%APPDATA%\dev.xushu.desktop\xushu.db`，不进入仓库。
- PRAGMA：`foreign_keys=ON`、`journal_mode=WAL`、`synchronous=NORMAL`、`busy_timeout=5000`。
- Schema：`schema_migrations` + `persistence_probes`；只存在 Phase 1 探针表。
- Capabilities：仅 `core:default`；无 Shell / FS / SQL 插件权限。
- CSP：
  - 生产：`default-src 'self'; connect-src ipc: http://ipc.localhost; ...; script-src 'self'; object-src 'none'; base-uri 'self'`
  - 开发：额外允许 `ws://localhost:1420/1421` 与 Vite 内联 refresh 脚本。
  - 生产 CSP 尚未用 `tauri build` 成品运行验证，属于遗留验证项。

## 7. 本轮实际通过的质量门禁（2026-11-06）

| 命令 | 退出码 | 结果 |
|---|---|---|
| `pnpm install --frozen-lockfile` | 0 | 锁文件一致 |
| `pnpm typecheck` | 0 | `tsc --noEmit` 通过 |
| `pnpm lint` | 0 | ESLint 10，`--max-warnings 0` |
| `pnpm test` | 0 | 4 个测试文件、13 个测试通过 |
| `pnpm build` | 0 | Vite 生产构建通过 |
| `pnpm format` | 0 | Prettier 检查通过 |
| `cargo check --workspace --all-targets --locked` | 0 | Rust workspace 通过 |
| `cargo fmt --all -- --check` | 0 | rustfmt 通过 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | Clippy 通过 |
| `cargo test --workspace` | 0 | 1 个 lib 测试 + 12 个持久化集成测试通过 |
| `pnpm tauri info` | 0 | 环境与依赖版本被正确识别 |

## 8. 已规划但本阶段未安装

- `xushu-hook` CLI（Phase 4）
- Claude Code Hooks 相关依赖（Phase 4）
- 项目/任务/验收业务表与对应 Rust 服务（Phase 2/3）
- 备份/恢复、日志脱敏与安全加固依赖（Phase 6）
- 安装包、签名与分发工具（Phase 7）

## 9. 官方参考（核对日期：2026-11-06）

- Tauri prerequisites：<https://v2.tauri.app/start/prerequisites/>
- Tauri 2.12 发布说明：<https://v2.tauri.app/blog/tauri-2.12/>
- Tauri capabilities：<https://v2.tauri.app/security/capabilities/>
- Tauri CSP：<https://v2.tauri.app/security/csp/>
- SQLite WAL：<https://www.sqlite.org/wal.html>
- SQLite PRAGMA：<https://www.sqlite.org/pragma.html>
- Vite：<https://vite.dev/guide/>
- shadcn/ui Tailwind v4：<https://ui.shadcn.com/docs/tailwind-v4>
- TypeScript 7 / typescript-eslint 兼容性：<https://github.com/typescript-eslint/typescript-eslint/issues/12518>