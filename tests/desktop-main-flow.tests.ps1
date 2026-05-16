Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot
$desktopRoot = Join-Path $repoRoot 'apps\desktop'
$tauriRoot = Join-Path $desktopRoot 'src-tauri'
$truthGatePath = Join-Path $repoRoot 'tools\assert-commonhe-truth-source.ps1'
$tmpRoot = Join-Path $repoRoot 'tmp\desktop-main-flow'
$fixtureRoot = Join-Path $tmpRoot ("fixture-" + [System.Guid]::NewGuid().ToString('N'))
$workspaceRoot = Join-Path $fixtureRoot 'workspace'

function Assert-True {
    param(
        [bool]$Condition,
        [string]$Message
    )

    if (-not $Condition) {
        throw $Message
    }
}

function Assert-False {
    param(
        [bool]$Condition,
        [string]$Message
    )

    if ($Condition) {
        throw $Message
    }
}

function Assert-Contains {
    param(
        [string]$Text,
        [string]$Pattern,
        [string]$Message
    )

    if ($Text -notmatch $Pattern) {
        throw $Message
    }
}

function Assert-NotContains {
    param(
        [string]$Text,
        [string]$UnexpectedSubstring,
        [string]$Message
    )

    if ($Text -like "*$UnexpectedSubstring*") {
        throw $Message
    }
}

function Assert-AuthoredInitPackage {
    param(
        [string]$Root,
        [string]$EntryFile,
        [string]$TargetClientName
    )

    $docs = @(
        (Join-Path $Root $EntryFile),
        (Join-Path $Root 'docs\project_context.md'),
        (Join-Path $Root 'docs\00-初始化结果索引.md'),
        (Join-Path $Root 'docs\skills\required-capabilities.md')
    )
    $combined = ($docs | ForEach-Object { Get-Content -Raw $_ }) -join "`n"

    foreach ($badText in @('{{', '}}', 'ExampleProject', 'Build the first usable version quickly', '当前无自动分析假设', '当前无自动分析信号', '生成项目骨架', '当前目录已经生成 HE 协作工程')) {
        Assert-NotContains -Text $combined -UnexpectedSubstring $badText -Message "Generated $TargetClientName init package must not contain template residue: $badText"
    }

    Assert-Contains -Text $combined -Pattern '初始化协作包' -Message "Generated $TargetClientName docs must state init package scope."
    Assert-Contains -Text $combined -Pattern $TargetClientName -Message "Generated docs must name the selected target client."
    Assert-Contains -Text $combined -Pattern 'superpowers' -Message "Generated docs must include selected capability status."
}

