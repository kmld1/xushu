# XUSHU Phase 5.1.1 Rust 测试稳定性报告

2026-11-13（北京时间） · `D:\MyProject\xushu` · 当时 **PENDING**，后续复测见第 5 节

本轮复现 Phase 5.1 的四项 Rust 失败。Git 恢复用例的夹具已修复。三个 Junction 用例仍在当前 Windows 执行环境的夹具创建步骤失败。Rust 全量运行 **150 PASS、3 FAIL、0 ignored**，退出 **101**。

## 1. 四项结果

| Rust 测试目标及完整名称 | 修复前实测 | 根因及改动 | 修复后实测 |
| --- | --- | --- | --- |
| `claude` · `directory_redirection_is_refused` | Cargo 101，`New-Item : Access is denied` | 创建 `.claude` Junction 时系统拒绝。统一使用夹具创建函数，记录子进程退出码与 stderr，成功时核对实际指向 | Cargo 101、PowerShell 1，创建阶段失败，安装拒绝断言未执行 |
| `claude` · `closeout_registered_root_rejects_junction_even_after_new_preview` | Cargo 101，`fixture junction could not be created` | 注册项目目录成功移走，创建指向同一隔离夹具内目标的 Junction 时系统拒绝。共享夹具函数补全错误证据 | Cargo 101、PowerShell 1，`Access is denied`，安装及重新预览断言未执行 |
| `claude` · `closeout_rejects_redirected_parent_and_backup_directory` | Cargo 101，`fixture junction could not be created` | 第一个父目录 Junction 创建被拒绝。共享夹具函数补全错误证据 | Cargo 101、PowerShell 1，`Access is denied`，父目录拒绝断言与后续备份目录场景均未执行 |
| `recovery` · `handles_git_failure_as_unknown_and_reports_missing_information` | Cargo 101，`remove repo: Os { code: 5, kind: PermissionDenied }` | Git for Windows 在临时仓库创建只读松散对象，递归删除先于产品断言失败。仅清除此测试仓库对象的 Windows 只读属性，显式检查夹具关闭结果 | Cargo 0，单独运行通过，最终全量的恢复目标 14/14 |

每项复现均使用 `cargo test -p xushu-core --locked --test <目标> <完整测试名> -- --exact --nocapture --test-threads=1`。Cargo 进程使用 D 盘 `target/phase5-1-1-rust-stability/tmp` 作为 TEMP/TMP。Git 搜索上限设在同一临时根目录，防止负例向上识别到 XUSHU 工作树。这些环境设置只作用于本轮命令进程。

## 2. 根因和修复范围

### Junction 创建

D 盘卷信息返回 **NTFS**，且支持 Reparse Points。用同一新建临时目录、同一绝对目标路径独立测试时，普通子目录创建成功，Windows PowerShell 5、PowerShell 7 和 `cmd.exe /d /c mklink /J` 均以 **1** 退出并返回拒绝访问，链接路径未生成。失败发生在产品 Hook 路径校验之前。证据定位到当前执行环境对 Junction 创建的权限限制，尚未确定拒绝来自具体哪一层权限策略。 [Microsoft mklink 文档](https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/mklink)

`crates/xushu-core/tests/claude.rs` 复用已有 `junction` 夹具函数并记录退出状态与 stderr，创建成功时校验实际指向。三个用例仍执行原断言流程，创建失败继续使 Cargo 返回失败。真实 Junction 下的路径拒绝断言仍待在支持该操作的 Windows 环境执行。

### 临时 Git 仓库清理

独立探针中，Git init、config、add、commit、status 均同步退出 0。对象目录内 3 个松散对象均有只读属性。等待 0.5 秒后，递归删除仍在第一个只读对象的 `unlink` 返回 WinError 5。普通目录递归删除、仓库同级重命名及普通文件删除均成功。单独创建的只读普通文件也复现 WinError 5，清除属性后可删除。单线程用例和独立探针均复现，修复后同一用例的删除和业务断言通过。上述结果将本次失败定位到只读属性。 [Microsoft DeleteFileW 文档](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-deletefilew)

