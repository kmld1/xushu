# XUSHU Phase 4 最终真实 Hooks 回归

2026-11-12，北京时间。基线为 `1bca890e65d9f23880dc451822aaa763b1106447`。Phase 4.3 修改后的 **真实 Claude Code Hooks 回归 PASS**。人工视觉仍为 **VISUAL_REVIEW_PENDING**，Phase 4 整体验收仍为 **PENDING**。

## 环境和证据

- 起始分支 `main`，HEAD 与指定基线相同，工作区干净。已核对 Phase 4.1 的真实事件记录、Phase 4.2 的桌面与数据库隔离记录，以及 Phase 4.3 的安全可靠性回归。
- 使用全新 D 盘测试 Git 仓库 `target/phase4-real-75_kp0ie/claude-fixture`。桌面使用独立标识、WebView 数据目录和 SQLite。接收器为本次 `cargo build --locked` 生成的 `xushu-hook.exe 0.1.0`。测试配置的备份证明安装目标指向该二进制，SHA-256 为 `79e0ed6ccfa8c5ee133d91bcd3753ee05335a8c43aeb1756941cf9693f8d6cf5`。
- 本机 Claude Code 版本为 **2.1.322**。隔离配置目录的 `auth status` 退出 1，返回未登录。Claude Code 以现有设置文件作只读启动输入，在隔离目录中通过真实模型预检，退出 0。正式用例使用默认权限，工具范围限于读取一个测试文本和执行固定的 `exit 7`。测试关闭会话持久化，没有触发登录或账户授权。 [Claude Code CLI 参考](https://code.claude.com/docs/en/cli-reference)
- 本轮生成的证据留在已忽略的 `target/phase4-real-75_kp0ie/`。 `scenario-evidence.json` 保存进程退出码、真实投递标识、事件类型、锁定阶段及 WebView DOM 断言。 `sqlite-and-isolation.json` 保存只读 SQLite 查询和隔离核对。 `crosscheck.json` 保存投递与入库的逐条匹配。未持久化原始 Claude Code 输出、原始会话 ID 或工具调用 ID。

## 实测结果

| 检查项 | 结论 | 本地证据 |
| --- | --- | --- |
| XUSHU 项目级授权 | **PASS** | 真实 Tauri IPC 对 `approved=false` 返回 `APPROVAL_REQUIRED`。实际 WebView 安装对话框中，授权框未勾选时提交禁用。勾选后安装成功。六个落盘处理器的命令、参数、matcher 和 timeout 与预览逐项一致。 |
| 六种 Hooks 的触发、接收与入库 | **PASS** | 实际模型会话于 19:08:53 启动，19:09:03 退出 0。Read 读取指定测试文件成功，Bash 执行 `exit 7` 得到真实失败结果。Spool 有 7 条新投递，逐一对应 SQLite 中的同一投递 ID。 |
| 界面显示 | **PASS（DOM）** | 实际项目页的活动面板包含会话开始、工具调用开始、工具调用成功、工具调用失败、一轮回答结束、会话结束，以及 Read 和 Bash。数据与同一次真实会话的 IPC 结果一致。 |
| 任务完成门禁 | **PASS** | 含必需人工验收条件的测试任务，真实事件前后均为 `awaiting_verification`、revision 2、`canComplete=false`。真实 IPC 尝试完成返回 `TASK_NOT_COMPLETABLE`。SQLite 无验证运行、无人工确认。 |
| 正常停用、卸载及重复卸载 | **PASS** | 分别验证正常停用和正常卸载。卸载及重复卸载后受管条目为 0，记录有卸载时间。停用后和卸载后各运行一次真实模型，均退出 0，XUSHU Spool 新增均为 0。 |
| Spool 文件被占用 | **PASS** | 第二个真实模型会话产生 3 条投递。用 Windows 独占文件句柄锁定其中 1 条。刷新后剩余 1 条，停用在句柄仍持有时成功，受管条目变为 0。 |
| 旧代次迟到消费 | **PASS** | 释放上述真实信封的独占锁后保留文件，重新启用获得不同采集代次，再刷新。旧投递被忽略 1 条、导入 0 条、剩余 0 条。SQLite 按原投递 ID 查无该事件。覆盖的是被占用而延后消费的真实信封。 |
| SQLite 写锁与状态恢复 | **PASS** | 锁定独立 SQLite 的第二个连接执行 `BEGIN IMMEDIATE`，此时收件箱中有真实会话的 3 条投递。卸载约 5.6 秒后返回 `CLAUDE_INTEGRATION_ERROR`，提示本地状态保存失败。项目配置已经移除所有 XUSHU Hooks。解锁后可读到 `needs_repair`、数据库仍记录启用、受管条目为 0。重试卸载成功，3 条旧投递被忽略且未入库。 |
| 第三方配置与用户设置 | **PASS（已核对范围）** | 安装差异仅有 XUSHU 的六个处理器。停用、卸载和部分成功后的项目 JSON 与安装前结构一致，原有两个第三方 Hook 组、权限和测试环境字段均保留。正式运行前后，监测的 Claude Code 全局设置、历史和状态文件的存在性、大小及修改时间一致。 |
| 正常数据库隔离 | **PASS** | 正式回归前后正常库 DB/WAL/SHM 指纹逐一相同。只在隔离库验证事件与任务，`integrity_check=ok`，外键违规 0。 |
| 接收器失败时 Claude Code 继续执行 | **PASS（缺少收件箱 Salt）** | 暂时移开本轮隔离收件箱的 Salt。Claude Code 报告两个 SessionStart 处理器均退出 0，模型正常回复并退出 0，Spool 新增 0 条。Salt 随后恢复到原位。 |
| 敏感字段过滤 | **PASS（真实样本）** | 对真实信封检查字段白名单和伪名格式。第一轮的 Spool、隔离 DB 与 WAL 均未出现该轮原始会话及工具调用 ID，也未出现测试提示词、文件内容、回复或设置值的标记。 |
| Windows Snap 菜单、人工视觉、生产安装包与 sidecar | **NOT_RUN** | 仅做 DOM 核对，人工视觉状态仍为 `VISUAL_REVIEW_PENDING`。Snap 实际菜单仍未确认，生产验证留待既定产品化阶段。 |
| 卸载后仍在运行的旧接收进程首次发布文件 | **NOT_RUN** | 本轮以真实已投递信封的占用和延后消费覆盖旧代次隔离，未将这一故障注入计作旧进程首次发布实测。 |

六种事件在主会话中的分布为 SessionStart 1、PreToolUse 2、PostToolUse 1、PostToolUseFailure 1、Stop 1、SessionEnd 1。SQLite 中 Read 为 `succeeded`，Bash 为 `failed`，失败分类为 `exit_code_nonzero`。全部场景完成后 SQLite 共保留 9 条事件，其中 7 条来自主会话，另 2 条是在 Spool 占用测试中、停用之前已消费的 Stop 和 SessionEnd。锁定的 1 条旧投递及写锁阶段的 3 条旧投递均按原 ID 核对未入库。最终待处理为 0，接入记录为已卸载。 [Claude Code Hooks 协议](https://code.claude.com/docs/en/hooks)

## 退出码和阶段结论

**FAIL 为 0 项。** 写锁下的 IPC 错误和固定命令 `exit 7` 均为用例预期结果。

| 命令或实际操作 | 退出码 |
| --- | --- |
| `claude --version`、真实模型预检 | 0 / 0 |
| `node --check target/phase4-real-75_kp0ie/scenario.mjs`、`python target/phase4-real-75_kp0ie/start.py` | 0 / 0 |
| 隔离构建 `cargo build --locked`、桌面正常关闭 | 0 / 0 |
| 正式六次 Claude Code 调用 | 0 / 0 / 0 / 0 / 0 / 0 |
| 逐条投递核对、隔离 SQLite 完整性和外键检查 | 0 |

未发现需要修改产品代码的确定性缺陷。已记录的项目配置叶文件替换竞态仍保留。基线提交保持原样，未新增 Git 提交。Phase 4 的真实 Hooks 回归已完成，人工验收继续按原清单处理。 [Phase 4.3 安全可靠性收尾](PHASE_4_3_SECURITY_CLOSEOUT.md) · [UI 审计](../design/UI_AUDIT.md)
