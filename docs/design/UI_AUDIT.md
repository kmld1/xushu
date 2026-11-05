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
| 任务详情 Task Detail | `/tasks/:id` | 已实现 | StateTrack、验收条件、验证配置/运行、完成门禁、StateSpine 历史 | PASS（Vitest + 桌面 E2E） | PENDING |
| 验收中心 Verification | `/verification` | 已实现 | 左侧待验收队列、右侧条件/命令/批准/运行/证据/历史/报告 | PASS（Vitest 空态 + 桌面 E2E） | PENDING |
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
| VerificationBadge | `src/components/status-badge.tsx` | 验收中心、任务详情、验证历史 | 已实现 |
| CriterionVerification | `src/features/verification/criterion-verification.tsx` | 验收中心、任务详情 | 已实现 |
| VerificationConfigDialog | `src/features/verification/verification-config-dialog.tsx` | 自动条件配置 | 已实现 |
| VerificationApprovalDialog | `src/features/verification/verification-approval-dialog.tsx` | 命令预览、风险提示、批准 | 已实现 |
| VerificationHistoryDialog | `src/features/verification/verification-history-dialog.tsx` | 运行历史 | 已实现 |
| EvidenceDialog | `src/features/verification/evidence-dialog.tsx` | 证据详情、脱敏输出 | 已实现 |
| AcceptanceReportDialog | `src/pages/verification-page.tsx` | 报告生成与复制 | 已实现 |

## 3. 状态表达检查

- [x] draft / ready / running / awaiting_verification / done / blocked / cancelled
      均使用图标 + 中文 + 颜色，不只依赖颜色。
- [x] 阻塞任务显示 `blockReason`（工作台与任务详情）。
- [x] 完成门禁显示 required/satisfied 数量与具体 blockers。
- [x] PASS / FAIL / NOT_RUN / INCONCLUSIVE / STALE 已接入 `VerificationBadge`，均使用图标 + 中文 + 颜色。
- [x] 自动条件显示当前有效状态、过期原因、命令、cwd、退出码、耗时与证据入口。
- [x] 运行中的验证显示旋转图标、状态文字与取消按钮，不只靠颜色。
- [x] 批准对话框展示程序、参数、cwd、超时、输出上限、成功规则与风险提示，并要求复选框确认。
- [x] 不把 AI 声明或前端状态当作完成证据；完成由 Rust 门禁校验。
- [x] 证据输出标注“模式化脱敏、非 100% 保证”，并展示内容摘要哈希。
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
| `pnpm test` | 0 | 7 个测试文件、40 个测试通过 |
| `pnpm build` | 0 | Vite 构建通过；JS 757.52 kB / gzip 229.36 kB |
| `pnpm format` | 0 | Prettier 检查通过 |
| `cargo test --workspace` | 0 | 11 lib + 19 business + 12 persistence + 26 verification |
| 桌面验证 E2E | 0 | 配置 → 批准 → 运行 → PASS → 完成 → 重启后仍为 done，证据可读 |

## 6. 桌面业务流程 E2E

- 环境：Windows 11、WebView2 156.0.4314.58、1100×720 窗口。
- 方法：通过 WebView2 CDP 驱动真实窗口与真实 Tauri IPC；临时脚本位于
  `target/e2e/`（被 `.gitignore` 排除，未提交），未使用截图/录屏。
- 第一轮（tag `20261106015932`）：导入临时 Git 项目 → 创建任务 → 添加必需自动条件 →
  ready → running → awaiting_verification → 配置 `git --version` → 在批准对话框勾选确认 →
  运行 → PASS → 标记完成 → 应用重建/重启 → 再次读取。
  - 结果：`statusDone=true`、`verificationPass=true`、`completionSatisfied=true`；
    证据摘要、哈希、修订号与指纹在重启后仍可读。
  - 该轮在“证据摘要自然化”修复前执行，历史证据保留旧文案 `Some(0)`；未改写历史。
- 第二轮（tag `20261106021137`，修复后二进制）：重复完整流程，证据摘要为
  “验证程序退出码 0，耗时 66 ms”，且 `summary_has_debug_artifact=false`。
- 第三轮（tag `20261106022214`，cwd 预检与进程清理加固后）：完整 smoke 通过，
  证据摘要为“验证程序退出码 0，耗时 68 ms”。
- 第四轮（tag `20261106023630`，最终复核）：完整 smoke 通过，证据摘要为
  “验证程序退出码 0，耗时 45 ms”；真实 IPC 导出的报告包含命令、工作目录、
  pass 与分享前检查提示。
- 四轮测试项目均在结束后通过 IPC 软归档；任务/运行/检查/证据保留在本地数据库。
- 结果文件：`target/e2e/e2e-result-full.json`、`target/e2e/e2e-result-verify.json`（未提交）。
- 未截图/录屏；视觉验收仍待人工截图。

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
9. **验收中心（空队列）**：空状态与“查看任务”入口。
10. **验收中心（有待验收任务）**：左侧队列、自动/人工计数、右侧条件面板。
11. **验证配置 Dialog**：模板选择、程序、参数、cwd、超时、输出上限、风险提示。
12. **批准 Dialog**：命令预览、解析路径、超时、输出上限、成功规则、复选框与禁用态。
13. **运行中**：旋转图标、状态文字、取消按钮；取消后的结果。
14. **证据 Dialog**：脱敏标记、内容摘要哈希、长输出滚动、深色主题。
15. **验证历史**：运行状态、结果、退出码、耗时、证据入口。
16. **过期证据**：STALE Badge、过期原因与重新运行入口。
17. **报告 Dialog**：文件名、Markdown 预览、复制按钮。
18. **本地数据与健康**：健康数据、探针表格、WAL/外键/schema。
19. **设置**：主题切换、关于、本地数据说明。
20. **窗口缩放**：900px 与 1400px 宽度；检查侧栏、内容、表格、对话框、Toast。

建议同时检查：

- 深色主题；
- 错误/空状态；
- 任务详情“待验证”且显示未通过原因；
- 长项目路径、长任务标题、长命令与长输出不撑破布局。

## 8. 已知风险与待确认

- 本阶段未自动截图；视觉一致性无法由自动化断言完全证明。
- 中文字体使用 Windows 原生 Segoe UI Variable / Microsoft YaHei UI；非 Windows 环境可能回退。
- 前端构建为单 chunk 757.52 kB（gzip 229.36 kB），Vite 提示 >500 kB；后续可路由级拆包。
- 表格在极窄窗口下允许横向滚动；需人工确认没有页面级溢出。
- 自动验证首版拒绝 `.cmd/.bat/.ps1` 包装器；pnpm/npm 在 Windows 上的批处理 shim 不能直接作为验证程序，需配置底层 `.exe`。
- 验证输出脱敏是模式化尽力而为，不能保证识别所有密钥格式；证据属于本地敏感数据。
- 验收报告可能包含本地命令、工作目录与脱敏输出；UI 与报告文末均提示分享前检查。
- 生产 CSP 尚未用打包产物实机验证。
- 项目/任务归档为软归档；四轮 E2E 测试项目已归档，数据保留在本地数据库。

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