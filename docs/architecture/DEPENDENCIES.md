# 依赖与工具链基线

> 当前 Windows x64 `0.1.1` 候选已通过 Phase 7 宿主门禁，Rust 213 项和前端 84 项通过，NSIS 构建及载荷核验通过（2026-11-18，北京时间）。代码签名仍待配置，Phase 7A.4 来宾及原始现场验收保持 `VALIDATION_PENDING`。
>
> 报告：`docs/phases/PHASE_6_REPORT.md`。本文件只记录实际安装、实际锁定并通过检查的版本。依赖升级必须重新生成
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
| Claude Code | 2.1.319 | PASS（Schema 核对） | `claude --version`；Hooks reference/guide/settings 官方文档 |

## 2. 前端直接依赖

锁文件：`pnpm-lock.yaml`（140 KB）；`pnpm install --frozen-lockfile` 退出码 0。

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
| `@radix-ui/react-dialog` | 1.3.1 | Dialog / ConfirmationDialog |
| `@radix-ui/react-dropdown-menu` | 2.2.1 | 项目/任务行操作菜单 |
| `@radix-ui/react-select` | 2.4.1 | 项目/状态/优先级筛选 |
| `@radix-ui/react-tabs` | 1.2.1 | 项目活动/归档分区 |
| `@radix-ui/react-tooltip` | 1.4.1 | 图标按钮提示 |
| `sonner` | 2.0.8 | 操作 Toast 反馈 |

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

锁文件：`Cargo.lock`（105 KB）；`cargo check --workspace --all-targets --locked` 退出码 0。

| 包 | 锁定版本 | 说明 |
|---|---|---|
| `tauri` | 2.12.5 | 桌面壳与 IPC |
| `tauri-build` | 2.7.4 | 构建期依赖（独立版本线） |
| `wry` | 0.57.1 | WebView 绑定 |
| `tao` | 0.37.2 | 窗口库 |
| `serde` | 1.0.229 | DTO/命令序列化 |
| `rusqlite` | 0.41.1 | SQLite 访问；启用 `bundled` + `backup`（Phase 6 的 Online Backup API；feature 不新增 crate/版本） |
| `libsqlite3-sys` | 0.39.1 | 随 rusqlite 锁定的 SQLite FFI |
| `uuid` | 1.29.0 | v4 记录 ID |
| `chrono` | 0.4.45 | UTC RFC3339 时间 |
| `thiserror` | 2.0.21 | 核心错误类型（传递依赖另有 1.0.69） |
| `regex` | 1.13.1 | 验证输出模式化脱敏 |
| `serde_json` | 1.0.154 | 验证参数数组与证据元数据 |
| `sha2` | 0.10.9 | 工作区内容指纹与证据摘要哈希 |
| `windows-sys` | 0.61.2 | Windows Job Object 进程树清理 + Phase 3.6 窗口 Snap 覆盖层（仅 Windows target） |
| `tempfile` | 3.27.0 | 仅 dev/test 临时数据库与 Hook 测试 |

内部 crate：

