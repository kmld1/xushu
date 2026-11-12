# Phase 4.2 最终 UI、交互与数据隔离验收

> 后续隔离加固、正常库清理预览、Git 审计与剩余门禁见 [Phase 4 收尾审计](PHASE_4_CLOSEOUT_AUDIT.md)。

日期为 2026-11-09，时区为 Asia/Shanghai。运行环境为 Windows 11 企业版 build 26200、Tauri 2.12.5、WebView2、Node 24.20.0、pnpm 12.16.1、Cargo 1.99.1。程序化验收与人工视觉复核分别记录。

| 验收项 | 状态 | 范围 |
| --- | --- | --- |
| 自动化功能验收 | **PASS** | 前端和 Rust 回归、真实桌面预览与活动交互、独立数据目录验证 |
| 真实数据 UI 验收 | **PASS** | Phase 4.1 真实 Claude Code 2.1.319 事件的 SQLite 证据副本，经真实 Tauri IPC 和 WebView 渲染 |
| E2E 数据隔离 | **PASS（当前入口）** | 历史入口 **FAIL**，正常库中仍有 6 个旧 E2E 项目和 9 个关联任务 |
| 键盘与无障碍 | **PASS（已测项）** | Dialog、Sheet、停用和卸载确认、浏览器无障碍树中的复选框名称。辅助技术人工实测单列 |
| 深浅主题程序化检查 | **PASS（已测矩阵）** | 原生 150% 和 WebView 100% 仿真。Windows 原生 100% **NOT_RUN** |
| 人工视觉复核 | **VISUAL_REVIEW_PENDING** | 等待用户截图和原生 Snap 菜单确认 |

## 安装预览与键盘

- 用空白隔离 Git 仓库生成真实预览，显示目标设置文件、六种 Hook、Matcher、5 秒超时、数据边界和授权说明。未勾选时确认按钮具有原生 `disabled`，从 Tab 顺序跳过。Space 勾选后启用，再按 Space 禁用，重开时授权清空。配置文件无效时，即使勾选也保持禁用。
- 用 WebView 键盘输入验证 Enter 打开预览、展开接收命令和取消，Esc 关闭，Tab 和 Shift+Tab 在弹层内循环，PageDown 滚动预览正文。500×480 下授权说明和复选框可达，滚动正文后关闭按钮和操作区仍可见。浏览器无障碍树报告复选框角色及关联的授权名称。
- 展开命令在矩阵中没有横向溢出。逐项比对真实预览返回的数据，接收命令由 `xushu-hook.exe`、管理标记、项目 ID、项目根路径和收件箱路径组成。检查范围内未显示预览 token、凭据、环境值、原始会话正文或工具输入输出。路径属于本机信息，报告只记录参数名。
- 检查活动 Sheet 的两个入口、Esc、Enter 关闭及焦点返回。另从「更多 Claude Code 接入操作」打开停用和卸载确认框，在深浅主题按 Esc 取消，焦点返回菜单按钮。桌面测试只查看预览和确认，授权后 Enter 提交在前端测试替身中验证。

## 活动 Sheet 数据来源与结果

### Phase 4.1 真实数据

复用 `D:\MyProject\xushu-hook-e2e-test-a6b35d18\evidence\xushu.phase41.sqlite`。从证据备份复制 SQLite 及 WAL 到一次性 app 数据目录。备份完整性检查为 `ok`，外键违规为 0。原项目设置文件与 Phase 4.1 卸载时的证据文件哈希一致。生成过程见 [Phase 4.1 真实 Hooks 联调报告](PHASE_4_1_E2E_REPORT.md)。

| 项目 | 桌面验收结果 |
| --- | --- |
| 会话与关联 | 4 个会话均显示已结束，每个会话的 `session_start` 和 `session_end` 关联同一会话 ID |
| 工具状态 | 2 个 `Read` 成功、2 个 `Bash` 失败，均有开始和结束观察。失败行显示「工具调用失败」「失败」和「命令退出码非 0」 |
| Stop 与任务状态 | 2 个 Stop 显示「一轮回答结束」。原任务保持 `draft`、revision 1，原证据库的验证运行数为 0 |
| 时间 | 18 行按接收时间由新到旧，`time[datetime]` 与 IPC 时间一致。本机时区 `Asia/Shanghai`，首行显示 `01:01:04`，完整时间提示为 `2026年11月9日 01:01` |
| 刷新 | 两次按 Enter 刷新均显示无待导入事件，刷新前后 18 条事件 ID 及顺序一致 |

