Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot
$orchestratorPath = Join-Path $repoRoot 'tools\common-he-init-orchestrator.ps1'
$initPath = Join-Path $repoRoot 'tools\init-common-he.ps1'
$truthGatePath = Join-Path $repoRoot 'tools\assert-commonhe-truth-source.ps1'
$sampleValuesPath = Join-Path $repoRoot 'config\init-values.sample.json'
$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("CommonHE-Tests-" + [System.Guid]::NewGuid().ToString('N'))

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
        throw "$Message`nExpected: $Expected`nActual: $Actual"
    }
}

function Assert-Contains {
    param(
        [string]$Text,
        [string]$ExpectedSubstring,
        [string]$Message
    )

    if (-not ([string]$Text).Contains([string]$ExpectedSubstring)) {
        throw "$Message`nExpected substring: $ExpectedSubstring`nActual text: $Text"
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

function Assert-NotContains {
    param(
        [string]$Text,
        [string]$UnexpectedSubstring,
        [string]$Message
    )

    if ($Text -like "*$UnexpectedSubstring*") {
        throw "$Message`nUnexpected substring: $UnexpectedSubstring`nActual text: $Text"
    }
}

function Assert-NoDuplicateMarkdownSectionNumbers {
    param(
        [string]$Text,
        [string]$Message
    )

    $seen = @{}
    foreach ($match in [regex]::Matches($Text, '(?m)^##\s+([0-9]+)\.')) {
        $number = [string]$match.Groups[1].Value
        if ($seen.ContainsKey($number)) {
            throw "$Message`nDuplicate markdown section number: $number"
        }
        $seen[$number] = $true
    }
}

function Set-TestCapabilityDefaults {
    param(
        [pscustomobject]$Values,
        [object[]]$Capabilities
    )

    $capabilityList = if (@($Capabilities).Count -gt 0) {
        ($Capabilities | ForEach-Object { "- $($_.name)" }) -join "`n"
    } else {
        "- superpowers`n- agent-browser`n- chrome-devtools`n- GitNexus`n- Speckit"
    }
    $probeSummary = if (@($Capabilities).Count -gt 0) {
        ($Capabilities | ForEach-Object { "- $($_.name): pass" }) -join "`n"
    } else {
        "- superpowers: pass`n- agent-browser: pass`n- chrome-devtools: pass`n- GitNexus: pass`n- Speckit: pass"
    }

    $Values | Add-Member -NotePropertyName required_capabilities_list -NotePropertyValue $capabilityList -Force
    $Values | Add-Member -NotePropertyName capability_probe_summary -NotePropertyValue $probeSummary -Force
    $Values | Add-Member -NotePropertyName capability_gate_status -NotePropertyValue '- 绿色：doctor 与 precheck 已通过' -Force
    $Values | Add-Member -NotePropertyName autodiscovery_assumptions -NotePropertyValue '- 当前没有额外未验证假设；后续范围变化需重新回写真源。' -Force
    $Values | Add-Member -NotePropertyName autodiscovery_signal_summary -NotePropertyValue '- 主 Agent 已完成当前收口，本轮未保留额外自动分析信号。' -Force
}

function New-TestEnvironment {
    param(
        [string]$Name,
        [string[]]$EnabledRoles,
        [object[]]$Integrations = @()
    )

    $root = Join-Path $tempRoot $Name
    $sessionRoot = Join-Path $root '.commonhe\session'
    New-Item -ItemType Directory -Path $sessionRoot -Force | Out-Null

    $values = Get-Content -Raw $sampleValuesPath | ConvertFrom-Json
    $values.project_name = "Test-$Name"
    $values.project_type = 'web-app'
    $values.enabled_roles = (($EnabledRoles | ForEach-Object { "- $_" }) -join "`n")
    $values.roles_and_manuals = $values.enabled_roles
    $values.agent_dispatch_matrix = $values.enabled_roles
    $values.core_goal = '完成初始化协作包收口，并准备首轮实施接手'
    $values.current_goals = "- 完成初始化协作包收口`n- 生成 Codex 原生入口`n- 保持后续接手真源清晰"
    $values.in_scope_items = "- 初始化协作包真源文档`n- Codex 原生入口与角色调度文件`n- 已选能力记录与后续接手说明"
    $values.out_of_scope_items = "- 本轮不生成业务项目成品、业务代码或业务脚手架`n- 未经用户确认的扩展能力不默认启用"
    $values.current_phase_tasks = "- 读取本初始化协作包的真源入口`n- 在 Codex 新会话中从 AGENTS.md 接手`n- 按 docs/workflow/first-task-pack.md 推进首轮实施"
    $values.project_goal = '完成初始化协作包收口'
    $values.project_goal_summary = '完成初始化协作包收口'
    $values.current_phase_goal = '完成面向 Codex 的初始化协作包收口，并准备首轮实施接手'
    $values.tech_stack = 'PowerShell'
    $requiredCapabilities = @(
        @{ name = 'superpowers'; display_name = 'superpowers' }
        @{ name = 'agent-browser'; display_name = 'agent-browser' }
        @{ name = 'chrome-devtools'; display_name = 'chrome-devtools' }
        @{ name = 'GitNexus'; display_name = 'GitNexus' }
        @{ name = 'Speckit'; display_name = 'Speckit' }
    )
    Set-TestCapabilityDefaults -Values $values -Capabilities $requiredCapabilities
    $valuesPath = Join-Path $root 'values.json'
    $values | ConvertTo-Json -Depth 10 | Set-Content -Path $valuesPath

    $decision = @{
        user_confirmed = $true
        project_name = "Test-$Name"
        project_type = 'web-app'
        solution_mode = 'balanced'
        enabled_roles = $EnabledRoles
        integrations = $Integrations
        discovery_mode = 'legacy_zero_question'
        auto_confirmed = $true
        confirmation_mode = 'auto_legacy_init'
        required_capabilities = $requiredCapabilities
        selected_capabilities = @(
            foreach ($capability in $requiredCapabilities) {
                @{
                    id = $capability.name
                    label = $capability.display_name
                    selected = $true
                    required = $true
                    locked = $true
                    status = 'available'
                }
            }
        )
        capability_probe_results = @(
            foreach ($capability in $requiredCapabilities) {
                @{
                    name = $capability.name
                    passed = $true
                    evidence = 'test seed'
                }
            }
        )
        analysis_confidence = 'high'
        autodiscovery_signals = @('test-environment')
        autodiscovery_assumptions = @()
    }
    $decision | ConvertTo-Json -Depth 10 | Set-Content -Path (Join-Path $sessionRoot 'decision.json')

    $status = @{
        stage = 'confirmed'
        current_question_index = $null
        session_root = [System.IO.Path]::GetFullPath($sessionRoot)
        started_at = '2026-04-14T10:00:00'
        last_postcheck_passed = $null
        init_closed = $false
        last_postcheck_summary = $null
        doctor_passed = $true
        doctor_failed = $false
        entrycheck_passed = $true
        entrycheck_failed = $false
        misplaced_package_detected = $false
        precheck_passed = $true
        precheck_failed = $false
        capability_gate_passed = $true
        legacy_project_detected = $true
        legacy_zero_question_mode = $true
        missing_capabilities = @()
    }
    $status | ConvertTo-Json -Depth 10 | Set-Content -Path (Join-Path $sessionRoot 'status.json')

    return @{
        Root = $root
        SessionRoot = $sessionRoot
        ValuesPath = $valuesPath
    }
}

function New-MinimalTruthSourcePackage {
    param(
        [string]$Root,
        [string]$TargetClient = 'codex',
        [bool]$IncludeSpeckit = $true
    )

    New-Item -ItemType Directory -Path $Root -Force | Out-Null
    if ($IncludeSpeckit) {
        New-Item -ItemType Directory -Path (Join-Path $Root '.specify\templates') -Force | Out-Null
        New-Item -ItemType Directory -Path (Join-Path $Root '.specify\scripts\powershell') -Force | Out-Null
        Set-Content -Path (Join-Path $Root '.specify\templates\spec-template.md') -Value '# spec template'
        Set-Content -Path (Join-Path $Root '.specify\scripts\powershell\create-new-feature.ps1') -Value 'param()'
    }
    New-Item -ItemType Directory -Path (Join-Path $Root 'docs') -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $Root '.commonhe\session') -Force | Out-Null
    Set-Content -Path (Join-Path $Root 'docs\project_context.md') -Value '# 项目语境`n这是面向学生管理系统的初始化协作包。'
    Set-Content -Path (Join-Path $Root 'docs\00-初始化结果索引.md') -Value '# 初始化结果索引`n初始化协作包已经收口。'
    $selectedCapabilities = @(
        @{ id = 'superpowers'; selected = $true; status = 'bundled' }
        @{ id = 'agent-browser'; selected = $true; status = 'bundled' }
        @{ id = 'chrome-devtools'; selected = $true; status = 'bundled' }
        @{ id = 'GitNexus'; selected = $true; status = 'bundled' }
        @{ id = 'Speckit'; selected = $true; status = 'bundled' }
    )
        Set-Content -Path (Join-Path $Root '.commonhe\session\decision.json') -Value (@{
            target_client = $TargetClient
            enabled_roles = @('backend', 'frontend', 'miniapp', 'qa', 'reviewer', 'docs')
            selected_capabilities = $selectedCapabilities
            selected_solution_id = 'B'
        } | ConvertTo-Json -Depth 8)
    Set-Content -Path (Join-Path $Root '.commonhe\session\status.json') -Value (@{
        stage = 'implementation_ready'
        question_source = 'desktop-agent'
        capability_gate_passed = $true
        semantic_review_passed = $true
        semantic_review_failed = $false
        semantic_review_issues = @()
        semantic_review_rounds = 1
    } | ConvertTo-Json -Depth 4)
    Set-Content -Path (Join-Path $Root '.commonhe\session\meng-xingxing-output.json') -Value (@{
        mainAgent = '梦星星'
        understandingSummary = '面向学生管理系统的初始化协作包。'
        selectedSolutionId = 'B'
        solutions = @()
    } | ConvertTo-Json -Depth 8)
    Set-Content -Path (Join-Path $Root '.commonhe\session\xing-mengmeng-review.json') -Value (@{
        passed = $true
        blockingIssues = @()
        questionsForMengXingxing = @()
        requiredRepairs = @()
        reviewSummary = '星梦梦复核通过。'
        confidence = 'high'
    } | ConvertTo-Json -Depth 8)
    Set-Content -Path (Join-Path $Root '.commonhe\session\agent-dialogue-rounds.jsonl') -Value (@{
        round = 1
        reviewerAgent = '星梦梦'
        mainAgent = '梦星星'
        review = @{ passed = $true; blockingIssues = @(); questionsForMengXingxing = @(); requiredRepairs = @(); reviewSummary = '星梦梦复核通过。'; confidence = 'high' }
    } | ConvertTo-Json -Depth 10 -Compress)
    Set-Content -Path (Join-Path $Root '.commonhe\session\repair-decisions.json') -Value '[]'
    Set-Content -Path (Join-Path $Root '.commonhe\session\final-acceptance.json') -Value (@{
        passed = $true
        reviewerAgent = '星梦梦'
        mainAgent = '梦星星'
        blockingIssues = @()
        acceptedAt = (Get-Date).ToString('s')
        targetClient = $TargetClient
        selectedSolutionId = 'B'
        reviewRounds = 1
    } | ConvertTo-Json -Depth 8)

    if ($TargetClient -eq 'claude-code') {
        New-Item -ItemType Directory -Path (Join-Path $Root '.claude\agents') -Force | Out-Null
        Set-Content -Path (Join-Path $Root 'CLAUDE.md') -Value '# CLAUDE`n初始化协作包入口。'
        Set-Content -Path (Join-Path $Root '.claude\settings.json') -Value '{}'
        Set-Content -Path (Join-Path $Root '.claude\agents\engineering-frontend-developer.md') -Value "# engineering-frontend-developer`n`n来源：agency-agents-zh"
    } else {
        New-Item -ItemType Directory -Path (Join-Path $Root '.codex\agents') -Force | Out-Null
        New-Item -ItemType Directory -Path (Join-Path $Root '.agents\skills') -Force | Out-Null
        Set-Content -Path (Join-Path $Root 'AGENTS.md') -Value "# AGENTS`n初始化协作包入口。`n`n## 1. 入口`n`n## 2. 调度`n`n- 统一后端 API -> backend；负责接口契约。"
        Set-Content -Path (Join-Path $Root '.codex\COORDINATOR-SUBAGENTS.md') -Value "# Coordinator`n`n- 统一后端 API -> backend；负责接口契约。"
        Set-Content -Path (Join-Path $Root '.agents\skills\required-capabilities.md') -Value '# required capabilities'
        Set-Content -Path (Join-Path $Root '.codex\agents\engineering-frontend-developer.md') -Value "# engineering-frontend-developer`n`n来源：agency-agents-zh"
    }
    New-Item -ItemType Directory -Path (Join-Path $Root 'docs\workflow') -Force | Out-Null
    Set-Content -Path (Join-Path $Root 'docs\workflow\current-stage-user-checklist.md') -Value "# 当前阶段用户待办清单`n`n- 阅读 AGENTS.md、docs/project_context.md 与 docs/workflow/first-task-pack.md。`n- 在新线程中确认首轮实施范围与验收口径。"
    Set-Content -Path (Join-Path $Root 'docs\workflow\first-sprint-contract.md') -Value "# 首轮实施合同`n`n当前初始化协作包的第一轮接手合同。`n`n## 第一优先工作流`n- 阅读入口文档、确认首轮范围、建立任务契约并记录验证证据。"
    Set-Content -Path (Join-Path $Root 'docs\workflow\sprint-contract-template.md') -Value "# Sprint Contract 通用模板`n`n后续任务复制本文件后再填写。"
}

function New-CapabilityCatalog {
    param(
        [string]$Root,
        [bool]$IncludeSuperpowers = $true,
        [bool]$IncludeAgentBrowser = $true,
        [bool]$IncludeChromeDevtools = $true,
        [bool]$IncludeGitNexus = $true,
        [bool]$IncludeSpeckit = $true
    )

    $codexConfigPath = Join-Path $Root 'codex\config.toml'
    $catalog = @(
        @{
            name = 'superpowers'
            display_name = 'superpowers'
            group = 'core'
            verify_command = '确认本机 skill 目录同时存在 using-superpowers 与 test-driven-development'
            verification_command = 'superpowers-check'
            remediation = @('install superpowers')
            install_commands = @('按照 superpowers 官方方式安装 using-superpowers 与 test-driven-development')
            install_notes = @('两项 skill 必须同时存在')
            probes = @(
                @{ type = 'file_exists'; paths = @((Join-Path $Root 'capabilities\superpowers.ok')) }
            )
        }
        @{
            name = 'agent-browser'
            display_name = 'agent-browser'
            group = 'browser'
            verify_command = '确认 agent-browser 已安装并具备本机配置'
            verification_command = 'agent-browser-check'
            remediation = @('install agent-browser')
            install_commands = @('按 agent-browser 官方安装说明完成安装与配置')
            install_notes = @('浏览器自动化能力组要求 agent-browser 与 chrome-devtools 同时存在')
            probes = @(
                @{ type = 'file_exists'; paths = @((Join-Path $Root 'capabilities\agent-browser.ok')) }
            )
        }
        @{
            name = 'chrome-devtools'
            display_name = 'chrome-devtools'
            group = 'browser'
            verify_command = '在 ~/.codex/config.toml 中确认存在 [mcp_servers.chrome-devtools]'
            verification_command = '~/.codex/config.toml contains [mcp_servers.chrome-devtools]'
            remediation = @('install chrome-devtools')
            install_commands = @('codex mcp add chrome-devtools -- npx chrome-devtools-mcp@latest')
            install_notes = @('默认通过 Codex MCP 配置加载，不依赖独立 CLI')
            probes = @(
                @{ type = 'file_contains'; path = $codexConfigPath; patterns = @('[mcp_servers.chrome-devtools]', 'chrome-devtools-mcp') }
            )
        }
        @{
            name = 'GitNexus'
            display_name = 'GitNexus'
            group = 'core'
            verify_command = 'gitnexus --version'
            verification_command = 'gitnexus --version'
            remediation = @('install GitNexus')
            install_commands = @('npm install -g gitnexus')
            install_notes = @('若全局命令不可用，可退回 npx gitnexus --help')
            probes = @(
                @{ type = 'file_exists'; paths = @((Join-Path $Root 'capabilities\gitnexus.ok')) }
            )
        }
        @{
            name = 'Speckit'
            display_name = 'Speckit'
            group = 'core'
            verify_command = 'specify --version'
            verification_command = 'specify --version'
            remediation = @('install Speckit')
            install_commands = @('uv tool install specify-cli')
            install_notes = @('Spec Kit 官方 CLI 通常是 specify')
            probes = @(
                @{ type = 'file_exists'; paths = @((Join-Path $Root 'capabilities\speckit.ok')) }
            )
        }
    )

    New-Item -ItemType Directory -Path $Root -Force | Out-Null
    New-Item -ItemType Directory -Path (Split-Path -Parent $codexConfigPath) -Force | Out-Null
    if ($IncludeChromeDevtools) {
        Set-Content -Path $codexConfigPath -Value @"
[mcp_servers.chrome-devtools]
type = "stdio"
command = "npx"
args = ["chrome-devtools-mcp@latest"]
"@
    } else {
        Set-Content -Path $codexConfigPath -Value @"
[mcp_servers.other]
type = "stdio"
command = "npx"
args = ["other-mcp"]
"@
    }
    $catalogPath = Join-Path $Root 'required-capabilities.json'
    $catalog | ConvertTo-Json -Depth 10 | Set-Content -Path $catalogPath

    $capabilityDir = Join-Path $Root 'capabilities'
    New-Item -ItemType Directory -Path $capabilityDir -Force | Out-Null
    if ($IncludeSuperpowers) { Set-Content -Path (Join-Path $capabilityDir 'superpowers.ok') -Value 'ok' }
    if ($IncludeAgentBrowser) { Set-Content -Path (Join-Path $capabilityDir 'agent-browser.ok') -Value 'ok' }
    if ($IncludeGitNexus) { Set-Content -Path (Join-Path $capabilityDir 'gitnexus.ok') -Value 'ok' }
    if ($IncludeSpeckit) { Set-Content -Path (Join-Path $capabilityDir 'speckit.ok') -Value 'ok' }

    $catalogPath
}

function New-LegacyProject {
    param(
        [string]$Name,
        [switch]$WithTests,
        [switch]$WithBackend,
        [switch]$WithDatabase,
        [switch]$WithDeploy
    )

    $root = Join-Path $tempRoot $Name
    New-Item -ItemType Directory -Path $root -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $root 'src') -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $root '.git') -Force | Out-Null
    Set-Content -Path (Join-Path $root 'package.json') -Value '{ "name": "legacy-demo", "private": true }'
    Set-Content -Path (Join-Path $root 'README.md') -Value '# Legacy Demo'
    Set-Content -Path (Join-Path $root 'src\index.ts') -Value 'export const demo = true;'
    if ($WithTests) {
        New-Item -ItemType Directory -Path (Join-Path $root 'tests') -Force | Out-Null
        Set-Content -Path (Join-Path $root 'tests\index.test.ts') -Value 'test("demo", () => {});'
        Set-Content -Path (Join-Path $root 'package.json') -Value '{ "name": "legacy-demo", "private": true, "scripts": { "test": "vitest" } }'
    }
    if ($WithBackend) {
        New-Item -ItemType Directory -Path (Join-Path $root 'server') -Force | Out-Null
        Set-Content -Path (Join-Path $root 'server\index.js') -Value 'module.exports = {};'
        $backendPackageJson = if ($WithTests) {
            '{ "name": "legacy-demo", "private": true, "scripts": { "test": "vitest", "start:api": "node server/index.js" }, "dependencies": { "express": "^4.0.0" } }'
        } else {
            '{ "name": "legacy-demo", "private": true, "scripts": { "start:api": "node server/index.js" }, "dependencies": { "express": "^4.0.0" } }'
        }
        Set-Content -Path (Join-Path $root 'package.json') -Value $backendPackageJson
    }
    if ($WithDatabase) {
        New-Item -ItemType Directory -Path (Join-Path $root 'migrations') -Force | Out-Null
        Set-Content -Path (Join-Path $root 'migrations\001_init.sql') -Value 'create table demo(id int);'
    }
    if ($WithDeploy) {
        Set-Content -Path (Join-Path $root 'Dockerfile') -Value 'FROM node:20-alpine'
    }
    $root
}

