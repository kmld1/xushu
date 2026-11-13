# XUSHU Design System

> 版本：2.4（Phase 6 · 数据保护，2026-11-13）
> 适用范围：XUSHU 所有桌面端页面、组件、窗口与后续阶段。
> 设计方法：先读本文件 → 使用 `frontend-design` Skill → 复用 Tokens / App Shell / 共享组件 → 实现 → 质量检查 → 更新 `UI_AUDIT.md` → 人工视觉复核。
> 视觉状态：**VISUAL_REVIEW_PENDING**（技术检查通过，等待用户手动截图）。

## 1. 品牌定位

**XUSHU / 序枢** 是面向独立开发者的本地优先 AI 工作控制系统。它不是“AI 展示页”，而是一张**可信的证据工作台**：

- 当前项目与任务始终可见；
- 状态沿序列推进，证据可追溯；
- 自动验证、人工确认与 AI 声明在视觉上严格区分；
- 原生桌面行为优先于视觉炫技。

视觉关键词：

- **一体化**：工作区从窗口顶端开始生长；品牌、项目上下文、页面名称与窗口控制属于同一个 44px 工作带。
- **克制**：不做大面积渐变、玻璃拟态、发光边框或装饰性动画。
- **精密**：标题、元数据、状态与证据位置稳定可预测。
- **可信**：状态必须图标 + 中文 + 颜色；不把 AI 文本或前端状态当作证据。
- **紧凑**：为开发者桌面使用设计，而不是营销落地页。

### 1.1 视觉母题：Evidence Workbench / 证据工作台

XUSHU 的识别点不是卡片墙，而是：

- 左侧稳定的项目上下文与导航；
- 任务详情的 **State Track**（水平状态序列）与 **State Spine**（垂直历史）；
- 验收中心的 **Command Terminal**（深色命令块）；
- 证据、哈希、退出码与修订号使用等宽字体；
- 审核动作（批准、运行、取消、确认）沿内容自然出现，而不是散落在多个卡片。

### 1.2 反模式

禁止：

- 在自定义顶部工作带之外再保留原生标题栏，或在工作带下再放一条全宽应用信息栏；
- 每个区块都套相同的圆角卡片 / 阴影；
- 七种任务状态铺成七张等宽统计卡；
- 空状态套在大卡片里，再套一层背景；
- 每个标题都配彩色图标；
- 大量胶囊 Badge；
- 大面积渐变、玻璃拟态、夸张阴影；
- 为“现代感”加入无意义动画；
- 只靠颜色表达状态；
- 用假项目、假任务、假证据填充界面。

## 2. 设计原则

1. **原生行为优先。** 自定义标题栏只自绘外观；拖动、双击最大化/还原、最小化、最大化、关闭、边缘缩放、Snap Layouts、DPI 与焦点行为必须保持。任何原生行为明显退化时，回退到 ADR-0005 的原生标题栏方案。
2. **内容优先。** 先回答“我在做什么、什么需要处理、下一步是什么”，再考虑装饰。
3. **减少容器。** 优先使用分区、行、表格、分隔线与留白；Card 只用于独立交互容器或安全提示。
4. **一个强调色。** 氧化青只用于主操作、选中与运行中；黄铜用于等待/注意；珊瑚红用于失败/阻塞。
5. **证据可读。** 命令、cwd、退出码、哈希、修订号必须使用等宽字体，长内容可滚动或截断，但不可隐藏关键事实。
6. **状态可信。** 图标 + 中文 + 颜色三重表达；验证结果与执行状态分离。
7. **密度有呼吸。** 13px 正文、32–48px 行高、24–32px 页面边距；信息密集但不拥挤。
8. **不展示假数据。** 空状态明确邀请下一步操作。

## 3. Color Tokens

所有颜色以 CSS 变量定义在 `apps/desktop/src/index.css`，通过 Tailwind `@theme inline` 暴露。

### 3.1 Light

| Token | Hex | 用途 |
|---|---|---|
| `--background` | `#EEF3F1` | 窗口画布、最外层背景 |
| `--sidebar` | `#E6EDEA` | 左侧工作区侧栏 |
| `--workspace` | `#F8FAF9` | 主内容工作区、输入背景 |
| `--card` | `#F8FAF9` | 独立面板（少量使用） |
| `--popover` | `#FFFFFF` | Dialog / Sheet / Dropdown |
| `--foreground` | `#172427` | 主文字 |
| `--muted-foreground` | `#5C6F73` | 次要文字、元数据 |
| `--border` / `--line` | `#D2DEDA` | 分隔线、默认边框 |
| `--line-strong` | `#B6C6C2` | 输入边框、强调分隔 |
| `--primary` | `#0F766E` | 主操作、选中、运行中 |
| `--primary-foreground` | `#F4FFFC` | 主操作文字 |
| `--accent` | `#D8EBE7` | 选中背景、品牌浅底 |
| `--accent-foreground` | `#0B5F59` | 选中文字 |
| `--warning` | `#8A651F` | 待验证、注意、过期 |
| `--success` | `#1F7A4D` | 完成、通过 |
| `--destructive` | `#AE3F3F` | 阻塞、失败、危险操作 |
| `--info` | `#33618F` | 就绪、信息 |
| `--code-bg` | `#10191B` | 命令块 / 证据输出 |
| `--code-fg` | `#DCE8E5` | 命令与输出文字 |
| `--code-muted` | `#90A4A6` | 命令块标签、cwd |
| `--code-border` | `#2A3B3E` | 命令块边框 |
| `--window-control-hover` | `#DBE5E1` | 最小化/最大化按钮悬停底 |
| `--window-close-hover` | `#C42B1C` | 关闭按钮悬停底（Windows 语义） |

### 3.2 Dark

