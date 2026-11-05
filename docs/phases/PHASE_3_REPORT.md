# Phase 3 报告：独立验收引擎

- 日期：2026-11-06
- 项目：序枢 XUSHU
- 根目录：`D:\MyProject\xushu`
- 总体状态：**PASS**
- 结论：XUSHU 已具备真实、可信、可追溯的独立验收闭环：自动验收条件 → 验证配置 → 用户预览与批准 → 后台执行 → 退出码/耗时/脱敏输出/代码状态证据 → PASS/FAIL/NOT_RUN/INCONCLUSIVE/STALE → Rust 完成门禁。自动条件不再永远阻塞，也不会被伪造为通过。
- UI 视觉状态：**VISUAL_REVIEW_PENDING**（自动化与桌面 E2E 通过，等待用户手动截图验收）。

## 1. 验证引擎架构

```text
React 验收中心 / 任务详情
  │ invoke（criterionId、期望修订号、approved=true；不提交任意命令）
  ▼
Tauri Commands（Rust 可信边界）
  │
  ├── VerificationService
  │     ├── 读取并校验 spec / task / criterion / project 边界
  │     ├── 写入 running 运行记录（approval、修订快照、代码指纹）
  │     ├── 后台线程执行 runner
  │     ├── finalize：脱敏、判定、证据、检查、运行状态
  │     └── 过期判定、队列、报告
  │
  ├── runner（std::process::Command，参数数组）
  │     ├── Windows Job Object（KILL_ON_JOB_CLOSE + TerminateJobObject）
  │     ├── 超时、取消、输出上限、管道排空
  │     └── 子进程树清理
  │
  ├── code_state（只读 Git HEAD/分支/status + 变更文件内容 SHA-256）
  │
  └── SQLite（迁移 3：specs / runs / checks / evidence）
```

关键设计：

- 验证在后台线程执行，`start_verification` 只做短事务准备并立即返回；不阻塞 Tauri UI。
- 前端不能在批准时提交任意命令；Rust 从数据库读取已保存的 `verification_specs`。
- 运行记录保存任务/条件/配置修订快照与开始/结束代码状态，结果无法跨版本复用。
- `STALE` 是查询时派生的证据有效性，不写入检查结果，不删除历史。
- 完成门禁在 `TaskService::transition_task_status` 中调用同一评估逻辑；前端只展示 blockers。

## 2. 数据库 Migration

迁移版本 3：`crates/xushu-core/migrations/0003_verification_engine.sql`，只追加，不修改迁移 1/2。

新增表：

- `verification_specs`
- `verification_runs`
- `evidence`
- `verification_checks`

关键约束：

- UUID 主键；UTC RFC3339；外键；事务。
- `verification_specs(criterion_id)` 唯一，每条自动条件最多一个配置。
- `verification_runs.task_id → tasks(id) ON DELETE RESTRICT`；`criterion_id → acceptance_criteria(id) ON DELETE SET NULL`；`spec_id → verification_specs(id) ON DELETE SET NULL`；运行同时保存条件描述与配置快照，删除条件不会丢历史。
- `verification_checks(run_id)` 唯一；`evidence(run_id)` 部分唯一索引。
- 执行状态、检查结果、成功规则、超时/输出上限均有 CHECK。
- 保留 Phase 1/2 全部表与数据。

实测数据库（Phase 2 已有数据 + 四轮 E2E 后）：

```json
{
  "integrity": "ok",
  "journal_mode": "wal",
  "schema_version": 3,
  "tables": [
    "acceptance_criteria", "evidence", "manual_confirmations", "persistence_probes",
    "projects", "schema_migrations", "task_transitions", "tasks",
    "verification_checks", "verification_runs", "verification_specs"
  ]
}
```

第二轮 E2E 运行记录：

```json
{
  "execution_status": "completed",
  "exit_code": 0,
  "check_result": "pass",
  "evidence_summary": "验证程序退出码 0，耗时 66 ms",
  "digest": "sha256:3af28e1db5537d066ffb46274948fa7f937a5eb67947f217c09d180e73c52c71",
  "task_revision": 2,
  "criterion_revision": 1,
  "spec_revision": 1,
  "start_fingerprint_complete": true,
  "end_fingerprint_complete": true,
  "fingerprints_equal": true,
  "task_status": "done",
  "project_archived": true,
  "summary_has_debug_artifact": false
}
```

