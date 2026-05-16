param(
    [string]$ReleaseName = 'xingxing-vibecoding-launcher-v1.0-portable-current',
    [string]$ReleaseZipPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot
if ([string]::IsNullOrWhiteSpace($ReleaseZipPath)) {
    $ReleaseZipPath = Join-Path $repoRoot "release\$ReleaseName.zip"
}
$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("CommonHE-ReleaseTests-" + [System.Guid]::NewGuid().ToString('N'))

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
        [string[]]$Entries,
        [string]$ExpectedEntry,
        [string]$Message
    )

    if ($ExpectedEntry -notin $Entries) {
        throw "$Message`nExpected entry: $ExpectedEntry"
    }
}

function Assert-TextContains {
    param(
        [string]$Text,
        [string]$ExpectedSubstring,
        [string]$Message
    )

    if (-not ([string]$Text).Contains([string]$ExpectedSubstring)) {
        throw "$Message`nExpected substring: $ExpectedSubstring"
    }
}

function Assert-TextNotContains {
    param(
        [string]$Text,
        [string]$UnexpectedSubstring,
        [string]$Message
    )

    if (([string]$Text).Contains([string]$UnexpectedSubstring)) {
        throw "$Message`nUnexpected substring: $UnexpectedSubstring"
    }
}

function New-CapabilityCatalog {
    param([string]$Root)

    $catalog = @(
        @{
            name = 'superpowers'
            display_name = 'superpowers'
            group = 'core'
            verify_command = 'superpowers-check'
            install_commands = @('install superpowers')
            install_notes = @('seeded by release smoke test')
            remediation = @('install superpowers')
            probes = @(
                @{ type = 'file_exists'; paths = @((Join-Path $Root 'capabilities\superpowers.ok')) }
            )
        }
        @{
            name = 'agent-browser'
            display_name = 'agent-browser'
            group = 'browser'
            verify_command = 'agent-browser-check'
            install_commands = @('install agent-browser')
            install_notes = @('seeded by release smoke test')
            remediation = @('install agent-browser')
            probes = @(
                @{ type = 'file_exists'; paths = @((Join-Path $Root 'capabilities\agent-browser.ok')) }
            )
        }
        @{
            name = 'chrome-devtools'
            display_name = 'chrome-devtools'
            group = 'browser'
            verify_command = '~/.codex/config.toml'
            install_commands = @('codex mcp add chrome-devtools -- npx chrome-devtools-mcp@latest')
            install_notes = @('requires [mcp_servers.chrome-devtools] and chrome-devtools-mcp in ~/.codex/config.toml')
            remediation = @('install chrome-devtools')
            probes = @(
                @{ type = 'file_contains'; paths = @((Join-Path $Root 'codex\config.toml')); patterns = @('[mcp_servers.chrome-devtools]', 'chrome-devtools-mcp') }
            )
        }
        @{
            name = 'GitNexus'
            display_name = 'GitNexus'
            group = 'analysis'
            verify_command = 'gitnexus --version'
            install_commands = @('install GitNexus')
            install_notes = @('seeded by release smoke test')
            remediation = @('install GitNexus')
            probes = @(
                @{ type = 'file_exists'; paths = @((Join-Path $Root 'capabilities\gitnexus.ok')) }
            )
        }
        @{
            name = 'Speckit'
            display_name = 'Speckit'
            group = 'spec'
            verify_command = 'specify --version'
            install_commands = @('install Speckit')
            install_notes = @('seeded by release smoke test')
            remediation = @('install Speckit')
            probes = @(
                @{ type = 'file_exists'; paths = @((Join-Path $Root 'capabilities\speckit.ok')) }
            )
        }
    )

    New-Item -ItemType Directory -Path (Join-Path $Root 'capabilities') -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $Root 'codex') -Force | Out-Null
    foreach ($marker in @('superpowers.ok', 'agent-browser.ok', 'gitnexus.ok', 'speckit.ok')) {
        New-Item -ItemType File -Path (Join-Path $Root "capabilities\$marker") -Force | Out-Null
    }
    Set-Content -Path (Join-Path $Root 'codex\config.toml') -Value "[mcp_servers.chrome-devtools]`ncommand = ""npx""`nargs = [""chrome-devtools-mcp@latest""]"

    $catalogPath = Join-Path $Root 'required-capabilities.json'
    $catalog | ConvertTo-Json -Depth 10 | Set-Content -Path $catalogPath
    $catalogPath
}

