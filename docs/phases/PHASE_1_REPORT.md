# Phase 1 报告：桌面基础与本地持久化

- 日期：2026-11-06
- 项目：序枢 XUSHU
- 根目录：`D:\MyProject\xushu`
- 总体状态：**PASS**
- 结论：XUSHU 已具备真实 SQLite 持久化、Rust 校验边界、受限 Tauri IPC、React 桌面外壳与跨进程读写验证。应用实际启动，写入记录后完全关闭进程，重新启动后通过 UI/IPC 读回同一 ID、内容和时间。

## 1. 实际完成的功能

### 1.1 SQLite 持久化基础

- 数据库位于 Tauri `app_data_dir()` 下的 `xushu.db`；Windows 实测
  `%APPDATA%\dev.xushu.desktop\xushu.db`，不进入 Git 仓库。
- 打开连接时启用 `foreign_keys=ON`、`journal_mode=WAL`、`synchronous=NORMAL`、
  `busy_timeout=5000ms`；WAL 无法启用时显式失败。
- `Database` 封装 `Mutex<rusqlite::Connection>`，提供打开、迁移、健康检查与事务 API。
- 健康检查执行 `PRAGMA integrity_check` 与 `PRAGMA foreign_key_check`，返回 schema
  版本、日志模式、外键状态、记录数与数据库文件名（不含绝对路径）。
- 错误类型 `CoreError` 覆盖数据库、IO、锁、校验、未找到、schema 过新与迁移配置错误。

### 1.2 版本化迁移

- `schema_migrations(version, name, applied_at)` 记录已应用版本。
- 每个迁移在独立事务中执行 SQL、写入版本记录、更新 `PRAGMA user_version`；失败回滚。
- 重复执行幂等；重复版本拒绝；数据库版本高于应用支持版本时拒绝降级。
- Phase 1 只有迁移 1：`0001_create_persistence_probes.sql`，创建
  `persistence_probes` 表与时间索引。未创建 Phase 2 项目/任务/验收表。

### 1.3 Tauri IPC

已注册且仅注册五个命令：

1. `get_app_info`
2. `get_database_health`
3. `create_probe_record`
4. `list_probe_records`
5. `get_probe_record`

- 写入输入由 Rust 再次校验：标签 1–64 字符、内容 1–2000 字符、拒绝控制字符；
  ID 必须为 UUID；列表 limit 1–100。
- 前端不执行 SQL，不接收数据库路径；未安装 Shell、文件系统或 SQL 插件。
- 错误返回 `{ code, message }`；内部数据库错误不向前端返回 SQL、路径或凭据。
- 数据库初始化失败时窗口仍启动，命令返回 `DATABASE_UNAVAILABLE`，UI 展示错误态。

### 1.4 React 桌面外壳

- 侧边导航与三页路由：概览、本地数据、设置；未知路径显示 404。
- Tailwind 4 + shadcn/ui 结构（Button、Card、Input、Textarea、Label、Badge、Alert、Skeleton）。
- 浅色/深色/跟随系统主题，Zustand + localStorage 持久化；`matchMedia` 跟随系统变化。
- TanStack Query 管理应用信息、健康状态、测试记录列表；写入后失效并刷新。
- React Hook Form + Zod 做表单体验校验，Rust 仍是最终校验。
- 全局 Error Boundary；健康与记录分别有加载、错误、空状态。
- 语义化导航、跳过链接、标签关联、`aria-invalid`、`aria-busy`、`role="alert"`、
  键盘焦点环；状态不只靠颜色表达。
- 不展示虚构项目、任务或 KPI；只展示真实后端查询与已实现范围。

### 1.5 跨进程持久化验证

已完成真实测试：

1. 启动桌面应用；
2. 通过 UI 表单提交（React → Tauri IPC → Rust → SQLite）写入记录；
3. 独立只读 SQLite 查询确认记录落盘；
4. 完全关闭 `xushu-desktop.exe`；
5. 重新启动应用；
6. 通过 UI 的数据列表（Tauri IPC `list_probe_records`）读回记录；
7. 比对 ID、内容、时间字段全部一致。