function Invoke-TestCase {
    param(
        [string]$Name,
        [scriptblock]$Body
    )

    try {
        & $Body
        Write-Host "PASS $Name"
    } catch {
        Write-Host "FAIL $Name"
        throw
    }
}

try {
    New-Item -ItemType Directory -Path $tempRoot -Force | Out-Null
    $originalCapabilityCatalogPath = $env:COMMONHE_REQUIRED_CAPABILITIES_PATH
    $defaultCapabilityCatalogPath = New-CapabilityCatalog -Root (Join-Path $tempRoot 'default-capabilities')
    $env:COMMONHE_REQUIRED_CAPABILITIES_PATH = $defaultCapabilityCatalogPath

    Invoke-TestCase -Name 'truth-source gate script catches target package violations' -Body {
        $gateRoot = Join-Path $tempRoot 'truth-gate-broken-codex'
        New-MinimalTruthSourcePackage -Root $gateRoot -TargetClient 'codex'
        Remove-Item -LiteralPath (Join-Path $gateRoot '.specify') -Recurse -Force
        Set-Content -Path (Join-Path $gateRoot 'CLAUDE.md') -Value '# wrong target entry'
        Set-Content -Path (Join-Path $gateRoot '.codex\agents\frontend.md') -Value '# placeholder agent'
        Add-Content -Path (Join-Path $gateRoot 'docs\project_context.md') -Value "`n业务项目成品已生成"
        Add-Content -Path (Join-Path $gateRoot 'docs\project_context.md') -Value "`n�"

        $gate = & $truthGatePath -GeneratedRoot $gateRoot -TargetClient codex -AsJson | ConvertFrom-Json

        Assert-False -Condition ([bool]$gate.Passed) -Message 'Broken Codex target package must fail truth-source gate.'
        Assert-Contains -Text ([string]($gate.Issues -join "`n")) -ExpectedSubstring '.specify' -Message 'Truth-source gate must report missing .specify.'
        Assert-Contains -Text ([string]($gate.Issues -join "`n")) -ExpectedSubstring 'CLAUDE.md' -Message 'Truth-source gate must report wrong target entry.'
        Assert-Contains -Text ([string]($gate.Issues -join "`n")) -ExpectedSubstring 'frontend.md' -Message 'Truth-source gate must report placeholder agent files.'
        Assert-Contains -Text ([string]($gate.Issues -join "`n")) -ExpectedSubstring '业务项目成品已生成' -Message 'Truth-source gate must report misleading business-product claims.'
        Assert-Contains -Text ([string]($gate.Issues -join "`n")) -ExpectedSubstring 'replacement character' -Message 'Truth-source gate must report mojibake replacement characters.'
    }

    Invoke-TestCase -Name 'truth-source gate passes clean minimal target packages' -Body {
        $codexGateRoot = Join-Path $tempRoot 'truth-gate-clean-codex'
        New-MinimalTruthSourcePackage -Root $codexGateRoot -TargetClient 'codex'
        $codexGate = & $truthGatePath -GeneratedRoot $codexGateRoot -TargetClient codex -AsJson | ConvertFrom-Json

        $claudeGateRoot = Join-Path $tempRoot 'truth-gate-clean-claude'
        New-MinimalTruthSourcePackage -Root $claudeGateRoot -TargetClient 'claude-code'
        $claudeGate = & $truthGatePath -GeneratedRoot $claudeGateRoot -TargetClient 'claude-code' -AsJson | ConvertFrom-Json

        Assert-True -Condition ([bool]$codexGate.Passed) -Message "Clean Codex target package should pass truth-source gate: $($codexGate.Issues -join '; ')"
        Assert-True -Condition ([bool]$claudeGate.Passed) -Message "Clean Claude Code target package should pass truth-source gate: $($claudeGate.Issues -join '; ')"
    }

    Invoke-TestCase -Name 'truth-source gate fails desktop package when selected audit fields drift' -Body {
        $gateRoot = Join-Path $tempRoot 'truth-gate-selected-audit-drift'
        New-MinimalTruthSourcePackage -Root $gateRoot -TargetClient 'codex'
        $decisionPath = Join-Path $gateRoot '.commonhe\session\decision.json'
        $statusPath = Join-Path $gateRoot '.commonhe\session\status.json'
        $agentsPath = Join-Path $gateRoot 'AGENTS.md'

        $decision = Get-Content -Raw $decisionPath | ConvertFrom-Json
        $decision.selected_capabilities = @()
        $decision.PSObject.Properties.Remove('selected_solution_id')
        $decision | Add-Member -NotePropertyName enabled_roles -NotePropertyValue @('backend', 'frontend', 'miniapp', 'qa', 'reviewer', 'docs') -Force
        $decision | ConvertTo-Json -Depth 8 | Set-Content -Path $decisionPath

        $status = Get-Content -Raw $statusPath | ConvertFrom-Json
        $status.capability_gate_passed = $false
        $status | ConvertTo-Json -Depth 8 | Set-Content -Path $statusPath
        Add-Content -Path $agentsPath -Value ("`n" + [string][char]7 + "`n这是补救措施，不是 CommonHE 正式流程。`n`n## 5. 本轮能力选择`n`n## 5. 任务状态机`n`n- 统一后端 API -> backend + database；负责接口契约。")
        Add-Content -Path (Join-Path $gateRoot '.codex\COORDINATOR-SUBAGENTS.md') -Value "`n- 统一后端 API -> backend + database；负责接口契约。"
        Set-Content -Path (Join-Path $gateRoot 'docs\workflow\current-stage-user-checklist.md') -Value '# 当前阶段用户待办清单`n`n- 先完成初始化落盘与 postcheck`n- 再进入实施线程'
        $sameContract = '# Sprint Contract 模板`n`n这是同一份模板。'
        Set-Content -Path (Join-Path $gateRoot 'docs\workflow\first-sprint-contract.md') -Value $sameContract
        Set-Content -Path (Join-Path $gateRoot 'docs\workflow\sprint-contract-template.md') -Value $sameContract

        $gate = & $truthGatePath -GeneratedRoot $gateRoot -TargetClient codex -AsJson | ConvertFrom-Json
        $issues = [string]($gate.Issues -join "`n")

        Assert-False -Condition ([bool]$gate.Passed) -Message 'Desktop package with empty selected_capabilities and stale public wording must fail truth-source gate.'
        Assert-Contains -Text $issues -ExpectedSubstring 'selected_capabilities' -Message 'Truth-source gate must require selected capabilities, not only required capabilities.'
        Assert-Contains -Text $issues -ExpectedSubstring 'selected_solution_id' -Message 'Truth-source gate must require selected solution audit consistency.'
        Assert-Contains -Text $issues -ExpectedSubstring 'capability_gate_passed' -Message 'Truth-source gate must require a green capability gate for desktop-agent packages.'
        Assert-Contains -Text $issues -ExpectedSubstring 'ASCII control character' -Message 'Truth-source gate must report illegal control characters.'
        Assert-Contains -Text $issues -ExpectedSubstring 'CommonHE 正式流程' -Message 'Truth-source gate must report old outward product wording.'
        Assert-Contains -Text $issues -ExpectedSubstring 'duplicate_heading_number' -Message 'Truth-source gate must report duplicate AGENTS section numbers.'
        Assert-Contains -Text $issues -ExpectedSubstring 'dangling_role_reference' -Message 'Truth-source gate must report role references that are not enabled.'
        Assert-Contains -Text $issues -ExpectedSubstring 'current-stage-user-checklist.md' -Message 'Truth-source gate must report stale implementation checklist items.'
        Assert-Contains -Text $issues -ExpectedSubstring 'first-sprint-contract.md' -Message 'Truth-source gate must report first sprint contract copied from generic template.'
    }

    Invoke-TestCase -Name 'truth-source gate fails semi-instantiated handoff template residue' -Body {
        $gateRoot = Join-Path $tempRoot 'truth-gate-semi-instantiated-handoff'
        New-MinimalTruthSourcePackage -Root $gateRoot -TargetClient 'codex'

        Set-Content -Path (Join-Path $gateRoot 'docs\workflow\current-stage-user-checklist.md') -Value @'
# 当前阶段用户待办清单

## 本轮已完成

- 初始化协作包已落盘

## 当前阶段实施清单

- 阅读 AGENTS.md 与 docs/project_context.md

## 若不处理会阻塞的点

- postcheck 未通过或能力门禁为红色时，不得宣布初始化成功，也不得进入业务实施。
'@

        Set-Content -Path (Join-Path $gateRoot 'docs\workflow\first-sprint-contract.md') -Value @'
# Sprint Contract 模板

> 本文件是任务级 Contract 脚手架。使用前请先复制或按需改写，不代表当前项目已经存在一个已签署的 live Contract。

## 元信息
- task_id: first-sprint
- implementer: architect
- evaluator: reviewer
- risk_gate: medium
- status: draft

## 需求摘要

- 当前项目第一轮目标：准备学生管理系统的实施合同、接口边界和验证计划。

## 验收标准

| # | 标准描述 | 验证方法 | 关联交付物 |
|---|---------|---------|-----------|
| 1 | 待填写 | 待填写 | 待填写 |
'@

        $gate = & $truthGatePath -GeneratedRoot $gateRoot -TargetClient codex -AsJson | ConvertFrom-Json
        $issues = [string]($gate.Issues -join "`n")

        Assert-False -Condition ([bool]$gate.Passed) -Message 'Semi-instantiated handoff docs with template residue must fail truth-source gate.'
        Assert-Contains -Text $issues -ExpectedSubstring 'generated.stale_handoff_checklist' -Message 'Truth-source gate must report postcheck residue in implementation checklist.'
        Assert-Contains -Text $issues -ExpectedSubstring 'generated.sprint_contract_instance' -Message 'Truth-source gate must report first sprint contract template residue.'
        Assert-Contains -Text $issues -ExpectedSubstring 'postcheck' -Message 'Truth-source gate issue must name the stale postcheck wording.'
        Assert-Contains -Text $issues -ExpectedSubstring 'Sprint Contract 模板' -Message 'Truth-source gate issue must name the template title residue.'
    }

    Invoke-TestCase -Name 'truth-source gate fails when final acceptance passes with blocking issues' -Body {
        $gateRoot = Join-Path $tempRoot 'truth-gate-final-acceptance-blockers'
        New-MinimalTruthSourcePackage -Root $gateRoot -TargetClient 'codex'

        $finalAcceptancePath = Join-Path $gateRoot '.commonhe\session\final-acceptance.json'
        $finalAcceptance = Get-Content -Raw $finalAcceptancePath | ConvertFrom-Json
        $finalAcceptance.blockingIssues = @('AGENTS.md 与 selectedSolution 明显矛盾，必须修复。')
        $finalAcceptance | ConvertTo-Json -Depth 8 | Set-Content -Path $finalAcceptancePath

        $gate = & $truthGatePath -GeneratedRoot $gateRoot -TargetClient codex -AsJson | ConvertFrom-Json
        $issues = [string]($gate.Issues -join "`n")

        Assert-False -Condition ([bool]$gate.Passed) -Message 'Generated package with passed final acceptance and blocking issues must fail truth-source gate.'
        Assert-Contains -Text $issues -ExpectedSubstring 'blockingIssues' -Message 'Truth-source gate must reject passed final acceptance with non-empty blockingIssues.'
    }

    Invoke-TestCase -Name 'repo truth-source gate validates source and test coverage' -Body {
        $gate = & $truthGatePath -RepoRoot $repoRoot -AsJson | ConvertFrom-Json

        Assert-True -Condition ([bool]$gate.Passed) -Message "Repository truth-source gate should pass: $($gate.Issues -join '; ')"
    }

    Invoke-TestCase -Name 'repo preserves generated package review contract' -Body {
        $contractPath = Join-Path $repoRoot 'specs\004-dual-agent-semantic-review\contracts\generated-package-review.md'
        Assert-True -Condition (Test-Path -LiteralPath $contractPath -PathType Leaf) -Message 'Generated package review contract must be documented.'

        $contract = Get-Content -LiteralPath $contractPath -Raw
        foreach ($marker in @(
            '星梦梦语义审查',
            'truth-source gate 硬门禁',
            'sanitizer',
            '重复编号',
            '未启用角色',
            '临时补救命令',
            'selected_capabilities',
            'first-sprint-contract.md',
            'sprint-contract-template.md'
        )) {
            Assert-Contains -Text $contract -ExpectedSubstring $marker -Message "Generated package review contract must preserve marker: $marker"
        }
    }

    Invoke-TestCase -Name 'orchestrator runtime paths should not use raw PSScriptRoot after scriptDir fallback' -Body {
        $orchestratorSource = Get-Content -Raw $orchestratorPath

        Assert-NotContains `
            -Text $orchestratorSource `
            -UnexpectedSubstring 'Join-Path $PSScriptRoot' `
            -Message 'Portable/runtime path joins must use $scriptDir after the Windows verbatim path fallback is resolved.'
    }

    Invoke-TestCase -Name 'orchestrator must not hardcode business-domain authored package content' -Body {
        $orchestratorSource = Get-Content -Raw $orchestratorPath

        foreach ($forbidden in @(
            'function Test-IsStudentManagementProject',
            '### 学生管理工作流',
            '活动发布/报名记录'
        )) {
            Assert-NotContains `
                -Text $orchestratorSource `
                -UnexpectedSubstring $forbidden `
                -Message "Orchestrator must not contain business-domain authoring shortcut: $forbidden"
        }
    }

    Invoke-TestCase -Name 'orchestrator works when invoked through Windows verbatim path' -Body {
        $projectRoot = Join-Path $tempRoot 'verbatim-path-empty'
        New-Item -ItemType Directory -Path $projectRoot -Force | Out-Null
        $verbatimOrchestratorPath = "\\?\$orchestratorPath"

        $result = & $verbatimOrchestratorPath -Stage start -ProjectRoot $projectRoot

        Assert-Equal -Actual $result.Stage -Expected 'discovery' -Message 'Verbatim path invocation should resolve sibling init script and start discovery.'
        Assert-Equal -Actual $result.QuestionId -Expected 'project_name' -Message 'Empty workspace should still enter first discovery question.'
    }

    Invoke-TestCase -Name 'doctor fails when chrome devtools capability is missing and surfaces install guidance' -Body {
        $projectRoot = New-LegacyProject -Name 'doctor-missing-chrome-devtools'
        $catalogPath = New-CapabilityCatalog -Root $projectRoot -IncludeChromeDevtools:$false
        $env:COMMONHE_REQUIRED_CAPABILITIES_PATH = $catalogPath

        try {
            $result = & $orchestratorPath -Stage doctor -ProjectRoot $projectRoot
            $status = Get-Content -Raw (Join-Path $projectRoot '.commonhe\session\status.json') | ConvertFrom-Json
            $doctor = Get-Content -Raw (Join-Path $projectRoot '.commonhe\session\doctor.json') | ConvertFrom-Json
            $doctorMarkdown = Get-Content -Raw (Join-Path $projectRoot '.commonhe\session\doctor.md')

            Assert-Equal -Actual $result.Stage -Expected 'doctor_failed' -Message 'Doctor should fail when chrome-devtools MCP is missing.'
            Assert-Equal -Actual $status.stage -Expected 'doctor_failed' -Message 'Status should record doctor_failed.'
            Assert-True -Condition ([bool]$status.doctor_failed) -Message 'Status should record doctor_failed=true.'
            Assert-False -Condition ([bool]$status.doctor_passed) -Message 'Status should keep doctor_passed=false on failure.'
            Assert-Contains -Text ([string]($doctor.missing_capabilities -join ' ')) -ExpectedSubstring 'chrome-devtools' -Message 'Doctor report should include chrome-devtools.'
            Assert-Contains -Text $doctorMarkdown -ExpectedSubstring 'codex mcp add chrome-devtools -- npx chrome-devtools-mcp@latest' -Message 'Doctor markdown should surface the official install command.'
            Assert-Contains -Text $doctorMarkdown -ExpectedSubstring '[mcp_servers.chrome-devtools]' -Message 'Doctor markdown should explain the MCP config probe.'
        } finally {
            $env:COMMONHE_REQUIRED_CAPABILITIES_PATH = $defaultCapabilityCatalogPath
        }
    }

    Invoke-TestCase -Name 'doctor blocks misplaced nested CommonHE package layouts before precheck' -Body {
        $projectRoot = Join-Path $tempRoot 'misplaced-package-root'
        $nestedPackageRoot = Join-Path $projectRoot 'CommonHE-v0.2.4'
        New-Item -ItemType Directory -Path $nestedPackageRoot -Force | Out-Null
        Set-Content -Path (Join-Path $nestedPackageRoot 'AGENTS.md') -Value '# CommonHE'
        New-Item -ItemType Directory -Path (Join-Path $nestedPackageRoot 'tools') -Force | Out-Null
        New-Item -ItemType Directory -Path (Join-Path $nestedPackageRoot 'templates') -Force | Out-Null
        Set-Content -Path (Join-Path $nestedPackageRoot 'README.md') -Value '# CommonHE'
        $catalogPath = New-CapabilityCatalog -Root (Join-Path $projectRoot 'capability-catalog')
        $env:COMMONHE_REQUIRED_CAPABILITIES_PATH = $catalogPath

        try {
            $result = & $orchestratorPath -Stage doctor -ProjectRoot $projectRoot
            $status = Get-Content -Raw (Join-Path $projectRoot '.commonhe\session\status.json') | ConvertFrom-Json
            $doctorMarkdown = Get-Content -Raw (Join-Path $projectRoot '.commonhe\session\doctor.md')

            Assert-Equal -Actual $result.Stage -Expected 'doctor_failed' -Message 'Doctor should block misplaced nested package layouts.'
            Assert-True -Condition ([bool]$status.entrycheck_failed) -Message 'Entry check should fail for nested package layouts.'
            Assert-True -Condition ([bool]$status.misplaced_package_detected) -Message 'Status should record misplaced package detection.'
            Assert-Contains -Text $doctorMarkdown -ExpectedSubstring 'release 的内容直接解压到目标项目根目录' -Message 'Doctor guidance should explain the correct extraction shape.'
            Assert-Contains -Text $doctorMarkdown -ExpectedSubstring 'CommonHE-v0.2.4' -Message 'Doctor guidance should mention the detected nested package directory.'
        } finally {
            $env:COMMONHE_REQUIRED_CAPABILITIES_PATH = $defaultCapabilityCatalogPath
        }
    }

    Invoke-TestCase -Name 'precheck returns precheck_failed instead of crashing when command capabilities are missing' -Body {
        $root = Join-Path $tempRoot 'missing-command-precheck'
        $projectRoot = Join-Path $root 'project'
        New-Item -ItemType Directory -Path $projectRoot -Force | Out-Null
        $catalogPath = Join-Path $root 'required-capabilities.json'
        @(
            @{
                name = 'GitNexus'
                display_name = 'GitNexus'
                verification_command = 'gitnexus --version'
                remediation = @('install GitNexus')
                probes = @(
                    @{
                        type = 'command_success'
                        commands = @('definitely-missing-gitnexus-probe --help')
                    }
                )
            }
            @{
                name = 'Speckit'
                display_name = 'Speckit'
                verification_command = 'specify --version'
                remediation = @('install Speckit')
                probes = @(
                    @{
                        type = 'command_success'
                        commands = @('definitely-missing-speckit-probe --help')
                    }
                )
            }
        ) | ConvertTo-Json -Depth 10 | Set-Content -Path $catalogPath

        $env:COMMONHE_REQUIRED_CAPABILITIES_PATH = $catalogPath

        try {
            $psi = [System.Diagnostics.ProcessStartInfo]::new()
            $psi.FileName = 'powershell'
            $psi.Arguments = "-NoProfile -ExecutionPolicy Bypass -File `"$orchestratorPath`" -Stage precheck -ProjectRoot `"$projectRoot`""
            $psi.WorkingDirectory = $projectRoot
            $psi.UseShellExecute = $false
            $psi.RedirectStandardOutput = $true
            $psi.RedirectStandardError = $true

            $process = [System.Diagnostics.Process]::Start($psi)
            $stdout = $process.StandardOutput.ReadToEnd()
            $stderr = $process.StandardError.ReadToEnd()
            $process.WaitForExit()

            Assert-Equal -Actual $process.ExitCode -Expected 0 -Message "Precheck should return a clean process exit even when command capabilities are missing. stderr: $stderr"
            Assert-Contains -Text $stdout -ExpectedSubstring 'precheck_failed' -Message "Precheck should report precheck_failed instead of crashing. stderr: $stderr"
            Assert-Contains -Text $stdout -ExpectedSubstring 'GitNexus' -Message 'Precheck should mention GitNexus in its result output.'
            Assert-Contains -Text $stdout -ExpectedSubstring 'Speckit' -Message 'Precheck should mention Speckit in its result output.'
        } finally {
            $env:COMMONHE_REQUIRED_CAPABILITIES_PATH = $defaultCapabilityCatalogPath
        }
    }

    Invoke-TestCase -Name 'legacy project start fails doctor before precheck when required capabilities are missing' -Body {
        $projectRoot = New-LegacyProject -Name 'legacy-missing-capabilities'
        $catalogPath = New-CapabilityCatalog -Root $projectRoot -IncludeSpeckit:$false
        $env:COMMONHE_REQUIRED_CAPABILITIES_PATH = $catalogPath

        try {
            $result = & $orchestratorPath -Stage start -ProjectRoot $projectRoot
            $status = Get-Content -Raw (Join-Path $projectRoot '.commonhe\session\status.json') | ConvertFrom-Json
            $doctor = Get-Content -Raw (Join-Path $projectRoot '.commonhe\session\doctor.json') | ConvertFrom-Json
            $doctorMarkdown = Get-Content -Raw (Join-Path $projectRoot '.commonhe\session\doctor.md')

            Assert-Equal -Actual $result.Stage -Expected 'doctor_failed' -Message 'Legacy project init should stop at doctor_failed when a required capability is missing.'
            Assert-Equal -Actual $status.stage -Expected 'doctor_failed' -Message 'Status should record doctor failure.'
            Assert-True -Condition ([bool]$status.doctor_failed) -Message 'Status should record doctor_failed=true.'
            Assert-Contains -Text ([string]($status.missing_capabilities -join ' ')) -ExpectedSubstring 'Speckit' -Message 'Missing capabilities should include Speckit.'
            Assert-Contains -Text ([string]($doctor.missing_capabilities -join ' ')) -ExpectedSubstring 'Speckit' -Message 'Doctor report should include Speckit.'
            Assert-Contains -Text $doctorMarkdown -ExpectedSubstring 'Speckit' -Message 'Doctor markdown should surface the missing capability.'
            Assert-Contains -Text $doctorMarkdown -ExpectedSubstring 'specify --version' -Message 'Doctor markdown should include the capability verification command.'
            Assert-Contains -Text ([string]$doctor.recommended_fix) -ExpectedSubstring 'install Speckit' -Message 'Doctor recommended fix should include the remediation hint.'
            Assert-False -Condition (Test-Path (Join-Path $projectRoot 'AGENTS.md')) -Message 'Failed precheck must not generate project AGENTS.md.'
            Assert-False -Condition (Test-Path (Join-Path $projectRoot 'docs')) -Message 'Failed precheck must not generate docs truth sources.'
        } finally {
            $env:COMMONHE_REQUIRED_CAPABILITIES_PATH = $defaultCapabilityCatalogPath
        }
    }

    Invoke-TestCase -Name 'legacy project start auto bootstraps after passing precheck' -Body {
        $projectRoot = New-LegacyProject -Name 'legacy-autobootstrap' -WithTests
        $catalogPath = New-CapabilityCatalog -Root $projectRoot
        $env:COMMONHE_REQUIRED_CAPABILITIES_PATH = $catalogPath

        try {
            $result = & $orchestratorPath -Stage start -ProjectRoot $projectRoot
            $status = Get-Content -Raw (Join-Path $projectRoot '.commonhe\session\status.json') | ConvertFrom-Json
            $decision = Get-Content -Raw (Join-Path $projectRoot '.commonhe\session\decision.json') | ConvertFrom-Json
            $rootAgents = Get-Content -Raw (Join-Path $projectRoot 'AGENTS.md')
            $coordinatorDoc = Get-Content -Raw (Join-Path $projectRoot '.codex\COORDINATOR-SUBAGENTS.md')
            $projectContext = Get-Content -Raw (Join-Path $projectRoot 'docs\project_context.md')
            $indexDoc = Get-Content -Raw (Join-Path $projectRoot 'docs\00-初始化结果索引.md')
            $capabilityDoc = Get-Content -Raw (Join-Path $projectRoot 'docs\skills\required-capabilities.md')
            $agentDoc = Get-Content -Raw (Join-Path $projectRoot '.codex\agents\engineering-frontend-developer.md')
            $reviewerHandbook = Get-Content -Raw (Join-Path $projectRoot 'docs\agents\reviewer-handbook.md')
            $checklistDoc = Get-Content -Raw (Join-Path $projectRoot 'docs\workflow\current-stage-user-checklist.md')
            $kickoffDoc = Get-Content -Raw (Join-Path $projectRoot 'docs\workflow\implementation-kickoff.md')
            $firstTaskPack = Get-Content -Raw (Join-Path $projectRoot 'docs\workflow\first-task-pack.md')
            $firstSprintContract = Get-Content -Raw (Join-Path $projectRoot 'docs\workflow\first-sprint-contract.md')

            Assert-Equal -Actual $result.Stage -Expected 'implementation_ready' -Message 'Legacy project start should complete end-to-end after precheck passes.'
            Assert-Equal -Actual $status.stage -Expected 'implementation_ready' -Message 'Status should promote legacy auto init into implementation_ready.'
            Assert-True -Condition ([bool]$status.doctor_passed) -Message 'Status should record doctor_passed=true.'
            Assert-True -Condition ([bool]$status.precheck_passed) -Message 'Status should record precheck_passed=true.'
            Assert-True -Condition ([bool]$status.legacy_project_detected) -Message 'Status should record legacy project detection.'
            Assert-True -Condition ([bool]$status.legacy_zero_question_mode) -Message 'Status should record zero-question legacy mode.'
            Assert-Equal -Actual $decision.discovery_mode -Expected 'legacy_zero_question' -Message 'Decision should record legacy_zero_question discovery mode.'
            Assert-Equal -Actual $decision.legacy_analysis_version -Expected 'v2' -Message 'Decision should record legacy analysis version v2.'
            Assert-True -Condition ([bool]$decision.auto_confirmed) -Message 'Decision should record auto_confirmed=true.'
            Assert-Equal -Actual $decision.confirmation_mode -Expected 'auto_legacy_init' -Message 'Decision should record auto_legacy_init confirmation mode.'
            Assert-True -Condition (@($decision.required_capabilities).Count -ge 5) -Message 'Decision should persist all required capabilities.'
            Assert-Contains -Text ([string]($decision.required_capabilities | ConvertTo-Json -Depth 10)) -ExpectedSubstring 'chrome-devtools' -Message 'Decision should include chrome-devtools capability.'
            Assert-Contains -Text ([string]($decision.signal_categories | ConvertTo-Json -Depth 10)) -ExpectedSubstring 'tests' -Message 'Decision should persist categorized legacy analysis signals.'
            Assert-Contains -Text ([string]($decision.role_rationale | ConvertTo-Json -Depth 10)) -ExpectedSubstring 'qa' -Message 'Decision should record role rationale for qa when tests exist.'
            Assert-True -Condition (Test-Path (Join-Path $projectRoot '.agents\skills\required-capabilities.md')) -Message 'Legacy auto init should generate Codex runtime skill manifest under .agents.'
            Assert-True -Condition (Test-Path (Join-Path $projectRoot 'docs\workflow\evaluator-protocol.md')) -Message 'Legacy auto init should generate evaluator protocol into project workflow docs.'
            Assert-True -Condition (Test-Path (Join-Path $projectRoot 'docs\workflow\grading-criteria.md')) -Message 'Legacy auto init should generate grading criteria into project workflow docs.'
            Assert-True -Condition (Test-Path (Join-Path $projectRoot 'docs\workflow\sprint-contract-template.md')) -Message 'Legacy auto init should generate sprint contract template into project workflow docs.'
            Assert-True -Condition (Test-Path (Join-Path $projectRoot 'docs\workflow\implementation-kickoff.md')) -Message 'Legacy auto init should generate implementation kickoff doc.'
            Assert-True -Condition (Test-Path (Join-Path $projectRoot 'docs\workflow\first-sprint-contract.md')) -Message 'Legacy auto init should generate first sprint contract doc.'
            Assert-True -Condition (Test-Path (Join-Path $projectRoot 'docs\workflow\first-task-pack.md')) -Message 'Legacy auto init should generate first task pack doc.'
            Assert-Contains -Text $capabilityDoc -ExpectedSubstring 'GitNexus' -Message 'Capability manifest should document GitNexus.'
            Assert-Contains -Text $capabilityDoc -ExpectedSubstring 'chrome-devtools' -Message 'Capability manifest should document chrome-devtools.'
            Assert-Contains -Text $agentDoc -ExpectedSubstring 'Speckit' -Message 'Generated agent should declare required capabilities.'
            Assert-Contains -Text $agentDoc -ExpectedSubstring 'chrome-devtools' -Message 'Generated agent should declare chrome-devtools capability.'
            Assert-Contains -Text $projectContext -ExpectedSubstring '自动分析假设' -Message 'Project context should include autodiscovery assumptions section.'
            Assert-NotContains -Text $checklistDoc -UnexpectedSubstring '当前初始化线程只负责补齐协作工程' -Message 'Implementation checklist must not leak init-only execution rules.'
            Assert-NotContains -Text $checklistDoc -UnexpectedSubstring '初始化收口清单' -Message 'Implementation checklist must not include init closure sections.'
            Assert-NotContains -Text $checklistDoc -UnexpectedSubstring 'postcheck' -Message 'Implementation checklist must not leave postcheck as a handoff task.'
            Assert-NotContains -Text $checklistDoc -UnexpectedSubstring 'bootstrap' -Message 'Implementation checklist must not leave bootstrap as a handoff task.'
            Assert-Contains -Text $indexDoc -ExpectedSubstring 'docs/workflow/current-stage-user-checklist.md' -Message 'Init result index should still point new threads to the implementation checklist.'
            Assert-Contains -Text $indexDoc -ExpectedSubstring 'docs/workflow/evaluator-protocol.md' -Message 'Init result index should expose advanced workflow docs.'
            Assert-Contains -Text $indexDoc -ExpectedSubstring 'docs/workflow/implementation-kickoff.md' -Message 'Init result index should expose implementation kickoff docs.'
            Assert-Contains -Text $rootAgents -ExpectedSubstring 'docs/workflow/evaluator-protocol.md' -Message 'Project AGENTS should reference the generated evaluator protocol.'
            Assert-Contains -Text $rootAgents -ExpectedSubstring 'docs/workflow/grading-criteria.md' -Message 'Project AGENTS should reference the generated grading criteria.'
            Assert-Contains -Text $rootAgents -ExpectedSubstring 'docs/workflow/sprint-contract-template.md' -Message 'Project AGENTS should reference the generated sprint contract template.'
            Assert-Contains -Text $rootAgents -ExpectedSubstring 'docs/workflow/implementation-kickoff.md' -Message 'Project AGENTS should point new threads to the kickoff docs.'
            Assert-Contains -Text $coordinatorDoc -ExpectedSubstring 'docs/workflow/evaluator-protocol.md' -Message 'Coordinator doc should reference the generated evaluator protocol.'
            Assert-Contains -Text $coordinatorDoc -ExpectedSubstring 'docs/workflow/grading-criteria.md' -Message 'Coordinator doc should reference the generated grading criteria.'
            Assert-Contains -Text $coordinatorDoc -ExpectedSubstring 'docs/workflow/sprint-contract-template.md' -Message 'Coordinator doc should reference the generated sprint contract template.'
            Assert-Contains -Text $coordinatorDoc -ExpectedSubstring 'docs/workflow/implementation-kickoff.md' -Message 'Coordinator doc should reference kickoff docs.'
            Assert-Contains -Text $reviewerHandbook -ExpectedSubstring 'docs/workflow/evaluator-protocol.md' -Message 'Reviewer handbook should reference the generated evaluator protocol.'
            Assert-Contains -Text $reviewerHandbook -ExpectedSubstring 'docs/workflow/grading-criteria.md' -Message 'Reviewer handbook should reference the generated grading criteria.'
            Assert-Contains -Text $reviewerHandbook -ExpectedSubstring 'docs/workflow/sprint-contract-template.md' -Message 'Reviewer handbook should reference the generated sprint contract template.'
            Assert-Contains -Text $kickoffDoc -ExpectedSubstring '第一轮实施' -Message 'Kickoff doc should orient the first implementation thread.'
            Assert-Contains -Text $firstTaskPack -ExpectedSubstring 'owner_role' -Message 'Task pack should assign owners for the suggested first tasks.'
            Assert-Contains -Text $firstSprintContract -ExpectedSubstring 'risk_gate' -Message 'First sprint contract should include a risk gate.'
        } finally {
            $env:COMMONHE_REQUIRED_CAPABILITIES_PATH = $defaultCapabilityCatalogPath
        }
    }

    Invoke-TestCase -Name 'legacy analysis v2 should keep frontend-only projects lightweight when tests are absent' -Body {
        $projectRoot = New-LegacyProject -Name 'legacy-frontend-only'
        $catalogPath = New-CapabilityCatalog -Root $projectRoot
        $env:COMMONHE_REQUIRED_CAPABILITIES_PATH = $catalogPath

        try {
            & $orchestratorPath -Stage start -ProjectRoot $projectRoot | Out-Null
            $decision = Get-Content -Raw (Join-Path $projectRoot '.commonhe\session\decision.json') | ConvertFrom-Json

            Assert-Equal -Actual $decision.legacy_analysis_version -Expected 'v2' -Message 'Legacy analysis should persist v2 marker.'
            Assert-Contains -Text ([string]($decision.recommended_roles_now -join ' ')) -ExpectedSubstring 'frontend' -Message 'Frontend-only projects should enable frontend immediately.'
            Assert-Contains -Text ([string]($decision.recommended_roles_now -join ' ')) -ExpectedSubstring 'reviewer' -Message 'Frontend-only projects should keep reviewer immediately.'
            Assert-Contains -Text ([string]($decision.recommended_roles_now -join ' ')) -ExpectedSubstring 'docs' -Message 'Frontend-only projects should keep docs immediately.'
            Assert-False -Condition ([bool](($decision.recommended_roles_now -join ' ') -like '*qa*')) -Message 'Frontend-only projects without test signals should not enable qa immediately.'
            Assert-Contains -Text ([string]($decision.role_rationale | ConvertTo-Json -Depth 10)) -ExpectedSubstring 'frontend' -Message 'Role rationale should explain why frontend is enabled.'
        } finally {
            $env:COMMONHE_REQUIRED_CAPABILITIES_PATH = $defaultCapabilityCatalogPath
        }
    }

    Invoke-TestCase -Name 'legacy analysis v2 should promote architect database and devops from stronger backend signals' -Body {
        $projectRoot = New-LegacyProject -Name 'legacy-complex-platform' -WithTests -WithBackend -WithDatabase -WithDeploy
        $catalogPath = New-CapabilityCatalog -Root $projectRoot
        $env:COMMONHE_REQUIRED_CAPABILITIES_PATH = $catalogPath

        try {
            & $orchestratorPath -Stage start -ProjectRoot $projectRoot | Out-Null
            $decision = Get-Content -Raw (Join-Path $projectRoot '.commonhe\session\decision.json') | ConvertFrom-Json

            Assert-Contains -Text ([string]($decision.recommended_roles_now -join ' ')) -ExpectedSubstring 'architect' -Message 'Mixed front/back projects should enable architect in legacy analysis v2.'
            Assert-Contains -Text ([string]($decision.recommended_roles_now -join ' ')) -ExpectedSubstring 'backend' -Message 'Backend signals should enable backend immediately.'
            Assert-Contains -Text ([string]($decision.recommended_roles_now -join ' ')) -ExpectedSubstring 'database' -Message 'Database signals should enable database immediately.'
            Assert-Contains -Text ([string]($decision.available_roles_later -join ' ')) -ExpectedSubstring 'devops' -Message 'Deploy signals should place devops into available_roles_later when not immediately required.'
            Assert-Contains -Text ([string]($decision.dominant_workstreams | ConvertTo-Json -Depth 10)) -ExpectedSubstring 'backend' -Message 'Dominant workstreams should include backend for mixed projects.'
        } finally {
            $env:COMMONHE_REQUIRED_CAPABILITIES_PATH = $defaultCapabilityCatalogPath
        }
    }

    Invoke-TestCase -Name 'legacy project generation should not reference disabled qa role' -Body {
        $projectRoot = Join-Path $tempRoot 'legacy-no-disabled-role-references'
        New-Item -ItemType Directory -Path $projectRoot -Force | Out-Null
        New-Item -ItemType Directory -Path (Join-Path $projectRoot 'src') -Force | Out-Null
        New-Item -ItemType Directory -Path (Join-Path $projectRoot '.git') -Force | Out-Null
        Set-Content -Path (Join-Path $projectRoot 'package.json') -Value '{ "name": "legacy-demo" }'
        Set-Content -Path (Join-Path $projectRoot 'README.md') -Value '# Legacy Demo'
        Set-Content -Path (Join-Path $projectRoot 'src\index.ts') -Value 'export const demo = true;'
        $catalogPath = New-CapabilityCatalog -Root $projectRoot
        $env:COMMONHE_REQUIRED_CAPABILITIES_PATH = $catalogPath

        try {
            $result = & $orchestratorPath -Stage start -ProjectRoot $projectRoot
            Assert-Equal -Actual $result.Stage -Expected 'implementation_ready' -Message 'Legacy project should still initialize successfully before reference checks.'
            $decision = Get-Content -Raw (Join-Path $projectRoot '.commonhe\session\decision.json') | ConvertFrom-Json

            $generatedDocs = @(
                Get-Content -Raw (Join-Path $projectRoot '.codex\COORDINATOR-SUBAGENTS.md')
                Get-Content -Raw (Join-Path $projectRoot '.codex\agents\engineering-frontend-developer.md')
                Get-Content -Raw (Join-Path $projectRoot 'docs\agents\frontend-handbook.md')
                Get-Content -Raw (Join-Path $projectRoot 'docs\project_context.md')
                Get-Content -Raw (Join-Path $projectRoot 'docs\roadmap\01-实施路线图.md')
                Get-Content -Raw (Join-Path $projectRoot 'docs\workflow\acceptance-gates.md')
                Get-Content -Raw (Join-Path $projectRoot 'docs\workflow\current-stage-user-checklist.md')
            ) -join "`n"

            Assert-Equal -Actual ((@($decision.enabled_roles) -join ',')) -Expected 'docs,frontend,reviewer' -Message 'Frontend-only legacy projects should not enable qa.'
            Assert-NotContains -Text $generatedDocs -UnexpectedSubstring '@qa' -Message 'Generated docs must not reference @qa when qa is not an enabled role.'
            Assert-NotContains -Text $generatedDocs -UnexpectedSubstring 'qa 完成' -Message 'Generated docs must not require qa completion when qa is not an enabled role.'
            Assert-NotContains -Text $generatedDocs -UnexpectedSubstring '测试 -> qa' -Message 'Coordinator dispatch triggers must not mention qa when qa is not enabled.'
            Assert-NotContains -Text $generatedDocs -UnexpectedSubstring 'implementation / review / qa' -Message 'Implementation checklist must not include qa when qa is not enabled.'
        } finally {
            $env:COMMONHE_REQUIRED_CAPABILITIES_PATH = $defaultCapabilityCatalogPath
        }
    }

    Invoke-TestCase -Name 'legacy project postcheck fails when capability manifest is removed' -Body {
        $projectRoot = New-LegacyProject -Name 'legacy-missing-capability-doc'
        $catalogPath = New-CapabilityCatalog -Root $projectRoot
        $env:COMMONHE_REQUIRED_CAPABILITIES_PATH = $catalogPath

        try {
            $startResult = & $orchestratorPath -Stage start -ProjectRoot $projectRoot
            Assert-Equal -Actual $startResult.Stage -Expected 'implementation_ready' -Message 'Legacy project should start in implementation_ready before postcheck damage.'

            Remove-Item -LiteralPath (Join-Path $projectRoot 'docs\skills\required-capabilities.md') -Force

            $result = & $orchestratorPath -Stage postcheck -ProjectRoot $projectRoot -TargetRoot $projectRoot
            Assert-Equal -Actual $result.Stage -Expected 'postcheck_failed' -Message 'Missing capability manifest should fail postcheck.'
            Assert-Contains -Text ([string]($result.Postcheck.MissingCoreFiles -join ' ')) -ExpectedSubstring 'docs/skills/required-capabilities.md' -Message 'Postcheck should report the missing capability manifest.'
        } finally {
            $env:COMMONHE_REQUIRED_CAPABILITIES_PATH = $defaultCapabilityCatalogPath
        }
    }

    Invoke-TestCase -Name 'legacy project postcheck fails when evaluator protocol is removed' -Body {
        $projectRoot = New-LegacyProject -Name 'legacy-missing-workflow-doc'
        $catalogPath = New-CapabilityCatalog -Root $projectRoot
        $env:COMMONHE_REQUIRED_CAPABILITIES_PATH = $catalogPath

        try {
            $startResult = & $orchestratorPath -Stage start -ProjectRoot $projectRoot
            Assert-Equal -Actual $startResult.Stage -Expected 'implementation_ready' -Message 'Legacy project should start in implementation_ready before workflow-doc damage.'

            Remove-Item -LiteralPath (Join-Path $projectRoot 'docs\workflow\evaluator-protocol.md') -Force

            $result = & $orchestratorPath -Stage postcheck -ProjectRoot $projectRoot -TargetRoot $projectRoot
            Assert-Equal -Actual $result.Stage -Expected 'postcheck_failed' -Message 'Missing evaluator protocol should fail postcheck.'
            Assert-Contains -Text ([string]($result.Postcheck.MissingCoreFiles -join ' ')) -ExpectedSubstring 'docs/workflow/evaluator-protocol.md' -Message 'Postcheck should report the missing evaluator protocol.'
        } finally {
            $env:COMMONHE_REQUIRED_CAPABILITIES_PATH = $defaultCapabilityCatalogPath
        }
    }

    Invoke-TestCase -Name 'legacy project postcheck fails when docs reference a disabled role' -Body {
        $projectRoot = Join-Path $tempRoot 'legacy-disabled-role-postcheck'
        New-Item -ItemType Directory -Path $projectRoot -Force | Out-Null
        New-Item -ItemType Directory -Path (Join-Path $projectRoot 'src') -Force | Out-Null
        New-Item -ItemType Directory -Path (Join-Path $projectRoot '.git') -Force | Out-Null
        Set-Content -Path (Join-Path $projectRoot 'package.json') -Value '{ "name": "legacy-disabled-role-postcheck" }'
        Set-Content -Path (Join-Path $projectRoot 'README.md') -Value '# Legacy Disabled Role Postcheck'
        Set-Content -Path (Join-Path $projectRoot 'src\index.ts') -Value 'export const demo = true;'
        $catalogPath = New-CapabilityCatalog -Root $projectRoot
        $env:COMMONHE_REQUIRED_CAPABILITIES_PATH = $catalogPath

        try {
            $startResult = & $orchestratorPath -Stage start -ProjectRoot $projectRoot
            Assert-Equal -Actual $startResult.Stage -Expected 'implementation_ready' -Message 'Legacy project should initialize before postcheck damage.'

            Add-Content -Path (Join-Path $projectRoot 'docs\workflow\acceptance-gates.md') -Value "`n- qa 已完成"

            $result = & $orchestratorPath -Stage postcheck -ProjectRoot $projectRoot -TargetRoot $projectRoot
            Assert-Equal -Actual $result.Stage -Expected 'postcheck_failed' -Message 'Postcheck should fail when generated docs reference a disabled role.'
            Assert-Contains -Text ([string]($result.Postcheck.DanglingRoleReferences -join ' ')) -ExpectedSubstring 'qa' -Message 'Postcheck should report the disabled role reference.'
        } finally {
            $env:COMMONHE_REQUIRED_CAPABILITIES_PATH = $defaultCapabilityCatalogPath
        }
    }

    Invoke-TestCase -Name 'legacy project postcheck fails when implementation checklist regains init-only content' -Body {
        $projectRoot = New-LegacyProject -Name 'legacy-init-only-checklist-postcheck'
        $catalogPath = New-CapabilityCatalog -Root $projectRoot
        $env:COMMONHE_REQUIRED_CAPABILITIES_PATH = $catalogPath

        try {
            $startResult = & $orchestratorPath -Stage start -ProjectRoot $projectRoot
            Assert-Equal -Actual $startResult.Stage -Expected 'implementation_ready' -Message 'Legacy project should initialize before checklist damage.'

            Add-Content -Path (Join-Path $projectRoot 'docs\workflow\current-stage-user-checklist.md') -Value "`n- 当前初始化线程只负责补齐协作工程，不直接展开业务实现。"

            $result = & $orchestratorPath -Stage postcheck -ProjectRoot $projectRoot -TargetRoot $projectRoot
            Assert-Equal -Actual $result.Stage -Expected 'postcheck_failed' -Message 'Postcheck should fail when implementation checklist leaks init-only content.'
            Assert-Contains -Text ([string]($result.Postcheck.InvalidWorkflowContent -join ' ')) -ExpectedSubstring 'current-stage-user-checklist.md' -Message 'Postcheck should report invalid implementation-checklist content.'
        } finally {
            $env:COMMONHE_REQUIRED_CAPABILITIES_PATH = $defaultCapabilityCatalogPath
        }
    }

    Invoke-TestCase -Name 'legacy project postcheck fails when agent capability section is removed' -Body {
        $projectRoot = New-LegacyProject -Name 'legacy-missing-agent-capabilities'
        $catalogPath = New-CapabilityCatalog -Root $projectRoot
        $env:COMMONHE_REQUIRED_CAPABILITIES_PATH = $catalogPath

        try {
            $startResult = & $orchestratorPath -Stage start -ProjectRoot $projectRoot
            Assert-Equal -Actual $startResult.Stage -Expected 'implementation_ready' -Message 'Legacy project should start in implementation_ready before agent damage.'

            Set-Content -Path (Join-Path $projectRoot '.codex\agents\engineering-frontend-developer.md') -Value '# broken agent'

            $result = & $orchestratorPath -Stage postcheck -ProjectRoot $projectRoot -TargetRoot $projectRoot
            Assert-Equal -Actual $result.Stage -Expected 'postcheck_failed' -Message 'Removing agent capability requirements should fail postcheck.'
            Assert-Contains -Text ([string]($result.Postcheck.MissingCapabilityDeclarations -join ' ')) -ExpectedSubstring '.codex/agents/engineering-frontend-developer.md' -Message 'Postcheck should report agent files missing capability declarations.'
        } finally {
            $env:COMMONHE_REQUIRED_CAPABILITIES_PATH = $defaultCapabilityCatalogPath
        }
    }

    Invoke-TestCase -Name 'empty directory start remains in discovery mode' -Body {
        $root = Join-Path $tempRoot 'empty-start-compat'
        New-Item -ItemType Directory -Path $root -Force | Out-Null

        $result = & $orchestratorPath -Stage start -ProjectRoot $root
        Assert-Equal -Actual $result.Stage -Expected 'discovery' -Message 'Empty directories should retain compatibility discovery flow.'
        Assert-Equal -Actual $result.QuestionSource -Expected 'fallback template' -Message 'Discovery should report whether questions came from the fallback template or LLM generation.'
    }

    Invoke-TestCase -Name 'proposal returns full option data for desktop display' -Body {
        $root = Join-Path $tempRoot 'proposal-options-desktop'
        New-Item -ItemType Directory -Path $root -Force | Out-Null
        $projectRoot = [System.IO.Path]::GetFullPath($root)

        & $orchestratorPath -Stage start -ProjectRoot $projectRoot | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '学生管理系统' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '做一个学生管理系统，解决学生信息、班级、成绩、活动的统一管理' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '学校教务老师和班主任' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '学生信息管理、班级管理、成绩录入、成绩查询、活动发布、报名记录' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '优先稳定和可验证，后续可扩展' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '从零开始，没有必须对接的平台' | Out-Null

        $result = & $orchestratorPath -Stage propose -ProjectRoot $projectRoot

        Assert-Equal -Actual $result.Stage -Expected 'proposal' -Message 'Propose should return the proposal stage.'
        Assert-Equal -Actual @($result.Options).Count -Expected 3 -Message 'Desktop needs all proposal options for card display.'
        Assert-Equal -Actual $result.Recommended -Expected 'B' -Message 'Balanced should remain the default recommendation without strong constraints.'
        Assert-Equal -Actual $result.RecommendedOption.id -Expected 'B' -Message 'RecommendedOption should be the full option object, not only the option id.'
        Assert-Contains -Text ([string]$result.ProposalPath) -ExpectedSubstring 'proposal.md' -Message 'Desktop should receive the generated proposal path.'
        Assert-Contains -Text ([string]($result.Options | ConvertTo-Json -Depth 8)) -ExpectedSubstring '学生信息管理' -Message 'Proposal should preserve user-provided feature terms instead of hardcoded domain wording.'
    }

    Invoke-TestCase -Name 'package-only extracted root remains in discovery mode' -Body {
        $root = Join-Path $tempRoot 'package-only-root'
        New-Item -ItemType Directory -Path $root -Force | Out-Null
        foreach ($dirName in @('config', 'core', 'examples', 'init', 'templates', 'tools')) {
            New-Item -ItemType Directory -Path (Join-Path $root $dirName) -Force | Out-Null
        }
        Set-Content -Path (Join-Path $root 'AGENTS.md') -Value '# CommonHE Init Entry'
        Set-Content -Path (Join-Path $root 'MANUAL-TEST.md') -Value '# Manual Test'
        Set-Content -Path (Join-Path $root 'README.md') -Value @'
# CommonHE

输入 `初始化 CommonHE`

当前只生成 HE 协作工程
'@
        Set-Content -Path (Join-Path $root '用户使用手册.md') -Value '# 用户使用手册'

        $result = & $orchestratorPath -Stage start -ProjectRoot $root
        Assert-Equal -Actual $result.Stage -Expected 'discovery' -Message 'Package-only roots should stay in compatibility discovery flow.'
    }

    Invoke-TestCase -Name 'bootstrap execute completes and closes init context after successful postcheck' -Body {
        $envData = New-TestEnvironment -Name 'happy-path' -EnabledRoles @('frontend', 'reviewer', 'docs')

        $result = & $orchestratorPath `
            -Stage bootstrap `
            -SessionRoot $envData.SessionRoot `
            -TargetRoot $envData.Root `
            -ValuesPath $envData.ValuesPath `
            -Execute `
            -Force

        $status = Get-Content -Raw (Join-Path $envData.SessionRoot 'status.json') | ConvertFrom-Json
        $handoffPath = Join-Path $envData.SessionRoot 'bootstrap-handoff.md'
        $handoffContent = Get-Content -Raw $handoffPath
        $checklistDoc = Get-Content -Raw (Join-Path $envData.Root 'docs\workflow\current-stage-user-checklist.md')

        Assert-Equal -Actual $result.Stage -Expected 'implementation_ready' -Message 'Bootstrap should promote the project into implementation_ready after a successful postcheck.'
        Assert-True -Condition ([bool]$result.Postcheck.Passed) -Message 'Postcheck should pass for a clean generated project.'
        Assert-True -Condition ([bool]$result.Postcheck.TruthSourceGatePassed) -Message 'Postcheck should include a passing truth-source gate for a clean generated project.'
        Assert-Equal -Actual $status.stage -Expected 'implementation_ready' -Message 'Session status should switch into implementation_ready after a successful postcheck.'
        Assert-True -Condition ([bool]$status.last_postcheck_passed) -Message 'Status should record the last postcheck as passing.'
        Assert-True -Condition ([bool]$status.truth_source_gate_passed) -Message 'Status should record truth-source gate pass.'
        Assert-False -Condition ([bool]$status.truth_source_gate_failed) -Message 'Status should record truth-source gate not failed.'
        Assert-True -Condition ([bool]$status.init_closed) -Message 'Status should mark the init context as closed after success.'
        Assert-True -Condition ([bool]$status.implementation_stage_promoted) -Message 'Status should record that the implementation stage has been promoted.'
        Assert-Contains -Text ([string]$result.Message) -ExpectedSubstring '初始化协作包已生成并通过 postcheck' -Message 'Bootstrap success message should declare init package closure.'
        Assert-Contains -Text ([string]$result.Message) -ExpectedSubstring '当前初始化线程到此收口' -Message 'Bootstrap success message should close the init context.'
        Assert-Contains -Text ([string]$result.Message) -ExpectedSubstring 'Codex' -Message 'Bootstrap success message should name the selected target client.'
        Assert-True -Condition (Test-Path $handoffPath) -Message 'Bootstrap should emit a session handoff document.'
        Assert-True -Condition (Test-Path (Join-Path $envData.Root 'docs\workflow\evaluator-protocol.md')) -Message 'Bootstrap should generate evaluator protocol docs.'
        Assert-True -Condition (Test-Path (Join-Path $envData.Root 'docs\workflow\grading-criteria.md')) -Message 'Bootstrap should generate grading criteria docs.'
        Assert-True -Condition (Test-Path (Join-Path $envData.Root 'docs\workflow\sprint-contract-template.md')) -Message 'Bootstrap should generate sprint contract template docs.'
        Assert-True -Condition (Test-Path (Join-Path $envData.Root 'docs\workflow\implementation-kickoff.md')) -Message 'Bootstrap should generate implementation kickoff docs.'
        Assert-True -Condition (Test-Path (Join-Path $envData.Root 'docs\workflow\first-sprint-contract.md')) -Message 'Bootstrap should generate first sprint contract docs.'
        Assert-True -Condition (Test-Path (Join-Path $envData.Root 'docs\workflow\first-task-pack.md')) -Message 'Bootstrap should generate first task pack docs.'
        Assert-NotContains -Text $checklistDoc -UnexpectedSubstring '初始化收口清单' -Message 'Implementation checklist should remain implementation-only after bootstrap.'
        Assert-Contains -Text $handoffContent -ExpectedSubstring 'postcheck：通过' -Message 'Bootstrap handoff should include the postcheck result.'
    }

    Invoke-TestCase -Name 'bootstrap execute fails postcheck when unexpected team files remain in target root' -Body {
        $envData = New-TestEnvironment -Name 'unexpected-team' -EnabledRoles @('frontend', 'reviewer', 'docs')
        $unexpectedAgentDir = Join-Path $envData.Root '.codex\agents'
        $unexpectedHandbookDir = Join-Path $envData.Root 'docs\agents'
        New-Item -ItemType Directory -Path $unexpectedAgentDir -Force | Out-Null
        New-Item -ItemType Directory -Path $unexpectedHandbookDir -Force | Out-Null
        Set-Content -Path (Join-Path $unexpectedAgentDir 'unexpected.md') -Value '# unexpected'
        Set-Content -Path (Join-Path $unexpectedHandbookDir 'unexpected-handbook.md') -Value '# unexpected'

        $result = & $orchestratorPath `
            -Stage bootstrap `
            -SessionRoot $envData.SessionRoot `
            -TargetRoot $envData.Root `
            -ValuesPath $envData.ValuesPath `
            -Execute `
            -Force

        $status = Get-Content -Raw (Join-Path $envData.SessionRoot 'status.json') | ConvertFrom-Json

        Assert-Equal -Actual $result.Stage -Expected 'postcheck_failed' -Message 'Unexpected team files should block init success.'
        Assert-False -Condition ([bool]$result.Postcheck.Passed) -Message 'Postcheck should fail when unexpected role files remain.'
        Assert-Contains -Text ([string]($result.Postcheck.UnexpectedAgentFiles -join ' ')) -ExpectedSubstring '.codex/agents/unexpected.md' -Message 'Unexpected agent file should be reported.'
        Assert-Contains -Text ([string]($result.Postcheck.UnexpectedHandbooks -join ' ')) -ExpectedSubstring 'docs/agents/unexpected-handbook.md' -Message 'Unexpected handbook file should be reported.'
        Assert-Equal -Actual $status.stage -Expected 'postcheck_failed' -Message 'Session status should record postcheck failure.'
        Assert-False -Condition ([bool]$status.init_closed) -Message 'Init context must remain open after a failed postcheck.'
        Assert-Contains -Text ([string]$result.Message) -ExpectedSubstring '不得宣布初始化成功' -Message 'Failure message should block success wording.'
    }

    Invoke-TestCase -Name 'bootstrap rerun removes obsolete managed role files before postcheck' -Body {
        $envData = New-TestEnvironment -Name 'rerun-removes-obsolete-roles' -EnabledRoles @('architect', 'backend', 'frontend', 'miniapp', 'reviewer', 'qa', 'docs')

        $first = & $orchestratorPath `
            -Stage bootstrap `
            -SessionRoot $envData.SessionRoot `
            -TargetRoot $envData.Root `
            -ValuesPath $envData.ValuesPath `
            -Execute `
            -Force
        Assert-Equal -Actual $first.Stage -Expected 'implementation_ready' -Message 'Initial web+miniapp bootstrap should pass.'
        Assert-True -Condition (Test-Path (Join-Path $envData.Root '.codex\agents\engineering-wechat-mini-program-developer.md')) -Message 'Initial package should contain the miniapp agent.'
        Assert-True -Condition (Test-Path (Join-Path $envData.Root 'docs\agents\miniapp-handbook.md')) -Message 'Initial package should contain the miniapp handbook.'

        $decisionPath = Join-Path $envData.SessionRoot 'decision.json'
        $decision = Get-Content -Raw $decisionPath | ConvertFrom-Json
        $decision.enabled_roles = @('architect', 'backend', 'frontend', 'reviewer', 'qa', 'docs')
        $decision | ConvertTo-Json -Depth 10 | Set-Content -Path $decisionPath

        $values = Get-Content -Raw $envData.ValuesPath | ConvertFrom-Json
        $values.enabled_roles = "- architect`n- backend`n- frontend`n- reviewer`n- qa`n- docs"
        $values.roles_and_manuals = $values.enabled_roles
        $values.agent_dispatch_matrix = $values.enabled_roles
        $values | ConvertTo-Json -Depth 10 | Set-Content -Path $envData.ValuesPath

        & $initPath `
            -TargetRoot $envData.Root `
            -ValuesPath $envData.ValuesPath `
            -DecisionPath $decisionPath `
            -Execute `
            -Force | Out-Null
        $second = & $orchestratorPath `
            -Stage postcheck `
            -SessionRoot $envData.SessionRoot `
            -TargetRoot $envData.Root

        Assert-True -Condition ([bool]$second.Postcheck.Passed) -Message 'Rerun generation should clean obsolete generated role files before postcheck.'
        Assert-False -Condition (Test-Path (Join-Path $envData.Root '.codex\agents\engineering-wechat-mini-program-developer.md')) -Message 'Obsolete miniapp agent should be removed on rerun.'
        Assert-False -Condition (Test-Path (Join-Path $envData.Root 'docs\agents\miniapp-handbook.md')) -Message 'Obsolete miniapp handbook should be removed on rerun.'
    }

    Invoke-TestCase -Name 'direct postcheck reports missing expected role files after generated team is damaged' -Body {
        $envData = New-TestEnvironment -Name 'missing-role' -EnabledRoles @('frontend', 'reviewer', 'docs')

        $bootstrapResult = & $orchestratorPath `
            -Stage bootstrap `
            -SessionRoot $envData.SessionRoot `
            -TargetRoot $envData.Root `
            -ValuesPath $envData.ValuesPath `
            -Execute `
            -Force

        Assert-Equal -Actual $bootstrapResult.Stage -Expected 'implementation_ready' -Message 'Bootstrap should first promote into implementation_ready before the test removes a file.'

        Remove-Item -LiteralPath (Join-Path $envData.Root '.codex\agents\engineering-frontend-developer.md') -Force

        $result = & $orchestratorPath `
            -Stage postcheck `
            -SessionRoot $envData.SessionRoot `
            -TargetRoot $envData.Root

        Assert-Equal -Actual $result.Stage -Expected 'postcheck_failed' -Message 'Direct postcheck should fail when an expected role file is missing.'
        Assert-Contains -Text ([string]($result.Postcheck.MissingAgentFiles -join ' ')) -ExpectedSubstring '.codex/agents/engineering-frontend-developer.md' -Message 'Missing role file should be reported by direct postcheck.'
    }

    Invoke-TestCase -Name 'postcheck fails truth-source gate when generated speckit payload is removed' -Body {
        $envData = New-TestEnvironment -Name 'truth-source-postcheck-speckit' -EnabledRoles @('frontend', 'reviewer', 'docs')

        $bootstrapResult = & $orchestratorPath `
            -Stage bootstrap `
            -SessionRoot $envData.SessionRoot `
            -TargetRoot $envData.Root `
            -ValuesPath $envData.ValuesPath `
            -Execute `
            -Force
        Assert-Equal -Actual $bootstrapResult.Stage -Expected 'implementation_ready' -Message 'Bootstrap should first pass before truth-source damage.'

        Remove-Item -LiteralPath (Join-Path $envData.Root '.specify') -Recurse -Force
        $postcheckResult = & $orchestratorPath -Stage postcheck -SessionRoot $envData.SessionRoot -TargetRoot $envData.Root
        $status = Get-Content -Raw (Join-Path $envData.SessionRoot 'status.json') | ConvertFrom-Json

        Assert-Equal -Actual $postcheckResult.Stage -Expected 'postcheck_failed' -Message 'Missing generated .specify must fail postcheck via truth-source gate.'
        Assert-False -Condition ([bool]$postcheckResult.Postcheck.TruthSourceGatePassed) -Message 'Postcheck should expose failed truth-source gate.'
        Assert-Contains -Text ([string]($postcheckResult.Postcheck.TruthSourceGateIssues -join ' ')) -ExpectedSubstring '.specify' -Message 'Truth-source issues should mention missing .specify.'
        Assert-True -Condition ([bool]$status.truth_source_gate_failed) -Message 'Status should record truth-source gate failure.'
    }

    Invoke-TestCase -Name 'bootstrap execute accepts integration roles as part of the expected team' -Body {
        $envData = New-TestEnvironment `
            -Name 'integration-role' `
            -EnabledRoles @('frontend', 'reviewer', 'docs') `
            -Integrations @(@{ name = 'feishu'; display_name = '飞书' })

        $result = & $orchestratorPath `
            -Stage bootstrap `
            -SessionRoot $envData.SessionRoot `
            -TargetRoot $envData.Root `
            -ValuesPath $envData.ValuesPath `
            -Execute `
            -Force

        Assert-Equal -Actual $result.Stage -Expected 'implementation_ready' -Message 'Integration roles should be included in a passing implementation-ready promotion.'
        Assert-True -Condition (Test-Path (Join-Path $envData.Root '.codex\agents\engineering-feishu-integration-developer.md')) -Message 'Integration agent file should be generated from agency-agents-zh.'
        Assert-True -Condition (Test-Path (Join-Path $envData.Root 'docs\agents\integration-feishu-handbook.md')) -Message 'Integration handbook should be generated.'
        Assert-Contains -Text ([string]($result.Postcheck.ExpectedRoles -join ' ')) -ExpectedSubstring 'integration-feishu' -Message 'Expected roles should include integration-feishu.'
    }

    Invoke-TestCase -Name 'proposal to bootstrap flow should not create blank integration roles when no integrations are detected' -Body {
        $root = Join-Path $tempRoot 'no-integrations-flow'
        New-Item -ItemType Directory -Path $root -Force | Out-Null
        $projectRoot = [System.IO.Path]::GetFullPath($root)

        & $orchestratorPath -Stage start -ProjectRoot $projectRoot | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '学生管理系统' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '做一个学生管理系统，解决学生信息、班级、成绩的统一管理' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '学校教务老师和班主任' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '学生信息管理、班级管理、成绩录入、成绩查询、基础统计' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '优先快速上线，但要保证后续可扩展' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '从零开始，没有必须对接的平台' | Out-Null
        & $orchestratorPath -Stage propose -ProjectRoot $projectRoot | Out-Null
        & $orchestratorPath -Stage confirm -ProjectRoot $projectRoot -Choice 'B' | Out-Null

        $result = & $orchestratorPath -Stage bootstrap -ProjectRoot $projectRoot -TargetRoot $projectRoot -Execute -Force
        $decision = Get-Content -Raw (Join-Path $projectRoot '.commonhe\session\decision.json') | ConvertFrom-Json

        Assert-Equal -Actual $result.Stage -Expected 'implementation_ready' -Message 'A project with no integrations should still promote into implementation_ready.'
        Assert-True -Condition ([bool]$result.Postcheck.Passed) -Message 'Postcheck should pass when no integrations were requested.'
        Assert-Equal -Actual @($decision.integrations).Count -Expected 0 -Message 'Decision file should keep integrations empty when none are detected.'
        Assert-False -Condition (Test-Path (Join-Path $projectRoot '.codex\agents\integration-.md')) -Message 'Blank integration agent file must not be generated.'
        Assert-False -Condition (Test-Path (Join-Path $projectRoot 'docs\agents\integration--handbook.md')) -Message 'Blank integration handbook must not be generated.'
    }

    Invoke-TestCase -Name 'all workflow capabilities remain required even if an older client sends unselected flags' -Body {
        $root = Join-Path $tempRoot 'capability-mandatory-all'
        New-Item -ItemType Directory -Path $root -Force | Out-Null
        $projectRoot = [System.IO.Path]::GetFullPath($root)

        & $orchestratorPath -Stage start -ProjectRoot $projectRoot | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '学生管理系统（Web + 小程序）' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '做一个面向老师的学生管理系统，统一管理学生档案、考勤、成绩、作业和缴费' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '老师、班主任和教务管理员' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '学生档案、考勤、成绩、作业、缴费' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '必须同时支持 Web 管理后台和微信小程序端' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '从零开始，没有必须对接的平台' | Out-Null
        & $orchestratorPath -Stage propose -ProjectRoot $projectRoot | Out-Null

        $decisionPath = Join-Path $projectRoot '.commonhe\session\decision.json'
        $selectedCapabilities = @(
            @{ id = 'superpowers'; label = 'superpowers'; selected = $true; status = 'available' }
            @{ id = 'agent-browser'; label = 'agent-browser'; selected = $false; status = 'skipped_by_user' }
            @{ id = 'chrome-devtools'; label = 'chrome-devtools'; selected = $true; status = 'available' }
            @{ id = 'GitNexus'; label = 'GitNexus'; selected = $true; status = 'available' }
            @{ id = 'Speckit'; label = 'Speckit'; selected = $false; status = 'skipped_by_user' }
        )
        $probeResults = @(
            foreach ($capability in $selectedCapabilities) {
                @{ name = $capability.id; display_name = $capability.label; passed = $true; evidence = "seed $($capability.id)" }
            }
        )
        @{
            target_client = 'codex'
            selected_capabilities = $selectedCapabilities
            capability_probe_results = $probeResults
        } | ConvertTo-Json -Depth 10 | Set-Content -Path $decisionPath

        & $orchestratorPath -Stage confirm -ProjectRoot $projectRoot -Choice 'B' | Out-Null
        $result = & $orchestratorPath -Stage bootstrap -ProjectRoot $projectRoot -TargetRoot $projectRoot -Execute -Force
        $decision = Get-Content -Raw $decisionPath | ConvertFrom-Json
        $values = Get-Content -Raw (Join-Path $projectRoot '.commonhe\session\generated-values.json') | ConvertFrom-Json
        $capabilityDoc = Get-Content -Raw (Join-Path $projectRoot 'docs\skills\required-capabilities.md')
        $requiredNames = [string]($decision.required_capabilities.name -join ' ')

        Assert-Equal -Actual $result.Stage -Expected 'implementation_ready' -Message 'Bootstrap should pass after normalizing all workflow capabilities to mandatory.'
        foreach ($capabilityName in @('superpowers', 'agent-browser', 'chrome-devtools', 'GitNexus', 'Speckit')) {
            Assert-Contains -Text $requiredNames -ExpectedSubstring $capabilityName -Message "$capabilityName must remain required even if an old client sent selected=false."
            Assert-Contains -Text ([string]$values.required_capabilities_list) -ExpectedSubstring $capabilityName -Message "Generated values must list mandatory $capabilityName."
            Assert-Contains -Text $capabilityDoc -ExpectedSubstring $capabilityName -Message "Capability document must describe mandatory $capabilityName."
        }
        Assert-True -Condition (Test-Path (Join-Path $projectRoot '.specify\templates\spec-template.md')) -Message 'Mandatory Speckit must generate .specify files.'
        Assert-True -Condition (Test-Path (Join-Path $projectRoot '.specify\scripts\powershell\create-new-feature.ps1')) -Message 'Mandatory Speckit must generate workflow scripts.'
    }

    Invoke-TestCase -Name 'web miniapp student management flow generates concrete dual-end team package' -Body {
        $root = Join-Path $tempRoot 'student-management-web-miniapp-quality'
        New-Item -ItemType Directory -Path $root -Force | Out-Null
        $projectRoot = [System.IO.Path]::GetFullPath($root)

        & $orchestratorPath -Stage start -ProjectRoot $projectRoot | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '学生管理系统（Web + 小程序）' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '做一个面向老师的学生管理系统，统一管理学生档案、考勤、成绩、作业和缴费' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '学校教务老师和班主任' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '学生档案、考勤、成绩、作业、缴费' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '优先稳定和可验证，后续可扩展' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '必须同时支持 Web 管理后台和微信小程序端' | Out-Null
        & $orchestratorPath -Stage propose -ProjectRoot $projectRoot | Out-Null
        & $orchestratorPath -Stage confirm -ProjectRoot $projectRoot -Choice 'B' | Out-Null

        $result = & $orchestratorPath -Stage bootstrap -ProjectRoot $projectRoot -TargetRoot $projectRoot -Execute -Force
        $status = Get-Content -Raw (Join-Path $projectRoot '.commonhe\session\status.json') | ConvertFrom-Json
        $decision = Get-Content -Raw (Join-Path $projectRoot '.commonhe\session\decision.json') | ConvertFrom-Json
        $projectContext = Get-Content -Raw (Join-Path $projectRoot 'docs\project_context.md')
        $architecture = Get-Content -Raw (Join-Path $projectRoot 'docs\architecture\01-项目架构设计书.md')
        $roadmap = Get-Content -Raw (Join-Path $projectRoot 'docs\roadmap\01-实施路线图.md')
        $firstTaskPack = Get-Content -Raw (Join-Path $projectRoot 'docs\workflow\first-task-pack.md')
        $rootAgents = Get-Content -Raw (Join-Path $projectRoot 'AGENTS.md')
        $coordinatorDoc = Get-Content -Raw (Join-Path $projectRoot '.codex\COORDINATOR-SUBAGENTS.md')
        $handoff = Get-Content -Raw (Join-Path $projectRoot '.commonhe\session\bootstrap-handoff.md')
        $indexDoc = Get-Content -Raw (Join-Path $projectRoot 'docs\00-初始化结果索引.md')
        $combined = @($projectContext, $architecture, $roadmap, $firstTaskPack, $rootAgents, $coordinatorDoc, $handoff, $indexDoc) -join "`n"

        Assert-Equal -Actual $result.Stage -Expected 'implementation_ready' -Message 'Student management smoke should complete initialization.'
        Assert-True -Condition ([bool]$status.last_postcheck_passed) -Message 'Student management smoke should pass postcheck.'
        Assert-Equal -Actual ([string]$decision.delivery_mode) -Expected 'web-miniapp' -Message 'Web + miniapp student management must not be collapsed to web-app.'
        Assert-Contains -Text ([string]($decision.enabled_roles -join ' ')) -ExpectedSubstring 'miniapp' -Message 'Web + miniapp package must include a miniapp role.'
        foreach ($expected in @('学生档案', '考勤', '成绩', '作业', '缴费', 'Web 管理后台', '微信小程序端', '统一后端 API')) {
            Assert-Contains -Text $combined -ExpectedSubstring $expected -Message "Generated student-management docs should include $expected."
        }
        Assert-Contains -Text $rootAgents -ExpectedSubstring '主 Agent 业务协作工作流' -Message 'Project AGENTS should include a main-agent-authored workflow section.'
        Assert-Contains -Text $coordinatorDoc -ExpectedSubstring '微信小程序端 -> miniapp' -Message 'Coordinator workflow should route miniapp work to miniapp role.'
        Assert-Contains -Text $coordinatorDoc -ExpectedSubstring '统一后端 API -> backend + database' -Message 'Coordinator workflow should route shared API/data model work.'
        Assert-NotContains -Text $combined -UnexpectedSubstring '活动发布' -Message 'Teacher student-management docs must not keep the old activity-management domain.'
        Assert-NotContains -Text $combined -UnexpectedSubstring '报名记录' -Message 'Teacher student-management docs must not keep the old signup-record domain.'
        foreach ($badText in @('重启 Codex', 'CommonHE 以独立初始化包', '污染', '待补充', '角��')) {
            Assert-NotContains -Text $combined -UnexpectedSubstring $badText -Message "Generated docs should not contain bad text: $badText"
        }
        foreach ($templateResidue in @('{{', '}}', '$confirmedProjectName', '$clientEntryFile', '$targetClientName', 'ExampleProject', 'Build the first usable version quickly', '当前无自动分析假设', '当前无自动分析信号', '生成项目骨架', '当前目录已经生成 HE 协作工程')) {
            Assert-NotContains -Text $combined -UnexpectedSubstring $templateResidue -Message "Generated student-management docs should not contain template residue: $templateResidue"
        }
        Assert-NotContains -Text $combined -UnexpectedSubstring '.codex/skills' -Message 'Codex target should reference runtime skills under .agents/skills, not .codex/skills.'
        Assert-Contains -Text $combined -ExpectedSubstring '.agents/skills/required-capabilities.md' -Message 'Codex target should point to the generated runtime skill manifest.'
        Assert-Contains -Text $combined -ExpectedSubstring '初始化协作包' -Message 'Generated docs should consistently describe an initialization collaboration package.'
        Assert-Contains -Text $combined -ExpectedSubstring '主 Agent' -Message 'Generated docs should read as a main-agent handoff.'
        Assert-Contains -Text $combined -ExpectedSubstring 'superpowers' -Message 'Generated docs should include selected capability status.'
        Assert-True -Condition (Test-Path (Join-Path $projectRoot '.specify\templates\spec-template.md')) -Message 'Generated init package must include Speckit .specify files for downstream planning.'
        Assert-True -Condition (Test-Path (Join-Path $projectRoot '.specify\scripts\powershell\create-new-feature.ps1')) -Message 'Generated init package must include Speckit PowerShell workflow scripts.'
        Assert-True -Condition (Test-Path (Join-Path $projectRoot '.codex\agents\engineering-frontend-developer.md')) -Message 'Codex package must generate selected agents from agency-agents-zh, not only internal generic role names.'
        Assert-True -Condition (Test-Path (Join-Path $projectRoot '.codex\agents\engineering-wechat-mini-program-developer.md')) -Message 'Web + miniapp package must generate the real agency miniapp agent.'
        Assert-True -Condition (Test-Path (Join-Path $projectRoot '.codex\agents\engineering-backend-architect.md')) -Message 'Student management package should include the backend architect from agency-agents-zh.'
        Assert-True -Condition (Test-Path (Join-Path $projectRoot '.codex\agents\testing-evidence-collector.md')) -Message 'Student management package should include a real testing agent from agency-agents-zh.'
        $frontendAgencyAgent = Get-Content -Raw (Join-Path $projectRoot '.codex\agents\engineering-frontend-developer.md')
        Assert-Contains -Text $frontendAgencyAgent -ExpectedSubstring '来源：agency-agents-zh' -Message 'Generated agent files should preserve agency-agents-zh provenance.'
        Assert-False -Condition (Test-Path (Join-Path $projectRoot '.codex\agents\frontend.md')) -Message 'Codex package must not generate internal generic frontend.md instead of agency-agents-zh agents.'
        Assert-NotContains -Text $rootAgents -UnexpectedSubstring '临时补救命令' -Message 'Green Codex AGENTS should not show temporary remediation.'
        Assert-NotContains -Text $rootAgents -UnexpectedSubstring 'CommonHE 正式流程' -Message 'Green Codex AGENTS should not expose old product-flow wording.'
        Assert-Contains -Text $rootAgents -ExpectedSubstring 'docs/skills/required-capabilities.md' -Message 'Codex AGENTS should still point to capability gate docs.'
        Assert-False -Condition (Test-Path (Join-Path $projectRoot 'CLAUDE.md')) -Message 'Codex target must not generate CLAUDE.md.'
    }

    Invoke-TestCase -Name 'web miniapp bootstrap normalizes missing miniapp qa reviewer roles before postcheck' -Body {
        $root = Join-Path $tempRoot 'web-miniapp-missing-qa-reviewer-normalization'
        New-Item -ItemType Directory -Path $root -Force | Out-Null
        $projectRoot = [System.IO.Path]::GetFullPath($root)

        & $orchestratorPath -Stage start -ProjectRoot $projectRoot | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '家装电器浴霸电商网站+微信小程序双端商城协作包' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '个人消费者、企业采购，家装电器，浴霸网站，支持小程序、web端双端' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '个人消费者、企业采购、门店导购和后台管理员' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '商品列表/详情、购物车下单、在线支付、企业询价、品牌展示、导购问答、后台管理、物流/售后' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '必须同时支持 Web 端和微信小程序端' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '从零开始，没有必须对接的平台' | Out-Null
        & $orchestratorPath -Stage propose -ProjectRoot $projectRoot | Out-Null
        & $orchestratorPath -Stage confirm -ProjectRoot $projectRoot -Choice 'B' | Out-Null

        $decisionPath = Join-Path $projectRoot '.commonhe\session\decision.json'
        $decision = Get-Content -Raw $decisionPath | ConvertFrom-Json
        $decision.delivery_mode = 'web-miniapp'
        $decision.enabled_roles = @('architect', 'backend', 'database', 'docs', 'frontend')
        $decision.recommended_roles_now = @('architect', 'backend', 'database', 'docs', 'frontend')
        $decision.available_roles_later = @('miniapp', 'reviewer', 'qa')
        $decision | ConvertTo-Json -Depth 20 | Set-Content -Path $decisionPath

        $result = & $orchestratorPath -Stage bootstrap -ProjectRoot $projectRoot -TargetRoot $projectRoot -Execute -Force
        $normalizedDecision = Get-Content -Raw $decisionPath | ConvertFrom-Json
        $rootAgents = Get-Content -Raw (Join-Path $projectRoot 'AGENTS.md')
        $coordinatorDoc = Get-Content -Raw (Join-Path $projectRoot '.codex\COORDINATOR-SUBAGENTS.md')

        Assert-Equal -Actual $result.Stage -Expected 'implementation_ready' -Message 'Web + miniapp bootstrap must normalize qa/reviewer and pass postcheck.'
        Assert-Contains -Text ([string]($normalizedDecision.enabled_roles -join ' ')) -ExpectedSubstring 'qa' -Message 'Web + miniapp decision must force qa into enabled roles.'
        Assert-Contains -Text ([string]($normalizedDecision.enabled_roles -join ' ')) -ExpectedSubstring 'reviewer' -Message 'Web + miniapp decision must force reviewer into enabled roles.'
        Assert-Contains -Text ([string]($normalizedDecision.enabled_roles -join ' ')) -ExpectedSubstring 'miniapp' -Message 'Web + miniapp decision must force miniapp into enabled roles.'
        Assert-Contains -Text ([string]($normalizedDecision.recommended_roles_now -join ' ')) -ExpectedSubstring 'qa' -Message 'Web + miniapp recommended roles must force qa.'
        Assert-Contains -Text ([string]($normalizedDecision.recommended_roles_now -join ' ')) -ExpectedSubstring 'reviewer' -Message 'Web + miniapp recommended roles must force reviewer.'
        Assert-Contains -Text ([string]($normalizedDecision.recommended_roles_now -join ' ')) -ExpectedSubstring 'miniapp' -Message 'Web + miniapp recommended roles must force miniapp.'
        Assert-Contains -Text $rootAgents -ExpectedSubstring '跨端验证 -> qa + reviewer' -Message 'AGENTS.md must keep qa + reviewer for Web + miniapp validation.'
        Assert-Contains -Text $coordinatorDoc -ExpectedSubstring '跨端验证 -> qa + reviewer' -Message 'Coordinator must keep qa + reviewer for Web + miniapp validation.'
    }

    Invoke-TestCase -Name 'home appliance ecommerce web miniapp flow auto generates qa reviewer team package' -Body {
        $root = Join-Path $tempRoot 'bathroom-heater-ecommerce-web-miniapp-auto'
        New-Item -ItemType Directory -Path $root -Force | Out-Null
        $projectRoot = [System.IO.Path]::GetFullPath($root)

        & $orchestratorPath -Stage start -ProjectRoot $projectRoot | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '电商网站' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '个人消费者、企业采购，家装电器，浴霸网站' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '提升线上成交、展示品牌与产品、支持企业批量询价采购，替代线下门店导购' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '商品列表/详情、购物车下单、在线支付、企业询价、品牌展示、导购问答、后台管理、物流/售后' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '小程序、web端双端' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '从零开始，没有必须对接的平台' | Out-Null
        & $orchestratorPath -Stage propose -ProjectRoot $projectRoot | Out-Null
        & $orchestratorPath -Stage confirm -ProjectRoot $projectRoot -Choice 'B' | Out-Null

        $result = & $orchestratorPath -Stage bootstrap -ProjectRoot $projectRoot -TargetRoot $projectRoot -Execute -Force
        $decision = Get-Content -Raw (Join-Path $projectRoot '.commonhe\session\decision.json') | ConvertFrom-Json
        $status = Get-Content -Raw (Join-Path $projectRoot '.commonhe\session\status.json') | ConvertFrom-Json
        $values = Get-Content -Raw (Join-Path $projectRoot '.commonhe\session\generated-values.json') | ConvertFrom-Json
        $finalAcceptancePath = Join-Path $projectRoot '.commonhe\session\final-acceptance.json'
        $mengOutputPath = Join-Path $projectRoot '.commonhe\session\meng-xingxing-output.json'
        $finalAcceptance = if (Test-Path $finalAcceptancePath) { Get-Content -Raw $finalAcceptancePath | ConvertFrom-Json } else { $null }
        $mengOutput = if (Test-Path $mengOutputPath) { Get-Content -Raw $mengOutputPath | ConvertFrom-Json } else { $null }
        $rootAgents = Get-Content -Raw (Join-Path $projectRoot 'AGENTS.md')
        $coordinatorDoc = Get-Content -Raw (Join-Path $projectRoot '.codex\COORDINATOR-SUBAGENTS.md')
        $projectContext = Get-Content -Raw (Join-Path $projectRoot 'docs\project_context.md')
        $architecture = Get-Content -Raw (Join-Path $projectRoot 'docs\architecture\01-项目架构设计书.md')
        $roadmap = Get-Content -Raw (Join-Path $projectRoot 'docs\roadmap\01-实施路线图.md')
        $acceptanceGates = Get-Content -Raw (Join-Path $projectRoot 'docs\workflow\acceptance-gates.md')
        $firstTaskPack = Get-Content -Raw (Join-Path $projectRoot 'docs\workflow\first-task-pack.md')
        $checklist = Get-Content -Raw (Join-Path $projectRoot 'docs\workflow\current-stage-user-checklist.md')
        $firstSprintContract = Get-Content -Raw (Join-Path $projectRoot 'docs\workflow\first-sprint-contract.md')
        $sprintContractTemplate = Get-Content -Raw (Join-Path $projectRoot 'docs\workflow\sprint-contract-template.md')
        $frontendHandbook = Get-Content -Raw (Join-Path $projectRoot 'docs\agents\frontend-handbook.md')
        $miniappHandbook = Get-Content -Raw (Join-Path $projectRoot 'docs\agents\miniapp-handbook.md')
        $backendHandbook = Get-Content -Raw (Join-Path $projectRoot 'docs\agents\backend-handbook.md')

        Assert-Equal -Actual $result.Stage -Expected 'implementation_ready' -Message 'Home appliance ecommerce web+miniapp flow must complete from generator without hand-editing output.'
        Assert-True -Condition ([bool]$status.last_postcheck_passed) -Message 'Generated ecommerce collaboration package must pass postcheck.'
        Assert-True -Condition ([bool]$status.capability_gate_passed) -Message 'Successful desktop-agent package must persist capability_gate_passed=true.'
        Assert-Equal -Actual ([string]$decision.delivery_mode) -Expected 'web-miniapp' -Message 'Ecommerce web + miniapp requirement must classify as web-miniapp.'
        Assert-Equal -Actual ([string]$decision.selected_solution_id) -Expected 'B' -Message 'Decision must persist the user-selected solution id.'
        if ($finalAcceptance) {
            Assert-Equal -Actual ([string]$decision.selected_solution_id) -Expected ([string]$finalAcceptance.selectedSolutionId) -Message 'Decision and final acceptance must agree on selected solution.'
        }
        if ($mengOutput) {
            Assert-Equal -Actual ([string]$decision.selected_solution_id) -Expected ([string]$mengOutput.selectedSolutionId) -Message 'Decision and 梦星星 output must agree on selected solution.'
        }
        Assert-Equal -Actual @($decision.selected_capabilities).Count -Expected 5 -Message 'Decision must persist the five selected mandatory capabilities.'
        Assert-Equal -Actual @($values.selected_capabilities).Count -Expected 5 -Message 'Generated values must carry the five selected mandatory capabilities.'
        foreach ($role in @('frontend', 'miniapp', 'backend', 'database', 'qa', 'reviewer', 'docs')) {
            Assert-Contains -Text ([string]($decision.enabled_roles -join ' ')) -ExpectedSubstring $role -Message "Generated team must include $role for ecommerce web+miniapp."
        }
        foreach ($capabilityName in @('superpowers', 'agent-browser', 'chrome-devtools', 'GitNexus', 'Speckit')) {
            $capability = @($decision.selected_capabilities) | Where-Object { [string]$_.id -eq $capabilityName } | Select-Object -First 1
            Assert-True -Condition ($null -ne $capability) -Message "Decision must include selected capability $capabilityName."
            Assert-True -Condition ([bool]$capability.selected) -Message "Selected capability $capabilityName must be selected=true."
        }
        foreach ($enabledRole in @($decision.enabled_roles)) {
            Assert-False -Condition ([bool](@($decision.available_roles_later) -contains $enabledRole)) -Message "Enabled role must not also appear in available_roles_later: $enabledRole"
        }
        Assert-Contains -Text $rootAgents -ExpectedSubstring '跨端验证 -> qa + reviewer' -Message 'Generated AGENTS.md must automatically include qa + reviewer validation routing.'
        Assert-Contains -Text $coordinatorDoc -ExpectedSubstring '跨端验证 -> qa + reviewer' -Message 'Generated coordinator must automatically include qa + reviewer validation routing.'
        Assert-True -Condition (Test-Path (Join-Path $projectRoot '.codex\agents\testing-evidence-collector.md')) -Message 'Generated package must include the real agency testing agent.'
        Assert-False -Condition ($frontendHandbook -eq $miniappHandbook) -Message 'Miniapp handbook must not be copied from frontend handbook.'
        Assert-Contains -Text $miniappHandbook -ExpectedSubstring '微信小程序' -Message 'Miniapp handbook must be specific to the miniapp role.'
        Assert-Contains -Text $miniappHandbook -ExpectedSubstring '字段说明：`handoff_to`' -Message 'Agent handbooks must define the handoff_to output field.'
        foreach ($domainText in @('家装电器', '浴霸', '企业询价', '品牌展示', '导购问答', '物流/售后', 'Web 管理后台', '微信小程序端')) {
            Assert-Contains -Text $projectContext -ExpectedSubstring $domainText -Message "Generated project context must preserve ecommerce domain text: $domainText"
        }
        foreach ($generatedDoc in @($projectContext, $architecture, $roadmap, $acceptanceGates, $firstSprintContract, $backendHandbook)) {
            Assert-NotContains -Text $generatedDoc -UnexpectedSubstring '老师/班主任' -Message 'Ecommerce collaboration package must not leak school-role security wording.'
            Assert-NotContains -Text $generatedDoc -UnexpectedSubstring '�' -Message 'Generated collaboration package must not contain replacement-character mojibake.'
            Assert-NotContains -Text $generatedDoc -UnexpectedSubstring '生成工作区包含协议、文档和 session 产物' -Message 'Implementation acceptance text must not make init-package structure look like a business acceptance criterion.'
        }
        foreach ($domainDoc in @($projectContext, $architecture, $roadmap, $firstSprintContract)) {
            Assert-Contains -Text $domainDoc -ExpectedSubstring '物流/售后' -Message 'Generated ecommerce docs must preserve slash-combined feature terms such as 物流/售后.'
        }
        Assert-Contains -Text $rootAgents -ExpectedSubstring '物流/售后' -Message 'Codex AGENTS entry must preserve all user-confirmed ecommerce features.'
        Assert-Contains -Text $coordinatorDoc -ExpectedSubstring '物流/售后' -Message 'Codex coordinator must preserve all user-confirmed ecommerce features.'
        $generatedTextFiles = @(Get-ChildItem -LiteralPath $projectRoot -Recurse -File -Include '*.md', '*.json', '*.jsonl' | Where-Object { $_.FullName -notmatch '\\\.specify\\' })
        foreach ($file in $generatedTextFiles) {
            $content = Get-Content -LiteralPath $file.FullName -Raw
            foreach ($char in $content.ToCharArray()) {
                $code = [int][char]$char
                Assert-False -Condition (($code -lt 32) -and ($code -notin @(9, 10, 13))) -Message "Generated file contains illegal ASCII control character: $($file.FullName)"
            }
            if ($file.FullName -notmatch '\\\.commonhe\\session\\') {
                Assert-NotContains -Text $content -UnexpectedSubstring 'CommonHE Init Orchestrator' -Message "Generated user-facing file must not expose old orchestrator name: $($file.FullName)"
                Assert-NotContains -Text $content -UnexpectedSubstring 'CommonHE 正式流程' -Message "Generated user-facing file must not expose old product-flow wording: $($file.FullName)"
            }
        }
        Assert-NotContains -Text $rootAgents -UnexpectedSubstring '临时补救命令' -Message 'Green capability gate packages must not show temporary remediation at the root entry.'
        Assert-NotContains -Text $rootAgents -UnexpectedSubstring '尚未记录 doctor 结果' -Message 'Green desktop-agent packages must not show stale doctor-yellow wording.'
        Assert-NotContains -Text $acceptanceGates -UnexpectedSubstring '核心实现已完成' -Message 'Acceptance gates must not claim business implementation is complete.'
        Assert-NotContains -Text $acceptanceGates -UnexpectedSubstring 'review 已完成' -Message 'Acceptance gates must not claim review is complete before implementation.'
        Assert-NotContains -Text $acceptanceGates -UnexpectedSubstring 'qa 已完成' -Message 'Acceptance gates must not claim QA is complete before implementation.'
        Assert-NotContains -Text $acceptanceGates -UnexpectedSubstring 'Web 管理后台覆盖' -Message 'Acceptance gates for a successful init package must not require business feature completion.'
        Assert-NotContains -Text $acceptanceGates -UnexpectedSubstring '微信小程序端覆盖' -Message 'Acceptance gates for a successful init package must not require miniapp feature completion.'
        Assert-Contains -Text $acceptanceGates -ExpectedSubstring '当前验收只确认协作包初始化完成' -Message 'Acceptance gates must describe init-package acceptance only.'
        Assert-NotContains -Text $firstTaskPack -UnexpectedSubstring '已有可验证实施成果' -Message 'First task pack must not claim implementation output exists.'
        Assert-NotContains -Text $firstTaskPack -UnexpectedSubstring '均有验证证据' -Message 'First task pack must not claim business verification evidence already exists.'
        Assert-Contains -Text $firstTaskPack -ExpectedSubstring '等待后续实施' -Message 'First task pack should frame business work as follow-up implementation.'
        Assert-NoDuplicateMarkdownSectionNumbers -Text $rootAgents -Message 'AGENTS.md must not contain duplicate numbered sections.'
        Assert-NotContains -Text $checklist -UnexpectedSubstring '先完成初始化落盘' -Message 'Implementation checklist must not ask the handoff thread to redo initialization.'
        Assert-NotContains -Text $checklist -UnexpectedSubstring 'postcheck' -Message 'Implementation checklist must not ask the handoff thread to reason about init postcheck.'
        Assert-NotContains -Text $checklist -UnexpectedSubstring 'bootstrap' -Message 'Implementation checklist must not leak bootstrap wording into the handoff thread.'
        Assert-False -Condition ($firstSprintContract -eq $sprintContractTemplate) -Message 'First sprint contract must be a concrete instance, not a byte-for-byte copy of the generic template.'
        Assert-NotContains -Text $firstSprintContract -UnexpectedSubstring 'Sprint Contract 模板' -Message 'First sprint contract must be a concrete project instance, not a template.'
        Assert-NotContains -Text $firstSprintContract -UnexpectedSubstring '使用前请先复制' -Message 'First sprint contract must not tell the handoff agent to copy a template.'
        Assert-NotContains -Text $firstSprintContract -UnexpectedSubstring '任务级 Contract 脚手架' -Message 'First sprint contract must not describe itself as a scaffold.'
        Assert-NotContains -Text $firstSprintContract -UnexpectedSubstring '不代表当前项目已经存在一个已签署' -Message 'First sprint contract must not deny that it is the current first-sprint contract.'
        Assert-NotContains -Text $firstSprintContract -UnexpectedSubstring '待填写' -Message 'First sprint contract must not contain placeholder cells.'
        Assert-Contains -Text $firstSprintContract -ExpectedSubstring '首轮实施合同' -Message 'First sprint contract must be titled as a first-sprint implementation contract.'
        Assert-Contains -Text $firstSprintContract -ExpectedSubstring '第一优先工作流' -Message 'First sprint contract must describe the current first-priority workflow.'
        Assert-NotContains -Text $sprintContractTemplate -UnexpectedSubstring '家装电器' -Message 'Generic sprint contract template must not be polluted by current business domain terms.'
        Assert-NotContains -Text $sprintContractTemplate -UnexpectedSubstring '浴霸' -Message 'Generic sprint contract template must not be polluted by current business domain terms.'
    }

    Invoke-TestCase -Name 'claude-code target omits remediation when capability gate is green and has no Codex entry' -Body {
        $envData = New-TestEnvironment -Name 'claude-remediation' -EnabledRoles @('frontend', 'reviewer', 'docs')
        $decisionPath = Join-Path $envData.SessionRoot 'decision.json'
        $valuesPath = $envData.ValuesPath
        $decision = Get-Content -Raw $decisionPath | ConvertFrom-Json
        $values = Get-Content -Raw $valuesPath | ConvertFrom-Json

        $decision | Add-Member -NotePropertyName target_client -NotePropertyValue 'claude-code' -Force
        $decision | ConvertTo-Json -Depth 12 | Set-Content -Path $decisionPath
        $values.target_client = 'claude-code'
        $values.target_client_name = 'Claude Code'
        $values.client_entry_file = 'CLAUDE.md'
        $values.client_coordinator_path = '.claude/settings.json'
        $values.client_agent_path = '.claude/agents/*.md'
        $values.client_skill_path = '.claude/skills/required-capabilities.md'
        $values | ConvertTo-Json -Depth 12 | Set-Content -Path $valuesPath

        $result = & $orchestratorPath `
            -Stage bootstrap `
            -SessionRoot $envData.SessionRoot `
            -TargetRoot $envData.Root `
            -ValuesPath $valuesPath `
            -Execute `
            -Force

        $claudeDoc = Get-Content -Raw (Join-Path $envData.Root 'CLAUDE.md')
        Assert-Equal -Actual $result.Stage -Expected 'implementation_ready' -Message 'Claude Code target should complete initialization.'
        Assert-True -Condition (Test-Path (Join-Path $envData.Root '.claude\settings.json')) -Message 'Claude Code target should generate .claude settings.'
        Assert-False -Condition (Test-Path (Join-Path $envData.Root 'AGENTS.md')) -Message 'Claude Code target must not generate AGENTS.md.'
        Assert-NotContains -Text $claudeDoc -UnexpectedSubstring '临时补救命令' -Message 'Green CLAUDE entry should not show temporary remediation.'
        Assert-NotContains -Text $claudeDoc -UnexpectedSubstring 'CommonHE 正式流程' -Message 'Green CLAUDE entry should not expose old product-flow wording.'
        Assert-Contains -Text $claudeDoc -ExpectedSubstring 'superpowers' -Message 'CLAUDE should still list required capabilities.'
    }

    Invoke-TestCase -Name 'showcase-site flow should promote init to implementation-ready with external references and lightweight roles' -Body {
        $root = Join-Path $tempRoot 'showcase-flow'
        New-Item -ItemType Directory -Path $root -Force | Out-Null
        $projectRoot = [System.IO.Path]::GetFullPath($root)

        & $orchestratorPath -Stage start -ProjectRoot $projectRoot | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText 'ByteKnowledge' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '先做一个解决方案展示型落地页，展示知识沉淀能力与双库设计，不做完整平台' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '潜在客户、销售和方案负责人' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '首屏价值表达、解决方案架构、双库设计说明、场景案例、行动引导' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '优先上线速度、稳定、后续可扩展；风格参考现有品牌前端；内容参考现有知识库沉淀说明' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '从零开始，无必须对接平台，本轮不做后端与集成' | Out-Null
        & $orchestratorPath -Stage propose -ProjectRoot $projectRoot | Out-Null
        & $orchestratorPath -Stage confirm -ProjectRoot $projectRoot -Choice 'A' | Out-Null

        $result = & $orchestratorPath -Stage bootstrap -ProjectRoot $projectRoot -TargetRoot $projectRoot -Execute -Force
        $status = Get-Content -Raw (Join-Path $projectRoot '.commonhe\session\status.json') | ConvertFrom-Json
        $decision = Get-Content -Raw (Join-Path $projectRoot '.commonhe\session\decision.json') | ConvertFrom-Json
        $projectContext = Get-Content -Raw (Join-Path $projectRoot 'docs\project_context.md')
        $roadmap = Get-Content -Raw (Join-Path $projectRoot 'docs\roadmap\01-实施路线图.md')
        $checklist = Get-Content -Raw (Join-Path $projectRoot 'docs\workflow\current-stage-user-checklist.md')
        $gates = Get-Content -Raw (Join-Path $projectRoot 'docs\workflow\acceptance-gates.md')
        $indexDoc = Get-Content -Raw (Join-Path $projectRoot 'docs\00-初始化结果索引.md')

        Assert-Equal -Actual $result.Stage -Expected 'implementation_ready' -Message 'Showcase-site projects should promote to implementation_ready after bootstrap.'
        Assert-Equal -Actual $status.stage -Expected 'implementation_ready' -Message 'Status should record implementation_ready after post-bootstrap promotion.'
        Assert-True -Condition ([bool]$status.implementation_stage_promoted) -Message 'Status should record implementation stage promotion.'
        Assert-Equal -Actual $decision.delivery_mode -Expected 'solution-site' -Message 'Showcase project should use a lightweight delivery mode.'
        Assert-Contains -Text ([string]($decision.recommended_roles_now -join ' ')) -ExpectedSubstring 'frontend' -Message 'Lightweight projects should keep frontend in current roles.'
        Assert-False -Condition ([bool](($decision.recommended_roles_now -join ' ') -like '*backend*')) -Message 'Lightweight showcase flow should not activate backend for the current stage.'
        Assert-Contains -Text ([string]($decision.available_roles_later -join ' ')) -ExpectedSubstring 'backend' -Message 'Backend should be deferred into available_roles_later for showcase projects.'
        Assert-Equal -Actual @($decision.external_references).Count -Expected 2 -Message 'External references should be promoted into long-term truth sources.'
        Assert-Contains -Text $projectContext -ExpectedSubstring '外部参考源' -Message 'Project context should include external references section.'
        Assert-Contains -Text $projectContext -ExpectedSubstring '现有品牌前端' -Message 'Project context should retain style reference.'
        Assert-Contains -Text $projectContext -ExpectedSubstring '现有知识库沉淀说明' -Message 'Project context should retain content reference.'
        Assert-Contains -Text $roadmap -ExpectedSubstring 'M2 首轮实施接手' -Message 'Roadmap should switch milestone wording to implementation-stage handoff.'
        Assert-False -Condition ([bool]($roadmap -like '*骨架已生成*')) -Message 'Roadmap should no longer treat skeleton generation as current success criteria.'
        Assert-Contains -Text $checklist -ExpectedSubstring '风格参考源' -Message 'Checklist should switch to implementation-stage items.'
        Assert-False -Condition ([bool]($checklist -like '*bootstrap*')) -Message 'Checklist should not retain bootstrap confirmation items after promotion.'
        Assert-Contains -Text $gates -ExpectedSubstring '当前验收只确认协作包初始化完成' -Message 'Acceptance gates should stay scoped to init-package acceptance.'
        Assert-NotContains -Text $gates -UnexpectedSubstring '外部参考已读取并落实' -Message 'Acceptance gates must not claim external-reference implementation is complete.'
        Assert-Contains -Text $indexDoc -ExpectedSubstring '新线程必读' -Message 'Init result index should expose a must-read section for new threads.'
        Assert-Contains -Text $result.Message -ExpectedSubstring '初始化协作包已生成并通过 postcheck' -Message 'Success message should explicitly announce init package closure.'
    }

    Invoke-TestCase -Name 'platform flow should remain engineering-oriented while still promoting to implementation-ready' -Body {
        $root = Join-Path $tempRoot 'platform-flow'
        New-Item -ItemType Directory -Path $root -Force | Out-Null
        $projectRoot = [System.IO.Path]::GetFullPath($root)

        & $orchestratorPath -Stage start -ProjectRoot $projectRoot | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText 'OpsPlatform' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '做一个多租户 SaaS 平台，先完成组织、成员、权限、后台管理和飞书集成' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '企业管理员和运营团队' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '组织管理、成员管理、权限配置、后台管理、飞书集成' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '稳定和扩展优先，需要支持多租户与后续合规' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '需要对接飞书，当前阶段就做后台和接口' | Out-Null
        & $orchestratorPath -Stage propose -ProjectRoot $projectRoot | Out-Null
        & $orchestratorPath -Stage confirm -ProjectRoot $projectRoot -Choice 'C' | Out-Null

        $result = & $orchestratorPath -Stage bootstrap -ProjectRoot $projectRoot -TargetRoot $projectRoot -Execute -Force
        $decision = Get-Content -Raw (Join-Path $projectRoot '.commonhe\session\decision.json') | ConvertFrom-Json

        Assert-Equal -Actual $result.Stage -Expected 'implementation_ready' -Message "Platform projects should also promote into implementation_ready. Postcheck: $($result.Postcheck | ConvertTo-Json -Depth 8)"
        Assert-Equal -Actual $decision.delivery_mode -Expected 'saas-platform' -Message 'Engineering-oriented platform projects should remain platform-typed.'
        Assert-Contains -Text ([string]($decision.recommended_roles_now -join ' ')) -ExpectedSubstring 'backend' -Message 'Platform flow should keep backend active in current roles.'
        Assert-Contains -Text ([string]($decision.integrations | ConvertTo-Json -Depth 10)) -ExpectedSubstring 'feishu' -Message 'Platform flow should keep immediate integrations in the decision structure when required now.'
    }

    Invoke-TestCase -Name 'closed implementation-ready sessions should reject further init stages except status and postcheck' -Body {
        $envData = New-TestEnvironment -Name 'closure-gate' -EnabledRoles @('frontend', 'reviewer', 'docs')

        $bootstrapResult = & $orchestratorPath `
            -Stage bootstrap `
            -SessionRoot $envData.SessionRoot `
            -TargetRoot $envData.Root `
            -ValuesPath $envData.ValuesPath `
            -Execute `
            -Force

        Assert-Equal -Actual $bootstrapResult.Stage -Expected 'implementation_ready' -Message 'Bootstrap should first reach implementation_ready before closure gate checks.'

        $statusResult = & $orchestratorPath -Stage status -SessionRoot $envData.SessionRoot
        $answerResult = & $orchestratorPath -Stage answer -SessionRoot $envData.SessionRoot -InputText '继续做页面'
        $proposeResult = & $orchestratorPath -Stage propose -SessionRoot $envData.SessionRoot
        $confirmResult = & $orchestratorPath -Stage confirm -SessionRoot $envData.SessionRoot -Choice 'A'
        $bootstrapAgainResult = & $orchestratorPath -Stage bootstrap -SessionRoot $envData.SessionRoot -TargetRoot $envData.Root -Execute -Force

        Assert-True -Condition ([bool]$statusResult.ClosureGateActive) -Message 'Status should expose closure_gate_active after implementation-ready closure.'
        Assert-Contains -Text ([string]$statusResult.ClosureMessage) -ExpectedSubstring 'Codex' -Message 'Status should expose the target-client closure message.'
        foreach ($result in @($answerResult, $proposeResult, $confirmResult, $bootstrapAgainResult)) {
            Assert-Equal -Actual $result.Stage -Expected 'completed_closure' -Message 'Closed init sessions should return the completed_closure output stage.'
            Assert-Contains -Text ([string]$result.Message) -ExpectedSubstring '初始化协作包已生成并通过 postcheck' -Message 'Closed init sessions should instruct the user to switch threads.'
        }

        $statusFile = Get-Content -Raw (Join-Path $envData.SessionRoot 'status.json') | ConvertFrom-Json
        Assert-Equal -Actual $statusFile.stage -Expected 'implementation_ready' -Message 'Closure gate should not reopen the init workflow.'
    }

    Invoke-TestCase -Name 'successful implementation-ready outputs should keep closure guidance out of implementation checklist and preserve safe-retained paths' -Body {
        $root = Join-Path $tempRoot 'closure-checklist'
        New-Item -ItemType Directory -Path $root -Force | Out-Null
        $projectRoot = [System.IO.Path]::GetFullPath($root)

        & $orchestratorPath -Stage start -ProjectRoot $projectRoot | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText 'ByteKnowledge' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '先做一个解决方案展示型落地页，展示知识沉淀能力与双库设计，不做完整平台' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '潜在客户、销售和方案负责人' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '首屏价值表达、解决方案架构、双库设计说明、场景案例、行动引导' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '优先上线速度、稳定、后续可扩展；风格参考现有品牌前端；内容参考现有知识库沉淀说明' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '从零开始，无必须对接平台，本轮不做后端与集成' | Out-Null
        & $orchestratorPath -Stage propose -ProjectRoot $projectRoot | Out-Null
        & $orchestratorPath -Stage confirm -ProjectRoot $projectRoot -Choice 'A' | Out-Null
        & $orchestratorPath -Stage bootstrap -ProjectRoot $projectRoot -TargetRoot $projectRoot -Execute -Force | Out-Null

        $checklist = Get-Content -Raw (Join-Path $projectRoot 'docs\workflow\current-stage-user-checklist.md')
        $handoff = Get-Content -Raw (Join-Path $projectRoot '.commonhe\session\bootstrap-handoff.md')
        $indexDoc = Get-Content -Raw (Join-Path $projectRoot 'docs\00-初始化结果索引.md')

        foreach ($doc in @($handoff, $indexDoc)) {
            Assert-Contains -Text $doc -ExpectedSubstring 'Codex' -Message 'Successful closure outputs should use target-client restart guidance.'
            Assert-NotContains -Text $doc -UnexpectedSubstring '手动移除' -Message 'Successful closure outputs should not mention manual package removal in Desktop v1.'
            Assert-Contains -Text $doc -ExpectedSubstring '.codex/' -Message 'Successful closure outputs should preserve .codex as a safe-retained path.'
            Assert-Contains -Text $doc -ExpectedSubstring '.agents/skills' -Message 'Successful closure outputs should preserve Codex skill path.'
            Assert-Contains -Text $doc -ExpectedSubstring '.commonhe/session' -Message 'Successful closure outputs should preserve .commonhe/session as a safe-retained path.'
        }

        Assert-NotContains -Text $checklist -UnexpectedSubstring '手动移除' -Message 'Implementation checklist should not carry package removal guidance.'
        Assert-NotContains -Text $checklist -UnexpectedSubstring '.commonhe/session' -Message 'Implementation checklist should stay focused on implementation, not init closure paths.'
        Assert-NotContains -Text $checklist -UnexpectedSubstring '初始化收口清单' -Message 'Implementation checklist should not contain init-only closure sections.'
    }

    Invoke-TestCase -Name 'failed postcheck outputs should not claim postcheck passed or implementation promotion' -Body {
        $root = Join-Path $tempRoot 'failure-doc-consistency'
        New-Item -ItemType Directory -Path $root -Force | Out-Null
        $projectRoot = [System.IO.Path]::GetFullPath($root)

        & $orchestratorPath -Stage start -ProjectRoot $projectRoot | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText 'ByteKnowledge' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '先做一个解决方案展示型落地页，展示知识沉淀能力与双库设计，不做完整平台' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '潜在客户、销售和方案负责人' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '首屏价值表达、解决方案架构、双库设计说明、场景案例、行动引导' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '优先上线速度、稳定、后续可扩展；风格参考现有品牌前端；内容参考现有知识库沉淀说明' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '从零开始，无必须对接平台，本轮不做后端与集成' | Out-Null
        & $orchestratorPath -Stage propose -ProjectRoot $projectRoot | Out-Null
        & $orchestratorPath -Stage confirm -ProjectRoot $projectRoot -Choice 'A' | Out-Null

        $unexpectedAgentDir = Join-Path $projectRoot '.codex\agents'
        $unexpectedHandbookDir = Join-Path $projectRoot 'docs\agents'
        New-Item -ItemType Directory -Path $unexpectedAgentDir -Force | Out-Null
        New-Item -ItemType Directory -Path $unexpectedHandbookDir -Force | Out-Null
        Set-Content -Path (Join-Path $unexpectedAgentDir 'unexpected.md') -Value '# unexpected'
        Set-Content -Path (Join-Path $unexpectedHandbookDir 'unexpected-handbook.md') -Value '# unexpected'

        & $orchestratorPath `
            -Stage bootstrap `
            -ProjectRoot $projectRoot `
            -TargetRoot $projectRoot `
            -Execute `
            -Force | Out-Null

        $handoff = Get-Content -Raw (Join-Path $projectRoot '.commonhe\session\bootstrap-handoff.md')
        $indexDoc = Get-Content -Raw (Join-Path $projectRoot 'docs\00-初始化结果索引.md')
        $checklist = Get-Content -Raw (Join-Path $projectRoot 'docs\workflow\current-stage-user-checklist.md')

        foreach ($doc in @($handoff, $indexDoc, $checklist)) {
            Assert-NotContains -Text $doc -UnexpectedSubstring 'postcheck 已通过' -Message 'Failed postcheck documents must not claim postcheck passed.'
            Assert-NotContains -Text $doc -UnexpectedSubstring '已推进到首个实施阶段' -Message 'Failed postcheck documents must not claim implementation promotion.'
        }
    }

    Invoke-TestCase -Name 'closed sessions should stay closed when allowed postcheck later fails' -Body {
        $envData = New-TestEnvironment -Name 'closed-postcheck-failure' -EnabledRoles @('frontend', 'reviewer', 'docs')

        $bootstrapResult = & $orchestratorPath `
            -Stage bootstrap `
            -SessionRoot $envData.SessionRoot `
            -TargetRoot $envData.Root `
            -ValuesPath $envData.ValuesPath `
            -Execute `
            -Force

        Assert-Equal -Actual $bootstrapResult.Stage -Expected 'implementation_ready' -Message 'Bootstrap should first close the init workflow.'

        Remove-Item -LiteralPath (Join-Path $envData.Root '.codex\agents\engineering-frontend-developer.md') -Force

        $postcheckResult = & $orchestratorPath `
            -Stage postcheck `
            -SessionRoot $envData.SessionRoot `
            -TargetRoot $envData.Root

        $status = Get-Content -Raw (Join-Path $envData.SessionRoot 'status.json') | ConvertFrom-Json
        $answerResult = & $orchestratorPath -Stage answer -SessionRoot $envData.SessionRoot -InputText '继续做页面'

        Assert-Equal -Actual $postcheckResult.Stage -Expected 'postcheck_failed' -Message 'Postcheck should still report the failed verification.'
        Assert-Equal -Actual $status.stage -Expected 'implementation_ready' -Message 'Failed postcheck after closure must not reopen the init workflow.'
        Assert-True -Condition ([bool]$status.init_closed) -Message 'Failed postcheck after closure must keep init_closed true.'
        Assert-Equal -Actual $answerResult.Stage -Expected 'completed_closure' -Message 'Closure gate should remain active after a failed postcheck recheck.'
    }

    Invoke-TestCase -Name 'negated platform wording should not classify showcase pages as saas platform' -Body {
        $root = Join-Path $tempRoot 'negated-platform-showcase'
        New-Item -ItemType Directory -Path $root -Force | Out-Null
        $projectRoot = [System.IO.Path]::GetFullPath($root)

        & $orchestratorPath -Stage start -ProjectRoot $projectRoot | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText 'DemoShowcase' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '先做展示页，不做平台，不扩展平台能力' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '销售团队' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '展示页首屏、能力介绍、联系入口' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '快速上线，稳定，后续可扩展' | Out-Null
        & $orchestratorPath -Stage answer -ProjectRoot $projectRoot -InputText '从零开始，没有必须对接的平台' | Out-Null
        & $orchestratorPath -Stage propose -ProjectRoot $projectRoot | Out-Null
        & $orchestratorPath -Stage confirm -ProjectRoot $projectRoot -Choice 'A' | Out-Null

        $decision = Get-Content -Raw (Join-Path $projectRoot '.commonhe\session\decision.json') | ConvertFrom-Json

        Assert-Equal -Actual $decision.delivery_mode -Expected 'showcase-site' -Message 'Negated platform wording should not override showcase delivery mode.'
        Assert-False -Condition ([bool](($decision.recommended_roles_now -join ' ') -like '*backend*')) -Message 'Negated platform showcase should stay lightweight.'
    }
} finally {
    if ($null -ne $originalCapabilityCatalogPath) {
        $env:COMMONHE_REQUIRED_CAPABILITIES_PATH = $originalCapabilityCatalogPath
    } else {
        Remove-Item Env:COMMONHE_REQUIRED_CAPABILITIES_PATH -ErrorAction SilentlyContinue
    }
    if (Test-Path $tempRoot) {
        Remove-Item -LiteralPath $tempRoot -Recurse -Force
    }
}