| crate | 说明 |
|---|---|
| `xushu-core` | 领域、SQLite、验证据、Claude 接入与导入 |
| `xushu-hook-protocol` | Hook 白名单信封、字段校验、带盐哈希；不依赖 SQLite |
| `xushu-hook` | 独立 Hook CLI；依赖 `serde_json` 与协议 crate |
| `xushu-desktop` | Tauri 命令、窗口与状态；Phase 6 直接使用已在锁文件中的 `chrono 0.4.45` 与 `uuid 1.29.0` 生成恢复时间与操作 ID |

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
8. **Vitest 5.1.0 / jsdom 31.0.0** 与 Node 24.20.0、Vite 8.3.6 兼容；Phase 4 实测 48 个前端测试通过。
9. **rusqlite 0.41.1 bundled** 在 MSVC 14.51 + Windows SDK 10.0.26100 下实际编译、链接并运行；WAL、外键与迁移测试通过。
10. `@tauri-apps/cli 2.12.4` 与 `tauri 2.12.5` 已在本机实际编译并启动。
11. **Radix UI 1.x/2.x 与 sonner 2.0.8** 均声明支持 React 19；实际 typecheck、Lint、Vitest 与桌面启动通过。
12. Phase 2 未新增 Rust crate；状态机、项目/任务服务、Git 只读检查使用现有 `rusqlite` / `serde` / `uuid` / `chrono` / `thiserror`。
13. Phase 3 新增 `regex 1.13.1`、`serde_json 1.0.154`、`sha2 0.10.9` 为直接依赖；三者此前已在 `Cargo.lock` 中作为传递依赖存在，版本未变。
14. `windows-sys 0.61.2` 在 `xushu-core` 中用于 Windows Job Object，启用 `Win32_Foundation`、`Win32_Security`、`Win32_System_JobObjects`、`Win32_System_Threading`；Phase 3.6 在 desktop crate 中为窗口 Snap 覆盖层启用 `Win32_Foundation`、`Win32_Graphics_Dwm`、`Win32_Graphics_Gdi`、`Win32_System_LibraryLoader`、`Win32_UI_HiDpi`、`Win32_UI_Input_KeyboardAndMouse`、`Win32_UI_Shell`、`Win32_UI_WindowsAndMessaging`。版本不变，无新 crate 版本进入锁文件；非 Windows 不编译该依赖。
15. 验证运行器使用 `std::process::Command` 与参数数组，不引入 Shell 插件或命令执行框架。
16. Phase 3.5 未新增前端或 Rust 依赖；仅新增 `core:window:allow-set-theme` 能力，用于原生标题栏主题同步。
17. Phase 3.6 未新增前端或 Rust 包版本；`windows-sys 0.61.2` 此前已在锁文件中，仅为 desktop crate 启用窗口相关 feature。未引入 `tauri-plugin-frame` / `decorum`，未启用 `withGlobalTauri`。capabilities 新增 `start-dragging`、`internal-toggle-maximize`、`minimize`、`toggle-maximize`、`close`、`is-maximized`，均为自定义窗口控制的最小权限。
18. Phase 4 新增内部 crate `xushu-hook-protocol` 与 `xushu-hook`；未引入 Clap、Tokio、HTTP 服务、Shell 插件或新第三方运行时。`serde_json`/`sha2`/`chrono`/`uuid`/`thiserror` 已在锁文件中，版本未变。
19. Phase 4 的 Hook 配置使用官方 `command` + `args` exec 形式，不经过 shell；Claude Code 2.1.319 官方文档核对了六个事件字段与 `PostToolUseFailure` 的 `error`/`is_interrupt`。`Stop` 的 `last_assistant_message` 与 `PostToolUse` 的 `tool_response` 被明确忽略。
20. Phase 4 没有新增 Tauri capability 或插件；新增的七个 Claude IPC 命令都是应用自定义 command，前端仍无通用 FS/Shell/SQL 权限。
21. Phase 6 只启用 `rusqlite` 的 `backup` feature；`Cargo.lock` 未新增第三方 crate/版本。`xushu-desktop` 新增的直接依赖 `chrono 0.4.45`、`uuid 1.29.0` 原本已在锁文件中（来自 `xushu-core`），版本不变。
22. Phase 6 未新增 Tauri capability、插件或前端依赖；新增的 15 个数据保护命令都是应用自定义 command。前端仍无通用 SQL、Shell、任意文件系统写入或系统配置修改权限。
23. 当前 Windows MSVC 发布由 `pnpm release:windows` 触发 Tauri 的 `beforeBuildCommand`，其中 `pnpm sidecar:prepare` 调用 `scripts/prepare-sidecar.mjs`。脚本用 `cargo rustc --locked --release -p xushu-hook -- -C target-feature=+crt-static` 构建接收器，有指定的编译目标时在 `--` 前加入 `--target <triple>`。生成文件暂存为 `apps/desktop/src-tauri/binaries/xushu-hook-<triple>.exe`，供 Tauri `externalBin` 打包。未引入 `tauri-plugin-shell`、`withGlobalTauri` 或新的 capability。
24. Phase 7A 未新增第三方 crate 或前端包；Release profile 只调整 Cargo 优化/LTO/strip。NSIS 3.13 与 nsis_tauri_utils 0.5.3 由 Tauri CLI 下载并校验，不属于应用运行时依赖。
25. Windows 安装包为 currentUser NSIS；WebView2 使用 `downloadBootstrapper`（安装时检测，缺失则静默运行官方 bootstrapper，需要联网）。未嵌入固定 WebView2 运行时。
26. 安装版主程序与 sidecar 版本必须一致；启动探测只执行固定的 `--version` 参数，不经过 shell。该探测不新增 IPC 命令或前端权限。

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
- Schema（迁移版本 7）：`schema_migrations`、`persistence_probes`、`projects`、`tasks`、
  `acceptance_criteria`、`task_transitions`、`manual_confirmations`、`verification_specs`、
  `verification_runs`、`verification_checks`、`evidence`、`claude_integrations`、
  `claude_sessions`、`claude_tool_calls`、`claude_events`、`project_memories`、
  `project_memory_revisions`、`decisions`、`decision_revisions`、`project_recovery_preferences`、
  `recovery_snapshots`、`backup_records`、`backup_settings`、`restore_operations`。
