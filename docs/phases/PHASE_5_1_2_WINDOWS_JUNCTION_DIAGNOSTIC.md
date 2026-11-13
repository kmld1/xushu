# XUSHU Phase 5.1.2 Windows Junction 环境诊断

2026-11-13（北京时间） · `D:\MyProject\xushu` · Codex **ENVIRONMENT_BLOCKED** · 普通 Windows 终端 **PRODUCT_PASS**

三个 Rust 用例在 Codex 执行链中均因 Junction 创建被拒绝而退出 **101**。PowerShell 的原生错误码为 **5（ERROR_ACCESS_DENIED）**，该环境中的产品安全断言为 **NOT_RUN**，Rust 全量为 **150 PASS、3 FAIL、0 IGNORED**。后续用户在普通 Windows 终端复测 **153 PASS、0 FAIL、0 IGNORED**，退出 **0**。证据与范围见第 7 节。

## 1. 三个失败的实际位置

以下路径以 `D:\MyProject\xushu\target` 为起点。临时目录名来自本轮运行中的目录观察记录。三个用例均经 Rust 调用 Windows PowerShell 5.1 的 `-NoProfile -NonInteractive -Command` 执行 `New-Item -ItemType Junction`，子进程退出 **1**，stderr 为 `New-Item : Access is denied`。Rust 在共享 `junction` 夹具函数的创建断言停止。

| Rust 集成测试 `claude` 中的完整名称 | 实际创建位置 | 目标位置 | 断言执行状态 |
| --- | --- | --- | --- |
| `directory_redirection_is_refused` | `phase5-1-2-junction-diagnostic\tmp\.tmpGYPrJb\项目 & spaces\repo\.claude` | `phase5-1-2-junction-diagnostic\tmp\.tmpGYPrJb\redirected-settings` | 夹具 **ENVIRONMENT_BLOCKED**，产品预览及安装拒绝 **NOT_RUN** |
| `closeout_registered_root_rejects_junction_even_after_new_preview` | `.tmpTsI7nI\项目 & spaces\repo` | `.tmpTsI7nI\unrelated` | 夹具 **ENVIRONMENT_BLOCKED**，注册项目根目录替换后的拒绝断言 **NOT_RUN** |
| `closeout_rejects_redirected_parent_and_backup_directory` | `.tmpVnXPjY\项目 & spaces` | `.tmpVnXPjY\previous-parent` | 首个父目录 Junction **ENVIRONMENT_BLOCKED**，父目录拒绝和后续备份目录用例均 **NOT_RUN** |

Rust 夹具退出码 **101** 为测试进程报告的失败，Windows PowerShell 子进程退出 **1**。Win32 错误码 **5** 来自同一执行链独立 PowerShell 创建探针捕获的 `Win32Exception.NativeErrorCode`。

## 2. 令牌、目录与普通终端对照