| Token | Hex | 用途 |
|---|---|---|
| `--background` | `#0A1012` | 窗口画布 |
| `--sidebar` | `#0D1618` | 左侧工作区侧栏 |
| `--workspace` | `#111A1C` | 主内容工作区 |
| `--card` | `#111A1C` | 独立面板 |
| `--popover` | `#162225` | Dialog / Sheet / Dropdown |
| `--foreground` | `#E3EBE9` | 主文字 |
| `--muted-foreground` | `#8DA1A4` | 次要文字 |
| `--border` / `--line` | `#223235` | 分隔线 |
| `--line-strong` | `#34484B` | 输入边框 |
| `--primary` | `#5FD0C2` | 主操作、选中、运行中 |
| `--primary-foreground` | `#06110F` | 主操作文字 |
| `--accent` | `#123B37` | 选中背景 |
| `--accent-foreground` | `#8FE0D6` | 选中文字 |
| `--warning` | `#D6AB61` | 待验证、注意 |
| `--success` | `#6FCE9C` | 完成、通过 |
| `--destructive` | `#E08585` | 阻塞、失败 |
| `--info` | `#8DB6E8` | 就绪、信息 |
| `--code-bg` | `#070D0E` | 命令块 / 证据输出 |
| `--code-fg` | `#DCE8E5` | 命令与输出文字 |
| `--code-muted` | `#82989B` | 命令块标签 |
| `--code-border` | `#223235` | 命令块边框 |
| `--window-control-hover` | `#1B2A2D` | 最小化/最大化按钮悬停底 |
| `--window-close-hover` | `#C42B1C` | 关闭按钮悬停底 |

### 3.3 使用原则

- 主操作、当前导航、运行中：`primary`。
- 等待验证、需要人工注意、证据过期：`warning`。
- 完成、有效 PASS：`success`。
- 阻塞、失败、删除：`destructive`。
- 就绪、信息：`info`。
- 命令/证据输出：`code-*` 深色终端块；浅色模式下也保持深色，以表达“将执行代码”。
- 窗口控制：悬停使用 `window-control-hover`；关闭按钮悬停使用 `window-close-hover` 与白色图标，符合 Windows 习惯。
- 正文目标对比度 ≥ 4.5:1；状态颜色必须配合文字或图标。
- 不允许页面硬编码另一套 hex 色值。

## 4. Surface Hierarchy

| 层级 | Token | 用途 |
|---|---|---|
| Window | `background` | 窗口最外层，显示在侧栏/工作区之外 |
| Window Chrome | `sidebar`（左）/ `workspace`（右） | 44px 顶部工作带；品牌、项目上下文、页面名称、主题、窗口控制 |
| Rail | `sidebar` | 240px 侧栏（<760px 收为 52px 图标轨）；导航 |
| Workspace | `workspace` | 主内容、输入框、选中行背景 |
| Panel | `card` | 独立表单/登录/安全提示；少量使用 |
| Raised | `surface-raised` | 行内提示、次级块、hover |
| Overlay | `popover` | Dialog、Sheet、Dropdown、Tooltip |
| Code | `code-bg` | 命令、证据日志、报告 Markdown |

规则：

- 顶部工作带的左侧品牌区与侧栏共用 `sidebar`，右侧与主内容共用 `workspace`；240px 竖线从窗口顶端贯通到底。
- 主内容与顶部工作带之间只保留一条 `line/60` 细线；左侧品牌区不画横线，保持侧栏连续。
- 主内容中优先用 `border-y` / `divide-y` 分隔区块，不为每个区块增加背景容器。
- Card 不作为默认页面分区；一个页面通常不超过 1–2 个 Card。
- Dialog / Sheet / 命令块可以使用更明显的边界，因为它们是独立交互或证据容器。

## 5. Typography

### 5.1 字体

| 角色 | 字体栈 |
|---|---|
| UI / 正文 | `"Segoe UI Variable Text", "Segoe UI", "Microsoft YaHei UI", "Microsoft YaHei", sans-serif` |
| 代码 / ID / 路径 / 哈希 | `"Cascadia Code", "Cascadia Mono", Consolas, monospace` |

- 不引入远程字体；Windows 使用系统字体，macOS 后续验证回退。
- 数字、计数、修订号使用 `tabular-nums`。
- 命令、cwd、Git HEAD、证据哈希、运行 ID 使用 mono。
- 长路径/长命令使用 `truncate` + `title` 或 `break-all`，不能撑破布局。

### 5.2 Type Scale

| Token | 大小 / 行高 | 用途 |
|---|---|---|
| `text-xs` | 11px / 1.5 | 元数据、标签、辅助说明 |
| `text-sm` | 13px / 1.55 | 控件、列表、正文 |
| `text-base` | 14px / 1.6 | 少量正文 |
| `text-lg` | 16px / 1.45 | 面板/对话框标题 |
| `text-xl` | 19px / 1.3 | 页面标题 |
| `text-2xl` | 23px / 1.2 | 空态/错误大标题（少用） |

规则：

- 页面标题 19px/600；区块标题 13px/600；正文 13px/400。
- 中文标题 600，正文 400/500；不滥用 700。
- 标题字距 `-0.01em` 至 `-0.015em`。
- 不使用全大写装饰标签。
- 不在每个标题下添加解释性副标题；只在确有上下文价值时使用一行 meta。

## 6. Spacing

4px 基准：4 / 8 / 12 / 16 / 20 / 24 / 32 / 40 / 48。

| 场景 | 值 |
|---|---|
| 顶部工作带 | 44px |
| 窗口控制按钮 | 46×44px，右侧贴边 |
| 页面左右内边距 | 20px（窄）/ 32px（≥1024）；<600px 为 12px |
| 页面顶部 | 16px（工作带以下） |
| 区块间距 | 20–24px |
| 行内元素间距 | 6–8px |
| 面板内边距 | 16–20px |
| 表格行高 | 40px 左右 |
| 表头高度 | 32px |
| 导航项高度 | 32px |
| 输入/按钮高度 | 28–32px |
| Dialog 内边距 | 20px |

- 页面内容最大宽度 1280px。
- 窗口最小尺寸 500×480；侧栏 240px，<760px 收为 52px 图标轨；主内容 `min-w-0`，长内容内部滚动。
- 主滚动区使用 `overflow-x-hidden` 与 `scrollbar-gutter: stable`，避免窄窗口出现页面级横向滚动条。
- 行高与可点击区域不可为了紧凑降到难以操作。

## 7. Border / Radius / Elevation

### 7.1 Radius

| Token | 值 | 用途 |
|---|---|---|
| `radius-sm` | 3px | Badge、标签、小按钮 |
| `radius-md` | 5px | Button、Input、Select、命令块 |
| `radius-lg` | 8px | Card、Dialog、Sheet、表格容器 |
| `radius-xl` | 12px | 仅大容器/空态底板（少用） |

- 不把所有组件做成同一大圆角。
- 状态 Badge 使用小圆角，不使用胶囊。
- 圆形仅用于状态节点、头像或指示点。

### 7.2 Border

