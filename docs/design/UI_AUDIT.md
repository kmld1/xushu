# XUSHU UI Audit

> 状态：**VISUAL_REVIEW_PENDING**（自动化检查通过；人工视觉验收需用户提供截图）
> 关联规范：`docs/design/DESIGN_SYSTEM.md`
> 更新规则：每次新增或修改页面前阅读设计系统；完成后更新本文件。
> 设计方法：已调用 `frontend-design` Skill（当前主会话，2026-11-06），未使用子 Agent。

## 1. 页面清单

| 页面 | 路由 | 状态 | 设计要点 | 自动化检查 | 人工视觉 |
|---|---|---|---|---|---|
| 工作台 Dashboard | `/` | 已实现 | Signal Rail、当前项目、需要关注、最近任务、系统健康 | PASS（Vitest） | PENDING |
| 项目 Projects | `/projects` | 已实现 | 行式项目列表、当前标记、Git 状态、导入/归档/恢复 | PASS（Vitest） | PENDING |
| 项目详情 Project Detail | `/projects/:id` | 已实现 | Git 状态、项目元数据、项目任务 | PASS（Vitest） | PENDING |
| 任务 Tasks | `/tasks` | 已实现 | 项目/状态/搜索筛选、任务表格、新建任务 | PASS（Vitest） | PENDING |
| 任务详情 Task Detail | `/tasks/:id` | 已实现 | StateTrack、验收条件、完成门禁、StateSpine 历史 | PASS（Vitest + 桌面 E2E） | PENDING |
| 本地数据与健康 | `/diagnostics` | 已实现 | 系统健康 + Phase 1 探针 | PASS（Vitest） | PENDING |
| 设置 Settings | `/settings` | 已实现 | 外观、关于、本地数据与隐私 | PASS（Vitest） | PENDING |
| 404 | `*` | 已实现 | 统一空状态与返回路径 | PASS（构建） | PENDING |

## 2. 组件复用清单

| 组件 | 位置 | 使用页面 | 状态 |
|---|---|---|---|
| App Shell / Sidebar / Topbar | `src/app/app-layout.tsx` | 全部 | 已实现 |
| PageHeader | `src/components/page-header.tsx` | 全部一级页面 | 已实现 |
| Panel / Card | `src/components/ui/card.tsx` | Dashboard、Projects、Tasks、Detail | 已实现 |
| Button | `src/components/ui/button.tsx` | 全部 | 已实现 |
| Input / Textarea / Select | `src/components/ui/*` | 表单/筛选 | 已实现 |
| Dialog / ConfirmationDialog | `src/components/ui/dialog.tsx`、`components/confirmation-dialog.tsx` | 导入、创建、编辑、归档 | 已实现 |
| DropdownMenu | `src/components/ui/dropdown-menu.tsx` | 项目行操作 | 已实现 |
| Tabs | `src/components/ui/tabs.tsx` | 项目活动/归档 | 已实现 |
| Table | `src/components/ui/table.tsx` | 任务列表、诊断记录 | 已实现 |
| Badge / StatusBadge / PriorityBadge | `src/components/ui/badge.tsx`、`components/status-badge.tsx` | 全部状态 | 已实现 |
| Tooltip | `src/components/ui/tooltip.tsx` | 图标按钮 | 已接入 Provider，按需使用 |
| Toast | `sonner` + `components/ui/sonner.tsx` | 操作反馈 | 已实现 |
| EmptyState | `src/components/empty-state.tsx` | Dashboard、Projects、Tasks、诊断 | 已实现 |
| LoadingState | `src/components/loading-state.tsx` | 查询加载 | 已实现 |
| ErrorState | `src/components/error-state.tsx` | 查询错误 | 已实现 |
| StateTrack | `src/components/state-track.tsx` | 任务详情 | 已实现 |
| StateSpine | `src/components/state-spine.tsx` | 任务历史 | 已实现 |

## 3. 状态表达检查

- [x] draft / ready / running / awaiting_verification / done / blocked / cancelled
      均使用图标 + 中文 + 颜色，不只依赖颜色。
- [x] 阻塞任务显示 `blockReason`（工作台与任务详情）。
- [x] 完成门禁显示 required/satisfied 数量与具体 blockers。
- [x] 预留 PASS / FAIL / NOT_RUN / INCONCLUSIVE / STALE 的 Token 与 `VerificationBadge`。
- [x] 不把 AI 声明或前端状态当作完成证据；完成由 Rust 门禁校验。
- [x] 任务详情展示 StateTrack 与 StateSpine，状态变化可追溯。

## 4. 可访问性检查（代码层面）

