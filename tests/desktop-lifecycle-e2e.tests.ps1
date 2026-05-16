Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot
$tmpRoot = Join-Path $repoRoot 'tmp\desktop-lifecycle-e2e'
$runRoot = Join-Path $tmpRoot ("run-" + [System.Guid]::NewGuid().ToString('N'))
$sourceOrchestratorPath = Join-Path $repoRoot 'tools\common-he-init-orchestrator.ps1'
$syncScriptPath = Join-Path $repoRoot 'scripts\sync-desktop-resources.ps1'
$sourceCssPath = Join-Path $repoRoot 'apps\desktop\src\styles.css'
$originalCapabilityCatalogPath = $env:COMMONHE_REQUIRED_CAPABILITIES_PATH

function Assert-True {
    param(
        [bool]$Condition,
        [string]$Message
    )

    if (-not $Condition) {
        throw $Message
    }
}

function Assert-Equal {
    param(
        $Actual,
        $Expected,
        [string]$Message
    )

    if ($Actual -ne $Expected) {
        throw "$Message Actual=[$Actual], Expected=[$Expected]"
    }
}

function Get-BrowserPath {
    foreach ($commandName in @('chrome.exe', 'msedge.exe')) {
        $command = Get-Command $commandName -ErrorAction SilentlyContinue
        if ($null -ne $command) {
            return $command.Source
        }
    }

    foreach ($candidate in @(
        'C:\Program Files\Google\Chrome\Application\chrome.exe',
        'C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe'
    )) {
        if (Test-Path -LiteralPath $candidate -PathType Leaf) {
            return $candidate
        }
    }

    throw 'Chrome or Edge is required for desktop lifecycle layout gate.'
}

function ConvertTo-FileUri {
    param([string]$Path)

    return ([System.Uri]([System.IO.Path]::GetFullPath($Path))).AbsoluteUri
}

function Invoke-HeadlessLayoutGate {
    param(
        [string]$Name,
        [string]$Html
    )

    $htmlPath = Join-Path $runRoot "$Name.html"
    Set-Content -Path $htmlPath -Value $Html -Encoding UTF8

    Get-BrowserPath | Out-Null
    $nodeScriptPath = Join-Path $repoRoot 'tests\desktop-layout-cdp.mjs'
    $json = & node $nodeScriptPath $htmlPath
    if ($LASTEXITCODE -ne 0) {
        throw "Layout gate [$Name] failed to execute Chrome CDP harness."
    }
    $result = (($json | ForEach-Object { [string]$_ }) -join "`n") | ConvertFrom-Json
    return $result
}

function Assert-CssContract {
    param(
        [string]$Css,
        [string]$Pattern,
        [string]$Message
    )

    if ($Css -notmatch $Pattern) {
        throw $Message
    }
}

function Assert-DesktopLayoutContracts {
    param([string]$CssPath)

    $css = Get-Content -Raw $CssPath

    Assert-CssContract -Css $css -Pattern 'body\s*\{(?s).*?overflow:\s*auto;' -Message 'Body must allow page-level adaptive scrolling instead of hard clipping.'
    Assert-CssContract -Css $css -Pattern '\.shell\s*\{(?s).*?min-height:\s*100vh;' -Message 'Shell must use min-height instead of fixed height.'
    Assert-CssContract -Css $css -Pattern '\.workspace\s*\{(?s).*?grid-template-rows:\s*auto minmax\(560px,\s*1fr\) auto;' -Message 'Workspace must reserve a usable main stage height.'
    Assert-CssContract -Css $css -Pattern '\.conversation-panel\s*\{(?s).*?display:\s*flex;(?s).*?flex-direction:\s*column;' -Message 'Conversation panel must use flex so the composer does not collapse the message area.'
    Assert-CssContract -Css $css -Pattern '\.conversation-log\s*\{(?s).*?flex:\s*1 1 260px;' -Message 'Conversation log must keep a usable minimum height.'
    Assert-CssContract -Css $css -Pattern '\.composer textarea\s*\{(?s).*?min-height:\s*128px;' -Message 'Composer textarea must keep a usable minimum height.'
    Assert-CssContract -Css $css -Pattern '\.handoff p\s*\{(?s).*?overflow-wrap:\s*anywhere;' -Message 'Failure messages must wrap long PowerShell paths instead of overflowing.'
    Assert-CssContract -Css $css -Pattern '\.stage-action-bar\s*\{(?s).*?flex:\s*0 0 auto;' -Message 'Stage action bar must remain outside scrollable content.'
    Assert-CssContract -Css $css -Pattern '\.initialize-stage\s*\{(?s).*?justify-content:\s*flex-start;' -Message 'Initialize stage must not spread the header and chat apart with large dead space.'
    Assert-CssContract -Css $css -Pattern '\.workspace-path-chip\s*\{(?s).*?overflow-wrap:\s*anywhere;' -Message 'Selected workspace path must be shown in the top area and wrap safely.'
    Assert-CssContract -Css $css -Pattern '\.diagnostic-details\s*\{(?s).*?grid-column:\s*1 / -1;' -Message 'Diagnostic logs must be a secondary expandable detail instead of a duplicate primary panel.'
}

