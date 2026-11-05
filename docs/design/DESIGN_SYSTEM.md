# XUSHU Design System

> 版本：1.1（Phase 3，2026-11-06）
> 适用范围：XUSHU 所有桌面端页面、组件与后续阶段。
> 设计方法：先读本文件 → 使用 `frontend-design` Skill → 复用 Tokens/App Shell/组件 → 实现 → 质量检查 → 更新 `UI_AUDIT.md` → 人工视觉复核。

## 1. 品牌定位

**XUSHU / 序枢** 是面向独立开发者的本地优先 AI 工作控制系统。核心不是“展示 AI”，而是让工作形成**有序、可信、可验证**的闭环。

视觉关键词：

- **克制**：不靠大面积渐变、玻璃拟态或装饰性动画制造“高级感”。
- **精密**：信息层级、对齐、状态与证据位置稳定可预测。
- **工作台**：像开发者每天打开的桌面工具，而不是营销落地页。
- **可追溯**：状态、历史、修订号与证据始终可读，不隐藏关键事实。

参考成熟工具的设计方法：Linear 的信息层级与状态表达、Raycast 的克制与操作效率、VS Code 的工作区结构、GitHub Desktop 的仓库状态与反馈。不复制其完整界面、图标或品牌资产。

### 视觉母题：Sequence Bench / 序列工作台

XUSHU 的识别元素是“序列”：

- 任务状态沿一条水平 **State Track** 推进；
- 任务历史沿垂直 **State Spine** 展开；
- Dashboard 使用一条真实状态的 **Signal Rail**，而不是一排同质 KPI 卡片。

这些结构同时表达产品语义（顺序、状态、证据），不是装饰。

### 反模式

禁止：

- 每个区块都做成相同的圆角卡片 + 同样阴影；
- 大面积紫色/蓝色渐变、玻璃拟态、发光边框；
- Emoji 代替图标；
- 只用颜色表达状态；
- 为了“丰富”而加入假 KPI、假项目、假任务；
- 全大写标签、装饰性 eyebrow、无意义编号；
- 与业务无关的复杂动画或页面加载编排。

## 2. 品牌与颜色

### 2.1 基础色板

XUSHU 使用“冷石墨 + 氧化青 + 黄铜”的控制台色板。青色是品牌信号，黄铜只用于“等待验证/注意”，珊瑚红只用于阻塞/危险。

**Light**

| Token | Hex | 用途 |
|---|---|---|
| `canvas` | `#F3F6F5` | 应用背景 |
| `surface` | `#FFFFFF` | 面板、输入、弹出层 |
| `surface-subtle` | `#EAF0EE` | 次级面板、表头、选中行 |
| `ink` | `#192428` | 主文字 |
| `ink-muted` | `#5E7077` | 次要文字 |
| `line` | `#D7E0DE` | 默认边框 |
| `line-strong` | `#BCCAC7` | 强调边框、分隔 |
| `brand` | `#16796F` | 品牌/主操作/运行中 |
| `brand-hover` | `#105F58` | 主操作 hover |
| `brand-soft` | `#DFF1ED` | 品牌浅底 |
| `brass` | `#9A6A1F` | 等待验证/注意 |
| `brass-soft` | `#F5EAD6` | 等待验证浅底 |
| `success` | `#237A52` | 完成/通过 |
| `success-soft` | `#DFF2E8` | 成功浅底 |
| `danger` | `#B04444` | 阻塞/失败/危险操作 |
| `danger-soft` | `#F7E3E1` | 危险浅底 |
| `info` | `#35679A` | 就绪/信息 |
| `info-soft` | `#E3EDF7` | 信息浅底 |

**Dark**

| Token | Hex | 用途 |
|---|---|---|
| `canvas` | `#0D1416` | 应用背景 |
| `surface` | `#141D20` | 面板、输入、弹出层 |
| `surface-raised` | `#1A2529` | 浮层、选中行 |
| `ink` | `#E5ECEB` | 主文字 |
| `ink-muted` | `#92A5A8` | 次要文字 |
| `line` | `#263538` | 默认边框 |
| `line-strong` | `#35494D` | 强调边框 |
| `brand` | `#62CFC2` | 品牌/主操作/运行中 |
| `brand-hover` | `#7CDFD3` | 主操作 hover |
| `brand-soft` | `#163A37` | 品牌浅底 |
| `brass` | `#D9AE63` | 等待验证/注意 |
| `brass-soft` | `#3A2E1A` | 等待验证浅底 |
| `success` | `#71CE9E` | 完成/通过 |
| `success-soft` | `#173528` | 成功浅底 |
| `danger` | `#E08585` | 阻塞/失败/危险操作 |
| `danger-soft` | `#3A1F20` | 危险浅底 |
| `info` | `#8DB6E8` | 就绪/信息 |
| `info-soft` | `#1B2B3D` | 信息浅底 |