详见第 9 节。

## 2. 最终工程目录结构

```text
D:\MyProject\xushu\
├── apps/desktop/
│   ├── components.json
│   ├── index.html
│   ├── package.json
│   ├── tsconfig.json
│   ├── vite.config.ts
│   ├── vitest.config.ts
│   ├── eslint.config.js
│   ├── src/
│   │   ├── App.tsx
│   │   ├── App.test.tsx
│   │   ├── main.tsx
│   │   ├── index.css
│   │   ├── vite-env.d.ts
│   │   ├── app/
│   │   │   ├── app-layout.tsx
│   │   │   ├── providers.tsx
│   │   │   └── theme-sync.tsx
│   │   ├── components/
│   │   │   ├── app-error-boundary.tsx
│   │   │   ├── app-error-boundary.test.tsx
│   │   │   ├── page-header.tsx
│   │   │   ├── theme-control.tsx
│   │   │   └── ui/
│   │   │       ├── alert.tsx
│   │   │       ├── badge.tsx
│   │   │       ├── button.tsx
│   │   │       ├── card.tsx
│   │   │       ├── input.tsx
│   │   │       ├── label.tsx
│   │   │       ├── skeleton.tsx
│   │   │       └── textarea.tsx
│   │   ├── features/
│   │   │   ├── persistence/
│   │   │   │   ├── api.ts
│   │   │   │   ├── probe-form.tsx
│   │   │   │   ├── probe-list.tsx
│   │   │   │   ├── queries.ts
│   │   │   │   ├── schema.ts
│   │   │   │   ├── schema.test.ts
│   │   │   │   └── types.ts
│   │   │   └── system/
│   │   │       ├── api.ts
│   │   │       ├── health-badge.tsx
│   │   │       ├── queries.ts
│   │   │       └── types.ts
│   │   ├── lib/
│   │   │   ├── ipc.ts
│   │   │   ├── ipc.test.ts
│   │   │   └── utils.ts
│   │   ├── pages/
│   │   │   ├── not-found-page.tsx
│   │   │   ├── overview-page.tsx
│   │   │   ├── persistence-page.tsx
│   │   │   └── settings-page.tsx
│   │   ├── stores/theme-store.ts
│   │   └── test/setup.ts
│   └── src-tauri/
│       ├── Cargo.toml
│       ├── build.rs
│       ├── tauri.conf.json
│       ├── capabilities/default.json
│       ├── icons/
│       ├── app-icon.svg
│       └── src/
│           ├── commands.rs
│           ├── error.rs
│           ├── lib.rs
│           ├── main.rs
│           └── state.rs
├── crates/xushu-core/
│   ├── Cargo.toml
│   ├── migrations/0001_create_persistence_probes.sql
│   ├── src/
│   │   ├── db/mod.rs
│   │   ├── db/migrations.rs
│   │   ├── error.rs
│   │   ├── lib.rs
│   │   └── probe.rs
│   └── tests/persistence.rs
├── docs/
│   ├── architecture/
│   │   ├── ADR/0001-local-first-tauri-stack.md
│   │   ├── ADR/0002-sqlite-persistence-and-ipc.md
│   │   └── DEPENDENCIES.md
│   └── phases/
│       ├── PHASE_0_REPORT.md
│       └── PHASE_1_REPORT.md
├── Cargo.toml
├── Cargo.lock
├── package.json
├── pnpm-workspace.yaml
├── pnpm-lock.yaml
├── rust-toolchain.toml
└── README.md
```

构建产物 `node_modules/`、`dist/`、`target/`、`src-tauri/gen/` 与运行时数据库均被忽略。

## 3. 新增及修改文件清单

### 3.1 基线提交

- `25c430b chore: 完成 Phase 0 工程基线`（本地提交，未推送）。
- Phase 1 变更未提交，等待用户审查；未执行 `git push`。

### 3.2 新增文件

**Rust / 数据库**

