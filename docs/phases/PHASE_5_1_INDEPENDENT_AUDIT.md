# XUSHU Phase 5.1 独立验收报告

2026-11-13（北京时间） · 工作树 `D:\MyProject\xushu` · **PENDING**

本报告基于 Phase 5 实际工作树、独立 Windows Tauri/WebView2、隔离 SQLite、真实 Git 仓库和 Claude Code 2.1.322 新会话。Phase 5 实施报告与 Phase 4 真实 Hooks 回归用于理解范围和既有边界。

## 1. 验收结论

| 验收项 | 状态 | 实测范围 |
| --- | --- | --- |
| `CODE_VERIFIED` | **PASS** | 两处时效缺陷及焦点问题均有红绿回归。普通 Windows PowerShell 7.6.9 的 Rust 全量复测 153 通过，退出 0，涵盖三项 Junction 断言。Codex 环境诊断见第 12 节 |
| `MEMORY_VERIFIED` | **PASS** | 真实 IPC 覆盖八类、修订、确认、归档、恢复、关联、跨项目及事务回滚 |
| `DECISIONS_VERIFIED` | **PASS** | 真实 IPC 和 SQLite 确认取代关系、历史、废弃、事务回滚与任务门禁 |
| `RECOVERY_SNAPSHOT_VERIFIED` | **PASS** | 真实 IPC 验证项目、Git、证据、任务与 Git 过期、重新生成、历史复制、20 份保留和 Git 失败降级 |
| `REAL_CLAUDE_NEW_SESSION_VERIFIED` | **PASS** | 当前快照复跑五问事实 5/5。字面评分保持 4/5，差异为执行中状态同义措辞。独立历史会话明确识别历史状态 |
| `DATA_ISOLATION_VERIFIED` | **PASS** | 真实隔离库通过完整性和外键检查，正常 DB/WAL/SHM 前后大小及 SHA-256 一致 |
| `UI_INTERACTION_VERIFIED` | **PASS** | 真实 WebView2 的 36 组布局、键盘、滚动、表单反馈、入口、过期和历史复制回归 |
| `VISUAL_REVIEW_PENDING` | **PENDING** | 人工视觉签收未进行 |

## 2. 代码与架构

审查 `CLAUDE.md`、总开发计划、Phase 5 实施报告、ADR-0008、设计系统与 UI 审计，以及 Phase 4.1 和 Phase 4 真实 Hooks 回归记录。

迁移 6 定义 `project_memories`、`project_memory_revisions`、`decisions`、`decision_revisions`、`project_recovery_preferences`、`recovery_snapshots`。当前记录与历史分表，实体与修订号具备唯一约束，确认与归档时间和状态受 SQLite CHECK 约束。持久化及迁移测试 13/13。

Tauri 注册 7 个记忆、6 个决策及 5 个恢复命令。Rust 服务通过 `AppState` 访问 SQLite，业务参数使用绑定，项目 ID、归属、修订和输入大小由服务层校验。产品窗口能力中未添加通用 SQL、Shell 或文件写入。测试启动时对独立应用标识及数据路径做配对校验。

## 3. 长期记忆与开发决策

| 范围 | 真实 Tauri IPC 和 SQLite 证据 |
| --- | --- |
| 记忆类别、来源、修订 | 八类全部创建。推断来源初始未确认，确认、编辑、归档、恢复累计五版。任务和证据 ID 保留 |
| 隔离与输入拒绝 | 跨项目读取返回 `NOT_FOUND`。跨项目关联任务与证据、超长正文、推断直接确认为事实返回 `VALIDATION_ERROR` |
| 记忆事务 | 在写入第 2 版历史时由隔离库触发器中止，IPC 返回 `DATABASE_ERROR`。当前内容仍为原值，修订仍为 1 |
| 决策生命周期 | 草稿确认后有效。新决策确认时，旧决策转为 `superseded`，新旧双向关系成立，历史分别为 3 版和 2 版。另验证废弃状态 |
| 决策非法操作 | 已取代决策再次确认、跨项目读取、直接改写已确认决策均被拒绝 |
| 决策事务 | 旧决策更新之后，新决策历史插入被触发器中止，IPC 返回 `DATABASE_ERROR`。旧决策及其历史回滚，新决策仍是草稿 |
| 任务门禁 | 决策操作前后所关联任务状态与修订相同。真实失败的验收任务仍为 `awaiting_verification`，试图完成返回 `TASK_NOT_COMPLETABLE` |

