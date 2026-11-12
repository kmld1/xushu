
# 序枢 XUSHU · 总开发计划（Master Development Plan）

> 文档版本：v1.0（2026-11-05）  
> 项目根目录（Windows）：`D:\MyProject\xushu`  
> 项目名称：序枢 XUSHU（工作名称，商标/域名未核验）  
> 状态：方案已规划，尚未宣称创建工程或完成任何功能  
> 开发方式：单人 + Claude Code（经用户自行配置的 DeepSeek API）；不使用多 Agent；不自动截图。

---

## 0. 执行摘要：我们到底要做什么

**一句话定位：** 面向个人开发者的、本地优先的 AI 工作控制系统。它不替代 Claude Code、编辑器和终端，而是让 AI 参与的开发工作形成**目标 → 执行记录 → 独立验证 → 事实/决策留存 → 断点恢复**的可信闭环。

**初始用户：** 使用 Claude Code、Git 和本地开发项目的独立开发者。  
**初始平台：** Windows 11 x64（开发与验收优先），架构兼容 macOS arm64。macOS 真机测试、签名和发布不是 Windows 首轮通过的前提。  
**最小完整 V1：** 从创建/导入本地 Git 项目开始，定义任务及验收标准；记录经过用户授权的 AI 执行事件；由 XUSHU 独立运行用户显式授权的验证命令并存证；根据证据判定任务状态；关闭程序后仍能恢复工作。

### 0.1 成功指标（产品假设，待真实使用验证）

- 典型任务可在 XUSHU 中看到目标、最新执行、验收状态、证据链接与下一步。
- AI 声称“完成”但验证命令失败时，产品必须明确显示“未通过”，不得被 AI 文本覆盖。
- 完全退出后重新打开应用，项目/任务/决策/验证报告保持一致。
- Hook 关闭、丢事件、Claude Code 不存在时，核心手工任务与验证流程仍可用。
- 本地敏感原文默认不上传、不采集、不展示；无云账户亦可使用。

### 0.2 核心原则

1. **以证据为准。** `AI 声明`、`Hook 观察`、`真实命令结果`、`人工确认`严格区分。
2. **本地优先。** V1 不依赖远程服务；不内置云同步、登录系统、付费网关。
3. **最小权限。** 不静默修改 `~/.claude`、环境变量、Shell Profile、Git 配置。
4. **单代理执行。** 用户只通过一个 Claude Code 会话开发；禁止子 Agent、多 Agent 编排。
5. **不自动截图。** UI 检查允许人工截图；测试中不启用失败自动截图/录屏。
6. **小步可验收。** 完成一个阶段再进入下一阶段；失败/阻塞必须记录。
7. **不伪造进度。** 缺工具、未跑测试、未实机验证均必须如实标记。
8. **避免为架构而架构。** 不采用微服务、消息队列、Kubernetes、云后端或提前拆分大型插件体系。

---

## 1. 功能范围

### 1.1 V1（必须交付）

| ID | 模块 | 必须完成的能力 | 可验证产物 |
|---|---|---|---|
| F01 | 项目工作台 | 导入已存在的本地 Git 项目，显示路径、分支、最近快照；支持多项目切换 | 导入/关闭/重开测试 |
| F02 | 任务与验收 | 新建任务、优先级、依赖（仅提醒）、验收条件与状态转换 | 状态机单测、UI 测试 |
| F03 | 事实事件流 | 本地写入结构化事件（会话开始/结束、工具使用元数据、人工备注） | 存储与幂等测试 |
| F04 | Claude Code 接入 | **用户主动启用**项目级 Hooks；原始内容默认不采集；可关闭并恢复原配置 | 安装/卸载/兼容测试 |
| F05 | 独立验收 | 用户批准后执行明确配置的测试命令（非 AI 自动任意命令），记录退出码、耗时和脱敏输出 | 成功/失败/超时/取消测试 |
| F06 | 证据报告 | 每条验收条件对应检查、来源、时间、版本与结果；区分未验证/通过/失败/跳过 | 测试失败时不能判定完成 |
| F07 | 决策与记忆 | 追加式决策记录、变更原因、任务摘要；保留来源引用 | 重启仍可追溯 |
| F08 | 断点恢复 | 生成当前项目恢复卡：最近动作、遗留问题、待验证项、下一步；导出可供 AI 阅读的 Markdown | 重启/切项目后可恢复 |
| F09 | 安全与维护 | 本地数据库迁移、备份/恢复、数据导出、异常恢复、日志脱敏 | 迁移/备份/恢复测试 |
| F10 | 桌面产品化 | Windows 可运行安装包，设置界面、基本空状态/错误态、快捷键和主题 | 实机启动/卸载检查 |