- `crates/xushu-core/src/db/mod.rs`
- `crates/xushu-core/src/db/migrations.rs`
- `crates/xushu-core/src/error.rs`
- `crates/xushu-core/src/probe.rs`
- `crates/xushu-core/migrations/0001_create_persistence_probes.sql`
- `crates/xushu-core/tests/persistence.rs`
- `apps/desktop/src-tauri/src/commands.rs`
- `apps/desktop/src-tauri/src/error.rs`
- `apps/desktop/src-tauri/src/state.rs`

**前端**

- `apps/desktop/components.json`
- `apps/desktop/src/app/app-layout.tsx`
- `apps/desktop/src/app/providers.tsx`
- `apps/desktop/src/app/theme-sync.tsx`
- `apps/desktop/src/components/app-error-boundary.tsx`
- `apps/desktop/src/components/app-error-boundary.test.tsx`
- `apps/desktop/src/components/page-header.tsx`
- `apps/desktop/src/components/theme-control.tsx`
- `apps/desktop/src/components/ui/{alert,badge,button,card,input,label,skeleton,textarea}.tsx`
- `apps/desktop/src/features/system/api.ts`
- `apps/desktop/src/features/system/types.ts`
- `apps/desktop/src/features/system/queries.ts`
- `apps/desktop/src/features/system/health-badge.tsx`
- `apps/desktop/src/features/persistence/api.ts`
- `apps/desktop/src/features/persistence/types.ts`
- `apps/desktop/src/features/persistence/schema.ts`
- `apps/desktop/src/features/persistence/queries.ts`
- `apps/desktop/src/features/persistence/probe-form.tsx`
- `apps/desktop/src/features/persistence/probe-list.tsx`
- `apps/desktop/src/features/persistence/schema.test.ts`
- `apps/desktop/src/lib/{ipc,utils}.ts`
- `apps/desktop/src/lib/ipc.test.ts`
- `apps/desktop/src/pages/{overview,persistence,settings,not-found}-page.tsx`
- `apps/desktop/src/stores/theme-store.ts`

**文档**

- `docs/architecture/ADR/0002-sqlite-persistence-and-ipc.md`
- `docs/phases/PHASE_1_REPORT.md`

### 3.3 修改文件

- `Cargo.toml`：新增 workspace `serde` 依赖。
- `Cargo.lock`：新增 rusqlite、chrono、uuid 等锁定版本。
- `crates/xushu-core/Cargo.toml`：Rust 依赖与 dev 依赖。
- `crates/xushu-core/src/lib.rs`：导出 DB、错误、测试记录 API。
- `apps/desktop/src-tauri/Cargo.toml`：serde、xushu-core 路径依赖。
- `apps/desktop/src-tauri/src/lib.rs`：数据库初始化、五个命令注册。
- `apps/desktop/src-tauri/tauri.conf.json`：生产 CSP 与 devCsp。
- `apps/desktop/package.json`：前端运行时/测试依赖。
- `apps/desktop/src/App.tsx`、`App.test.tsx`：路由、Providers、测试。
- `apps/desktop/src/index.css`：Tailwind 4 + shadcn 设计令牌、深色变体。
- `apps/desktop/src/test/setup.ts`：`matchMedia` mock。
- `apps/desktop/tsconfig.json`：`@/*` 路径别名。
- `apps/desktop/vite.config.ts`、`vitest.config.ts`：别名配置。
- `pnpm-lock.yaml`、`pnpm-workspace.yaml`：新增依赖与供应链豁免记录。
- `README.md`、`docs/architecture/DEPENDENCIES.md`：Phase 1 状态与版本。

## 4. SQLite 数据库与 Migration 设计

### 4.1 数据库位置与配置

```text
Windows: %APPDATA%\dev.xushu.desktop\xushu.db
        xushu.db-wal
        xushu.db-shm
```

- 由 Rust `AppState::initialize` 使用 Tauri `app_data_dir()` 创建目录并打开数据库。
- 运行时数据库不被 Git 跟踪；`.gitignore` 已覆盖 `*.db`、`*.db-wal`、`*.db-shm`。
- PRAGMA：
  - `foreign_keys = ON`
  - `journal_mode = WAL`
  - `synchronous = NORMAL`
  - `busy_timeout = 5000`

### 4.2 Schema