## 3. 验证配置与运行模型

### 3.1 配置（verification_specs）

| 字段 | 说明 |
|---|---|
| `id` | UUID |
| `criterion_id` | 自动验收条件，唯一 |
| `executable` | 可执行程序名或绝对路径 |
| `args_json` | 参数数组 JSON；程序与参数分离 |
| `cwd` | 项目根目录或其子目录；canonicalize 校验 |
| `timeout_seconds` | 1–3600 秒 |
| `output_limit_bytes` | 每个输出流 1024–1048576 字节 |
| `success_rule` | 首版固定 `exit_code_zero`，显式展示 |
| `revision` | 配置修订号；变更使旧证据过期 |
| `created_at` / `updated_at` | UTC |

保存配置使用乐观并发：`expected_task_revision`、`expected_criterion_revision`、`expected_spec_revision`。旧修订返回 `CONFLICT`。

### 3.2 运行（verification_runs）

保存：

- 任务、条件、条件描述、spec、spec 修订；
- 任务修订、条件修订；
- 程序、解析后的程序、参数、cwd、超时、输出上限、成功规则；
- 批准时间、开始/结束时间、执行状态、退出码、耗时；
- 开始/结束 Git HEAD、分支、detached、unborn、dirty；
- 开始/结束工作区内容指纹与完整性、变更文件数、备注；
- 脱敏 stdout/stderr、总字节、截断标记、敏感性标记、内容摘要哈希；
- 错误码/消息、取消原因。

### 3.3 三层状态

- **执行状态**：`running | completed | cancelled | timed_out | start_failed | interrupted`
- **检查结果**：`pass | fail | not_run | inconclusive`
- **证据有效性**：派生 `valid | stale`，不写回历史。

判定：

| 情况 | 执行状态 | 检查结果 |
|---|---|---|
| 正常退出，退出码 0 | completed | pass |
| 正常退出，退出码非 0 | completed | fail |
| 超时 | timed_out | fail |
| 用户取消 | cancelled | inconclusive |
| 程序不存在 / 工作目录无效 / 启动失败 | start_failed | not_run |
| 应用重启导致中断 | interrupted | inconclusive |
| 运行期间代码变化 / 指纹不完整 | completed | inconclusive |

## 4. Tauri IPC 清单

Phase 1 保留 5 个；Phase 2 保留 21 个；Phase 3 新增 12 个：

| 命令 | 作用 |
|---|---|
| `list_verification_templates` | 返回内置模板（不含 Shell 包装器） |
| `get_verification_spec` | 读取条件的验证配置 |
| `save_verification_spec` | 新建/更新配置，乐观并发 |
| `preview_verification` | 返回命令、cwd、解析路径、风险提示与修订号 |
| `start_verification` | 校验批准与修订，写入 running 并启动后台线程 |
| `cancel_verification` | 设置取消标志并终止进程树 |
| `get_verification_run` | 轮询运行状态 |
| `list_verification_runs` | 任务验证历史 |
| `get_verification_check` | 运行的检查判定 |
| `get_evidence` | 证据详情与脱敏输出 |
| `list_verification_queue` | 待验收队列与真实门禁摘要 |
| `export_acceptance_report` | 生成 Markdown 报告并写入 exports/ |

权限：

- capabilities 仍只有 `core:default`；无 Shell/FS/SQL 插件。
- 前端不接收 SQL、数据库路径或通用命令执行能力。
- 错误仍为 `{ code, message }`；新增 `APPROVAL_REQUIRED`、`VERIFICATION_ERROR`。
- 内部路径、SQL、凭据不返回前端。

## 5. 用户授权机制

1. 自动条件必须先配置验证命令。
2. “运行验证”打开批准对话框；后端 `preview_verification` 返回已保存配置的完整预览。
3. 对话框展示程序、参数、cwd、解析到的绝对程序、超时、输出上限、成功规则与风险提示。
4. 用户必须勾选“我确认该项目可信……”后按钮才可用。
5. `start_verification` 再次校验：
   - `approved == true`，否则 `APPROVAL_REQUIRED`；
   - 任务必须处于 `awaiting_verification`；
   - 任务/条件/配置修订号全部匹配；
   - 同一任务没有其他 running 运行；
   - 项目未归档、任务未归档。