- 运行账户为本机交互用户，精确账户名见本地 `account.json`。有效管理员权限为 false，未提权，`TokenElevationType=3`。从 `powershell.exe`、`pwsh.exe` 和 `cmd.exe` 分别启动的只读令牌探针均返回 `TokenIsAppContainer=true`、`integrityRid=4096`（Low）、`TokenHasRestrictions=true` 和 `inJob=true`。`IsTokenRestricted=false` 及限制 SID 数 0 是另一组 API 字段，与 AppContainer 状态同时记录。 [Microsoft AppContainer 文档](https://learn.microsoft.com/en-us/windows/win32/secauthz/implementing-an-appcontainer) · [IsTokenRestricted 文档](https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-istokenrestricted)
- D 盘为 NTFS，卷能力包含 Reparse Points。已读取 `D:\`、项目祖先目录、`target`、新探针父目录和目标的 DACL。所读规则可继承，未见显式 Deny，包含 `Authenticated Users: Modify, Synchronize`。新父目录和目标均为普通目录，属性包含 `Directory, NotContentIndexed`，没有 `ReadOnly` 或 `ReparsePoint`。在同一父目录创建普通子目录成功。普通 DACL、成功创建普通目录和 AppContainer 的有效访问范围各有独立证据。

| 执行方式 | 父进程环境 | 结果 |
| --- | --- | --- |
| Windows PowerShell 5.1.26100.9711 内联命令 | Codex 低完整性 AppContainer | 退出 1，Win32 5，未生成 Junction |
| PowerShell 7.6.9 `New-Item -ItemType Junction` | Codex 低完整性 AppContainer | 退出 1，Win32 5，未生成 Junction |
| `cmd.exe /d /c mklink /J` | Codex 低完整性 AppContainer | 退出 1，`Access is denied`，未生成 Junction |
| 三个项目 Rust 测试各单独执行 | Codex 执行链 | 均退出 101，错误出现在实际创建夹具时 |
| 系统独立启动的普通 PowerShell 与 `cmd.exe` 探针 | 普通终端 | 独立探针 **NOT_RUN**，Rust 全量补测见第 7 节 |

原生探针在 `target/phase5-1-2-junction-diagnostic/native-probe-6ee8d975` 下运行。源和目标均位于新目录，名称不含空格和中文。带中文、空格和 `&` 的项目测试夹具也复现。Windows PowerShell 的 `-File` 调用另有 `AuthorizationManager check failed`，该启动问题与实际测试使用 `-Command` 得到的 Win32 5 分别记录。 [Microsoft mklink 文档](https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/mklink)

## 3. 只读安全诊断

| 检查 | 当前可观测结果 |
| --- | --- |
| Defender 与 Controlled Folder Access | PowerShell 7 的 ConfigDefender 模块加载受限。Windows PowerShell 5 的 `Get-MpComputerStatus` 和 `Get-MpPreference` 退出 1，报告 CIM 资源不可访问。检查指定的 CFA 组策略注册表路径时未见该键，有效运行状态仍为 **NOT_RUN** |
| AppLocker 有效策略 | PowerShell 7 命令模块加载失败，有效规则 **NOT_RUN** |
| Defender CFA、AppLocker、Code Integrity 事件 | `Get-WinEvent` 报权限不足，`wevtutil` 的 Defender 事件查询也退出 5。事件关联 **NOT_RUN** |
| PowerShell 执行策略 | Windows PowerShell 5 的 `Get-ExecutionPolicy -List` 报注册表访问被拒绝。项目实际使用的内联命令已执行到 Junction 创建并返回 Win32 5 |

已确认当前受限执行链内的失败位置。现有证据没有定位到拒绝来自 AppContainer 的哪条访问规则、特定文件系统过滤器或系统安全产品。安全设置仅作只读查询。 [Microsoft Defender CFA 事件说明](https://learn.microsoft.com/defender-endpoint/evaluate-controlled-folder-access)

## 4. Rust 结果和门禁

| 命令 | 退出码 | 结果 |
| --- | ---: | --- |
| `cargo test -p xushu-core --locked --test claude <完整测试名> -- --exact --nocapture --test-threads=1` | 各 101 | 三个用例均在创建夹具时失败 |
| `cargo test --workspace --locked` | 101 | 66 通过、3 失败，在 `claude` 目标结束后停止 |
| `cargo test --workspace --locked --no-fail-fast` | 101 | **150 通过、3 失败、0 ignored** |
| `cargo fmt --all -- --check` | 0 | 通过 |
| `cargo check --workspace --all-targets --locked` | 0 | 通过 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | 通过 |
| `git diff --check` | 0 | 通过 |

- **PRODUCT_PASS** · 已实际执行并通过的其他产品断言，例如普通目录替换的 `closeout_preview_rejects_replaced_directory_at_same_path`。
- **PRODUCT_FAIL** · 三个 Junction 场景均未到达产品断言，无此类实测结果。
- **TEST_FIXTURE_FAIL** · 三个测试记录到夹具创建阶段失败。普通命令对新目录也复现同一拒绝，未定位到夹具构造代码缺陷。
- **ENVIRONMENT_BLOCKED** · 三个用例在当前 Codex 执行环境的创建步骤。
- **NOT_RUN** · Codex 执行链中三个真实 Junction 下的产品断言，以及未能读取的安全策略和事件。普通终端复测见第 7 节。

诊断阶段只更新了诊断与验收记录。三个 Rust 用例、断言和产品代码保持原状。当时 Rust 门禁为 **PENDING**，人工视觉为 `VISUAL_REVIEW_PENDING`。

## 5. 普通用户终端复测

从 Windows 开始菜单独立打开普通 PowerShell 7，在该终端粘贴下列完整脚本。脚本会新建一个 D 盘隔离目录，依次保存真实 Junction 探针和三个 Rust 用例的退出码。若本地诊断令牌脚本存在，也会记录该终端子进程的令牌字段。此脚本已在 Codex 内验证可运行，独立普通终端结果待录入。

```powershell
& {
    $ErrorActionPreference = 'Stop'
    $PSNativeCommandUseErrorActionPreference = $false
    $repo = 'D:\MyProject\xushu'
    $run = Join-Path $repo ('target\phase5-1-2-terminal-' + [guid]::NewGuid().ToString('N'))
    $temp = Join-Path $run 'tmp'
    $destination = Join-Path $run 'destination'
    New-Item -ItemType Directory -Path $run | Out-Null
    New-Item -ItemType Directory -Path $temp,$destination | Out-Null

    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [Security.Principal.WindowsPrincipal]::new($identity)
    $tokenProbe = Join-Path $repo 'target\phase5-1-2-junction-diagnostic\token-probe.py'
    [pscustomobject]@{
        account = $identity.Name
        effectiveAdministrator = $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
        powershell = $PSVersionTable.PSVersion.ToString()
    } | ConvertTo-Json | Set-Content "$run/account.json" -Encoding utf8
    if ((Get-Command python -ErrorAction SilentlyContinue) -and (Test-Path -LiteralPath $tokenProbe)) {
        & python $tokenProbe *> "$run/token.log"
    }

    $psLink = Join-Path $run 'ps-junction'
    $psMessage = ''
    $psNativeCode = $null
    try {
        $null = New-Item -ItemType Junction -Path $psLink -Target $destination
        $psExit = 0
    } catch {
        $psExit = 1
        $psMessage = $_.Exception.Message
        if ($_.Exception -is [ComponentModel.Win32Exception]) {
            $psNativeCode = $_.Exception.NativeErrorCode
        }
    }

    $cmdLink = Join-Path $run 'cmd-junction'
    $cmdMessage = & cmd.exe /d /c mklink /J $cmdLink $destination 2>&1
    $cmdExit = $LASTEXITCODE
    $probes = @(
        [pscustomobject]@{method='PowerShell'; path=$psLink; exit=$psExit; nativeErrorCode=$psNativeCode; message=$psMessage}
        [pscustomobject]@{method='mklink /J'; path=$cmdLink; exit=$cmdExit; message=($cmdMessage | Out-String).Trim()}
    )
    foreach ($probe in $probes) {
        $item = Get-Item -LiteralPath $probe.path -ErrorAction SilentlyContinue
        $probe | Add-Member -NotePropertyName junction -NotePropertyValue (
            $null -ne $item -and $item.LinkType -eq 'Junction' -and
            [bool]($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -and
            $item.Target -eq $destination
        )
    }
    $probes | ConvertTo-Json | Set-Content "$run/junction.json" -Encoding utf8

    $previous = @{TEMP=$env:TEMP; TMP=$env:TMP; GIT_CEILING_DIRECTORIES=$env:GIT_CEILING_DIRECTORIES}
    Push-Location -LiteralPath $repo
    try {
        $env:TEMP = $temp
        $env:TMP = $temp
        $env:GIT_CEILING_DIRECTORIES = $temp
        $tests = @(
            'directory_redirection_is_refused'
            'closeout_registered_root_rejects_junction_even_after_new_preview'
            'closeout_rejects_redirected_parent_and_backup_directory'
        )
        $results = foreach ($test in $tests) {
            & cargo test -p xushu-core --locked --test claude $test -- --exact --nocapture --test-threads=1 *> "$run/$test.log"
            [pscustomobject]@{test=$test; exit=$LASTEXITCODE}
        }
        $results | ConvertTo-Json | Set-Content "$run/rust-exits.json" -Encoding utf8
    } finally {
        $env:TEMP = $previous.TEMP
        $env:TMP = $previous.TMP
        $env:GIT_CEILING_DIRECTORIES = $previous.GIT_CEILING_DIRECTORIES
        Pop-Location
    }
    $probes | Format-Table method,exit,nativeErrorCode,junction,message
    $results | Format-Table
    "Logs: $run"
}
```

回传新目录里的 `junction.json`、`rust-exits.json` 和可选的 `token.log` 即可对照。若普通终端能创建真实 Junction 且三个 Rust 用例均退出 0，三个场景可记 **PRODUCT_PASS**。若真实 Junction 创建成功而 Rust 仍失败，依据日志里的首个失败位置区分 **TEST_FIXTURE_FAIL** 或 **PRODUCT_FAIL**。若普通终端也拒绝创建，三项产品断言仍为 **NOT_RUN**。

## 6. 数据和证据

正常数据库以只读文件哈希核对，DB、WAL、SHM 的大小及 SHA-256 在本轮前后全部相同。基线和末态见 `normal-before.json` 与 `normal-after.json`。HEAD 保持 `1bca890e65d9f23880dc451822aaa763b1106447`，暂存区为空。

证据位于 `target/phase5-1-2-junction-diagnostic/`，包括 `account.json`、`token.json`、`launcher-tokens.json`、`acl.json`、`native-junction.json`、`powershell-inline.json`、`security-policy.json`、`security-policy-powershell5.json`、`rust-fixture-paths.json`、`targeted-exits.json`、`cargo-exits.json` 和 `test-all.log`。
## 7. 普通 Windows 终端复测归档

依据为用户在普通 Windows PowerShell 7.6.9 执行全量命令后提供的结果摘要。用户报告执行 `cargo test --workspace --locked`，**153 PASS、0 FAIL、0 IGNORED**，进程退出码 **0**。Rust 增量编译缓存的硬链接失败后回退复制，为非阻断警告。Rust 全量门禁为 **PASS**。

| `claude` 集成测试 | 普通 Windows PowerShell 7.6.9 | Codex AppContainer 历史结果 |
| --- | --- | --- |
| `directory_redirection_is_refused` | **PRODUCT_PASS**，拒绝 `.claude` 目录重定向 | 夹具 **ENVIRONMENT_BLOCKED**，断言 **NOT_RUN** |
| `closeout_registered_root_rejects_junction_even_after_new_preview` | **PRODUCT_PASS**，拒绝注册项目根目录替换后的操作 | 夹具 **ENVIRONMENT_BLOCKED**，断言 **NOT_RUN** |
| `closeout_rejects_redirected_parent_and_backup_directory` | **PRODUCT_PASS**，覆盖父目录和备份目录重定向 | 夹具 **ENVIRONMENT_BLOCKED**，断言 **NOT_RUN** |

普通终端单独执行的 `mklink /J` 探针为 **NOT_RUN**，本次结论来自项目实际 Rust 夹具和断言。普通终端未提供单独的令牌或 DB/WAL/SHM 指纹记录。第 6 节的指纹对照适用于 Codex 诊断。两种环境的结果已记录，触发 Codex 拒绝的具体系统规则仍未定位。人工视觉保持 `VISUAL_REVIEW_PENDING`。