### 2.2 品牌使用原则

- 主操作、当前选中、运行中状态使用 `brand`。
- 品牌色不用于大面积背景；用于按钮、图标、选中边线、少量标签。
- 黄铜色仅用于“等待验证/需要关注”，不得作为第二品牌色滥用。
- 珊瑚红仅用于阻塞、失败、删除/危险操作。
- 深浅主题必须保持相同语义，不改变状态含义。

## 3. Design Tokens

所有 Token 以 CSS 变量定义在 `apps/desktop/src/index.css`，通过 Tailwind `@theme inline` 暴露为工具类。

### 3.1 Color Tokens

- Surface：`background`, `card`, `popover`, `secondary`, `muted`, `accent`
- Content：`foreground`, `card-foreground`, `muted-foreground`, `primary-foreground`
- Border/Input/Ring：`border`, `input`, `ring`
- Semantic：`primary`(brand), `success`, `warning`(brass), `info`, `destructive`
- Status：`status-draft`, `status-ready`, `status-running`, `status-awaiting`, `status-done`, `status-blocked`, `status-cancelled`
- Future verification：`verification-pass`, `verification-fail`, `verification-not-run`, `verification-inconclusive`, `verification-stale`
- Layout：`line-strong`, `surface-raised`

### 3.2 Typography Tokens

| Token | 值 | 用途 |
|---|---|---|
| `--font-sans` | `"Segoe UI Variable Text", "Segoe UI", "Microsoft YaHei UI", "Microsoft YaHei", sans-serif` | 界面正文/标题 |
| `--font-mono` | `"Cascadia Code", "Cascadia Mono", Consolas, monospace` | ID、路径、HEAD、命令预览 |
| `text-xs` | 12px / 1.5 | 元数据、辅助说明 |
| `text-sm` | 13px / 1.55 | 控件、列表、正文 |
| `text-base` | 14px / 1.6 | 页面正文 |
| `text-lg` | 16px / 1.45 | 面板标题 |
| `text-xl` | 20px / 1.3 | 页面标题 |
| `text-2xl` | 24px / 1.2 | 工作台主标题 |
| `text-3xl` | 28px / 1.15 | 登录/空态大标题（当前不使用） |

规则：

- 中文标题使用 600 weight，正文 400/500；不滥用 700。
- 标题字距 `-0.01em`；正文默认 0。
- 不使用全大写英文标签作为装饰。
- 长文本行宽控制在 68–78 个字符；窄栏可更短。
- 数字、ID、Git HEAD 使用 `font-mono`。

### 3.3 Spacing Tokens

4px 基准：`4, 8, 12, 16, 20, 24, 32, 40, 48`。

- 页面内容间距：32px（窄窗口 24px）。
- 面板内边距：20–24px。
- 表单字段间距：16px；字段标签与输入 6px。
- 列表行垂直内边距：12–14px。
- 图标与文字间距：8px。

### 3.4 Radius Tokens

| Token | 值 | 用途 |
|---|---|---|
| `radius-sm` | 4px | Badge、标签 |
| `radius-md` | 6px | Button、Input、Select |
| `radius-lg` | 10px | Panel、Dialog |
| `radius-xl` | 14px | 仅大容器/空态插画底板 |

### 3.5 Shadow Tokens

- 面板默认：无阴影，只用 1px 边框。
- 浮层/Dialog/Toast：`0 16px 40px -16px rgb(9 18 20 / 0.35)`。
- 不使用每个卡片相同的柔和阴影。

### 3.6 Border Tokens

- 默认 1px `line`。
- 选中/聚焦 1px `brand` + ring。
- 分隔线使用 `line`，不使用粗重横线。
- 表格不使用外框+内框双重线条；仅行分隔与表头底线。

### 3.7 Motion Tokens