function New-TargetClientFixture {
    param(
        [string]$Root,
        [string]$TargetClient,
        [string]$ProjectName
    )

    $sessionRoot = Join-Path $Root '.commonhe\session'
    New-Item -ItemType Directory -Path $sessionRoot -Force | Out-Null
    $mandatoryCapabilities = @(
        @{ id = 'superpowers'; label = 'superpowers / required skills'; detail = 'skill_presence: using-superpowers, test-driven-development' }
        @{ id = 'agent-browser'; label = 'agent-browser'; detail = 'agent-browser is required for browser automation handoff' }
        @{ id = 'chrome-devtools'; label = 'chrome-devtools MCP'; detail = 'chrome-devtools MCP is required for browser verification' }
        @{ id = 'GitNexus'; label = 'GitNexus'; detail = 'gitnexus --version' }
        @{ id = 'Speckit'; label = 'Speckit'; detail = 'specify --version' }
    )
    @{
        user_confirmed = $true
        project_name = $ProjectName
        project_type = '学生管理系统'
        target_client = $TargetClient
        selected_solution_id = 'B'
        selected_solution_title = '学生管理系统平衡方案'
        solution_mode = 'balanced'
        enabled_roles = @('docs', 'reviewer')
        recommended_roles_now = @('docs', 'reviewer')
        available_roles_later = @()
        integrations = @()
        selected_capabilities = @(
            foreach ($capability in $mandatoryCapabilities) {
                @{
                    id = $capability.id
                    label = $capability.label
                    recommended = $true
                    selected = $true
                    required = $true
                    locked = $true
                    status = 'fallback'
                    detail = $capability.detail
                }
            }
        )
        external_references = @()
        detected_integrations = @()
        current_stage = 'implementation'
        current_stage_goal = '生成面向学生管理系统的初始化协作包'
        primary_workstream = 'fullstack'
        stage_constraints = @('必须保持桌面主流程真实、可验证、可收口')
        deferred_capabilities = @()
        implementation_checklist_seed = @('初始化协作包已生成')
        implementation_acceptance_seed = @('postcheck 通过')
        required_capabilities = @(
            foreach ($capability in $mandatoryCapabilities) {
                @{ name = $capability.id; display_name = $capability.label }
            }
        )
        capability_probe_results = @(
            foreach ($capability in $mandatoryCapabilities) {
                @{ name = $capability.id; display_name = $capability.label; passed = $true; evidence = 'test fixture'; verification_command = $capability.detail; remediation = @() }
            }
        )
        signal_categories = @()
        role_rationale = @{}
        confidence_breakdown = @{}
        dominant_workstreams = @()
        kickoff_pack = @{}
        autodiscovery_signals = @()
        autodiscovery_assumptions = @()
        project_goal_summary = '产品类型：学生管理系统；目标用户：老师；核心功能：学生管理、成绩管理、班级管理。'
        target_users_summary = '老师'
        core_features_summary = '学生管理、成绩管理、班级管理'
        constraints_summary = '支持 Web 和小程序'
    } | ConvertTo-Json -Depth 20 | Set-Content -Path (Join-Path $sessionRoot 'decision.json')

    @{
        project_name = $ProjectName
        project_goal = '产品类型：学生管理系统；目标用户：老师；核心功能：学生管理、成绩管理、班级管理。'
        target_users = '老师'
        core_features = '学生管理、成绩管理、班级管理'
        constraints = '支持 Web 和小程序'
    } | ConvertTo-Json -Depth 10 | Set-Content -Path (Join-Path $sessionRoot 'answers.json')

    @{
        stage = 'proposal'
        precheck_passed = $true
        precheck_failed = $false
        session_root = $sessionRoot
        question_source = 'desktop-agent'
        started_at = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds().ToString()
        semantic_review_passed = $true
        semantic_review_failed = $false
        semantic_review_issues = @()
        semantic_review_rounds = 1
        capability_gate_passed = $true
    } | ConvertTo-Json -Depth 10 | Set-Content -Path (Join-Path $sessionRoot 'status.json')

    @{
        mainAgent = '梦星星'
        understandingSummary = '面向学生管理系统的初始化协作包。'
        selectedSolutionId = 'B'
        solutions = @()
    } | ConvertTo-Json -Depth 10 | Set-Content -Path (Join-Path $sessionRoot 'meng-xingxing-output.json')
    @{
        passed = $true
        blockingIssues = @()
        questionsForMengXingxing = @()
        requiredRepairs = @()
        reviewSummary = '星梦梦复核通过。'
        confidence = 'high'
    } | ConvertTo-Json -Depth 10 | Set-Content -Path (Join-Path $sessionRoot 'xing-mengmeng-review.json')
    @{
        round = 1
        reviewerAgent = '星梦梦'
        mainAgent = '梦星星'
        review = @{ passed = $true; blockingIssues = @(); questionsForMengXingxing = @(); requiredRepairs = @(); reviewSummary = '星梦梦复核通过。'; confidence = 'high' }
    } | ConvertTo-Json -Depth 10 -Compress | Set-Content -Path (Join-Path $sessionRoot 'agent-dialogue-rounds.jsonl')
    Set-Content -Path (Join-Path $sessionRoot 'repair-decisions.json') -Value '[]'
    @{
        passed = $true
        reviewerAgent = '星梦梦'
        mainAgent = '梦星星'
        blockingIssues = @()
        acceptedAt = (Get-Date).ToString('s')
        targetClient = $TargetClient
        selectedSolutionId = 'B'
        reviewRounds = 1
    } | ConvertTo-Json -Depth 10 | Set-Content -Path (Join-Path $sessionRoot 'final-acceptance.json')

    $sessionRoot
}

