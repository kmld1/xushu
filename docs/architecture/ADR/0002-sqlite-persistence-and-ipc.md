# ADR-0002：Phase 1 本地 SQLite 持久化与受控 IPC

- 状态：Accepted
- 日期：2026-11-06
- 决策者：项目负责人（用户）与主开发助手
- 相关文档：`MASTER_DEVELOPMENT_PLAN.md`、`docs/architecture/DEPENDENCIES.md`、`docs/phases/PHASE_1_REPORT.md`
- 上游决策：ADR-0001（Tauri 2 + React + Rust/SQLite 本地优先架构）

## 背景

Phase 1 需要证明 XUSHU 不只是静态脚手架：桌面应用必须拥有真实的本地 SQLite
持久化、Rust 业务边界、受限 Tauri IPC 与基础 React UI，并能在完全退出后重新
读取同一份数据。

Phase 0 只建立了 Tauri/React 最小工程；当时没有数据库、业务命令、路由或产品页面。
Phase 1 不实现 Phase 2 的项目/任务/验收业务模型，也不接入 Claude Code Hooks。

## 决策

### 1. 数据库位置与配置

- 数据库文件为 Tauri `app_data_dir()` 下的 `xushu.db`，Windows 实测位于
  `%APPDATA%\dev.xushu.desktop\xushu.db`。
- 数据库文件、WAL/SHM 文件不进入 Git 仓库；`.gitignore` 已覆盖 `*.db`、
  `*.db-wal`、`*.db-shm`。
- 打开连接时设置：
  - `PRAGMA foreign_keys = ON`
  - `PRAGMA journal_mode = WAL`（若无法启用则显式失败，不静默降级）
  - `PRAGMA synchronous = NORMAL`
  - `busy_timeout = 5000ms`
- 使用 `rusqlite 0.41.1` 的 `bundled` SQLite，避免依赖系统 SQLite 版本。

### 2. Rust 数据访问边界

- 核心库 `crates/xushu-core` 持有 `Database`，内部为 `Mutex<rusqlite::Connection>`。
  单用户桌面应用不需要连接池；串行化可避免把非 `Sync` 连接暴露给多线程运行时。
- `Database` 提供打开/迁移/健康检查/事务 API，IPC 命令不直接操作 `Connection`，
  前端也不接收 SQL、数据库路径或连接字符串。
- 事务通过 `with_transaction` 表达：闭包返回错误时不提交，从而由 SQLite 回滚。
- 健康检查执行 `PRAGMA integrity_check`、`PRAGMA foreign_key_check`，并读取
  `foreign_keys`、`journal_mode`、schema 版本与记录数。

### 3. 迁移机制

- 使用 `schema_migrations(version, name, applied_at)` 记录已应用迁移。
- 每个迁移在独立事务中执行 SQL、写入版本记录并更新 `PRAGMA user_version`；
  任一语句失败则回滚整个迁移。
- 迁移按版本排序并拒绝重复版本；已应用迁移重复执行时跳过（幂等）。
- 如果数据库已有版本高于当前应用支持的最大版本，返回 `SchemaTooNew`，禁止
  旧应用对更新数据库静默降级。
- 迁移只追加，不修改已发布迁移。Phase 1 只有迁移 1：
  `0001_create_persistence_probes.sql`。

### 4. Phase 1 数据表范围

- 只创建 `persistence_probes(id, label, content, created_at)` 及其索引。
- 该表是明确的 Phase 1 持久化探针，不是 Phase 2 的任务、项目、验收条件或证据表。
- 输入由 Rust 再次校验：标签 1–64 字符、内容 1–2000 字符、拒绝控制字符；
  ID 必须是 UUID；列表上限 1–100。
- ID 使用 UUID v4，时间使用 UTC RFC3339（毫秒、`Z` 后缀）。

### 5. Tauri IPC 命令白名单

只注册以下五个命令：

| 命令 | 作用 | Rust 侧校验 |
|---|---|---|
| `get_app_info` | 产品名、应用/Tauri 版本、阶段、平台 | 不接收参数 |
| `get_database_health` | SQLite 健康、schema、WAL、外键、记录数 | 只读 |
| `create_probe_record` | 写入一条测试记录 | 标签/内容/控制字符 |
| `list_probe_records` | 读取最近记录 | limit 1–100 |
| `get_probe_record` | 按 ID 读取 | UUID 格式 |