function New-ConversationLayoutHtml {
    param([string]$CssUri)

    return @"
<!doctype html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <link rel="stylesheet" href="$CssUri" />
    <title>Conversation Lifecycle Layout Gate</title>
  </head>
  <body>
    <main class="shell">
      <aside class="rail" aria-label="workflow">
        <div class="mark">星</div>
        <div class="rail-step is-done"><span>1</span><strong>环境</strong></div>
        <div class="rail-step is-done"><span>2</span><strong>渠道</strong></div>
        <div class="rail-step is-done"><span>3</span><strong>工作区</strong></div>
        <div class="rail-step is-current"><span>4</span><strong>对话</strong></div>
        <div class="rail-step"><span>5</span><strong>收口</strong></div>
      </aside>
      <section class="workspace">
        <header class="topbar">
          <div>
            <p class="eyebrow">星星的vibecoding启动器</p>
            <h1>让梦星星先把想法聊明白</h1>
          </div>
          <button class="icon-button" type="button">↻</button>
        </header>
        <section class="stage initialize-stage">
          <div class="workspace-path-chip">E:\test\test-shop</div>
          <div class="conversation-layout">
            <div class="conversation-panel">
              <div class="conversation-log">
                <article class="message assistant">
                  <strong>梦星星</strong>
                  <p>老师使用这个系统时，最想优先解决的一个核心问题是什么？例如学生信息维护、成绩管理、考勤、作业或家校沟通。</p>
                </article>
              </div>
              <div class="composer">
                <textarea rows="4">告诉梦星星你想做的产品、目标用户、核心功能、偏好的实现形态</textarea>
                <button class="primary" type="button">发送给梦星星</button>
              </div>
            </div>
            <div class="solution-panel">
              <h3>当前理解</h3>
              <p>产品类型：学生管理系统；目标用户：老师；核心问题：帮助老师集中维护学生资料，并高效管理成绩、考勤和作业，减少手工表格管理成本；关键功能：学生信息维护、成绩管理、考勤管理、作业管理、登录与基础权限；建议约束：Web 系统、本地开发运行、轻量技术栈、数据库 SQLite/MySQL、简洁后台界面。</p>
              <p>当前仍需补充：用户确认</p>
            </div>
          </div>
        </section>
        <section class="evidence">
          <div>
            <h3>能力门禁</h3>
            <div class="capability-list">
              <span class="capability pass">superpowers</span>
              <span class="capability pass">agent-browser</span>
              <span class="capability pass">chrome-devtools</span>
              <span class="capability pass">GitNexus</span>
              <span class="capability pass">Speckit</span>
            </div>
          </div>
          <div>
            <h3>运行日志</h3>
            <pre>梦星星会话已启动，接下来会通过自然语言对话澄清需求并准备三套方案。
梦星星会话已建立。</pre>
          </div>
        </section>
      </section>
    </main>
    <pre id="layout-result"></pre>
    <script>
      const rect = (sel) => document.querySelector(sel).getBoundingClientRect();
      const send = [...document.querySelectorAll('button')].find((button) => button.textContent.includes('发送给梦星星')).getBoundingClientRect();
      const stage = rect('.stage');
      const log = rect('.conversation-log');
      const textarea = rect('textarea');
      document.getElementById('layout-result').textContent = JSON.stringify({
        viewport: { width: innerWidth, height: innerHeight },
        metrics: {
          stageHeight: stage.height,
          conversationLogHeight: log.height,
          textareaHeight: textarea.height,
          sendButtonTop: send.top,
          sendButtonBottom: send.bottom,
          bodyScrollWidth: document.body.scrollWidth,
          viewportWidth: document.documentElement.clientWidth
        },
        assertions: {
          stageNotTiny: stage.height >= 560,
          conversationLogNotCollapsed: log.height >= 180,
          textareaUsable: textarea.height >= 120,
          sendButtonVisible: send.top >= 0 && send.bottom <= innerHeight,
          noHorizontalOverflow: document.body.scrollWidth <= document.documentElement.clientWidth + 2
        }
      });
    </script>
  </body>
</html>
"@
}