```sql
CREATE TABLE schema_migrations (
    version INTEGER PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    applied_at TEXT NOT NULL
);

CREATE TABLE persistence_probes (
    id TEXT PRIMARY KEY NOT NULL,
    label TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX idx_persistence_probes_created_at
    ON persistence_probes (created_at DESC);
```

`persistence_probes` 是 Phase 1 的持久化探针，不是 Phase 2 的任务/项目/验收表。

### 4.3 迁移算法

1. `CREATE TABLE IF NOT EXISTS schema_migrations`；
2. 校验迁移版本为正整数、名称非空、版本不重复；
3. 按版本排序，读取已应用版本；
4. 若数据库已应用版本高于当前支持版本，返回 `SchemaTooNew`；
5. 对每个未应用迁移：
   - 开始事务；
   - `execute_batch` 执行迁移 SQL；
   - 写入 `schema_migrations`；
   - 更新 `PRAGMA user_version`；
   - 提交；
   - 任一步失败则事务回滚；
6. 返回本次应用版本与当前版本。

### 4.4 事务与错误

- `Database::with_transaction` 提供显式事务；闭包返回错误时不提交。
- `CoreError` 区分 `Database`、`Io`、`Unavailable`、`LockPoisoned`、`Validation`、
  `NotFound`、`SchemaTooNew`、`InvalidMigration`。
- 迁移失败、重复版本、schema 降级、外键违规、事务回滚均有集成测试。

### 4.5 Rust 测试覆盖（12 个集成测试 + 1 个 lib 测试）

- 打开数据库后 PRAGMA/健康状态正确；
- 迁移幂等；
- 迁移失败整段回滚；
- 重复迁移版本拒绝；
- 更新版本数据库拒绝降级；
- 外键约束真实生效；
- 事务错误回滚；
- 记录 ID/内容/时间往返一致；
- 非法标签/内容/ID/limit 校验；
- 缺失记录返回 `None`；
- 自动创建嵌套父目录。

## 5. Tauri IPC 命令与权限

### 5.1 命令表

| 命令 | 输入 | 输出 | 校验 / 备注 |
|---|---|---|---|
| `get_app_info` | 无 | `name, version, tauriVersion, phase, platform` | 不接收参数 |
| `get_database_health` | 无 | 健康、完整性、外键、WAL、schema、记录数、文件名 | 只读 |
| `create_probe_record` | `{ input: { label, content } }` | `ProbeRecord` | Rust 校验长度/控制字符 |
| `list_probe_records` | `{ limit?: number }` | `ProbeRecord[]` | 1–100，默认 20 |
| `get_probe_record` | `{ id: string }` | `ProbeRecord` | UUID 校验；不存在返回 `NOT_FOUND` |

`ProbeRecord`：`{ id, label, content, createdAt }`，时间为 UTC RFC3339。

### 5.2 错误模型

```json
{ "code": "VALIDATION_ERROR", "message": "标签不能为空" }
```

稳定错误代码：`VALIDATION_ERROR`、`NOT_FOUND`、`DATABASE_UNAVAILABLE`、
`DATABASE_LOCK_ERROR`、`DATABASE_ERROR`、`SCHEMA_TOO_NEW`。

内部错误只记录错误代码到 stderr，不向前端返回 SQL、绝对路径或凭据。

### 5.3 权限与 CSP

- `capabilities/default.json` 仅 `core:default`，窗口仅 `main`。
- 没有安装 `tauri-plugin-shell`、`tauri-plugin-fs`、`tauri-plugin-sql` 或任意通用执行插件。
- 生产 CSP：
  ```text
  default-src 'self'; connect-src ipc: http://ipc.localhost;
  img-src 'self' asset: http://asset.localhost data:;
  style-src 'self' 'unsafe-inline'; font-src 'self' data:;
  script-src 'self'; object-src 'none'; base-uri 'self'
  ```
- 开发 `devCsp` 额外允许 `ws://localhost:1420/1421` 与 Vite 内联 refresh 脚本。
- `style-src 'unsafe-inline'` 是 shadcn/Radix/主题切换所需；脚本源仍受限。
- 生产 CSP 已通过 `pnpm tauri info` 显示，但尚未用 `tauri build` 成品运行验证。