- Phase 2 业务表使用 UUID、UTC RFC3339、外键、CHECK 约束、唯一根路径 key 与人工确认修订唯一索引。
- Phase 3 验证表保留运行快照、修订号、Git/工作区指纹、脱敏输出与证据哈希；检查与证据按 run 唯一。
- Phase 4 Claude 表只保存白名单元数据与带盐哈希；不保存原始 Hook JSON、Prompt、命令、文件内容、工具输出或 transcript。`delivery_id` 唯一约束保证导入幂等，工具事件带语义去重键。
- Capabilities：`core:default` + `core:window:allow-set-theme` + `start-dragging` / `internal-toggle-maximize` / `minimize` / `toggle-maximize` / `close` / `is-maximized`（Phase 3.6 自定义窗口控制）；无 Shell / FS / SQL 插件权限，未启用 `withGlobalTauri`。
- CSP：
  - 生产：`default-src 'self'; connect-src ipc: http://ipc.localhost; ...; script-src 'self'; object-src 'none'; base-uri 'self'`
  - 开发：额外允许 `ws://localhost:1420/1421` 与 Vite 内联 refresh 脚本。
  - 生产 CSP 已在 Phase 7A 安装版 E2E 中实际加载页面并通过 CDP 调用 Tauri IPC；未放宽 `script-src`。

## 7. 本轮实际通过的质量门禁（2026-11-09）

| 命令 | 退出码 | 结果 |
|---|---|---|
| `pnpm install --frozen-lockfile` | 0 | 锁文件一致 |
| `pnpm typecheck` | 0 | `tsc --noEmit` 通过 |
| `pnpm lint` | 0 | ESLint 10，`--max-warnings 0` |
| `pnpm test` | 0 | 8 个测试文件、48 个测试通过（Phase 4 新增 5 个接入测试） |
| `pnpm build` | 0 | Vite 生产构建通过；JS 827.13 kB / gzip 246.40 kB（有 >500 kB chunk 提示） |
| `pnpm format` | 0 | Prettier 检查通过 |
| `cargo check --workspace --all-targets --locked` | 0 | Rust workspace 通过 |
| `cargo fmt --all -- --check` | 0 | rustfmt 通过 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | Clippy 通过 |
| `cargo test --workspace` | 0 | 11 个 lib + 19 个业务 + 19 个 Claude + 12 个持久化 + 26 个验证 + 2 个桌面 + 6 个 Hook CLI + 5 个协议 = 100 个测试通过 |
| `pnpm tauri info` | 0 | 环境与依赖版本被正确识别 |
| Hook release 计时 | 0 | `xushu-hook.exe` 30 次调用平均 14.46 ms；无敏感内容落入 spool |
| 桌面无截图冒烟 | 0 | 项目详情接入区块、安装预览、1366/500 布局、工作台回归；未安装任何真实项目 Hooks |

### 7.1 Phase 6 实际质量门禁（2026-11-13）

| 命令 | 退出码 | 结果 |
|---|---|---|
| `pnpm typecheck` | 0 | 通过 |
| `pnpm lint` | 0 | ESLint 10，`--max-warnings 0` |
| `pnpm test` | 0 | 11 个文件、**83** 项通过 |
| `pnpm build` | 0 | JS **980.05 kB / gzip 280.77 kB**；仍有 >500 kB 提示 |
| `pnpm format` | 0 | Prettier 通过 |
| `cargo fmt --all -- --check` | 0 | 通过 |
| `cargo check --workspace --all-targets --locked` | 0 | 通过 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | 无警告 |
| `cargo test --workspace --locked` | 0 | **186** 项通过，0 失败，0 ignored |
| 隔离 Tauri E2E（完整恢复） | 0 | 备份、校验、损坏拒绝、正式恢复、最终校验、UI 可见 |
| 隔离 Tauri E2E（关闭重开） | 0 | 恢复后快照重开一致；正常 DB/WAL/SHM 指纹不变 |
| 隔离 Tauri E2E（布局/主题） | 0 | 4 视口 × 深浅主题 + 恢复向导；无页面级横向溢出 |

