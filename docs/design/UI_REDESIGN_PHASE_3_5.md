# Phase 3.5 报告：桌面 UI/UX 产品化重设计

- 日期：2026-11-06
- 项目：序枢 XUSHU
- 根目录：`D:\MyProject\xushu`
- 基线提交：`f3ff6a8 feat: 完成 Phase 3 独立验收引擎`
- 任务性质：全局 UI/UX 审计、桌面窗口一体化、视觉系统优化、现有功能界面精修
- 技术交付门禁：**PASS**
- UI 人工视觉验收：**VISUAL_REVIEW_PENDING**
- Git：本轮**未创建提交**，等待用户审查
- Phase 4：**未进入**

## 1. 旧界面的主要问题

1. **顶部三层断层**：原生标题栏 → 56px 应用顶栏 → 页面标题，背景、边框与高度不一致。
2. **信息重复**：当前项目名称在侧栏、顶栏、页面标题与卡片中反复出现。
3. **卡片墙**：工作台由多个 Card 嵌套组成，空状态又被包在 Card 内。
4. **机械统计**：七种任务状态铺成七张等宽统计卡，没有回答“下一步做什么”。
5. **解释性副标题过多**：每个标题下都有说明文字，信息密度低、视觉噪音高。
6. **上下文弱**：当前项目、Git 状态、待验收事项与最近验证不在一个清晰的工作流中。
7. **操作分散**：任务行缺少行内操作；项目列表与任务列表的扫描效率不足。
8. **证据表达不够突出**：验收中心的命令、运行状态、证据与历史仍像普通表单；日志阅读空间不足。
9. **视觉系统不统一**：部分页面硬编码 emerald 等颜色，Card、Badge、Tabs 风格重复但不一致。

## 2. 研究过的成熟软件与实际借鉴原则

| 参考 | 借鉴原则 | 未复制的内容 |
|---|---|---|
| Obsidian | 窗口与工作区的一体感、固定侧栏、内容区独立滚动 | 品牌资产、主题、完整布局 |
| Linear | 安静的信息层级、下划线 Tabs、密集列表、克制的强调色 | 品牌色、图标体系、界面 |
| Raycast | 紧凑命令表面、主操作优先、行内操作与键盘效率 | 命令面板视觉、品牌 |
| VS Code | 侧栏 / 工作区 / 主滚动区结构、上下文与状态分离 | 编辑器 UI、图标 |
| GitHub Desktop | 仓库分支/HEAD/dirty 的紧凑表达、操作反馈 | 品牌、布局 |

XUSHU 的独立识别点：

- **Evidence Workbench**：项目上下文、状态序列、命令终端与证据在同一工作区；
- 冷石墨中性色 + 氧化青信号 + 黄铜等待色；
- 自动验证、人工确认、AI 声明在视觉上来源可辨；
- 不把“通过”当作装饰，而把它当作需要证据的状态。

## 3. frontend-design Skill 使用情况

- 在开始 Phase 3.5 UI 前实际调用 `frontend-design` Skill（当前主会话，2026-11-06）。
- 先形成设计计划，再对照 brief 审查：
  - **Color**：冷石墨 canvas/sidebar/workspace 三层 + 氧化青主操作 + 黄铜等待 + 珊瑚失败；未使用暖米色/衬线/陶土色、黑底酸性绿等默认 AI 风格。
  - **Type**：Segoe UI Variable / Microsoft YaHei UI；Cascadia Code 用于命令、cwd、HEAD、哈希、修订号。
  - **Layout**：原生标题栏 + 240px 侧栏 + 48px 页面上下文行 + 1280px 内容；无全宽应用顶栏。
  - **Principles**：证据优先、减少容器、一个强调色、原生行为优先、密度有呼吸。
- 设计计划审查后保留了“原生标题栏 + 一体化侧栏”的方案；没有为了视觉自由改用高风险的自定义标题栏。
- 未使用子 Agent；未截图/录屏。

## 4. 自定义标题栏方案与取舍

### 4.1 研究

评估了 Tauri 2 的：

- Native window decorations；
- Custom titlebar / frameless window；
- `data-tauri-drag-region`；
- Window API（`startDragging`、`minimize`、`toggleMaximize`、`close`、`setTheme`）；
- Windows 窗口缩放、Snap Layouts、DPI、多显示器与焦点行为。

### 4.2 选择：保留原生 Windows 标题栏