## 6. React UI 架构

### 6.1 路由与布局

- `HashRouter` + `Routes`，避免 Tauri 自定义协议下深层路径刷新问题。
- `AppLayout`：侧边导航（概览/本地数据/设置）、品牌区、主题控件、主内容区。
- 404 页面与返回概览入口。
- 跳过链接可聚焦 `main`，导航有 `aria-label`，当前项由 NavLink 标记。

### 6.2 状态分工

| 状态 | 工具 | 说明 |
|---|---|---|
| IPC 查询/失效 | TanStack Query | 应用信息、健康、记录列表 |
| 主题模式 | Zustand + persist | `light` / `dark` / `system` |
| 表单状态/校验 | React Hook Form + Zod | 前端体验校验；Rust 二次校验 |
| 路由 | React Router 8 | HashRouter |
| 渲染异常 | Error Boundary | 错误兜底与重新加载 |

### 6.3 页面

- **概览**：真实数据库健康、应用版本、Phase 1 已实现范围；无虚构 KPI。
- **本地数据**：测试记录表单 + 最近记录列表；加载、错误、空状态齐全；写入成功后显示
  ID 与时间，并失效健康/列表查询。
- **设置**：浅色/深色/跟随系统；关于信息；本地数据与权限说明（不展示绝对路径）。
- **404**：无匹配路由时的明确状态。

### 6.4 视觉与可访问性

- 设计令牌：CSS 变量 + Tailwind 4 `@theme inline`；浅色/深色两套 oklch 颜色。
- 组件遵循 shadcn/ui 结构（`components/ui`、`lib/utils`、`components.json`）。
- 状态使用图标 + 文本 + Badge，不只靠颜色。
- 表单错误 `role="alert"`，输入有 `aria-invalid`/`aria-describedby`。
- 加载容器 `aria-busy`，隐藏状态文本用 `sr-only`。
- 支持 `prefers-reduced-motion` 降低动画。
- 未使用自动截图；视觉验收需用户手动查看。

### 6.5 前端测试

- 4 个测试文件、13 个测试：
  - 应用渲染真实健康/版本；
  - 导航到本地数据与空状态；
  - 空表单校验不调用后端；
  - 写入表单 → IPC 调用 → 记录显示；
  - 深浅主题切换；
  - 健康查询失败显示可读错误；
  - `invokeCommand` 结构化/非结构化错误处理；
  - Zod 校验（中文、换行、空值、超长、控制字符）；
  - Error Boundary 兜底。

## 7. 实际执行的测试命令与退出码

最终复跑时间：2026-11-06。

| 命令 | 退出码 | 结果 |
|---|---|---|
| `pnpm install --frozen-lockfile` | 0 | 锁文件一致 |
| `pnpm typecheck` | 0 | `tsc --noEmit` 通过 |
| `pnpm lint` | 0 | ESLint 10，`--max-warnings 0` |
| `pnpm test` | 0 | 4 个文件、13 个测试通过 |
| `pnpm build` | 0 | Vite 生产构建；JS 492.54 kB / gzip 154.01 kB |
| `pnpm format` | 0 | Prettier 全部匹配 |
| `cargo check --workspace --all-targets --locked` | 0 | Rust workspace 通过 |
| `cargo fmt --all -- --check` | 0 | rustfmt 通过 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | Clippy 无警告 |
| `cargo test --workspace` | 0 | 1 个 lib 测试 + 12 个集成测试通过 |
| `pnpm tauri info` | 0 | Tauri/Rust/Node/WebView2/MSVC 正确识别 |

开发过程中发现并修复的真实问题（均已复跑通过）：

1. TypeScript 6.0 对 `baseUrl` 发出弃用错误（TS5101）；移除 `baseUrl`，保留 `paths`。
2. ESLint 报告 `no-control-regex` 与 react-refresh 非组件导出警告；改为字符扫描校验，
   并停止从组件文件导出未使用的 variants。