- [x] 交互元素有 `focus-visible` ring。
- [x] Dialog 使用 Radix：焦点陷阱、Escape、关闭后返回焦点由 primitives 提供。
- [x] 表单 Label 关联、错误 `role="alert"`、必填/可选说明。
- [x] 表格有表头；行链接有可读名称。
- [x] 状态 Badge 有文字与图标，不只靠颜色。
- [x] 主题 radio 使用原生 input；Select 使用 Radix。
- [x] `prefers-reduced-motion` 在全局 CSS 中处理。
- [ ] 900px/1400px 窗口下无溢出与遮挡：待人工视觉确认。
- [ ] 深色/浅色对比度：待人工视觉确认（Token 目标 ≥ 4.5:1）。

## 5. 自动化检查记录

| 命令 | 退出码 | 结果 |
|---|---|---|
| `pnpm typecheck` | 0 | `tsc --noEmit` 通过 |
| `pnpm lint` | 0 | ESLint 10，`--max-warnings 0` |
| `pnpm test` | 0 | 7 个测试文件、39 个测试通过 |
| `pnpm build` | 0 | Vite 构建通过；JS 719.50 kB / gzip 220.27 kB |
| `pnpm format` | 0 | Prettier 检查通过 |
| `cargo test --workspace` | 0 | 4 lib + 19 business + 12 persistence |
| 桌面业务 E2E | 0 | 导入/建任务/验收/状态/确认/完成/重启读取全部通过 |

## 6. 桌面业务流程 E2E

- 环境：Windows 11、WebView2 156.0.4314.58、1500×1000 窗口。
- 流程：导入临时 Git 项目 → 创建任务 → 添加必需人工验收条件 → draft → ready →
  running → awaiting_verification → 人工确认 → done → 完全关闭 → 重启 → 读回。
- 结果：`statusDone=true`、`confirmed=true`、`completionSatisfied=true`；
  任务标题与验收条件在重启后仍存在。
- 证据：`C:\Users\kmld\AppData\Local\Temp\xushu-phase2-e2e\business-e2e-evidence.json`
  （临时目录，未提交）。
- 未截图/录屏。

## 7. 人工视觉复核清单

不自动截图。请用户按以下路径操作并提供截图（浅色/深色各一组）：

1. **工作台（无项目/有项目）**：Signal Rail、当前项目、需要关注、系统健康。
2. **项目导入 Dialog**：路径/名称字段、说明、按钮层级、Escape 关闭。
3. **项目列表**：当前项目标记、Git 状态、行操作菜单、归档确认。
4. **项目详情**：Git 状态、路径、项目任务。
5. **任务列表**：筛选栏、状态/优先级、空结果。
6. **任务详情（草稿）**：StateTrack、验收条件、完成门禁 blockers、状态操作。
7. **任务详情（待验证）**：人工确认、确认记录、完成按钮禁用原因。
8. **任务详情（已完成/阻塞/取消）**：状态视觉与操作可用性。
9. **本地数据与健康**：健康数据、探针表格、WAL/外键/schema。
10. **设置**：主题切换、关于、本地数据说明。
11. **窗口缩放**：900px 与 1400px 宽度；检查侧栏、内容、表格、对话框、Toast。

建议同时检查：

- 深色主题；
- 错误/空状态；
- 任务详情“待验证”且显示未通过原因；
- 长项目路径、长任务标题不撑破布局。

## 8. 已知风险与待确认

- 本阶段未自动截图；视觉一致性无法由自动化断言完全证明。
- 中文字体使用 Windows 原生 Segoe UI Variable / Microsoft YaHei UI；非 Windows 环境可能回退。
- 前端构建为单 chunk 719.50 kB（gzip 220.27 kB），Vite 提示 >500 kB；后续可路由级拆包。
- 表格在极窄窗口下允许横向滚动；需人工确认没有页面级溢出。
- 未来 Phase 3 的 PASS/FAIL/NOT_RUN/INCONCLUSIVE/STALE 仅预留 Token，尚无真实验证数据。
- 生产 CSP 尚未用打包产物实机验证。
- 项目/任务归档为软归档；测试项目会保留在本地数据库，可手动归档/恢复。

## 9. 签收

- 自动化检查：**PASS**
- 桌面业务 E2E：**PASS**
- 人工视觉复核：**PENDING**
- 最终 UI 状态：**VISUAL_REVIEW_PENDING**
- 未获得用户截图确认前，不得声称 UI 已完美通过。

## 10. 设计计划自检记录

- 已调用 `frontend-design` Skill（当前会话，2026-11-06），未使用子 Agent。
- 方案避开通用 AI 页面默认色板与组件堆叠；以“序列/证据”结构作为 XUSHU 识别点。
- 色彩、字体、间距、圆角、阴影、动效与状态均有 Token 约束。
- 页面统一使用 App Shell、PageHeader、Panel、StatusBadge、Dialog、Toast、Empty/Error/Loading 组件。
- 视觉验收保留给用户手动截图。