- 默认 1px `line`；强调/输入使用 `line-strong`。
- 不叠加外框与内框双重线条。
- 表格仅保留行分隔与表头底线。
- 主内容区块优先 `border-y` / `divide-y`，避免每个区块独立卡片。

### 7.3 Elevation

- 面板默认无阴影，仅 1px 边框。
- Dialog / Sheet / Dropdown / Tooltip 使用 `shadow-overlay`：
  `0 18px 44px -20px rgb(5 12 14 / 0.42)`。
- 不使用柔和阴影堆叠卡片。

## 8. App Shell 与 Window Chrome

### 8.1 自定义标题栏决策

XUSHU 自 Phase 3.6 起使用自定义标题栏（`decorations: false` + `shadow: true`），完整取舍见 `docs/architecture/ADR/0006-custom-titlebar-window-chrome.md`。

- 顶部 44px 工作带由 XUSHU 绘制，是窗口唯一的第一层表面。
- 工作带信息层级：项目名使用 13px/500，页面名称使用 13px/600；「当前项目」保留可访问语义，不再占用可见空间。Git 元数据使用 11px，分支与 Commit 分开标识；≥1100px 显示分支与工作区状态，≥1280px 再显示 Commit。窄窗口优先保证项目选择、页面名称、主题与窗口控制；完整 Git 信息仍可在仓库/项目详情查看。
- 左侧 240px 品牌区与侧栏同色；<760px 收为 52px 图标轨。
- 项目上下文（当前项目、branch/HEAD/dirty）、页面名称、主题与 46×44px 窗口控制均在同一个工作带内。
- 拖动通过 `data-tauri-drag-region="deep"`；双击最大化/还原由 Tauri `internal_toggle_maximize` 处理。
- 窗口控制通过最小权限调用 `minimize`、`toggleMaximize`、`close`、`isMaximized`。
- Windows 11 Snap Layouts 通过最大化按钮位置的不可见原生子窗口返回 `HTMAXBUTTON` 恢复；不模拟按键/鼠标。
- 窗口最小尺寸 500×480，以支持 Snap 贴靠；窗口圆角由 DWM 提示，边框颜色设为无。
- 原生窗口行为出现明显退化时，按 ADR-0006 回滚到 ADR-0005，不强行保留视觉方案。

### 8.2 App Shell

```text
┌──────────────────────────────────────────────────────────────────────┐
│ 序 序枢 XUSHU │ 当前项目 [项目 ▾]  main/HEAD ● │ 工作台 │ ☀ ☾ ◐ │ – □ × │ 44px
├───────────────┼──────────────────────────────────────────────────────┤
│ 工作台         │ SignalRail / 焦点任务 / 最近验证                     │
│ 项目           │                                                      │
│ 任务     1     │  主内容（max 1280px，左右 20/32px，顶部 16px）       │
│ 验收中心 2     │                                                      │
│ 诊断           │  分区用 border-y / divide-y / 留白                   │
│ 设置           │                                                      │
│               │                                                      │
│ 本地 · 无云同步 │                                                      │
│ v0.1.0         │                                                      │
└───────────────┴──────────────────────────────────────────────────────┘
```

- 顶部工作带 44px；左侧 240px / 窄窗口 52px；主内容自适应。
- 顶部工作带与主内容之间只有 `line/60` 细线；左侧品牌区不画横线，侧栏连续。
- 主内容独立滚动（`main` `overflow-y-auto overflow-x-hidden [scrollbar-gutter:stable]`），侧栏固定。
- 页面内容最大宽度 1280px；窗口最小尺寸 500×480。
- 窗口背景使用 `background`，侧栏 `sidebar`，主内容 `workspace`；240px 竖线从窗口顶端贯通。
- 窗口控制按钮右侧贴边；关闭按钮悬停为 Windows 红。

## 9. Sidebar

结构（Phase 3.6 起项目上下文与主题上移到顶部工作带）：

1. 主导航：工作台、项目、任务、验收中心、诊断、设置。
   - 导航项 32px；左侧 2px 选中边线；选中背景 `workspace`。
   - 任务显示阻塞数，验收中心显示待验证数；数字使用 mono，不在无数据时显示 0。
   - <760px 收为 52px 图标轨：隐藏文字与数字，保留图标、可访问名称与状态圆点。
2. 底部（仅 ≥760px）：`本地 · 无云同步`、版本、Phase 标签；主题控件在顶部工作带。

规则：

- 侧栏不随主内容滚动；从窗口顶端工作带下方开始，但左侧表面与工作带连续。
- 项目名与路径在顶部工作带截断并提供 title，不撑破工作带。
- 不在侧栏放搜索、KPI 或大块操作按钮。
- 窄窗口不隐藏导航能力；图标仍可点击，名称通过 `sr-only` 保留。

## 10. Page Header / Context Header

- 一级页面（工作台、项目、任务、验收中心、诊断、设置）不再显示 19px 页面大标题；页面名称由 44px 顶部工作带承载，页面内使用 `sr-only h1` 保留语义。
- 列表页主操作进入上下文工具栏：项目页导入按钮与 Tabs 同行；任务页新建按钮在筛选工具栏右侧；验收中心导出按钮在证据工作区标题行。
- 详情页保留 `PageHeader`：项目名/任务标题是内容级 `h1`；`meta` 一行 11px 放状态、修订、更新时间或路径。
- `PageHeader` 标题 19px/600；主操作在右侧 `sm` 按钮；窄窗口换行，不挤出容器。
- 不使用装饰性 eyebrow、全大写标签或解释性副标题。
- 详情页的返回链接放在标题上方，使用 ghost `sm` 按钮。
- 一级页面名称由顶部工作带承载；详情页使用统一 `PageHeader`，不允许逐页自造标题栏。

## 11. Status System

### 11.1 任务状态

| 状态 | 中文 | 图标 | Token |
|---|---|---|---|
| draft | 草稿 | `FilePenLine` | `status-draft` |
| ready | 就绪 | `CircleDot` | `status-ready` |
| running | 执行中 | `Play` | `status-running` |
| awaiting_verification | 待验证 | `Hourglass` | `status-awaiting` |
| done | 已完成 | `CheckCircle2` | `status-done` |
| blocked | 已阻塞 | `OctagonAlert` | `status-blocked` |
| cancelled | 已取消 | `Ban` | `status-cancelled` |

### 11.2 验证结果