function New-HandoffFailureLayoutHtml {
    param([string]$CssUri)

    $longError = 'Join-Path : Cannot process argument because the value of argument "drive" is null. Change the value of argument "drive" to a non-null value. At \\?\<repo-root>\release\portable-current\resources\commonhe\tools\common-he-init-orchestrator.ps1:4051 char:19'

    return @"
<!doctype html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <link rel="stylesheet" href="$CssUri" />
    <title>Failure Lifecycle Layout Gate</title>
  </head>
  <body>
    <main class="shell">
      <aside class="rail" aria-label="workflow">
        <div class="mark">星</div>
        <div class="rail-step is-current"><span>5</span><strong>收口</strong></div>
      </aside>
      <section class="workspace">
        <header class="topbar">
          <div>
            <p class="eyebrow">星星的vibecoding启动器</p>
            <h1>让星星先把想法聊明白</h1>
          </div>
        </header>
        <section class="stage handoff is-failure">
          <svg width="40" height="40" viewBox="0 0 24 24"><path fill="currentColor" d="M1 21h22L12 2z"/></svg>
          <h2>初始化被阻断</h2>
          <p>$longError</p>
        </section>
        <section class="evidence">
          <div><h3>能力门禁</h3><div class="capability-list"><span class="capability pass">Speckit</span></div></div>
          <div><h3>运行日志</h3><pre>$longError</pre></div>
        </section>
      </section>
    </main>
    <pre id="layout-result"></pre>
    <script>
      const stage = document.querySelector('.stage').getBoundingClientRect();
      const message = document.querySelector('.handoff p').getBoundingClientRect();
      document.getElementById('layout-result').textContent = JSON.stringify({
        viewport: { width: innerWidth, height: innerHeight },
        metrics: {
          stageWidth: stage.width,
          messageWidth: message.width,
          bodyScrollWidth: document.body.scrollWidth,
          viewportWidth: document.documentElement.clientWidth,
          overflowWrap: getComputedStyle(document.querySelector('.handoff p')).overflowWrap
        },
        assertions: {
          failureMessageInsideStage: message.right <= stage.right + 2 && message.left >= stage.left - 2,
          noHorizontalOverflow: document.body.scrollWidth <= document.documentElement.clientWidth + 2,
          longErrorCanBreak: getComputedStyle(document.querySelector('.handoff p')).overflowWrap === 'anywhere'
        }
      });
    </script>
  </body>
</html>
"@
}

function ConvertTo-VerbatimPath {
    param([string]$Path)

    $fullPath = [System.IO.Path]::GetFullPath($Path)
    if ($fullPath.StartsWith('\\')) {
        return "\\?\UNC\$($fullPath.TrimStart('\'))"
    }

    return "\\?\$fullPath"
}

function New-TestCapabilityCatalog {
    param([string]$Root)

    $capabilityRoot = Join-Path $Root 'capabilities'
    New-Item -ItemType Directory -Path $capabilityRoot -Force | Out-Null
    $catalog = @()
    foreach ($name in @('superpowers', 'agent-browser', 'chrome-devtools', 'GitNexus', 'Speckit')) {
        $marker = Join-Path $capabilityRoot "$name.ok"
        Set-Content -Path $marker -Value 'ok'
        $catalog += @{
            name = $name
            display_name = $name
            required = $true
            verify_command = "$name fixture"
            verification_command = "$name fixture"
            remediation = @("install $name")
            install_commands = @("install $name")
            probes = @(
                @{
                    type = 'file_exists'
                    paths = @($marker)
                }
            )
        }
    }

    $catalogPath = Join-Path $Root 'required-capabilities.json'
    $catalog | ConvertTo-Json -Depth 10 | Set-Content -Path $catalogPath
    $catalogPath
}