本轮使用的是已结束的真实会话。实时「执行中」的真实来源见 Phase 4.1，当前 UI 的该分支由前端回归和下述隔离样本覆盖。重新触发 Claude Code、模型调用和真实 Hooks 安装均为 **NOT_RUN**。

### 隔离测试数据

- 空 Sheet 显示「尚未收到 Claude Code 事件」，没有时间线。一次性项目的无效设置文件触发「配置异常」和受阻预览。活动读取错误及重试恢复通过前端测试验证。
- 在同一隔离库的另一个项目构造 5000 条事件、20 个会话、100 个工具调用和 64 字符长工具名。UI 按现有限额渲染最近 100 条。500×480 逻辑外窗的 Sheet 正文可以用 PageDown 和 End 滚到最后，刷新和关闭操作始终可见。修复前正文横向溢出 388 CSS px，修复后为 0。最终实机抽样从触发打开到检测到 100 行耗时 102 ms，含测试轮询，受缓存和桌面负载影响。

## 深浅主题与布局

预览和真实活动 Sheet 各运行 40 个组合。每种尺寸包含浅色、深色、跟随本机系统、跟随仿真浅色和跟随仿真深色，并分别检查原生 150% 与 WebView 100% 仿真。系统设置保持原值。

| 逻辑外窗尺寸 | 原生 DPI 144 / 150% WebView 视口 | 100% 仿真视口 |
| --- | --- | --- |
| 1366×768 | 1352×760 | 1366×768 |
| 1600×900 | 1586×892 | 1600×900 |
| 1920×1080 | 1906×1072 | 1920×1080 |
| 500×480 | 486×472 | 500×480 |

- 44 CSS px 顶部工作带保持一体。项目详情根节点、主区域和接入区块的页面级横向溢出均为 0。Dialog 和 Sheet 使用现有表面及边框令牌，浅色分别为 `#FFFFFF`、`#D2DEDA`，深色为 `#162225`、`#223235`。
- 对比度按实际 CSS 令牌计算。抽测正文、次要文字、命令块、主按钮和状态色配对，浅色最小为 5.04:1，深色最小为 6.02:1，均达到设计系统的 4.5:1 目标。失败有明确文字和图标。按钮禁用时计算透明度为 0.5 且不可点击，启用后存在悬停色和可见焦点环。
- 真实鼠标移到最大化按钮覆盖层，确认命中 `HTMAXBUTTON=9`。至少一次在隔离窗口处于前台且鼠标命中时，未可靠识别 Windows Snap 布局菜单。菜单实际呈现及布局选择保留人工复核。

## E2E 数据隔离

历史结果为 **FAIL**。常规 Tauri 标识为 `dev.xushu.desktop`，正常库为 `%APPDATA%\dev.xushu.desktop\xushu.db`。从普通库和 WAL 的稳定副本只读核对，完整性为 `ok`。旧 Phase 2 脚本指向普通库，旧 Phase 3 桌面覆盖配置沿用常规标识，未指定独立数据目录。发现以下留存记录。