6. 前端不能在执行时替换 executable/args/cwd；只能引用已保存 spec。
7. 验证在后台线程执行；UI 只轮询运行记录，可取消。

安全边界：

- 拒绝 `cmd`、`powershell`、`pwsh`、`sh`、`bash`、`zsh`、`fish`、`wsl`、`cscript`、`wscript`、`mshta`。
- Windows 拒绝 `.bat/.cmd/.ps1/.vbs/.js/.wsf` 包装器；PATH 解析只接受 `.exe/.com`。
- 不使用 `cmd /c`、`powershell -Command`、`sh -c`。
- 明确承认：无 Shell 拼接不等于脚本安全；测试脚本仍以用户权限执行，必须确认项目可信。

## 6. 证据可信规则

自动条件满足必须同时满足：

1. 存在最新运行；
2. 执行状态 `completed`；
3. 检查结果 `pass`；
4. 退出码为 0（`exit_code_zero`）；
5. 运行 `task_revision`、`criterion_revision`、`spec_revision` 与当前一致；
6. 运行开始与结束时指纹完整且一致；
7. 当前工作区内容指纹与运行结束时一致（HEAD、分支、detached、unborn、dirty、内容哈希）。

证据：

- 每次运行生成一条证据（含失败/取消/中断），保存摘要、脱敏输出、内容 SHA-256、敏感性标记。
- 脱敏规则：私钥块、Authorization、Bearer、凭据赋值、常见 API Key 前缀、数据库连接串。
- 明确说明：模式化脱敏是尽力而为，不是 100% 防泄漏；原始未脱敏输出不落库。
- 原始敏感输出不返回前端；证据详情只展示脱敏文本与摘要哈希。

## 7. 验证结果过期机制

`STALE` 在读取任务详情、验收队列与完成门禁时计算：

- 任务修订变化（编辑标题/描述/优先级、增删改条件、离开待验证、重新开启）；
- 验收条件修订变化；
- 验证配置修订变化；
- Git HEAD 或分支变化；
- 工作区未提交内容变化（通过内容指纹，不只比较 dirty）；
- 运行开始/结束代码状态不一致 → `INCONCLUSIVE`；
- 指纹不完整或当前状态无法采集 → `INCONCLUSIVE`。

历史运行、检查与证据不删除；过期只改变当前有效性视图。完成门禁只接受当前有效 PASS。

## 8. 任务完成门禁

修改 `criteria_with_completion`：

- 人工条件：仍要求匹配当前 `criterion_revision` 与 `task_revision` 的人工确认。
- 自动条件：要求当前有效 PASS 证据。
- 任一未满足产生具体 blocker：
  - 未配置验证；
  - 未运行；
  - 运行中；
  - 未通过（含退出码/原因）；
  - 无法判定（含原因）；
  - 证据已过期（含过期原因）。
- 无必需条件、豁免或“跳过”路径。
- 门禁在 Rust 事务内；前端按钮禁用与 blockers 只是展示，不构成授权。
- 旧版本证据、失败结果、AI 文本、Git 改动、Hook 事件均不能替代证据。

## 9. 验收中心 UI

新增页面 `/verification`：

- 左侧待验收队列：任务、项目、自动通过数、人工待确认数、运行中图标、门禁状态；每 3 秒刷新。
- 右侧证据工作区：
  - 任务摘要、状态、优先级、修订、更新时间；
  - 自动/人工/门禁计数与具体 blockers；
  - 自动条件：`VerificationBadge`、命令、cwd、配置修订、当前判定、过期原因；
  - 操作：配置/编辑配置、运行验证、取消、查看证据、验证历史；
  - 报告导出：写入应用数据目录 `exports/`，Dialog 展示文件名、Markdown 与复制按钮。
- 人工条件：显示确认状态，链接到任务详情确认；不重复实现人工确认 Dialog。
- 任务详情：自动条件内嵌同一 `CriterionVerification`，显示真实命令、运行状态、证据与历史。
- 空队列：明确空状态与“查看任务”入口，不展示假任务/假证据。
- 所有状态：图标 + 中文 + 颜色；深/浅主题一致；不自动截图。