隔离备份复查为 schema 6、`integrity_check=ok`、外键违规 0。记录见 `cdp-evidence.json`、`cdp-sqlite.json`。Rust 记忆与决策各 8/8。

## 4. 恢复包真实性与时效

夹具为 D 盘新建的合成仓库，Git 分支 `phase51`，HEAD `d41750e24158a7b229f11bbc1fba991cc4e888a4`。先用真正的 Git 命令生成 PASS 与 FAIL 两种验收结果，然后通过真实 IPC 生成快照。

| 检查 | 结果 |
| --- | --- |
| 初始状态、项目、Git | 首次为 `no_snapshot`。项目 ID、分支、HEAD 和初始 clean 与夹具一致 |
| 当前工作 | `ORBIT-42 / NEXT-EXPORT` 为 `running`，用户明确选择为下一步 |
| 通过与失败证据 | `GREEN-BASELINE` 已完成且证据有效，恢复包证据 ID 与真实检查一致。`CHECK-RED` 为 FAIL，其任务保持待验证 |
| 记忆与决策可信范围 | `BIRCH-EXPORT` 和 `CONSTRAINT-OFFLINE` 进入已确认区。未确认条目在独立区，已归档和其他项目条目排除。`DECISION-NDJSON` 是唯一当前决策，旧决策为历史 |
| Fresh / Stale / Regenerate | 新生成后 `fresh`，任务更新后 `stale`，重新生成后 `fresh`。随后外部写入合成 Git 文件，当前快照转 `stale`，移除后恢复 `fresh` |
| Historical / Copy | 旧快照指纹保持不变，跨项目读取返回 `NOT_FOUND`。WebView2 历史预览和复制含历史警告，外部 Git 变化后的复制需二次确认并含过期警告。预览内容与对应快照正文逐字一致 |
| 来源、冲突、未知 | 实测来源结构及 4 项缺失信息。Rust 覆盖证据过期和关联已取消任务冲突。另用一次性仓库模拟不可读 Git，实机 IPC 将旧快照标为 stale，并在新快照写入 UNKNOWN 与缺失信息 |
| 20 份历史保留 | 通过真实 IPC 连续生成 22 份，列表保留最新 20 份且首份已移除 |

应用在构建恢复包时使用 SQLite、只读 Git、验证元数据和已导入的 Hooks 元数据。建议文件通过存在性探测获得。恢复包不会把失败或未知验收结果视为通过，也不宣称项目已经完成。

## 5. 真实 Claude Code 跨会话测试

CLI 版本 **2.1.322**。首次预检和首次正式调用分别在 50 秒内超时。重连后的最小预检返回合成标记并退出 0。正式复测将 IPC 生成并经 SQLite 备份核对的快照 Markdown 传入新会话，关闭会话持久化、执行工具与 MCP，只允许在合成项目中使用 Read。真实会话退出 0，调用 Read 1 次，正确返回 README 标记，并核对 Git 分支、HEAD 与 clean。没有触发登录或付费授权。

当前快照 ID 为 `14ae9fe8-5373-44c8-91d5-4d657e9d72b9`，输入 Markdown SHA-256 为 `87023052b4cc35101cc542c8031851c69aef1770766f623797d13fbe0180283d`。首个完成会话摘要 `1a940f9462f274ae`。

| 问题 | 固定预期 | 首次严格评分 | 同快照复跑的事实结果 |
| --- | --- | --- | --- |
| 当前开发目标 | `BIRCH-EXPORT`，确定性离线导出 | 1 | 匹配，提供来源 |
| 上次停在哪里 | `ORBIT-42` 仍在执行，状态 `running` | 0 | 提到 `ORBIT-42`，用等价的进行中措辞，提供来源。字面断言仍为 0 |
| 当前有效决策 | `DECISION-NDJSON`，CSV 决策已被取代 | 1 | 匹配，提供来源 |
| 未通过验收 | `CHECK-RED` 失败 | 1 | 匹配，提供来源 |
| 下一步 | 用户明确指定 `NEXT-EXPORT` | 1 | 匹配，提供来源 |