| 状态 | 中文 | 图标 | Token |
|---|---|---|---|
| pass | 通过 | `BadgeCheck` | `verification-pass` |
| fail | 失败 | `CircleX` | `verification-fail` |
| not_run | 未运行 | `CircleSlash` | `verification-not-run` |
| inconclusive | 无法判定 | `CircleHelp` | `verification-inconclusive` |
| stale | 已过期 | `History` | `verification-stale` |

### 11.3 执行状态

| 状态 | 中文 | 表达 |
|---|---|---|
| running | 运行中 | 旋转图标 + primary 文字 |
| completed | 已完成 | 结果由 VerificationBadge 表达 |
| cancelled | 已取消 | Ban + muted |
| timed_out | 已超时 | TimerOff + fail 色 |
| start_failed | 启动失败 | CircleSlash + not-run 色 |
| interrupted | 已中断 | Unplug + inconclusive 色 |

规则：

- 不允许只靠颜色；必须有图标 + 文本。
- 任务状态与验证结果不能混用同一颜色语义。
- 阻塞任务必须显示 `blockReason`；证据过期必须显示 `staleReason`。
- 完成门禁必须列出具体 blockers，不能只说“不能完成”。

### 11.4 Claude Code 接入状态

| 状态 | 中文 | 图标 | Token |
|---|---|---|---|
| not_configured | 未配置 | `CloudOff` | muted |
| disabled | 已停用 | `Ban` | muted |
| installed | 已启用 | `CircleDot` | success |
| partial | 部分安装 | `Wrench` | warning |
| needs_repair | 需要修复 | `Wrench` | warning |
| configuration_error | 配置异常 | `OctagonAlert` | destructive |
| receiver_unavailable | 接收器不可用 | `CircleSlash` | destructive |
| project_unavailable | 项目不可用 | `OctagonAlert` | destructive |
| desynchronized | 状态不一致 | `History` | warning |

活动事件状态：`执行中`（Loader2 旋转 + primary）、`成功`（CheckCircle2 + success）、`失败`（CircleX + destructive）、`状态未知`（CircleHelp + warning）、`已观察`（CircleDot + info）、`已结束`（CircleSlash + muted）。会话状态：`进行中`、`已结束`、`结束未确认`。不得把“未观察到结束”显示为成功。

### 11.5 记忆、决策与恢复状态

| 对象 | 状态 | 中文 | 图标 | Token |
|---|---|---|---|---|
| 记忆确认 | confirmed | 已确认 | `BadgeCheck` | success |
| 记忆确认 | unconfirmed | 未确认 | `CircleHelp` | warning |
| 记忆有效性 | archived | 已归档 | `Archive` | muted |
| 决策 | draft | 草稿 | `FilePenLine` | info |
| 决策 | confirmed | 已确认 | `BadgeCheck` | success |
| 决策 | superseded | 已被取代 | `History` | warning |
| 决策 | deprecated | 已废弃 | `Ban` | muted |
| 恢复快照 | fresh | 当前 | `BadgeCheck` | success |
| 恢复快照 | stale | 已过期 | `History` | warning |
| 恢复快照 | no_snapshot | 尚未生成 | `CircleSlash` | muted |
| 下一步 | user_selected | 用户指定 | `UserRoundCheck` | success |
| 下一步 | suggested | 系统建议 | `Wand2` | info |
| 下一步 | needs_confirmation | 需要人工确认 | `CircleHelp` | warning |
| Git | available | 可用 | — | success |
| Git | unknown | UNKNOWN | — | warning |
| 备份完整性 | verified | 已验证 | `BadgeCheck` | success |
| 备份完整性 | failed | 校验失败 | `CircleX` | destructive |
| 备份完整性 | unverified | 未校验 | `CircleHelp` | warning |
| 备份完整性 | missing | 目录缺失 | `CircleSlash` | muted |
| 备份来源 | manual | 手动 | `UserRound` | muted/info |
| 备份来源 | automatic | 自动 | `Timer` | muted/info |
| 完整性检查项 | pass | 通过 | `BadgeCheck` | success |
| 完整性检查项 | fail | 失败 | `CircleX` | destructive |
| 完整性检查项 | not_run | 未运行 | `CircleSlash` | muted |
| 恢复结果 | completed | 恢复完成 | `CheckCircle2` | success |
| 恢复结果 | rolled_back | 已回退 | `Undo2` | warning |
| 恢复结果 | rollback_failed | 回退失败 | `OctagonAlert` | destructive |
| 恢复步骤 | done/active/pending/failed | 完成/当前/待进行/失败 | 数字或 ✓ | success/primary/muted/destructive |

规则：

- 记忆/决策/恢复状态必须图标 + 中文 + 颜色，不得只靠颜色。
- 未确认记忆必须与已确认记忆分开展示，并明确“不是事实”，不得作为权威上下文。
- 草稿决策必须明确“不会进入恢复包”；已被取代/废弃决策不得作为当前约束展示。
- 恢复快照过期时必须显示变化来源与“重新生成”建议；历史快照必须标记为历史，不得冒充当前。
- 复制过期或历史恢复指令时，剪贴板文本开头必须加入警告；复制必须是用户主动操作。
- 记忆、决策、Hook 事件与恢复包都不得显示为验收证据；任务完成仍由独立验证与人工确认决定。

## 12. Components

### 12.1 Button

- 高度：`sm` 28px / 默认 32px / `lg` 36px / icon 32px。
- 变体：primary、outline、ghost、secondary、destructive、link。
- 主操作每页最多一个；行内操作使用 ghost 或 icon。
- 危险操作使用 destructive + ConfirmationDialog。

### 12.2 Input / Select / Textarea

- 高度 32px；背景 `workspace`；边框 `input`。
- Focus ring 2px `ring`；错误 `aria-invalid` + 字段级 `role="alert"`。
- Select 使用 Radix；保持键盘与可访问性。
- 表单标签在上方；必填/可选明确；不在每个字段下写长说明。

### 12.3 Section / SectionHeader

- 页面分区优先使用 `Section` + `SectionHeader`。
- SectionHeader：13px/600 标题 + 11px meta + 右侧轻量操作。
- 分区之间使用 `border-t` 或 24px 留白；不使用 Card 包裹。

### 12.4 Card / Panel

- 仅用于独立交互容器、重要安全信息或需要明确边界的内容。
- 边框 1px `line/80`；无阴影；内边距 16px。
- 不嵌套 Card；EmptyState 不放在 Card 内再套 Card。