- Hover/press：120ms `ease-out`，只改变颜色/边框/透明度。
- 浮层/Dialog：160–180ms，轻微淡入+位移 4px。
- 不自动播放页面入场动画；不使用循环装饰动画。
- 遵循 `prefers-reduced-motion`。

### 3.8 Z-index Tokens

| Token | 值 | 用途 |
|---|---|---|
| `z-base` | 0 | 内容 |
| `z-sticky` | 10 | sticky 表头/工具栏 |
| `z-header` | 20 | 顶部栏 |
| `z-overlay` | 40 | 遮罩 |
| `z-dialog` | 50 | Dialog |
| `z-toast` | 60 | Toast |

## 4. App Shell 与布局

```text
┌──────────────┬──────────────────────────────────────────────┐
│ Brand        │ Top bar: context · theme · quick actions     │ 56px
│ Current proj │──────────────────────────────────────────────│
│              │ Page header: title / description / actions   │
│ Navigation   │                                              │
│              │ Content max 1240px, padding 32px             │
│              │                                              │
│ Footer       │                                              │
└──────────────┴──────────────────────────────────────────────┘
   252px
```

- 左侧导航固定宽度 **252px**，不随内容抖动；窗口最小宽度 900px 时仍可用。
- 顶部工具栏高度 **56px**。
- 页面内容最大宽度 **1240px**，居中，左右 padding 32px（窄于 1100px 时 24px）。
- 页面标题 20–24px，描述 13–14px `ink-muted`。
- 主内容采用 **Panel**（边框+表面）而非漂浮卡片；同一页面最多 2–3 个主面板。
- 列表/表格容器 `min-width: 0`，内部允许横向滚动，禁止页面整体横向溢出。
- 页面切换保持滚动位置合理：切换路由回到内容顶部。
- 长路径/ID 使用 `truncate` + `title`；禁止撑破布局。
- 窗口缩放：侧栏固定，内容栏 `min-w-0`；表格列可压缩，操作列固定在右侧。

## 5. 统一交互组件

组件位于 `apps/desktop/src/components/ui`，必须复用：

- Button：primary / secondary / outline / ghost / destructive；sm / default / lg / icon。
- Input / Textarea / Select（原生 select 的 shadcn 风格封装，保证键盘与可访问性）。
- Dialog / ConfirmationDialog：Radix Dialog，焦点陷阱、Escape 关闭、返回焦点。
- DropdownMenu：Radix Dropdown Menu，用于行内“更多操作”。
- Tabs：Radix Tabs，用于任务详情/项目详情的分区。
- Table：语义化 table，行分隔、表头、可横向滚动。
- Card / Panel：统一边框、标题区与内容区。
- Badge / StatusIndicator：图标 + 文字 + 颜色。
- Tooltip：图标按钮的补充说明，不能承载唯一关键信息。
- Toast：sonner，操作成功/失败的非阻塞反馈。
- EmptyState / LoadingState / ErrorState：统一插画/图标、文案与主操作。
- StateTrack / StateSpine：状态序列与历史时间线。
- PageHeader：标题、描述、主操作。

规则：

- 同一操作在所有页面使用相同文案与位置（主操作在页头右侧）。
- 破坏性操作放在 Dropdown 的 danger 区，并通过 ConfirmationDialog 二次确认。
- 所有图标按钮必须有 `aria-label` 或 Tooltip + 可见/隐藏文本。
- 表单错误紧邻字段，使用 `role="alert"`；全局错误用 ErrorState。
- Toast 只做确认反馈，不替代持久化状态展示。

## 6. 状态表达规范

状态必须同时使用 **图标 + 中文文字 + 颜色**，不得只依赖颜色。

| 状态 | 中文 | 图标（Lucide） | 颜色 token | 含义 |
|---|---|---|---|---|
| draft | 草稿 | `FilePenLine` | `status-draft` | 尚未定义完整 |
| ready | 就绪 | `CircleDot` | `status-ready` | 可开始执行 |
| running | 执行中 | `Play` | `status-running` | 正在执行，不代表完成 |
| awaiting_verification | 待验证 | `Hourglass` | `status-awaiting` | 执行结束，等待证据 |
| done | 已完成 | `CheckCircle2` | `status-done` | 必需验收已满足 |
| blocked | 已阻塞 | `OctagonAlert` | `status-blocked` | 需要人工解除 |
| cancelled | 已取消 | `Ban` | `status-cancelled` | 终止状态 |

