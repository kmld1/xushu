# Phase 3.6 报告：窗口一体化与顶部融合重构

- 日期：2026-11-06
- 项目：序枢 XUSHU
- 根目录：`D:\MyProject\xushu`
- 基线提交：`f3ff6a8 feat: 完成 Phase 3 独立验收引擎`
- 任务性质：窗口结构重评估、自定义标题栏、顶部工作带、工作台首屏重构、响应式窄窗口
- 技术交付门禁：**PASS**
- UI 人工视觉验收：**VISUAL_REVIEW_PENDING**
- Git：本轮**未创建提交**，等待用户审查
- Phase 4：**未进入**

## 1. 本轮要解决的问题

Phase 3.5 已移除应用级顶栏，但视觉审查仍确认：

1. Windows 原生标题栏与应用内容形成“标题栏一层、应用内容一层”的两层结构；
2. 左侧栏虽然与页面标题行对齐，但没有从窗口顶端真正生长出来；
3. 工作台仍以“工作台”大标题开场，保留管理后台感；
4. 顶部区域无法容纳品牌、项目上下文、页面名称、主题与窗口控制于同一视觉表面；
5. 应用看起来仍像“网页套在桌面壳里”。

本轮目标不是继续局部美化，而是从窗口结构层面重做顶部：

- 让窗口最上方成为 XUSHU 自己的工作带；
- 评估并实现 Tauri 2 自定义标题栏；
- 恢复 Windows 11 Snap Layouts；
- 工作台首屏优先呈现项目上下文、焦点任务、最近验证与下一步；
- 保持 Phase 1–3 的 Rust、SQLite、IPC、验证引擎与完成门禁不变。

## 2. frontend-design Skill 与设计计划

### 2.1 Skill 使用

- 开始前实际调用 `frontend-design` Skill（当前主会话，2026-11-06）。
- 未使用子 Agent，未截图/录屏。
- 先形成设计计划，再对照 brief 审查，然后实现。

### 2.2 设计计划

**Color**

- 保留 Phase 3.5 的冷石墨 + 氧化青 + 黄铜 + 珊瑚语义色板。
- 新增两个窗口级 Token：`window-control-hover`（浅 `#DBE5E1` / 深 `#1B2A2D`）与 `window-close-hover`（`#C42B1C`，Windows 关闭语义）。
- 工作带左侧使用 `sidebar`，右侧使用 `workspace`；不使用第二种背景或渐变。
- 命令、证据、路径继续使用 `code-*` 深色终端。

**Type**

- UI：Segoe UI Variable Text / Microsoft YaHei UI；Mono：Cascadia Code。
- 工作带品牌 13px/600；项目上下文 10–12px；页面名称 13px/600；窗口控制使用 10×10 线性图标。
- 一级页面不再使用 19px 大标题；详情页实体标题保持 19px/600。

**Layout**

```text
┌──────────────────────────────────────────────────────────────────────┐
│ 序 序枢 XUSHU │ 当前项目 [项目 ▾]  main/HEAD ● │ 工作台 │ ☀ ☾ ◐ │ – □ × │ 44
├───────────────┼──────────────────────────────────────────────────────┤
│ 导航          │ 工作台首屏：状态条 → 焦点任务 → 最近验证             │
│               │ 需要处理 / 最近任务 / 仓库 / 系统                    │
└───────────────┴──────────────────────────────────────────────────────┘
```

- 一个 44px 工作带，从左到右：品牌区（240px）→ 项目上下文 → 页面名称 → 主题 → 窗口控制（138px）。
- 左侧品牌区与侧栏同宽同色；<760px 收为 52px 图标轨，项目选择器仍保留。
- 主内容 max 1280px；顶部 16px；侧栏独立，主内容独立滚动。
- 窗口最小尺寸 500×480，以支持 Snap 贴靠与窄窗口。

**Principles**

- 一个窗口只允许一个第一层工作带；不允许原生标题栏或第二层应用栏。
- 工作台从工作流开始，不从管理后台标题开始。
- 原生行为优先：自定义外观，但不牺牲拖动、双击、Snap、缩放与关闭。
- 状态表达继续使用图标 + 中文 + 颜色；不把 AI 文本当证据。
- 减少容器，依靠表面、分隔线、留白与行式结构。