- 不安装 Shell、文件系统、SQL 插件；capabilities 仍只有 `core:default`。
- 命令错误返回 `{ code, message }`；内部数据库错误、SQL 与绝对路径不返回前端。
- 数据库初始化失败时窗口仍可启动，命令返回 `DATABASE_UNAVAILABLE`，UI 展示错误态。

### 6. 前端状态与路由

- 使用 `HashRouter`，避免 Tauri 自定义协议下的深层路径刷新问题。
- TanStack Query 管理 IPC 异步状态与失效；Zustand 只保存主题模式（localStorage
  持久化）；React Hook Form + Zod 做表单体验校验，Rust 仍是最终校验边界。
- 使用 shadcn/ui 结构（`components/ui`、`lib/utils`、CSS 变量）与 Tailwind 4；
  组件不引入远程字体或外部资源。
- 全局 Error Boundary 捕获渲染异常；健康/记录查询分别展示加载、错误、空状态。
- UI 只展示真实查询结果，不展示虚构的项目、任务或 KPI。

### 7. CSP 策略

- 生产 `csp`：仅允许 `self`、Tauri IPC 与必要的数据/样式来源；
  `script-src 'self'`，不使用远程脚本源。
- 开发 `devCsp`：额外允许 Vite HMR 的 `ws://localhost:1420/1421` 与
  内联 React Refresh 脚本；仅用于开发。
- `style-src` 保留 `'unsafe-inline'`，因为 shadcn/Radix 与主题切换使用内联样式；
  这不放宽脚本执行权限。
- 生产 CSP 尚未通过 `tauri build` 的成品运行验证，必须在后续打包阶段补测。

## 备选方案与取舍

| 备选 | 未采用原因 |
|---|---|
| 前端直接执行 SQL | 绕开 Rust 状态机/校验边界；无法接受。 |
| IPC 返回数据库绝对路径 | 增加隐私泄漏面，UI 不需要该信息。 |
| 每条命令新建连接 / 连接池 | 单用户桌面应用无此并发需求，增加复杂度。 |
| JSON 文件或 localStorage 持久化 | 不能满足事务、外键、WAL、迁移与完整性检查要求。 |
| 提前创建任务/项目表 | 会越过 Phase 1 边界并固化未验证的 Phase 2 模型。 |
| BrowserRouter | Tauri 自定义协议下深层刷新兼容性更差。 |
| 为前端开放 Shell/FS 插件 | 违反最小权限；Phase 1 不需要。 |

## 后果

**正面：**

- 从 UI 到 Rust 到 SQLite 的真实链路已建立，且能在进程重启后读回同一记录。
- 数据库 schema 演进有事务化、可测试的迁移机制。
- IPC 白名单与 capabilities 最小化，错误信息不泄漏路径或 SQL。
- 前端具备可复用的状态、主题、错误和空态骨架。

**负面 / 风险：**

- `Mutex<Connection>` 串行化写入；未来若出现长时间查询，需要评估异步/分离连接。
- 迁移失败会阻止应用打开数据库；需要保持清晰的启动错误提示与备份策略。
- E2E 验证依赖 Windows UI Automation（开发测试用），不是跨平台自动化测试。
- 生产 CSP 与安装包尚未实机验证；Phase 1 的 PASS 不覆盖打包产物。
- Phase 1 测试表未来可能需要迁移或删除；删除必须通过显式迁移，不能手工改历史。

## 回滚

若未来替换存储层，应保留 `xushu-core` 的服务接口，通过新 ADR 记录迁移与数据兼容
方案；不得让前端直接访问新的存储实现，也不得删除用户已有数据库而不提供导出/
备份路径。

## 参考

- <https://www.sqlite.org/wal.html>
- <https://www.sqlite.org/lang_transaction.html>
- <https://www.sqlite.org/pragma.html>
- <https://v2.tauri.app/security/capabilities/>
- <https://v2.tauri.app/security/csp/>
- <https://ui.shadcn.com/docs/tailwind-v4>