### 7.2 历史 Phase 7A 实际质量门禁（2026-11-14）

| 命令/构建 | 退出码 | 结果 |
|---|---|---|
| `pnpm install --frozen-lockfile` | 0 | 锁文件一致；供应链策略通过 |
| `pnpm typecheck` | 0 | 通过 |
| `pnpm lint` | 0 | ESLint 10，`--max-warnings 0` |
| `pnpm test` | 0 | 11 个文件、**83** 项通过 |
| `pnpm build` | 0 | Vite 通过；单 chunk **980.05 kB / gzip 280.77 kB**（原有 >500 kB 提示） |
| `pnpm format` | 0 | Prettier 通过 |
| `cargo fmt --all -- --check` | 0 | 通过 |
| `cargo check --workspace --all-targets --locked` | 0 | 通过 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | 无警告 |
| `cargo test --workspace --locked` | 0 | **207** 项通过，0 失败，0 ignored |
| `pnpm release:windows`（Tauri NSIS） | 0 | Release + `XUSHU_0.1.0_x64-setup.exe`；安装包 3,304,035 B |
| 隔离安装版 E2E | 0 | 安装/首启/重开/非正常退出/升级 0.1.1/sidecar 异常/窗口/卸载；正常 DB 指纹不变 |
| `git diff --check` | 0 | 无空白错误 |

- 测试构成：core lib 33、backup 22、business 19、claude 37、decisions 8、memory 8、persistence 13、recovery 14、verification 26、desktop lib 14、hook CLI 8、hook protocol 5 = **207**。
- Release profile：`codegen-units = 1`、`lto = true`、`opt-level = "s"`、`strip = true`；本机 Release 构建约 2m35s–3m06s，主程序约 11 MB。
- Tauri CLI 首次打包下载并校验 **NSIS 3.13** 与 **nsis_tauri_utils 0.5.3**；当时使用 Windows 用户工具缓存 `%LOCALAPPDATA%\tauri`。Phase 7 的 `0.1.1` 宿主构建通过 `bundle.useLocalToolsDir=true` 使用隔离的 `cargo-target/.tauri/NSIS/`。两处缓存均在提交范围外。
- 产物：`target/phase7a/production/XUSHU_0.1.0_x64-setup.exe`（SHA-256 `3bf2143f…`）、`xushu-desktop.exe`（`fb2d23ef…`）；隔离 E2E 包 0.1.0 `e6f771e6…`、0.1.1 `f675e465…`。
- 代码签名：`SIGNING_NOT_CONFIGURED`（Authenticode 检查结果为未签名）。

## 8. 已规划但本阶段未安装

- 备份加密与密钥管理依赖（未进入 Phase 6；需要新 ADR）
- 常驻自动备份调度/系统计划任务（未进入 Phase 6；当前仅应用运行期间检查）
- Windows 代码签名证书、时间戳服务与发布签名工具（Phase 7B）
- 自动更新签名、元数据与发布基础设施（未启用）
- MSI/WiX 打包（Phase 7A 只验证 NSIS）
- macOS `.app`/DMG、公证与 arm64 sidecar（Phase 7B）

## 9. 官方参考（核对日期：2026-11-14）

- Tauri prerequisites：<https://v2.tauri.app/start/prerequisites/>
- Tauri 2.12 发布说明：<https://v2.tauri.app/blog/tauri-2.12/>
- Tauri 嵌入外部二进制（externalBin）：<https://v2.tauri.app/develop/sidecar/>
- Tauri Windows 安装包与 WebView2：<https://v2.tauri.app/distribute/windows-installer/>
- Tauri 配置 schema（2.12.4 本地 `config.schema.json` 核对）
- Tauri capabilities：<https://v2.tauri.app/security/capabilities/>
- Tauri CSP：<https://v2.tauri.app/security/csp/>
- SQLite WAL：<https://www.sqlite.org/wal.html>
- SQLite PRAGMA：<https://www.sqlite.org/pragma.html>
- Claude Code Hooks：<https://code.claude.com/docs/en/hooks>
- Claude Code Hooks guide：<https://code.claude.com/docs/en/hooks-guide>
- Claude Code settings：<https://code.claude.com/docs/en/settings>
- Vite：<https://vite.dev/guide/>
- shadcn/ui Tailwind v4：<https://ui.shadcn.com/docs/tailwind-v4>
- TypeScript 7 / typescript-eslint 兼容性：<https://github.com/typescript-eslint/typescript-eslint/issues/12518>