| 历史测试根目录 | 项目数 | 关联任务数 |
| --- | --- | --- |
| `%TEMP%\xushu-phase2-e2e\` | 1 | 1 |
| `D:\MyProject\xushu\target\e2e\` | 5 | 8 |
| 合计 | 6 | 9 |

这些记录和目录保留原状。Phase 4.1 已使用独立标识，其证据目录、隔离 AppData 和 WebView 缓存作为历史留存，见 Phase 4.1 报告。

当前入口 `python scripts/desktop-e2e.py` 为 **PASS**。每次构建使用独立的 Tauri 标识和临时 `appDirectoriesOverride`，启动时要求 SQLite、Hook 收件箱和 WebView2 配置均出现在本轮目录，再执行任何业务写入。通过真实 IPC 导入测试项目及创建任务，关闭后查询独立 SQLite，确认两者正确关联。测试前后常规库和 WAL 的 SHA-256 一致。编译缓存位于 `target/desktop-e2e-build/`，和测试数据分开。正常结束以及故意在创建任务后失败时，运行目录均清理，失败保留真实非零退出码。

试跑遇到 WebView2 退出后仍占用缓存的情况。失败路径现在先请求关闭隔离窗口，清理在有界时间内重试共享冲突。另一次因短重试超时的自建目录已单独清理。首次路径 `target/run-ncvqwdjf/` 的 SQLite 和 WebView 文件已清理，仅剩空的隐藏 `.git` 目录及其父目录。自动审批拒绝了针对该路径的强制递归删除，理由为策略阻止，已保留这一空目录。历史项目、数据库和未知目录没有被纳入清理。

## 修复与测试

- `apps/desktop/src/features/claude/claude-install-dialog.tsx` 将预览限制为一个可聚焦的滚动正文，保持标题、关闭和操作区可达。`claude-activity-sheet.tsx` 补齐正文键盘滚动和长工具名换行。
- `claude-integration-section.tsx`、`apps/desktop/src/components/confirmation-dialog.tsx` 恢复触发按钮焦点，覆盖两个活动入口和两个确认框。`claude-integration-section.test.tsx` 覆盖焦点、空态、错误重试和键盘提交。
- `scripts/desktop-e2e.py` 与 `scripts/desktop-e2e.mjs` 提供隔离桌面启动和真实项目/任务写入回归。UI 修复应用了 `frontend-design` Skill，沿用现有设计系统的表面、字体、语义状态和焦点令牌。

| 实际命令 | 退出码 | 结果 |
| --- | --- | --- |
| `pnpm typecheck`、`pnpm lint`、`pnpm format` | 0 / 0 / 0 | 通过 |
| `pnpm test` | 0 | 8 个测试文件，54 项通过 |
| `pnpm build` | 0 | Vite 构建通过，保留既有大于 500 kB 的 chunk 提示 |
| `cargo fmt --all -- --check` | 0 | 通过 |
| `cargo check --workspace --all-targets --locked` | 0 | 通过 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | 通过 |
| `cargo test --workspace --locked` | 0 | 109 项通过 |
| `cargo build -p xushu-hook --locked` | 0 | 开发接收器构建通过，供预览解析 |
| `python scripts/desktop-e2e.py` | 0 | 真实桌面写入与隔离清理通过 |
| `python scripts/desktop-e2e.py --snapshot <Phase 4.1 备份> --scenario target/phase42-audit/audit.mjs` | 0 | 无截图桌面矩阵、真实数据和隔离场景通过 |
| `python scripts/desktop-e2e.py --scenario target/phase42-audit/failure.mjs` | 1（预期） | 故意在写入后失败，验证运行目录清理和正常库不变 |
| `git diff --check`、`git diff --cached --check` | 0 / 0 | 工作区及已暂存差异无空白错误 |

复现阶段的焦点回归、窄窗关闭按钮、长工具名及确认框缺陷均产生了退出码 1，修复后使用同一断言复跑。首轮桌面测试还修正了测试脚本的 Hash 路由、Enter 输入和滚动边界的亚像素容差。完整无截图场景和文本日志保留在已忽略的 `target/phase42-audit/`。

## 人工视觉与 Git 状态

- 待用户以手动截图确认深浅主题的接入区块、状态标记、预览和 Sheet 层次、长文本换行、禁用和悬停观感、滚动条与 500×480 可读性。Windows 原生 100% 缩放、跨显示器 DPI、原生 Snap 悬停菜单与布局选择，以及屏幕阅读器的朗读顺序待人工确认。此次未自动截图或录屏。
- `main`，HEAD `f3ff6a8`。初始 `git status --short` 80 项，最终 83 项。两条已暂存的重命名与现有未提交工作保留。本轮在既有 Phase 4 文件上做局部修复，并新增隔离脚本目录和本报告，未暂存、提交或推送。