### 12.5 Dialog / Sheet

- Dialog：短表单、确认、批准。最大宽度 512/672px；内边距 20px。
- Sheet：证据、历史、报告等长内容；右侧面板，宽 `min(92vw, 760px)`，Header 固定，Body 独立滚动。
- 两者均使用 Radix：焦点陷阱、Escape、关闭后返回焦点。
- 命令与风险提示不可为了美观而弱化。

### 12.6 Table / List

- 表格：表头 32px/11px；行 40px；仅行分隔与表头底线；hover `hover/70`。
- 表格容器允许横向滚动；页面不得整体横向溢出。
- 列表：`divide-y border-y`；行内操作使用 ghost 按钮或 Dropdown。
- 行标题可点击；操作菜单不触发行点击。

### 12.7 Badge / Alert / Tooltip / Toast

- Badge 小圆角、小字号；用于状态/类型/必需性。不要把所有元数据都做成 Badge。
- Alert 只用于错误、成功、风险等需要强调的信息。
- Tooltip 只补充说明，不承载唯一关键信息。
- Toast 做非阻塞反馈，不替代持久化状态。

### 12.8 CommandBlock / CriterionVerification

- CommandBlock：深色终端块，显示可执行程序、参数、cwd；使用 `code-*` Tokens。
- CriterionVerification：状态行 + 命令块 + 运行中 strip + 操作；不把整个自动条件包成多层卡片。
- 运行中：旋转图标 + 文字 + 取消按钮。
- 证据与历史通过 Sheet 展示，不挤在条件行内。

### 12.9 Empty / Loading / Error

- EmptyState：无边框、居中、图标 + 标题 + 一句方向说明 + 最多一个操作。
- LoadingState：行式 Skeleton，与真实列表结构一致；不使用大卡片。
- ErrorState：Alert 形式，说明发生了什么 + 重试操作。
- 不展示假数据填充空状态。

### 12.10 Claude Code 接入组件

- `ClaudeIntegrationSection`：项目详情中的横向 strip，使用 `border-y` 与 `SectionHeader`，不套 Card；状态 Badge + 原因 + 最近接收时间 + 操作。配置异常/需要修复/导入错误使用带左边线的 warning/destructive Alert，不隐藏原因。
- `ClaudeInstallDialog`：最大宽度 2xl；正文可滚动；显示写入位置、六个 Hooks 表、命令 details、会采集/不会采集两列、前置确认与复选框；未勾选时“安装并启用”必须禁用；受阻时显示原因并禁用主操作。
- `ClaudeActivitySheet`：右侧 760px Sheet；会话列表 + 未结束工具调用 + 事件时间线；命令、哈希、时间使用 mono；不展示原始 Prompt、命令、文件内容或输出。
- `ClaudeStateBadge` / `ClaudeEventStatus` / `ClaudeSessionState`：图标 + 中文 + 语义色；running 使用旋转图标，其他状态不使用装饰动画。
- 接入区块只负责观察，不出现“完成”“通过”或任务 KPI；文案明确“事件不等于验收通过”。

### 12.11 项目记忆、决策与恢复组件

- `MemoryConfirmationBadge` / `MemoryArchivedBadge`：记忆确认/归档状态；图标 + 中文 + 语义色。
- `DecisionStatusBadge`：草稿/已确认/已被取代/已废弃；草稿与已确认优先使用 info/success。
- `RecoveryFreshnessBadge` / `NextStepKindBadge` / `RecoveryGitBadge`：快照新鲜度、下一步来源、Git 可用性。
- `MemoryDetailSheet` / `DecisionDetailSheet`：右侧 760px Sheet；正文可滚动，Header/Footer 固定；Footer 使用 `flex-wrap`，窄窗口不隐藏操作。
- `MemoryFormDialog` / `DecisionFormDialog`：短表单 Dialog，最大宽度 2xl/3xl；正文可滚动；标签在输入上方；错误字段 `aria-invalid` + `role="alert"`。
- `RecoveryInstructionsSheet`：完整恢复指令预览；来源边界提示、深色 `code-*` Markdown 块、复制按钮；过期/历史警告置顶。
- `RecoveryHistorySheet`：最近 20 份快照；当前/历史明确标记；查看时显示加载态，不伪造内容。
- 项目知识页的记忆/决策行使用“类别/状态 + 标题 + 元数据 + 查看”的日志式行，不套 Card；分类图标只承担识别，不承担唯一状态表达。
- 恢复页的“下一步”使用现有 Focus Panel 语言（2px primary 左边线 + `surface-raised`），不新增 KPI 卡片。

### 12.12 数据保护组件

- `DataProtectionPage` 状态带：数据库健康、最近备份、策略与未加密提示；使用图标 + 标签 + 值 + 一行说明的紧凑网格，不使用 KPI 卡片。
- `CreateBackupDialog`：目标目录、范围、预计大小、未加密警告、真实阶段状态、取消请求、结果与失败原因；目标路径使用 mono。
- `BackupPolicyDialog`：自动开关、目标、间隔、保留、最近自动备份/错误、旧 automatic 备份预览与两步确认清理；手动备份明确不进入自动清理。
- `BackupVerifySheet`：清单/哈希/隔离完整性检查；校验失败不显示为成功。
- `RestoreWizardSheet`：7 步真实序列 + 内容分区；兼容性/影响范围/隔离验证依次可见；确认区必须输入“恢复”并勾选风险；执行中不可关闭；结果区分完成/回退/回退失败。
- `IntegrityCheckList` / `IntegrityReportSummary`：检查项、Schema、时间、计数；失败项保留 detail，不只显示“失败”。
- `PrivacyInventory` / `HooksBoundary`：数据资产行式清单与 Hook 采集/排除边界；不显示 Salt 内容或事件正文；敏感/是否进入备份使用图标 + 文字 + 语义色。
- 备份/恢复路径、SHA-256、Schema、备份 ID 使用 mono；长路径 `break-all`，不撑破布局。

## 13. Page Patterns

### 13.1 工作台 Dashboard