本轮**不采用** custom titlebar，原因：

- 自定义标题栏需要自行处理拖动、双击最大化/还原、最小化、最大化、关闭、边缘缩放；
- Windows 11 Snap Layouts 依赖原生 caption 区域，自定义按钮无法可靠复刻；
- 多显示器、不同 DPI、焦点与辅助功能需要在更多实机组合验证；
- 当前单显示器/单 DPI 环境不足以证明自绘窗口行为不退化；
- 用户明确要求“不要为了视觉效果牺牲稳定性和用户操作习惯”。

实现：

- 保留 `decorations: true`；
- 移除应用级 56px 顶栏；
- 侧栏品牌行与 PageHeader 标题行均为 48px，形成单一顶部工作带；
- 当前项目、分支/HEAD/dirty 与项目详情入口集中到侧栏；
- 通过窗口级 `plugin:window|set_theme`（`label: main`）同步原生标题栏主题；
- capabilities 仅增加 `core:window:allow-set-theme`，不开放 Shell/FS/任意窗口控制。

### 4.3 取舍

**收益：**

- 原生拖动、双击、最小化/最大化/关闭、边缘缩放、Snap、DPI 与焦点行为完整保留；
- 视觉断层从三层降为“原生标题栏 + 单一工作区”；
- 安全面只增加一个窄范围主题权限。

**代价：**

- 原生标题栏仍是一条 OS 管理的横条，无法做到 Obsidian 式完全客户区绘制；
- 标题栏外观受 Windows 主题与版本影响；
- 本轮未验证多显示器与其它 DPI 组合。

完整 ADR：`docs/architecture/ADR/0005-native-window-integrated-shell.md`。

## 5. 新的统一视觉设计规范

完整规范见 `docs/design/DESIGN_SYSTEM.md` v2.0。核心：

### 5.1 Surface Hierarchy

`background`（窗口）→ `sidebar`（240px 工作区侧栏）→ `workspace`（主内容）→ `card`（少量独立面板）→ `popover`（Dialog/Sheet）→ `code`（命令/证据终端）。

- 侧栏与主内容仅一条竖向 `line` 分隔；
- 页面区块优先 `border-y` / `divide-y`；
- Card 只用于独立交互或安全提示；
- 命令与证据使用深色终端块。

### 5.2 Color

- 冷石墨 + 氧化青 + 黄铜 + 珊瑚；
- 浅色：`#EEF3F1` canvas / `#E6EDEA` sidebar / `#F8FAF9` workspace；
- 深色：`#0A1012` canvas / `#0D1618` sidebar / `#111A1C` workspace；
- 青色只用于主操作、选中、运行中；黄铜用于等待/注意；珊瑚用于失败/阻塞；
- 所有页面使用语义 Token，不硬编码颜色。

### 5.3 Typography

- UI：Segoe UI Variable Text / Microsoft YaHei UI；
- Mono：Cascadia Code；
- 页面标题 19px/600；区块标题 13px/600；正文 13px；
- 元数据 11px；标题字距 -0.01em；
- 命令、cwd、Git HEAD、哈希、修订号使用 mono；
- 不使用全大写装饰标签或每个标题下的解释性副标题。

### 5.4 Spacing / Radius / Elevation

- 4px 基准；页面左右 20/32px；区块间距 20–24px；表格行 40px；表头 32px；控件 28–32px。
- Radius 3/5/8/12px；不把所有组件做成相同大圆角。
- 面板无阴影；Dialog/Sheet/Dropdown/Toast 使用单一 `shadow-overlay`。

### 5.5 Status

- 任务状态：7 种；验证结果：PASS/FAIL/NOT_RUN/INCONCLUSIVE/STALE；执行状态单独表达；
- 全部使用图标 + 中文 + 颜色；
- 阻塞显示原因；过期显示 staleReason；完成门禁列出具体 blockers。

### 5.6 Component System

- Button/Input/Select/Textarea 紧凑化；
- Tabs 改为下划线；
- Table 紧凑行高；
- Dialog 用于短表单/批准；Sheet 用于证据/历史/报告；
- Section/SectionHeader 替代大部分 Card；
- Empty/Loading/Error 统一且不嵌套 Card；
- CommandBlock 深色终端；CriterionVerification 轻量化。

## 6. 每个页面的主要改动

### 6.1 App Shell