## 10. frontend-design Skill 实际使用情况

- 在开始 Phase 3 UI 前调用了 `frontend-design` Skill（当前主会话，2026-11-06）。
- 设计计划：
  - 色板：沿用冷石墨 + 氧化青 + 黄铜；验证状态使用既有 `verification-*` Tokens，不新增第二套颜色。
  - 字体：沿用 Segoe UI Variable / Microsoft YaHei UI；命令、cwd、哈希使用 Cascadia Code。
  - 布局：验收中心采用“待验收队列 + 证据工作区”双栏；窄屏堆叠；批准 Dialog 以命令预览为核心。
  - 原则：证据优先、状态不只靠颜色、空/错误/加载状态完整、不使用假数据。
- 未使用子 Agent；未截图/录屏。
- 设计规范已同步到 `docs/design/DESIGN_SYSTEM.md` 第 7.8 与 9.1 节。

## 11. 新增与修改文件

### 11.1 新增：Rust

- `crates/xushu-core/migrations/0003_verification_engine.sql`
- `crates/xushu-core/src/code_state.rs`
- `crates/xushu-core/src/redaction.rs`
- `crates/xushu-core/src/runner.rs`
- `crates/xushu-core/src/verification.rs`
- `crates/xushu-core/tests/verification.rs`

### 11.2 新增：Tauri 与前端

- `apps/desktop/src/features/verification/`（api、queries、schema、配置/批准/历史/证据 Dialog、CriterionVerification）
- `apps/desktop/src/lib/verification-display.ts`
- `apps/desktop/src/pages/verification-page.tsx`
- `apps/desktop/src/features/verification/*` 共 8 个文件

### 11.3 修改

- `crates/xushu-core/Cargo.toml`、`Cargo.lock`
- `crates/xushu-core/src/{db/migrations,error,git,lib,models,task}.rs`
- `crates/xushu-core/tests/{business,persistence}.rs`
- `apps/desktop/src-tauri/src/{business_commands,error,lib,state}.rs`
- `apps/desktop/src/{App.tsx,App.test.tsx}`
- `apps/desktop/src/app/app-layout.tsx`
- `apps/desktop/src/features/tasks/{criterion-dialog,criterion-list}.tsx`
- `apps/desktop/src/lib/{domain,format}.ts`
- `apps/desktop/src/pages/task-detail-page.tsx`
- `README.md`
- `docs/architecture/DEPENDENCIES.md`
- `docs/design/DESIGN_SYSTEM.md`
- `docs/design/UI_AUDIT.md`

### 11.4 新增文档

- `docs/architecture/ADR/0004-independent-verification-engine.md`
- `docs/phases/PHASE_3_REPORT.md`

### 11.5 Git

- Phase 2 基线：`b516cb7 feat: 完成 Phase 2 项目管理、任务状态机与验收门禁`。
- Phase 3 变更：**本地提交** `feat: 完成 Phase 3 独立验收引擎`，未推送。
- 未修改历史提交，未 push。

## 12. 实际测试命令与退出码

最终复跑（2026-11-06）：

| 命令 | 退出码 | 结果 |
|---|---|---|
| `pnpm install --frozen-lockfile` | 0 | 锁文件一致 |
| `pnpm typecheck` | 0 | `tsc --noEmit` 通过 |
| `pnpm lint` | 0 | ESLint 10，`--max-warnings 0` |
| `pnpm test` | 0 | 7 个测试文件、40 个测试通过 |
| `pnpm build` | 0 | Vite 构建通过；JS 757.52 kB / gzip 229.36 kB |
| `pnpm format` | 0 | Prettier 通过 |
| `cargo fmt --all -- --check` | 0 | rustfmt 通过 |
| `cargo check --workspace --all-targets --locked` | 0 | Rust workspace 通过 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | 无警告 |
| `cargo test --workspace` | 0 | 11 lib + 19 business + 12 persistence + 26 verification = 68 项通过 |
| 桌面验证 E2E 第一轮 | 0 | 配置/批准/运行/PASS/完成/重启恢复 |
| 桌面验证 E2E 第二轮 | 0 | 修复后摘要 + 完整 PASS/完成/归档 |
| 桌面验证 E2E 第三轮 | 0 | 加固后 smoke：PASS/完成/归档 |
| 桌面验证 E2E 第四轮 | 0 | 最终复核：绝对路径执行/PASS/完成/报告导出 |