- 不再以大标题“工作台”开场；页面名称在顶部工作带，内容用 `sr-only h1`。
- 首屏顺序：`SignalRail` 紧凑状态计数 → 当前焦点任务“下一步” → “最近验证” → 下方“需要处理 / 最近任务”与右侧仓库/系统。
- “下一步”：紧凑 Focus Panel，使用 2px 品牌左边线与 `surface-raised` 底色；11px 区块名 + 状态、16px 任务标题、11px 原因、单一主操作。内边距 12px/16px，窄窗口主操作移到下一行；阻塞/过期原因完整换行，不作单行裁切。无任务时给出新建入口。
- “需要处理”：阻塞、待验证、失败/过期证据；已在「下一步」展开的焦点任务不重复列出，标注「焦点任务见上方」及其余数量。只有焦点任务时明确告知已列在「下一步」，不显示「没有阻塞」。任务筛选、排序及数据来源不变。
- “最近任务”：状态、标题、优先级、更新时间；提供新建与查看全部。
- 任务列表/最近任务采用约 40px 扫描行；状态可用图标 + 中文 + 语义色的无底色行内形式，详情与焦点面板继续使用 Badge。最近验证采用结果/条件、所属任务/时间两层记录，退出码与耗时同时可读；入口保留对应任务详情，以支持已完成任务的历史与证据查询。
- 侧列：当前仓库、系统健康、验收中心入口。
- 无项目：直接空状态 + 三步工作流提示；不套 Card。
- 1366×768 下首屏应同时可见状态条、焦点任务与最近验证。

### 13.2 项目 Projects

- 无独立页头；下划线 Tabs 与“导入项目”按钮同一行。
- 下划线 Tabs：进行中/已归档。
- 行式项目列表：当前点、名称、路径（mono）、分支/HEAD/dirty（紧凑 mono）、打开/操作。
- 不在每行套 Card。

### 13.3 项目详情 Project Detail

- 返回链接 + 页头（名称、路径、当前/归档、操作）。
- 仓库 strip：分支、HEAD、工作区、检查时间；只读说明。
- 主列：项目任务表；侧列：项目信息与只读说明。
- 任务为空时使用 plain EmptyState。

### 13.4 任务 Tasks

- 无独立页头；新建任务在筛选工具栏右侧。
- 单行工具栏：搜索、项目筛选、状态筛选、包含归档、清除、匹配数量、新建任务。
- 任务表：标题、状态、优先级、项目、修订、更新时间、行操作菜单（打开/归档/恢复）。
- 表格容器只有一个边框；空/错误状态独立展示。

### 13.5 任务详情 Task Detail

- 返回链接 + 页头（标题、项目、状态、优先级、修订、更新时间、进入验收/编辑/归档）。
- StateTrack strip（无 Card）。
- 主列：目标、验收条件、状态历史。
- 侧列：完成门禁、状态操作、元数据、当前阻塞原因。
- 完成门禁用左边线 + 浅色底的 strip，列出具体 blockers 与下一步。
- 自动条件内嵌 `CriterionVerification`；人工条件保留人工确认入口。
- 证据与历史使用 Sheet。

### 13.6 验收中心 Verification Center

- 无独立页头；待验收数量在队列标题，导出报告在证据工作区标题行。
- 双栏：300px 待验收队列 + 证据工作区；窄屏纵向。
- 左栏：任务、项目、自动通过数、人工待确认、运行中；选中项左侧青线。
- 右栏：任务摘要、三项统计 strip、blockers strip、验收条件列表。
- 自动条件显示 CommandBlock、VerificationBadge、运行/取消/证据/历史。
- 人工条件显示确认状态并链接到任务详情。
- 空队列：plain EmptyState。
- 报告：右侧 Sheet，Markdown 深色代码块 + 复制。

### 13.7 设置 Settings

- 单列 max-width 3xl。
- 外观：三行 radio（浅色/深色/跟随系统），不是三张卡。
- 关于、本地数据与隐私：definition list 行式布局。
- 不展示绝对路径、API Key、token。

### 13.8 诊断 Diagnostics

- 页头“诊断” + meta“本地数据与系统健康”。
- 系统健康：plain 分区 + 指标网格，无 Card。
- 持久化探针说明：左边线提示条。
- 探针表单与记录列表分两栏；表单字段、记录行式展示；无 Card 叠 Card。

### 13.9 404 / Error Boundary

- 404：居中、简洁、返回工作台。
- Error Boundary：窗口级错误说明 + 重新加载 + 技术信息；使用统一 Tokens，不使用营销式卡片。

### 13.10 项目详情 · Claude Code 接入

- 顺序：仓库 strip → Claude Code 接入 strip → 项目记忆与决策 strip → 主列任务表 / 侧列项目信息。
- 接入 strip 不抢任务主列的视觉重量；未配置时正文只保留一段观察范围说明与一个主操作，不显示空 KPI。
- 已启用时显示最近 4 条事件与“查看全部活动”；状态、待导入、未结束工具调用和错误原因同屏可见。
- 活动 Sheet 中的会话/时间线是真实数据；没有事件时使用 plain EmptyState，不构造演示会话。
- 归档项目不允许安装/修复，但允许停用/卸载已有 Hooks，并明确说明。

### 13.11 项目记忆与决策

- 路由：`/projects/:projectId/knowledge`；详情型子页面，保留 `PageHeader`（标题“记忆与决策”，meta 项目名/路径/归档状态）。
- 顶部说明“记忆与决策是开发上下文，不会改变任务状态、验收证据或完成门禁”；右上提供“生成恢复包”入口。
- 下划线 Tabs：项目记忆 / 开发决策；Tabs 上方可显示来自任务详情的“仅显示关联任务”筛选条，使用左侧 info 线 + 清除按钮。
- 记忆工具栏：类别筛选、包含已归档、数量、新建记忆；决策工具栏：状态筛选、数量、新建决策。主操作在工具栏右侧，不另设页头大按钮。
- 列表使用 `divide-y` 日志行：类别/状态 + 标题 + 确认/决策 Badge；第二行内容摘要；第三行修订、更新时间、关联任务与“查看”。不套 Card。
- 记忆详情/决策详情使用右侧 Sheet；修订历史使用可展开的 `details`，默认显示版本、标题、变更说明与时间。
- 新建/编辑使用 Dialog；决策支持“每行一个备选方案”，并可在草稿阶段选择“取代已确认决策”。
- 归档项目只读：隐藏新建按钮，详情 Sheet 不显示编辑/确认/归档操作，明确说明。
- 空白状态使用 plain EmptyState + 新建入口；错误/加载状态复用共享组件。

### 13.12 恢复 / 跨会话上下文