**必要限定：** 事件流仅表示“观察到某操作”，不能代替测试、代码审计或用户验收；外部 Hook 可能漏报或被绕过。任务完成必须满足配置的验收门禁。

### 1.2 非目标（V1 明确不做）

- 自动操控 Claude Code、自动修复代码、无人值守运行任意终端命令。
- 完整解析/采集所有对话、API Key、Token、`.env`、SSH 私钥、私有源码。
- 自动多 Agent 协作；跨设备实时云同步；团队协作；账号/权限 SaaS。
- 微信自动登录/群聊抓取、智能家居、模型微调全流程管理。
- 内置 IDE、代码托管平台、训练平台、通用 Agent 市场。
- “代码绝对正确”“防篡改审计”“所有 Claude Code 操作必然完整捕捉”等无法保证的承诺。

### 1.3 后续扩展顺序（V1 通过后再评估）

1. 环境医生：只读体检 → 明示风险 → 用户批准修复。
2. 跨机器导入导出：先加密离线迁移，后按需求评估同步。
3. 项目代码学习教练：从已授权的代码抽取练习，测量个人掌握情况。
4. 扩展 AI 工具适配器：标准化事件契约，不绑定单一供应商。
5. 外部任务入口：邮件/群聊/日程仅使用合规、授权的接入能力。
6. 本地模型实验看板：数据/评测/模型版本资产可追踪。

---

## 2. 冻结的技术路线（V1）

| 位置 | 技术 | 选择原因与约束 |
|---|---|---|
| 桌面壳 | **Tauri 2.x** | Windows/macOS 桌面分发；权限细粒度配置 |
| 前端 | **React + TypeScript（strict）+ Vite** | 清晰组件模型与快速本地开发 |
| UI | **Tailwind CSS + shadcn/ui + Lucide** | 可访问、可维护的统一设计系统；不依赖重型组件库 |
| 路由 | **React Router** | 项目/任务/证据/设置页路由 |
| 请求/缓存 | **TanStack Query** | 仅管理 UI↔Tauri 命令异步状态；无需 HTTP 服务 |
| 轻量状态 | **Zustand** | 当前选中项目、筛选器、UI 状态；不复制服务端业务事实 |
| 表单与校验 | **React Hook Form + Zod** | 输入验证；Rust 后端再次执行可信校验 |
| 后端业务 | **Rust（Tauri Commands + 领域服务）** | 文件访问、SQLite 写入、验证命令、权限检查都在 Rust 侧 |
| 本地数据库 | **SQLite + rusqlite + 版本化迁移** | 离线、单机、事务一致性与可备份性 |
| Git 读取 | Rust 受限调用 `git` CLI（参数数组，无 shell 拼接） | Git 版本状态；只读操作优先 |
| AI 事件接入 | Claude Code Hooks + 本地 `xushu-hook` Rust 小程序 | 仅明确授权的项目级事件捕获；失败不阻塞正常开发 |
| 日志 | `tracing`（Rust）+ 脱敏日志 | 错误定位，默认不包含敏感原文 |
| 前端测试 | Vitest + React Testing Library | 状态与组件单测 |
| Rust 测试 | `cargo test` + 临时 SQLite | 状态机、数据库、权限、运行器与导入测试 |
| 端到端 | 先手工验收；可加受控自动化 | 不自动截图/录屏；不以“有 UI”代替真正验证 |
| 格式与质量 | ESLint + Prettier + `tsc` + `cargo fmt` + Clippy | 格式、类型、静态检查与回归验证 |
| 包管理 | **pnpm**（前端），**Cargo**（Rust） | 使用锁文件固定解析版本 |

**版本策略：** 固定 Tauri **v2 主版本**，按开发当日官方兼容性选稳定补丁版；`package.json` 明确 `packageManager`，提交 `pnpm-lock.yaml` 与 `Cargo.lock`。不盲目使用 `latest` 或复制可能过期的版本号。初始化时先查官方文档与依赖兼容矩阵，记录实际版本到 `docs/architecture/DEPENDENCIES.md`。