开发中发现并修复的真实问题：

1. `regex` 不支持反向引用：脱敏规则改为非反向引用形式。
2. `CreateJobObjectW` 需要 `Win32_Security` feature；补齐后 Job Object 编译通过。
3. 证据摘要使用 `{:?}` 产生 `Some(0)`；改为自然中文，并新增回归断言。
4. 任务详情/验收队列原在数据库锁内采集 Git 指纹，可能与后台 finalize 写入相互等待；改为锁外采集、锁内短事务评估。
5. 种子测试的 schema 版本断言从 2 更新到 3（迁移 3 的预期变化）。
6. 自动条件 blocker 文案从 “Phase 3 验证” 更新为真实“尚未配置验证”，并更新 Phase 2 回归测试断言。
7. 未解析到可执行程序时原会尝试启动；最终复核改为运行前阻断并记录 NOT_RUN，且执行准备阶段解析到的绝对路径，避免批准后 PATH 变化执行另一程序。
8. 验收报告原缺少实际命令与工作目录；运行摘要与报告补充 `executable/args/cwd`，并在报告中加入本地路径分享提示。
9. 验收队列在指纹采集后重新读取任务，避免采集期间任务被修改却显示旧修订的“门禁已满足”。

**NOT_RUN：**

- `pnpm tauri build` / Windows 安装包；
- 生产 CSP 下的成品运行验证；
- macOS 实机；
- Claude Code Hooks（Phase 4）；
- 决策记忆与断点恢复（Phase 5）。

## 13. 桌面真实 E2E 结果

### 13.1 方法

- Windows 11，WebView2 156.0.4314.58，窗口 1100×720。
- 使用 WebView2 CDP（临时调试端口 9222）驱动真实窗口与真实 Tauri IPC；临时配置与脚本放在 `target/e2e/`（`.gitignore` 排除，未提交）。
- 未使用截图/录屏；未使用 UI 自动化截图断言。
- E2E 配置仅存在于临时文件，不修改仓库内 `tauri.conf.json`；测试结束后应用与 Vite 均已停止，9222 无监听。

### 13.2 第一轮（tag `20261106015932`）

流程：

1. 导入临时 Git 仓库；
2. 创建任务与必需自动条件；
3. ready → running → awaiting_verification；
4. 通过 IPC 保存 `git --version` 验证配置；
5. 在真实 UI 点击“运行验证”；
6. 在批准 Dialog 勾选确认并点击“批准并运行”；
7. 轮询到“当前有效 PASS”；
8. 点击“标记完成”；
9. 应用重建/重启后重新读取。

结果：

```json
{
  "passBadgeVisible": true,
  "evidenceButtonVisible": true,
  "doneButtonEnabled": true,
  "taskDone": true,
  "evidenceRecorded": true,
  "evidenceDigestPresent": true,
  "verificationCenterRendered": true,
  "statusDone": true,
  "verificationPass": true,
  "completionSatisfied": true,
  "passVisibleAfterRestart": true,
  "evidenceVisibleAfterRestart": true,
  "evidenceDialogOpen": true
}
```

第一轮在“证据摘要自然化”修复前执行，历史证据保留旧文案 `Some(0)`；未改写历史记录。

### 13.3 第二轮（tag `20261106021137`，修复后二进制）

重复完整流程，结果：

```json
{
  "passBadgeVisible": true,
  "evidenceButtonVisible": true,
  "doneButtonEnabled": true,
  "taskDone": true,
  "evidenceRecorded": true,
  "evidenceDigestPresent": true,
  "verificationCenterRendered": true,
  "projectArchivedAfterTest": true
}
```

证据摘要：`验证程序退出码 0，耗时 66 ms`；`summary_has_debug_artifact=false`。

### 13.4 第三轮（tag `20261106022214`，cwd 预检与进程清理加固后）

在加入“运行前再次校验 cwd 边界”和“正常退出后清理残留子孙进程”后补跑完整 smoke：