- 路由：`/recovery`；一级页面，页面名称由 44px 顶部工作带承载，内容使用 `sr-only h1`。
- 首屏“恢复带”：新鲜度 Badge、生成时间、指纹、任务/记忆/决策计数、当前项目；下方为一组单行操作：生成/重新生成、预览完整指令、复制恢复指令、历史快照、检查是否过期。
- 尚无快照：plain EmptyState“尚未生成恢复包” + 两组说明（会包含什么 / 数据边界）；不构造演示恢复内容。
- 快照过期：顶部 warning Alert 列出变化来源，建议重新生成；复制时先二次确认，剪贴板文本开头加入过期警告。
- 下一步：沿用 Focus Panel（2px primary 左边线 + `surface-raised`）；显示来源（用户指定/系统建议/需要人工确认）、任务状态、依据；提供任务选择与“设为下一步/清除指定”。
- 摘要主体：双列（≥1280px）——项目与 Git、任务、独立验证；Claude Code 活动、已确认记忆、开发决策、待读文件、冲突与缺失信息；窄窗口纵向堆叠。
- 验证状态必须写“按生成时状态”，不得把过期结果写成“当前测试通过”。
- “数据来源、包含上限与排除项”使用 `<details>` 折叠；保留数据库读取时间、Git 检查时间、计数、长文本截断说明与隐私排除项。
- 完整恢复指令使用右侧 Sheet：边界提示 + 深色 `code-*` Markdown 块 + 复制；历史快照在该 Sheet 中明确“这是历史快照，不代表当前状态”。
- 历史快照 Sheet：最近 20 份，当前/历史 Badge；查看时显示加载态；不把历史快照标记为当前。
- 500×480 下按钮换行、任务选择宽度收敛、Sheet Footer 可换行；不得出现页面级横向滚动。

### 13.13 数据保护 / 备份与恢复

- 路由：`/data-protection`；一级页面，页面名称由 44px 顶部工作带承载，内容使用 `sr-only h1`。
- 首屏顺序：启动恢复提示（如有）→ 状态带 → 备份与恢复 → 恢复历史 → 数据完整性 → 隐私与本地数据 → 数据目录。
- 状态带四项：数据库版本/完整性、最近备份、备份策略、备份隐私。使用 `border-y` 与紧凑网格，不套卡片。
- 备份列表使用 `divide-y` 日志行：来源、时间、完整性 Badge、Schema/格式/大小、路径（mono）、SHA-256、操作（校验/恢复/删除）。没有备份时 plain EmptyState + 创建入口。
- 备份策略与创建备份使用 Dialog；校验使用 Sheet；恢复使用 7 步向导 Sheet。破坏性操作（删除备份、执行恢复）必须二次确认/输入确认。
- 未加密、备份范围、Hooks 不自动修改、恢复会替换当前数据库等风险在创建/预检/确认/结果处持续可见，不得只放在文档里。
- 恢复向导：步骤轨是真实序列，不使用装饰性编号；预检未通过不显示“继续确认”；执行中禁用关闭；结果显示最终校验与回退状态；回退不得显示为成功恢复。
- 隐私资产清单使用行式 `details` 展开保留/影响/导出；Hook 采集与排除分两列；Salt 只显示存在性/大小。
- 500×480 下操作按钮换行、向导正文可滚动、Dialog/Sheet 不溢出；不得出现页面级横向滚动。

## 14. Motion

- Hover/press：120ms ease-out，只改颜色/边框/透明度。
- Dialog/Sheet：160–180ms，轻微淡入/位移。
- 不自动播放页面入场动画；不使用循环装饰动画。
- 运行中旋转图标是状态反馈，不是装饰。
- 遵循 `prefers-reduced-motion`。

## 15. Focus / Accessibility

- 所有交互元素有 `focus-visible` ring（品牌色）。
- 语义化 landmark：aside / nav / header / main。
- Dialog/Sheet 使用 Radix：焦点陷阱、Escape、关闭后返回焦点。
- 表单 Label 与控件关联；错误 `role="alert"`。
- 状态不只靠颜色；有文字与图标。
- 表格有表头；行操作有可读 `aria-label`。
- 键盘可完成主流程：导航、筛选、创建、状态转换、确认、归档、批准运行。
- 自绘窗口控制提供中文可访问名称与键盘焦点；可见按钮不因 Snap 覆盖层而失去键盘操作。
- 顶部工作带不遮挡系统辅助功能；Snap 覆盖层是原生命中区，不进入 UIA 树，但不承载唯一信息。
- 窗口焦点反馈由内容 focus ring、窗口阴影与工作带表面层级共同提供（人工复核项）。

## 16. Light / Dark Themes

- 浅色不是白底灰框：使用冷灰绿 canvas / sidebar / workspace 三层。
- 深色不是黑底绿按钮：使用石墨蓝绿层级，青色只做信号。
- 两套主题共享相同语义与状态色，不改变含义。
- 命令块在浅色与深色下都保持深色终端表面。
- 自定义顶部工作带使用 `sidebar` / `workspace` Token；主题切换后应检查工作带、窗口控制、菜单、Dialog、Sheet、Toast、Skeleton 与滚动条。
- 原生窗口主题仍通过 `setTheme` 同步，主要影响窗口阴影/系统菜单；工作带不依赖系统标题栏配色。
- 主题设置持久化在本地；跟随系统使用 `prefers-color-scheme`。

## 17. Z-index

| Token / 层级 | 值 | 用途 |
|---|---|---|
| base | 0 | 内容 |
| window-chrome | 10 | 顶部工作带（窗口级，独立于主滚动区） |
| sticky | 10 | sticky 表头/工具栏 |
| header | 20 | 页面内吸顶元素 |
| overlay | 40 | Dialog/Sheet 遮罩 |
| dialog | 50 | Dialog/Sheet/Dropdown/Tooltip |
| toast | 60 | Toast |

## 18. Governance

- 新页面/组件必须先使用 `frontend-design` Skill，并阅读本文件。
- 复用 `components/ui`、`PageHeader`、`Section`、`StatusBadge`、`EmptyState`、`LoadingState`、`ErrorState`、`Dialog`、`Sheet`、`Toast`。
- 新视觉 Token 必须写入 `index.css` 并同步本文档。
- 改变全局视觉语言必须说明原因与影响范围，不能只改单页。
- 页面变更后更新 `docs/design/UI_AUDIT.md`。
- 修改顶部工作带、窗口控制或 `window_chrome.rs` 后，必须回归拖动、双击、最小化/最大化/还原/关闭、Snap 命中、DPI 与窄窗口布局。
- 不自动截图/录屏；视觉检查由用户手动提供截图。
- 未完成人工视觉检查时标记 `VISUAL_REVIEW_PENDING`，不得声称 UI 已完美通过。
- 技术检查通过不等于视觉验收通过。