- 删除 Phase 3 的 56px 应用顶栏。
- 侧栏 240px：品牌、当前项目选择、分支/HEAD/dirty、导航、主题、版本。
- 导航项显示阻塞/待验证计数；任务与验收中心有真实数字时才显示。
- 主内容独立滚动；PageHeader 与侧栏品牌行对齐；内容最大 1280px。
- 原生标题栏主题同步。

### 6.2 工作台

- 从“卡片 + 七状态统计”改为当前项目作战台：
  - SignalRail 一行状态计数；
  - 下一步 strip（阻塞/失败/待验证优先）；
  - 需要处理列表；
  - 最近任务列表；
  - 验证动态（PASS/FAIL、退出码、时间）；
  - 当前仓库、系统、验收中心侧栏；
  - 无项目时 plain 空状态 + 三步工作流提示。

### 6.3 项目

- 下划线 Tabs；行式项目列表；当前项目点；路径、分支/HEAD/dirty 紧凑行；打开与操作菜单。
- 项目详情：返回链接、仓库 strip、任务表、项目信息侧栏；不再用两张 Card 叠放。

### 6.4 任务

- 单行工具栏（搜索/项目/状态/包含归档/清除/匹配数）。
- 密集任务表；状态、优先级、项目、修订、更新时间；行操作菜单（打开/归档/恢复）。
- 任务详情：StateTrack strip、目标/验收/历史主列、完成门禁/状态操作/元数据侧栏；自动验证内嵌 CommandBlock；证据/历史 Sheet。

### 6.5 验收中心

- 300px 待验收队列 + 证据工作区；窄屏纵向。
- 任务摘要、三项统计 strip、blockers strip、验收条件列表。
- 自动条件：VerificationBadge、CommandBlock、运行/取消/查看证据/历史；人工条件链接任务详情确认。
- FAIL/PASS/过期原因清晰展示。
- 报告使用右侧 Sheet，Markdown 深色代码块 + 复制。

### 6.6 验证配置 / 批准

- 配置 Dialog 保留模板、程序、参数、cwd、超时、输出上限、成功规则与风险提示。
- 批准 Dialog 以深色 CommandBlock 为核心，展示解析程序、cwd、超时、输出上限、成功规则与风险；复选框确认后才可运行。

### 6.7 证据 / 历史

- 从居中 Dialog 改为右侧 Sheet：
  - 固定 Header，Body 独立滚动；
  - 证据：摘要、哈希、敏感性标记、脱敏输出（深色终端）。
  - 历史：执行状态、验证结果、退出码、耗时、命令、修订、证据入口。

### 6.8 设置 / 诊断

- 设置：分组行式列表、主题 radio 行、关于/数据定义列表；无大卡片。
- 诊断：plain 系统健康分区、探针说明、表单/记录两栏；无 Card 叠 Card。
- 移除探针成功框中的硬编码 emerald 颜色，改用语义 success Token。

### 6.9 404 / Error Boundary

- 404 保留简洁空状态。
- Error Boundary 改为统一 Token，去掉 `shadow-sm`/`rounded-xl` 等旧视觉。

## 7. 桌面窗口功能测试

### 7.1 方案

保留原生 Windows 标题栏；使用 Win32 + PowerShell 操作真实窗口，使用 WebView2 CDP 读取页面尺寸与主题。

### 7.2 窗口行为

| 测试 | 方法 | 结果 |
|---|---|---|
| 最小化 | `ShowWindowAsync(SW_MINIMIZE)` + `IsIconic` | PASS |
| 最大化 | `ShowWindowAsync(SW_MAXIMIZE)` + `IsZoomed` | PASS |
| 还原 | `ShowWindowAsync(SW_RESTORE)` | PASS |
| 关闭 | `WM_CLOSE` | PASS，进程退出 |
| 程序化缩放 | `SetWindowPos` | PASS，页面保持可用 |
| 原生边缘缩放 | 原生 decorations 保留 | 系统提供；未自动化拖拽 |
| Snap Layouts | 原生 caption buttons 保留 | 未自动化悬停验证 |
| 双击标题栏 | 原生 decorations 保留 | 系统提供；未自动化验证 |
| 多显示器 | 未测试 | 单显示器环境 |
| 焦点/失焦 | 原生标题栏 + 内容 focus ring | 待人工视觉确认 |