3. 健康查询错误测试的默认 1 秒超时早于 TanStack Query 重试结束；按真实重试时间延长断言。
4. 新增 Rust 文件缺少 rustfmt 末尾换行/换行风格；执行 `cargo fmt` 并复跑。
5. `Cargo.lock` 尚未包含桌面新增依赖；先更新锁文件，再以 `--locked` 复验。

**NOT_RUN：**

- `pnpm tauri build` / Windows 安装包；
- 生产 CSP 下的成品运行验证；
- macOS 实机；
- `get_probe_record` 的 Tauri 端到端调用（其核心 `get_probe` 有集成测试；E2E 读取走
  `list_probe_records`）；
- Phase 2 业务功能测试。

## 8. 应用真实启动结果

### 8.1 首次启动（写入前）

命令：`pnpm tauri dev`（设置 `CARGO_BUILD_JOBS=2`，后台执行）。

日志证据：

```text
VITE v8.3.6 ready in 241 ms
Local: http://localhost:1420/
Compiling xushu-desktop v0.1.0
Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.21s
Running `D:\MyProject\xushu\target\debug\xushu-desktop.exe`
```

- Windows UI Automation 读取到真实窗口 `序枢 XUSHU`（句柄 38931202）。
- UI 显示：
  - 数据库文件 `xushu.db`
  - Schema 版本 `v1`
  - 日志模式 `WAL`
  - 外键约束 `已开启`
  - 完整性检查 `通过`
  - 测试记录 `0 条`
  - 应用版本 `0.1.0`、Tauri `2.12.5`、平台 `windows`
- 应用数据目录实际出现 `xushu.db`、`xushu.db-shm`、`xushu.db-wal`。

### 8.2 内存事件

首次后台 `pnpm tauri dev` 在窗口启动后被 Claude Code 因系统内存不足终止；应用进程
（PID 34644）仍然存活，窗口与 WebView2 正常。该事件不是应用启动失败。

- 为降低后续内存压力，用户明确授权一次重启；
- 重启未使用 `tauri dev`，而是复用 Vite 开发服务器并直接运行已编译的
  `target/debug/xushu-desktop.exe`；
- 第一次重启编排的就绪检查误用 `127.0.0.1`，而 Vite 绑定 `localhost`，在应用启动前
  即被清理，未产生应用进程；修正就绪检查后只启动应用一次；
- 重启后的进程为 PID 21708，窗口句柄 9571224，UI 正常加载并读回数据；
- 验证结束后关闭应用，清理 Vite 进程，确认无 `xushu-desktop` 进程、1420/1421/9222 端口无监听。

### 8.3 CDP 说明

尝试通过 `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222` 启用
WebView2 调试端口，但该参数未出现在 WebView2 子进程命令行中，9222 未开放。
因此改用 **Windows UI Automation** 驱动真实窗口与表单，仍通过 UI 的 IPC 行为完成
写入与读取；未使用截图或录屏。

## 9. 跨进程持久化测试结果

### 9.1 写入

通过 UI Automation 在“本地数据”页输入并提交：

| 字段 | 值 |
|---|---|
| 标签 | `E2E 跨进程验证` |
| 内容 | `phase1-persistence-2026-11-05T15:57:55.635Z` |
| 生成的 ID | `41092a7e-dc01-40e3-bb50-fb0629a6584d` |
| 生成时间（UTC） | `2026-11-05T15:57:55.675Z` |
| 本地显示时间 | `2026年11月5日 23:57:55` |

写入后 UI 显示“已写入 SQLite”，列表出现该记录，计数为 1。

### 9.2 独立 SQLite 只读核验

使用 Python `sqlite3` 以 `mode=ro` 打开数据库（只读，不通过应用）：

```text
integrity: ok
journal_mode: wal
migrations: 1 条（0001_create_persistence_probes）
records: 1 条
id: 41092a7e-dc01-40e3-bb50-fb0629a6584d
content: phase1-persistence-2026-11-05T15:57:55.635Z
created_at: 2026-11-05T15:57:55.675Z
```

说明：该独立连接未设置 `PRAGMA foreign_keys=ON`，所以 Python 连接读到 `foreign_keys=0`；
这是每个 SQLite 连接独立的默认值，不是应用连接配置。应用健康查询通过 IPC 显示
外键已开启。