```json
{
  "passBadgeVisible": true,
  "evidenceButtonVisible": true,
  "doneButtonEnabled": true,
  "taskDone": true,
  "evidenceRecorded": true,
  "evidenceDigestPresent": true,
  "verificationCenterRendered": true,
  "projectArchivedAfterTest": true
}
```

证据摘要：`验证程序退出码 0，耗时 68 ms`。

### 13.5 第四轮（tag `20261106023630`，最终复核）

在加入“运行前阻断未解析程序、执行解析后的绝对路径、运行摘要/报告包含命令与 cwd、队列重新读取任务”后补跑：

```json
{
  "passBadgeVisible": true,
  "evidenceButtonVisible": true,
  "doneButtonEnabled": true,
  "taskDone": true,
  "evidenceRecorded": true,
  "evidenceDigestPresent": true,
  "verificationCenterRendered": true,
  "projectArchivedAfterTest": true
}
```

证据摘要：`验证程序退出码 0，耗时 45 ms`。通过真实 IPC 导出验收报告，报告包含：

- `命令` 代码块：`git --version`
- `工作目录`
- `pass`

本轮同时验证解析后的绝对可执行路径在真实 Tauri 运行链路中可用。报告分享前检查提示在最终代码中加入，并由 `report_export_contains_real_results_and_is_not_a_pass_guarantee` 集成测试断言。

四轮测试项目均在结束后通过 IPC 软归档；任务、运行、检查、证据保留在本地数据库。测试数据名称包含运行 tag，未删除用户数据。

结果文件（未提交）：`target/e2e/e2e-result-full.json`、`target/e2e/e2e-result-verify.json`、`target/e2e/e2e-result-report.json`。最后一轮覆盖了 `target/e2e/e2e-result-full.json`。

## 14. 安全与隐私检查

- 未读取/输出 API Key、`.env`、SSH 私钥、token；未修改全局 Claude Code / DeepSeek / Git / Shell / 注册表配置。
- 未使用子 Agent / 多 Agent；未截图/录屏。
- 未安装 Claude Code Hooks；未注册系统服务；未修改其他项目。
- Tauri capabilities 仍只有 `core:default`；没有 Shell/FS/SQL 插件权限。
- 验证执行：
  - 程序 + 参数数组，无 Shell 拼接；
  - 拒绝 Shell 与 `.cmd/.bat/.ps1` 包装器；
  - 执行准备阶段解析到的绝对路径；未解析到程序时不启动并记录 NOT_RUN；
  - 工作目录 canonicalize 后必须在项目根目录内，运行前再次校验；
  - 前端不能在批准时替换命令；
  - 未批准一律拒绝；
  - 同一任务同一时间只允许一个运行。
- Windows 进程树：Job Object + `KILL_ON_JOB_CLOSE`；取消/超时 `TerminateJobObject`；正常退出后也终止残留子孙进程，避免守护子进程占用管道；Job 分配失败回退 `%SystemRoot%\System32\taskkill.exe /T /F`（固定系统路径，避免 PATH 劫持）。自动化测试验证子进程在取消后不会写入标记文件。
- 输出：按上限截断并排空管道；落库前模式化脱敏；原始敏感输出不落库；证据内容哈希使用 SHA-256。
- 数据库：位于系统应用数据目录，不进入 Git；E2E 前后只读查询 `integrity=ok`、WAL、schema=3。
- Git：只读 `rev-parse`、`symbolic-ref`、`status`；不修改用户仓库。
- 报告：写入应用数据目录 `exports/`，IPC 不返回绝对路径；报告内容可能包含本地命令、工作目录与脱敏输出，UI 与 Markdown 文末均提示分享前检查。
- 未把“正则脱敏”描述为 100% 防泄漏；UI 明确提示证据是本地敏感数据。
- E2E 临时调试端口仅用于本地测试，测试后已停止；仓库配置未加入调试端口。

## 15. UI 人工验收状态

- 自动化：类型、Lint、Vitest、构建、桌面 E2E 均通过。
- 人工视觉：**VISUAL_REVIEW_PENDING**。
- 需要用户手动截图确认：
  - 验收中心空队列与有任务状态；
  - 配置 Dialog、批准 Dialog（复选框禁用/启用）；
  - 运行中与取消；
  - 证据 Dialog（脱敏标记、长输出）；
  - 验证历史；
  - STALE 状态；
  - 深/浅主题、900/1400px 窗口、长命令/长输出不撑破布局。