try {
    New-Item -ItemType Directory -Path $workspaceRoot -Force | Out-Null
    Set-Content -Path (Join-Path $fixtureRoot 'fixture.json') -Value (@{
        created_at = (Get-Date).ToString('s')
        workspace = $workspaceRoot
        purpose = 'desktop-main-flow automated test harness'
    } | ConvertTo-Json -Depth 5)

    $goalDoc = Get-Content -Raw (Join-Path $repoRoot 'docs\CommonHE开发目标.md')
    $agentsDoc = Get-Content -Raw (Join-Path $repoRoot 'AGENTS.md')

    Assert-Contains -Text $goalDoc -Pattern 'tmp/' -Message 'True source doc must define tmp/ usage.'
    Assert-Contains -Text $goalDoc -Pattern 'docs/' -Message 'True source doc must define docs/ usage.'
    Assert-Contains -Text $agentsDoc -Pattern 'Speckit|speckit' -Message 'AGENTS.md must require Speckit planning.'
    Assert-Contains -Text $agentsDoc -Pattern 'tmp/' -Message 'AGENTS.md must mention tmp/ conventions.'

    $desktopApp = Get-Content -Raw (Join-Path $repoRoot 'apps\desktop\src\App.tsx')
    $agentFlow = Get-Content -Raw (Join-Path $repoRoot 'apps\desktop\src\agentFlow.ts')
    Assert-Contains -Text $agentFlow -Pattern '发送给梦星星' -Message 'Desktop UI must send to 梦星星.'
    Assert-Contains -Text ($agentFlow + "`n" + $desktopApp) -Pattern '梦星星思考中|梦星星正在思考' -Message 'Desktop UI must expose a visible loading state while 梦星星 is responding.'
    Assert-Contains -Text ($agentFlow + "`n" + $desktopApp) -Pattern '星梦梦' -Message 'Desktop UI must expose 星梦梦 semantic review state.'
    if ($desktopApp -match '主Agent') {
        throw 'Desktop UI must not expose 主Agent wording in the main path.'
    }

    Push-Location $desktopRoot
    try {
        npm test | Out-Null
    }
    finally {
        Pop-Location
    }

    Push-Location $tauriRoot
    try {
        cargo test execute_solution_bootstrap -- --nocapture | Out-Null
        cargo test solutions_are_blocked_until_readiness_is_confirmed -- --nocapture | Out-Null
    }
    finally {
        Pop-Location
    }

    Assert-True (Test-Path -LiteralPath $tmpRoot -PathType Container) 'tmp/desktop-main-flow must exist for harness output.'
    Assert-True (Test-Path -LiteralPath $fixtureRoot -PathType Container) 'Harness fixture root must be created inside tmp/.'
    Assert-True (Test-Path -LiteralPath (Join-Path $fixtureRoot 'fixture.json') -PathType Leaf) 'Harness must leave a fixture manifest in tmp/.'

    $codexRoot = Join-Path $fixtureRoot 'codex-package'
    $codexSession = New-TargetClientFixture -Root $codexRoot -TargetClient 'codex' -ProjectName '学生管理Codex协作包'
    & (Join-Path $repoRoot 'tools\common-he-init-orchestrator.ps1') -Stage bootstrap -SessionRoot $codexSession -TargetRoot $codexRoot -Execute -Force | Out-Null
    $codexGate = & $truthGatePath -GeneratedRoot $codexRoot -TargetClient codex -AsJson | ConvertFrom-Json
    Assert-True (Test-Path -LiteralPath (Join-Path $codexRoot 'AGENTS.md') -PathType Leaf) 'Codex package must generate AGENTS.md.'
    Assert-True (Test-Path -LiteralPath (Join-Path $codexRoot '.codex\COORDINATOR-SUBAGENTS.md') -PathType Leaf) 'Codex package must generate .codex coordinator.'
    Assert-True (Test-Path -LiteralPath (Join-Path $codexRoot '.agents\skills\required-capabilities.md') -PathType Leaf) 'Codex package must generate .agents skills manifest.'
    Assert-False (Test-Path -LiteralPath (Join-Path $codexRoot 'CLAUDE.md') -PathType Leaf) 'Codex package must not generate CLAUDE.md as primary entry.'
    Assert-True ([bool]$codexGate.Passed) "Codex target package must pass truth-source gate: $($codexGate.Issues -join '; ')"
    Assert-AuthoredInitPackage -Root $codexRoot -EntryFile 'AGENTS.md' -TargetClientName 'Codex'

    $claudeRoot = Join-Path $fixtureRoot 'claude-package'
    $claudeSession = New-TargetClientFixture -Root $claudeRoot -TargetClient 'claude-code' -ProjectName '学生管理Claude协作包'
    & (Join-Path $repoRoot 'tools\common-he-init-orchestrator.ps1') -Stage bootstrap -SessionRoot $claudeSession -TargetRoot $claudeRoot -Execute -Force | Out-Null
    $claudeGate = & $truthGatePath -GeneratedRoot $claudeRoot -TargetClient 'claude-code' -AsJson | ConvertFrom-Json
    Assert-True (Test-Path -LiteralPath (Join-Path $claudeRoot 'CLAUDE.md') -PathType Leaf) 'Claude Code package must generate CLAUDE.md.'
    Assert-True (Test-Path -LiteralPath (Join-Path $claudeRoot '.claude\settings.json') -PathType Leaf) 'Claude Code package must generate .claude settings.'
    Assert-False (Test-Path -LiteralPath (Join-Path $claudeRoot 'AGENTS.md') -PathType Leaf) 'Claude Code package must not generate AGENTS.md as primary entry.'
    Assert-True ([bool]$claudeGate.Passed) "Claude Code target package must pass truth-source gate: $($claudeGate.Issues -join '; ')"
    Assert-AuthoredInitPackage -Root $claudeRoot -EntryFile 'CLAUDE.md' -TargetClientName 'Claude Code'
}
finally {
    if (Test-Path -LiteralPath $fixtureRoot) {
        Remove-Item -LiteralPath $fixtureRoot -Recurse -Force
    }
}