首次与复跑的严格字面评分均为 **4/5**。同一份快照在会话 `d0d2d241106ecc40` 再次返回退出 0。复跑分别记录任务 ID 和状态表达，确认第 2 项为同义措辞，五项事实 **5/5**。夹具预期及严格脚本结果保持原样。首次仅保留汇总标记，无法追溯该次具体措辞。模型同时识别了 `CONSTRAINT-OFFLINE`、通过证据、Git 状态，以及未确认记忆和已被取代决策的边界。快照正文含 `ORBIT-42` 与对应执行中状态。

另用真实 WebView2 历史复制警告与旧快照正文启动独立会话，摘要 `304bb78aab0fe82f`，退出 0，Read 1 次。模型明确回答历史快照不可视为当前，识别了相同的 Git 分支、HEAD 与 README 标记。该次单独核对历史边界。三个完成会话前后合成仓库 Git 指纹一致。只保存合成标记、布尔值、摘要和结果状态。CLI 权限参数参考 [Claude Code CLI 文档](https://code.claude.com/docs/en/cli-reference)。

## 6. WebView2 UI、键盘与无障碍

通过 Edge/WebView2 157 的 CDP 实测 500×480、1366×768、1600×900、1920×1080。浅色、深色、跟随系统三个模式与记忆、决策、恢复三个页面形成 **36 组**，测量到的 CSS 视口均与目标一致，页面及主内容无横向溢出。未自动截图或录屏，人工视觉保持 `VISUAL_REVIEW_PENDING`。

实机复测覆盖恢复入口、过期确认、确认前不触发写入、过期提示及焦点回到复制按钮、历史预览与文本。测试在隔离 WebView 中捕获 Clipboard API 载荷，未读取或覆盖 Windows 用户剪贴板。

实机发现记忆详情切到编辑后，Esc 关闭表单未把焦点还给原行。前端回归也复现同类决策编辑问题及从历史列表进入预览后焦点回错入口。修复后通过真实 WebView2 的 Enter、Space、Tab、Shift+Tab、Esc 与焦点回归，覆盖记忆及决策详情、新建与编辑表单、恢复预览和历史入口。

500×480 下，长内容在详情内换行并可滚动，空表单提交显示字段错误，已确认决策没有编辑草稿操作。工作台、项目与任务相关入口可达。交互沿用 XUSHU 现有设计系统的按钮、Sheet 与 Dialog。 [前端设计指南](https://github.com/anthropics/skills/tree/main/skills/frontend-design)

## 7. 确认并修复的缺陷

| 缺陷 | 最小修复及回归 |
| --- | --- |
| 指定草稿更新后旧快照可能仍为 fresh | 将指定草稿修订纳入快照指纹，其他快照序列化保持兼容。用例先复现 `Fresh != Stale`，修复后通过，并验证重生成恢复 fresh。原有缺少该修订的指定草稿快照在核对时会要求重生成 |
| 恢复页和预览 Sheet 复制可能使用缓存时效 | 两个入口统一在复制前从 Rust 重新查询。已过期先确认，核对失败不写剪贴板。若另一份快照已生成，保持用户选中的旧正文并加历史警告。三条红绿用例及一条 IPC 错误回归通过 |
| 编辑 Dialog 和历史预览关闭后焦点丢失或回错入口 | 沿用现有 Sheet/Dialog 及设计系统，显式把焦点还给列表行、新建按钮或历史入口。三条前端断言先失败，修复后通过 |

## 8. 隐私和数据库隔离

测试数据库位于 Tauri E2E 创建的独立目录，备份也只取自隔离库。确认状态及项目归属由 Rust 校验。恢复包排除已归档记忆，未确认条目明确作为待核对信息。保存的证据限于合成项目、经过校验的元数据与运行结果。

正常数据库采用文件只读 SHA-256 比对，未通过 SQLite 打开正常数据库。测试前、Windows 重连时及最后的交互复测后三件套大小及 SHA-256 一致。指纹记录在 `normal-before.json`、`normal-resume.json` 与 `normal-after.json`。

| 文件 | 大小 | 测试前 SHA-256 |
| --- | ---: | --- |
| DB | 4096 | `89A2318C76452BE80E1D743F163F3DD96623B2B05604AA06985C6B5501238362` |
| WAL | 2233072 | `44A6AFB7DCD5A73D0FEA762D6B8EE3F738DD4596AE50F1E52F950CC302E445C3` |
| SHM | 32768 | `CF51378723AFA5B41F5FB1D76BFFCA05D02499887E463C42D6398FC7A79BF458` |

## 9. 命令与退出码

| 命令 | 退出码 | 结果 |
| --- | ---: | --- |
| `pnpm typecheck` | 0 | 类型检查通过 |
| `pnpm lint` | 0 | ESLint 通过 |
| `pnpm test` | 0 | 9 个文件，**72 项通过** |
| `pnpm build` | 0 | 类型检查和前端构建通过 |
| `pnpm format` | 0 | Prettier 通过 |
| `cargo fmt --all -- --check` | 0 | 通过 |
| `cargo check --workspace --all-targets --locked` | 0 | 通过 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | 通过 |
| `cargo test --workspace --locked` | 101 | 13 核心、19 业务、34 Hooks 测试通过，3 个 Hooks 目录联接用例失败后停止 |
| `cargo test --workspace --locked --no-fail-fast` | 101 | **149 通过、4 失败** |
| `git diff --check` | 0 | 无空白错误 |
| Windows Tauri IPC E2E | 0 | 真实 IPC、SQLite、隔离、20 份保留及 Git 失败降级通过 |
| Windows Tauri WebView2 | 0 | 最终 36 组布局、键盘、焦点与复制验证 |
| `claude --version` | 0 | 2.1.322 |
| 真实模型预检 / 当前快照 / 当前复跑 / 历史快照 | 0 / 0 / 0 / 0 | 首次预检与首次正式请求超时另记为 124 |

Rust 最后一轮补跑使用进程级隔离 TEMP/TMP 和 Git 搜索上限。三条 Phase 4 目录联接用例为 `directory_redirection_is_refused`、`closeout_registered_root_rejects_junction_even_after_new_preview`、`closeout_rejects_redirected_parent_and_backup_directory`。另一个为 `handles_git_failure_as_unknown_and_reports_missing_information`，在删除临时 Git 仓库时被拒绝，未进入恢复断言。相应产品行为另由真实 IPC 使用不可读 Git 夹具复核通过，Rust 原用例的失败仍保留。

重连前的 Corepack 版本下载失败及环回拒绝仍保留在旧日志。重连后固定 pnpm 12.16.1 和标准桌面脚本均已实跑。离线嵌入资源备选方式没有完成业务握手，不作为通过依据。

## 10. Git、证据与检查点

起始 `main` 的 HEAD 为 `1bca890e65d9f23880dc451822aaa763b1106447`，25 个已跟踪修改、33 个未跟踪文件，暂存区为空。末态 HEAD 与暂存区一致，新增本报告后为 25 个已跟踪修改、34 个未跟踪文件。其余未跟踪文件为原 Phase 5 的 IPC、前端、迁移、领域代码、测试和原有报告，完整清单见 `git-after.txt`。未提交或推送，未进入 Phase 6。

Windows 证据目录为 `target/phase5-1-audit-4d93a192/`。包括 `git-before.txt`、`git-after.txt`、正常数据库三次指纹、`cdp-evidence.json`、`cdp-sqlite.json`、`cdp-ui-final-evidence.json`、`cdp-git-unknown.json`、`draft-red.log`、`draft-green.log`、`copy-red.log`、`copy-green.log`、`focus-red.log`、`focus-green.log`、`rust-fixed/`、`postfix-pnpm-exits.json`、`claude-preflight.json`、`claude-recovery.json`、`claude-recovery-repeat.json` 和 `claude-historical.json`。

**Phase 5 具备建立 `PENDING` 本地审计检查点的条件。** Rust 全量门禁及三项 Junction 产品断言已在普通 Windows 终端通过。人工视觉、实际 Windows 用户剪贴板、屏幕阅读器、Snap 实际菜单以及生产安装包和 sidecar 验证仍待完成。复测依据与本次 Git 审查见第 13 节。

## 11. Phase 5.1.1 Rust 失败复核

本节补充最终复核结果，第 9 节保留 Phase 5.1 当时的实测记录。Git 恢复用例已定位到 Git for Windows 的只读对象属性。修复一次性仓库的清理步骤后，原用例及恢复目标 **14/14** 通过。真实 Tauri IPC 再次验证 Git 不可读时生成 `UNKNOWN`。三项 Junction 用例均在产品断言前因 Windows 拒绝创建链接而失败，原断言保留。

最终 Rust 全量 **150 PASS、3 FAIL、0 ignored**，退出 **101**。Cargo check、Clippy、格式检查、前端 72 项和隔离 Tauri 回归均通过。正常 DB/WAL/SHM 的测试前后指纹一致。人工视觉保持 `VISUAL_REVIEW_PENDING`。 [Phase 5.1.1 Rust 稳定性报告](PHASE_5_1_1_RUST_STABILITY_REPORT.md)

## 12. Phase 5.1.2 Windows Junction 环境诊断

已确认 Codex 执行链使用未提权的低完整性 AppContainer。新建的 D 盘 NTFS 隔离目录可创建普通子目录，Windows PowerShell 5、PowerShell 7 及 `mklink /J` 创建 Junction 均被拒绝。PowerShell 探针捕获 Win32 5，三个实际 Rust 测试各退出 101。Codex 中夹具结果为 `ENVIRONMENT_BLOCKED`，产品安全断言为 `NOT_RUN`。现有证据尚未确定其他系统策略是否参与拒绝。普通终端补测见第 13 节。

当时 Rust 全量复测 **150 PASS、3 FAIL、0 IGNORED**，退出 **101**。格式、Cargo check 和 Clippy 退出 0。正常 DB/WAL/SHM 的测试前后指纹相同。当时 Rust 门禁为 `PENDING`，人工视觉为 `VISUAL_REVIEW_PENDING`。 [Phase 5.1.2 Windows Junction 诊断](PHASE_5_1_2_WINDOWS_JUNCTION_DIAGNOSTIC.md)

## 13. 普通 Windows 终端复测与检查点准备

独立复测依据为用户在普通 Windows PowerShell 7.6.9 中执行 `cargo test --workspace --locked` 后提供的结果。Rust 全量 **153 PASS、0 FAIL、0 IGNORED**，实际进程退出码 **0**，门禁更新为 **PASS**。Rust 增量编译缓存的硬链接失败后回退复制，为非阻断警告。归档范围为用户提供的结果摘要。

`directory_redirection_is_refused`、`closeout_registered_root_rejects_junction_even_after_new_preview` 和 `closeout_rejects_redirected_parent_and_backup_directory` 在普通终端均为 `PRODUCT_PASS`。Codex AppContainer 的夹具 `ENVIRONMENT_BLOCKED` 与断言 `NOT_RUN` 作为历史记录保留。普通终端单独的 `mklink /J` 探针为 `NOT_RUN`。 [Phase 5.1.2 复测明细](PHASE_5_1_2_WINDOWS_JUNCTION_DIAGNOSTIC.md#7-普通-windows-终端复测归档)

复核时 `main` 的 HEAD 为 `1bca890e65d9f23880dc451822aaa763b1106447`，暂存区为空。审查的 62 个变更路径包括 26 个已跟踪修改和 36 个未跟踪文件。Phase 5 候选共 61 个路径。独立的 `docs/phases/PHASE_4_REAL_HOOKS_REGRESSION.md` 保留在工作区，排除在本次候选清单之外。

对候选路径和文本检查了凭据、用户数据及生成物。两处疑似令牌命中均为测试夹具。候选中未发现真实凭据、数据库、真实 Claude Code 会话记录或构建产物。诊断日志和测试数据继续保留在 Git 忽略路径。基于已验证范围，检查点应标为 `PENDING`。人工视觉仍为 `VISUAL_REVIEW_PENDING`，其余原有 `NOT_RUN` 项保持原状态。