- 未获得用户截图前，不得声称 UI 已完美通过。

## 16. 遗留问题与风险

1. **Windows 批处理包装器不支持**：首版拒绝 `.cmd/.bat/.ps1`；pnpm/npm 在 Windows 的 `.cmd` shim 不能直接作为验证程序，需要配置底层 `.exe`（如 `node`、`cargo`）。
2. **macOS 未验证**：非 Windows 取消仅终止主进程，未做进程组验证；Job Object 与路径边界需在 macOS 重新评估。
3. **生产 CSP 与安装包未验证**：`pnpm tauri build`、安装/卸载、生产 CSP 成品运行均为 NOT_RUN。
4. **指纹上限**：最多 2000 个变更文件、单文件 8 MB、总 64 MB；超过则指纹不完整，自动条件为 INCONCLUSIVE。超大工作区需重新验证策略。
5. **TOCTOU 窗口**：代码指纹采集与完成提交之间代码仍可能变化；当前通过“下次读取立即过期”控制，但不能完全消除竞态。
6. **多实例假设**：启动恢复会把数据库中 `running` 运行标为中断；若同时启动第二个应用实例，可能误伤第一个实例的运行。当前假设单实例使用，后续需单实例锁。
7. **脱敏是模式匹配**：不能保证识别所有密钥格式；用户不应把证据输出粘贴到外部服务。
8. **单连接 SQLite**：验证 finalize 与 UI 查询可能短暂排队；已把 Git 指纹采集移到锁外，但长查询仍需后续评估。
9. **无硬删除/备份**：运行、检查、证据持续保留；备份/恢复与保留策略在后续阶段。
10. **前端单 chunk 757.52 kB**（gzip 229.36 kB），Vite 提示 >500 kB；后续可路由级拆包。
11. **E2E 为 Windows CDP 脚本**：不是跨平台自动化套件，脚本未入仓。
12. **测试数据**：四轮 E2E 项目已软归档，任务/证据保留在真实本地数据库。
13. **列表上限**：验证历史 IPC 最多返回 100 条，待验收队列最多 50 条；更早数据仍保留在数据库中，后续需要分页/导出策略。
14. **被 Git 忽略的文件不进入指纹**（如 `node_modules/`、构建产物）；若验证依赖这些文件，命令本身应负责重建它们。子模块目录会标记指纹不完整，自动 PASS 会被拒绝。
15. **符号链接**按链接目标字符串纳入指纹，不读取链接目标内容；跨根目录链接不会被跟随。

## 17. Phase 3 PASS / FAIL / BLOCKED 结论

| 验收目标 | 结果 | 证据 |
|---|---|---|
| 验证配置：程序/参数/cwd/超时/输出上限 | PASS | Rust 校验 + 配置测试 |
| 用户预览与明确批准 | PASS | 批准 Dialog + `APPROVAL_REQUIRED` 测试 |
| 成功运行 → PASS | PASS | 集成测试 + 四轮桌面 E2E |
| 非零退出 → FAIL | PASS | 集成测试 |
| 命令不存在 / 工作目录无效 → NOT_RUN | PASS | 集成测试 |
| 超时 → FAIL | PASS | 集成测试 |
| 用户取消 → INCONCLUSIVE，无假 PASS | PASS | 集成测试 + 子进程树测试 |
| 输出上限与管道排空 | PASS | 集成测试 |
| 敏感输出脱敏 | PASS | 集成测试 + 证据检查 |
| 数据库写入失败回滚 | PASS | 单元测试（唯一约束 / 只读库） |
| 结果与证据持久化 / 重启恢复 | PASS | 集成测试 + 桌面 E2E 重启 |
| 自动验收 PASS/FAIL/NOT_RUN/INCONCLUSIVE/STALE | PASS | 26 项验证测试 |
| Git HEAD / 未提交代码变化导致过期 | PASS | 集成测试 |
| 任务/条件/配置修订导致过期 | PASS | 集成测试 |
| 人工确认与自动验收混合 | PASS | 集成测试 + 桌面 E2E |
| 无有效证据禁止 done | PASS | 集成测试 + 门禁测试 |
| Phase 1/2 回归 | PASS | 19 business + 12 persistence 测试 |
| IPC 权限与命令边界 | PASS | capabilities `core:default`；无 Shell/FS/SQL |
| 统一 UI 与设计系统 | PASS（视觉待人工） | DESIGN_SYSTEM + UI_AUDIT；`VISUAL_REVIEW_PENDING` |
| 不提前进入 Phase 4 | PASS | 无 Hooks、无事件采集 |

