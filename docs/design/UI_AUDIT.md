# XUSHU UI Audit

## v1.0.0 macOS 窗口收口

macOS 将系统交通灯、品牌、项目、页面和主题放入同一条 44px 工作带。左上角预留系统控件空间，空白区域支持拖动与按系统偏好双击，沿用暖白与墨绿令牌、240px 侧栏及 52px 窄窗图标轨。正式版本信息位于侧栏和设置页。Windows 的 44px 拖动工作带、右侧窗口按钮及 Snap 覆盖层沿用既有实现。前端增加 macOS 与 Windows 条件渲染测试，Rust 配置测试核对 macOS 覆盖后的完整窗口属性。

> **VISUAL_REVIEW_PENDING**。Phase 6 已完成数据保护页的程序化、隔离桌面 E2E 与布局/主题测量；人工视觉仍未签收。当前待验收清单见 [Phase 6 报告第 22 节](../phases/PHASE_6_REPORT.md#22-人工验收清单)。Phase 4/5 的历史 PENDING 未被本轮改为 PASS。
> 关联规范：`docs/design/DESIGN_SYSTEM.md`（v2.4 / Phase 6）
> 关联报告：`docs/design/UI_REDESIGN_PHASE_3_6.md`
> 窗口决策：`docs/architecture/ADR/0006-custom-titlebar-window-chrome.md`（取代 ADR-0005）
> 更新规则：每次新增或修改页面前阅读设计系统；完成后更新本文件。
> 设计方法：已调用 `frontend-design` Skill（当前主会话，2026-11-06），未使用子 Agent。

## 1. 页面清单

| 页面 | 路由 | 状态 | 设计要点 | 自动化检查 | 人工视觉 |
|---|---|---|---|---|---|
| 工作台 Dashboard | `/` | Phase 3.6 精修 | 紧凑 Focus Panel；待处理排除已展开焦点；任务行与验证记录区分密度 | PASS（Vitest + 桌面回归 + 布局测量） | PENDING（精修） |
| 项目 Projects | `/projects` | Phase 3.6 调整 | 无独立页头；Tabs 与导入按钮同行；行式项目列表 | PASS（Vitest + 布局测量） | PENDING |
| 项目详情 Project Detail | `/projects/:id` | Phase 4 增强 | 仓库 strip、Claude Code 接入 strip、任务表、项目信息侧栏；栅格 `min-w-0` | PASS（Vitest + 桌面冒烟 + 布局测量） | PENDING |
| 任务 Tasks | `/tasks` | Phase 3.6 精修 | 保留筛选/操作；状态统一为图标 + 中文 + 颜色的行内形式 | PASS（Vitest + 布局测量） | PENDING |
| 任务详情 Task Detail | `/tasks/:id` | 保留 + 响应式修复 | StateTrack、目标/验收/历史、门禁侧栏；栅格 `min-w-0` | PASS（Vitest + 桌面 E2E + 布局测量） | PENDING |
| 验收中心 Verification | `/verification` | Phase 3.6 调整 | 无独立页头；导出在证据工作区标题行；队列 + 证据工作区 | PASS（Vitest + 桌面 E2E + 布局测量） | PENDING |
| 项目记忆与决策 Knowledge | `/projects/:id/knowledge` | Phase 5 新增 | PageHeader、记忆/决策 Tabs、日志式行、详情 Sheet、编辑 Dialog、任务筛选条 | PASS（Vitest + 桌面 E2E 文本可见） | PENDING |
| 恢复 Recovery | `/recovery` | Phase 5 新增 | 恢复带、Focus Panel 下一步、双列摘要、完整指令/历史 Sheet、来源 details | PASS（Vitest + 桌面 E2E） | PENDING |
| 数据保护 Data Protection | `/data-protection` | Phase 6 新增 | 启动提示、状态带、备份日志、恢复历史、完整性报告、隐私资产清单、7 步恢复向导 | PASS（Vitest + 隔离桌面 E2E + 布局/主题测量） | PENDING |
| 本地数据与健康 | `/diagnostics` | Phase 3.6 调整 | 无独立页头；系统健康、探针说明、表单/记录两栏 | PASS（Vitest + 布局测量） | PENDING |
| 设置 Settings | `/settings` | Phase 3.6 调整 | 无独立页头；分组行式设置、主题 radio、关于/数据 | PASS（Vitest + 布局测量） | PENDING |
| 404 | `*` | 保留 | 居中空状态；顶部工作带显示“未找到” | PASS（布局测量） | PENDING |

## 2. 组件复用清单

| 组件 | 位置 | 使用页面 | 状态 |
|---|---|---|---|
| WorkspaceChrome | `src/app/workspace-chrome.tsx` | 全部 | 新增：44px 顶部工作带、品牌、项目上下文、页面名称、主题、窗口控制 |
| WindowControls | `src/components/window-controls.tsx` | 全部 | 新增：46×44px 最小化/最大化/还原/关闭；键盘可访问 |
| Window Chrome (Rust) | `src-tauri/src/window_chrome.rs` | 主窗口 | 新增：DWM 圆角/边框、Snap `HTMAXBUTTON` 覆盖层、悬停事件 |
| App Shell / Sidebar | `src/app/app-layout.tsx` | 全部 | 重构：240px / <760px 52px 图标轨；导航与版本；不含项目上下文 |
| PageHeader | `src/components/page-header.tsx` | 项目详情、任务详情 | 保留：实体 `h1`、meta、操作 |
| Section / SectionHeader | `src/components/section.tsx` | 工作台、项目、任务、验收、设置、诊断 | 使用 |
| Button | `src/components/ui/button.tsx` | 全部 | 使用 |
| Input / Textarea / Select | `src/components/ui/*` | 表单/筛选 | 使用 |
| Dialog | `src/components/ui/dialog.tsx` | 导入、创建、编辑、批准、确认 | 使用 |
| Sheet | `src/components/ui/sheet.tsx` | 证据、验证历史、报告 | 使用 |
| Tabs | `src/components/ui/tabs.tsx` | 项目活动/归档 | 使用；与导入按钮同行 |
| Table | `src/components/ui/table.tsx` | 任务列表、项目任务 | 使用 |
| Badge / StatusBadge | `src/components/ui/badge.tsx`、`components/status-badge.tsx` | 全部状态 | 使用 |
| DropdownMenu | `src/components/ui/dropdown-menu.tsx` | 项目/任务行操作 | 使用 |
| Toast | `sonner` + `components/ui/sonner.tsx` | 操作反馈 | 使用 |
| EmptyState | `src/components/empty-state.tsx` | 工作台、项目、任务、验收、诊断 | 使用 |
| LoadingState | `src/components/loading-state.tsx` | 查询加载 | 使用 |
| ErrorState | `src/components/error-state.tsx` | 查询错误 | 使用 |
| StateTrack | `src/components/state-track.tsx` | 任务详情 | 使用；窄窗口内部横向滚动 |
| StateSpine | `src/components/state-spine.tsx` | 任务历史 | 使用 |
| CommandBlock | `src/features/verification/command-block.tsx` | 任务详情、验收中心、批准 Dialog | 使用 |
| CriterionVerification | `src/features/verification/criterion-verification.tsx` | 任务详情、验收中心 | 使用 |
| SystemHealthCard | `src/features/system/system-health-card.tsx` | 工作台侧栏、诊断 | 使用 |
| MemoryConfirmationBadge / MemoryArchivedBadge | `src/features/knowledge/knowledge-badges.tsx` | 项目知识页、项目/任务详情 | 新增：图标 + 中文 + 语义色 |
| DecisionStatusBadge | `src/features/knowledge/knowledge-badges.tsx` | 项目知识页、恢复页、项目/任务详情 | 新增：草稿/已确认/已被取代/已废弃 |
| MemoryDetailSheet / DecisionDetailSheet | `src/features/knowledge/*-detail-sheet.tsx` | 项目知识页 | 新增：760px Sheet、修订历史、状态操作 |
| MemoryFormDialog / DecisionFormDialog | `src/features/knowledge/*-form-dialog.tsx` | 项目知识页 | 新增：RHF + Zod、类别/状态、关联任务、取代选择 |
| RecoveryFreshnessBadge / NextStepKindBadge / RecoveryGitBadge | `src/features/recovery/recovery-badges.tsx` | 恢复页、指令 Sheet | 新增：当前/过期/未生成、用户指定/建议/需确认、Git 可用性 |
| RecoveryInstructionsSheet | `src/features/recovery/recovery-instructions-sheet.tsx` | 恢复页 | 新增：完整指令预览、边界提示、深色 Markdown、复制 |
| RecoveryHistorySheet | `src/features/recovery/recovery-history-sheet.tsx` | 恢复页 | 新增：最近 20 份快照、当前/历史、加载态 |
| BackupIntegrityBadge / BackupOriginBadge / IntegrityStatusBadge | `src/features/data-protection/data-protection-badges.tsx` | 数据保护页、校验 Sheet、恢复向导 | 新增：已验证/失败/未校验/目录缺失、手动/自动、通过/失败/未运行；图标 + 中文 + 语义色 |
| RestoreOutcomeBadge / RestoreHistoryOutcomeBadge | `src/features/data-protection/data-protection-badges.tsx` | 数据保护页、恢复向导 | 新增：完成/已回退/回退失败；回退不显示为成功 |
| CreateBackupDialog | `src/features/data-protection/create-backup-dialog.tsx` | 数据保护页 | 新增：目标、范围、预计大小、未加密警告、阶段状态、取消、结果 |
| BackupPolicyDialog | `src/features/data-protection/backup-policy-dialog.tsx` | 数据保护页 | 新增：自动策略、保留、旧 automatic 预览与两步清理 |
| BackupVerifySheet | `src/features/data-protection/backup-verify-sheet.tsx` | 数据保护页 | 新增：清单/哈希/隔离完整性校验结果 |
| RestoreWizardSheet | `src/features/data-protection/restore-wizard-sheet.tsx` | 数据保护页 | 新增：7 步序列、影响范围、隔离验证、输入“恢复”+勾选、结果/回退 |
| IntegrityCheckList / IntegrityReportSummary | `src/features/data-protection/integrity-report.tsx` | 数据保护页、校验 Sheet、恢复向导 | 新增：检查项、Schema、计数、失败 detail |
| PrivacyInventory / HooksBoundary | `src/features/data-protection/privacy-inventory.tsx` | 数据保护页 | 新增：数据资产、备份包含性、敏感性、Hook 采集/排除、Spool/Salt 边界 |

## 3. 状态表达检查

- [x] 七种任务状态使用图标 + 中文 + 颜色，不只依赖颜色。
- [x] PASS / FAIL / NOT_RUN / INCONCLUSIVE / STALE 使用 `VerificationBadge`。
- [x] 执行状态（运行中/已完成/取消/超时/启动失败/中断）与检查结果分开表达。
- [x] 阻塞任务显示 `blockReason`。
- [x] 完成门禁显示 required/satisfied 计数与具体 blockers。
- [x] 过期证据显示 STALE Badge 与 `staleReason`。
- [x] 不把 AI 声明或前端状态当作完成证据；完成由 Rust 门禁校验。
- [x] 自动验证、人工确认、AI 声明在列表/详情中来源可辨。
- [x] 验证运行中显示旋转图标 + 文字 + 取消按钮。
- [x] 工作台首屏的“下一步”与“最近验证”直接来自真实任务/运行数据，不构造 KPI。
- [x] 备份完整性使用“已验证/校验失败/未校验/目录缺失”，未校验或失败不显示为成功。
- [x] 完整性检查项使用“通过/失败/未运行”+ detail；检查失败不只显示颜色。
- [x] 恢复结果显示“恢复完成/已回退/回退失败”；回退不显示为成功恢复。
- [x] 恢复向导 7 步使用真实序列与 `aria-current`；预检未通过不显示“继续确认”。
- [x] 备份未加密、备份范围、Hooks 不自动修改、恢复会替换当前数据库在创建/预检/确认/结果处可见。
- [x] 数据资产清单明确 Spool/Salt/项目配置/导出/WebView 偏好是否进入备份；Salt 不显示内容。

## 4. 可访问性检查（代码层面）

- [x] 交互元素有 `focus-visible` ring；窗口控制使用 inset ring，避免被窗口边缘裁切。
- [x] 窗口控制有中文 `aria-label` 与 `title`；最大化/还原名称随状态变化。
- [x] 一级页面使用 `sr-only h1`；详情页使用可见 `h1`。
- [x] 顶部工作带的 `data-tauri-drag-region="deep"` 不阻断按钮、链接、选择器。
- [x] Dialog/Sheet 使用 Radix：焦点陷阱、Escape、关闭后返回焦点。
- [x] 表单 Label 关联、错误 `role="alert"`、必填/可选说明。
- [x] 表格有表头；行链接与操作按钮有可读名称。
- [x] 状态 Badge 有文字与图标，不只靠颜色。
- [x] 主题 radio 使用原生 input；Select 使用 Radix。
- [x] `prefers-reduced-motion` 在全局 CSS 中处理。
- [x] 窄窗口导航使用 `sr-only` 保留链接名称；状态数字改为圆点但仍可读。
- [ ] 500px 窗口下窗口控制、主题与页面名称无重叠：技术测量通过，待人工视觉确认。
- [ ] 深色/浅色对比度：Token 目标 ≥ 4.5:1，待人工视觉确认。
- [ ] 窗口失焦/焦点反馈：窗口阴影 + 内容 focus ring，待人工视觉确认。

## 5. 自动化检查记录

| 命令 | 退出码 | 结果 |
|---|---|---|
| `pnpm typecheck` | 0 | `tsc --noEmit` 通过 |
| `pnpm lint` | 0 | ESLint 10，`--max-warnings 0` |
| `pnpm test` | 0 | 7 个测试文件、43 个测试通过（精修新增 3 个行为回归） |
| `pnpm build` | 0 | Vite 构建通过；JS 796.60 kB / gzip 238.60 kB；仍有原有 >500 kB 提示 |
| `pnpm format` | 0 | Prettier 通过 |
| `cargo fmt --all -- --check` | 0 | rustfmt 通过 |
| `cargo check --workspace --all-targets --locked` | 0 | Rust workspace 通过 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | 无警告 |
| `cargo test --workspace` | 0 | 11 + 19 + 12 + 26 + 1 = 69 项通过 |
| 桌面窗口/布局测量 | 0 | 1366/1600/1920/500 × 深浅主题；无页面级横向溢出 |
| 覆盖层命中/点击/关闭 | 0 | `HTMAXBUTTON=9`；最大化/还原；关闭后进程退出 |

## 6. 桌面窗口与布局验证

### 6.1 窗口方案

- 自定义标题栏：`decorations: false`、`shadow: true`、`minWidth: 500`、`minHeight: 480`。
- 顶部 44px 工作带；左侧品牌 240px（<760px 为 52px）；窗口控制 46×44px。
- 窗口控制权限：`start-dragging`、`internal-toggle-maximize`、`minimize`、`toggle-maximize`、`close`、`is-maximized`、`set-theme`。
- Snap 覆盖层：`XushuSnapOverlay` 原生子窗口，`WM_NCHITTEST → HTMAXBUTTON`；DWM 圆角 + 无边框色。
- 不启用 `withGlobalTauri`；不新增第三方 Tauri 插件。
- 详细决策：`docs/architecture/ADR/0006-custom-titlebar-window-chrome.md`。

### 6.2 窗口行为

| 操作 | 方法 | 结果 |
|---|---|---|
| 最小化 | 点击自绘最小化按钮 + `IsIconic` | PASS（Tauri Window `iconic=true`） |
| 最大化 | 悬停后点击 Snap 覆盖层 + `IsZoomed` | PASS |
| 还原 | 再次点击覆盖层 + `IsZoomed` | PASS |
| 最大化/还原按钮状态 | 点击 DOM 按钮，读取 `aria-label` | PASS（最大化 → 还原） |
| 关闭 | 点击自绘关闭按钮 | PASS（进程退出） |
| 拖动 | 真实鼠标按下标题带并移动 | PASS（窗口位移 +120/+48） |
| 双击最大化 | CDP 双击标题带空白区 | PASS（`IsZoomed=true`）；实机鼠标双击待人工复核 |
| Snap 覆盖层存在 | `EnumChildWindows` | PASS（150% DPI 下 69×66；100% 下 46×44） |
| Snap 命中 | `SendMessage(WM_NCHITTEST)` | PASS（返回 9 / `HTMAXBUTTON`） |
| Snap 悬停菜单视觉 | Windows 系统 UI | **NOT_RUN / 待人工** |
| 边缘缩放 | Tauri undecorated resize | 保留；未自动化拖拽 |
| 多显示器 | — | 未测试（单显示器环境） |
| 125%/175%/200% DPI | — | 未测试 |
| 焦点/失焦 | — | 待人工视觉确认 |

### 6.3 顶部工作带结构测量

- 工作带高度：44px；侧栏起点 y=44。
- 品牌区：240px（≥760px）/ 52px（<760px）。
- 窗口控制：138px（3×46）、高 44px，右侧贴边。
- 项目上下文：项目名 13px/500、页面名称 13px/600；「当前项目」保留为 `sr-only`。≥1100px 显示 Git 分支与中文工作区状态，≥1280px 再显示 Commit；选择器提供完整项目名/路径 title，Git 提供完整分支/HEAD title。窄窗口保留项目选择与全部窗口操作。
- 页面名称：由工作带承载；一级页面内容使用 `sr-only h1`。
- 本次精修工作台首屏：1366×768 下「下一步」top≈119、高≈97px；「最近验证」top≈236。500×480 下 Focus Panel 主操作换行，高≈137px；保持完整原因可读。

### 6.4 尺寸与 DPI

**自然 150% DPI（AppliedDPI=144）**

| 窗口（Win32 设置） | CSS 视口 | 侧栏 | 7+2 页面横向溢出 |
|---|---|---|---|
| 1366 × 768 | 1352 × 760 | 240 | 无 |
| 1600 × 900 | 1586 × 892 | 240 | 无 |
| 1920 × 1080 | 1906 × 1072 | 240 | 无 |
| 500 × 480 | 486 × 472 | 52 | 无页面级溢出 |

> 任务详情的 StateTrack 在窄窗口内部横向滚动；主滚动区使用 `overflow-x-hidden` 与 `scrollbar-gutter: stable`，不产生页面级横向滚动。

**100% 缩放仿真（CDP `Emulation.setDeviceMetricsOverride`, dsf=1）**

| 视口 | 7+2 页面横向溢出 |
|---|---|
| 1366 × 768 | 无 |
| 1600 × 900 | 无 |
| 1920 × 1080 | 无 |

覆盖页面：工作台、项目、项目详情、任务、任务详情、验收中心、诊断、设置、404。

### 6.5 主题

| 主题 | `documentElement.dark` | 页面背景 | 工作带 | 侧栏 |
|---|---|---|---|---|
| 深色 | true | `rgb(10,16,18)` | `rgb(17,26,28)` | `rgb(13,22,24)` |
| 浅色 | false | `rgb(238,243,241)` | `rgb(248,250,249)` | `rgb(230,237,234)` |

- 深浅主题切换通过；工作带、侧栏与主内容表面层级正确。
- 窗口阴影/圆角由 DWM 控制；Snap 覆盖层不可见。
- 未自动截图；等待用户截图确认视觉层级、对比度与细节。

### 6.6 真实业务交互

- Phase 3.5 的桌面 E2E 数据仍保留：项目 `Phase 3.5 UI 审阅`、任务、验证运行与证据。
- Phase 3.6 未修改 Rust 业务命令、SQLite 迁移、验证引擎或完成门禁。
- 本轮前端 43 个测试与 Rust 69 个测试全部通过，覆盖焦点去重、剩余任务/阻塞原因、验证记录入口、状态机、验证过期、完成门禁与持久化。

## 7. 人工视觉复核清单

不自动截图。请用户按以下路径操作并提供截图（浅色/深色各一组）：

1. **窗口顶部**：自定义工作带与侧栏是否连续；是否还能看出“标题栏 + 内容”两层。
2. **品牌与项目上下文**：240px 品牌区、当前项目选择器、branch/HEAD/dirty、页面名称是否在同一工作带内。
3. **窗口控制**：46×44px 按钮、悬停、关闭红色悬停、最大化/还原图标切换。
4. **拖动/双击**：拖动顶部空白区移动窗口；双击空白区最大化/还原。
5. **Snap Layouts**：悬停最大化按钮是否出现 Windows 11 贴靠菜单；选择半屏/三栏是否成功。
6. **窗口圆角/阴影**：深浅主题下是否有异常白边、黑边或直角。
7. **工作台（有项目/有任务）**：首屏状态条、焦点任务、最近验证、需要处理。
8. **工作台（无项目）**：空状态与三步提示；顶部项目上下文显示导入入口。
9. **项目列表**：Tabs 与导入按钮同行；当前项目点、Git 行、行操作菜单。
10. **项目详情**：仓库 strip、任务表、项目信息侧栏；窄窗口栅格不溢出。
11. **任务列表**：筛选工具栏含新建任务；表格密度、状态/优先级、行操作。
12. **任务详情（待验证）**：StateTrack、验收条件、命令块、完成门禁 blockers、状态操作。
13. **任务详情（已完成/阻塞/取消）**：状态视觉、只读操作、blockReason。
14. **验收中心（空队列/待验收）**：队列、统计、blockers、自动/人工条件、FAIL/PASS。
15. **证据/历史/报告 Sheet**：长输出滚动、深色终端、复制。
16. **设置/诊断**：无大标题；分组行式；系统健康与探针。
17. **窄窗口 500×480**：52px 图标轨、项目选择器、页面名称、主题、窗口控制是否可操作。
18. **窗口尺寸**：1366×768、1600×900、1920×1080；主内容无页面级横向滚动。
19. **主题**：深色/浅色下工作带、窗口控制、菜单、Dialog、Sheet、Toast、滚动条。
20. **键盘/焦点**：Tab 导航、窗口控制焦点、Dialog Escape、主题 radio、行操作菜单。

## 8. 已知风险与待确认

- **Snap 悬停菜单**：`HTMAXBUTTON` 与覆盖层位置已验证；Windows 系统贴靠菜单的视觉与选择行为需人工确认。
- **双击**：CDP 双击通过；真实鼠标双击的时序与焦点需人工确认。
- **多显示器/跨屏 DPI**：未测试；当前为单显示器 150% 环境。
- **其它 DPI**：125%/175%/200% 未验证。
- **边缘缩放**：Tauri undecorated resize 保留；未自动化拖拽。
- **macOS/Linux**：未验证；`window_chrome.rs` 非 Windows 为空实现，窗口控制仍使用 Tauri API。
- **生产 CSP/安装包**：未用 `tauri build` 成品运行验证。
- **前端 bundle**：约 796.6 kB（gzip 238.6 kB），仍为单 chunk 并触发 >500 kB 提示；本轮未扩展为构建拆分。
- **本地测试数据**：`Phase 3.5 UI 审阅` 等项目为测试数据，可在项目页归档；未删除用户数据。
- **视觉验收**：`VISUAL_REVIEW_PENDING`；未获得用户截图前不得声称 UI 已通过。

## 9. 签收

- 自动化检查：**PASS**
- 桌面窗口/尺寸/主题测量：**PASS（已测项）**
- 真实业务交互回归：**PASS**
- 人工视觉复核：**PENDING**
- 最终 UI 状态：**VISUAL_REVIEW_PENDING**

## 13. Phase 4.2 实机复核（2026-11-09）

| 验收范围 | 结果 | 证据 |
| --- | --- | --- |
| 安装预览 Dialog | **PASS** | 500×480 下只滚动正文，授权说明、复选框、取消、关闭可达。未授权或预览受阻时原生禁用，切换授权后状态正确。接收命令换行且只显示预期的本机参数 |
| 活动 Sheet 真实数据 | **PASS** | 读取 Phase 4.1 的证据副本，4 个会话、18 条事件、4 次工具调用。失败带文字说明，Stop 显示一轮回答结束，两次刷新记录不变 |
| 空态、异常与长列表 | **PASS（隔离样本）** | 空态和配置异常经桌面复测，活动读取错误和重试由前端回归覆盖。5000 条输入按限额显示 100 条，64 字符工具名换行，正文无横向溢出 |
| 键盘与语义 | **PASS（已测项）** | Tab 和 Shift+Tab 焦点循环，Enter 和 Esc，Space 授权，PageDown 和 End 滚动。焦点返回实际打开按钮。浏览器无障碍树识别授权复选框 |
| 主题、尺寸和对比度 | **PASS（程序化）** | 浅色、深色、跟随系统，1366×768、1600×900、1920×1080、500×480。原生 150% 和 WebView 100% 仿真。抽测文字令牌对比度达 4.5:1，44 CSS px 工作带，无页面级横向溢出 |
| E2E 数据隔离 | **PASS（当前入口）** | 历史普通库中有 6 个旧测试项目和 9 个关联任务。新入口的测试写入在独立数据目录，普通库和 WAL 指纹一致。历史记录保留 |
| 人工视觉与 Snap | **PENDING** | 最大化覆盖层命中 `HTMAXBUTTON=9`，Snap 菜单未可靠观测。原生 100% DPI、实际屏幕阅读器及截图复核待用户确认 |

本轮应用 `frontend-design` Skill，沿用 v2.2 的表面、边框、语义色和焦点令牌。视觉复核优先看深浅主题的弹层层级、失败状态、禁用和悬停观感、窄窗换行与滚动条。真实事件来源、回归命令及退出码见 [Phase 4.2 验收报告](../phases/PHASE_4_2_FINAL_AUDIT.md)。
- 未获得用户截图确认前，不得声称 UI 已完美通过。
- 本轮未创建 Git 提交；等待用户审查。

## 10. 设计计划自检记录

- 已调用 `frontend-design` Skill（当前会话，2026-11-06），未使用子 Agent。
- 方案避开通用 AI 页面默认：没有暖米色+衬线+陶土色、黑底酸性绿、全卡片 SaaS 套件、装饰性全大写 eyebrow。
- 识别点集中在“证据工作台”结构：窗口顶端工作带、项目上下文、Status 序列、Command Terminal、证据/历史 Sheet。
- 窗口视觉语言为“一个工作带 + 一个侧栏 + 一个主滚动区”；没有第二层标题栏。
- 工作台首屏改为工作流顺序，而不是管理后台标题。
- 色彩、字体、间距、圆角、阴影、动效与状态均有语义 Token 约束。
- 视觉验收保留给用户手动截图。

## 11. Phase 3.6 视觉审查后精修（2026-11-06）

### 11.1 范围与设计自检

- 已实际读取并应用已安装的 `frontend-design` Skill，先复核设计系统与现有真实页面；由当前主会话独立完成，无子 Agent。
- 短设计方案：保留 `workspace #F8FAF9`、`sidebar #E6EDEA`、`foreground #172427`、`muted-foreground #5C6F73`、`primary #0F766E` 与现有深色映射；UI 继续使用 Segoe UI/微软雅黑，分支、Commit、命令、退出码与修订使用 Cascadia/Consolas。
- 布局：`项目选择 / Git 元数据 | 页面名称 | 主题 / 窗口控制`；内容维持 `SignalRail → Focus Panel → 最近验证 → 其余待处理/最近任务`，文字左对齐。品牌识别集中在 Focus Panel 的青色左边线；无新卡片、渐变或装饰动画。
- 方案复核后采用 `surface-raised` 替代整块强调色底，以减少「下一步」的视觉重量；任务使用紧凑扫描行，验证使用两层记录，历史保留命令/修订/证据结构，不套同一模板。
- 顶部只调整内容排版与响应式显示优先级；44px 工作带、46×44px 窗口按钮、拖动属性、权限、自绘窗口与 Snap Overlay 实现均未改动。

### 11.2 本轮修改与业务边界

- `workspace-chrome.tsx` / `project-git-summary.tsx`：项目名、页面名称、分支、Commit 降噪分层；选择器与项目详情入口有统一悬停/焦点反馈；Git 工作区状态有图标与中文。
- `overview-page.tsx`：紧凑 Focus Panel；焦点任务不再在「需要处理」完整重复，余下数量与空态明确指向上方；保留原任务筛选、排序、采样范围和所有主操作。最近任务约 40px 行高；验证记录同时展示退出码与耗时，任务与运行时间分层。
- `status-badge.tsx` / `task-table.tsx`：复用同一状态映射的行内形式，继续提供图标、中文与颜色；详情/焦点 Badge 保留。
- `verification-history-sheet.tsx`：命令与修订字号提升至现有 11px Token，时间使用 `time` 与等宽数字；证据入口至少 28px，可键盘访问。历史事实、原因、证据与脱敏提示保留。
- `section.tsx`：支持原生 section 属性；焦点、待处理、最近验证提供具名区域，方便辅助技术导航与行为回归。
- `App.test.tsx`：新增 3 个行为测试，验证单焦点不重复、不误报无阻塞、剩余任务/原因保留，以及验证结果、退出码、耗时和对应任务入口。
- 桌面回归确认：已完成任务不在待验收队列，因此最近验证保留打开对应任务详情的原有行为；实际打开已完成任务的历史及证据成功，不会跳到队列中另一任务。
- 本轮未修改 Rust、SQLite 迁移、IPC、验证引擎、完成门禁、依赖或全局配置；现有未提交工作保留。未创建 Git 提交，未进入 Phase 4。

### 11.3 无截图桌面回归

- 使用本地 Vite + 实际 Tauri/WebView2 桌面应用与既有 `Phase 3.5 UI 审阅` 数据，未创建或删除业务记录；通过实际 IPC 比对回归前后项目、任务、条件、历史与验证对象，保持一致。
- 自然 150% 环境：1366×768、1600×900、1920×1080、500×480 × 深浅主题 × 9 页面，共 72 个布局组合，无页面级横向溢出；顶部内容无重叠，工作带为 44px，三个窗口按钮均 46×44px。
- 100% CDP 仿真：1366×768、1600×900、1920×1080、500×480，以及 759/760、1100、1280px 断点工作台布局均通过；仿真不修改 Windows 全局缩放。此项不代替真实 100% DPI 或跨屏验收。
- 真实交互：焦点任务入口、完整阻塞原因、完成门禁、最近验证对应任务、验证历史、证据内容与摘要哈希、Escape 关闭均通过。项目选择器可用 Space 打开、Escape 关闭并返回焦点。
- 窗口回归：最小化、最大化/还原、CDP 双击切换、Snap `WM_NCHITTEST=HTMAXBUTTON(9)`、原生覆盖层点击切换均通过；最后通过关闭按钮退出桌面进程（退出确认 PASS）。
- 回归脚本首次在最小化后误取辅助窗口句柄；改为按进程及 Snap 子窗口定位主窗后通过，无需修改应用窗口代码。
- 额外长文案探针曾直接替换 React 管理的 DOM 文本，触发 `removeChild / NotFoundError`；该探针方法无效，结果作废并从最终脚本移除。重载应用后，真实项目选择器的 Space/Escape、焦点返回及关闭退出回归通过；未将此探针算作长文本验收。关闭过程开发日志另有一次 Tauri custom protocol 回退提示，postMessage 回退后桌面进程正常退出；生产包验证仍待进行。
- 测试脚本与 JSON 结果保存在已忽略的 `target/phase3.6-audit/`，没有截图、录屏或测试专用产品代码。

### 11.4 保留的人工验收事项

- [ ] 精修后的浅色/深色截图复核：顶部信息层级、Focus Panel 重量、任务/验证记录密度、悬停/键盘焦点、长文本换行。
- [ ] 极长项目名、分支名与阻塞原因使用真实数据的边界复核（代码具备截断/title 或完整换行，本轮未伪造持久化记录）。
- [ ] 系统 Snap 悬停菜单的外观与实际选择半屏/三栏。
- [ ] 实机鼠标拖动、双击时序、边缘缩放及窗口失焦反馈。
- [ ] 多显示器、跨屏 DPI、125%/175%/200%、真实 100% 环境。
- [ ] 安装包/生产 CSP、macOS/Linux（沿用原未验证范围）。

用户提供的浅色截图已用于确认原 Phase 3.6 窗口方案；本轮精修未进行新的截图验收。以上事项仍为 PENDING，最终状态保持 **VISUAL_REVIEW_PENDING**。

## 12. Phase 4 Claude Code 接入 UI 审计（2026-11-09）

### 12.1 范围与设计自检

- 已实际调用 `frontend-design` Skill（当前主会话，2026-11-09），未使用子 Agent，未自动截图/录屏。
- 复用 `docs/design/DESIGN_SYSTEM.md` v2.2：`workspace`/`sidebar`/`line`/`primary`/`warning`/`success`/`destructive`/`info`/`code-*` Tokens；UI 字体与 Cascadia/Consolas 等宽字体未变。
- 保持 44px 顶部工作带、240px/52px 侧栏、自定义窗口控制与 Snap Overlay；未新增一级导航，未重构既有页面。
- 设计方向：接入是项目详情中的一条“证据传感器” strip，而不是 KPI 卡片墙；视觉重量低于任务主列。状态用图标 + 中文 + 颜色；事件时间线是实际时间序列，不使用装饰性编号或全大写标签。

### 12.2 新增页面与组件

| 组件/位置 | 说明 | 自动化 | 人工视觉 |
|---|---|---|---|
| 项目详情“Claude Code 接入” | 状态、原因、最近接收、待导入、未结束工具、最近 4 条事件、主操作/更多菜单 | PASS（Vitest + 桌面冒烟） | PENDING |
| `ClaudeInstallDialog` | 写入位置、6 个 Hook 表、命令 details、数据边界、前置确认、复选框授权 | PASS（Vitest：未勾选禁用、安装调用） | PENDING |
| `ClaudeActivitySheet` | 最近会话、未结束/未确认工具、事件时间线、刷新 | PASS（Vitest：执行中/成功/失败/状态未知） | PENDING |
| `ClaudeStateBadge` / `ClaudeEventStatus` / `ClaudeSessionState` | 接入状态、事件状态、会话状态；图标 + 文字 + 语义色 | PASS（Vitest） | PENDING |
| 安装/停用/卸载 | 预览 token、确认对话框、错误 Toast；停用/卸载二次确认 | PASS（Vitest + Rust 19 项） | PENDING |

### 12.3 状态表达检查

- [x] 接入状态使用 `ClaudeStateBadge`：未配置/已停用/已启用/部分安装/需要修复/配置异常/接收器不可用/项目不可用/状态不一致。
- [x] 事件使用图标 + 中文 + 颜色区分执行中、成功、失败、状态未知；running 才有旋转图标。
- [x] 会话结束缺失显示“结束未确认”，不显示成功。
- [x] 配置异常/需要修复/导入错误显示原因与修复入口，不隐藏 blockers。
- [x] 接入区块明确“事件不代表验收通过”，不修改任务完成状态。
- [x] 没有事件时使用 plain EmptyState；不构造演示会话或 KPI。
- [x] 安装按钮在阅读预览并勾选确认前禁用；停用/卸载有二次确认。

### 12.4 无障碍与布局检查（代码/桌面）

- [x] 接入 Section 有 `aria-label="Claude Code 接入"`，成为可导航 region。
- [x] 复选框与说明使用 label 关联；按钮均有可读名称；更多菜单有 `aria-label`。
- [x] Dialog/Sheet 使用 Radix：焦点陷阱、Escape、关闭后返回焦点。
- [x] 事件时间使用 `<time dateTime>` 与 mono；状态不只靠颜色。
- [x] 1366×768 与 500×480 仿真下接入 Section/main 无横向溢出；页面级主滚动条仍在 1366 下无溢出。
- [x] 桌面冒烟回到工作台，“下一步”与“最近验证”仍正常渲染。
- [ ] 浅色/深色截图复核接入 strip、预览 Dialog、活动 Sheet、长文本与窄窗口：待用户提供。
- [ ] 真实键盘 Tab 顺序与焦点返回：代码层使用 Radix，待人工复核。

### 12.5 当前自动化结果

| 命令 | 退出码 | 结果 |
|---|---|---|
| `pnpm typecheck` | 0 | `tsc --noEmit` 通过 |
| `pnpm lint` | 0 | ESLint 10，`--max-warnings 0` |
| `pnpm test` | 0 | 8 个测试文件、48 个测试通过（Phase 4 新增 5 个接入/活动/配置异常/停用/空状态测试） |
| `pnpm build` | 0 | Vite 构建通过；JS 827.13 kB / gzip 246.40 kB（原有 >500 kB 提示） |
| `pnpm format` | 0 | Prettier 通过 |
| `cargo fmt --all -- --check` | 0 | rustfmt 通过 |
| `cargo check --workspace --all-targets --locked` | 0 | Rust workspace 通过 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | 无警告 |
| `cargo test --workspace` | 0 | 100 项通过（含 19 个 Claude 接入、6 个 CLI、5 个协议） |
| 桌面无截图冒烟 | 0 | 接入区块可见；预览含 Hook 表；安装按钮未勾选时禁用；1366/500 无页面级横向溢出；工作台回归；未安装真实项目 Hooks |

### 12.6 待人工视觉复核

1. 项目详情（未配置）浅色/深色：strip 与仓库 strip 的层级、主操作位置。
2. 安装预览 Dialog：六个 Hook 表、命令 details、数据边界两列、复选框与禁用态、长文本换行。
3. 配置异常/需要修复：Alert 与修复入口是否清楚、未弱化原因。
4. 已启用 + 最近活动：事件行密度、状态颜色、待导入/未结束提示。
5. 活动 Sheet：会话列表、未结束工具、时间线、空状态、滚动与关闭焦点。
6. 500×480：strip 操作按钮、Dialog、Sheet 是否可操作且无页面级横向滚动。
7. 深色主题下 `code-*` 命令块、Badge、Alert 与滚动条。
8. 键盘：Tab 到安装按钮/复选框、Escape 关闭、关闭后焦点返回。

### 12.7 结论

- 自动化与技术检查：**PASS**
- 桌面无截图冒烟：**PASS（已测项）**
- 人工视觉复核：**PENDING**
- 真实 Claude Code 联调：**NOT_RUN**（需用户授权的测试项目，步骤见 `docs/architecture/CLAUDE_HOOKS.md` 第 8 节）
- 最终 UI 状态：**VISUAL_REVIEW_PENDING**

## 14. Phase 5 记忆、决策与恢复 UI 审计（2026-11-12）

### 14.1 范围与设计自检

- 已实际调用 `frontend-design` Skill（当前主会话，2026-11-12），未使用子 Agent，未自动截图/录屏。
- 复用 `docs/design/DESIGN_SYSTEM.md` v2.3 的 `workspace`/`sidebar`/`line`/`primary`/`warning`/`success`/`destructive`/`info`/`code-*` Tokens；未新增全局配色或字体。
- 保持 44px 顶部工作带、240/52px 侧栏、自定义窗口控制与 Snap Overlay；新增一级“恢复”导航与项目详情子页面“记忆与决策”。
- 设计方向：记忆/决策是“开发上下文日志”，不是 KPI 或卡片墙；恢复页是“可信交接带”，识别点是来源指纹、新鲜度与下一步，而不是装饰。
- 恢复包明确区分事实/记忆/决策/建议；过期与历史快照均有警告；复制入口不自动执行。

### 14.2 新增页面与组件

| 页面/组件 | 说明 | 自动化 | 人工视觉 |
|---|---|---|---|
| 项目记忆与决策 `/projects/:id/knowledge` | Tabs、类别/状态筛选、包含归档、日志行、关联任务筛选条 | PASS（Vitest + 桌面 E2E 文本可见） | PENDING |
| 记忆详情/编辑 | 760px Sheet、修订历史、确认/归档/恢复；Dialog 表单 | PASS（Vitest 打开详情 + 创建记忆） | PENDING |
| 决策详情/编辑/取代 | Sheet 展示问题/背景/备选/最终方案/理由/风险；草稿编辑、确认、废弃、创建取代 | PASS（Vitest 详情/创建草稿 + Rust 决策测试） | PENDING |
| 恢复 `/recovery` | 恢复带、下一步 Focus Panel、双列摘要、来源 details | PASS（Vitest + 桌面 E2E） | PENDING |
| 完整恢复指令 Sheet | 边界提示、深色 Markdown、复制、过期/历史警告 | PASS（Vitest 剪贴板 + 历史快照） | PENDING |
| 历史快照 Sheet | 最近 20 份、当前/历史、查看加载态 | PASS（Vitest） | PENDING |
| 项目详情“项目记忆与决策”strip | 计数 + 最近记忆/决策 + 入口 | PASS（Vitest 项目详情回归） | PENDING |
| 任务详情“相关决策与记忆” | 最多 3 条记忆/决策 + 项目上下文入口 | PASS（Vitest 任务详情回归） | PENDING |

### 14.3 状态表达检查

- [x] 记忆确认/未确认/归档使用图标 + 中文 + 颜色。
- [x] 决策草稿/已确认/已被取代/已废弃使用图标 + 中文 + 颜色。
- [x] 未确认记忆与已确认记忆分开展示，明确“不是事实”。
- [x] 草稿决策明确“不会进入恢复包”；已被取代/废弃决策不作为当前约束。
- [x] 恢复快照当前/过期/未生成使用 Badge；过期时列出变化来源。
- [x] 历史快照明确“不代表当前状态”；复制历史/过期快照时剪贴板文本带警告。
- [x] 验证结果写“按生成时状态”，不把过期结果写成“当前测试通过”。
- [x] 记忆/决策/Hook 事件不显示为验收证据；完成门禁不变。
- [x] 无快照时使用 plain EmptyState，不构造演示恢复内容。
- [x] 复制恢复指令是用户主动点击，不自动注入 Claude Code。

### 14.4 无障碍与布局检查（代码/桌面）

- [x] 记忆/决策行标题是按钮，可键盘打开详情；详情 Sheet 使用 Radix 焦点陷阱与 Escape。
- [x] 详情 Sheet Footer 使用 `flex-wrap`，窄窗口操作按钮不隐藏、不溢出。
- [x] Dialog 使用 RHF + Zod，Label 关联、`aria-invalid`、字段错误 `role="alert"`。
- [x] 恢复页“下一步”区域有 `aria-label="下一步"`；任务选择有可访问名称。
- [x] 恢复页按钮在 500px 下换行；500×480、1366×768、1600×900 共 9 组布局测量无页面/主区横向溢出，工作带 44px，侧栏 52/240px。
- [x] 数据来源使用 `<details>`，summary 有可见焦点环。
- [x] 历史快照查看显示 `Loader2` 加载态，不显示伪造结果。
- [ ] 浅色/深色截图复核知识页、恢复页、Dialog/Sheet、长文本与窄窗：待用户提供。
- [ ] 真实键盘 Tab 顺序、焦点返回与 500×480 实际窗口：待人工复核。
- [ ] 1366×768、1600×900 下恢复页双列与知识页长内容：程序化布局测量通过，待人工截图复核视觉层级。

### 14.5 当前自动化与桌面结果

| 命令 | 退出码 | 结果 |
|---|---|---|
| `pnpm typecheck` | 0 | `tsc --noEmit` 通过 |
| `pnpm lint` | 0 | ESLint 10，`--max-warnings 0` |
| `pnpm test` | 0 | 9 个测试文件、66 个测试通过（Phase 5 新增 12 个测试） |
| `pnpm build` | 0 | Vite 通过；JS 917.84 kB / gzip 266.03 kB（原有 >500 kB 提示） |
| `pnpm format` | 0 | Prettier 通过 |
| `cargo fmt --all -- --check` | 0 | rustfmt 通过 |
| `cargo check --workspace --all-targets --locked` | 0 | Rust workspace 通过 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | 无警告 |
| `cargo test --workspace --locked` | 0 | 152 项通过（含 30 项 Phase 5 新增领域/恢复/迁移/重启测试） |
| 隔离桌面 E2E | 0 | 记忆/决策/快照 IPC、详情 Sheet、fresh → stale → fresh、任务状态不变、知识页/恢复页文本可见；正常数据库指纹不变 |
| 隔离桌面布局测量 | 0 | 500×480、1366×768、1600×900 × 知识页/恢复页共 9 组；工作带 44px、侧栏 52/240px、无页面/主区横向溢出 |

> E2E 证据：`target/phase5-audit/phase5-e2e-result.json`、`target/phase5-audit/phase5-layout.json`（已忽略目录）。未自动截图/录屏。

### 14.6 待人工视觉复核

1. 项目详情“项目记忆与决策”strip 与既有 Claude 接入 strip 的层级是否清晰。
2. 知识页记忆/决策 Tabs、筛选工具栏、日志行密度、Badge 与关联任务。
3. 记忆/决策详情 Sheet 与编辑 Dialog：长文本、滚动、Footer 换行、焦点返回。
4. 决策取代流程：草稿选择“取代已确认决策”、确认后的双向提示。
5. 恢复页恢复带、下一步 Focus Panel、双列摘要、来源 details。
6. 完整恢复指令 Sheet：深色 Markdown、过期/历史警告、复制按钮。
7. 历史快照 Sheet：当前/历史标记、加载态、关闭焦点。
8. 500×480、1366×768、1600×900、深浅主题、150% 缩放下的实际窗口截图。
9. 键盘：Tab 到新建/查看/确认/复制/历史、Escape 关闭、焦点返回。

### 14.7 结论

- 自动化与技术检查：**PASS**
- 隔离桌面 E2E（已测项）：**PASS**
- 人工视觉复核：**PENDING**
- 最终 UI 状态：**VISUAL_REVIEW_PENDING**
- 未获得用户截图前，不得声称 Phase 5 UI 已通过人工视觉验收。

## Phase 6 数据保护页审计（2026-11-13）

- 已调用 `frontend-design` Skill（当前主会话），未使用子 Agent，未自动截图/录屏。
- 页面：`/data-protection`；状态带、备份日志、恢复历史、完整性报告、隐私资产清单、数据目录。
- 复用：WorkspaceChrome、240/52px 侧栏、Section、Dialog、Sheet、ConfirmationDialog、EmptyState、LoadingState、ErrorState、Alert、Badge、Button、Input；未新增全局配色或字体。
- 状态：备份完整性/来源、完整性检查项、恢复结果均为图标 + 中文 + 颜色；长路径/哈希/Schema 使用 mono。
- 交互：创建备份、策略保存、校验、删除备份、恢复向导；恢复确认要求输入“恢复”+勾选；执行中不可关闭；结果区分完成/回退/回退失败。
- 自动化：
  - Vitest 11 个文件、83 项通过；数据保护页测试覆盖创建、校验、完整性检查、恢复确认门禁、预检失败、启动提示。
  - 隔离 Tauri E2E：备份 → 校验 → 修改 → 预检 → 损坏备份拒绝 → 正式恢复 → 最终校验 → 关闭重开；数据保护页/恢复向导可见。
  - 布局测量：500×480、1366×768、1600×900、1920×1080，浅色/深色；顶部工作带 44px；侧栏 52/240px；无页面级/主区横向溢出；可见按钮均 ≥20px；500×480 恢复向导宽 460px 且正文可滚动。
- 证据：`target/phase6-audit/phase6-e2e-result.json`、`phase6-reopen-result.json`、`phase6-layout.json`（已忽略目录）。
- 人工视觉：**VISUAL_REVIEW_PENDING**。创建/策略 Dialog、校验 Sheet、恢复向导、隐私清单、150% 缩放、键盘焦点与真实剪贴板仍需用户手动截图/操作确认。
- 未改变窗口控制、44px 工作带、Snap Overlay、侧栏宽度或其他页面结构；未新增 capability。

## Phase 7A 安装版审计（2026-11-14）

- 未新增或重排页面、组件、配色、字体、间距或窗口结构；未调用 `frontend-design` 重做设计。唯一可见变化是设置页“阶段”值由 Rust `get_app_info` 从 “Phase 6” 更新为 “Phase 7A”。
- 安装版 E2E（隔离 identifier，真实 NSIS 安装）测量：
  - 1100×720 窗口：工作带 44px、侧栏 240px、无横向溢出；数据保护页“数据库备份/数据完整性/Hook 待导入 Spool”可见。
  - 500×480 窗口：工作带 44px、侧栏 52px、无横向溢出；`data-tauri-drag-region` 存在。
  - 窗口最大化 → 还原 → 最小化 → 关闭均由真实安装版 IPC/原生状态验证；Snap Overlay 子窗口存在且 `WM_NCHITTEST` 返回 `HTMAXBUTTON (9)`。
  - 当前环境 DPI 144、显示器 1 台。
- 未自动截图/录屏。Windows Snap 实际悬停菜单、标题栏拖拽手感、多显示器、其他 DPI 与 150% 缩放的视觉检查仍为 **VISUAL_REVIEW_PENDING**；不得以自动化测量代替人工视觉签收。
- 证据：`target/phase7a/installed-full-result.json`、`installed-window-result.json`、`installed-e2e-report.json`（已忽略目录）。
