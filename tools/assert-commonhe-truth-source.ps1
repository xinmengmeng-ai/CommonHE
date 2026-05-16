param(
    [string]$RepoRoot,
    [string]$GeneratedRoot,
    [ValidateSet('codex', 'claude-code')]
    [string]$TargetClient,
    [switch]$AsJson
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Normalize-FullPath {
    param([string]$Path)
    ([System.IO.Path]::GetFullPath($Path)).TrimEnd('\', '/')
}

function Add-Issue {
    param(
        [System.Collections.ArrayList]$Issues,
        [string]$RuleId,
        [string]$Message
    )

    [void]$Issues.Add("${RuleId}: $Message")
}

function Test-PathRelative {
    param(
        [string]$Root,
        [string]$RelativePath,
        [string]$PathType = 'Any'
    )

    $path = Join-Path $Root $RelativePath
    if ($PathType -eq 'Leaf') {
        return Test-Path -LiteralPath $path -PathType Leaf
    }
    if ($PathType -eq 'Container') {
        return Test-Path -LiteralPath $path -PathType Container
    }
    Test-Path -LiteralPath $path
}

function Get-TextIfExists {
    param([string]$Path)
    if (Test-Path -LiteralPath $Path -PathType Leaf) {
        $content = Get-Content -LiteralPath $Path -Raw
        if ($null -eq $content) { return '' }
        return [string]$content
    }
    ''
}

function Assert-FileContains {
    param(
        [System.Collections.ArrayList]$Issues,
        [string]$RuleId,
        [string]$Path,
        [string[]]$Needles
    )

    $text = Get-TextIfExists -Path $Path
    foreach ($needle in $Needles) {
        if (-not $text.Contains($needle)) {
            Add-Issue -Issues $Issues -RuleId $RuleId -Message "$Path must contain '$needle'."
        }
    }
}

function Get-CapabilityIdentity {
    param($Capability)

    if ($null -eq $Capability) { return '' }
    if ($Capability -is [string]) { return ([string]$Capability).Trim() }
    foreach ($propertyName in @('id', 'name', 'display_name', 'label')) {
        if ($Capability.PSObject.Properties.Match($propertyName).Count -gt 0) {
            $value = ([string]$Capability.$propertyName).Trim()
            if ($value) { return $value }
        }
    }
    ''
}

function Test-CapabilitySelected {
    param($Capability)

    if ($null -eq $Capability) { return $false }
    if ($Capability -is [string]) { return $true }
    if ($Capability.PSObject.Properties.Match('selected').Count -gt 0) {
        return [bool]$Capability.selected
    }
    $true
}

function Get-DecisionRoleNames {
    param($Decision)

    if ($null -eq $Decision) { return @() }

    $roles = New-Object System.Collections.ArrayList
    foreach ($propertyName in @('enabled_roles', 'recommended_roles_now')) {
        if ($Decision.PSObject.Properties.Match($propertyName).Count -eq 0) {
            continue
        }
        foreach ($role in @($Decision.$propertyName)) {
            $roleName = ([string]$role).Trim()
            if ($roleName) {
                [void]$roles.Add($roleName)
            }
        }
    }

    @($roles | Sort-Object -Unique)
}

function Add-DuplicateMarkdownSectionNumberIssues {
    param(
        [System.Collections.ArrayList]$Issues,
        [string]$Root,
        [string[]]$RelativePaths
    )

    foreach ($relativePath in $RelativePaths) {
        $fullPath = Join-Path $Root $relativePath
        if (-not (Test-Path -LiteralPath $fullPath -PathType Leaf)) {
            continue
        }
        $content = Get-TextIfExists -Path $fullPath
        $seen = @{}
        foreach ($match in [regex]::Matches($content, '(?m)^##\s+([0-9]+)\.')) {
            $number = [string]$match.Groups[1].Value
            if ($seen.ContainsKey($number)) {
                Add-Issue -Issues $Issues -RuleId 'generated.duplicate_heading_number' -Message "$relativePath has duplicate markdown section number ## $number."
            } else {
                $seen[$number] = $true
            }
        }
    }
}

function Add-DanglingRoleReferenceIssues {
    param(
        [System.Collections.ArrayList]$Issues,
        [string]$Root,
        [object]$Decision,
        [string[]]$RelativePaths
    )

    $enabledRoles = @(Get-DecisionRoleNames -Decision $Decision)
    if ($enabledRoles.Count -eq 0) {
        return
    }

    foreach ($roleName in @('database', 'devops', 'compliance')) {
        if ($enabledRoles -contains $roleName) {
            continue
        }
        foreach ($relativePath in $RelativePaths) {
            $fullPath = Join-Path $Root $relativePath
            if (-not (Test-Path -LiteralPath $fullPath -PathType Leaf)) {
                continue
            }
            $content = Get-TextIfExists -Path $fullPath
            foreach ($pattern in @("+ $roleName", "-> $roleName", "/ $roleName", "通知 $roleName")) {
                if ($content.Contains($pattern)) {
                    Add-Issue -Issues $Issues -RuleId 'generated.dangling_role_reference' -Message "$relativePath references disabled role '$roleName' via '$pattern'."
                    break
                }
            }
        }
    }
}

function Add-WorkflowHandoffIssues {
    param(
        [System.Collections.ArrayList]$Issues,
        [string]$Root,
        [object]$Status
    )

    $isImplementationReady = $false
    if ($Status -and $Status.PSObject.Properties.Match('stage').Count -gt 0) {
        $isImplementationReady = ([string]$Status.stage -eq 'implementation_ready')
    }

    $checklistPath = Join-Path $Root 'docs\workflow\current-stage-user-checklist.md'
    if ($isImplementationReady -and (Test-Path -LiteralPath $checklistPath -PathType Leaf)) {
        $checklist = Get-TextIfExists -Path $checklistPath
        foreach ($pattern in @('初始化协作包已落盘', '先完成初始化落盘', '再进入实施线程', 'postcheck', 'bootstrap', '初始化落盘', '初始化线程')) {
            if ($checklist.Contains($pattern)) {
                Add-Issue -Issues $Issues -RuleId 'generated.stale_handoff_checklist' -Message "docs/workflow/current-stage-user-checklist.md contains stale init-only item '$pattern'."
            }
        }
    }

    $firstSprintContractPath = Join-Path $Root 'docs\workflow\first-sprint-contract.md'
    $sprintContractTemplatePath = Join-Path $Root 'docs\workflow\sprint-contract-template.md'
    if ((Test-Path -LiteralPath $firstSprintContractPath -PathType Leaf) -and (Test-Path -LiteralPath $sprintContractTemplatePath -PathType Leaf)) {
        $firstSprintContract = (Get-TextIfExists -Path $firstSprintContractPath).Trim()
        $sprintContractTemplate = (Get-TextIfExists -Path $sprintContractTemplatePath).Trim()
        if ($firstSprintContract -and $firstSprintContract -eq $sprintContractTemplate) {
            Add-Issue -Issues $Issues -RuleId 'generated.sprint_contract_instance' -Message 'docs/workflow/first-sprint-contract.md must be a concrete first-sprint instance, not identical to docs/workflow/sprint-contract-template.md.'
        }
        foreach ($pattern in @('Sprint Contract 模板', '使用前请先复制', '任务级 Contract 脚手架', '不代表当前项目已经存在一个已签署', '待填写')) {
            if ($firstSprintContract.Contains($pattern)) {
                Add-Issue -Issues $Issues -RuleId 'generated.sprint_contract_instance' -Message "docs/workflow/first-sprint-contract.md contains template residue '$pattern'."
            }
        }
        foreach ($requiredPhrase in @('首轮实施合同', '第一优先工作流')) {
            if (-not $firstSprintContract.Contains($requiredPhrase)) {
                Add-Issue -Issues $Issues -RuleId 'generated.sprint_contract_instance' -Message "docs/workflow/first-sprint-contract.md must contain '$requiredPhrase'."
            }
        }
    }
}

function Test-GeneratedCapabilityEnabled {
    param(
        [string]$Root,
        [string]$CapabilityName
    )

    $decisionPath = Join-Path $Root '.commonhe\session\decision.json'
    if (-not (Test-Path -LiteralPath $decisionPath -PathType Leaf)) {
        return $true
    }

    $decision = Get-Content -LiteralPath $decisionPath -Raw | ConvertFrom-Json
    $selectedCapabilities = @()
    if ($decision.PSObject.Properties.Match('selected_capabilities').Count -gt 0) {
        $selectedCapabilities = @(foreach ($item in $decision.selected_capabilities) { $item })
    }
    if ($selectedCapabilities.Count -gt 0) {
        foreach ($capability in $selectedCapabilities) {
            $name = Get-CapabilityIdentity -Capability $capability
            if ($name -and $name.Equals($CapabilityName, [System.StringComparison]::OrdinalIgnoreCase)) {
                return (Test-CapabilitySelected -Capability $capability)
            }
        }
        return $false
    }

    $false
}

function Get-Manifest {
    param([string]$Root)

    $manifestPath = Join-Path $Root 'config\commonhe-truth-source-gates.json'
    if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
        throw "Truth-source gate manifest is missing: $manifestPath"
    }
    Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
}

function Test-RepoTruthSource {
    param([string]$Root)

    $root = Normalize-FullPath $Root
    $manifest = Get-Manifest -Root $root
    $issues = New-Object System.Collections.ArrayList

    foreach ($relativePath in @('docs\CommonHE开发目标.md', 'AGENTS.md', 'product-manager.md', 'init\init-flow.md')) {
        if (-not (Test-PathRelative -Root $root -RelativePath $relativePath -PathType Leaf)) {
            Add-Issue -Issues $issues -RuleId 'repo.required_truth_source' -Message "Missing required truth-source file: $relativePath."
        }
    }

    Assert-FileContains -Issues $issues -RuleId 'repo.product_name' -Path (Join-Path $root 'docs\CommonHE开发目标.md') -Needles @([string]$manifest.product_name)
    Assert-FileContains -Issues $issues -RuleId 'repo.speckit_marker' -Path (Join-Path $root 'AGENTS.md') -Needles @('specs/004-dual-agent-semantic-review/plan.md', 'specs/003-truth-source-acceptance-gates/plan.md', 'specs/002-client-targeted-init-package/plan.md')
    Assert-FileContains -Issues $issues -RuleId 'repo.generated_package_review_contract' -Path (Join-Path $root 'specs\004-dual-agent-semantic-review\contracts\generated-package-review.md') -Needles @(
        '星梦梦语义审查',
        'truth-source gate 硬门禁',
        'sanitizer',
        '重复编号',
        '未启用角色',
        '临时补救命令',
        'selected_capabilities',
        'first-sprint-contract.md',
        'sprint-contract-template.md'
    )

    $appText = Get-TextIfExists -Path (Join-Path $root 'apps\desktop\src\App.tsx')
    foreach ($needle in @('availableModels.length > 0', '<select value={selectedModel}', 'useCustomModel', '使用自定义模型', 'setSelectedProviderId("deepseek")', 'solutionDialogStep', 'projectName', 'targetClient', 'selectedCapabilities', 'chooseAgentSolution')) {
        if (-not $appText.Contains($needle)) {
            Add-Issue -Issues $issues -RuleId 'repo.provider_model_controlled' -Message "Desktop App.tsx must preserve controlled provider/model and post-solution confirmation flow marker: $needle."
        }
    }

    $providerText = Get-TextIfExists -Path (Join-Path $root 'apps\desktop\src-tauri\src\commonhe_bridge\provider.rs')
    foreach ($needle in @('deepseek-v4-flash', 'deepseek-v4-pro', 'requires_api_key', 'custom_provider_rejects_invalid_base_url', 'base_url_invalid', 'api_key_required', 'provider_connectivity_failed')) {
        if (-not $providerText.Contains($needle)) {
            Add-Issue -Issues $issues -RuleId 'repo.deepseek_custom_validation' -Message "provider.rs must preserve DeepSeek/custom validation marker: $needle."
        }
    }

    $agentText = Get-TextIfExists -Path (Join-Path $root 'apps\desktop\src-tauri\src\commonhe_bridge\agent.rs')
    foreach ($needle in @('product-manager.md', 'agency-agents-zh', 'open_solution_selector', 'architecture_summary', 'team_composition', 'token_estimate', 'role_rationale', 'omitted_role_rationale', '星梦梦', 'SemanticReviewResult', 'AgentDialogueRound', 'request_semantic_review')) {
        if (-not $agentText.Contains($needle)) {
            Add-Issue -Issues $issues -RuleId 'repo.main_agent_contract' -Message "agent.rs must preserve main-agent contract marker: $needle."
        }
    }

    $syncText = Get-TextIfExists -Path (Join-Path $root 'scripts\sync-desktop-resources.ps1')
    foreach ($needle in @('.specify', 'agency-agents-zh', 'config', 'tools')) {
        if (-not $syncText.Contains($needle)) {
            Add-Issue -Issues $issues -RuleId 'repo.resources_packaged' -Message "Resource sync script must include marker: $needle."
        }
    }

    foreach ($relativePath in @(
        'config\commonhe-truth-source-gates.json',
        'tools\assert-commonhe-truth-source.ps1',
        '.specify\templates\spec-template.md',
        '.specify\extensions\git\scripts\powershell\create-new-feature.ps1',
        'agency-agents-zh\README.md',
        'agency-agents-zh\engineering\engineering-frontend-developer.md'
    )) {
        if (-not (Test-PathRelative -Root $root -RelativePath $relativePath)) {
            Add-Issue -Issues $issues -RuleId 'repo.resources_packaged' -Message "Repository is missing required packaged resource: $relativePath."
        }
    }

    $agentCount = 0
    $agentRoot = Join-Path $root 'agency-agents-zh'
    if (Test-Path -LiteralPath $agentRoot -PathType Container) {
        $agentCount = @(Get-ChildItem -LiteralPath $agentRoot -Recurse -File -Filter '*.md').Count
    }
    if ($agentCount -lt 200) {
        Add-Issue -Issues $issues -RuleId 'repo.agency_agents_library' -Message "agency-agents-zh must contain the real 200+ agent library; found $agentCount markdown files."
    }

    [pscustomobject]@{
        Passed = ($issues.Count -eq 0)
        Mode = 'repo'
        TargetClient = $null
        Issues = @($issues)
    }
}

function Test-GeneratedTruthSource {
    param(
        [string]$Root,
        [string]$Client
    )

    $root = Normalize-FullPath $Root
    $manifestRoot = if ($RepoRoot) { Normalize-FullPath $RepoRoot } else { Split-Path -Parent $PSScriptRoot }
    $manifestRoot = Split-Path -Parent $PSScriptRoot
    $manifest = Get-Manifest -Root $manifestRoot
    $issues = New-Object System.Collections.ArrayList

    if (-not (Test-Path -LiteralPath $root -PathType Container)) {
        Add-Issue -Issues $issues -RuleId 'generated.root' -Message "Generated root does not exist: $root."
    }

    foreach ($relativePath in @('docs', '.commonhe\session')) {
        if (-not (Test-PathRelative -Root $root -RelativePath $relativePath -PathType Container)) {
            Add-Issue -Issues $issues -RuleId 'generated.commonhe_state' -Message "Missing required initialization collaboration package directory: $relativePath."
        }
    }

    foreach ($relativePath in @('.specify', '.specify\templates\spec-template.md', '.specify\scripts\powershell\create-new-feature.ps1')) {
        $pathType = if ($relativePath -eq '.specify') { 'Container' } else { 'Leaf' }
        if (-not (Test-PathRelative -Root $root -RelativePath $relativePath -PathType $pathType)) {
            Add-Issue -Issues $issues -RuleId 'generated.commonhe_state' -Message "Missing required Speckit file: $relativePath."
        }
    }

    foreach ($capabilityName in @('superpowers', 'agent-browser', 'chrome-devtools', 'GitNexus', 'Speckit')) {
        if (-not (Test-GeneratedCapabilityEnabled -Root $root -CapabilityName $capabilityName)) {
            Add-Issue -Issues $issues -RuleId 'generated.mandatory_capability' -Message "Mandatory capability is not selected or required: $capabilityName."
        }
    }

    $decisionPath = Join-Path $root '.commonhe\session\decision.json'
    $statusPath = Join-Path $root '.commonhe\session\status.json'
    $isCompatibilityPath = $false
    $isDesktopAgentPath = $false
    $decisionForMode = $null
    $statusForMode = $null
    if (Test-Path -LiteralPath $decisionPath -PathType Leaf) {
        try {
            $decisionForMode = Get-Content -LiteralPath $decisionPath -Raw | ConvertFrom-Json
            $decisionCompatibilityPath = ($decisionForMode.PSObject.Properties.Match('compatibility_path').Count -gt 0 -and [bool]$decisionForMode.compatibility_path)
            if ([string]$decisionForMode.discovery_mode -eq 'legacy_zero_question' -or $decisionCompatibilityPath) {
                $isCompatibilityPath = $true
            }
        }
        catch {}
    }
    if (Test-Path -LiteralPath $statusPath -PathType Leaf) {
        try {
            $statusForMode = Get-Content -LiteralPath $statusPath -Raw | ConvertFrom-Json
            $statusLegacyZeroQuestion = ($statusForMode.PSObject.Properties.Match('legacy_zero_question_mode').Count -gt 0 -and [bool]$statusForMode.legacy_zero_question_mode)
            $statusSemanticCompatibility = ($statusForMode.PSObject.Properties.Match('semantic_review_compatibility_path').Count -gt 0 -and [bool]$statusForMode.semantic_review_compatibility_path)
            if ($statusLegacyZeroQuestion -or $statusSemanticCompatibility) {
                $isCompatibilityPath = $true
            }
            if ([string]$statusForMode.question_source -eq 'desktop-agent') {
                $isDesktopAgentPath = $true
            }
        }
        catch {}
    }

    if ($decisionForMode) {
        $selectedCapabilities = @()
        if ($decisionForMode.PSObject.Properties.Match('selected_capabilities').Count -gt 0) {
            $selectedCapabilities = @(foreach ($item in $decisionForMode.selected_capabilities) { $item })
        }
        if ($selectedCapabilities.Count -eq 0) {
            Add-Issue -Issues $issues -RuleId 'generated.selected_capabilities' -Message 'decision.selected_capabilities must include the five selected mandatory capabilities.'
        }
    }

    if ((-not $isCompatibilityPath) -and $isDesktopAgentPath) {
        if (-not $statusForMode -or $statusForMode.PSObject.Properties.Match('capability_gate_passed').Count -eq 0 -or -not [bool]$statusForMode.capability_gate_passed) {
            Add-Issue -Issues $issues -RuleId 'generated.capability_gate_passed' -Message 'desktop-agent status.capability_gate_passed must be true before the generated package can pass.'
        }

        foreach ($relativePath in @(
            '.commonhe\session\meng-xingxing-output.json',
            '.commonhe\session\xing-mengmeng-review.json',
            '.commonhe\session\agent-dialogue-rounds.jsonl',
            '.commonhe\session\repair-decisions.json',
            '.commonhe\session\final-acceptance.json'
        )) {
            if (-not (Test-PathRelative -Root $root -RelativePath $relativePath -PathType Leaf)) {
                Add-Issue -Issues $issues -RuleId 'generated.semantic_acceptance' -Message "Missing dual-agent semantic acceptance artifact: $relativePath."
            }
        }

        $finalAcceptancePath = Join-Path $root '.commonhe\session\final-acceptance.json'
        $finalAcceptance = $null
        if (Test-Path -LiteralPath $finalAcceptancePath -PathType Leaf) {
            try {
                $finalAcceptance = Get-Content -LiteralPath $finalAcceptancePath -Raw | ConvertFrom-Json
                if (-not [bool]$finalAcceptance.passed) {
                    Add-Issue -Issues $issues -RuleId 'generated.semantic_acceptance' -Message 'final-acceptance.json must have passed=true.'
                }
                $finalBlockingIssues = @()
                if ($finalAcceptance.PSObject.Properties.Match('blockingIssues').Count -gt 0) {
                    if ($null -eq $finalAcceptance.blockingIssues) {
                        $finalBlockingIssues = @()
                    } elseif ($finalAcceptance.blockingIssues -is [array]) {
                        $finalBlockingIssues = @($finalAcceptance.blockingIssues)
                    } else {
                        $finalBlockingIssues = @($finalAcceptance.blockingIssues)
                    }
                }
                if ([bool]$finalAcceptance.passed -and $finalBlockingIssues.Count -gt 0) {
                    Add-Issue -Issues $issues -RuleId 'generated.semantic_acceptance' -Message 'final-acceptance.json passed=true must not contain blockingIssues.'
                }
                if ([string]$finalAcceptance.reviewerAgent -ne '星梦梦') {
                    Add-Issue -Issues $issues -RuleId 'generated.semantic_acceptance' -Message 'final-acceptance.json reviewerAgent must be 星梦梦.'
                }
                if ([string]$finalAcceptance.mainAgent -ne '梦星星') {
                    Add-Issue -Issues $issues -RuleId 'generated.semantic_acceptance' -Message 'final-acceptance.json mainAgent must be 梦星星.'
                }
                if ([string]$finalAcceptance.targetClient -ne $Client) {
                    Add-Issue -Issues $issues -RuleId 'generated.semantic_acceptance' -Message "final-acceptance.json targetClient must be $Client."
                }
            }
            catch {
                Add-Issue -Issues $issues -RuleId 'generated.semantic_acceptance' -Message 'final-acceptance.json must be valid JSON.'
            }
        }

        $mengOutputPath = Join-Path $root '.commonhe\session\meng-xingxing-output.json'
        $mengOutput = $null
        if (Test-Path -LiteralPath $mengOutputPath -PathType Leaf) {
            try {
                $mengOutput = Get-Content -LiteralPath $mengOutputPath -Raw | ConvertFrom-Json
            } catch {
                Add-Issue -Issues $issues -RuleId 'generated.semantic_acceptance' -Message 'meng-xingxing-output.json must be valid JSON.'
            }
        }

        $decisionSelectedSolutionId = if ($decisionForMode -and $decisionForMode.PSObject.Properties.Match('selected_solution_id').Count -gt 0) { [string]$decisionForMode.selected_solution_id } else { '' }
        if (-not $decisionSelectedSolutionId) {
            Add-Issue -Issues $issues -RuleId 'generated.selected_solution_id' -Message 'decision.selected_solution_id must record the user-selected solution.'
        }
        if ($finalAcceptance -and $decisionSelectedSolutionId -and [string]$finalAcceptance.selectedSolutionId -ne $decisionSelectedSolutionId) {
            Add-Issue -Issues $issues -RuleId 'generated.selected_solution_id' -Message 'decision.selected_solution_id must match final-acceptance.json selectedSolutionId.'
        }
        if ($mengOutput -and $decisionSelectedSolutionId -and [string]$mengOutput.selectedSolutionId -ne $decisionSelectedSolutionId) {
            Add-Issue -Issues $issues -RuleId 'generated.selected_solution_id' -Message 'decision.selected_solution_id must match meng-xingxing-output.json selectedSolutionId.'
        }
    }

    if ($Client -eq 'claude-code') {
        foreach ($relativePath in @('CLAUDE.md', '.claude')) {
            if (-not (Test-PathRelative -Root $root -RelativePath $relativePath)) {
                Add-Issue -Issues $issues -RuleId 'generated.claude_entry' -Message "Claude Code target is missing $relativePath."
            }
        }
        if (Test-PathRelative -Root $root -RelativePath 'AGENTS.md' -PathType Leaf) {
            Add-Issue -Issues $issues -RuleId 'generated.claude_entry' -Message 'Claude Code target must not generate AGENTS.md.'
        }
        $agentDir = Join-Path $root '.claude\agents'
    } else {
        foreach ($relativePath in @('AGENTS.md', '.codex', '.agents\skills')) {
            if (-not (Test-PathRelative -Root $root -RelativePath $relativePath)) {
                Add-Issue -Issues $issues -RuleId 'generated.codex_entry' -Message "Codex target is missing $relativePath."
            }
        }
        if (Test-PathRelative -Root $root -RelativePath 'CLAUDE.md' -PathType Leaf) {
            Add-Issue -Issues $issues -RuleId 'generated.codex_entry' -Message 'Codex target must not generate CLAUDE.md.'
        }
        $agentDir = Join-Path $root '.codex\agents'
    }

    if ($Client -eq 'codex') {
        Add-DuplicateMarkdownSectionNumberIssues -Issues $issues -Root $root -RelativePaths @('AGENTS.md')
        Add-DanglingRoleReferenceIssues -Issues $issues -Root $root -Decision $decisionForMode -RelativePaths @('AGENTS.md', '.codex\COORDINATOR-SUBAGENTS.md')
    }
    Add-WorkflowHandoffIssues -Issues $issues -Root $root -Status $statusForMode

    if (-not (Test-Path -LiteralPath $agentDir -PathType Container)) {
        Add-Issue -Issues $issues -RuleId 'generated.agency_agents' -Message "Missing target client agent directory: $agentDir."
    } else {
        $agentFiles = @(Get-ChildItem -LiteralPath $agentDir -File -Filter '*.md')
        if ($agentFiles.Count -eq 0) {
            Add-Issue -Issues $issues -RuleId 'generated.agency_agents' -Message "No target client agent files generated under $agentDir."
        }
        foreach ($file in $agentFiles) {
            if ($manifest.placeholder_agent_files -contains $file.Name) {
                Add-Issue -Issues $issues -RuleId 'generated.agency_agents' -Message "Placeholder agent file is not allowed: $($file.Name)."
            }
            $content = Get-Content -LiteralPath $file.FullName -Raw
            if ($null -eq $content) { $content = '' }
            if (-not $content.Contains('来源：agency-agents-zh')) {
                Add-Issue -Issues $issues -RuleId 'generated.agency_agents' -Message "Agent file must preserve agency-agents-zh provenance: $($file.Name)."
            }
        }
    }

    $allTextFiles = @()
    $textFiles = @()
    if (Test-Path -LiteralPath $root -PathType Container) {
        $allTextFiles = @(
            Get-ChildItem -LiteralPath $root -Recurse -File -Include '*.md', '*.json', '*.jsonl', '*.toml', '*.txt' |
                Where-Object {
                    $_.FullName -notmatch '\\\.specify\\'
                }
        )
        $textFiles = @(
            $allTextFiles |
                Where-Object {
                    $_.FullName -notmatch '\\\.commonhe\\session\\' -and
                    $_.FullName -notmatch '\\\.commonhe\\session\\generated-values(\.failure)?\.json$'
                }
        )
    }

    foreach ($file in $allTextFiles) {
        $content = Get-Content -LiteralPath $file.FullName -Raw
        if ($null -eq $content) { $content = '' }
        foreach ($char in $content.ToCharArray()) {
            $code = [int][char]$char
            if (($code -lt 32) -and ($code -notin @(9, 10, 13))) {
                $relative = $file.FullName.Substring($root.Length).TrimStart('\', '/')
                Add-Issue -Issues $issues -RuleId 'generated.no_control_chars' -Message "$relative contains illegal ASCII control character 0x$($code.ToString('X2'))."
                break
            }
        }
    }

    foreach ($file in $textFiles) {
        $content = Get-Content -LiteralPath $file.FullName -Raw
        if ($null -eq $content) { $content = '' }
        if ($content.Contains([string][char]0xFFFD)) {
            $relative = $file.FullName.Substring($root.Length).TrimStart('\', '/')
            Add-Issue -Issues $issues -RuleId 'generated.no_mojibake' -Message "$relative contains Unicode replacement character mojibake."
        }
        foreach ($phrase in @($manifest.forbidden_generated_phrases)) {
            if ($content.Contains([string]$phrase)) {
                $relative = $file.FullName.Substring($root.Length).TrimStart('\', '/')
                Add-Issue -Issues $issues -RuleId 'generated.no_template_or_business_claims' -Message "$relative contains forbidden phrase '$phrase'."
            }
        }
    }

    $combinedDocs = ''
    foreach ($docPath in @('AGENTS.md', 'CLAUDE.md', 'docs\project_context.md', 'docs\00-初始化结果索引.md')) {
        $combinedDocs += "`n" + (Get-TextIfExists -Path (Join-Path $root $docPath))
    }
    if (-not $combinedDocs.Contains('初始化协作包')) {
        Add-Issue -Issues $issues -RuleId 'generated.init_package_scope' -Message 'Generated docs must explicitly describe the output as 初始化协作包.'
    }

    [pscustomobject]@{
        Passed = ($issues.Count -eq 0)
        Mode = 'generated'
        TargetClient = $Client
        Issues = @($issues)
    }
}

if (-not $RepoRoot -and -not $GeneratedRoot) {
    throw 'Either -RepoRoot or -GeneratedRoot is required.'
}
if ($GeneratedRoot -and -not $TargetClient) {
    throw '-TargetClient is required with -GeneratedRoot.'
}

$result = if ($GeneratedRoot) {
    Test-GeneratedTruthSource -Root $GeneratedRoot -Client $TargetClient
} else {
    Test-RepoTruthSource -Root $RepoRoot
}

if ($AsJson) {
    $result | ConvertTo-Json -Depth 10
} else {
    $result
}

if (-not [bool]$result.Passed) {
    exit 1
}