function Set-TestCapabilityDefaults {
    param([pscustomobject]$Values)

    $selectedCapabilities = @(
        @{ id = 'superpowers'; label = 'superpowers'; selected = $true; recommended = $true; status = 'pass'; detail = 'release smoke seed' }
        @{ id = 'agent-browser'; label = 'agent-browser'; selected = $true; recommended = $true; status = 'pass'; detail = 'release smoke seed' }
        @{ id = 'chrome-devtools'; label = 'chrome-devtools'; selected = $true; recommended = $true; status = 'pass'; detail = 'release smoke seed' }
        @{ id = 'GitNexus'; label = 'GitNexus'; selected = $true; recommended = $true; status = 'pass'; detail = 'release smoke seed' }
        @{ id = 'Speckit'; label = 'Speckit'; selected = $true; recommended = $true; status = 'pass'; detail = 'release smoke seed' }
    )
    $Values | Add-Member -NotePropertyName selected_capabilities -NotePropertyValue $selectedCapabilities -Force
    $Values | Add-Member -NotePropertyName required_capabilities_list -NotePropertyValue "- superpowers`n- agent-browser`n- chrome-devtools`n- GitNexus`n- Speckit" -Force
    $Values | Add-Member -NotePropertyName capability_probe_summary -NotePropertyValue "- superpowers: pass`n- agent-browser: pass`n- chrome-devtools: pass`n- GitNexus: pass`n- Speckit: pass" -Force
    $Values | Add-Member -NotePropertyName capability_gate_status -NotePropertyValue '- 绿色：能力探测与 precheck 已通过' -Force
    $Values | Add-Member -NotePropertyName autodiscovery_assumptions -NotePropertyValue '- release smoke 使用最小 Web 产品语境验证初始化协作包闭环' -Force
    $Values | Add-Member -NotePropertyName autodiscovery_signal_summary -NotePropertyValue '- 检测到 package.json、src、tests，按既有 Web 项目协作包路径收口' -Force
    $Values | Add-Member -NotePropertyName implementation_owner_role -NotePropertyValue 'frontend' -Force
    $Values | Add-Member -NotePropertyName implementation_support_roles -NotePropertyValue '- reviewer`n- docs' -Force
    $Values | Add-Member -NotePropertyName dominant_workstreams_summary -NotePropertyValue '- frontend' -Force
    $Values | Add-Member -NotePropertyName kickoff_required_reads -NotePropertyValue '- docs/project_context.md`n- docs/workflow/implementation-kickoff.md`n- docs/workflow/first-task-pack.md' -Force
    $Values | Add-Member -NotePropertyName first_task_pack_items -NotePropertyValue "### task_1`n- title: 锁定首轮实施范围与真源`n- owner_role: docs`n- support_roles: reviewer`n- depends_on: docs/project_context.md`n- done_signal: 真源与范围已收口，尚未声称业务实现完成" -Force
    $Values | Add-Member -NotePropertyName first_task_pack_gate_note -NotePropertyValue '进入 review 前先补齐证据采集计划；业务证据需等后续实施后再补' -Force
    $Values | Add-Member -NotePropertyName task_id -NotePropertyValue 'first-sprint' -Force
    $Values | Add-Member -NotePropertyName implementer_role -NotePropertyValue 'frontend' -Force
    $Values | Add-Member -NotePropertyName evaluator_role -NotePropertyValue 'reviewer' -Force
    $Values | Add-Member -NotePropertyName risk_gate -NotePropertyValue 'low' -Force
    $Values | Add-Member -NotePropertyName requirement_summary -NotePropertyValue '- 完成 release smoke 的初始化协作包收口' -Force
    $Values | Add-Member -NotePropertyName criteria_1 -NotePropertyValue '初始化协作包通过 postcheck，不声称业务代码已完成' -Force
    $Values | Add-Member -NotePropertyName verify_method_1 -NotePropertyValue '检查 kickoff docs、postcheck 与 truth-source gate' -Force
    $Values | Add-Member -NotePropertyName deliverable_ref_1 -NotePropertyValue 'docs/workflow/implementation-kickoff.md' -Force
}