### 2.3 自检

对照通用 AI 设计默认，本轮明确避开：

- 暖米色 + 衬线 + 陶土色；
- 黑底 + 酸性绿；
- 全卡片 SaaS 套件；
- 每个标题上的全大写 eyebrow；
- 大标题 + 副标题的营销式首屏。

保留的识别点是 XUSHU 自己的“证据工作台”：窗口顶端工作带、项目上下文、状态序列、命令终端与证据 Sheet。

## 3. Tauri 2 自定义标题栏评估

### 3.1 能力核对

核对 Tauri 2.12 官方文档与本地 crate 源码：

- `decorations: false` 可移除原生标题栏；
- `data-tauri-drag-region` 支持拖动；Tauri 2.12 的拖动脚本在双击时调用 `internal_toggle_maximize`；
- 窗口控制通过 `minimize`、`toggle_maximize`、`close`、`is_maximized`；
- `shadow: true` 在 Windows 11 提供窗口阴影与圆角；
- Tauri 2 没有 Windows Snap Layouts 的一等 API（issue #4531 仍开放）。

### 3.2 WebView2 命中测试约束

`decorations: false` 后 WebView2 子窗口覆盖客户区并先接收鼠标消息。只在 Tauri 主窗口的 `WM_NCHITTEST` 返回 `HTMAXBUTTON` 不会生效。Microsoft 官方要求自定义标题栏在最大化按钮区域返回 `HTMAXBUTTON`，Windows 11 才会显示 Snap Layouts。

### 3.3 采用方案

**自定义标题栏 + 最小原生 Snap 覆盖层。**

- `decorations: false`、`shadow: true`、`minWidth: 500`、`minHeight: 480`。
- 44px 工作带由 React 绘制；窗口控制由 React 绘制并调用 Tauri 窗口命令。
- 拖动使用 `data-tauri-drag-region="deep"`；双击使用 Tauri 内置行为。
- 在最大化按钮位置创建不可见原生子窗口 `XushuSnapOverlay`：
  - `WM_NCHITTEST → HTMAXBUTTON`；
  - `WM_NCLBUTTONDOWN` 激活父窗口并切换最大化/还原；
  - `WM_NCMOUSEMOVE` / `WM_NCMOUSELEAVE` 发出悬停事件；
  - `WM_SIZE` / `WM_DPICHANGED` 更新位置与尺寸；
  - DWM 圆角 + 无边框色。
- 不启用 `withGlobalTauri`；不引入第三方 Tauri 插件；`windows-sys 0.61.2` 版本不变。

### 3.4 未采用方案

| 方案 | 原因 |
|---|---|
| 保留原生标题栏（ADR-0005） | 无法解决顶部两层结构。 |
| 无装饰窗口 + 不恢复 Snap | 最大化按钮悬停菜单退化。 |
| `tauri-plugin-frame` / `decorum` | 注入自带 HTML 标题栏、需要额外插件依赖或 `withGlobalTauri`，与已有工作带重叠。 |
| 只在主窗口处理 `WM_NCHITTEST` | WebView2 先接收命中测试，不生效。 |

完整决策见 `docs/architecture/ADR/0006-custom-titlebar-window-chrome.md`。

## 4. 实现内容

### 4.1 窗口与 Rust

新增 `apps/desktop/src-tauri/src/window_chrome.rs`：

- `install`：在主窗口上安装 `XushuSnapOverlay` 子窗口；
- `WM_NCHITTEST` 返回 `HTMAXBUTTON`；
- 点击最大化按钮直接调用 Tauri `is_maximized` + `maximize/unmaximize`，避免前端事件时序问题；
- 悬停事件 `xushu://chrome/snap-enter` / `snap-leave` 交给前端高亮；
- DWM `DWMWA_WINDOW_CORNER_PREFERENCE=DWMWCP_ROUND`；
- DWM `DWMWA_BORDER_COLOR=DWMWA_COLOR_NONE`，避免深色窗口白边；
- 单元测试验证 96/144/192 DPI 下的逻辑像素缩放。

`lib.rs` 在 setup 中安装覆盖层；安装失败只记录错误，不阻止应用启动。

### 4.2 Tauri 配置与权限

`tauri.conf.json`：

- `decorations: false`
- `shadow: true`
- `minWidth: 500`
- `minHeight: 480`
- `center: true`