### 9.3 完全关闭与重新启动

1. 通过 UIA `WindowPattern.Close()` 关闭应用；
2. 确认 `xushu-desktop` 进程不存在；
3. 启动 Vite，再直接运行 `target/debug/xushu-desktop.exe`；
4. 新进程 PID 21708，真实窗口句柄 9571224；
5. 导航到“本地数据”，列表由 `list_probe_records` 通过 IPC 从 SQLite 读取。

### 9.4 比对结果

UIA 读取到的文本/HelpText 与写入值比对：

| 检查 | 结果 |
|---|---|
| ID 一致 | PASS |
| 标签一致 | PASS |
| 内容一致 | PASS |
| 本地时间一致（`2026年11月5日 23:57:55`） | PASS |
| 原始 UTC 时间出现在 HelpText（`2026-11-05T15:57:55.675Z`） | PASS |

证据文件（临时目录，未提交）：

- `C:\Users\kmld\AppData\Local\Temp\xushu-phase1-e2e\uia-write-evidence.json`
- `C:\Users\kmld\AppData\Local\Temp\xushu-phase1-e2e\uia-read-evidence.json`

读取证据中的 `checks` 全为 `true`；`appPid=21708`、`windowHandle=9571224`、
`capturedAt=2026-11-05T16:08:33.910Z`。

该结果不是单次运行中的内存状态：应用进程已完全退出并重新创建，数据来自重启后的
SQLite 读取。

## 10. 安全与隐私检查

- 未读取或输出 API Key、`.env`、SSH 私钥、token、credential store。
- 未修改全局 Claude Code / DeepSeek / Git / Shell / 注册表配置。
- 未执行管理员安装；未安装任何系统级软件。
- 未安装 Claude Code Hooks；未注册系统服务；未修改其他项目。
- 未使用子 Agent / 多 Agent；未截图/录屏。
- Tauri capabilities 仅 `core:default`；没有 Shell/FS/SQL 插件权限。
- IPC 白名单只有五个命令；前端不能执行任意 SQL、任意 shell 或任意文件读写。
- 写入输入在 Rust 侧二次校验；错误消息不含绝对路径、SQL 或凭据。
- 数据库位于系统应用数据目录，不进入 Git；`.gitignore` 已覆盖数据库/WAL/日志/密钥。
- CSP 已配置生产/开发两套策略；生产 `script-src 'self'`。
- 未执行破坏性 Git 操作；Phase 0 基线为本地提交，未推送；Phase 1 变更未提交。
- 内存安全：Cargo 使用 `CARGO_BUILD_JOBS=2`；未同时运行多个大型 Rust 编译；
  验证后清理了本次启动的 Vite/应用进程；未终止其他用户进程。

## 11. 遗留问题与技术风险

1. **生产 CSP 未运行验证**：`tauri.conf.json` 的生产 CSP 只做了配置与 `tauri info`
   检查，未执行 `pnpm tauri build` 并启动成品；需要在 Phase 7 前补测。
2. **未构建安装包**：没有 MSI/NSIS 产物，也没有干净环境安装/卸载测试。
3. **macOS 未验证**：当前仅在 Windows 11 x64 实机验证；跨平台路径、进程与打包仍需适配。
4. **单一 SQLite 连接串行化**：`Mutex<Connection>` 适合当前单用户小查询；未来长时间查询
   或并发写入需要评估异步任务/连接分离。
5. **E2E 依赖 Windows UI Automation**：脚本位于临时目录，非跨平台自动化测试；不能替代
   未来正式 E2E 方案。
6. **启动失败恢复有限**：数据库初始化失败会显示 `DATABASE_UNAVAILABLE`，但没有重试、
   备份或修复 UI；迁移失败会阻止数据库打开。
7. **测试表生命周期**：`persistence_probes` 是 Phase 1 探针，后续如需删除或迁移，必须新增
   迁移，不能手工改历史；当前没有清理/保留策略。
8. **测试覆盖边界**：前端 13 个测试、Rust 13 个测试；没有 Tauri 命令层的独立集成测试
   （核心服务已覆盖），也没有完整 E2E 自动化套件。