function New-BootstrapEnvironment {
    param(
        [string]$Root,
        [string]$SampleValuesPath,
        [string[]]$EnabledRoles
    )

    $sessionRoot = Join-Path $Root '.commonhe\session'
    New-Item -ItemType Directory -Path $sessionRoot -Force | Out-Null

    $values = Get-Content -Raw $SampleValuesPath | ConvertFrom-Json
    $values.project_name = 'ReleaseSmoke'
    $values.project_type = 'web-app'
    $values.enabled_roles = (($EnabledRoles | ForEach-Object { "- $_" }) -join "`n")
    $values.roles_and_manuals = $values.enabled_roles
    $values.agent_dispatch_matrix = $values.enabled_roles
    $values.current_goals = '- 完成发布包 smoke test'
    $values.project_goal = '完成发布包 smoke test'
    $values.project_goal_summary = '完成发布包 smoke test'
    $values.tech_stack = 'PowerShell'
    Set-TestCapabilityDefaults -Values $values
    $valuesPath = Join-Path $Root 'values.json'
    $values | ConvertTo-Json -Depth 20 | Set-Content -Path $valuesPath

    $requiredCapabilities = @(
        @{ name = 'superpowers'; display_name = 'superpowers' }
        @{ name = 'agent-browser'; display_name = 'agent-browser' }
        @{ name = 'chrome-devtools'; display_name = 'chrome-devtools' }
        @{ name = 'GitNexus'; display_name = 'GitNexus' }
        @{ name = 'Speckit'; display_name = 'Speckit' }
    )
    $selectedCapabilities = @(
        foreach ($capability in $requiredCapabilities) {
            @{
                id = $capability.name
                label = $capability.display_name
                selected = $true
                recommended = $true
                status = 'pass'
                detail = 'release smoke seed'
            }
        }
    )

    $decision = @{
        user_confirmed = $true
        project_name = 'ReleaseSmoke'
        project_type = 'web-app'
        solution_mode = 'balanced'
        enabled_roles = $EnabledRoles
        integrations = @()
        discovery_mode = 'legacy_zero_question'
        auto_confirmed = $true
        confirmation_mode = 'auto_legacy_init'
        selected_solution_id = 'B'
        selected_solution_title = 'Release smoke balanced package'
        solution_architecture_summary = 'Release smoke validates portable collaboration package generation.'
        selected_capabilities = $selectedCapabilities
        required_capabilities = $requiredCapabilities
        capability_probe_results = @(
            foreach ($capability in $requiredCapabilities) {
                @{
                    name = $capability.name
                    display_name = $capability.display_name
                    passed = $true
                    evidence = 'release smoke seed'
                }
            }
        )
        legacy_analysis_version = 'v2'
        signal_categories = @('frontend')
        role_rationale = @{ frontend = 'release smoke' }
        omitted_role_rationale = @{ qa = 'release smoke keeps QA deferred unless selected' }
        confidence_breakdown = @{ signals = 'high' }
        dominant_workstreams = @('frontend')
        kickoff_pack = @{
            implementation_kickoff = 'docs/workflow/implementation-kickoff.md'
            first_sprint_contract = 'docs/workflow/first-sprint-contract.md'
            first_task_pack = 'docs/workflow/first-task-pack.md'
        }
        analysis_confidence = 'high'
        autodiscovery_signals = @('release-smoke')
        autodiscovery_assumptions = @()
    }
    $decision | ConvertTo-Json -Depth 20 | Set-Content -Path (Join-Path $sessionRoot 'decision.json')

    $status = @{
        stage = 'confirmed'
        current_question_index = $null
        session_root = [System.IO.Path]::GetFullPath($sessionRoot)
        started_at = '2026-04-22T10:00:00'
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
        legacy_project_detected = $true
        legacy_zero_question_mode = $true
        missing_capabilities = @()
        capability_gate_passed = $true
        question_source = 'desktop-agent'
    }
    $status | ConvertTo-Json -Depth 20 | Set-Content -Path (Join-Path $sessionRoot 'status.json')

    @{
        Root = $Root
        SessionRoot = $sessionRoot
        ValuesPath = $valuesPath
    }
}