capabilities：

- `core:default`
- `core:window:allow-set-theme`
- `core:window:allow-start-dragging`
- `core:window:allow-internal-toggle-maximize`
- `core:window:allow-minimize`
- `core:window:allow-toggle-maximize`
- `core:window:allow-close`
- `core:window:allow-is-maximized`

未开放 Shell、FS、SQL 或任意窗口创建/销毁权限。

### 4.3 顶部工作带

新增 `apps/desktop/src/app/workspace-chrome.tsx`：

- 44px 全宽工作带；
- 左侧 240px / <760px 52px 品牌区，与侧栏同色；
- 当前项目选择器、branch/HEAD/dirty、项目详情入口；
- 当前页面名称（路由映射）；
- 主题控件；
- 46×44px 最小化/最大化/还原/关闭；
- `data-tauri-drag-region="deep"` 的空白拖动区；
- 窄窗口隐藏“当前项目”标签与 Git 摘要，但保留项目选择器。

新增 `apps/desktop/src/components/window-controls.tsx`：

- `getCurrentWindow()` 的最小化、切换最大化、关闭；
- `onResized` 同步最大化图标；
- 窗口级 `listen` 接收 Snap 悬停事件；
- 键盘可访问的中文名称与 focus ring；
- 在非 Tauri 测试环境安全降级，不调用 IPC。

### 4.4 App Shell

`app-layout.tsx`：

- 移除侧栏品牌、当前项目与主题（上移到工作带）；
- 侧栏只保留导航与底部本地信息；
- 240px / <760px 52px 图标轨；窄窗口隐藏文字与计数数字，保留可访问名称与状态圆点；
- 主内容 `overflow-y-auto overflow-x-hidden`，`scrollbar-gutter: stable`；
- 主内容顶部 16px，页面内容最大 1280px。

### 4.5 页面调整

- **工作台**：移除大标题；首屏改为 SignalRail → 焦点任务 → 最近验证；下方需要处理/最近任务；侧栏当前仓库/系统/验收中心。
- **项目**：移除独立页头；Tabs 与导入按钮同行。
- **任务**：移除独立页头；新建任务进入筛选工具栏。
- **验收中心**：移除独立页头；导出报告进入证据工作区标题行。
- **设置/诊断**：移除独立页头；使用 `sr-only h1`。
- **项目详情/任务详情**：保留 `PageHeader`；补充栅格 `min-w-0`，修复窄窗口内部溢出。
- **404**：保留简洁空状态；工作带显示“未找到”。

### 4.6 Tokens

`index.css` 新增：

- `--window-control-hover`：浅 `#DBE5E1` / 深 `#1B2A2D`
- `--window-close-hover`：`#C42B1C`

全部通过 `@theme inline` 暴露为 `bg-window-control-hover` / `bg-window-close`。

## 5. 实际执行的测试

### 5.1 最终质量门禁

| 命令 | 退出码 | 结果 |
|---|---|---|
| `pnpm typecheck` | 0 | 通过 |
| `pnpm lint` | 0 | ESLint 10，`--max-warnings 0` |
| `pnpm test` | 0 | 7 个测试文件、40 个测试通过 |
| `pnpm build` | 0 | JS 795.07 kB / gzip 238.09 kB；CSS 52.81 kB / gzip 9.71 kB |
| `pnpm format` | 0 | Prettier 通过 |
| `cargo fmt --all -- --check` | 0 | 通过 |
| `cargo check --workspace --all-targets --locked` | 0 | 通过 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | 无警告 |
| `cargo test --workspace` | 0 | 11 + 19 + 12 + 26 + 1 = 69 项通过 |

### 5.2 桌面窗口行为

使用真实 Tauri 窗口、Win32 消息与临时 CDP 会话；临时调试配置与脚本位于 `target/e2e/`（已忽略，不提交）。