> 由于未采用 custom titlebar，拖动、双击、Snap、边缘缩放、DPI 与焦点由 Windows 原生窗口提供；未引入 `data-tauri-drag-region` 或自定义窗口按钮。

### 7.3 原生标题栏主题

| 应用主题 | `documentElement.dark` | 原生窗口主题 |
|---|---|---|
| 深色 | true | `dark` |
| 浅色 | false | `light` |
| 跟随系统 | 当前系统浅色 → false | `light` |

- 通过 `plugin:window|theme` 实测；`core:window:allow-set-theme` 生效。
- 主题同步失败只影响标题栏配色，不影响页面主题。

## 8. 深浅主题与窗口缩放检查

### 8.1 自然环境：150% 缩放

| 外层窗口（物理） | CSS 视口 | 7 页面横向溢出 |
|---|---|---|
| 1366 × 768 | 896 × 475 | 无 |
| 1600 × 900 | 1052 × 563 | 无 |
| 1920 × 1080 | 1266 × 683 | 无 |

### 8.2 强制 100% WebView 缩放

| 外层窗口（物理） | CSS 视口 | 7 页面横向溢出 |
|---|---|---|
| 1366 × 768 | 1344 × 712 | 无 |
| 1600 × 900 | 1578 × 844 | 无 |
| 1920 × 1080 | 1898 × 1024 | 无 |

覆盖页面：工作台、项目、任务、验收中心、任务详情、设置、诊断。

检查项：

- `main.scrollWidth <= main.clientWidth`；
- `documentElement`/`body` 无整体横向溢出；
- 排除内部表格滚动容器与 `pre` 后，无可见溢出元素；
- 侧栏 240px 固定；
- 页面标题、项目上下文、操作区可见；
- 深/浅主题 Token 切换正确。

结果：**PASS（技术测量）**。视觉细节仍需用户截图确认。

## 9. 所有真实测试命令和退出码

最终复跑（2026-11-06）：

| 命令 | 退出码 | 结果 |
|---|---|---|
| `pnpm install --frozen-lockfile` | 0 | 锁文件一致 |
| `pnpm typecheck` | 0 | 通过 |
| `pnpm lint` | 0 | ESLint 10，`--max-warnings 0` |
| `pnpm test` | 0 | 7 个测试文件、40 个测试通过 |
| `pnpm build` | 0 | JS 774.76 kB / gzip 233.28 kB |
| `pnpm format` | 0 | Prettier 通过 |
| `cargo fmt --all -- --check` | 0 | 通过 |
| `cargo check --workspace --all-targets --locked` | 0 | 通过 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | 通过 |
| `cargo test --workspace` | 0 | 11 + 19 + 12 + 26 = 68 项通过 |
| 桌面 UI 数据/交互 E2E | 0 | 重跑验证、人工确认、done、历史/证据 Sheet |
| 桌面窗口/尺寸/主题测量 | 0 | 3 尺寸 × 2 缩放 × 7 页面，无横向溢出 |

桌面交互关键结果：

```json
{
  "runCompleted": true,
  "gateSatisfied": true,
  "taskDone": true,
  "completionSatisfied": true,
  "historySheetOpen": true,
  "historySheetClosed": true
}
```

稳定后复核：

```json
{
  "passVisibleAfterSettle": true,
  "evidenceSheetContent": true,
  "evidenceOutputVisible": true,
  "failVisible": true,
  "failedCriterionVisible": true,
  "blockerVisible": true,
  "passInActivity": true,
  "failInActivity": true
}
```

> 首次自动检查存在 TanStack Query 刷新时序：新运行完成后页面尚未更新 PASS/证据面板；等待查询刷新后复核全部通过。该现象是测试时序，不是业务缺陷。

## 10. 未完成和未验证事项

1. **人工视觉验收**：`VISUAL_REVIEW_PENDING`；需用户手动截图。
2. **Snap Layouts 悬停行为**：原生 caption buttons 保留；未自动化验证。
3. **双击标题栏**：原生行为保留；未自动化验证。
4. **多显示器与跨屏 DPI**：未测试；当前为单显示器 150% 环境。
5. **DPI 组合**：自然 150% 与强制 100% 已验证；125%/175%/200% 未验证。
6. **macOS**：未验证字体回退、窗口装饰与 DPI。
7. **生产 CSP / 安装包**：未用 `tauri build` 成品运行验证。
8. **前端 bundle**：774.76 kB（gzip 233.28 kB），仍有 >500 kB 提示。
9. **窗口焦点/失焦视觉**：原生标题栏 + focus ring；待人工确认。
10. **Git**：本轮未提交；基线为 `f3ff6a8`。
11. **本地测试数据**：真实桌面验证在本地数据库新增了名为 `Phase 3.5 UI 审阅` 的项目，含 4 个任务（完成/失败待验收/阻塞/草稿）与验证记录；用于人工视觉复核，不是用户真实项目，可在项目页软归档。未删除任何用户数据。

