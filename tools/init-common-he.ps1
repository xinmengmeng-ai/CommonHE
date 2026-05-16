[CmdletBinding()]
param(
    [string]$TemplatesRoot,
    [Parameter(Mandatory = $true)]
    [string]$TargetRoot,
    [Parameter(Mandatory = $true)]
    [string]$ValuesPath,
    [string]$DecisionPath,
    [switch]$Execute,
    [switch]$Force
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$scriptDir = if ($PSScriptRoot) { $PSScriptRoot } else { Split-Path -Parent $MyInvocation.MyCommand.Path }
$packageRoot = Split-Path -Parent $scriptDir

function Remove-InvalidTextControlCharacters {
    param([AllowNull()][string]$Text)

    if ($null -eq $Text) { return '' }
    $builder = [System.Text.StringBuilder]::new()
    foreach ($char in $Text.ToCharArray()) {
        $code = [int][char]$char
        if (($code -ge 32) -or ($code -in @(9, 10, 13))) {
            [void]$builder.Append($char)
        }
    }
    $builder.ToString()
}

if (-not $TemplatesRoot) {
    $TemplatesRoot = Join-Path $packageRoot 'templates'
}

function Get-CoreTemplateMap {
    param(
        [string]$Root,
        [string]$TargetClient = 'codex'
    )

    $entries = @(
        @{ Source = (Join-Path $Root 'init-result-index.template.md'); Target = 'docs/00-初始化结果索引.md' }
        @{ Source = (Join-Path $Root 'project_context.template.md'); Target = 'docs/project_context.md' }
        @{ Source = (Join-Path $Root 'skills/required-capabilities.template.md'); Target = 'docs/skills/required-capabilities.md' }
        @{ Source = (Join-Path $Root 'architecture/01-项目架构设计书.template.md'); Target = 'docs/architecture/01-项目架构设计书.md' }
        @{ Source = (Join-Path $Root 'roadmap/01-实施路线图.template.md'); Target = 'docs/roadmap/01-实施路线图.md' }
        @{ Source = (Join-Path $Root 'workflow/current-stage-user-checklist.template.md'); Target = 'docs/workflow/current-stage-user-checklist.md' }
        @{ Source = (Join-Path $Root 'workflow/archive-policy.template.md'); Target = 'docs/workflow/archive-policy.md' }
        @{ Source = (Join-Path $Root 'workflow/acceptance-gates.template.md'); Target = 'docs/workflow/acceptance-gates.md' }
        @{ Source = (Join-Path $Root 'workflow/evaluator-protocol.template.md'); Target = 'docs/workflow/evaluator-protocol.md' }
        @{ Source = (Join-Path $Root 'workflow/grading-criteria.template.md'); Target = 'docs/workflow/grading-criteria.md' }
        @{ Source = (Join-Path $Root 'workflow/sprint-contract-generic.template.md'); Target = 'docs/workflow/sprint-contract-template.md' }
        @{ Source = (Join-Path $Root 'workflow/implementation-kickoff.template.md'); Target = 'docs/workflow/implementation-kickoff.md' }
        @{ Source = (Join-Path $Root 'workflow/sprint-contract.template.md'); Target = 'docs/workflow/first-sprint-contract.md' }
        @{ Source = (Join-Path $Root 'workflow/first-task-pack.template.md'); Target = 'docs/workflow/first-task-pack.md' }
    )

    if ($TargetClient -eq 'claude-code') {
        $entries += @(
            @{ Source = (Join-Path $Root 'CLAUDE.template.md'); Target = 'CLAUDE.md' }
            @{ Source = (Join-Path $Root 'claude/settings.template.json'); Target = '.claude/settings.json' }
            @{ Source = (Join-Path $Root 'skills/runtime-required-capabilities.template.md'); Target = '.claude/skills/required-capabilities.md' }
        )
    } else {
        $entries += @(
            @{ Source = (Join-Path $Root 'AGENTS.template.md'); Target = 'AGENTS.md' }
            @{ Source = (Join-Path $Root 'COORDINATOR-SUBAGENTS.template.md'); Target = '.codex/COORDINATOR-SUBAGENTS.md' }
            @{ Source = (Join-Path $Root 'skills/runtime-required-capabilities.template.md'); Target = '.agents/skills/required-capabilities.md' }
        )
    }

    $entries
}

function Get-RoleTemplateMap {
    @{
        architect = 'agents/architect.template.md'
        backend = 'agents/backend.template.md'
        frontend = 'agents/frontend.template.md'
        miniapp = 'agents/miniapp.template.md'
        reviewer = 'agents/reviewer.template.md'
        qa = 'agents/qa.template.md'
        docs = 'agents/docs.template.md'
        database = 'agents/database.template.md'
        devops = 'agents/devops.template.md'
        compliance = 'agents/compliance.template.md'
    }
}

function Get-HandbookTemplateMap {
    @{
        architect = 'handbooks/architect-handbook.template.md'
        backend = 'handbooks/backend-handbook.template.md'
        frontend = 'handbooks/frontend-handbook.template.md'
        miniapp = 'handbooks/miniapp-handbook.template.md'
        reviewer = 'handbooks/reviewer-handbook.template.md'
        qa = 'handbooks/qa-handbook.template.md'
        docs = 'handbooks/docs-handbook.template.md'
        database = 'handbooks/database-handbook.template.md'
        devops = 'handbooks/devops-handbook.template.md'
        compliance = 'handbooks/compliance-handbook.template.md'
    }
}

function Get-EnabledRolesFromReplacementTable {
    param([hashtable]$Table)

    $sourceText = ''
    if ($Table.ContainsKey('enabled_roles') -and [string]$Table['enabled_roles']) {
        $sourceText = [string]$Table['enabled_roles']
    } elseif ($Table.ContainsKey('roles_and_manuals') -and [string]$Table['roles_and_manuals']) {
        $sourceText = [string]$Table['roles_and_manuals']
    }

    $roles = New-Object System.Collections.ArrayList
    foreach ($line in ($sourceText -split "`r?`n")) {
        if ($line -match '^\s*-\s*(.+?)\s*$') {
            [void]$roles.Add([string]$matches[1])
        }
    }

    @($roles | Sort-Object -Unique)
}

function Test-ReplacementRoleEnabled {
    param(
        [object[]]$EnabledRoles,
        [string]$RoleName
    )

    foreach ($role in @($EnabledRoles)) {
        if ([string]$role -eq $RoleName) {
            return $true
        }
    }

    $false
}

function Get-ReplacementWorkflowStageText {
    param([object[]]$EnabledRoles)

    $stages = New-Object System.Collections.ArrayList
    [void]$stages.Add('implementation')
    if (Test-ReplacementRoleEnabled -EnabledRoles $EnabledRoles -RoleName 'reviewer') { [void]$stages.Add('review') }
    if (Test-ReplacementRoleEnabled -EnabledRoles $EnabledRoles -RoleName 'qa') { [void]$stages.Add('qa') }

    (@($stages) -join ' / ')
}

function Get-ReplacementDispatchTriggers {
    param([object[]]$EnabledRoles)

    $items = New-Object System.Collections.ArrayList
    if (Test-ReplacementRoleEnabled -EnabledRoles $EnabledRoles -RoleName 'architect') { [void]$items.Add('- 架构/方案 -> architect') }
    if (Test-ReplacementRoleEnabled -EnabledRoles $EnabledRoles -RoleName 'backend') { [void]$items.Add('- 后端/API -> backend') }
    if (Test-ReplacementRoleEnabled -EnabledRoles $EnabledRoles -RoleName 'frontend') { [void]$items.Add('- 页面/UI -> frontend') }
    if (Test-ReplacementRoleEnabled -EnabledRoles $EnabledRoles -RoleName 'miniapp') { [void]$items.Add('- 微信小程序端 -> miniapp') }
    if (Test-ReplacementRoleEnabled -EnabledRoles $EnabledRoles -RoleName 'database') { [void]$items.Add('- 数据结构/迁移 -> database') }
    if (Test-ReplacementRoleEnabled -EnabledRoles $EnabledRoles -RoleName 'devops') { [void]$items.Add('- 部署/流水线 -> devops') }
    if (Test-ReplacementRoleEnabled -EnabledRoles $EnabledRoles -RoleName 'docs') { [void]$items.Add('- 真源/文档 -> docs') }
    if (Test-ReplacementRoleEnabled -EnabledRoles $EnabledRoles -RoleName 'reviewer') { [void]$items.Add('- 审查 -> reviewer') }
    if (Test-ReplacementRoleEnabled -EnabledRoles $EnabledRoles -RoleName 'qa') { [void]$items.Add('- 测试/回归 -> qa') }
    if (Test-ReplacementRoleEnabled -EnabledRoles $EnabledRoles -RoleName 'compliance') { [void]$items.Add('- 合规/策略 -> compliance') }

    $itemsText = @($items) -join "`n"
    if ($itemsText) { return $itemsText }
    '- 页面/UI -> frontend'
}

function Get-ReplacementCoreGateItems {
    param([object[]]$EnabledRoles)

    $items = New-Object System.Collections.ArrayList
    [void]$items.Add('- 真源文档可读')
    [void]$items.Add('- capability gate 为绿色')
    [void]$items.Add('- 初始化协作包结构、目标软件入口与 session 审计产物已通过 postcheck')
    [void]$items.Add('- 梦星星方案输出、用户选择与星梦梦语义验收记录一致')
    [void]$items.Add('- 不得声称业务系统、业务代码、评审或测试已经完成')
    @($items) -join "`n"
}

function Get-ReplacementFinalGateItems {
    param([object[]]$EnabledRoles)

    $items = New-Object System.Collections.ArrayList
    if (Test-ReplacementRoleEnabled -EnabledRoles $EnabledRoles -RoleName 'reviewer') {
        [void]$items.Add('- reviewer 级结构与风险审查')
    } else {
        [void]$items.Add('- 结构与风险审查')
    }
    if (Test-ReplacementRoleEnabled -EnabledRoles $EnabledRoles -RoleName 'qa') {
        [void]$items.Add('- qa 级测试与回归验证')
    } else {
        [void]$items.Add('- 必要测试与回归验证证据')
    }
    [void]$items.Add('- 用户可见行为证据')
    [void]$items.Add('- 自动化检查与门禁')
    @($items) -join "`n"
}

function Get-ReplacementCollaborationText {
    param(
        [string]$LeadText,
        [string]$EvidenceText,
        [object[]]$EnabledRoles,
        [switch]$NoteOnly
    )

    $hasReviewer = Test-ReplacementRoleEnabled -EnabledRoles $EnabledRoles -RoleName 'reviewer'
    $hasQa = Test-ReplacementRoleEnabled -EnabledRoles $EnabledRoles -RoleName 'qa'

    if ($NoteOnly) {
        if ($hasReviewer -and $hasQa) { return ('- ' + $LeadText + '时通知 `reviewer` 与 `qa`') }
        if ($hasReviewer) { return ('- ' + $LeadText + '时通知 `reviewer`，并补回 ' + $EvidenceText) }
        if ($hasQa) { return ('- ' + $LeadText + '时通知 `qa`') }
        return ('- ' + $LeadText + '时补回 ' + $EvidenceText)
    }

    if ($hasReviewer -and $hasQa) { return ('- ' + $LeadText + '，必须拉 `@reviewer` + `@qa`') }
    if ($hasReviewer) { return ('- ' + $LeadText + '，必须拉 `@reviewer`，并补回 ' + $EvidenceText) }
    if ($hasQa) { return ('- ' + $LeadText + '，必须拉 `@qa`') }
    ('- ' + $LeadText + '，必须补回 ' + $EvidenceText)
}

function ConvertTo-ReplacementTable {
    param($ValuesObject)
    $table = @{}
    foreach ($property in $ValuesObject.PSObject.Properties) {
        $table[$property.Name] = [string]$property.Value
    }

    $defaults = @{
        architecture_version = 'v0.1.0'
        architecture_date = (Get-Date -Format 'yyyy-MM-dd')
        architecture_owner = '星星的vibecoding启动器初始化编排器'
        architecture_status = 'draft'
        product_positioning = '当前项目的产品定位待进一步细化'
        target_users = '目标用户待进一步确认'
        core_value = '聚焦当前阶段最核心的用户价值'
        current_scope = '当前仅覆盖初始化阶段所需内容'
        out_of_scope = '- 本轮未确认能力'
        architecture_overview = '当前按最小可运行方案初始化'
        tech_stack = '后续按项目类型和实现阶段继续细化'
        service_or_module_design = '后续按实际项目结构继续细化'
        data_model_principles = '以最小必要数据结构为准'
        api_contract_principles = '接口保持清晰、稳定、可验证'
        security_and_compliance = '按当前项目需要分级处理'
        deployment_and_ops = '优先保证当前阶段可落地'
        architecture_tradeoffs = '当前优先交付速度与可验证性'
        current_architecture_conclusion = '当前架构文档为初始化阶段草案'
        roadmap_version = 'v0.1.0'
        roadmap_date = (Get-Date -Format 'yyyy-MM-dd')
        roadmap_owner = '星星的vibecoding启动器初始化编排器'
        roadmap_status = 'draft'
        project_goal = '完成项目初始化并进入首轮实现'
        current_phase_goal = '完成初始化阶段核心生成物'
        success_criteria = '- 初始化链路通过`n- 核心文档生成完成'
        milestone_overview = 'M1 初始化、M2 实现、M3 验收'
        milestone_details = '- M1 初始化`n- M2 实现`n- M3 验收'
        role_responsibilities = '- 主控负责调度`n- 员工负责执行'
        deliverables = '- 协议文件`n- 文档骨架`n- 角色模板'
        target_client = 'codex'
        target_client_name = 'Codex'
        client_entry_file = 'AGENTS.md'
        client_coordinator_path = '.codex/COORDINATOR-SUBAGENTS.md'
        client_agent_path = '.codex/agents/*.md'
        client_skill_path = '.agents/skills/required-capabilities.md'
        risks_and_mitigations = '- 范围漂移：通过真源控制'
        execution_sequence = '- discovery`n- proposal`n- bootstrap'
        completed_items = '- 初始化前尚无已完成项'
        must_confirm_items = '- 当前无新增确认项'
        must_provide_items = '- 当前无新增提供项'
        deferred_decisions = '- 后续按实施阶段再补'
        blocking_points = '- 当前无阻塞点'
        required_capabilities_list = "- superpowers`n- agent-browser`n- chrome-devtools`n- GitNexus`n- Speckit"
        capability_probe_summary = '- 当前无探测结果'
        capability_scope_notes = '- 仅按用户确认的能力启用对应 Skill / MCP / CLI；未选能力不安装、不配置、不生成产物。'
        capability_gate_status = '- 当前能力门禁状态待补充（doctor / precheck）'
        capability_remediation_section = ''
        autodiscovery_assumptions = '- 当前无自动分析假设'
        autodiscovery_signal_summary = '- 当前无自动分析信号'
        project_specific_dimensions = '- 当前无额外项目特定维度'
        calibration_examples = '- 当前无额外校准样例'
        implementation_owner_role = 'frontend'
        implementation_support_roles = '- reviewer`n- docs'
        dominant_workstreams_summary = '- frontend'
        kickoff_required_reads = '- docs/project_context.md`n- docs/workflow/implementation-kickoff.md`n- docs/workflow/first-task-pack.md'
        first_task_pack_items = "### task_1`n- title: 锁定首轮实施范围与真源`n- owner_role: docs`n- support_roles: reviewer`n- depends_on: docs/project_context.md`n- done_signal: 真源与范围已收口，尚未声称业务实现完成`n`n### task_2`n- title: 准备第一优先工作流实施合同`n- owner_role: frontend`n- support_roles: reviewer, docs`n- depends_on: docs/workflow/implementation-kickoff.md`n- done_signal: 实施合同、接口边界和验证计划已成文，等待后续实施"
        first_task_pack_gate_note = '进入 review 前先补齐证据采集计划；业务证据需等后续实施后再补'
        task_title = '首轮实施准备'
        task_id = 'first-sprint'
        implementer_role = 'docs'
        evaluator_role = 'reviewer'
        risk_gate = 'low'
        date = (Get-Date -Format 'yyyy-MM-dd')
        requirement_summary = "- 当前阶段目标：完成初始化协作包接手`n- 第一优先工作流：首轮实施准备"
        criteria_1 = '完成首轮接手准备，并满足当前阶段最小验收标准'
        verify_method_1 = '按 docs/workflow/acceptance-gates.md 完成验证，并补齐证据'
        deliverable_ref_1 = 'docs/workflow/implementation-kickoff.md'
        additional_dimensions = '| 当前无额外维度 | - | - |'
        known_risks = '- 当前无已知风险'
        test_strategy = '- 先补充自动化验证与运行态验证'
    }
    foreach ($key in $defaults.Keys) {
        if (-not $table.ContainsKey($key)) {
            $table[$key] = $defaults[$key]
        }
    }

    $enabledRoles = @(Get-EnabledRolesFromReplacementTable -Table $table)
    if ($enabledRoles.Count -gt 0) {
        $table['dispatch_triggers'] = Get-ReplacementDispatchTriggers -EnabledRoles $enabledRoles
        $table['blocked_workflow_stages'] = Get-ReplacementWorkflowStageText -EnabledRoles $enabledRoles
        $table['core_gate_items'] = Get-ReplacementCoreGateItems -EnabledRoles $enabledRoles
        $table['final_gate_items'] = Get-ReplacementFinalGateItems -EnabledRoles $enabledRoles
        $table['frontend_collaboration_trigger'] = Get-ReplacementCollaborationText -LeadText '改关键页面链路、权限显示、导航时' -EvidenceText '关键验证证据' -EnabledRoles $enabledRoles
        $table['backend_collaboration_trigger'] = Get-ReplacementCollaborationText -LeadText '改接口、鉴权、核心业务流时' -EvidenceText '接口与鉴权验证证据' -EnabledRoles $enabledRoles
        $table['database_collaboration_trigger'] = Get-ReplacementCollaborationText -LeadText '涉及表结构、迁移策略、索引策略变化时' -EvidenceText '迁移验证与回滚证据' -EnabledRoles $enabledRoles
        $table['devops_collaboration_trigger'] = Get-ReplacementCollaborationText -LeadText '涉及流水线、部署配置、环境策略变化时' -EvidenceText '部署验证与回滚证据' -EnabledRoles $enabledRoles
        $table['frontend_collaboration_note'] = Get-ReplacementCollaborationText -LeadText '涉及关键页面链路' -EvidenceText '关键验证证据' -EnabledRoles $enabledRoles -NoteOnly
        $table['backend_collaboration_note'] = Get-ReplacementCollaborationText -LeadText '涉及接口、鉴权、关键业务流' -EvidenceText '接口与鉴权验证证据' -EnabledRoles $enabledRoles -NoteOnly
    }

    $table
}

function Get-ReplacementValueOrDefault {
    param(
        [hashtable]$ReplacementTable,
        [string]$Key,
        [string]$DefaultValue
    )

    if ($ReplacementTable.ContainsKey($Key) -and [string]$ReplacementTable[$Key]) {
        return [string]$ReplacementTable[$Key]
    }

    $DefaultValue
}

function Get-TemplateFallbackValue {
    param(
        [string]$Key,
        [hashtable]$ReplacementTable
    )

    $targetClient = Get-ReplacementValueOrDefault -ReplacementTable $ReplacementTable -Key 'target_client' -DefaultValue 'codex'
    $targetClientName = if ($targetClient -eq 'claude-code') { 'Claude Code' } else { 'Codex' }
    $clientEntryFile = if ($targetClient -eq 'claude-code') { 'CLAUDE.md' } else { 'AGENTS.md' }
    $clientCoordinatorPath = if ($targetClient -eq 'claude-code') { '.claude/settings.json' } else { '.codex/COORDINATOR-SUBAGENTS.md' }
    $clientAgentPath = if ($targetClient -eq 'claude-code') { '.claude/agents/*.md' } else { '.codex/agents/*.md' }
    $clientSkillPath = if ($targetClient -eq 'claude-code') { '.claude/skills/required-capabilities.md' } else { '.agents/skills/required-capabilities.md' }
    $projectName = Get-ReplacementValueOrDefault -ReplacementTable $ReplacementTable -Key 'project_name' -DefaultValue '未命名协作包'
    $enabledRoles = Get-ReplacementValueOrDefault -ReplacementTable $ReplacementTable -Key 'enabled_roles' -DefaultValue '- docs'

    switch ($Key) {
        'package_intro' { return "这是主 Agent 为 ``$projectName`` 收口的初始化协作包：它负责沉淀方案、目标软件入口、能力门禁和第一轮接手方式，不代表业务系统已经实现。" }
        'handoff_summary' { return "主 Agent 已完成方案与目标软件收口；后续请在 $targetClientName 中从 ``$clientEntryFile`` 重新接手，并以 docs/ 与 .commonhe/session/ 作为追溯真源。" }
        'target_client_name' { return $targetClientName }
        'client_entry_file' { return $clientEntryFile }
        'client_coordinator_path' { return $clientCoordinatorPath }
        'client_agent_path' { return $clientAgentPath }
        'client_skill_path' { return $clientSkillPath }
        'external_references' { return '- 本轮未引入外部参考源；后续如新增参考材料，需要先写入真源。' }
        'stage_constraints' { return "- 当前阶段只生成面向 $targetClientName 的初始化协作包，不直接展开业务实现。`n- 后续实施必须从 $clientEntryFile 和 docs/ 真源重新接手。" }
        'deferred_capabilities' { return '- 五项工作流能力均为必需能力；缺失时必须先安装或配置，不进入业务实施。' }
        'recommended_roles_now' { return $enabledRoles }
        'available_roles_later' { return '- 当前先不追加其他后续角色；如实施范围扩大，再由主控重新评估。' }
        'selected_capabilities_summary' { return "- superpowers / required skills（superpowers，推荐且必选）`n- agent-browser（agent-browser，推荐且必选）`n- chrome-devtools MCP（chrome-devtools，推荐且必选）`n- GitNexus（GitNexus，推荐且必选）`n- Speckit（Speckit，推荐且必选）`n- 当前版本五项能力均为必需能力，不提供取消入口。" }
        'capability_scope_notes' { return '- superpowers、agent-browser、chrome-devtools、GitNexus、Speckit 均为必需能力；缺失时必须安装或配置后再继续。' }
        'capability_remediation_section' { return '' }
        'domain_workflow_section' { return '' }
        'postcheck_status' { return '`postcheck` 已通过' }
        'closure_summary' { return "初始化协作包已生成并通过 postcheck。请在 $targetClientName 中新开会话或重启会话，从 $clientEntryFile 和 docs/ 真源重新接手；当前初始化线程到此收口，不继续业务实现。" }
        'safe_retained_paths' { return "- docs/`n- .commonhe/session/`n- $clientEntryFile`n- $clientCoordinatorPath" }
        'current_phase_goal' { return "完成面向 $targetClientName 的初始化协作包收口，并准备首轮实施接手" }
        'current_goals' { return "- 完成方案收口`n- 生成 $targetClientName 原生入口`n- 保持初始化协作包口径一致" }
        'in_scope_items' { return "- 初始化协作包真源文档`n- $targetClientName 原生入口与角色调度文件`n- 已选能力记录与后续接手说明" }
        'in_scope' { return "- 初始化协作包真源文档`n- $targetClientName 原生入口与角色调度文件`n- 已选能力记录与后续接手说明" }
        'out_of_scope_items' { return "- 本轮不生成业务项目成品、业务代码或业务脚手架`n- 未经用户确认的扩展能力不默认启用" }
        'out_of_scope' { return "- 本轮不生成业务项目成品、业务代码或业务脚手架`n- 未经用户确认的扩展能力不默认启用" }
        'acceptance_criteria' { return "- 初始化协作包文件齐全`n- $clientEntryFile 与目标软件配置正确`n- postcheck 通过且无模板占位符残留" }
        'core_gate_items' { return "- 真源文档可读`n- capability gate 为绿色`n- 初始化协作包结构、目标软件入口与 session 审计产物已通过 postcheck" }
        'final_gate_items' { return "- 结构与风险审查`n- 用户可见行为证据`n- 自动化检查与门禁" }
        'implementation_checklist_items' { return "- 读取本初始化协作包的真源入口`n- 在 $targetClientName 新会话中从 $clientEntryFile 接手`n- 按 docs/workflow/first-task-pack.md 推进首轮实施" }
        'current_phase_tasks' { return "- 读取本初始化协作包的真源入口`n- 在 $targetClientName 新会话中从 $clientEntryFile 接手`n- 按 docs/workflow/first-task-pack.md 推进首轮实施" }
        'completed_items' { return "- 接手包真源、目标入口与审计记录已准备完成`n- 业务实施尚未开始" }
        'must_confirm_items' { return "- [ ] 待用户确认：是否以当前真源作为 $targetClientName 新会话的唯一接手依据" }
        'must_provide_items' { return "- 若后续要加入外部风格、内容或接口参考，请提供稳定路径并回写真源。" }
        'deferred_decisions' { return '- 本轮未确认的能力不默认启用；需要后续主控重新评估后再写入真源。' }
        'blocking_points' { return '- 未确认首轮范围、责任角色与验证证据前，不得签收业务实现完成。' }
        'dominant_workstreams_summary' { return "- 初始化协作包接手`n- 首轮实施准备" }
        'implementation_owner_role' { return 'docs' }
        'implementation_support_roles' { return "- reviewer" }
        'kickoff_required_reads' { return "- docs/project_context.md`n- docs/roadmap/01-实施路线图.md`n- docs/workflow/implementation-kickoff.md`n- docs/workflow/first-task-pack.md" }
        'first_task_pack_items' { return "### task_1`n- title: 锁定首轮实施范围与真源`n- owner_role: docs`n- support_roles: reviewer`n- depends_on: docs/project_context.md`n- done_signal: 真源与范围已收口，尚未声称业务实现完成`n`n### task_2`n- title: 准备第一优先工作流实施合同`n- owner_role: docs`n- support_roles: reviewer`n- depends_on: docs/workflow/implementation-kickoff.md`n- done_signal: 实施合同、接口边界和验证计划已成文，等待后续实施" }
        'first_task_pack_gate_note' { return '进入验收前先补齐证据采集计划；业务证据需等后续实施后再补' }
        'task_id' { return 'first-sprint' }
        'implementer_role' { return 'docs' }
        'evaluator_role' { return 'reviewer' }
        'risk_gate' { return 'low' }
        'requirement_summary' { return "- 当前阶段目标：完成初始化协作包接手`n- 第一优先工作流：首轮实施准备" }
        'first_priority_workflow' { return '- 围绕当前项目准备首轮实施边界、责任角色和验证计划；第一轮只按真源推进，不提前签收业务实现。' }
        'criteria_1' { return '完成首轮接手准备，并满足当前阶段最小验收标准' }
        'verify_method_1' { return '按 docs/workflow/acceptance-gates.md 完成验证，并补齐证据' }
        'deliverable_ref_1' { return 'docs/workflow/implementation-kickoff.md' }
        'additional_dimensions' { return '| 用户可见行为证据 | >= 6 | 中 |' }
        'known_risks' { return "- 范围漂移：通过 kickoff pack 与真源收口`n- 证据不足：由 reviewer / qa 补齐验证" }
        'test_strategy' { return '- 关键验证命令 + 用户可见行为证据' }
        'first_sprint_deliverables' { return "- 首轮实施范围与责任角色确认记录`n- 首轮接口、页面或流程边界说明`n- 面向 reviewer / qa 的验证证据采集计划" }
        'evidence_plan' { return "- 每个首轮任务完成时记录命令输出、截图或日志路径`n- reviewer 复核范围漂移与角色交接证据`n- qa 只在有可执行实现后补齐回归证据" }
        'date' { return (Get-Date -Format 'yyyy-MM-dd') }
        'deliverables' { return "- $clientEntryFile 与目标软件配置`n- docs/ 真源文档`n- .commonhe/session/ 可追溯状态" }
        'project_specific_dimensions' { return '- 当前无额外项目特定维度' }
        'calibration_examples' { return '- 当前无额外校准样例' }
        'autodiscovery_signal_summary' { return '- 主 Agent 已完成当前收口，本轮未保留额外自动分析信号。' }
        'autodiscovery_assumptions' { return '- 当前没有额外未验证假设；后续范围变化需重新回写真源。' }
        default { return $null }
    }
}

function Get-RenderedTemplateContentIssues {
    param(
        [string]$Content,
        [string]$Target
    )

    $issues = New-Object System.Collections.ArrayList
    foreach ($placeholderMatch in [regex]::Matches($Content, '\{\{[a-zA-Z0-9_\-]+\}\}')) {
        [void]$issues.Add("unresolved_placeholder:$($placeholderMatch.Value)")
    }
    foreach ($variableMatch in [regex]::Matches($Content, '\$[a-zA-Z_][a-zA-Z0-9_]*')) {
        [void]$issues.Add("unresolved_script_variable:$($variableMatch.Value)")
    }

    foreach ($phrase in @(
        'ExampleProject',
        'Build the first usable version quickly',
        '当前无自动分析假设',
        '当前无自动分析信号',
        '当前无外部参考源',
        '当前无明确延后能力',
        '生成项目骨架',
        '当前目录已经生成 HE 协作工程'
    )) {
        if ($Content -like "*$phrase*") {
            [void]$issues.Add("template_phrase:$phrase")
        }
    }

    @($issues)
}

function Expand-TemplateContent {
    param(
        [string]$Content,
        [hashtable]$ReplacementTable
    )

    [regex]::Replace($Content, '\{\{([a-zA-Z0-9_\-]+)\}\}', {
        param($match)
        $key = $match.Groups[1].Value
        if ($ReplacementTable.ContainsKey($key)) {
            return $ReplacementTable[$key]
        }
        $fallbackValue = Get-TemplateFallbackValue -Key $key -ReplacementTable $ReplacementTable
        if ($null -ne $fallbackValue) {
            return $fallbackValue
        }
        return $match.Value
    })
}

function Add-TemplateEntry {
    param(
        [System.Collections.ArrayList]$TemplateEntries,
        [string]$Source,
        [string]$Target,
        [hashtable]$ExtraReplacements,
        [string]$EntryType = 'Template'
    )

    [void]$TemplateEntries.Add([pscustomobject]@{
        Source = $Source
        Target = $Target
        ExtraReplacements = $ExtraReplacements
        EntryType = $EntryType
    })
}

function Get-AgencyAgentsRoot {
    $candidates = @(
        (Join-Path $packageRoot 'agency-agents-zh'),
        (Join-Path $packageRoot 'references\agency-agents-zh'),
        (Join-Path (Split-Path -Parent $packageRoot) 'agency-agents-zh')
    )

    foreach ($candidate in $candidates) {
        if (Test-Path -LiteralPath $candidate -PathType Container) {
            return [System.IO.Path]::GetFullPath($candidate)
        }
    }

    return ''
}

function Get-AgencyAgentIdsForRole {
    param([string]$RoleName)

    switch ($RoleName) {
        'architect' { return @('engineering-software-architect') }
        'backend' { return @('engineering-backend-architect') }
        'frontend' { return @('engineering-frontend-developer') }
        'miniapp' { return @('engineering-wechat-mini-program-developer') }
        'reviewer' { return @('engineering-code-reviewer') }
        'qa' { return @('testing-evidence-collector') }
        'docs' { return @('engineering-technical-writer') }
        'database' { return @('engineering-database-optimizer') }
        'devops' { return @('engineering-devops-automator') }
        'compliance' { return @('compliance-auditor') }
        'integration-feishu' { return @('engineering-feishu-integration-developer') }
        'integration-dingtalk' { return @('engineering-dingtalk-integration-developer') }
        default {
            if ($RoleName -like 'integration-*') {
                $integrationName = $RoleName.Substring('integration-'.Length)
                $candidate = "engineering-$integrationName-integration-developer"
                return @($candidate)
            }
            return @($RoleName)
        }
    }
}

function Remove-ObsoleteManagedRoleFiles {
    param(
        [string]$TargetRoot,
        [string]$TargetClient,
        [object[]]$TemplateEntries,
        [switch]$Execute,
        [switch]$Force
    )

    if (-not $Execute -or -not $Force) {
        return
    }

    $expectedTargets = @(
        foreach ($entry in $TemplateEntries) {
            if ($entry -and $entry.PSObject.Properties.Name -contains 'Target') {
                [string]$entry.Target
            }
        }
    )
    $agentRoot = if ($TargetClient -eq 'claude-code') { '.claude/agents' } else { '.codex/agents' }
    $handbookMap = Get-HandbookTemplateMap

    foreach ($roleName in $handbookMap.Keys) {
        foreach ($agentId in (Get-AgencyAgentIdsForRole -RoleName $roleName)) {
            $relativePath = "$agentRoot/$agentId.md"
            if ($expectedTargets -contains $relativePath) {
                continue
            }
            $fullPath = Join-Path $TargetRoot $relativePath
            if (Test-Path -LiteralPath $fullPath -PathType Leaf) {
                Remove-Item -LiteralPath $fullPath -Force
            }
        }

        $handbookRelativePath = "docs/agents/$roleName-handbook.md"
        if ($expectedTargets -contains $handbookRelativePath) {
            continue
        }
        $handbookFullPath = Join-Path $TargetRoot $handbookRelativePath
        if (Test-Path -LiteralPath $handbookFullPath -PathType Leaf) {
            Remove-Item -LiteralPath $handbookFullPath -Force
        }
    }
}

function Find-AgencyAgentFile {
    param(
        [string]$AgencyRoot,
        [string]$AgentId
    )

    if (-not $AgencyRoot -or -not (Test-Path -LiteralPath $AgencyRoot -PathType Container)) {
        return ''
    }

    $direct = Get-ChildItem -LiteralPath $AgencyRoot -Recurse -File -Filter "$AgentId.md" -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($direct) {
        return [System.IO.Path]::GetFullPath($direct.FullName)
    }

    return ''
}

function Convert-AgencyTemplateMarkersToText {
    param([string]$Content)

    if ($null -eq $Content) {
        return ''
    }

    $converted = [regex]::Replace([string]$Content, '\{\{\s*([^}]+?)\s*\}\}', {
        param($Match)
        "<agency-template:$($Match.Groups[1].Value.Trim())>"
    })
    $converted.Replace('{{', '<agency-template-open>').Replace('}}', '<agency-template-close>')
}

function ConvertTo-AgencyAgentContent {
    param(
        [string]$OriginalContent,
        [string]$AgentId,
        [string]$SourceRole,
        [hashtable]$ReplacementTable
    )

    $projectName = Get-ReplacementValueOrDefault -ReplacementTable $ReplacementTable -Key 'project_name' -DefaultValue '未命名协作包'
    $targetClientName = Get-ReplacementValueOrDefault -ReplacementTable $ReplacementTable -Key 'target_client_name' -DefaultValue 'Codex'
    $requiredCapabilities = Get-ReplacementValueOrDefault -ReplacementTable $ReplacementTable -Key 'required_capabilities_list' -DefaultValue "- superpowers`n- agent-browser`n- chrome-devtools`n- GitNexus`n- Speckit"
    $sanitizedOriginalContent = Convert-AgencyTemplateMarkersToText -Content $OriginalContent

@"
# $AgentId

    来源：agency-agents-zh
星星的vibecoding启动器映射角色：$SourceRole
目标初始化协作包：$projectName
后续接管软件：$targetClientName

## 必须具备的能力

$requiredCapabilities

## CommonHE 接手约束

- 本文件来自 `agency-agents-zh` 的真实 agent 角色库，不是启动器内部通用占位角色。
- 你只能在新会话接手阶段执行对应职责；当前产物仍是初始化协作包，不代表业务系统已经实现。
- 如果任一必需能力缺失，先按能力门禁说明完成检测、安装或配置，并重启服务或会话后再继续。

## 原始 Agent 人格

> 星星的vibecoding启动器保留真实 agent 原文语义；如上游 agent 示例中含有双大括号形式的外部模板变量，已转写为 `<agency-template:...>`，避免最终初始化协作包残留未展开占位符。

$sanitizedOriginalContent
"@
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

function Test-DecisionCapabilityEnabled {
    param(
        $Decision,
        [string]$CapabilityName
    )

    if ($null -eq $Decision) { return $true }
    $selectedCapabilities = @()
    if ($Decision.PSObject.Properties.Match('selected_capabilities').Count -gt 0) {
        $selectedCapabilities = @(foreach ($item in $Decision.selected_capabilities) { $item })
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

    if ($Decision.PSObject.Properties.Match('required_capabilities').Count -gt 0) {
        foreach ($capability in @($Decision.required_capabilities)) {
            $name = Get-CapabilityIdentity -Capability $capability
            if ($name -and $name.Equals($CapabilityName, [System.StringComparison]::OrdinalIgnoreCase)) {
                return $true
            }
        }
        return $false
    }

    $true
}

function Copy-SpeckitScaffold {
    param(
        [string]$TargetRoot,
        [switch]$Execute
    )

    $sourceRoot = Join-Path $packageRoot '.specify'
    if (-not (Test-Path -LiteralPath $sourceRoot -PathType Container)) {
        $sourceRoot = Join-Path $packageRoot 'references\.specify'
    }
    if (-not (Test-Path -LiteralPath $sourceRoot -PathType Container)) {
        return @()
    }

    $normalizedSourceRoot = ([System.IO.Path]::GetFullPath($sourceRoot)).TrimEnd('\', '/')
    $copyResults = @()
    foreach ($item in Get-ChildItem -LiteralPath $sourceRoot -Recurse -Force) {
        if ($item.PSIsContainer) {
            continue
        }

        $normalizedItemPath = [System.IO.Path]::GetFullPath($item.FullName)
        $relativePath = $normalizedItemPath.Substring($normalizedSourceRoot.Length).TrimStart('\', '/')
        $targetRelativePath = Join-Path '.specify' $relativePath
        $targetPath = [System.IO.Path]::GetFullPath((Join-Path $TargetRoot $targetRelativePath))
        $copyResults += [pscustomobject]@{
            Mode = if ($Execute) { 'Execute' } else { 'DryRun' }
            Template = $item.FullName
            Target = $targetPath
            TargetExists = Test-Path -LiteralPath $targetPath
            Status = if ($Execute) { 'Pending' } else { 'Planned' }
        }

        if ($Execute) {
            $targetParent = Split-Path -Parent $targetPath
            New-Item -ItemType Directory -Path $targetParent -Force | Out-Null
            Copy-Item -LiteralPath $item.FullName -Destination $targetPath -Force
            $copyResults[-1].Status = 'Generated'
        }
    }

    $compatibilityScriptRoot = Join-Path $sourceRoot 'extensions\git\scripts\powershell'
    if (Test-Path -LiteralPath $compatibilityScriptRoot -PathType Container) {
        foreach ($item in Get-ChildItem -LiteralPath $compatibilityScriptRoot -File -Filter '*.ps1') {
            $targetRelativePath = Join-Path '.specify\scripts\powershell' $item.Name
            $targetPath = [System.IO.Path]::GetFullPath((Join-Path $TargetRoot $targetRelativePath))
            if (@($copyResults | Where-Object { [string]$_.Target -eq $targetPath }).Count -eq 0) {
                $copyResults += [pscustomobject]@{
                    Mode = if ($Execute) { 'Execute' } else { 'DryRun' }
                    Template = $item.FullName
                    Target = $targetPath
                    TargetExists = Test-Path -LiteralPath $targetPath
                    Status = if ($Execute) { 'Pending' } else { 'Planned' }
                }
            }

            if ($Execute) {
                $targetParent = Split-Path -Parent $targetPath
                New-Item -ItemType Directory -Path $targetParent -Force | Out-Null
                Copy-Item -LiteralPath $item.FullName -Destination $targetPath -Force
                $copyResults[-1].Status = 'Generated'
            }
        }
    }

    @($copyResults)
}

if (-not (Test-Path $TemplatesRoot)) { throw "Templates root not found: $TemplatesRoot" }
if (-not (Test-Path $ValuesPath)) { throw "Values file not found: $ValuesPath" }

$targetRoot = [System.IO.Path]::GetFullPath($TargetRoot)
$templatesRoot = [System.IO.Path]::GetFullPath($TemplatesRoot)
$values = Get-Content -Raw $ValuesPath | ConvertFrom-Json
$replacementTable = ConvertTo-ReplacementTable -ValuesObject $values
$templateEntries = [System.Collections.ArrayList]::new()

$targetClient = 'codex'
$decisionObject = $null

if ($DecisionPath) {
    if (-not (Test-Path $DecisionPath)) { throw "Decision file not found: $DecisionPath" }

    $decision = Get-Content -Raw $DecisionPath | ConvertFrom-Json
    $decisionObject = $decision
    $isConfirmed = $false
    if ($decision.PSObject.Properties.Name -contains 'user_confirmed') {
        $isConfirmed = [bool]$decision.user_confirmed
    }
    if (-not $isConfirmed) {
        throw "Decision-driven generation requires explicit user confirmation. Set 'user_confirmed' to true in the decision file."
    }

    if ($decision.PSObject.Properties.Name -contains 'target_client' -and [string]$decision.target_client) {
        $targetClient = [string]$decision.target_client
    }
}

(Get-CoreTemplateMap -Root $templatesRoot -TargetClient $targetClient) | ForEach-Object {
    Add-TemplateEntry -TemplateEntries $templateEntries -Source $_.Source -Target $_.Target -ExtraReplacements @{}
}

if ($DecisionPath) {
    $decision = Get-Content -Raw $DecisionPath | ConvertFrom-Json
    $decisionObject = $decision

    $roleMap = Get-RoleTemplateMap
    $handbookMap = Get-HandbookTemplateMap
    $agentRoot = if ($targetClient -eq 'claude-code') { '.claude/agents' } else { '.codex/agents' }
    $agencyRoot = Get-AgencyAgentsRoot

    if ($decision.enabled_roles) {
        foreach ($role in $decision.enabled_roles) {
            $roleName = [string]$role
            if (-not $handbookMap.ContainsKey($roleName)) {
                throw "Unsupported handbook role in decision file: $roleName"
            }

            $agencyAgentIds = @(Get-AgencyAgentIdsForRole -RoleName $roleName)
            foreach ($agentId in $agencyAgentIds) {
                $agencyAgentFile = Find-AgencyAgentFile -AgencyRoot $agencyRoot -AgentId $agentId
                if (-not $agencyAgentFile) {
                    throw "Agency agent file not found for role '$roleName': $agentId"
                }

                Add-TemplateEntry `
                    -TemplateEntries $templateEntries `
                    -Source $agencyAgentFile `
                    -Target "$agentRoot/$agentId.md" `
                    -ExtraReplacements @{ source_role = $roleName; agency_agent_id = $agentId } `
                    -EntryType 'AgencyAgent'
            }

            Add-TemplateEntry `
                -TemplateEntries $templateEntries `
                -Source (Join-Path $templatesRoot $handbookMap[$roleName]) `
                -Target "docs/agents/$roleName-handbook.md" `
                -ExtraReplacements @{}
        }
    }

    if ($decision.integrations) {
        foreach ($integration in $decision.integrations) {
            $integrationName = if ($integration -and $integration.PSObject.Properties.Match('name').Count -gt 0) {
                [string]$integration.name
            } else {
                [string]$integration
            }
            $displayName = if ($integration -and $integration.PSObject.Properties.Match('display_name').Count -gt 0) {
                [string]$integration.display_name
            } else {
                $integrationName
            }
            foreach ($agentId in (Get-AgencyAgentIdsForRole -RoleName "integration-$integrationName")) {
                $agencyAgentFile = Find-AgencyAgentFile -AgencyRoot $agencyRoot -AgentId $agentId
                if (-not $agencyAgentFile) {
                    throw "Agency agent file not found for integration '$integrationName': $agentId"
                }

                Add-TemplateEntry `
                    -TemplateEntries $templateEntries `
                    -Source $agencyAgentFile `
                    -Target "$agentRoot/$agentId.md" `
                    -ExtraReplacements @{ source_role = "integration-$integrationName"; agency_agent_id = $agentId } `
                    -EntryType 'AgencyAgent'
            }

            Add-TemplateEntry `
                -TemplateEntries $templateEntries `
                -Source (Join-Path $templatesRoot 'handbooks/integration-generic-handbook.template.md') `
                -Target "docs/agents/integration-$integrationName-handbook.md" `
                -ExtraReplacements @{
                    integration_name = $integrationName
                    integration_display_name = $displayName
                }
        }
    }
}

$null = Remove-ObsoleteManagedRoleFiles `
    -TargetRoot $targetRoot `
    -TargetClient $targetClient `
    -TemplateEntries @($templateEntries) `
    -Execute:$Execute `
    -Force:$Force

$results = @()
$results += @(Copy-SpeckitScaffold -TargetRoot $targetRoot -Execute:$Execute)

foreach ($entry in $templateEntries) {
    if (-not (Test-Path $entry.Source)) { throw "Template file not found: $($entry.Source)" }

    $targetPath = [System.IO.Path]::GetFullPath((Join-Path $targetRoot $entry.Target))
    $targetExists = Test-Path $targetPath
    $mode = if ($Execute) { 'Execute' } else { 'DryRun' }

    $result = [pscustomobject]@{
        Mode = $mode
        Template = $entry.Source
        Target = $targetPath
        TargetExists = $targetExists
        Status = if ($Execute) { 'Pending' } else { 'Planned' }
    }

    if ($Execute) {
        if ($targetExists -and -not $Force) {
            throw "Target already exists. Re-run with -Force to overwrite: $targetPath"
        }

        $content = Get-Content -Raw $entry.Source
        $mergedReplacements = @{}
        foreach ($key in $replacementTable.Keys) {
            $mergedReplacements[$key] = $replacementTable[$key]
        }
        if ($entry.ExtraReplacements) {
            foreach ($key in $entry.ExtraReplacements.Keys) {
                $mergedReplacements[$key] = [string]$entry.ExtraReplacements[$key]
            }
        }

        if ($entry.EntryType -eq 'AgencyAgent') {
            $expanded = ConvertTo-AgencyAgentContent `
                -OriginalContent $content `
                -AgentId ([string]$mergedReplacements['agency_agent_id']) `
                -SourceRole ([string]$mergedReplacements['source_role']) `
                -ReplacementTable $mergedReplacements
        } else {
            $expanded = Expand-TemplateContent -Content $content -ReplacementTable $mergedReplacements
        }
        $contentIssues = @(if ($entry.EntryType -eq 'AgencyAgent') {
            @(Get-RenderedTemplateContentIssues -Content $expanded -Target $entry.Target | Where-Object { [string]$_ -notlike 'unresolved_script_variable:*' })
        } else {
            @(Get-RenderedTemplateContentIssues -Content $expanded -Target $entry.Target)
        })
        if ($contentIssues.Count -gt 0) {
            throw "Rendered template quality gate failed for $($entry.Target): $($contentIssues -join '; ')"
        }
        $targetDir = Split-Path -Parent $targetPath
        if (-not (Test-Path $targetDir)) {
            New-Item -ItemType Directory -Path $targetDir -Force | Out-Null
        }
        $expanded = Remove-InvalidTextControlCharacters -Text $expanded
        Set-Content -Path $targetPath -Value $expanded
        $result.Status = 'Generated'
    }

    $results += $result
}

$results