function Assert-WorkflowOutputs {
    param([string]$ProjectRoot)

    foreach ($requiredPath in @(
        'docs/workflow/evaluator-protocol.md',
        'docs/workflow/grading-criteria.md',
        'docs/workflow/sprint-contract-template.md',
        'docs/workflow/implementation-kickoff.md',
        'docs/workflow/first-sprint-contract.md',
        'docs/workflow/first-task-pack.md'
    )) {
        Assert-True -Condition (Test-Path (Join-Path $ProjectRoot $requiredPath)) -Message "Missing workflow artifact: $requiredPath"
    }

    $checklist = Get-Content -Raw (Join-Path $ProjectRoot 'docs\workflow\current-stage-user-checklist.md')
    Assert-TextNotContains -Text $checklist -UnexpectedSubstring '当前初始化线程只负责补齐协作工程' -Message 'Implementation checklist should not leak init-only guidance.'
    Assert-TextNotContains -Text $checklist -UnexpectedSubstring '初始化收口清单' -Message 'Implementation checklist should not contain init closure sections.'

    $agentsDoc = Get-Content -Raw (Join-Path $ProjectRoot 'AGENTS.md')
    $coordinatorDoc = Get-Content -Raw (Join-Path $ProjectRoot '.codex\COORDINATOR-SUBAGENTS.md')
    $indexDoc = Get-Content -Raw (Join-Path $ProjectRoot 'docs\00-初始化结果索引.md')
    Assert-TextContains -Text $agentsDoc -ExpectedSubstring 'docs/workflow/implementation-kickoff.md' -Message 'AGENTS should reference implementation kickoff doc.'
    Assert-TextContains -Text $coordinatorDoc -ExpectedSubstring 'docs/workflow/implementation-kickoff.md' -Message 'Coordinator should reference implementation kickoff doc.'
    Assert-TextContains -Text $indexDoc -ExpectedSubstring 'docs/workflow/implementation-kickoff.md' -Message 'Init result index should reference implementation kickoff doc.'
}

$originalCapabilityCatalogPath = if (Test-Path Env:COMMONHE_REQUIRED_CAPABILITIES_PATH) {
    (Get-Item Env:COMMONHE_REQUIRED_CAPABILITIES_PATH).Value
} else {
    $null
}