**为什么不把 SQLite 直接暴露给前端：** 所有数据库读写必须经过 Rust 领域服务及 Tauri 命令，防止前端绕开任务状态机、验收门禁与路径授权；首版不需要给 WebView 通用 SQL 执行权限。

**为什么不在 V1 内置独立服务器：** 所有核心功能都可以通过桌面端 IPC 与本地数据库完成，不需要本地 HTTP Server、Docker 或云服务。

### 2.1 Windows 先决条件

在**只读检查**后确认：Node.js（建议当前官方 LTS，至少满足依赖要求）、pnpm、Rust stable MSVC、Cargo、Git、Microsoft C++ Build Tools（C++ 桌面开发工作负载）、WebView2。Windows 上还要注意 `D:` 盘文件权限、PowerShell 路径和带空格路径。

> 若缺 Rust/MSVC/WebView2，Claude Code 应输出准确安装说明；未经授权不得以管理员身份安装软件或修改全局配置。Tauri Windows 先决条件见 https://v2.tauri.app/start/prerequisites/ 。

### 2.2 macOS 后续适配

Rust stable、Node/pnpm、Git、Xcode Command Line Tools；使用相同的数据库迁移和领域测试。路径、可执行文件后缀、进程终止、文件权限需平台抽象；不得将 `D:\` 硬编码到应用逻辑。Windows 的构建通过不代表 macOS 真机已验证。

---

## 3. 建议仓库布局

```text
D:\MyProject\xushu\
├── CLAUDE.md                        # 项目协作与执行规则
├── MASTER_DEVELOPMENT_PLAN.md       # 本文件：全局需求与开发路线
├── README.md
├── .gitignore
├── package.json                    # 前端 workspace 工具链
├── pnpm-workspace.yaml
├── pnpm-lock.yaml
├── Cargo.toml                      # Rust workspace
├── Cargo.lock
├── apps/
│   └── desktop/
│       ├── src/                    # React 应用、页面、组件
│       ├── src-tauri/              # Tauri Rust 命令及桌面能力
│       ├── index.html
│       └── package.json
├── crates/
│   ├── xushu-core/                 # 领域模型、状态机、仓储与验证服务
│   └── xushu-hook/                 # 可选的原生 Hook 事件收集 CLI
├── packages/
│   └── ui/                         # 共享 UI；仅存在实际复用需要时创建
├── docs/
│   ├── architecture/
│   │   ├── ADR/                    # 不可覆盖的架构决策
│   │   ├── DATA_MODEL.md
│   │   ├── SECURITY.md
│   │   └── DEPENDENCIES.md
│   ├── phases/                     # 每阶段任务与验收报告
│   └── testing/
├── scripts/                         # 幂等脚本，必须有风险说明
└── tests/                           # 受控集成测试样例与 fixtures
```

**约束：** 不为填满目录而创建空包；在实际需要时才增加 `packages/ui`。启动前检查目录现状，任何已有文件保留，不粗暴覆盖。

---

## 4. 系统边界与架构

```text
用户
  │
  ▼
React UI（展示/表单/授权入口）
  │ invoke（强类型参数；前端 Zod 验证仅用于体验）
  ▼
Tauri Commands（Rust 可信边界、权限与参数校验）
  │
  ├── Project/Task Service ── SQLite（迁移、事务、备份）
  ├── Verification Service ── 受批准命令运行器（无 shell 拼接）
  ├── Git Inspector ──────── git CLI（只读检查）
  ├── Event Inbox ───────── 本地 Hook Spool 导入与去重
  ├── Decision Journal ──── 追加式记录、来源引用
  └── Recovery Builder ──── 基于已存证事实生成 Markdown

Claude Code 项目级 Hooks（需用户显式允许）
  │ JSON/stdin（受限字段、脱敏、大小上限）
  ▼
xushu-hook 本地 CLI
  │ 原子文件写入 / 最小化本地待导入事件
  ▼