验证状态（Phase 3 已接入）：

| 状态 | 中文 | 图标 | Token |
|---|---|---|---|
| PASS | 通过 | `BadgeCheck` | `verification-pass` |
| FAIL | 失败 | `CircleX` | `verification-fail` |
| NOT_RUN | 未运行 | `CircleSlash` | `verification-not-run` |
| INCONCLUSIVE | 无法判定 | `CircleHelp` | `verification-inconclusive` |
| STALE | 已过期 | `History` | `verification-stale` |

状态展示组件：

- `StatusBadge`：列表/表头使用，图标 12px + 12px 文本。
- `StateTrack`：任务详情/页头使用，显示状态序列与当前节点；已完成节点用 success，当前节点用对应状态色。
- `StateSpine`：任务历史使用，节点+竖线，记录 from → to、时间、原因、修订号。
- 阻塞任务必须同时展示当前 `blockReason`，不能只显示红色 Badge。
- 完成门禁必须列出未通过条件与下一步，不得只显示“不能完成”。

## 7. 页面设计规范

### 7.1 工作台 Dashboard

- 页头：工作台 / 当前项目切换 / 主操作“新建任务”。
- 顶部 **Signal Rail**：真实任务总数与各状态计数，横向紧凑排布，不用巨大 KPI 卡片。
- 左主列：需要关注（阻塞 + 待验证，按更新时间）、最近任务。
- 右侧：当前项目 Panel（名称、路径、Git 分支/HEAD/脏状态、项目详情入口）、系统健康 Panel。
- 无项目：大号空状态 + “导入第一个 Git 项目”主操作；不展示假数据。
- 无任务：任务区空状态 + “新建任务”。

### 7.2 项目 Projects

- 页头 + “导入项目”主操作。
- 过滤器/Tabs：进行中 / 已归档。
- 项目列表为行式布局：名称、路径（mono、truncate）、Git 状态（分支/HEAD/脏）、当前标记、操作菜单。
- 当前项目用左侧 brand 竖线 + “当前”Badge 标记。
- 行点击进入项目详情；操作菜单不触发行点击。
- 导入使用 Dialog：路径输入 + 可选名称；说明只读检查、不修改仓库。
- 归档/恢复使用确认对话框，说明不删除磁盘文件与任务数据。

### 7.3 项目详情 Project Detail

- 页头：项目名、路径、当前/归档 Badge、操作（设为当前、重命名、归档/恢复）。
- Git 状态 Panel：分支、HEAD、脏状态、detached/unborn 说明、最近检查时间。
- 任务 Panel：该项目任务列表，支持状态筛选；创建任务入口。
- 说明：XUSHU 只读 Git，不修改仓库。

### 7.4 任务 Tasks

- 页头 + “新建任务”主操作。
- 筛选栏：项目 Select、状态 Select、搜索框、显示归档开关。
- 任务表格：标题、状态、优先级、修订、更新时间、项目；行点击进入详情。
- 优先级用文字+图标，不用纯色点。
- 空状态区分“没有项目”与“没有匹配任务”。

### 7.5 任务详情 Task Detail

- 页头：任务标题、项目链接、状态 Badge、优先级、操作。
- **StateTrack** 展示状态序列；任务已归档/项目归档时只读提示。
- 主列：目标描述、验收条件、状态历史（StateSpine）。
- 侧列：完成门禁（必需/已满足/阻塞原因）、元数据（修订号、创建/更新时间、当前阻塞原因）、下一步建议。
- 验收条件行：类型（自动/人工）、必需/可选、满足状态、修订号；编辑/删除操作。
- 人工确认只在“待验证”状态可用；按钮说明当前是否可确认。
- 状态转换按后端 `allowedTransitions` 渲染；完成按钮在 `canComplete=false` 时禁用并列出原因。
- 重新开启只对 cancelled 显示，需填写原因。

### 7.6 本地数据与系统健康

- 保留 Phase 1 的持久化测试记录与健康状态。
- 健康摘要使用紧凑状态行；数据库/迁移/记录数真实展示。
- 测试记录列表用统一表格/行样式，不另造一套视觉。

### 7.7 设置 Settings