`crates/xushu-core/tests/recovery.rs` 保留已提交的 Git 夹具及原有业务断言。Windows 下只处理该一次性仓库 `.git/objects` 下的普通文件，随后执行原有递归删除并显式关闭临时目录。原用例验证项目路径消失后恢复包的 Git 状态为 `UNKNOWN`、含 Git 缺失信息及说明、协作文件列表为空。Clippy 初次运行将 `set_readonly(false)` 判为 Unix 权限风险，本次用带原因的局部 lint 预期限定在 Windows 夹具循环，复跑退出 0。

本轮改动限于两份 Rust 集成测试及两份阶段报告。Junction 的现有产品断言保留，Git 回归由同一原始用例的失败和通过结果证明。

## 3. 最终验证

| 命令 | 退出码 | 实测结果 |
| --- | ---: | --- |
| `cargo test --workspace --locked` | 101 | 66 通过、3 失败，在 `claude` 测试目标结束后停止 |
| `cargo test --workspace --locked --no-fail-fast` | 101 | **150 通过、3 失败、0 ignored**。恢复目标 14/14，三个失败均为 Junction 创建 |
| `cargo check --workspace --all-targets --locked` | 0 | 通过 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | 通过 |
| `cargo fmt --all -- --check` | 0 | 通过 |
| `pnpm test` | 0 | 9 个文件、72 项通过 |
| `python -u scripts/desktop-e2e.py --scenario target/phase5-1-1-rust-stability/cdp_git_unknown.mjs` | 0 | 真实 Tauri IPC 将旧快照标为 `stale`，Git 读取失败后新快照为 `UNKNOWN`，含缺失信息，保留最近 20 份。使用隔离 SQLite |
| `git diff --check` | 0 | 通过 |

正常数据库三件套以文件字节读取计算 SHA-256。以下大小和指纹在本轮前后完全一致。

| 文件 | 字节数 | SHA-256 |
| --- | ---: | --- |
| DB | 4096 | `89A2318C76452BE80E1D743F163F3DD96623B2B05604AA06985C6B5501238362` |
| WAL | 2233072 | `44A6AFB7DCD5A73D0FEA762D6B8EE3F738DD4596AE50F1E52F950CC302E445C3` |
| SHM | 32768 | `CF51378723AFA5B41F5FB1D76BFFCA05D02499887E463C42D6398FC7A79BF458` |

## 4. 证据与剩余门禁

证据目录为 `target/phase5-1-1-rust-stability/`。四项的修复前后日志为 `before-<完整测试名>.log` 与 `after-<完整测试名>.log`，另有 `before-exits.json`、`after-exits.json`、`junction-absolute.json`、`volume.json`、`git-fixture-probe.json`、`cargo-final-exits.json`、`final-test-all.log`、`clippy-final.exit`、`pnpm-test.exit`、`tauri-recovery-e2e.exit` 与数据库前后指纹。

`main` 的 HEAD 为 `1bca890e65d9f23880dc451822aaa763b1106447`，暂存区为空。保留既有工作树和历史报告，未提交或推送。Phase 5.1 独立验收报告已追加本轮结论。

**当时 Rust 全套门禁为 PENDING。** 三个 Junction 用例在夹具阶段停止，重定向拒绝断言在该环境中为 `NOT_RUN`。人工视觉为 `VISUAL_REVIEW_PENDING`。

## 5. 后续普通 Windows 终端复测

用户在普通 Windows PowerShell 7.6.9 执行 `cargo test --workspace --locked`，报告 **153 PASS、0 FAIL、0 IGNORED**，进程退出 **0**，含三项 Junction 产品断言。Rust 全量门禁据此更新为 **PASS**。本报告第 3 节的 Codex 复跑记录为当时实测。 [Phase 5.1.2 Windows Junction 诊断和复测](PHASE_5_1_2_WINDOWS_JUNCTION_DIAGNOSTIC.md#7-普通-windows-终端复测归档)