## 11. 人工视觉验收清单

详见 `docs/design/UI_AUDIT.md` 第 7 节。至少包括：

- 工作台有/无项目；
- 项目列表与详情；
- 任务列表与详情（待验证/完成/阻塞/取消）；
- 验收中心空/队列/PASS/FAIL/过期；
- 验证配置与批准 Dialog；
- 运行中与取消；
- 证据/历史/报告 Sheet；
- 设置与诊断；
- 原生标题栏深/浅色与最小化/最大化/还原/关闭；
- 1366×768、1600×900、1920×1080；
- 键盘焦点、Tab、Escape、行操作菜单。

不自动截图；由用户提供浅色/深色截图。

## 12. Phase 3.5 技术交付门禁结论

| 验收目标 | 结果 | 证据 |
|---|---|---|
| 顶部窗口断层处理 | PASS（技术+交互） | 移除应用顶栏；侧栏与 PageHeader 单工作带；原生标题栏主题同步 |
| 原生窗口行为 | PASS（已测项） | 最小化/最大化/还原/关闭/缩放；Snap/双击保留原生，未自动化 |
| 统一 App Shell | PASS | 240px 侧栏、1280px 内容、统一 PageHeader/Section |
| 减少卡片墙 | PASS | 工作台/项目/任务/验收/设置/诊断改为分区/列表/表格 |
| 统一视觉系统 | PASS | DESIGN_SYSTEM v2.0 + 语义 Token；无页面硬编码颜色 |
| 工作台信息层级 | PASS | 下一步/需要处理/最近任务/验证动态/仓库/系统 |
| 验收中心与证据 | PASS | 队列/命令/批准/运行/证据/历史/报告 Sheet |
| Phase 1–3 业务回归 | PASS | 前端 40 + Rust 68 测试；桌面 E2E 完成验证→人工确认→done |
| 深浅主题 | PASS（技术） | 原生标题栏 dark/light/system；待人工视觉 |
| 窗口尺寸 | PASS（技术） | 150%/100% × 1366/1600/1920 × 7 页面，无横向溢出 |
| 人工视觉验收 | PENDING | `VISUAL_REVIEW_PENDING` |
| 不进入 Phase 4 | PASS | 未开发 Hooks 或后续功能 |
| Git | 未提交 | 等待用户审查 |

**技术交付门禁：PASS。**
**视觉交付门禁：VISUAL_REVIEW_PENDING。**
**本轮未创建 Git 提交。**

## 13. 标准汇报摘要

```text
当前阶段 / 任务：Phase 3.5 — 桌面 UI/UX 产品化重设计
实施摘要：移除应用顶栏，保留原生 Windows 标题栏并同步主题；建立 240px 侧栏 + PageHeader 的一体化工作区；重写工作台、项目、任务、任务详情、验收中心、设置与诊断；新增 Section/Sheet/CommandBlock；减少卡片墙，统一语义 Token、状态表达与深浅主题。
修改文件：见 UI_AUDIT 组件清单与 Git 状态；本轮未提交。
实际执行的命令与退出码：pnpm 全套 0；cargo 全套 0；桌面窗口/尺寸/主题/业务交互验证通过。
测试：PASS — 前端 40、Rust 68、桌面 E2E；NOT_RUN — 人工视觉截图、Snap 悬停、多显示器、其它 DPI、macOS、生产 CSP/安装包。
安全/隐私检查：未读取/输出密钥；未修改全局配置；未使用子 Agent；未截图；仅新增 core:window:allow-set-theme；未削弱 Phase 3 门禁。
明确未完成或尚未验证：VISUAL_REVIEW_PENDING；Snap/双击/多显示器/其它 DPI 未自动化；生产 CSP/安装包未验证。
下一步：等待用户手动截图审查；根据截图修正视觉问题；不改业务逻辑、不进入 Phase 4、不自动提交。
```