- 外观：浅色/深色/跟随系统（原生 radio，键盘可用）。
- 关于：应用版本、Tauri、阶段、平台。
- 本地数据：数据库文件名、隐私边界、无云同步说明。
- 不展示绝对路径、密钥或 token。

### 7.8 验收中心 Verification Center

- 页头：验收中心 / 待验证任务与真实证据 / 主操作“导出验收报告”（需选中任务）。
- 布局：宽屏为 `340px` 待验收队列 + 右侧证据工作区；窄于 `xl` 时纵向堆叠。
- 左栏：任务标题、项目名、自动条件通过数、人工待确认数、门禁状态、运行中图标；选中项使用 `accent` 与品牌边线，不只靠颜色。
- 右栏：
  - 任务摘要：状态、优先级、修订、更新时间、自动/人工/门禁三项计数、具体 blockers；
  - 验收条件：自动条件显示 `VerificationBadge`、命令、cwd、当前判定、过期原因、配置/运行/取消/证据/历史操作；人工条件显示确认状态并链接到任务详情确认；
  - 报告导出：写入应用数据目录 `exports/`，Dialog 展示文件名、Markdown 与复制按钮，不展示绝对路径。
- 批准 Dialog：命令使用 `font-mono` 原样展示，列出 cwd、解析到的程序、超时、输出上限、成功规则与风险；必须勾选确认后才能启用“批准并运行”。
- 运行状态：旋转图标 + “运行中”文字 + 取消按钮；结束后显示退出码、耗时、结果与证据入口。
- 所有验证状态使用既有 `verification-*` Tokens；不得新增第二套状态颜色。
- 空队列使用统一 `EmptyState`，引导去任务页配置自动条件；不展示假任务或假证据。

## 8. 可访问性与交互

- 所有交互元素有可见 `focus-visible` ring（品牌色）。
- 语义化 landmark：`aside`/`nav`/`header`/`main`。
- 对话框 `role="dialog"` + `aria-modal` + 标题关联；Escape 关闭；关闭后焦点返回触发按钮。
- 表单 Label 与控件关联；错误 `role="alert"`；必填/可选状态明确。
- 状态不只靠颜色；图表/信号轨有文字标签。
- 表格有表头；行操作有可读名称。
- 键盘可完成主流程：导航、筛选、创建、状态转换、确认、归档。
- 支持 `prefers-reduced-motion`；不强制动画。
- 深色/浅色下均检查对比度；正文目标 ≥ 4.5:1。

## 9. 实现与治理

- Token 落在 `apps/desktop/src/index.css`；组件落在 `apps/desktop/src/components`。
- 页面禁止写死与 Token 冲突的颜色/圆角/阴影；新增 Token 需更新本文档。
- 新增页面/组件必须复用 App Shell、PageHeader、Panel、StatusBadge、EmptyState 等。
- 变更全局设计语言必须说明原因与影响范围，不能只改单页。
- 每次页面变更更新 `docs/design/UI_AUDIT.md`。
- 每次 UI 任务先调用 `frontend-design` Skill；不得使用子 Agent。
- 不自动截图/录屏；视觉检查由用户手动提供截图。未完成视觉检查时标记 `VISUAL_REVIEW_PENDING`。
- 自动化类型/Lint/测试/构建通过不等于视觉验收通过。

## 9.1 Phase 3 实现说明

- 验收中心与任务详情复用 App Shell、PageHeader、Card、Badge、Dialog、Loading/Empty/Error 与 StatusBadge。
- 验证状态使用 `VerificationBadge`；执行状态（运行中/已完成/取消/超时/启动失败/中断）使用图标 + 文字，结果仍由 `VerificationBadge` 表达。
- 命令、cwd、哈希、退出码使用 `font-mono`；长文本 `break-all` 或 `truncate + title`，不撑破布局。
- 未自动截图；视觉验收继续保持 `VISUAL_REVIEW_PENDING`。

## 10. 设计计划自检

本方案刻意避开通用 AI 页面常见默认：没有暖米色+衬线+陶土色，没有黑底酸性绿，没有全卡片 SaaS 套件，没有装饰性全大写 eyebrow。XUSHU 的识别点集中在“状态序列/证据”结构与冷石墨+氧化青+黄铜色板；信息结构服务于“可信执行与验证”，而不是装饰。

**视觉验收状态：VISUAL_REVIEW_PENDING**（等待用户手动截图确认）。