9. **CSP 样式内联**：`style-src 'unsafe-inline'` 为 shadcn/Radix/主题所需；脚本源仍严格。
10. **供应链豁免**：`pnpm-workspace.yaml` 记录了 pnpm 12 对新版本的
    `minimumReleaseAgeExclude`；升级依赖时需重新评估。
11. **内存压力**：本轮两次后台任务被系统低内存策略终止；已通过 `CARGO_BUILD_JOBS=2`、
    直接运行二进制和及时清理降低风险，但机器内存仍是后续开发的风险因素。

## 12. Phase 1 最终验收

| 验收目标 | 结果 | 证据 |
|---|---|---|
| XUSHU 能启动 | PASS | 真实窗口「序枢 XUSHU」、WebView2、UIA 健康状态 |
| 写入本地 SQLite | PASS | UIA 表单提交；记录 ID/内容/时间；独立只读查询 |
| 完全退出后重新启动 | PASS | 原进程消失；新 PID 21708 |
| 重启后通过 IPC 读回原始数据 | PASS | 列表通过 `list_probe_records` 显示记录；ID/标签/内容/时间全部匹配 |
| SQLite 迁移/事务/回滚/完整性 | PASS | 12 个持久化集成测试；健康检查 |
| 受控 IPC 与权限 | PASS | 五个命令；capabilities 仅 `core:default`；无 Shell/FS/SQL 插件 |
| React UI 基础能力 | PASS | 路由、主题、shadcn 组件、错误边界、加载/错误/空态 |
| 质量门禁 | PASS | 10 条命令最终全部退出码 0 |
| 未提前进入 Phase 2 | PASS | 只有 `persistence_probes`，无任务/项目/验收业务表 |
| 安全与隐私 | PASS（带遗留项） | 无密钥/全局配置/管理员安装/Hooks；CSP 生产未实测 |

**Phase 1 最终状态：PASS。**

该 PASS 覆盖 Phase 1 的桌面启动、SQLite 持久化、IPC、UI 基础与跨进程读写；**不覆盖**
安装包、生产 CSP 实机、macOS 或 Phase 2 业务功能。

## 13. 标准汇报摘要

```text
当前阶段 / 任务：Phase 1 — 桌面基础与本地持久化
实施摘要：实现 rusqlite 数据库/迁移/事务/健康检查，五个 Tauri IPC 命令，React Router + Zustand + TanStack Query + shadcn/ui 桌面外壳，主题/错误边界/加载空态；完成真实跨进程写入/重启/读取验证。
修改文件：见第 3 节；新增 xushu-core DB/迁移/测试、Tauri commands/state/error、前端 app/components/features/pages/stores、ADR-0002、PHASE_1_REPORT；更新配置、锁文件、README、DEPENDENCIES。
实际执行的命令与退出码：见第 7 节；10 条质量门禁最终全部为 0。
测试：PASS — 前端 4 文件 13 测试；Rust 1 lib + 12 集成测试；冻结安装、typecheck、lint、build、format、cargo check --locked、fmt、clippy、cargo test、tauri info；跨进程 UIA 比对 ID/标签/内容/时间全部通过。NOT_RUN — tauri build 安装包、生产 CSP 成品、macOS。
安全/隐私检查：未读取/输出密钥；未修改全局配置；未执行管理员安装；未安装 Hooks；未使用子 Agent；未截图；capabilities 仅 core:default；错误不泄漏路径。
明确未完成或尚未验证：安装包、生产 CSP 实机、macOS、Phase 2 业务功能、正式跨平台 E2E。
下一步：等待用户审查 Phase 1；确认后再规划 Phase 2，不自动进入。
```

## 14. 结论

Phase 1 已于 2026-11-06 实际完成并通过验收。应用能够启动、写入真实 SQLite、完全退出后
重新启动并读回原始记录；全部可用质量门禁均真实执行且退出码为 0。没有把内存事件、
未运行命令或配置检查伪装成测试通过。

**停止于 Phase 1，等待用户确认是否进入 Phase 2。**