**Phase 3 最终状态：PASS。**

该 PASS 覆盖配置、批准、执行、超时/取消、输出/脱敏、证据、过期、完成门禁、IPC、UI 与真实 Windows 桌面 E2E；**不覆盖**安装包、生产 CSP 成品、macOS、多实例、人工视觉验收。

## 18. Phase 4 建议起点

Phase 4 的核心是 **Claude Code Hooks 可选接入**，不是让 Hook 成为完成证据：

1. 新增 `xushu-hook` Rust CLI，只写本地 spool，不读取 transcript、源码、完整工具参数或密钥。
2. 项目级 Hook 安装必须 opt-in；只修改用户选定项目配置，先备份、结构化合并、展示 diff。
3. 初始事件仅白名单元数据：事件类型、去标识会话、项目标识、工具名、时间、可选受限路径元数据。
4. 原子写入 inbox、导入去重、缺失事件提示；Hook 失败不阻塞 Claude Code。
5. 卸载只删除 XUSHU 自己写入的条目；配置冲突时停止并给出人工复核说明。
6. Hook 事件只作为“观察到某操作”，不得替代 Phase 3 验证 PASS 或人工确认。
7. 保持 Phase 3 的完成门禁不变；Hook 数据缺失时不得推断成功。
8. 完成真实 Windows Claude Code Hook 触发、入库、去重、卸载与故障不阻塞测试后，再进入 Phase 5。

当前不自动进入 Phase 4，等待用户确认。

## 19. 标准汇报摘要

```text
当前阶段 / 任务：Phase 3 — 独立验收引擎
实施摘要：新增迁移 3 与 VerificationSpec/Run/Check/Evidence 领域；安全参数数组运行器、Windows Job Object 进程树清理、超时/取消、输出上限与脱敏；工作区内容指纹、修订与代码状态过期判定；完成门禁接入自动 PASS；12 个验证 IPC；验收中心与任务详情验证 UI；四轮真实 Windows 桌面 E2E。
修改文件：见第 11 节；Phase 3 已创建本地提交，未推送。
实际执行的命令与退出码：见第 12 节；前端 40 测试、Rust 68 测试、全部质量门禁退出码 0。
测试：PASS — 单元/集成/桌面 E2E；NOT_RUN — tauri build 安装包、生产 CSP 成品、macOS、Hooks、人工视觉验收。
安全/隐私检查：未读取/输出密钥；未修改全局配置；未使用子 Agent；未截图；capabilities 仅 core:default；验证无 Shell、拒绝批处理包装器、进程树清理、输出脱敏。
明确未完成或尚未验证：安装包、生产 CSP、macOS、多实例、人工视觉验收、超大工作区指纹策略。
下一步：等待用户审查 Phase 3 与 UI 截图；确认后再规划 Phase 4。
```

## 20. 结论

Phase 3 已在 2026-11-06 实际完成并通过验收。XUSHU 现在能够：

- 由用户配置并批准真实验证命令；
- 在后台安全执行，处理超时、取消、输出上限与子进程树；
- 保存退出码、耗时、脱敏输出、Git/工作区指纹与内容哈希；
- 严格区分 PASS / FAIL / NOT_RUN / INCONCLUSIVE / STALE；
- 在任务、条件、配置或代码变化后使旧证据失效；
- 通过 Rust 完成门禁阻止无证据或过期证据的完成；
- 在验收中心查看队列、命令、证据、历史并导出报告。

UI 自动化与桌面 E2E 通过不等于视觉验收通过；在用户提供截图前，视觉状态保持 `VISUAL_REVIEW_PENDING`。

**停止于 Phase 3，等待用户确认是否进入 Phase 4。**