try {
    Assert-True -Condition (Test-Path $ReleaseZipPath) -Message "Release zip not found: $ReleaseZipPath"

    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $archive = [System.IO.Compression.ZipFile]::OpenRead($ReleaseZipPath)
    try {
        $entries = @($archive.Entries | ForEach-Object { ([string]$_.FullName).Replace('\', '/') })
    } finally {
        $archive.Dispose()
    }

    foreach ($requiredEntry in @(
        'commonhe-desktop.exe',
        'resources/commonhe/config/required-capabilities.json',
        'resources/commonhe/config/commonhe-truth-source-gates.json',
        'resources/commonhe/specs/004-dual-agent-semantic-review/plan.md',
        'resources/commonhe/.specify/templates/spec-template.md',
        'resources/commonhe/agency-agents-zh/README.md',
        'resources/commonhe/references/product-manager.md',
        'resources/commonhe/references/agency-agents-zh-README.md',
        'resources/commonhe/templates/workflow/evaluator-protocol.template.md',
        'resources/commonhe/templates/workflow/grading-criteria.template.md',
        'resources/commonhe/templates/workflow/sprint-contract.template.md',
        'resources/commonhe/templates/workflow/implementation-kickoff.template.md',
        'resources/commonhe/templates/workflow/first-task-pack.template.md',
        'resources/commonhe/tools/common-he-init-orchestrator.ps1',
        'resources/commonhe/tools/init-common-he.ps1',
        'resources/commonhe/tools/assert-commonhe-truth-source.ps1'
    )) {
        Assert-Contains -Entries $entries -ExpectedEntry $requiredEntry -Message 'Release zip is missing a required entry.'
    }

    New-Item -ItemType Directory -Path $tempRoot -Force | Out-Null
    $packageRoot = Join-Path $tempRoot 'package'
    Expand-Archive -Path $ReleaseZipPath -DestinationPath $packageRoot -Force
    $commonheRoot = Join-Path $packageRoot 'resources\commonhe'
    Assert-True -Condition (Test-Path $commonheRoot) -Message "Portable resources root not found: $commonheRoot"

    $capabilityCatalogPath = New-CapabilityCatalog -Root (Join-Path $tempRoot 'capability-catalog')
    $env:COMMONHE_REQUIRED_CAPABILITIES_PATH = $capabilityCatalogPath
    $env:COMMONHE_HOME_OVERRIDE = (Join-Path $tempRoot 'capability-catalog')

    $doctorProject = Join-Path $tempRoot 'doctor-project'
    New-Item -ItemType Directory -Path $doctorProject -Force | Out-Null
    $doctorResult = & (Join-Path $commonheRoot 'tools\common-he-init-orchestrator.ps1') -Stage doctor -ProjectRoot $doctorProject
    Assert-Equal -Actual $doctorResult.Stage -Expected 'doctor' -Message 'Doctor smoke should pass from the release package.'

    $legacyRoot = Join-Path $tempRoot 'legacy-project'
    New-Item -ItemType Directory -Path $legacyRoot -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $legacyRoot 'src') -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $legacyRoot 'tests') -Force | Out-Null
    Set-Content -Path (Join-Path $legacyRoot 'package.json') -Value '{ "name": "legacy-project", "private": true, "scripts": { "test": "vitest" } }'
    Set-Content -Path (Join-Path $legacyRoot 'src\main.ts') -Value 'console.log("legacy smoke");'
    Set-Content -Path (Join-Path $legacyRoot 'tests\main.test.ts') -Value 'test("legacy", () => {});'

    $legacyResult = & (Join-Path $commonheRoot 'tools\common-he-init-orchestrator.ps1') -Stage start -ProjectRoot $legacyRoot
    Assert-Equal -Actual $legacyResult.Stage -Expected 'implementation_ready' -Message 'Legacy release smoke should auto-bootstrap to implementation_ready.'
    Assert-WorkflowOutputs -ProjectRoot $legacyRoot

    $legacyStatus = Get-Content -Raw (Join-Path $legacyRoot '.commonhe\session\status.json') | ConvertFrom-Json
    Assert-Equal -Actual $legacyStatus.stage -Expected 'implementation_ready' -Message 'Legacy release smoke should persist implementation_ready.'
    Assert-True -Condition ([bool]$legacyStatus.init_closed) -Message 'Legacy release smoke should close init context.'
    Assert-True -Condition ([bool]$legacyStatus.doctor_passed) -Message 'Legacy release smoke should persist doctor_passed.'

    $bootstrapEnv = New-BootstrapEnvironment `
        -Root (Join-Path $tempRoot 'bootstrap-project') `
        -SampleValuesPath (Join-Path $commonheRoot 'config\init-values.sample.json') `
        -EnabledRoles @('frontend', 'reviewer', 'docs')

    $bootstrapResult = & (Join-Path $commonheRoot 'tools\common-he-init-orchestrator.ps1') `
        -Stage bootstrap `
        -SessionRoot $bootstrapEnv.SessionRoot `
        -TargetRoot $bootstrapEnv.Root `
        -ValuesPath $bootstrapEnv.ValuesPath `
        -Execute `
        -Force

    Assert-Equal -Actual $bootstrapResult.Stage -Expected 'implementation_ready' -Message 'Direct bootstrap smoke should complete successfully from the release package.'
    Assert-WorkflowOutputs -ProjectRoot $bootstrapEnv.Root

    Write-Host "PASS release zip $ReleaseName contains required portable files and passes doctor/legacy/direct-bootstrap smoke tests"
} finally {
    if ($null -ne $originalCapabilityCatalogPath) {
        $env:COMMONHE_REQUIRED_CAPABILITIES_PATH = $originalCapabilityCatalogPath
    } else {
        Remove-Item Env:COMMONHE_REQUIRED_CAPABILITIES_PATH -ErrorAction SilentlyContinue
    }
    Remove-Item Env:COMMONHE_HOME_OVERRIDE -ErrorAction SilentlyContinue

    if (Test-Path $tempRoot) {
        Remove-Item -LiteralPath $tempRoot -Recurse -Force
    }
}