受控 Event Inbox（应用启动/主动刷新时导入）
```

- **不默认驻留后台。** 桌面程序关闭后，Hook CLI 可以将最小事件写入本地待导入目录，下一次打开导入。CLI 采集失败不得阻断 Claude Code。
- **事件与业务事实分离。** Hook 只是来源之一；人工操作、Git 快照、验证执行也能产生事件。
- **无神奇自动关联。** 通过用户选择的项目、受信根目录、会话 ID、任务 ID 显式关联；无法识别时进入“未关联事件”，不强行挂到错误任务。
- **跨平台路径。** 后端以 canonical path 判断受信根目录，拒绝目录穿越和符号链接越界；保留原始展示路径。

---

## 5. 业务模型与数据契约

### 5.1 核心实体（最小集合）

- `projects`：`id, name, root_path, git_remote_optional, created_at, updated_at`。
- `tasks`：`id, project_id, title, description, priority, status, created_at, updated_at, revision`。
- `acceptance_criteria`：`id, task_id, description, kind(auto|manual), required, verification_spec_optional`。
- `sessions`：`id, project_id, external_session_hash_optional, started_at, ended_at_optional, source`。
- `events`：`id, project_id, task_id_optional, session_id_optional, source, event_type, payload_redacted, occurred_at, ingested_at, source_dedupe_key`。
- `verification_runs`：`id, task_id, approved_by_user_at, executable, argv_json, cwd, started_at, finished_at, outcome, exit_code_optional, git_head_optional, timed_out`。
- `verification_checks`：`id, run_id, criterion_id, outcome(pass|fail|not_run|inconclusive), evidence_id_optional`。
- `evidence`：`id, project_id, task_id_optional, kind, source_ref, summary, content_digest_optional, captured_at, sensitive_flag`。
- `decisions`：`id, project_id, task_id_optional, decision, rationale, source_ref_optional, created_at, supersedes_id_optional`。
- `recovery_snapshots`：`id, project_id, created_at, status_digest, markdown_body, based_on_revision`。
- `app_migrations` / migration 元数据：由迁移框架托管；不手改历史迁移。

全部使用统一 ID（UUID），时间为 UTC RFC3339，UI 按本地时区渲染。数据库开启外键与适用的 WAL 模式，所有跨表写入使用事务。敏感输出可按策略仅保存摘要、哈希或裁剪后的脱敏片段。

### 5.2 任务状态机

`draft → ready → running → awaiting_verification → done`，任何进行中状态都可进入 `blocked`；`cancelled` 为终止态，重新开启须显式操作并留下事件。

**强制门禁：**

- `done` 仅在所有**必需**验收条件有当前任务修订版对应的有效证据且通过后允许；手工验收条件须人工确认并标记来源。
- 验证结果为 `fail`、`not_run`、`inconclusive` 时不得因为 AI 回复“已完成”而变 `done`。
- 改动任务要求/验收条件/受影响源码之后，旧验证结果需标为 `stale`（过期），不得继续作为自动完成依据。
- 允许“明确放弃/豁免”但单独记录，不能伪装成 `pass`；有必需项被豁免时报告必须显示偏离，不得称“全部通过”。
- `blocked` 必须包含阻塞原因和下一步建议；不自动创造未执行的测试结果。

### 5.3 证据可信等级

| 来源 | 可以证明什么 | 不可以证明什么 |
|---|---|---|
| 用户批准的真实验证运行 | 特定代码/环境/命令在特定时间得到的退出码与输出 | 整个软件绝无缺陷 |
| Git 快照 | 某个时间点的 HEAD、工作区状态、变更文件 | 代码语义正确 |
| Claude Code Hook 元数据 | 某事件曾由可用的 Hook 通路上报 | 全部操作都已记录、执行结果一定正确 |
| 人工确认 | 用户在特定时间确认了某个检查项 | 机器测试已经跑过 |
| AI 文本声明 | AI 声称做了什么 | 实际通过验收 |

### 5.4 独立验证命令规范

- 每个项目的验证配置由用户建立/确认；建议采用受限模板（例如已识别的 `pnpm test`、`cargo test`），**不能把 AI 产生的任意字符串直接送进 shell**。
- Rust 使用**可执行文件 + 参数数组 + 显式工作目录**运行；首版禁止默认 `cmd /c`、`powershell -Command`、`sh -c` 和任意 Shell 字符串。任何必要例外单独审批与审计。
- 显式提示：测试脚本本身可能执行任意代码；执行前确认 Git 项目可信、展示命令/目录/潜在风险。
- 必须支持超时、手动取消、输出大小上限、stdout/stderr 脱敏、退出码记录，防止 UI 卡死。
- 结果关联 `task revision`、运行时 Git HEAD 和工作区状态；有未提交改动时需注明快照不完整，不得仅用 HEAD 断言源代码相同。

---

## 6. Claude Code Hooks 接入规范

1. **用户主动点击启用。** 只修改用户选定项目的 `.claude/settings.local.json`（或由当时官方文档确认的合适项目级设置位置），不修改全局 `~/.claude/settings.json`；若配置存在，先备份，结构化合并，展示 diff。
2. **初始事件范围：** `SessionStart`、`PostToolUse`、`PostToolUseFailure`、`Stop`、`SessionEnd`；实际可用事件与字段必须以当前官方 Hooks 文档及实测为准。
3. **最小采集白名单：** 事件类型、去标识化会话 ID、项目标识、工具名称、时间、可选受限文件路径元数据；默认**不采集** prompt 原文、工具完整输入/输出、transcript 内容、凭据、环境变量、源码或网络正文。
4. **Hook 输入不可信。** 严格 JSON Schema/结构验证、大小限制、速率限制、拒绝异常路径；不能把事件 payload 当命令执行。
5. **原子写入。** Hook CLI 将单事件文件写入用户 AppData 对应 XUSHU inbox 的临时文件，写完原子重命名；应用导入、去重，成功后归档或安全移除。事件 ID 可由来源稳定字段加散列构成，避免重复导入。
6. **卸载可恢复。** 只删除自身准确匹配的 Hook 配置片段，不触碰用户别的 Hooks；冲突时输出人工复核说明。
7. **失败不阻塞。** Hook CLI 不向 stdout 注入误导性内容；异常时记录有限本地诊断，尽可能非阻塞退出。
8. **开发包与发布包分离。** 开发阶段 CLI 可直接从 Rust workspace 运行；正式发布需随安装包提供平台对应可执行文件并验证路径，**不得要求最终用户下载整个源代码**。

参考文档：https://code.claude.com/docs/en/hooks 。

---

## 7. 安全、隐私与数据恢复

- 应用自身不存储 DeepSeek/Claude API Key；不读取已有 AI 工具密钥或令牌。V1 无 AI 接口也可运行。
- 使用 Tauri 2 的最小 capabilities；前端不授予通用 shell、通用文件系统写入或原始 SQLite 执行权限。
- 用户仅能选择明确项目根目录；后端处理路径归一化、符号链接越界、拒绝受保护目录的非法访问。
- `xushu-hook` 严禁读取 transcript 文件内容；收到字段默认丢弃，仅允许结构化白名单字段。
- 敏感输出显示前做脱敏；绝不把“正则脱敏”描述为百分之百防泄漏。提供“仅存元数据”和“输出不落盘”选项。
- 执行验收命令前呈现确认卡（程序、参数、cwd、超时）；未批准不执行。
- 应用操作记录为追加式审计日志；普通用户本地文件可以被修改，所以**不得声称防篡改**。
- SQLite 备份使用一致性备份机制（SQLite Backup API 或已验证的 checkpoint/事务流程），不能在线仅复制主 `.db` 而忽略 WAL；做恢复演练。
- 迁移必须可测试、失败回滚、记录 schema 版本；不会对已存在数据做未经用户授权的破坏性修改。
- `.gitignore` 排除运行时数据库、备份、secret、日志、产物和安装包。导出前明确告知可能包含路径/元数据。
- 任何依赖下载优先可靠官方源；可兼顾国内可达性，但镜像不得牺牲来源可核实、版本正确和完整性。

---

## 8. UI/UX 信息架构

**一级页面（V1）：** `工作台`、`项目`、`任务详情`、`验收中心`、`事件/决策`、`断点恢复`、`设置`。

- 工作台：活跃项目、待验收任务、阻塞事项、最近工作（绝不展示虚假真实数据）。
- 项目：项目根路径、Git 状态、任务分组、最近事件；异常路径/非 Git 仓库可解释。
- 任务详情：目标、验收条件、执行事件、证据时间线、状态变更；显示证据不足。
- 验收中心：即将运行的命令预览、风险提示、用户批准、实时状态、结果与证据导出。
- 断点恢复：可复制的 Markdown 上下文卡，明确区分已证实事实和待确认事项。
- 设置：隐私开关、Hook 开关、备份/恢复、日志导出、数据目录、关于。

**视觉规范：** 专业简洁、中文优先、清晰层级，深浅双主题，键盘可操作，清楚区分 `通过/失败/未验证/过期`；错误态、加载态和空状态齐全。不可仅靠颜色表达状态。不得使用硬编码假 KPI 伪装真实进度。

---

## 9. 阶段开发总路线（严格顺序，禁止跳阶段报完成）

### Phase 0 — 仓库体检与工程启动

**工作：** 只读检查 `D:\MyProject\xushu`、Git 状态、Node/pnpm、Rust/Cargo、MSVC、WebView2、当前文档；辨认已有文件；记录可用环境。初始化 monorepo 基础文档与最小脚手架（只有在不覆盖的前提下）。

**验收门禁：** 架构决策文件存在；仓库结构正确；安装依赖可复现；`pnpm install --frozen-lockfile`、`cargo check`、前端类型检查在有完整工具链时通过；缺工具明确列阻塞，不编造 PASS。

### Phase 1 — 桌面基础与本地持久化

**工作：** Tauri 2 + React/Vite 桌面可运行；主题、路由、错误边界、Rust IPC；SQLite 初始化、版本化迁移、事务测试；基础 UI 组件。

**验收门禁：** Windows 桌面窗口成功启动；能写/读测试记录；退出重启保留数据；迁移失败能正确报告；无全盘写权限。

### Phase 2 — 项目、任务与状态机

**工作：** 选择本地项目路径、Git 只读检测、任务增删改查、验收条件、状态流转、项目看板。

**验收门禁：** 单测覆盖合法/非法状态转换；项目路径验证通过；未满足必要条件无法完成；重启可恢复项目/任务。

### Phase 3 — 独立验收引擎（**首个产品价值里程碑**）

**工作：** 用户定义并批准验证模板；安全命令执行；失败/通过/超时/取消/输出脱敏；结果关联代码状态和任务验收条件；生成报告。

**验收门禁：** 故意失败的测试必须显示失败；未运行必须显示未验证；过期证据不可用于当前任务完成；运行器能取消且无遗留运行中的任务假象。

### Phase 4 — Claude Code Hook 适配

**工作：** Rust `xushu-hook` CLI，项目级 Hook 用户授权安装、无损卸载、事件投递、离线导入、去重、会话关联、缺失事件提示。

**验收门禁：** 在 Windows Claude Code 中真实触发至少开始/工具/结束类可用事件并验证入库；重复投递不重复记账；关闭/卸载后无新采集；Hook 出错不阻断 Claude Code。

> **Phase 4 实施状态（2026-11-12）**。Windows 六种真实 Hooks 联调、已测功能与当前 E2E 隔离通过。整体验收 **PENDING**。 [Phase 4.3 安全可靠性收尾](docs/phases/PHASE_4_3_SECURITY_CLOSEOUT.md)
>
> - 已完成：`xushu-hook` CLI + 最小事件协议、项目级 `.claude/settings.local.json` 授权安装/停用/卸载、备份与原子写入、配置异常/重定向/外部修改检测、离线 spool 导入、并发/重复/缺失事件处理、迁移 4 数据模型、受限 IPC、项目详情接入与活动 UI。
> - 已验证：Rust workspace 110 项、前端 54 项、真实 Claude Code 六事件入库与去重、正常停用和卸载、真实数据 UI 交互、E2E 正常/失败/异常退出隔离。
> - Phase 4.3 回归覆盖故障下撤销采集、项目根目录重定向、外部设置编辑和迟到事件。新构建的真实 Claude Code 会话及人工视觉仍待手动复验。
> - 未完成：生产安装包 sidecar 打包与升级路径验证（Phase 7）；macOS 与旧版 Claude Code 未验证；人工视觉复核仍为 `VISUAL_REVIEW_PENDING`。

### Phase 5 — 决策记忆与断点恢复

**工作：** 追加式决策、事实摘要（优先确定性模板）、恢复卡（Markdown 导出）、任务状态/阻塞/证据引用汇总；重启恢复。

**验收门禁：** 关闭全部程序再打开，恢复卡事实与数据库一致；失败不能变成功；决策变更保留旧记录及原因；无法解析的事件不伪装成事实。

### Phase 6 — 产品完整性与安全加固

**工作：** Hook 配置冲突处理、敏感字段回归测试、数据库一致性备份/恢复、异常状态恢复、Windows 路径边界、键盘/空态/错误态、用户文档。

**验收门禁：** 完成安全测试矩阵；备份恢复真实演练；非法路径与未经批准执行被拒绝；日志无敏感原文；重大故障无无声失败。

### Phase 7 — V1 实机验收与分发

**工作：** Windows 全流程实机用例、打包安装器、干净环境运行验证、卸载影响说明、release notes、已知限制；macOS 编译可行性记录（若有真机再实测）。

**验收门禁：** `创建项目 → 任务 → 验收 → 失败/成功报告 → 决策 → 重启恢复` 全链路通过；安装包实际运行；所有测试命令与结果可复核；没有把未进行的 macOS 测试写为通过。

> **阶段原则：** Phase 0/1 可先纯工程初始化；Phase 3 必须尽早产生真实产品价值。Phase 4 Hooks 属于可选事件来源，不能成为阶段 3 独立验收的前提。每次只实现当前阶段下一个最小闭环任务。

---

## 10. V1 端到端用户故事与验收样例

### US-01 导入项目

用户从界面选择 Git 项目 → 程序在已授权范围内只读检查 Git → 记录项目 → 重启仍存在。非 Git 目录给出明确错误；不自动初始化 Git。

### US-02 创建任务

用户创建“修复 SQLite 嵌套事务问题”，设置两条自动验收条件与一条人工条件 → 任务尚未通过时只能显示准备中/待验收。

### US-03 AI 声称完成但测试失败

用户让 Claude Code 修改项目代码 → Hook 上报可能的操作事件 → 用户主动运行验证命令 → 某测试退出非零 → 报告记录失败、stdout/stderr 摘要、退出码、耗时与代码状态 → 任务不能进入 `done`。

### US-04 测试成功但不能过度断言

两条自动条件测试通过，人工条件未确认 → 报告显示“自动检查通过、人工待确认”，不得显示“全部完成”。人工确认后才能变 `done`。

### US-05 证据过期

验证通过后修改验收要求或相关工作区内容 → 旧证据过期提示 → 需重新验收，状态随规则更新。

### US-06 恢复任务

关闭应用和 Claude Code → 再启动 XUSHU → 查看最后有效验收、未解决问题、决策与建议下一步 → 复制恢复 Markdown 到新会话；没有历史记录时明示“不足以推断”。

### US-07 采集隐私

Hook 输入包含任意敏感字段、长内容、异常路径 → 白名单过滤后只落允许的元数据 → 删除 Hook 后不再采集 → 核查配置未损坏原有其他 Hooks。

### US-08 确认与命令安全

未受信项目、越界目录、未授权命令、超时或用户取消 → 后端阻断/停止并保留原因；不能仅通过前端按钮隐藏实现权限控制。

---

## 11. 质量门禁与交付纪律

**每次开发结束至少记录：**

- `本次目标`、`修改文件列表`、`实际执行的命令`、`输出/退出码`、`真实测试结果`、`尚未验证项`、`下一步`。
- 前端变更需按实际工程脚本执行：类型检查、Lint、相关 Vitest；Rust 变更执行 `cargo fmt --check`、`cargo clippy`、`cargo test`（或说明阻塞原因）。
- 不能只说“应该通过”；没有运行就是 `NOT_RUN`。不能用代码审查推断动态测试已通过。
- 每个阶段增加 `docs/phases/PHASE_N_REPORT.md`，包含具体证据、阻塞与可复现命令；禁止先写 PASS 再补证据。
- 新技术/依赖/架构变化先写 ADR，说明问题、选项、取舍、兼容性与回滚办法；不可随意改变已冻结栈。
- 不自动截图；必要 UI 视觉验证请用户手动截图，代码层可做无需截图的测试。

### 11.1 完成定义（DoD）

一个任务被称为完成，需同时满足：实现了定义的范围；必需自动检查真实通过；人工条件已经人工确认；相关风险与文档更新；证据链接可定位；没有遗留重大阻塞。若部分条件无法验证，标记“待验证/部分交付”，不得将其算作正式完成。

### 11.2 真实使用评估（商业验证）

V1 做好后，连续使用于至少两个真实 Git 项目，记录：

- AI 说完成但被验证系统识别出的失败数量（记录漏报与误报）。
- 任务恢复一次所需时间与人工重复整理量（上线前后同口径比较）。
- 安装/Hook 授权/命令确认带来的新增摩擦。
- 是否愿意每周持续打开，是否有 5–10 位外部开发者愿意试用。

**商业模式暂不预设。** 先证明对自己有用，再验证其他开发者的真实需求与愿付费程度；避免把产品愿景当作市场事实。

---

## 12. 风险清单与减缓措施

| 风险 | 后果 | 应对 |
|---|---|---|
| Hooks 事件缺失、字段版本变更 | 误判运行过程 | 适配层版本检测；明确缺失；永不作为唯一完成证据 |
| 用户运行恶意测试脚本 | 本地代码执行风险 | 项目受信确认、命令预览、严格授权、无隐式 shell |
| 本地日志泄露令牌 | 安全与隐私问题 | 白名单事件、输出限制与脱敏、默认少采集、可清除 |
| SQLite/WAL 备份不一致 | 数据损坏/丢失 | 使用 SQLite 一致性备份、完整性检查与恢复演练 |
| 多平台路径差异 | Windows 能用、macOS 失败 | 后端路径抽象；每平台真实打包/验证，不假装兼容 |
| V1 需求膨胀 | 久做不完 | 冻结 F01–F10，严格延期扩展模块 |
| AI 报告虚假 PASS | 错误发布/误判 | 自动化硬证据 + 不可绕过的状态门禁 |
| 本地 Git 状态不代表完整源码 | 证据失效 | HEAD、工作区 dirty 状态同时记录，变动后过期 |
| 核心能力过度依赖 DeepSeek | Token 成本与不可用 | 核心功能完全确定性实现，AI 摘要后续可选 |

---

## 13. 第一轮 Claude Code 开发任务（直接执行）

**目标：只做 Phase 0。** 不要一口气实现所有阶段，不要提前跑 Hook 注册，更不要自动修改用户已有 Claude/DeepSeek 配置。

1. 在 PowerShell / Claude Code 当前会话中确认 `D:\MyProject\xushu` 存在；不存在才创建；列出已有文件和 `git status`，记录是否有未提交改动。
2. 阅读本文件和项目根目录 `CLAUDE.md`；确认固定技术栈与 V1 范围。
3. 检查 Node/pnpm/Git/rustc/Cargo/MSVC/WebView2 条件，输出 `PASS / MISSING / UNVERIFIED`，不自动管理员安装。
4. 查阅 Tauri v2 / React / Rust 官方文档，确定互相兼容的稳定版本并锁定（记录来源）。
5. 在不覆盖现有用户文件的条件下，初始化根 `pnpm` + `Cargo` workspace、Tauri/React 最小桌面项目及必要 `.gitignore`、`README`、架构决策。
6. 对当前能运行的构建/类型/编译检查进行实测。若工具缺失，停在可复核的阻塞状态，给出最少人工步骤。
7. 输出 `docs/phases/PHASE_0_REPORT.md`：变更、命令、实际结果、阻塞、下一项计划。
8. **Phase 0 成功后停止，不自动进入 Phase 1。** 用户确认下一阶段再推进。

### 13.1 开发时可直接复用的首轮验收口径

- 目录未被清空、已有文件未丢失。
- 依赖及工具链版本真实列明、锁文件可复现。
- 从仓库根能清晰找到桌面端启动脚本及 Rust workspace。
- 有实际的类型/编译检查结果；缺任何先决条件不能谎报完成。
- 记录了项目本地运行需要的命令与必要前置条件。

---

## 14. 官方资料（核对基线）

- Tauri 2 桌面开发及 Windows 前置要求：https://v2.tauri.app/start/prerequisites/
- Tauri 2 capabilities 安全模型：https://v2.tauri.app/security/capabilities/
- Tauri 2 官方项目结构：https://v2.tauri.app/start/project-structure/
- Claude Code Hooks 生命周期和输入格式：https://code.claude.com/docs/en/hooks
- Rust 官方文档：https://doc.rust-lang.org/book/
- React 官方文档：https://react.dev/
- SQLite 事务/备份文档：https://www.sqlite.org/backup.html

> 以上是官方资料入口。执行前以当时可访问的官方文档和实际工具版本为准；如与本文有冲突，先记录 ADR，再按最小兼容调整，不擅自推翻产品范围。