| 测试 | 方法 | 结果 |
|---|---|---|
| 最小化 | 点击自绘按钮 + `IsIconic` | PASS（`iconic=true`） |
| 最大化 | 悬停后点击 Snap 覆盖层 + `IsZoomed` | PASS |
| 还原 | 再次点击覆盖层 | PASS |
| DOM 最大化按钮 | 点击并读取 `aria-label` | PASS（最大化 → 还原） |
| 关闭 | 点击自绘关闭按钮 | PASS（进程退出） |
| 拖动 | 真实鼠标拖动标题带 | PASS（窗口位移 +120/+48） |
| 双击最大化 | CDP 双击标题带空白区 | PASS（`IsZoomed=true`） |
| Snap 覆盖层 | `EnumChildWindows` | PASS（144 DPI：69×66；96 DPI 逻辑值 46×44） |
| Snap 命中 | `SendMessage(WM_NCHITTEST)` | PASS（返回 9 / `HTMAXBUTTON`） |
| Snap 悬停菜单视觉 | Windows 系统 UI | **NOT_RUN / 待人工** |
| 边缘缩放 | Tauri undecorated resize | 保留；未自动化拖拽 |
| 多显示器 | — | NOT_TESTED |
| 125%/175%/200% DPI | — | NOT_TESTED |

> 覆盖层点击测试需要先模拟 `WM_NCMOUSEMOVE` 悬停再发送点击；这是自动化消息测试的约束，真实鼠标操作会自然产生悬停。覆盖层点击后由 Rust 直接切换最大化，不依赖前端事件时序。

### 5.3 布局与溢出

**自然 150% DPI（AppliedDPI=144）**

| 窗口（Win32） | CSS 视口 | 侧栏 | 9 页面页面级横向溢出 |
|---|---|---|---|
| 1366 × 768 | 1352 × 760 | 240 | 无 |
| 1600 × 900 | 1586 × 892 | 240 | 无 |
| 1920 × 1080 | 1906 × 1072 | 240 | 无 |
| 500 × 480 | 486 × 472 | 52 | 无 |

**100% 缩放仿真（CDP 设备指标，dsf=1）**

| 视口 | 9 页面页面级横向溢出 |
|---|---|
| 1366 × 768 | 无 |
| 1600 × 900 | 无 |
| 1920 × 1080 | 无 |

覆盖页面：工作台、项目、项目详情、任务、任务详情、验收中心、诊断、设置、404。

窄窗口补充：

- 任务详情 StateTrack 使用内部 `overflow-x-auto`；
- 主滚动区 `overflow-x-hidden` + `scrollbar-gutter: stable`；
- 所有栅格列补 `min-w-0`，避免 grid 子项被最小内容宽度撑破。

### 5.4 工作台首屏

1366×768（CSS 1352×760）测量：

- “下一步” top ≈ 133px；
- “最近验证” top ≈ 243px；
- “需要处理” top ≈ 431px；
- 三者均在首屏 760px 内。

### 5.5 主题

| 主题 | `documentElement.dark` | 页面背景 | 工作带 | 侧栏 |
|---|---|---|---|---|
| 深色 | true | `rgb(10,16,18)` | `rgb(17,26,28)` | `rgb(13,22,24)` |
| 浅色 | false | `rgb(238,243,241)` | `rgb(248,250,249)` | `rgb(230,237,234)` |

## 6. 未完成与未验证

1. **人工视觉验收**：`VISUAL_REVIEW_PENDING`；需用户手动截图。
2. **Snap 悬停菜单视觉**：`HTMAXBUTTON` 与覆盖层位置已验证；系统贴靠菜单的视觉与选择需人工确认。
3. **真实鼠标双击**：CDP 双击通过；实机鼠标双击的时序与焦点需人工确认。
4. **多显示器与跨屏 DPI**：未测试（单显示器 150% 环境）。
5. **DPI 组合**：150% 实测、100% 仿真；125%/175%/200% 未验证。
6. **边缘缩放**：Tauri undecorated resize 保留；未自动化拖拽边缘。
7. **焦点/失焦视觉**：窗口阴影 + 内容 focus ring；待人工确认。
8. **macOS/Linux**：未验证；Snap 覆盖层为空实现。
9. **生产 CSP/安装包**：未用 `tauri build` 成品运行验证。
10. **前端 bundle**：795.07 kB（gzip 238.09 kB），仍为单 chunk 并触发 >500 kB 提示。
11. **Git**：本轮未提交；基线为 `f3ff6a8`。
12. **本地测试数据**：`Phase 3.5 UI 审阅` 等项目为测试数据，可归档；未删除用户数据。

## 7. 人工视觉验收清单