function Invoke-OrchestratorLifecycle {
    param(
        [string]$Name,
        [string]$OrchestratorPath
    )

    $workspaceRoot = Join-Path $runRoot $Name
    New-Item -ItemType Directory -Path $workspaceRoot -Force | Out-Null
    $scriptPath = ConvertTo-VerbatimPath -Path $OrchestratorPath
    $statusPath = Join-Path $workspaceRoot '.commonhe\session\status.json'

    function Get-SessionStage {
        if (-not (Test-Path -LiteralPath $statusPath -PathType Leaf)) {
            return ''
        }
        $status = Get-Content -Raw $statusPath | ConvertFrom-Json
        [string]$status.stage
    }

    & $scriptPath -Stage start -ProjectRoot $workspaceRoot | Out-Null
    foreach ($answer in @(
        '学生管理系统',
        '老师使用，管理学生资料、成绩、考勤和作业',
        '目标用户是老师，需要本地运行的 Web 系统',
        '学生信息维护、成绩管理、考勤管理、作业管理、登录权限',
        '轻量技术栈，SQLite 或 MySQL，后台界面简洁',
        '从零开始，优先生成初始化协作包'
    )) {
        if ((Get-SessionStage) -ne 'discovery') {
            break
        }
        & $scriptPath -Stage answer -ProjectRoot $workspaceRoot -InputText $answer | Out-Null
    }
    Assert-Equal -Actual (Get-SessionStage) -Expected 'proposal_ready' -Message "[$Name] lifecycle should finish discovery before proposal."
    & $scriptPath -Stage propose -ProjectRoot $workspaceRoot | Out-Null
    & $scriptPath -Stage confirm -ProjectRoot $workspaceRoot -Choice 'A' | Out-Null
    $result = & $scriptPath -Stage bootstrap -ProjectRoot $workspaceRoot -TargetRoot $workspaceRoot -Execute -Force

    Assert-Equal -Actual $result.Stage -Expected 'implementation_ready' -Message "[$Name] lifecycle should reach implementation_ready."
    Assert-True -Condition ([bool]$result.Postcheck.Passed) -Message "[$Name] lifecycle postcheck should pass."
    Assert-True -Condition ([bool]$result.Postcheck.TruthSourceGatePassed) -Message "[$Name] lifecycle truth-source gate should pass."
    Assert-True -Condition (Test-Path -LiteralPath (Join-Path $workspaceRoot 'AGENTS.md') -PathType Leaf) -Message "[$Name] lifecycle should generate AGENTS.md."
    Assert-True -Condition (Test-Path -LiteralPath (Join-Path $workspaceRoot '.codex\agents\engineering-frontend-developer.md') -PathType Leaf) -Message "[$Name] lifecycle should generate real mapped agents."
    Assert-True -Condition (Test-Path -LiteralPath (Join-Path $workspaceRoot '.specify\templates\spec-template.md') -PathType Leaf) -Message "[$Name] lifecycle should generate .specify payload."

    return $result
}

try {
    New-Item -ItemType Directory -Path $runRoot -Force | Out-Null
    $env:COMMONHE_REQUIRED_CAPABILITIES_PATH = New-TestCapabilityCatalog -Root $runRoot
    Assert-True -Condition (Test-Path -LiteralPath $sourceCssPath -PathType Leaf) -Message 'Desktop CSS is missing.'
    Assert-True -Condition (Test-Path -LiteralPath $sourceOrchestratorPath -PathType Leaf) -Message 'Source orchestrator is missing.'

    Assert-DesktopLayoutContracts -CssPath $sourceCssPath
    if ($env:COMMONHE_ENABLE_BROWSER_LAYOUT_GATE -eq '1') {
        $cssUri = ConvertTo-FileUri -Path $sourceCssPath
        Invoke-HeadlessLayoutGate -Name 'conversation-layout' -Html (New-ConversationLayoutHtml -CssUri $cssUri) | Out-Null
        Invoke-HeadlessLayoutGate -Name 'handoff-failure-layout' -Html (New-HandoffFailureLayoutHtml -CssUri $cssUri) | Out-Null
    }

    Invoke-OrchestratorLifecycle -Name 'source-lifecycle' -OrchestratorPath $sourceOrchestratorPath | Out-Null

    $portableResourceRoot = Join-Path $runRoot 'portable\resources\commonhe'
    & $syncScriptPath -DestinationRoot $portableResourceRoot | Out-Null
    $portableOrchestratorPath = Join-Path $portableResourceRoot 'tools\common-he-init-orchestrator.ps1'
    Invoke-OrchestratorLifecycle -Name 'portable-resource-lifecycle' -OrchestratorPath $portableOrchestratorPath | Out-Null
}
finally {
    if ($null -ne $originalCapabilityCatalogPath) {
        $env:COMMONHE_REQUIRED_CAPABILITIES_PATH = $originalCapabilityCatalogPath
    } else {
        Remove-Item Env:COMMONHE_REQUIRED_CAPABILITIES_PATH -ErrorAction SilentlyContinue
    }
    if (Test-Path -LiteralPath $runRoot) {
        Remove-Item -LiteralPath $runRoot -Recurse -Force
    }
}