## 19. Phase 3.5 实现说明

- 已调用 `frontend-design` Skill（当前主会话，2026-11-06），未使用子 Agent。
- 已保留原生 Windows 标题栏；移除应用级顶栏；侧栏 + PageHeader 构成单一工作带。
- 已新增 `core:window:allow-set-theme` 权限，仅用于原生标题栏主题同步。
- 已用 `Section` / `SectionHeader` / 行式列表替代大部分 Card。
- 命令与证据使用深色终端块；证据/历史使用右侧 Sheet。
- 已在 1366×768、1600×900、1920×1080（物理，100%/150% 缩放）检查无横向溢出。
- 视觉状态：**VISUAL_REVIEW_PENDING**。等待用户手动截图确认。

## 20. Phase 3.6 实现说明

- 已再次调用 `frontend-design` Skill（当前主会话，2026-11-06），未使用子 Agent。
- 已切换为自定义标题栏：`decorations: false`、44px 顶部工作带、46×44px 自绘窗口控制、240/52px 品牌与侧栏。
- 已实现 Windows 11 Snap Layouts 命中覆盖层（`HTMAXBUTTON`），并保留拖动、双击、最小化/最大化/还原/关闭。
- 顶部工作带左侧与侧栏、右侧与主内容共用表面；240px 竖线从窗口顶端贯通。
- 工作台移除大标题，首屏改为状态条 → 焦点任务 → 最近验证；列表页主操作进入上下文工具栏。
- 窗口最小尺寸调整为 500×480；<760px 侧栏收为图标轨；主内容禁止页面级横向滚动。
- 已在 1366×768、1600×900、1920×1080、500×480 与 100%/150% 缩放下检查无页面级横向溢出。
- 视觉状态：**VISUAL_REVIEW_PENDING**。Snap 悬停菜单、窗口圆角、焦点与多显示器需用户手动确认。

## 21. Phase 4 实现说明

- 已实际调用 `frontend-design` Skill（当前主会话，2026-11-09），未使用子 Agent，未自动截图。
- 新增项目详情“Claude Code 接入”strip、安装预览 Dialog、活动 Sheet 与接入/事件状态组件；复用 Section、Badge、Dialog、Sheet、EmptyState、LoadingState、ErrorState、Alert、ConfirmationDialog。
- 未新增一级导航、未改变 44px 顶部工作带、窗口控制、侧栏或任务/验收页面结构。
- 接入状态使用图标 + 中文 + 颜色；事件成功/失败/未知与会话结束未确认分开表达；不把 Hook 事件当作任务完成证据。
- 已通过 typecheck、Lint、48 个 Vitest、生产构建与无截图桌面冒烟（接入区块、预览、1366/500 布局、工作区回归）。
- 视觉状态：**VISUAL_REVIEW_PENDING**。安装预览与活动 Sheet 的浅色/深色人工视觉复核尚未完成。

## 22. Phase 5 实现说明

- 已实际调用 `frontend-design` Skill（当前主会话，2026-11-12），未使用子 Agent，未自动截图/录屏。
- 新增一级导航“恢复”，以及项目详情子页面“记忆与决策”；保留 44px 顶部工作带、240/52px 侧栏、自定义窗口控制与 Snap Overlay。
- 项目详情新增“项目记忆与决策”strip；任务详情新增“相关决策与记忆”；工作台焦点面板增加“查看恢复上下文”入口。
- 记忆/决策使用日志式 `divide-y` 行、右侧详情 Sheet 与编辑 Dialog；恢复页使用“恢复带 + Focus Panel + 双列摘要 + 数据来源 details”；完整指令与历史快照使用 Sheet。
- 颜色、字体、间距、圆角、阴影与状态全部复用现有 Tokens；未新增全局配色。新增状态映射：记忆确认/归档、决策草稿/确认/取代/废弃、恢复新鲜度、下一步来源与 Git 可用性。
- `SheetFooter` 增加 `flex-wrap`，保证 500px 窗口下多个操作按钮可换行；未改变其他页面结构。
- 项目知识页与恢复页通过 typecheck、Lint、Vitest、生产构建与隔离桌面 E2E；E2E 验证记忆/决策/快照 IPC、fresh → stale → fresh、任务状态不变与页面文本可见。
- 视觉状态：**VISUAL_REVIEW_PENDING**。记忆/决策 Dialog 与 Sheet、恢复页双列与窄窗、深浅主题、过期复制确认与历史快照仍需用户手动截图复核。

## 23. Phase 6 实现说明

- 已实际调用 `frontend-design` Skill（当前主会话，2026-11-13），未使用子 Agent，未自动截图/录屏。
- 新增一级导航“数据保护”（`/data-protection`）；保留 44px 顶部工作带、240/52px 侧栏、自定义窗口控制与 Snap Overlay。
- 页面使用“状态带 + 备份日志 + 恢复历史 + 完整性报告 + 隐私资产清单 + 数据目录”的账本式结构，不使用 KPI 卡片墙。
- 新增创建备份 Dialog、备份策略 Dialog、备份校验 Sheet、7 步恢复向导 Sheet、完整性检查列表、数据资产清单与 Hook 边界组件。
- 颜色、字体、间距、圆角、阴影与状态全部复用现有 Tokens；新增备份完整性/来源、完整性检查项与恢复结果状态映射。未引入新的全局配色或字体。
- 恢复向导使用真实 7 步序列（选择、兼容性、影响、隔离验证、确认、执行、结果）；确认必须输入“恢复”并勾选风险；执行中不可关闭；结果显示最终校验与回退状态。
- 布局 E2E 覆盖 500×480、1366×768、1600×900、1920×1080 与深浅主题：顶部工作带 44px、侧栏 52/240px、无页面级横向溢出、可见按钮 ≥20px；500×480 恢复向导正文可滚动。
- 视觉状态：**VISUAL_REVIEW_PENDING**。数据保护页、创建/策略 Dialog、校验 Sheet、恢复向导的浅色/深色人工视觉、150% 缩放、键盘焦点与真实剪贴板仍需用户手动截图复核。