详见 `docs/design/UI_AUDIT.md` 第 7 节。重点：

- 顶部工作带是否已是单一表面，侧栏是否从顶端自然延伸；
- 悬停最大化按钮是否出现 Snap Layouts；选择半屏/三栏是否成功；
- 窗口圆角/阴影在深浅主题下是否正常；
- 1366/1600/1920 与 500×480；
- 深浅主题；
- 工作台首屏是否仍有后台味；
- 键盘焦点与窗口控制可访问性。

不自动截图；由用户提供浅色/深色截图。

## 8. Phase 3.6 技术交付门禁结论

| 验收目标 | 结果 | 证据 |
|---|---|---|
| 顶部是否为单一工作带 | PASS（技术测量） | 44px 工作带；侧栏 y=44；240px 竖线贯通；无原生标题栏 |
| 侧栏与顶部自然衔接 | PASS（技术测量） | 品牌区与侧栏同色同宽；工作带左侧无横线 |
| 自定义标题栏 | PASS | `decorations:false`；自绘控制；窗口命令实测 |
| 原生窗口行为 | PASS（已测项） | 拖动、双击、最小化、最大化、还原、关闭 |
| Snap Layouts | PASS（命中）/ 待人工（菜单） | `XushuSnapOverlay` 返回 `HTMAXBUTTON=9` |
| 工作台首屏 | PASS | 状态条 → 焦点 → 最近验证；1366 首屏可见 |
| 页面后台感 | PASS（结构） | 一级页面移除大标题；操作进入工具栏 |
| 响应式窄窗口 | PASS | 500×480：52px 图标轨，无页面级横向溢出 |
| Phase 1–3 业务回归 | PASS | 前端 40 + Rust 69 测试 |
| 深浅主题 | PASS（技术） | Token 与表面颜色正确 |
| 窗口尺寸 | PASS（技术） | 1366/1600/1920/500；100%/150% |
| 人工视觉验收 | PENDING | `VISUAL_REVIEW_PENDING` |
| 不进入 Phase 4 | PASS | 未开发 Hooks 或后续功能 |
| Git | 未提交 | 等待用户审查 |

**技术交付门禁：PASS。**
**视觉交付门禁：VISUAL_REVIEW_PENDING。**
**本轮未创建 Git 提交。**

## 9. 标准汇报摘要

```text
当前阶段 / 任务：Phase 3.6 — 窗口一体化与顶部融合重构
实施摘要：切换为 Tauri 自定义标题栏，建立 44px 单一顶部工作带；实现原生 Snap Layouts HTMAXBUTTON 覆盖层；恢复拖动、双击、最小化/最大化/还原/关闭；工作台首屏改为状态条→焦点任务→最近验证；一级页面移除大标题，主操作进入上下文工具栏；窄窗口收为 52px 图标轨并禁止页面级横向滚动。
修改文件：新增 workspace-chrome.tsx、window-controls.tsx、window_chrome.rs、ADR-0006、UI_REDESIGN_PHASE_3_6.md；更新 app-layout、工作台/项目/任务/验收/设置/诊断页面、index.css、tauri.conf.json、capabilities、设计系统与 UI_AUDIT；未提交 Git。
实际执行的命令与退出码：pnpm typecheck/lint/test/build/format 均 0；cargo fmt/check/clippy/test 均 0（69 项）。
测试：PASS — 前端 40、Rust 69、窗口控制/拖拽/双击/关闭、Snap 命中、三尺寸 + 窄窗口 + 100%/150% 无页面级横向溢出；NOT_RUN — Snap 菜单视觉、实机鼠标双击、多显示器、125%/175%/200%、边缘缩放拖拽、macOS/Linux、生产安装包。
安全/隐私检查：未读取/输出密钥；未修改全局配置；未使用子 Agent；未截图；未启用 withGlobalTauri；未新增第三方插件；仅新增最小窗口控制权限；Phase 1–3 门禁未削弱。
明确未完成或尚未验证：VISUAL_REVIEW_PENDING；Snap 菜单视觉、实机双击、多显示器、其它 DPI、边缘缩放拖拽、生产 CSP/安装包。
下一步：等待用户手动截图与窗口行为审查；根据截图修正视觉问题；不改业务逻辑、不进入 Phase 4、不自动提交。
```