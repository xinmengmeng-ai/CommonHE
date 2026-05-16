[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('start', 'answer', 'propose', 'confirm', 'status', 'bootstrap', 'postcheck', 'precheck', 'doctor')]
    [string]$Stage,
    [string]$SessionRoot,
    [string]$ProjectRoot,
    [string]$InputText,
    [string]$Choice,
    [string]$TargetRoot,
    [string]$ValuesPath,
    [string]$Provider,
    [string]$Model,
    [string]$BaseUrl,
    [string]$ApiKey,
    [switch]$Execute,
    [switch]$Force
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function ConvertFrom-WindowsVerbatimPath {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path)) {
        return $Path
    }

    if ($Path.StartsWith('\\?\UNC\', [System.StringComparison]::OrdinalIgnoreCase)) {
        return "\\$($Path.Substring(8))"
    }

    if ($Path.StartsWith('\\?\', [System.StringComparison]::OrdinalIgnoreCase)) {
        return $Path.Substring(4)
    }

    return $Path
}

$SessionRoot = ConvertFrom-WindowsVerbatimPath -Path $SessionRoot
$ProjectRoot = ConvertFrom-WindowsVerbatimPath -Path $ProjectRoot
$TargetRoot = ConvertFrom-WindowsVerbatimPath -Path $TargetRoot
$ValuesPath = ConvertFrom-WindowsVerbatimPath -Path $ValuesPath

$rawScriptDir = if ($PSScriptRoot) {
    $PSScriptRoot
} elseif ($PSCommandPath) {
    Split-Path -Parent $PSCommandPath
} else {
    Split-Path -Parent $MyInvocation.MyCommand.Path
}
$scriptDir = ConvertFrom-WindowsVerbatimPath -Path $rawScriptDir
if (-not $scriptDir) {
    throw 'Unable to resolve CommonHE orchestrator script directory.'
}
$packageRoot = Split-Path -Parent $scriptDir
$initScript = Join-Path $scriptDir 'init-common-he.ps1'
$bootstrapHandoffTemplatePath = Join-Path $packageRoot 'init\bootstrap-handoff-template.md'
$requiredCapabilitiesConfigPath = Join-Path $packageRoot 'config\required-capabilities.json'

if (-not $SessionRoot) {
    if (-not $ProjectRoot) {
        throw "SessionRoot or ProjectRoot is required."
    }
    $SessionRoot = Join-Path $ProjectRoot '.commonhe\session'
}

$sessionRoot = [System.IO.Path]::GetFullPath($SessionRoot)

function Normalize-ToArray {
    param($Value)

    if ($null -eq $Value) {
        return @()
    }

    if ($Value -is [string]) {
        return @($Value)
    }

    if ($Value -is [System.Collections.IEnumerable]) {
        return @(foreach ($item in $Value) { $item })
    }

    @($Value)
}

function ConvertTo-CapabilitySelectionSummaryText {
    param($SelectedCapabilities)

    $lines = New-Object System.Collections.ArrayList
    foreach ($capability in (Normalize-ToArray -Value $SelectedCapabilities)) {
        if ($null -eq $capability) {
            continue
        }

        $capabilityId = ''
        $label = ''
        $status = ''
        $detail = ''
        $selected = $true
        $recommended = $false

        if ($capability -is [string]) {
            $capabilityId = [string]$capability
            $label = [string]$capability
            $status = 'selected'
            $detail = '已写入本次初始化协作包'
        } else {
            if ($capability.PSObject.Properties.Match('id').Count -gt 0) { $capabilityId = [string]$capability.id }
            if ($capability.PSObject.Properties.Match('label').Count -gt 0) { $label = [string]$capability.label }
            if ($capability.PSObject.Properties.Match('display_name').Count -gt 0 -and -not $label) { $label = [string]$capability.display_name }
            if ($capability.PSObject.Properties.Match('name').Count -gt 0 -and -not $capabilityId) { $capabilityId = [string]$capability.name }
            if ($capability.PSObject.Properties.Match('status').Count -gt 0) { $status = [string]$capability.status }
            if ($capability.PSObject.Properties.Match('detail').Count -gt 0) { $detail = [string]$capability.detail }
            if ($capability.PSObject.Properties.Match('selected').Count -gt 0) { $selected = [bool]$capability.selected }
            if ($capability.PSObject.Properties.Match('recommended').Count -gt 0) { $recommended = [bool]$capability.recommended }
        }

        if (-not $capabilityId) { $capabilityId = $label }
        if (-not $label) { $label = $capabilityId }
        if (-not $status -or -not $selected) { $status = 'mandatory' }
        if (-not $detail) { $detail = '必需能力，已写入本次初始化协作包' }

        $tag = if ($recommended) {
            '推荐且必选'
        } else {
            '必选'
        }
        [void]$lines.Add("- $label（$capabilityId，$tag，状态：$status）：$detail")
    }

    if ($lines.Count -eq 0) {
        return '- 当前版本没有可取消能力；superpowers、agent-browser、chrome-devtools、GitNexus、Speckit 均为必需能力。'
    }

    [void]$lines.Add('- 当前版本五项能力均为必需能力，不提供取消入口；缺失时必须安装或配置后再继续。')
    @($lines) -join "`n"
}

function ConvertTo-CapabilityScopeNotesText {
    param($Capabilities)

    $names = @(
        foreach ($capability in (Normalize-ToArray -Value $Capabilities)) {
            $name = Get-CapabilityIdentity -Capability $capability
            if ($name) { $name.ToLowerInvariant() }
        }
    ) | Select-Object -Unique

    $lines = New-Object System.Collections.ArrayList
    if ($names -contains 'chrome-devtools') {
        [void]$lines.Add('- `chrome-devtools`：用于控制台、网络、DOM、性能与页面状态诊断。')
    }
    if ($names -contains 'agent-browser') {
        [void]$lines.Add('- `agent-browser`：用于端到端交互、流程自动化、页面操作与截图取证。')
    }
    if (($names -contains 'chrome-devtools') -and ($names -contains 'agent-browser')) {
        [void]$lines.Add('- 两项浏览器能力同时为绿色，才允许把浏览器相关工作派给 frontend / reviewer / qa。')
    } elseif (($names -contains 'chrome-devtools') -or ($names -contains 'agent-browser')) {
        [void]$lines.Add('- 本轮只启用了已列出的浏览器能力；不得假定另一项浏览器自动化能力可用。')
    } else {
        [void]$lines.Add('- 本轮没有启用浏览器自动化能力；不得派发依赖浏览器自动化的验证任务。')
    }
    if ($names -contains 'speckit') {
        [void]$lines.Add('- `Speckit`：用于后续规范化 feature/spec/plan/tasks 工作流；当前版本为必需能力，必须生成 `.specify/`。')
    } else {
        [void]$lines.Add('- `Speckit` 当前为必需能力；若缺失则必须先安装或配置，不得跳过规范化计划目录。')
    }

    @($lines) -join "`n"
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

function Get-SelectedCapabilityRecordsFromDecision {
    param($Decision)

    try {
        $catalog = Read-RequiredCapabilitiesCatalog
        return @(
            foreach ($capability in $catalog.Capabilities) {
                [pscustomobject]@{
                    name = [string]$capability.name
                    display_name = if ($capability.PSObject.Properties.Match('display_name').Count -gt 0 -and [string]$capability.display_name) { [string]$capability.display_name } else { [string]$capability.name }
                }
            }
        )
    } catch {
        return @(
            [pscustomobject]@{ name = 'superpowers'; display_name = 'superpowers / required skills' }
            [pscustomobject]@{ name = 'agent-browser'; display_name = 'agent-browser' }
            [pscustomobject]@{ name = 'chrome-devtools'; display_name = 'chrome-devtools MCP' }
            [pscustomobject]@{ name = 'GitNexus'; display_name = 'GitNexus' }
            [pscustomobject]@{ name = 'Speckit'; display_name = 'Speckit' }
        )
    }
}

function Get-MandatorySelectedCapabilityRecords {
    param(
        $RequiredCapabilities,
        $ProbeResults
    )

    $probeByName = @{}
    foreach ($probe in (Normalize-ToArray -Value $ProbeResults)) {
        $probeName = Get-CapabilityIdentity -Capability $probe
        if ($probeName) {
            $probeByName[$probeName.ToLowerInvariant()] = $probe
        }
    }

    @(
        foreach ($capability in (Normalize-ToArray -Value $RequiredCapabilities)) {
            $name = Get-CapabilityIdentity -Capability $capability
            if (-not $name) {
                continue
            }

            $displayName = $name
            if ($capability.PSObject.Properties.Match('display_name').Count -gt 0 -and [string]$capability.display_name) {
                $displayName = [string]$capability.display_name
            }

            $probe = $null
            $key = $name.ToLowerInvariant()
            if ($probeByName.ContainsKey($key)) {
                $probe = $probeByName[$key]
            }

            $detail = '必需能力，缺失时必须安装或配置后再继续。'
            if ($null -ne $probe) {
                if ($probe.PSObject.Properties.Match('evidence').Count -gt 0 -and [string]$probe.evidence) {
                    $detail = [string]$probe.evidence
                } elseif ($probe.PSObject.Properties.Match('verification_command').Count -gt 0 -and [string]$probe.verification_command) {
                    $detail = [string]$probe.verification_command
                }
            }

            [pscustomobject]@{
                id = $name
                label = $displayName
                recommended = $true
                selected = $true
                required = $true
                locked = $true
                status = if ($null -ne $probe -and -not [bool]$probe.passed) { 'blocked' } else { 'available' }
                detail = $detail
            }
        }
    )
}

function Get-NormalizedIntegrations {
    param($Integrations)

    $normalized = New-Object System.Collections.ArrayList
    foreach ($integration in (Normalize-ToArray -Value $Integrations)) {
        $integrationName = Get-IntegrationName -Integration $integration
        if ($integrationName) {
            [void]$normalized.Add($integration)
        }
    }
    @($normalized)
}

function Get-FallbackDiscoveryQuestions {
    @(
        [pscustomobject]@{ id = 'project_name'; prompt = '我们先定个名字。你想把这个项目叫什么？' }
        [pscustomobject]@{ id = 'project_goal'; prompt = '你想做一个什么产品？它最核心要解决的问题是什么？' }
        [pscustomobject]@{ id = 'target_users'; prompt = '这个产品主要给谁用？' }
        [pscustomobject]@{ id = 'core_features'; prompt = '如果只先做第一版，你最想先落地的 3-5 个核心功能是什么？' }
        [pscustomobject]@{ id = 'constraints'; prompt = '你更在意什么：上线速度、预算、稳定性、还是后续扩展？有没有明显约束？' }
        [pscustomobject]@{ id = 'existing_assets'; prompt = '这是从零开始，还是基于现有项目继续？有没有必须对接的平台或系统？' }
    )
}

function ConvertTo-DiscoveryQuestionSet {
    param($Questions)

    $requiredIds = @('project_name', 'project_goal', 'target_users', 'core_features', 'constraints', 'existing_assets')
    $normalized = New-Object System.Collections.ArrayList
    foreach ($id in $requiredIds) {
        $match = @($Questions | Where-Object { [string]$_.id -eq $id } | Select-Object -First 1)
        $prompt = if (@($match).Count -gt 0 -and [string]$match[0].prompt) {
            [string]$match[0].prompt
        } else {
            [string](@(Get-FallbackDiscoveryQuestions | Where-Object { [string]$_.id -eq $id } | Select-Object -First 1)[0].prompt)
        }

        [void]$normalized.Add([pscustomobject]@{
            id = $id
            prompt = $prompt
        })
    }

    @($normalized)
}

function Get-OpenAICompatibleDiscoveryQuestions {
    param(
        [string]$ProjectRoot,
        [string]$Provider,
        [string]$Model,
        [string]$BaseUrl,
        [string]$ApiKey
    )

    if (-not $ApiKey) { return $null }
    $effectiveProvider = if ($Provider) { $Provider.Trim().ToLowerInvariant() } else { '' }
    if ($effectiveProvider -notin @('custom', 'openai', 'codex')) { return $null }

    $effectiveBaseUrl = if ($BaseUrl) { $BaseUrl.TrimEnd('/') } else { 'https://api.openai.com/v1' }
    $endpoint = if ($effectiveBaseUrl.EndsWith('/chat/completions')) {
        $effectiveBaseUrl
    } else {
        "$effectiveBaseUrl/chat/completions"
    }
    $effectiveModel = if ($Model) { $Model } else { 'gpt-4.1-mini' }
    $projectSignals = Get-ProjectSignalText -ProjectRoot $ProjectRoot
    if (-not $projectSignals) { $projectSignals = '空目录或暂无项目文件。' }

    $systemPrompt = '你是 CommonHE 初始化问题生成器。必须返回 JSON，不要输出 Markdown。保持 id 固定，只重写 prompt，使问题更贴合当前项目。'
    $userPrompt = @"
请基于以下项目线索生成 6 个初始化问题。必须返回：
{"questions":[{"id":"project_name","prompt":"..."},{"id":"project_goal","prompt":"..."},{"id":"target_users","prompt":"..."},{"id":"core_features","prompt":"..."},{"id":"constraints","prompt":"..."},{"id":"existing_assets","prompt":"..."}]}

项目线索：
$projectSignals
"@

    try {
        $body = @{
            model = $effectiveModel
            messages = @(
                @{ role = 'system'; content = $systemPrompt }
                @{ role = 'user'; content = $userPrompt }
            )
            temperature = 0.2
        } | ConvertTo-Json -Depth 8

        $headers = @{
            Authorization = "Bearer $ApiKey"
            'Content-Type' = 'application/json'
        }
        $response = Invoke-RestMethod -Method Post -Uri $endpoint -Headers $headers -Body $body -TimeoutSec 20
        $content = [string]$response.choices[0].message.content
        if (-not $content) { return $null }
        $json = $content.Trim()
        if ($json.StartsWith('```')) {
            $json = ($json -replace '^```json\s*', '' -replace '^```\s*', '' -replace '\s*```$', '').Trim()
        }
        $parsed = $json | ConvertFrom-Json
        if (-not $parsed.questions) { return $null }
        ConvertTo-DiscoveryQuestionSet -Questions $parsed.questions
    } catch {
        $null
    }
}

function Get-DiscoveryQuestionPack {
    param(
        [string]$ProjectRoot,
        [string]$Provider,
        [string]$Model,
        [string]$BaseUrl,
        [string]$ApiKey
    )

    $llmQuestions = Get-OpenAICompatibleDiscoveryQuestions -ProjectRoot $ProjectRoot -Provider $Provider -Model $Model -BaseUrl $BaseUrl -ApiKey $ApiKey
    if ($llmQuestions) {
        return [pscustomobject]@{
            Source = 'LLM generated'
            Questions = @($llmQuestions)
        }
    }

    [pscustomobject]@{
        Source = 'fallback template'
        Questions = @(Get-FallbackDiscoveryQuestions)
    }
}

function Get-ProjectSignalText {
    param([string]$ProjectRoot)

    $buffer = New-Object System.Collections.ArrayList
    $readmePath = Join-Path $ProjectRoot 'README.md'
    if (Test-Path $readmePath) {
        [void]$buffer.Add((Get-Content -Raw $readmePath))
    }

    $docsDir = Join-Path $ProjectRoot 'docs'
    if (Test-Path $docsDir) {
        foreach ($docFile in (Get-ChildItem -Path $docsDir -Recurse -File -Filter *.md | Select-Object -First 5)) {
            [void]$buffer.Add((Get-Content -Raw $docFile.FullName))
        }
    }

    $topLevelNames = @(
        Get-ChildItem -Path $ProjectRoot -Force |
            Select-Object -ExpandProperty Name
    )
    if ($topLevelNames.Count -gt 0) {
        [void]$buffer.Add(($topLevelNames -join ' '))
    }

    (($buffer | ForEach-Object { [string]$_ }) -join ' ')
}

function Test-IsCommonHEPackageReadme {
    param([string]$ProjectRoot)

    $readmePath = Join-Path $ProjectRoot 'README.md'
    if (-not (Test-Path $readmePath)) {
        return $false
    }

    try {
        $content = Get-Content -Raw $readmePath
    } catch {
        return $false
    }

    $markers = @(
        'CommonHE',
        '初始化 CommonHE',
        '当前只生成 HE 协作工程'
    )

    $matched = 0
    foreach ($marker in $markers) {
        if ($content -like "*$marker*") {
            $matched += 1
        }
    }

    ($matched -ge 2)
}

function Get-LegacyProjectSignals {
    param([string]$ProjectRoot)

    $signals = New-Object System.Collections.ArrayList

    $candidateSignals = @(
        @{ Path = '.git'; Label = '检测到 Git 仓库' }
        @{ Path = 'package.json'; Label = '检测到 package.json' }
        @{ Path = 'pnpm-lock.yaml'; Label = '检测到前端锁文件' }
        @{ Path = 'pom.xml'; Label = '检测到 Maven 构建文件' }
        @{ Path = 'build.gradle'; Label = '检测到 Gradle 构建文件' }
        @{ Path = 'requirements.txt'; Label = '检测到 Python 依赖文件' }
        @{ Path = 'pyproject.toml'; Label = '检测到 Python 项目配置' }
        @{ Path = 'src'; Label = '检测到源码目录 src/' }
        @{ Path = 'app'; Label = '检测到源码目录 app/' }
        @{ Path = 'tests'; Label = '检测到测试目录 tests/' }
        @{ Path = 'Dockerfile'; Label = '检测到 Dockerfile' }
        @{ Path = 'docker-compose.yml'; Label = '检测到 docker-compose.yml' }
        @{ Path = 'nginx.conf'; Label = '检测到 nginx 配置' }
        @{ Path = 'docs'; Label = '检测到现有 docs/' }
        @{ Path = 'README.md'; Label = '检测到现有 README.md' }
        @{ Path = 'migrations'; Label = '检测到迁移目录 migrations/' }
        @{ Path = 'sql'; Label = '检测到 SQL 目录 sql/' }
    )

    foreach ($candidate in $candidateSignals) {
        if (
            ($candidate.Path -eq 'README.md') -and
            (Test-IsCommonHEPackageReadme -ProjectRoot $ProjectRoot)
        ) {
            continue
        }

        if (Test-Path (Join-Path $ProjectRoot $candidate.Path)) {
            [void]$signals.Add([string]$candidate.Label)
        }
    }

    @($signals | Sort-Object -Unique)
}

function Test-IsLegacyProject {
    param([string]$ProjectRoot)

    (@(Get-LegacyProjectSignals -ProjectRoot $ProjectRoot).Count -gt 0)
}

function Get-LegacyProjectAnalysis {
    param([string]$ProjectRoot)

    $projectName = Split-Path -Leaf $ProjectRoot
    $signalText = Get-ProjectSignalText -ProjectRoot $ProjectRoot
    $signals = @(Get-LegacyProjectSignals -ProjectRoot $ProjectRoot)
    $analysisAnswers = @{ summary = $signalText }

    $packageJsonPath = Join-Path $ProjectRoot 'package.json'
    $packageJsonText = if (Test-Path $packageJsonPath) { Get-Content -Raw $packageJsonPath } else { '' }
    $readmeText = if (Test-Path (Join-Path $ProjectRoot 'README.md')) { Get-Content -Raw (Join-Path $ProjectRoot 'README.md') } else { '' }
    $combinedSignalText = @($signalText, $packageJsonText, $readmeText) -join "`n"

    $hasFrontend = (Test-Path (Join-Path $ProjectRoot 'package.json')) -or (Test-Path (Join-Path $ProjectRoot 'src')) -or (Test-Path (Join-Path $ProjectRoot 'public'))
    $hasBackend = (Test-Path (Join-Path $ProjectRoot 'pom.xml')) -or (Test-Path (Join-Path $ProjectRoot 'build.gradle')) -or (Test-Path (Join-Path $ProjectRoot 'requirements.txt')) -or (Test-Path (Join-Path $ProjectRoot 'pyproject.toml')) -or (Test-Path (Join-Path $ProjectRoot 'src\main\java')) -or ($packageJsonText -match '"(express|fastify|koa|nestjs)"')
    $hasTests = (Test-Path (Join-Path $ProjectRoot 'tests')) -or (Test-Path (Join-Path $ProjectRoot 'src\test')) -or ($packageJsonText -match '"(vitest|jest|playwright|cypress|testing-library)"') -or ($packageJsonText -match '"test"\s*:')
    $hasDatabase = (Test-Path (Join-Path $ProjectRoot 'migrations')) -or (Test-Path (Join-Path $ProjectRoot 'sql')) -or ($combinedSignalText -match 'prisma|typeorm|sequelize|mysql|postgres|migration|schema')
    $hasDeploy = (Test-Path (Join-Path $ProjectRoot 'Dockerfile')) -or (Test-Path (Join-Path $ProjectRoot 'docker-compose.yml')) -or (Test-Path (Join-Path $ProjectRoot '.github\workflows')) -or (Test-Path (Join-Path $ProjectRoot 'nginx.conf')) -or (Test-Path (Join-Path $ProjectRoot 'k8s')) -or ($combinedSignalText -match 'deploy|docker|vercel|nginx|workflow')
    $isComplexMonorepo = (Test-Path (Join-Path $ProjectRoot 'apps')) -or (Test-Path (Join-Path $ProjectRoot 'packages')) -or (Test-Path (Join-Path $ProjectRoot 'services'))

    $deliveryMode = Get-DeliveryMode -Answers $analysisAnswers
    $projectType = Get-DetectedProjectType -Answers $analysisAnswers
    $integrations = @(Get-DetectedIntegrations -Answers $analysisAnswers)
    $signalCategories = New-Object System.Collections.ArrayList
    if ($hasFrontend) { [void]$signalCategories.Add('frontend') }
    if ($hasBackend) { [void]$signalCategories.Add('backend') }
    if ($hasTests) { [void]$signalCategories.Add('tests') }
    if ($hasDatabase) { [void]$signalCategories.Add('database') }
    if ($hasDeploy) { [void]$signalCategories.Add('deploy') }
    if ((Test-Path (Join-Path $ProjectRoot 'README.md')) -or (Test-Path (Join-Path $ProjectRoot 'docs'))) { [void]$signalCategories.Add('docs') }

    $rolesNow = New-Object System.Collections.ArrayList
    $rolesLater = New-Object System.Collections.ArrayList
    $roleRationale = @{}

    if (($hasBackend -and $hasFrontend) -or $isComplexMonorepo) {
        [void]$rolesNow.Add('architect')
        $roleRationale['architect'] = '同时识别到前后端或多模块结构，当前阶段需要架构边界与任务编排。'
    }
    if ($hasBackend) {
        [void]$rolesNow.Add('backend')
        $roleRationale['backend'] = '识别到服务端框架、API 目录或后端依赖信号，应启用 backend 角色。'
    }
    if ($hasFrontend) {
        [void]$rolesNow.Add('frontend')
        $roleRationale['frontend'] = '识别到前端框架、页面目录或前端构建链路，应启用 frontend 角色。'
    }
    [void]$rolesNow.Add('reviewer')
    $roleRationale['reviewer'] = 'reviewer 是默认启用角色，用于结构审查、风险识别与变更核验。'
    [void]$rolesNow.Add('docs')
    $roleRationale['docs'] = 'docs 是默认启用角色，用于维护项目真源与协作边界。'
    if ($hasTests) {
        [void]$rolesNow.Add('qa')
        $roleRationale['qa'] = '识别到测试目录、测试依赖或验证脚本，应启用 qa 角色。'
    }
    if ($hasDatabase) {
        [void]$rolesNow.Add('database')
        $roleRationale['database'] = '识别到 migration、schema 或 SQL 信号，应启用 database 角色。'
    } elseif ($hasBackend) {
        [void]$rolesLater.Add('database')
    }
    if ($hasDeploy) {
        [void]$rolesLater.Add('devops')
        $roleRationale['devops'] = '识别到 Docker、CI、nginx 或部署配置，后续应启用 devops 角色。'
    }
    if ($hasBackend -or $hasFrontend) { [void]$rolesLater.Add('compliance') }
    foreach ($integration in $integrations) {
        $integrationName = Get-IntegrationName -Integration $integration
        if ($integrationName) {
            [void]$rolesLater.Add("integration-$integrationName")
        }
    }

    if (@($rolesNow).Count -eq 0) {
        foreach ($role in @('reviewer', 'docs')) {
            [void]$rolesNow.Add($role)
        }
    }

    $externalReferences = @()
    $readmePath = Join-Path $ProjectRoot 'README.md'
    if (Test-Path $readmePath) {
        $externalReferences += [pscustomobject]@{
            type = 'existing-readme'
            path = $readmePath
            purpose = '现有项目说明与上下文参考'
            must_read = $true
        }
    }

    $stageConstraints = @(
        '当前阶段优先尊重现有项目结构与交付边界。'
    )
    if ($deliveryMode -in @('landing-page', 'solution-site', 'showcase-site')) {
        $stageConstraints += '当前阶段优先前台交付，不自动扩展为平台工程。'
    }

    $deferredCapabilities = @()
    if ($deliveryMode -in @('landing-page', 'solution-site', 'showcase-site')) {
        $deferredCapabilities += @('平台化能力', '后端服务', '深度第三方集成')
    }
    if (-not $hasDeploy) {
        $deferredCapabilities += '部署治理细化'
    }

    $assumptions = New-Object System.Collections.ArrayList
    [void]$assumptions.Add('当前为老项目零提问初始化，未向用户追加 discovery 问答。')
    if (-not $hasTests) {
        [void]$assumptions.Add('未识别到明确测试目录，当前按保守方式建模测试门禁。')
    }
    if (-not $hasBackend -and -not $hasFrontend) {
        [void]$assumptions.Add('未识别到明确业务代码目录，当前按保守协作模式初始化。')
    }
    if (-not $hasDeploy) {
        [void]$assumptions.Add('未识别到明确部署治理信号，当前仅将 devops 记录为后续可启用角色。')
    }

    $analysisConfidence = if ($signals.Count -ge 5) {
        'high'
    } elseif ($signals.Count -ge 3) {
        'medium'
    } else {
        'low'
    }

    $projectGoalSummary = if ($hasBackend -and $hasFrontend) {
        '基于现有前后端项目结构补齐 CommonHE 协作工程，并推进首轮实施。'
    } elseif ($hasFrontend) {
        '基于现有前端项目补齐 CommonHE 协作工程，并推进首轮实施。'
    } elseif ($hasBackend) {
        '基于现有后端项目补齐 CommonHE 协作工程，并推进首轮实施。'
    } else {
        '基于现有项目目录补齐 CommonHE 协作工程，并推进首轮实施。'
    }

    $coreFeaturesSummary = if ($hasBackend -and $hasFrontend) {
        '保留现有前后端结构、补齐协作协议、围绕已有代码继续实施'
    } elseif ($hasFrontend) {
        '保留现有前端结构、补齐协作协议、围绕已有页面与交互继续实施'
    } elseif ($hasBackend) {
        '保留现有后端结构、补齐协作协议、围绕已有接口与服务继续实施'
    } else {
        '保留现有项目结构、补齐协作协议、围绕已有内容继续实施'
    }

    $constraintsSummary = '优先尊重现有项目结构，避免擅自平台化、AI 化或框架化。'
    $currentStage = 'implementation-v1'
    $currentStageGoal = if ($deliveryMode -in @('landing-page', 'solution-site', 'showcase-site')) {
        '完成当前展示型项目的首个可交付版本'
    } else {
        '完成当前业务目标的首轮实现'
    }

    $enabledRolesNow = @($rolesNow | Sort-Object -Unique)
    $dominantWorkstreams = New-Object System.Collections.ArrayList
    if ($hasFrontend) { [void]$dominantWorkstreams.Add('frontend') }
    if ($hasBackend) { [void]$dominantWorkstreams.Add('backend') }
    if ($hasDatabase) { [void]$dominantWorkstreams.Add('database') }
    if ($hasTests) { [void]$dominantWorkstreams.Add('qa') }
    if ($dominantWorkstreams.Count -eq 0) {
        [void]$dominantWorkstreams.Add((Get-PrimaryWorkstream -DeliveryMode $deliveryMode))
    }
    $confidenceBreakdown = @{
        signals = if ($signals.Count -ge 5) { 'high' } elseif ($signals.Count -ge 3) { 'medium' } else { 'low' }
        structure = if ($hasBackend -or $hasFrontend) { 'high' } else { 'medium' }
        docs = if ((Test-Path (Join-Path $ProjectRoot 'README.md')) -or (Test-Path (Join-Path $ProjectRoot 'docs'))) { 'medium' } else { 'low' }
        integrations = if ($integrations.Count -gt 0) { 'medium' } else { 'low' }
    }

    [pscustomobject]@{
        project_name = $projectName
        project_type = $projectType
        delivery_mode = $deliveryMode
        solution_mode = 'balanced'
        enabled_roles = @($enabledRolesNow)
        recommended_roles_now = @($enabledRolesNow)
        available_roles_later = @($rolesLater | Sort-Object -Unique)
        integrations = @($integrations)
        detected_integrations = @($integrations)
        external_references = @($externalReferences)
        current_stage = $currentStage
        current_stage_goal = $currentStageGoal
        primary_workstream = Get-PrimaryWorkstream -DeliveryMode $deliveryMode
        stage_constraints = @($stageConstraints | Sort-Object -Unique)
        deferred_capabilities = @($deferredCapabilities | Sort-Object -Unique)
        implementation_checklist_seed = @(Get-ImplementationChecklistSeed -Answers $analysisAnswers -DeliveryMode $deliveryMode -ExternalReferences $externalReferences -StageConstraints $stageConstraints -EnabledRoles $enabledRolesNow)
        implementation_acceptance_seed = @(Get-ImplementationAcceptanceSeed -DeliveryMode $deliveryMode -ExternalReferences $externalReferences -EnabledRoles $enabledRolesNow)
        project_goal_summary = $projectGoalSummary
        target_users_summary = '待在后续实施阶段结合现有产品上下文继续细化'
        core_features_summary = $coreFeaturesSummary
        constraints_summary = $constraintsSummary
        legacy_analysis_version = 'v2'
        signal_categories = @($signalCategories | Sort-Object -Unique)
        role_rationale = $roleRationale
        confidence_breakdown = $confidenceBreakdown
        dominant_workstreams = @($dominantWorkstreams | Sort-Object -Unique)
        analysis_confidence = $analysisConfidence
        autodiscovery_signals = @($signals)
        autodiscovery_assumptions = @($assumptions | Sort-Object -Unique)
    }
}

function Write-ProjectAnalysisArtifacts {
    param($Analysis)

    Write-JsonFile -Path (Get-SessionPath 'project-analysis.json') -Value $Analysis

    $lines = New-Object System.Collections.ArrayList
    [void]$lines.Add('# Project Analysis')
    [void]$lines.Add('')
    [void]$lines.Add("- project_name: $($Analysis.project_name)")
    [void]$lines.Add("- project_type: $($Analysis.project_type)")
    [void]$lines.Add("- delivery_mode: $($Analysis.delivery_mode)")
    [void]$lines.Add("- legacy_analysis_version: $($Analysis.legacy_analysis_version)")
    [void]$lines.Add("- analysis_confidence: $($Analysis.analysis_confidence)")
    [void]$lines.Add('')
    [void]$lines.Add('## signal_categories')
    foreach ($item in (Normalize-ToArray -Value $Analysis.signal_categories)) {
        [void]$lines.Add("- $item")
    }
    [void]$lines.Add('')
    [void]$lines.Add('## autodiscovery_signals')
    foreach ($item in (Normalize-ToArray -Value $Analysis.autodiscovery_signals)) {
        [void]$lines.Add("- $item")
    }
    [void]$lines.Add('')
    [void]$lines.Add('## role_rationale')
    foreach ($property in $Analysis.role_rationale.PSObject.Properties) {
        [void]$lines.Add("- $($property.Name): $($property.Value)")
    }
    [void]$lines.Add('')
    [void]$lines.Add('## autodiscovery_assumptions')
    foreach ($item in (Normalize-ToArray -Value $Analysis.autodiscovery_assumptions)) {
        [void]$lines.Add("- $item")
    }

    Set-Content -Path (Get-SessionPath 'project-analysis.md') -Value ($lines -join "`r`n")
}

function Write-AutodiscoveryDecision {
    param(
        $Analysis,
        $PrecheckSummary
    )

    $decision = @{
        user_confirmed = $true
        auto_confirmed = $true
        confirmation_mode = 'auto_legacy_init'
        discovery_mode = 'legacy_zero_question'
        project_name = [string]$Analysis.project_name
        project_type = [string]$Analysis.project_type
        delivery_mode = [string]$Analysis.delivery_mode
        solution_mode = [string]$Analysis.solution_mode
        enabled_roles = @(Normalize-ToArray -Value $Analysis.enabled_roles)
        recommended_roles_now = @(Normalize-ToArray -Value $Analysis.recommended_roles_now)
        available_roles_later = @(Normalize-ToArray -Value $Analysis.available_roles_later)
        integrations = @(Normalize-ToArray -Value $Analysis.integrations)
        detected_integrations = @(Normalize-ToArray -Value $Analysis.detected_integrations)
        external_references = @(Normalize-ToArray -Value $Analysis.external_references)
        current_stage = [string]$Analysis.current_stage
        current_stage_goal = [string]$Analysis.current_stage_goal
        primary_workstream = [string]$Analysis.primary_workstream
        stage_constraints = @(Normalize-ToArray -Value $Analysis.stage_constraints)
        deferred_capabilities = @(Normalize-ToArray -Value $Analysis.deferred_capabilities)
        implementation_checklist_seed = @(Normalize-ToArray -Value $Analysis.implementation_checklist_seed)
        implementation_acceptance_seed = @(Normalize-ToArray -Value $Analysis.implementation_acceptance_seed)
        required_capabilities = @(
            foreach ($capability in (Normalize-ToArray -Value $PrecheckSummary.results)) {
                @{
                    name = [string]$capability.name
                    display_name = [string]$capability.display_name
                }
            }
        )
        capability_probe_results = @(Normalize-ToArray -Value $PrecheckSummary.results)
        legacy_analysis_version = [string]$Analysis.legacy_analysis_version
        signal_categories = @(Normalize-ToArray -Value $Analysis.signal_categories)
        role_rationale = $Analysis.role_rationale
        confidence_breakdown = $Analysis.confidence_breakdown
        dominant_workstreams = @(Normalize-ToArray -Value $Analysis.dominant_workstreams)
        kickoff_pack = @{
            implementation_kickoff = 'docs/workflow/implementation-kickoff.md'
            first_sprint_contract = 'docs/workflow/first-sprint-contract.md'
            first_task_pack = 'docs/workflow/first-task-pack.md'
        }
        analysis_confidence = [string]$Analysis.analysis_confidence
        autodiscovery_signals = @(Normalize-ToArray -Value $Analysis.autodiscovery_signals)
        autodiscovery_assumptions = @(Normalize-ToArray -Value $Analysis.autodiscovery_assumptions)
        project_goal_summary = [string]$Analysis.project_goal_summary
        target_users_summary = [string]$Analysis.target_users_summary
        core_features_summary = [string]$Analysis.core_features_summary
        constraints_summary = [string]$Analysis.constraints_summary
    }

    Write-JsonFile -Path (Get-SessionPath 'decision.json') -Value $decision
    $decision
}

function Get-SessionPath {
    param([string]$Name)
    Join-Path $sessionRoot $Name
}

function Read-JsonFile {
    param([string]$Path)
    if (-not (Test-Path $Path)) {
        throw "Required session file not found: $Path"
    }
    Get-Content -Raw $Path | ConvertFrom-Json
}

function Write-JsonFile {
    param(
        [string]$Path,
        [object]$Value
    )
    $json = $Value | ConvertTo-Json -Depth 10
    $json = Remove-InvalidTextControlCharacters -Text $json
    Set-Content -Path $Path -Value $json
}

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

function Get-StatusPropertyValue {
    param(
        $StatusObject,
        [string]$PropertyName,
        $DefaultValue
    )

    if ($StatusObject -and $StatusObject.PSObject.Properties.Name -contains $PropertyName) {
        return $StatusObject.$PropertyName
    }

    $DefaultValue
}

function Set-ObjectPropertyValue {
    param(
        $ObjectValue,
        [string]$PropertyName,
        $PropertyValue
    )

    if ($ObjectValue.PSObject.Properties.Name -contains $PropertyName) {
        $ObjectValue.$PropertyName = $PropertyValue
    } else {
        Add-Member -InputObject $ObjectValue -NotePropertyName $PropertyName -NotePropertyValue $PropertyValue
    }
}

function Get-ResolvedProjectRoot {
    if ($ProjectRoot) {
        return [System.IO.Path]::GetFullPath($ProjectRoot)
    }

    if ($TargetRoot) {
        return [System.IO.Path]::GetFullPath($TargetRoot)
    }

    $sessionParent = Split-Path -Parent $sessionRoot
    if ($sessionParent) {
        return [System.IO.Path]::GetFullPath($sessionParent)
    }

    throw 'ProjectRoot or TargetRoot is required to resolve the project root.'
}

function Get-UserHomePath {
    if ($env:COMMONHE_HOME_OVERRIDE) {
        return [System.IO.Path]::GetFullPath($env:COMMONHE_HOME_OVERRIDE)
    }
    if ($env:USERPROFILE) {
        return [System.IO.Path]::GetFullPath($env:USERPROFILE)
    }
    if ($HOME) {
        return [System.IO.Path]::GetFullPath($HOME)
    }

    throw 'Unable to resolve user home path.'
}

function Resolve-CapabilityPath {
    param([string]$PathValue)

    if (-not $PathValue) {
        return ''
    }

    if ($PathValue.StartsWith('~/') -or $PathValue.StartsWith('~\')) {
        $relative = $PathValue.Substring(2).Replace('/', '\')
        return [System.IO.Path]::GetFullPath((Join-Path (Get-UserHomePath) $relative))
    }

    [System.IO.Path]::GetFullPath($PathValue)
}

function Get-RequiredCapabilitiesCatalogPath {
    if ($env:COMMONHE_REQUIRED_CAPABILITIES_PATH) {
        return [System.IO.Path]::GetFullPath($env:COMMONHE_REQUIRED_CAPABILITIES_PATH)
    }

    [System.IO.Path]::GetFullPath($requiredCapabilitiesConfigPath)
}

function Read-RequiredCapabilitiesCatalog {
    $catalogPath = Get-RequiredCapabilitiesCatalogPath
    if (-not (Test-Path $catalogPath)) {
        throw "Required capabilities catalog not found: $catalogPath"
    }

    $catalogContent = Get-Content -Raw $catalogPath | ConvertFrom-Json

    [pscustomobject]@{
        Path = $catalogPath
        Capabilities = @(Normalize-ToArray -Value $catalogContent)
    }
}

function Test-CapabilityFileExistsProbe {
    param($Probe)

    $pathValues = @()
    if ($Probe.PSObject.Properties.Name -contains 'paths') {
        $pathValues = @(Normalize-ToArray -Value $Probe.paths)
    } elseif ($Probe.PSObject.Properties.Name -contains 'path') {
        $pathValues = @([string]$Probe.path)
    }

    $resolvedPaths = @(
        foreach ($pathValue in $pathValues) {
            $resolved = Resolve-CapabilityPath -PathValue ([string]$pathValue)
            if ($resolved) { $resolved }
        }
    )
    $matchedPath = $resolvedPaths | Where-Object { Test-Path $_ } | Select-Object -First 1
    [pscustomobject]@{
        Passed = [bool]$matchedPath
        Evidence = if ($matchedPath) { "found: $matchedPath" } else { "missing: $([string]::Join(', ', $resolvedPaths))" }
    }
}

function Test-CapabilityFileContainsProbe {
    param($Probe)

    $pathValues = @()
    if ($Probe.PSObject.Properties.Name -contains 'paths') {
        $pathValues = @(Normalize-ToArray -Value $Probe.paths)
    } elseif ($Probe.PSObject.Properties.Name -contains 'path') {
        $pathValues = @([string]$Probe.path)
    }

    $resolvedPaths = @(
        foreach ($pathValue in $pathValues) {
            $resolved = Resolve-CapabilityPath -PathValue ([string]$pathValue)
            if ($resolved) { $resolved }
        }
    )
    if ($resolvedPaths.Count -eq 0 -and $Probe.PSObject.Properties.Name -contains 'path') {
        $singleResolvedPath = Resolve-CapabilityPath -PathValue ([string]$Probe.path)
        if ($singleResolvedPath) {
            $resolvedPaths = @($singleResolvedPath)
        }
    }

    $patternValues = @()
    if ($Probe.PSObject.Properties.Name -contains 'patterns') {
        $patternValues = @(Normalize-ToArray -Value $Probe.patterns)
    } elseif ($Probe.PSObject.Properties.Name -contains 'pattern') {
        $patternValues = @([string]$Probe.pattern)
    }

    $patterns = @(
        foreach ($patternValue in $patternValues) {
            $normalizedPattern = [string]$patternValue
            if ($normalizedPattern) { $normalizedPattern }
        }
    )
    if ($patterns.Count -eq 0 -and $Probe.PSObject.Properties.Name -contains 'pattern') {
        $singlePattern = [string]$Probe.pattern
        if ($singlePattern) {
            $patterns = @($singlePattern)
        }
    }

    if ($resolvedPaths.Count -eq 0) {
        throw 'file_contains capability probe requires path or paths.'
    }
    if ($patterns.Count -eq 0) {
        throw 'file_contains capability probe requires pattern or patterns.'
    }

    foreach ($resolvedPath in $resolvedPaths) {
        if (-not (Test-Path $resolvedPath)) {
            continue
        }

        $content = Get-Content -Raw $resolvedPath
        $missingPatterns = @(
            foreach ($pattern in $patterns) {
                if (-not $content.Contains([string]$pattern)) {
                    $pattern
                }
            }
        )
        if ($missingPatterns.Count -eq 0) {
            return [pscustomobject]@{
                Passed = $true
                Evidence = "found patterns in: $resolvedPath"
            }
        }
        return [pscustomobject]@{
            Passed = $false
            Evidence = "missing patterns in: $resolvedPath -> $([string]::Join(', ', $missingPatterns))"
        }
    }

    [pscustomobject]@{
        Passed = $false
        Evidence = "missing files: $([string]::Join(', ', $resolvedPaths))"
    }
}

function Get-SkillCandidatePaths {
    param([string]$SkillName)

    $homePath = Get-UserHomePath
    @(
        [System.IO.Path]::GetFullPath((Join-Path $homePath ".agents\skills\$SkillName\SKILL.md"))
        [System.IO.Path]::GetFullPath((Join-Path $homePath ".codex\superpowers\skills\$SkillName\SKILL.md"))
        [System.IO.Path]::GetFullPath((Join-Path $homePath ".codex\skills\$SkillName\SKILL.md"))
    ) | Select-Object -Unique
}

function Test-CapabilitySkillPresenceProbe {
    param($Probe)

    $skillEvidence = New-Object System.Collections.ArrayList
    $missingSkills = New-Object System.Collections.ArrayList

    foreach ($skillName in (Normalize-ToArray -Value $Probe.skill_names)) {
        $candidatePath = Get-SkillCandidatePaths -SkillName ([string]$skillName) | Where-Object { Test-Path $_ } | Select-Object -First 1
        if ($candidatePath) {
            [void]$skillEvidence.Add("$skillName -> $candidatePath")
        } else {
            [void]$missingSkills.Add([string]$skillName)
            [void]$skillEvidence.Add("$skillName -> missing")
        }
    }

    [pscustomobject]@{
        Passed = (@($missingSkills).Count -eq 0)
        Evidence = [string]::Join('; ', @($skillEvidence))
    }
}

function Test-CapabilityCommandProbe {
    param($Probe)

    $commands = @(
        foreach ($commandValue in (Normalize-ToArray -Value $Probe.commands)) {
            $normalized = [string]$commandValue
            if ($normalized) { $normalized }
        }
    )
    if ($commands.Count -eq 0 -and $Probe.PSObject.Properties.Name -contains 'command') {
        $commandText = [string]$Probe.command
        if ($commandText) {
            $commands = @($commandText)
        }
    }

    if ($commands.Count -eq 0) {
        throw 'Command capability probe requires command or commands.'
    }

    $evidenceLines = New-Object System.Collections.ArrayList
    foreach ($commandText in $commands) {
        $processInfo = [System.Diagnostics.ProcessStartInfo]::new()
        $processInfo.FileName = 'cmd.exe'
        $processInfo.Arguments = "/d /c $commandText"
        $processInfo.UseShellExecute = $false
        $processInfo.RedirectStandardOutput = $true
        $processInfo.RedirectStandardError = $true

        $process = [System.Diagnostics.Process]::Start($processInfo)
        $stdout = $process.StandardOutput.ReadToEnd()
        $stderr = $process.StandardError.ReadToEnd()
        $process.WaitForExit()

        $exitCode = $process.ExitCode
        $combinedOutput = @($stdout, $stderr) -join "`n"
        $trimmedOutput = (($combinedOutput -split "`r?`n" | Where-Object { $_ }) | Select-Object -First 3) -join ' | '

        if ($exitCode -eq 0) {
            [void]$evidenceLines.Add("command ok: $commandText")
            return [pscustomobject]@{
                Passed = $true
                Evidence = [string]::Join(' || ', @($evidenceLines))
            }
        }

        if ($trimmedOutput) {
            [void]$evidenceLines.Add("command failed: $commandText | $trimmedOutput")
        } else {
            [void]$evidenceLines.Add("command failed: $commandText | exit code $exitCode")
        }
    }

    [pscustomobject]@{
        Passed = $false
        Evidence = [string]::Join(' || ', @($evidenceLines))
    }
}

function Invoke-CapabilityProbe {
    param($Probe)

    $probeType = [string]$Probe.type
    switch ($probeType) {
        'file_exists' { return Test-CapabilityFileExistsProbe -Probe $Probe }
        'file_contains' { return Test-CapabilityFileContainsProbe -Probe $Probe }
        'skill_presence' { return Test-CapabilitySkillPresenceProbe -Probe $Probe }
        'command_success' { return Test-CapabilityCommandProbe -Probe $Probe }
        default { throw "Unsupported capability probe type: $probeType" }
    }
}

function Get-CapabilitySummary {
    param(
        [string]$SuccessMessage,
        [string]$FailureLeadText
    )

    $catalog = Read-RequiredCapabilitiesCatalog
    $results = New-Object System.Collections.ArrayList
    $missingCapabilities = New-Object System.Collections.ArrayList
    $missingCapabilityDetails = New-Object System.Collections.ArrayList

    foreach ($capability in $catalog.Capabilities) {
        $probeResults = @(
            foreach ($probe in (Normalize-ToArray -Value $capability.probes)) {
                Invoke-CapabilityProbe -Probe $probe
            }
        )
        $passed = (@($probeResults | Where-Object { -not [bool]$_.Passed }).Count -eq 0)
        $displayName = if ($capability.PSObject.Properties.Name -contains 'display_name') { [string]$capability.display_name } else { [string]$capability.name }
        $group = if ($capability.PSObject.Properties.Name -contains 'group') { [string]$capability.group } else { '' }
        $verifyCommand = if ($capability.PSObject.Properties.Name -contains 'verify_command' -and [string]$capability.verify_command) {
            [string]$capability.verify_command
        } elseif ($capability.PSObject.Properties.Name -contains 'verification_command') {
            [string]$capability.verification_command
        } else {
            ''
        }
        $installCommands = if ($capability.PSObject.Properties.Name -contains 'install_commands') { @(Normalize-ToArray -Value $capability.install_commands) } else { @() }
        $installNotes = if ($capability.PSObject.Properties.Name -contains 'install_notes') { @(Normalize-ToArray -Value $capability.install_notes) } else { @() }
        $remediation = if ($capability.PSObject.Properties.Name -contains 'remediation') { @(Normalize-ToArray -Value $capability.remediation) } else { @() }
        if (@($remediation).Count -eq 0) {
            $remediation = @($installNotes)
        }

        if (-not $passed) {
            [void]$missingCapabilities.Add($displayName)
            [void]$missingCapabilityDetails.Add([pscustomobject]@{
                name = [string]$capability.name
                display_name = $displayName
                group = $group
                verification_command = $verifyCommand
                install_commands = $installCommands
                install_notes = $installNotes
                remediation = $remediation
            })
        }

        [void]$results.Add([pscustomobject]@{
            name = [string]$capability.name
            display_name = $displayName
            group = $group
            passed = $passed
            evidence = (($probeResults | ForEach-Object { [string]$_.Evidence }) -join ' || ')
            verification_command = $verifyCommand
            install_commands = $installCommands
            install_notes = $installNotes
            remediation = $remediation
        })
    }

    $recommendedFix = if (@($missingCapabilities).Count -eq 0) {
        $SuccessMessage
    } else {
        $detailLines = @(
            foreach ($detail in $missingCapabilityDetails) {
                $verifyText = if ($detail.verification_command) { "（验证命令：$($detail.verification_command)）" } else { '' }
                $remediationText = (($detail.remediation | Where-Object { $_ } | Select-Object -First 1) | ForEach-Object { [string]$_ })
                $installText = (($detail.install_commands | Where-Object { $_ } | Select-Object -First 1) | ForEach-Object { [string]$_ })
                $noteText = (($detail.install_notes | Where-Object { $_ } | Select-Object -First 1) | ForEach-Object { [string]$_ })
                if ($remediationText) {
                    "$($detail.display_name)$verifyText：$remediationText"
                } elseif ($installText) {
                    "$($detail.display_name)$verifyText：$installText"
                } elseif ($noteText) {
                    "$($detail.display_name)$verifyText：$noteText"
                } else {
                    "$($detail.display_name)$verifyText"
                }
            }
        )
        "$FailureLeadText$([string]::Join('；', $detailLines))"
    }

    [pscustomobject]@{
        passed = (@($missingCapabilities).Count -eq 0)
        catalog_path = $catalog.Path
        results = @($results)
        missing_capabilities = @($missingCapabilities)
        missing_capability_details = @($missingCapabilityDetails)
        recommended_fix = $recommendedFix
    }
}

function Write-PrecheckArtifacts {
    param($PrecheckSummary)

    Write-JsonFile -Path (Get-SessionPath 'precheck.json') -Value $PrecheckSummary

    $lines = New-Object System.Collections.ArrayList
    [void]$lines.Add('# Precheck')
    [void]$lines.Add('')
    [void]$lines.Add("catalog: $($PrecheckSummary.catalog_path)")
    [void]$lines.Add('')
    [void]$lines.Add('## 结果')
    [void]$lines.Add("- passed: $($PrecheckSummary.passed)")
    foreach ($capability in (Normalize-ToArray -Value $PrecheckSummary.results)) {
        [void]$lines.Add("- $($capability.display_name): $(if ([bool]$capability.passed) { 'pass' } else { 'fail' })")
        [void]$lines.Add("  evidence: $($capability.evidence)")
        if (-not [bool]$capability.passed) {
            if ($capability.PSObject.Properties.Name -contains 'verification_command' -and [string]$capability.verification_command) {
                [void]$lines.Add("  verify: $($capability.verification_command)")
            }
            foreach ($remediation in (Normalize-ToArray -Value $capability.remediation)) {
                [void]$lines.Add("  remediation: $remediation")
            }
        }
    }
    if (@($PrecheckSummary.missing_capabilities).Count -gt 0) {
        [void]$lines.Add('')
        [void]$lines.Add('## 缺失能力')
        foreach ($item in $PrecheckSummary.missing_capabilities) {
            [void]$lines.Add("- $item")
        }
    }
    if (@($PrecheckSummary.missing_capability_details).Count -gt 0) {
        [void]$lines.Add('')
        [void]$lines.Add('## 修复建议')
        foreach ($detail in (Normalize-ToArray -Value $PrecheckSummary.missing_capability_details)) {
            [void]$lines.Add("- $($detail.display_name)")
            if ($detail.PSObject.Properties.Name -contains 'verification_command' -and [string]$detail.verification_command) {
                [void]$lines.Add("  verify: $($detail.verification_command)")
            }
            foreach ($remediation in (Normalize-ToArray -Value $detail.remediation)) {
                [void]$lines.Add("  remediation: $remediation")
            }
        }
    }
    [void]$lines.Add('')
    [void]$lines.Add('## 建议')
    [void]$lines.Add("- $($PrecheckSummary.recommended_fix)")

    Set-Content -Path (Get-SessionPath 'precheck.md') -Value ($lines -join "`r`n")
}

function Write-DoctorArtifacts {
    param($DoctorSummary)

    Write-JsonFile -Path (Get-SessionPath 'doctor.json') -Value $DoctorSummary

    $lines = New-Object System.Collections.ArrayList
    [void]$lines.Add('# Doctor')
    [void]$lines.Add('')
    [void]$lines.Add("catalog: $($DoctorSummary.catalog_path)")
    [void]$lines.Add("- passed: $($DoctorSummary.passed)")
    [void]$lines.Add("- entrycheck_passed: $($DoctorSummary.entrycheck_passed)")
    [void]$lines.Add("- misplaced_package_detected: $($DoctorSummary.misplaced_package_detected)")

    [void]$lines.Add('')
    [void]$lines.Add('## 入口自检')
    foreach ($issue in (Normalize-ToArray -Value $DoctorSummary.entrycheck_issues)) {
        [void]$lines.Add("- $issue")
    }
    if (@(Normalize-ToArray -Value $DoctorSummary.entrycheck_issues).Count -eq 0) {
        [void]$lines.Add('- 当前入口结构通过自检')
    }

    [void]$lines.Add('')
    [void]$lines.Add('## 能力诊断')
    foreach ($capability in (Normalize-ToArray -Value $DoctorSummary.results)) {
        [void]$lines.Add("- $($capability.display_name): $(if ([bool]$capability.passed) { 'pass' } else { 'fail' })")
        if ($capability.PSObject.Properties.Name -contains 'group' -and [string]$capability.group) {
            [void]$lines.Add("  group: $($capability.group)")
        }
        [void]$lines.Add("  evidence: $($capability.evidence)")
        if ($capability.PSObject.Properties.Name -contains 'verification_command' -and [string]$capability.verification_command) {
            [void]$lines.Add("  verify: $($capability.verification_command)")
        }
        foreach ($installCommand in (Normalize-ToArray -Value $capability.install_commands)) {
            [void]$lines.Add("  install: $installCommand")
        }
        foreach ($installNote in (Normalize-ToArray -Value $capability.install_notes)) {
            [void]$lines.Add("  note: $installNote")
        }
        foreach ($remediation in (Normalize-ToArray -Value $capability.remediation)) {
            [void]$lines.Add("  remediation: $remediation")
        }
    }

    if (@($DoctorSummary.missing_capabilities).Count -gt 0) {
        [void]$lines.Add('')
        [void]$lines.Add('## 缺失能力')
        foreach ($item in $DoctorSummary.missing_capabilities) {
            [void]$lines.Add("- $item")
        }
    }

    if (@($DoctorSummary.missing_capability_details).Count -gt 0) {
        [void]$lines.Add('')
        [void]$lines.Add('## 官方安装与验证')
        foreach ($detail in (Normalize-ToArray -Value $DoctorSummary.missing_capability_details)) {
            [void]$lines.Add("- $($detail.display_name)")
            if ($detail.PSObject.Properties.Name -contains 'verification_command' -and [string]$detail.verification_command) {
                [void]$lines.Add("  verify: $($detail.verification_command)")
            }
            foreach ($installCommand in (Normalize-ToArray -Value $detail.install_commands)) {
                [void]$lines.Add("  install: $installCommand")
            }
            foreach ($installNote in (Normalize-ToArray -Value $detail.install_notes)) {
                [void]$lines.Add("  note: $installNote")
            }
        }
    }

    [void]$lines.Add('')
    [void]$lines.Add('## 建议')
    [void]$lines.Add("- $($DoctorSummary.recommended_fix)")

    Set-Content -Path (Get-SessionPath 'doctor.md') -Value ($lines -join "`r`n")
}

function Get-PrecheckFailureMessage {
    param($PrecheckSummary)

    $missingText = [string]::Join('、', @(Normalize-ToArray -Value $PrecheckSummary.missing_capabilities))
    "初始化在 precheck 阶段失败：缺少必需能力 [$missingText]。请先修复依赖后重试；当前不得进入 bootstrap、postcheck 或业务实施。"
}

function Get-DoctorFailureMessage {
    param($DoctorSummary)

    if (-not [bool]$DoctorSummary.entrycheck_passed) {
        return "初始化在 doctor 阶段失败：当前入口结构不正确。$($DoctorSummary.recommended_fix)"
    }

    $missingText = [string]::Join('、', @(Normalize-ToArray -Value $DoctorSummary.missing_capabilities))
    "初始化在 doctor 阶段失败：缺少必需能力 [$missingText]。请先按 doctor 报告修复环境后重试；当前不得进入 precheck、bootstrap 或业务实施。"
}

function Sync-DecisionCapabilities {
    param($CapabilitySummary)

    $decisionPath = Get-SessionPath 'decision.json'
    if (-not (Test-Path $decisionPath)) {
        return
    }

    $decision = Read-JsonFile -Path $decisionPath
    $requiredCapabilities = @(
        foreach ($capability in (Normalize-ToArray -Value $CapabilitySummary.results)) {
            @{
                name = [string]$capability.name
                display_name = [string]$capability.display_name
            }
        }
    )
    $selectedCapabilities = @(Get-MandatorySelectedCapabilityRecords -RequiredCapabilities $requiredCapabilities -ProbeResults $CapabilitySummary.results)

    Set-ObjectPropertyValue -ObjectValue $decision -PropertyName 'required_capabilities' -PropertyValue $requiredCapabilities
    Set-ObjectPropertyValue -ObjectValue $decision -PropertyName 'selected_capabilities' -PropertyValue $selectedCapabilities
    Set-ObjectPropertyValue -ObjectValue $decision -PropertyName 'capability_probe_results' -PropertyValue @(Normalize-ToArray -Value $CapabilitySummary.results)
    Write-JsonFile -Path $decisionPath -Value $decision
}

function Get-EntryCheckSummary {
    param([string]$ProjectRoot)

    $issues = New-Object System.Collections.ArrayList
    $packageMarkers = @('AGENTS.md', 'README.md', 'tools', 'templates')
    $misplacedCandidates = @(
        Get-ChildItem -Path $ProjectRoot -Directory -Force -ErrorAction SilentlyContinue |
            Where-Object { $_.Name -match '^CommonHE(?:-v[\d\.]+)?$' -or $_.Name -match '^CommonHE-v[\d\.]+$' }
    )
    $misplacedPackageDetected = $false

    foreach ($candidate in $misplacedCandidates) {
        $candidateHasMarkers = $true
        foreach ($marker in $packageMarkers) {
            if (-not (Test-Path (Join-Path $candidate.FullName $marker))) {
                $candidateHasMarkers = $false
                break
            }
        }

        if ($candidateHasMarkers) {
            $misplacedPackageDetected = $true
            [void]$issues.Add("检测到疑似错误解压层级：$($candidate.Name)")
            [void]$issues.Add('请把 release 的内容直接解压到目标项目根目录，而不是再套一层 CommonHE-v* 目录。')
        }
    }

    [pscustomobject]@{
        passed = (-not $misplacedPackageDetected)
        entrycheck_passed = (-not $misplacedPackageDetected)
        entrycheck_failed = $misplacedPackageDetected
        misplaced_package_detected = $misplacedPackageDetected
        entrycheck_issues = @($issues)
        recommended_fix = if ($misplacedPackageDetected) {
            '请把 release 的内容直接解压到目标项目根目录，确认当前工作区根目录直接包含 AGENTS.md、tools/、templates/、README.md，再重新执行初始化。'
        } else {
            '当前入口结构通过 doctor 自检，可以继续。'
        }
    }
}

function Invoke-Doctor {
    param(
        [string]$ProjectRoot,
        [switch]$PersistStatus,
        [bool]$LegacyProjectDetected = $false,
        [bool]$LegacyZeroQuestionMode = $false
    )

    $entryCheck = Get-EntryCheckSummary -ProjectRoot $ProjectRoot
    $capabilitySummary = Get-CapabilitySummary `
        -SuccessMessage 'doctor 已通过，环境与入口结构可继续进入 precheck。' `
        -FailureLeadText '请先补齐缺失能力并修复环境后，再重新执行初始化。'
    $summary = [pscustomobject]@{
        passed = ([bool]$entryCheck.passed) -and ([bool]$capabilitySummary.passed)
        catalog_path = $capabilitySummary.catalog_path
        results = @($capabilitySummary.results)
        missing_capabilities = @($capabilitySummary.missing_capabilities)
        missing_capability_details = @($capabilitySummary.missing_capability_details)
        entrycheck_passed = [bool]$entryCheck.entrycheck_passed
        entrycheck_failed = [bool]$entryCheck.entrycheck_failed
        misplaced_package_detected = [bool]$entryCheck.misplaced_package_detected
        entrycheck_issues = @($entryCheck.entrycheck_issues)
        recommended_fix = if (-not [bool]$entryCheck.passed) { [string]$entryCheck.recommended_fix } else { [string]$capabilitySummary.recommended_fix }
    }

    Sync-DecisionCapabilities -CapabilitySummary $summary
    Write-DoctorArtifacts -DoctorSummary $summary

    if ($PersistStatus) {
        $currentStatus = if (Test-Path (Get-SessionPath 'status.json')) {
            Read-JsonFile -Path (Get-SessionPath 'status.json')
        } else {
            $null
        }
        $resolvedLegacyProjectDetected = if ($PSBoundParameters.ContainsKey('LegacyProjectDetected')) {
            $LegacyProjectDetected
        } else {
            [bool](Get-StatusPropertyValue -StatusObject $currentStatus -PropertyName 'legacy_project_detected' -DefaultValue $false)
        }
        $resolvedLegacyZeroQuestionMode = if ($PSBoundParameters.ContainsKey('LegacyZeroQuestionMode')) {
            $LegacyZeroQuestionMode
        } else {
            [bool](Get-StatusPropertyValue -StatusObject $currentStatus -PropertyName 'legacy_zero_question_mode' -DefaultValue $false)
        }

        if ([bool]$summary.passed) {
            Set-Status -StageName 'doctor' -DoctorPassed $true -DoctorFailed $false -EntrycheckPassed $true -EntrycheckFailed $false -MisplacedPackageDetected $false -LegacyProjectDetected $resolvedLegacyProjectDetected -LegacyZeroQuestionMode $resolvedLegacyZeroQuestionMode -MissingCapabilities @()
        } else {
            Set-Status -StageName 'doctor_failed' -DoctorPassed $false -DoctorFailed $true -EntrycheckPassed ([bool]$summary.entrycheck_passed) -EntrycheckFailed ([bool]$summary.entrycheck_failed) -MisplacedPackageDetected ([bool]$summary.misplaced_package_detected) -LegacyProjectDetected $resolvedLegacyProjectDetected -LegacyZeroQuestionMode $resolvedLegacyZeroQuestionMode -MissingCapabilities @($summary.missing_capabilities)
        }
    }

    $summary
}

function Invoke-Precheck {
    param(
        [string]$ProjectRoot,
        [switch]$PersistStatus,
        [bool]$LegacyProjectDetected = $false,
        [bool]$LegacyZeroQuestionMode = $false
    )

    $summary = Get-CapabilitySummary `
        -SuccessMessage '五项能力已通过 precheck，可以继续自动初始化。' `
        -FailureLeadText '请先补齐缺失能力，再重新执行初始化。'

    Sync-DecisionCapabilities -CapabilitySummary $summary
    Write-PrecheckArtifacts -PrecheckSummary $summary

    if ($PersistStatus) {
        $currentStatus = if (Test-Path (Get-SessionPath 'status.json')) {
            Read-JsonFile -Path (Get-SessionPath 'status.json')
        } else {
            $null
        }
        $resolvedLegacyProjectDetected = if ($PSBoundParameters.ContainsKey('LegacyProjectDetected')) {
            $LegacyProjectDetected
        } else {
            [bool](Get-StatusPropertyValue -StatusObject $currentStatus -PropertyName 'legacy_project_detected' -DefaultValue $false)
        }
        $resolvedLegacyZeroQuestionMode = if ($PSBoundParameters.ContainsKey('LegacyZeroQuestionMode')) {
            $LegacyZeroQuestionMode
        } else {
            [bool](Get-StatusPropertyValue -StatusObject $currentStatus -PropertyName 'legacy_zero_question_mode' -DefaultValue $false)
        }

        if ([bool]$summary.passed) {
            Set-Status -StageName 'precheck' -PrecheckPassed $true -PrecheckFailed $false -LegacyProjectDetected $resolvedLegacyProjectDetected -LegacyZeroQuestionMode $resolvedLegacyZeroQuestionMode -MissingCapabilities @() -CapabilityGatePassed $true
        } else {
            Set-Status -StageName 'precheck_failed' -PrecheckPassed $false -PrecheckFailed $true -LegacyProjectDetected $resolvedLegacyProjectDetected -LegacyZeroQuestionMode $resolvedLegacyZeroQuestionMode -MissingCapabilities @($summary.missing_capabilities) -CapabilityGatePassed $false
        }
    }

    $summary
}

function Get-TargetClientName {
    param([string]$TargetClient = 'codex')

    if ($TargetClient -eq 'claude-code') { return 'Claude Code' }
    'Codex'
}

function Get-CurrentTargetClient {
    $decisionPath = Get-SessionPath 'decision.json'
    if (Test-Path $decisionPath) {
        try {
            $decision = Read-JsonFile -Path $decisionPath
            if ($decision -and $decision.PSObject.Properties.Name -contains 'target_client' -and [string]$decision.target_client) {
                return [string]$decision.target_client
            }
        } catch {
            return 'codex'
        }
    }
    'codex'
}

function Get-ClosureMessage {
    param([string]$TargetClient = '')

    $targetClientName = Get-TargetClientName -TargetClient $(if ($TargetClient) { $TargetClient } else { Get-CurrentTargetClient })
    "初始化协作包已生成并通过 postcheck。请在 $targetClientName 中新开会话或重启会话，让新生成的协作协议生效；当前初始化流程已结束。"
}

function Test-ClosureGateActive {
    param($StatusObject)

    if (-not $StatusObject) { return $false }
    $stage = [string](Get-StatusPropertyValue -StatusObject $StatusObject -PropertyName 'stage' -DefaultValue '')
    $initClosed = [bool](Get-StatusPropertyValue -StatusObject $StatusObject -PropertyName 'init_closed' -DefaultValue $false)
    ($stage -eq 'implementation_ready') -and $initClosed
}

function Get-ClosureGateResponse {
    [pscustomobject]@{
        Stage = 'completed_closure'
        SessionRoot = $sessionRoot
        ClosureGateActive = $true
        Message = Get-ClosureMessage
    }
}

function Ensure-SessionSkeleton {
    param(
        [string]$ProjectRoot,
        [string]$Provider,
        [string]$Model,
        [string]$BaseUrl,
        [string]$ApiKey
    )

    if (-not (Test-Path $sessionRoot)) {
        New-Item -ItemType Directory -Path $sessionRoot -Force | Out-Null
    }

    $questionPack = Get-DiscoveryQuestionPack -ProjectRoot $ProjectRoot -Provider $Provider -Model $Model -BaseUrl $BaseUrl -ApiKey $ApiKey
    $questions = @($questionPack.Questions)
    Write-JsonFile -Path (Get-SessionPath 'questions.json') -Value $questions
    Write-JsonFile -Path (Get-SessionPath 'question-source.json') -Value @{
        source = [string]$questionPack.Source
        provider = if ($Provider) { [string]$Provider } else { '' }
        model = if ($Model) { [string]$Model } else { '' }
    }
    Write-JsonFile -Path (Get-SessionPath 'answers.json') -Value @{}

    Set-Content -Path (Get-SessionPath 'discovery.md') -Value "# Discovery Session`n`n"
    Set-Content -Path (Get-SessionPath 'proposal.md') -Value "# Proposal`n`n尚未生成。`n"
    Write-JsonFile -Path (Get-SessionPath 'proposal-options.json') -Value @()
    Write-JsonFile -Path (Get-SessionPath 'decision.json') -Value @{
        user_confirmed = $false
        auto_confirmed = $false
        confirmation_mode = ''
        discovery_mode = ''
        project_name = ''
        project_type = ''
        delivery_mode = ''
        solution_mode = ''
        enabled_roles = @()
        recommended_roles_now = @()
        available_roles_later = @()
        integrations = @()
        detected_integrations = @()
        external_references = @()
        current_stage = ''
        current_stage_goal = ''
        primary_workstream = ''
        stage_constraints = @()
        deferred_capabilities = @()
        implementation_checklist_seed = @()
        implementation_acceptance_seed = @()
        required_capabilities = @()
        capability_probe_results = @()
        legacy_analysis_version = ''
        signal_categories = @()
        role_rationale = @{}
        confidence_breakdown = @{}
        dominant_workstreams = @()
        kickoff_pack = @{}
        analysis_confidence = ''
        autodiscovery_signals = @()
        autodiscovery_assumptions = @()
        project_goal_summary = ''
        target_users_summary = ''
        core_features_summary = ''
        constraints_summary = ''
    }
    Write-JsonFile -Path (Get-SessionPath 'status.json') -Value @{
        stage = 'discovery'
        current_question_index = 0
        session_root = $sessionRoot
        question_source = [string]$questionPack.Source
        started_at = (Get-Date).ToString('s')
        last_postcheck_passed = $null
        init_closed = $false
        last_postcheck_summary = $null
        implementation_stage_promoted = $false
        current_delivery_mode = ''
        current_stage_goal = ''
        closure_gate_active = $false
        doctor_passed = $false
        doctor_failed = $false
        entrycheck_passed = $false
        entrycheck_failed = $false
        misplaced_package_detected = $false
        precheck_passed = $false
        precheck_failed = $false
        legacy_project_detected = $false
        legacy_zero_question_mode = $false
        missing_capabilities = @()
    }
}

function Get-CurrentQuestion {
    $status = Read-JsonFile -Path (Get-SessionPath 'status.json')
    $questions = Read-JsonFile -Path (Get-SessionPath 'questions.json')
    $index = [int]$status.current_question_index
    if ($index -lt 0 -or $index -ge $questions.Count) { return $null }
    $questions[$index]
}

function Append-DiscoveryMarkdown {
    param(
        [string]$Question,
        [string]$Answer
    )
    Add-Content -Path (Get-SessionPath 'discovery.md') -Value "## Q`n$Question`n`n## A`n$Answer`n"
}

function Get-AnswerTable {
    $answersObject = Read-JsonFile -Path (Get-SessionPath 'answers.json')
    $table = @{}
    foreach ($property in $answersObject.PSObject.Properties) {
        $table[$property.Name] = [string]$property.Value
    }
    $table
}

function Get-DetectedProjectType {
    param([hashtable]$Answers)
    $text = (($Answers.Values | ForEach-Object { $_ }) -join ' ')
    if (($text -match 'web|网页|后台') -and ($text -match '小程序|微信小程序|mini program|miniapp')) { return 'web-miniapp' }
    if ($text -match '解决方案展示|解决方案站|solution site|solution-site') { return 'solution-site' }
    if ($text -match '落地页|landing page|landing-page') { return 'landing-page' }
    if ($text -match '展示站|展示型|展示页|showcase') { return 'showcase-site' }
    if ($text -match '门户|portal') { return 'portal-site' }
    if ($text -match '平台|saas|租户') { return 'saas-platform' }
    if ($text -match '内部|后台|admin|管理') { return 'internal-tool' }
    return 'web-app'
}

function Get-DetectedIntegrations {
    param([hashtable]$Answers)
    $text = (($Answers.Values | ForEach-Object { $_ }) -join ' ')
    $integrations = New-Object System.Collections.ArrayList
    if ($text -match '飞书|feishu') {
        [void]$integrations.Add([pscustomobject]@{ name = 'feishu'; display_name = '飞书' })
    }
    if ($text -match '火山|volcano') {
        [void]$integrations.Add([pscustomobject]@{ name = 'volcano'; display_name = '火山' })
    }
    if ($text -match 'coze|扣子|科兹') {
        [void]$integrations.Add([pscustomobject]@{ name = 'coze'; display_name = 'Coze' })
    }
    $integrations
}

function Get-DeliveryMode {
    param([hashtable]$Answers)

    $text = (($Answers.Values | ForEach-Object { $_ }) -join ' ')
    if (($text -match 'web|网页|后台') -and ($text -match '小程序|微信小程序|mini program|miniapp')) { return 'web-miniapp' }
    if ($text -match '解决方案展示|解决方案站|solution site|solution-site') { return 'solution-site' }
    if ($text -match '落地页|landing page|landing-page') { return 'landing-page' }
    if ($text -match '展示站|展示型|showcase') { return 'showcase-site' }
    if ($text -match '平台|saas|租户') { return 'saas-platform' }
    if ($text -match '内部|后台|admin') { return 'internal-tool' }
    'web-app'
}

function Get-ExternalReferences {
    param([hashtable]$Answers)

    $text = (($Answers.Values | ForEach-Object { $_ }) -join ' ')
    $references = New-Object System.Collections.ArrayList
    $stylePatterns = @(
        '(?i)(?:风格参考|样式参考|视觉参考|UI参考)\s*[:：]?\s*([A-Za-z]:\\[^;；`n`r]+)',
        '(?i)([A-Za-z]:\\[^;；`n`r]*ui-model[^;；`n`r]*)'
    )
    $contentPatterns = @(
        '(?i)(?:内容参考|内容文档|文案参考)\s*[:：]?\s*([A-Za-z]:\\[^;；`n`r]+)',
        '(?i)([A-Za-z]:\\[^;；`n`r]*README[^;；`n`r]*)'
    )

    foreach ($pattern in $stylePatterns) {
        $matches = [regex]::Matches($text, $pattern)
        foreach ($match in $matches) {
            $path = [string]$match.Groups[1].Value
            if ($path) {
                [void]$references.Add([pscustomobject]@{
                    type = 'style-reference'
                    path = $path.Trim()
                    purpose = '视觉风格与界面表达参考'
                    must_read = $true
                })
            }
        }
    }

    foreach ($pattern in $contentPatterns) {
        $matches = [regex]::Matches($text, $pattern)
        foreach ($match in $matches) {
            $path = [string]$match.Groups[1].Value
            if ($path) {
                [void]$references.Add([pscustomobject]@{
                    type = 'content-reference'
                    path = $path.Trim()
                    purpose = '内容结构与叙事表达参考'
                    must_read = $true
                })
            }
        }
    }

    @(
        $references |
            Sort-Object type, path -Unique
    )
}

function Get-StageConstraints {
    param(
        [hashtable]$Answers,
        [string]$DeliveryMode
    )

    $constraints = New-Object System.Collections.ArrayList
    $text = (($Answers.Values | ForEach-Object { $_ }) -join ' ')

    if ($DeliveryMode -in @('landing-page', 'solution-site', 'showcase-site')) {
        [void]$constraints.Add('当前阶段仅聚焦展示型前台交付，不扩展为平台工程。')
    }
    if ($text -match '不做后端|不需要后端|先不做后端') {
        [void]$constraints.Add('当前阶段不做后端实现。')
    }
    if ($text -match '不做完整平台|先不做平台|不扩展平台') {
        [void]$constraints.Add('当前阶段不扩展平台能力。')
    }
    if ($text -match '上线速度|尽快上线|快速上线') {
        [void]$constraints.Add('当前阶段优先上线速度与可交付性。')
    }

    @($constraints | Sort-Object -Unique)
}

function Get-DeferredCapabilities {
    param(
        [hashtable]$Answers,
        [string]$DeliveryMode
    )

    $capabilities = New-Object System.Collections.ArrayList
    $text = (($Answers.Values | ForEach-Object { $_ }) -join ' ')

    if ($DeliveryMode -in @('landing-page', 'solution-site', 'showcase-site')) {
        [void]$capabilities.Add('平台化能力')
        [void]$capabilities.Add('后端服务')
        [void]$capabilities.Add('深度第三方集成')
    }
    if ($text -match '后续可扩展|后续扩展') {
        [void]$capabilities.Add('后续扩展能力')
    }

    @($capabilities | Sort-Object -Unique)
}

function Get-CurrentAndLaterRoles {
    param(
        [string]$DeliveryMode,
        [string]$SolutionMode,
        [object[]]$DetectedIntegrations
    )

    $rolesNow = New-Object System.Collections.ArrayList
    $rolesLater = New-Object System.Collections.ArrayList

    switch ($DeliveryMode) {
        'landing-page' {
            foreach ($role in @('frontend', 'reviewer', 'docs')) { [void]$rolesNow.Add($role) }
            foreach ($role in @('backend', 'database', 'devops', 'compliance')) { [void]$rolesLater.Add($role) }
        }
        'solution-site' {
            foreach ($role in @('frontend', 'reviewer', 'docs')) { [void]$rolesNow.Add($role) }
            if ($SolutionMode -in @('balanced', 'enterprise')) { [void]$rolesNow.Add('architect') }
            foreach ($role in @('backend', 'database', 'devops', 'compliance')) { [void]$rolesLater.Add($role) }
        }
        'showcase-site' {
            foreach ($role in @('frontend', 'reviewer', 'docs')) { [void]$rolesNow.Add($role) }
            if ($SolutionMode -eq 'enterprise') { [void]$rolesNow.Add('architect') }
            foreach ($role in @('backend', 'database', 'devops', 'compliance')) { [void]$rolesLater.Add($role) }
        }
        'saas-platform' {
            foreach ($role in @('architect', 'backend', 'frontend', 'reviewer', 'qa', 'docs')) { [void]$rolesNow.Add($role) }
            if ($SolutionMode -eq 'enterprise') {
                foreach ($role in @('database', 'devops', 'compliance')) { [void]$rolesNow.Add($role) }
            } else {
                foreach ($role in @('database', 'devops', 'compliance')) { [void]$rolesLater.Add($role) }
            }
        }
        'web-miniapp' {
            foreach ($role in @('backend', 'frontend', 'miniapp', 'reviewer', 'qa', 'docs')) { [void]$rolesNow.Add($role) }
            if ($SolutionMode -in @('balanced', 'enterprise')) {
                foreach ($role in @('architect', 'database')) { [void]$rolesNow.Add($role) }
            } else {
                foreach ($role in @('architect', 'database')) { [void]$rolesLater.Add($role) }
            }
            if ($SolutionMode -eq 'enterprise') {
                foreach ($role in @('devops', 'compliance')) { [void]$rolesNow.Add($role) }
            } else {
                foreach ($role in @('devops', 'compliance')) { [void]$rolesLater.Add($role) }
            }
        }
        'internal-tool' {
            foreach ($role in @('backend', 'frontend', 'reviewer', 'qa', 'docs')) { [void]$rolesNow.Add($role) }
            foreach ($role in @('database', 'devops', 'compliance')) { [void]$rolesLater.Add($role) }
        }
        default {
            foreach ($role in @('backend', 'frontend', 'reviewer', 'docs')) { [void]$rolesNow.Add($role) }
            if ($SolutionMode -in @('balanced', 'enterprise')) { [void]$rolesNow.Add('qa') }
            foreach ($role in @('database', 'devops', 'compliance')) { [void]$rolesLater.Add($role) }
        }
    }

    if ($DeliveryMode -eq 'saas-platform') {
        foreach ($integration in (Get-NormalizedIntegrations -Integrations $DetectedIntegrations)) {
            $integrationName = Get-IntegrationName -Integration $integration
            if ($integrationName) {
                [void]$rolesLater.Add("integration-$integrationName")
            }
        }
    } else {
        foreach ($integration in (Get-NormalizedIntegrations -Integrations $DetectedIntegrations)) {
            $integrationName = Get-IntegrationName -Integration $integration
            if ($integrationName) {
                [void]$rolesLater.Add("integration-$integrationName")
            }
        }
    }

    [pscustomobject]@{
        RecommendedRolesNow = @($rolesNow | Sort-Object -Unique)
        AvailableRolesLater = @($rolesLater | Sort-Object -Unique)
    }
}

function Get-PrimaryWorkstream {
    param([string]$DeliveryMode)

    if ($DeliveryMode -in @('landing-page', 'solution-site', 'showcase-site')) {
        return 'showcase-site'
    }

    'product-implementation'
}

function Test-RoleEnabled {
    param(
        [object[]]$EnabledRoles,
        [string]$RoleName
    )

    foreach ($role in (Normalize-ToArray -Value $EnabledRoles)) {
        if ([string]$role -eq $RoleName) {
            return $true
        }
    }

    $false
}

function Get-WorkflowStageNames {
    param([object[]]$EnabledRoles)

    $stages = New-Object System.Collections.ArrayList
    [void]$stages.Add('implementation')
    if (Test-RoleEnabled -EnabledRoles $EnabledRoles -RoleName 'reviewer') {
        [void]$stages.Add('review')
    }
    if (Test-RoleEnabled -EnabledRoles $EnabledRoles -RoleName 'qa') {
        [void]$stages.Add('qa')
    }

    @($stages)
}

function Get-WorkflowStageText {
    param([object[]]$EnabledRoles)

    $stages = @(Get-WorkflowStageNames -EnabledRoles $EnabledRoles)
    if ($stages.Count -eq 0) {
        return 'implementation'
    }

    ($stages -join ' / ')
}

function Get-DispatchTriggerItems {
    param([object[]]$EnabledRoles)

    $items = New-Object System.Collections.ArrayList
    if (Test-RoleEnabled -EnabledRoles $EnabledRoles -RoleName 'architect') { [void]$items.Add('架构/方案 -> architect') }
    if (Test-RoleEnabled -EnabledRoles $EnabledRoles -RoleName 'backend') { [void]$items.Add('后端/API -> backend') }
    if (Test-RoleEnabled -EnabledRoles $EnabledRoles -RoleName 'frontend') { [void]$items.Add('页面/UI -> frontend') }
    if (Test-RoleEnabled -EnabledRoles $EnabledRoles -RoleName 'miniapp') { [void]$items.Add('微信小程序端 -> miniapp') }
    if (Test-RoleEnabled -EnabledRoles $EnabledRoles -RoleName 'database') { [void]$items.Add('数据结构/迁移 -> database') }
    if (Test-RoleEnabled -EnabledRoles $EnabledRoles -RoleName 'devops') { [void]$items.Add('部署/流水线 -> devops') }
    if (Test-RoleEnabled -EnabledRoles $EnabledRoles -RoleName 'docs') { [void]$items.Add('真源/文档 -> docs') }
    if (Test-RoleEnabled -EnabledRoles $EnabledRoles -RoleName 'reviewer') { [void]$items.Add('审查 -> reviewer') }
    if (Test-RoleEnabled -EnabledRoles $EnabledRoles -RoleName 'qa') { [void]$items.Add('测试/回归 -> qa') }
    if (Test-RoleEnabled -EnabledRoles $EnabledRoles -RoleName 'compliance') { [void]$items.Add('合规/策略 -> compliance') }

    @($items)
}

function Get-CoreGateItems {
    param([object[]]$EnabledRoles)

    $items = New-Object System.Collections.ArrayList
    [void]$items.Add('真源文档可读')
    [void]$items.Add('capability gate 为绿色')
    [void]$items.Add('初始化协作包结构、目标软件入口与 session 审计产物已通过 postcheck')
    [void]$items.Add('梦星星方案输出、用户选择与星梦梦语义验收记录一致')
    [void]$items.Add('不得声称业务系统、业务代码、评审或测试已经完成')
    @($items)
}

function Get-FinalGateItems {
    param([object[]]$EnabledRoles)

    $items = New-Object System.Collections.ArrayList
    if (Test-RoleEnabled -EnabledRoles $EnabledRoles -RoleName 'reviewer') {
        [void]$items.Add('reviewer 级结构与风险审查')
    } else {
        [void]$items.Add('结构与风险审查')
    }

    if (Test-RoleEnabled -EnabledRoles $EnabledRoles -RoleName 'qa') {
        [void]$items.Add('qa 级测试与回归验证')
    } else {
        [void]$items.Add('必要测试与回归验证证据')
    }

    [void]$items.Add('用户可见行为证据')
    [void]$items.Add('自动化检查与门禁')
    @($items)
}

function Get-ImplementationSequenceItems {
    param([object[]]$EnabledRoles)

    $items = New-Object System.Collections.ArrayList
    [void]$items.Add('implementation-planning')
    [void]$items.Add('implementation')
    if (Test-RoleEnabled -EnabledRoles $EnabledRoles -RoleName 'miniapp') {
        [void]$items.Add('miniapp-integration')
    }
    if (Test-RoleEnabled -EnabledRoles $EnabledRoles -RoleName 'reviewer') {
        [void]$items.Add('review')
    }
    if (Test-RoleEnabled -EnabledRoles $EnabledRoles -RoleName 'qa') {
        [void]$items.Add('qa')
    }
    [void]$items.Add('acceptance')
    @($items)
}

function Get-CollaborationRequirementText {
    param(
        [string]$LeadText,
        [string]$EvidenceText,
        [object[]]$EnabledRoles
    )

    $mentions = New-Object System.Collections.ArrayList
    if (Test-RoleEnabled -EnabledRoles $EnabledRoles -RoleName 'reviewer') { [void]$mentions.Add('`@reviewer`') }
    if (Test-RoleEnabled -EnabledRoles $EnabledRoles -RoleName 'qa') { [void]$mentions.Add('`@qa`') }

    if ($mentions.Count -gt 0) {
        $text = "${LeadText}，必须拉 $($mentions -join ' + ')"
        if (-not (Test-RoleEnabled -EnabledRoles $EnabledRoles -RoleName 'qa')) {
            $text += "，并补回 $EvidenceText"
        }
        return $text
    }

    "${LeadText}，必须补回 $EvidenceText"
}

function Get-CollaborationNotificationText {
    param(
        [string]$LeadText,
        [string]$EvidenceText,
        [object[]]$EnabledRoles
    )

    $roles = New-Object System.Collections.ArrayList
    if (Test-RoleEnabled -EnabledRoles $EnabledRoles -RoleName 'reviewer') { [void]$roles.Add('`reviewer`') }
    if (Test-RoleEnabled -EnabledRoles $EnabledRoles -RoleName 'qa') { [void]$roles.Add('`qa`') }

    if ($roles.Count -gt 1) {
        return "${LeadText}时通知 $($roles -join ' 与 ')"
    }

    if ($roles.Count -eq 1) {
        $text = "${LeadText}时通知 $($roles[0])"
        if (-not (Test-RoleEnabled -EnabledRoles $EnabledRoles -RoleName 'qa')) {
            $text += "，并补回 $EvidenceText"
        }
        return $text
    }

    "${LeadText}时补回 $EvidenceText"
}

function Get-ImplementationChecklistSeed {
    param(
        [hashtable]$Answers,
        [string]$DeliveryMode,
        [object[]]$ExternalReferences,
        [object[]]$StageConstraints,
        [object[]]$EnabledRoles
    )

    $items = New-Object System.Collections.ArrayList
    $workflowStageText = Get-WorkflowStageText -EnabledRoles $EnabledRoles
    if ($DeliveryMode -in @('landing-page', 'solution-site', 'showcase-site')) {
        [void]$items.Add('是否锁定本轮只做展示站，不扩展平台能力')
        if ($ExternalReferences | Where-Object { $_.type -eq 'style-reference' }) {
            [void]$items.Add('是否锁定风格参考源')
        }
        if ($ExternalReferences | Where-Object { $_.type -eq 'content-reference' }) {
            [void]$items.Add('是否锁定内容参考源')
        }
        [void]$items.Add('是否确认页面 section 与信息架构')
        [void]$items.Add('是否补齐缺失素材')
        [void]$items.Add("是否进入 $workflowStageText")
    } else {
        [void]$items.Add('是否锁定首轮实施范围')
        [void]$items.Add('是否确认接口与模块边界')
        [void]$items.Add("是否进入 $workflowStageText")
    }

    foreach ($constraint in $StageConstraints) {
        [void]$items.Add($constraint)
    }

    @($items | Sort-Object -Unique)
}

function Get-ImplementationAcceptanceSeed {
    param(
        [string]$DeliveryMode,
        [object[]]$ExternalReferences,
        [object[]]$EnabledRoles
    )

    $items = New-Object System.Collections.ArrayList
    if ($DeliveryMode -in @('landing-page', 'solution-site', 'showcase-site')) {
        if ($ExternalReferences.Count -gt 0) {
            [void]$items.Add('外部参考已读取并落实')
        }
        [void]$items.Add('展示型首轮实施范围已在初始化协作包中收口')
        [void]$items.Add('后续页面结构与内容验收需在新实施会话中完成')
    } else {
        [void]$items.Add('当前阶段业务实施目标已在初始化协作包中定义')
        [void]$items.Add('后续实现、review、QA 与关键验证需在新实施会话中完成')
    }

    @($items | Sort-Object -Unique)
}

function Get-ConfirmedFeatureTerms {
    param([hashtable]$Answers)

    $sources = @(
        if ($Answers.ContainsKey('core_features')) { [string]$Answers['core_features'] }
        if ($Answers.ContainsKey('project_goal')) { [string]$Answers['project_goal'] }
    )
    $terms = New-Object System.Collections.ArrayList
    foreach ($source in $sources) {
        foreach ($part in ($source -split '[、,，;；\|\r\n]+')) {
            $term = ([string]$part).Trim()
            $term = $term -replace '^(关键功能|核心功能|功能|包括|包含|以及|和|与|统一管理|管理)\s*[:：]?\s*', ''
            $term = $term -replace '\s*(系统|平台|模块)$', ''
            if ($term.Length -ge 2 -and $term.Length -le 24 -and $term -notmatch '^(做一个|面向|解决|目标用户|产品类型|从零开始|没有必须对接的平台)') {
                [void]$terms.Add($term)
            }
        }
        if ($terms.Count -gt 0) { break }
    }

    @($terms | Select-Object -First 8 | Sort-Object -Unique)
}

function Test-IsWebMiniappProject {
    param([hashtable]$Answers)

    $text = (($Answers.Values | ForEach-Object { $_ }) -join ' ')
    (($text -match 'web|网页|后台') -and ($text -match '小程序|微信小程序|mini program|miniapp'))
}

function Get-DomainModuleItems {
    param(
        [hashtable]$Answers,
        [string]$DeliveryMode
    )

    $features = @(Get-ConfirmedFeatureTerms -Answers $Answers)
    if ($features.Count -eq 0) { return @() }
    $featureText = [string]::Join('、', $features)

    if ($DeliveryMode -eq 'web-miniapp') {
        return @(
            "Web 管理后台：围绕 $featureText 建立桌面端/后台端管理入口"
            "微信小程序端：围绕 $featureText 建立移动端高频操作与查询入口"
            "统一后端 API：为 Web 与小程序提供同一套鉴权、业务接口和状态同步边界"
            "数据真源：$featureText 必须共享同一套数据定义、权限规则和验收口径"
        )
    }

    @(
        foreach ($feature in $features) {
            "$feature：明确页面/接口入口、核心数据对象、主要操作、成功/失败状态和首轮验收样例"
        }
    )
}

function Get-DomainAcceptanceItems {
    param(
        [hashtable]$Answers,
        [string]$DeliveryMode
    )

    $features = @(Get-ConfirmedFeatureTerms -Answers $Answers)
    if ($features.Count -eq 0) { return @() }
    $featureText = [string]::Join('、', $features)

    if ($DeliveryMode -eq 'web-miniapp') {
        return @(
            "Web 管理后台覆盖 $featureText 的主流程入口、权限提示和错误反馈"
            "微信小程序端覆盖 $featureText 的移动端关键路径，并与 Web 端状态一致"
            'Web 与小程序共享统一后端 API、鉴权边界和数据真源'
            "围绕 $featureText 的关键表单或操作具备必填校验、错误提示和基础权限边界"
        )
    }

    @(
        foreach ($feature in $features) {
            "$feature 具备可验证的用户路径、数据状态和验收证据"
        }
    )
}

function Get-DomainTaskItems {
    param(
        [hashtable]$Answers,
        [string]$ImplementationOwnerRole,
        [object[]]$SupportRoles,
        [string]$ValidationOwnerRole
    )

    $features = @(Get-ConfirmedFeatureTerms -Answers $Answers)
    if ($features.Count -gt 0) {
        $supportRoleText = if (@($SupportRoles).Count -gt 0) { [string]::Join(', ', @($SupportRoles)) } else { 'reviewer' }
        $featureText = [string]::Join('、', $features)
        $thirdTaskSupport = if (Test-IsWebMiniappProject -Answers $Answers) { 'miniapp, frontend, backend' } else { $ImplementationOwnerRole }
        return @(
            "### task_1`n- title: 锁定首轮业务语义与协作边界`n- owner_role: $ImplementationOwnerRole`n- support_roles: docs, reviewer`n- depends_on: docs/project_context.md, docs/architecture/01-项目架构设计书.md`n- done_signal: $featureText 的范围、角色、数据边界和验收口径已在真源中收口，尚未声称业务实现完成"
            "### task_2`n- title: 准备第一优先业务路径与接口边界`n- owner_role: $ImplementationOwnerRole`n- support_roles: $supportRoleText`n- depends_on: docs/workflow/implementation-kickoff.md, docs/workflow/first-sprint-contract.md`n- done_signal: $featureText 的实施合同、接口边界和验证计划已成文，等待后续实施"
            "### task_3`n- title: 定义跨端/跨角色验证计划`n- owner_role: $ValidationOwnerRole`n- support_roles: $thirdTaskSupport`n- depends_on: docs/workflow/acceptance-gates.md, docs/workflow/grading-criteria.md`n- done_signal: $featureText 的关键风险、证据采集方式和下一轮建议已成文，等待后续实施后补证据"
        )
    }

    @()
}

function Get-DomainWorkflowItems {
    param(
        [hashtable]$Answers,
        [object[]]$EnabledRoles,
        [string]$DeliveryMode
    )

    $features = @(Get-ConfirmedFeatureTerms -Answers $Answers)
    if ($features.Count -eq 0) { return @() }
    $featureText = [string]::Join('、', $features)

    if ($DeliveryMode -eq 'web-miniapp') {
        $apiRoles = New-Object System.Collections.ArrayList
        [void]$apiRoles.Add('backend')
        if ($EnabledRoles -contains 'database') {
            [void]$apiRoles.Add('database')
        }
        $apiRoleText = [string]::Join(' + ', @($apiRoles))
        return @(
            "Web 管理后台 -> frontend；负责 $featureText 的后台端信息架构、页面路径和可见反馈"
            "微信小程序端 -> miniapp；负责 $featureText 的移动端高频路径、端侧状态和交互反馈"
            "统一后端 API -> $apiRoleText；负责 $featureText 的接口契约、数据模型、鉴权和状态同步"
            '跨端验证 -> qa + reviewer；覆盖 Web、小程序、API 三方联动和关键状态一致性'
        )
    }

    @(
        foreach ($feature in $features) {
            $ownerRole = if ($EnabledRoles -contains 'backend') { 'backend' } elseif ($EnabledRoles -contains 'frontend') { 'frontend' } else { 'docs' }
            $validationRoles = @(
                if ($EnabledRoles -contains 'reviewer') { 'reviewer' }
                if ($EnabledRoles -contains 'qa') { 'qa' }
            )
            $validationText = if ($validationRoles.Count -gt 0) { "，并由 $([string]::Join(' / ', $validationRoles)) 补齐验证证据" } else { '，并补齐可验证证据' }
            "$feature -> $ownerRole；按真源边界拆解实施路径$validationText"
        }
    )
}

function Get-ProposalOptions {
    param([hashtable]$Answers)

    $projectType = Get-DetectedProjectType -Answers $Answers
    $integrations = @(Get-DetectedIntegrations -Answers $Answers)
    $deliveryMode = Get-DeliveryMode -Answers $Answers
    $externalReferences = @(Get-ExternalReferences -Answers $Answers)
    $stageConstraints = @(Get-StageConstraints -Answers $Answers -DeliveryMode $deliveryMode)
    $deferredCapabilities = @(Get-DeferredCapabilities -Answers $Answers -DeliveryMode $deliveryMode)
    $domainModules = @(Get-DomainModuleItems -Answers $Answers -DeliveryMode $deliveryMode)
    $domainAcceptance = @(Get-DomainAcceptanceItems -Answers $Answers -DeliveryMode $deliveryMode)
    $constraints = if ($Answers.ContainsKey('constraints')) { $Answers['constraints'] } else { '' }
    $currentStage = 'implementation-v1'
    $currentStageGoal = if ($deliveryMode -in @('landing-page', 'solution-site', 'showcase-site')) {
        '完成当前展示型项目的首个可交付版本'
    } else {
        '完成当前业务目标的首轮实现'
    }
    $primaryWorkstream = Get-PrimaryWorkstream -DeliveryMode $deliveryMode
    $optionARoleSet = Get-CurrentAndLaterRoles -DeliveryMode $deliveryMode -SolutionMode 'fast-mvp' -DetectedIntegrations $integrations
    $optionBRoleSet = Get-CurrentAndLaterRoles -DeliveryMode $deliveryMode -SolutionMode 'balanced' -DetectedIntegrations $integrations
    $optionCRoleSet = Get-CurrentAndLaterRoles -DeliveryMode $deliveryMode -SolutionMode 'enterprise' -DetectedIntegrations $integrations

    $optionA = [pscustomobject]@{
        id = 'A'
        name = '快速 MVP 方案'
        positioning = '优先快速启动的初始化方案'
        project_type = $projectType
        delivery_mode = $deliveryMode
        solution_mode = 'fast-mvp'
        enabled_roles = @($optionARoleSet.RecommendedRolesNow)
        recommended_roles_now = @($optionARoleSet.RecommendedRolesNow)
        available_roles_later = @($optionARoleSet.AvailableRolesLater)
        integrations = @($integrations)
        detected_integrations = @($integrations)
        external_references = @($externalReferences)
        stage_constraints = @($stageConstraints)
        deferred_capabilities = @($deferredCapabilities)
        current_stage = $currentStage
        current_stage_goal = $currentStageGoal
        primary_workstream = $primaryWorkstream
        implementation_checklist_seed = @(Get-ImplementationChecklistSeed -Answers $Answers -DeliveryMode $deliveryMode -ExternalReferences $externalReferences -StageConstraints $stageConstraints -EnabledRoles @($optionARoleSet.RecommendedRolesNow))
        implementation_acceptance_seed = @($domainAcceptance + @(Get-ImplementationAcceptanceSeed -DeliveryMode $deliveryMode -ExternalReferences $externalReferences -EnabledRoles @($optionARoleSet.RecommendedRolesNow)))
        domain_modules = @($domainModules)
        time_cost = '低'
        deployment_cost = '低'
        development_difficulty = '中'
        scalability = '中'
        risks = '后续需要补测试与治理能力'
        recommendation_reason = '适合尽快启动第一版，但治理与测试深度较轻。'
    }
    $optionB = [pscustomobject]@{
        id = 'B'
        name = '平衡型方案'
        positioning = '兼顾交付速度和协作治理'
        project_type = $projectType
        delivery_mode = $deliveryMode
        solution_mode = 'balanced'
        enabled_roles = @($optionBRoleSet.RecommendedRolesNow)
        recommended_roles_now = @($optionBRoleSet.RecommendedRolesNow)
        available_roles_later = @($optionBRoleSet.AvailableRolesLater)
        integrations = @($integrations)
        detected_integrations = @($integrations)
        external_references = @($externalReferences)
        stage_constraints = @($stageConstraints)
        deferred_capabilities = @($deferredCapabilities)
        current_stage = $currentStage
        current_stage_goal = $currentStageGoal
        primary_workstream = $primaryWorkstream
        implementation_checklist_seed = @(Get-ImplementationChecklistSeed -Answers $Answers -DeliveryMode $deliveryMode -ExternalReferences $externalReferences -StageConstraints $stageConstraints -EnabledRoles @($optionBRoleSet.RecommendedRolesNow))
        implementation_acceptance_seed = @($domainAcceptance + @(Get-ImplementationAcceptanceSeed -DeliveryMode $deliveryMode -ExternalReferences $externalReferences -EnabledRoles @($optionBRoleSet.RecommendedRolesNow)))
        domain_modules = @($domainModules)
        time_cost = '中'
        deployment_cost = '低中'
        development_difficulty = '中'
        scalability = '中高'
        risks = '前期文档和测试工作量更高'
        recommendation_reason = if ($domainModules.Count -gt 0) { '当前项目包含多个业务模块，平衡型方案能同时保留交付速度、测试验证和协作治理。' } else { '适合在交付速度和长期治理之间保持平衡。' }
    }
    $optionC = [pscustomobject]@{
        id = 'C'
        name = '企业扩展型方案'
        positioning = '强调长期治理和扩展能力'
        project_type = $projectType
        delivery_mode = $deliveryMode
        solution_mode = 'enterprise'
        enabled_roles = @($optionCRoleSet.RecommendedRolesNow)
        recommended_roles_now = @($optionCRoleSet.RecommendedRolesNow)
        available_roles_later = @($optionCRoleSet.AvailableRolesLater)
        integrations = @($integrations)
        detected_integrations = @($integrations)
        external_references = @($externalReferences)
        stage_constraints = @($stageConstraints)
        deferred_capabilities = @($deferredCapabilities)
        current_stage = $currentStage
        current_stage_goal = $currentStageGoal
        primary_workstream = $primaryWorkstream
        implementation_checklist_seed = @(Get-ImplementationChecklistSeed -Answers $Answers -DeliveryMode $deliveryMode -ExternalReferences $externalReferences -StageConstraints $stageConstraints -EnabledRoles @($optionCRoleSet.RecommendedRolesNow))
        implementation_acceptance_seed = @($domainAcceptance + @(Get-ImplementationAcceptanceSeed -DeliveryMode $deliveryMode -ExternalReferences $externalReferences -EnabledRoles @($optionCRoleSet.RecommendedRolesNow)))
        domain_modules = @($domainModules)
        time_cost = '高'
        deployment_cost = '中'
        development_difficulty = '高'
        scalability = '高'
        risks = '初期投入较大'
        recommendation_reason = '适合强治理、强扩展和合规要求明显的长期平台。'
    }

    $recommended = if ($constraints -match '快速|预算|尽快|有限') { 'A' } elseif ($constraints -match '企业|合规') { 'C' } else { 'B' }
    [pscustomobject]@{
        Options = @($optionA, $optionB, $optionC)
        Recommended = $recommended
    }
}

function Write-ProposalMarkdown {
    param(
        [hashtable]$Answers,
        [object[]]$Options,
        [string]$Recommended
    )

    $projectName = if ($Answers.ContainsKey('project_name')) { $Answers['project_name'] } else { '未命名项目' }
    $lines = New-Object System.Collections.ArrayList
    [void]$lines.Add("# Proposal for $projectName")
    [void]$lines.Add("")
    [void]$lines.Add("## 需求理解")
    [void]$lines.Add("- 产品目标：$($Answers['project_goal'])")
    [void]$lines.Add("- 目标用户：$($Answers['target_users'])")
    [void]$lines.Add("- 核心功能：$($Answers['core_features'])")
    [void]$lines.Add("- 关键约束：$($Answers['constraints'])")
    [void]$lines.Add("")
    [void]$lines.Add("## 候选方案")

    foreach ($option in $Options) {
        [void]$lines.Add("")
        [void]$lines.Add("### 方案 $($option.id)：$($option.name)")
        [void]$lines.Add("- 定位：$($option.positioning)")
        [void]$lines.Add("- 时间成本：$($option.time_cost)")
        [void]$lines.Add("- 部署成本：$($option.deployment_cost)")
        [void]$lines.Add("- 开发难度：$($option.development_difficulty)")
        [void]$lines.Add("- 扩展性：$($option.scalability)")
        [void]$lines.Add("- 主要风险：$($option.risks)")
        [void]$lines.Add("- 启用角色：$([string]::Join(', ', $option.enabled_roles))")
        if (@($option.domain_modules).Count -gt 0) {
            [void]$lines.Add("- 业务模块：$([string]::Join('；', @($option.domain_modules)))")
        }
        if ($option.PSObject.Properties.Name -contains 'recommendation_reason' -and [string]$option.recommendation_reason) {
            [void]$lines.Add("- 适用理由：$($option.recommendation_reason)")
        }
    }

    [void]$lines.Add("")
    [void]$lines.Add("## 推荐")
    [void]$lines.Add("当前更推荐方案 $Recommended。只有在用户明确拍板后，才能进入 confirm 与 bootstrap。")
    Set-Content -Path (Get-SessionPath 'proposal.md') -Value ($lines -join "`r`n")
}

function Set-Status {
    param(
        [string]$StageName,
        [Nullable[int]]$CurrentQuestionIndex = $null,
        $LastPostcheckPassed = $null,
        [bool]$InitClosed = $false,
        $LastPostcheckSummary = $null,
        [bool]$ImplementationStagePromoted = $false,
        [string]$CurrentDeliveryMode = '',
        [string]$CurrentStageGoal = '',
        [bool]$ClosureGateActive = $false,
        [bool]$DoctorPassed = $false,
        [bool]$DoctorFailed = $false,
        [bool]$EntrycheckPassed = $false,
        [bool]$EntrycheckFailed = $false,
        [bool]$MisplacedPackageDetected = $false,
        [bool]$PrecheckPassed = $false,
        [bool]$PrecheckFailed = $false,
        [bool]$LegacyProjectDetected = $false,
        [bool]$LegacyZeroQuestionMode = $false,
        [object[]]$MissingCapabilities = @(),
        $CapabilityGatePassed = $null,
        $TruthSourceGatePassed = $null,
        $TruthSourceGateFailed = $null,
        [object[]]$TruthSourceGateIssues = @()
    )

    $current = Read-JsonFile -Path (Get-SessionPath 'status.json')
    $status = @{
        stage = $StageName
        current_question_index = $CurrentQuestionIndex
        session_root = $current.session_root
        question_source = [string](Get-StatusPropertyValue -StatusObject $current -PropertyName 'question_source' -DefaultValue 'fallback template')
        started_at = $current.started_at
        last_postcheck_passed = if ($PSBoundParameters.ContainsKey('LastPostcheckPassed')) { $LastPostcheckPassed } else { Get-StatusPropertyValue -StatusObject $current -PropertyName 'last_postcheck_passed' -DefaultValue $null }
        init_closed = if ($PSBoundParameters.ContainsKey('InitClosed')) { $InitClosed } else { [bool](Get-StatusPropertyValue -StatusObject $current -PropertyName 'init_closed' -DefaultValue $false) }
        last_postcheck_summary = if ($PSBoundParameters.ContainsKey('LastPostcheckSummary')) { $LastPostcheckSummary } else { Get-StatusPropertyValue -StatusObject $current -PropertyName 'last_postcheck_summary' -DefaultValue $null }
        implementation_stage_promoted = if ($PSBoundParameters.ContainsKey('ImplementationStagePromoted')) { $ImplementationStagePromoted } else { [bool](Get-StatusPropertyValue -StatusObject $current -PropertyName 'implementation_stage_promoted' -DefaultValue $false) }
        current_delivery_mode = if ($PSBoundParameters.ContainsKey('CurrentDeliveryMode')) { $CurrentDeliveryMode } else { [string](Get-StatusPropertyValue -StatusObject $current -PropertyName 'current_delivery_mode' -DefaultValue '') }
        current_stage_goal = if ($PSBoundParameters.ContainsKey('CurrentStageGoal')) { $CurrentStageGoal } else { [string](Get-StatusPropertyValue -StatusObject $current -PropertyName 'current_stage_goal' -DefaultValue '') }
        closure_gate_active = if ($PSBoundParameters.ContainsKey('ClosureGateActive')) { $ClosureGateActive } else { [bool](Get-StatusPropertyValue -StatusObject $current -PropertyName 'closure_gate_active' -DefaultValue $false) }
        doctor_passed = if ($PSBoundParameters.ContainsKey('DoctorPassed')) { $DoctorPassed } else { [bool](Get-StatusPropertyValue -StatusObject $current -PropertyName 'doctor_passed' -DefaultValue $false) }
        doctor_failed = if ($PSBoundParameters.ContainsKey('DoctorFailed')) { $DoctorFailed } else { [bool](Get-StatusPropertyValue -StatusObject $current -PropertyName 'doctor_failed' -DefaultValue $false) }
        entrycheck_passed = if ($PSBoundParameters.ContainsKey('EntrycheckPassed')) { $EntrycheckPassed } else { [bool](Get-StatusPropertyValue -StatusObject $current -PropertyName 'entrycheck_passed' -DefaultValue $false) }
        entrycheck_failed = if ($PSBoundParameters.ContainsKey('EntrycheckFailed')) { $EntrycheckFailed } else { [bool](Get-StatusPropertyValue -StatusObject $current -PropertyName 'entrycheck_failed' -DefaultValue $false) }
        misplaced_package_detected = if ($PSBoundParameters.ContainsKey('MisplacedPackageDetected')) { $MisplacedPackageDetected } else { [bool](Get-StatusPropertyValue -StatusObject $current -PropertyName 'misplaced_package_detected' -DefaultValue $false) }
        precheck_passed = if ($PSBoundParameters.ContainsKey('PrecheckPassed')) { $PrecheckPassed } else { [bool](Get-StatusPropertyValue -StatusObject $current -PropertyName 'precheck_passed' -DefaultValue $false) }
        precheck_failed = if ($PSBoundParameters.ContainsKey('PrecheckFailed')) { $PrecheckFailed } else { [bool](Get-StatusPropertyValue -StatusObject $current -PropertyName 'precheck_failed' -DefaultValue $false) }
        legacy_project_detected = if ($PSBoundParameters.ContainsKey('LegacyProjectDetected')) { $LegacyProjectDetected } else { [bool](Get-StatusPropertyValue -StatusObject $current -PropertyName 'legacy_project_detected' -DefaultValue $false) }
        legacy_zero_question_mode = if ($PSBoundParameters.ContainsKey('LegacyZeroQuestionMode')) { $LegacyZeroQuestionMode } else { [bool](Get-StatusPropertyValue -StatusObject $current -PropertyName 'legacy_zero_question_mode' -DefaultValue $false) }
        missing_capabilities = if ($PSBoundParameters.ContainsKey('MissingCapabilities')) { @(Normalize-ToArray -Value $MissingCapabilities) } else { @(Normalize-ToArray -Value (Get-StatusPropertyValue -StatusObject $current -PropertyName 'missing_capabilities' -DefaultValue @())) }
        capability_gate_passed = if ($PSBoundParameters.ContainsKey('CapabilityGatePassed')) { $CapabilityGatePassed } else { Get-StatusPropertyValue -StatusObject $current -PropertyName 'capability_gate_passed' -DefaultValue $null }
        truth_source_gate_passed = if ($PSBoundParameters.ContainsKey('TruthSourceGatePassed')) { $TruthSourceGatePassed } else { Get-StatusPropertyValue -StatusObject $current -PropertyName 'truth_source_gate_passed' -DefaultValue $null }
        truth_source_gate_failed = if ($PSBoundParameters.ContainsKey('TruthSourceGateFailed')) { $TruthSourceGateFailed } else { Get-StatusPropertyValue -StatusObject $current -PropertyName 'truth_source_gate_failed' -DefaultValue $null }
        truth_source_gate_issues = if ($PSBoundParameters.ContainsKey('TruthSourceGateIssues')) { @(Normalize-ToArray -Value $TruthSourceGateIssues) } else { @(Normalize-ToArray -Value (Get-StatusPropertyValue -StatusObject $current -PropertyName 'truth_source_gate_issues' -DefaultValue @())) }
    }
    Write-JsonFile -Path (Get-SessionPath 'status.json') -Value $status
}

function Get-IntegrationName {
    param($Integration)

    if ($null -eq $Integration) {
        return ''
    }

    if ($Integration.PSObject -and $Integration.PSObject.Properties.Match('name').Count -gt 0) {
        return [string]$Integration.name
    }

    [string]$Integration
}

function Get-DecisionPropertyValue {
    param(
        $DecisionObject,
        [string]$PropertyName,
        $DefaultValue
    )

    if ($DecisionObject -and $DecisionObject.PSObject.Properties.Name -contains $PropertyName) {
        return $DecisionObject.$PropertyName
    }

    $DefaultValue
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
                return @("engineering-$integrationName-integration-developer")
            }
            return @($RoleName)
        }
    }
}

function Get-ExpectedAgencyAgentNames {
    param([object[]]$ExpectedRoles)

    @(
        foreach ($role in (Normalize-ToArray -Value $ExpectedRoles)) {
            foreach ($agentId in (Get-AgencyAgentIdsForRole -RoleName ([string]$role))) {
                if ($agentId) { [string]$agentId }
            }
        }
    ) | Sort-Object -Unique
}

function Get-IntegrationDisplayName {
    param($Integration)

    if ($null -eq $Integration) {
        return ''
    }

    if ($Integration.PSObject -and $Integration.PSObject.Properties.Match('display_name').Count -gt 0) {
        return [string]$Integration.display_name
    }

    $name = Get-IntegrationName -Integration $Integration
    if ($name) { return $name }
    ''
}

function Get-ExpectedRoleNames {
    param($Decision)

    $roles = New-Object System.Collections.ArrayList
    foreach ($role in (Normalize-ToArray -Value $Decision.enabled_roles)) {
        $roleName = [string]$role
        if ($roleName) {
            [void]$roles.Add($roleName)
        }
    }

    foreach ($integration in (Normalize-ToArray -Value $Decision.integrations)) {
        $integrationName = Get-IntegrationName -Integration $integration
        if ($integrationName) {
            [void]$roles.Add("integration-$integrationName")
        }
    }

    @($roles | Sort-Object -Unique)
}

function Get-AllRoleNamesForValidation {
    param($Decision)

    $roles = New-Object System.Collections.ArrayList
    foreach ($role in @('architect', 'backend', 'frontend', 'miniapp', 'reviewer', 'qa', 'docs', 'database', 'devops', 'compliance')) {
        [void]$roles.Add($role)
    }

    foreach ($role in (Normalize-ToArray -Value (Get-DecisionPropertyValue -DecisionObject $Decision -PropertyName 'enabled_roles' -DefaultValue @()))) {
        $roleName = [string]$role
        if ($roleName) {
            [void]$roles.Add($roleName)
        }
    }

    foreach ($role in (Normalize-ToArray -Value (Get-DecisionPropertyValue -DecisionObject $Decision -PropertyName 'recommended_roles_now' -DefaultValue @()))) {
        $roleName = [string]$role
        if ($roleName) {
            [void]$roles.Add($roleName)
        }
    }

    foreach ($role in (Normalize-ToArray -Value (Get-DecisionPropertyValue -DecisionObject $Decision -PropertyName 'available_roles_later' -DefaultValue @()))) {
        $roleName = [string]$role
        if ($roleName) {
            [void]$roles.Add($roleName)
        }
    }

    foreach ($integration in (Normalize-ToArray -Value (Get-DecisionPropertyValue -DecisionObject $Decision -PropertyName 'integrations' -DefaultValue @()))) {
        $integrationName = Get-IntegrationName -Integration $integration
        if ($integrationName) {
            [void]$roles.Add("integration-$integrationName")
        }
    }

    @($roles | Sort-Object -Unique)
}

function Normalize-DecisionRolesForDeliveryMode {
    param($Decision)

    if ($null -eq $Decision) {
        return $Decision
    }

    $deliveryMode = [string](Get-DecisionPropertyValue -DecisionObject $Decision -PropertyName 'delivery_mode' -DefaultValue '')
    if ($deliveryMode -ne 'web-miniapp') {
        return $Decision
    }

    $mandatoryRoles = @('miniapp', 'qa', 'reviewer')
    foreach ($propertyName in @('enabled_roles', 'recommended_roles_now')) {
        $roles = New-Object System.Collections.ArrayList
        foreach ($role in (Normalize-ToArray -Value (Get-DecisionPropertyValue -DecisionObject $Decision -PropertyName $propertyName -DefaultValue @()))) {
            $roleName = ([string]$role).Trim()
            if ($roleName) {
                [void]$roles.Add($roleName)
            }
        }
        foreach ($mandatoryRole in $mandatoryRoles) {
            [void]$roles.Add($mandatoryRole)
        }
        Set-ObjectPropertyValue -ObjectValue $Decision -PropertyName $propertyName -PropertyValue @($roles | Sort-Object -Unique)
    }

    $laterRoles = @(
        foreach ($role in (Normalize-ToArray -Value (Get-DecisionPropertyValue -DecisionObject $Decision -PropertyName 'available_roles_later' -DefaultValue @()))) {
            $roleName = ([string]$role).Trim()
            if ($roleName -and ($mandatoryRoles -notcontains $roleName)) {
                $roleName
            }
        }
    ) | Sort-Object -Unique
    Set-ObjectPropertyValue -ObjectValue $Decision -PropertyName 'available_roles_later' -PropertyValue @($laterRoles)

    $Decision
}

function Get-RoleReferencePatterns {
    param([string]$RoleName)

    @(
        ('@' + $RoleName)
        ('`@' + $RoleName + '`')
        ($RoleName + ' 完成')
        ($RoleName + ' 已完成')
        ('+ ' + $RoleName)
        ('-> ' + $RoleName)
        ('/ ' + $RoleName)
        ('通知 `' + $RoleName + '`')
        ('通知 ' + $RoleName)
    ) | Select-Object -Unique
}

function Get-DanglingRoleReferences {
    param(
        [string]$TargetRoot,
        [object]$Decision,
        [object[]]$ExpectedRoles,
        [object[]]$ExpectedAgentFiles,
        [object[]]$ExpectedHandbooks
    )

    $allRoles = @(Get-AllRoleNamesForValidation -Decision $Decision)
    $disabledRoles = @(
        foreach ($role in $allRoles) {
            if ($ExpectedRoles -notcontains $role) {
                [string]$role
            }
        }
    ) | Sort-Object -Unique
    $disabledRoles = @($disabledRoles)

    if ($disabledRoles.Count -eq 0) {
        return @()
    }

    $filesToCheck = @(
        'AGENTS.md'
        '.codex/COORDINATOR-SUBAGENTS.md'
        'docs/project_context.md'
        'docs/roadmap/01-实施路线图.md'
        'docs/workflow/current-stage-user-checklist.md'
        'docs/workflow/acceptance-gates.md'
        'docs/workflow/implementation-kickoff.md'
        'docs/workflow/first-task-pack.md'
        'docs/00-初始化结果索引.md'
    ) + @($ExpectedAgentFiles) + @($ExpectedHandbooks) | Sort-Object -Unique
    $filesToCheck = @($filesToCheck)

    $dangling = New-Object System.Collections.ArrayList
    foreach ($relativePath in $filesToCheck) {
        $fullPath = Join-Path $TargetRoot $relativePath
        if (-not (Test-Path $fullPath)) {
            continue
        }

        $content = Get-Content -Raw $fullPath
        foreach ($role in $disabledRoles) {
            foreach ($pattern in (Get-RoleReferencePatterns -RoleName $role)) {
                if ($content.Contains($pattern)) {
                    [void]$dangling.Add("$relativePath -> $role ($pattern)")
                    break
                }
            }
        }
    }

    @($dangling | Sort-Object -Unique)
}

function Get-WorkflowDocRelativePaths {
    @(
        'docs/workflow/current-stage-user-checklist.md'
        'docs/workflow/archive-policy.md'
        'docs/workflow/acceptance-gates.md'
        'docs/workflow/evaluator-protocol.md'
        'docs/workflow/grading-criteria.md'
        'docs/workflow/sprint-contract-template.md'
        'docs/workflow/implementation-kickoff.md'
        'docs/workflow/first-sprint-contract.md'
        'docs/workflow/first-task-pack.md'
    )
}

function Get-WorkflowReferenceRules {
    param(
        [object[]]$ExpectedRoles,
        [string]$TargetClient = 'codex'
    )

    $rules = New-Object System.Collections.ArrayList
    $sharedReferences = @(
        'docs/workflow/evaluator-protocol.md'
        'docs/workflow/grading-criteria.md'
        'docs/workflow/sprint-contract-template.md'
    )
    $kickoffReferences = @(
        'docs/workflow/implementation-kickoff.md'
        'docs/workflow/first-sprint-contract.md'
        'docs/workflow/first-task-pack.md'
    )

    if ($TargetClient -eq 'claude-code') {
        [void]$rules.Add([pscustomobject]@{
            Path = 'CLAUDE.md'
            References = @($kickoffReferences)
        })
    } else {
        [void]$rules.Add([pscustomobject]@{
            Path = 'AGENTS.md'
            References = @($sharedReferences + $kickoffReferences)
        })
        [void]$rules.Add([pscustomobject]@{
            Path = '.codex/COORDINATOR-SUBAGENTS.md'
            References = @($sharedReferences + $kickoffReferences)
        })
    }
    [void]$rules.Add([pscustomobject]@{
        Path = 'docs/00-初始化结果索引.md'
        References = $kickoffReferences
    })

    if ($ExpectedRoles -contains 'reviewer') {
        [void]$rules.Add([pscustomobject]@{
            Path = 'docs/agents/reviewer-handbook.md'
            References = $sharedReferences
        })
    }

    if ($ExpectedRoles -contains 'qa') {
        [void]$rules.Add([pscustomobject]@{
            Path = 'docs/agents/qa-handbook.md'
            References = $sharedReferences
        })
    }

    @($rules)
}

function Get-BrokenWorkflowReferences {
    param(
        [string]$TargetRoot,
        [object[]]$ExpectedRoles,
        [string]$TargetClient = 'codex'
    )

    $issues = New-Object System.Collections.ArrayList
    foreach ($rule in (Get-WorkflowReferenceRules -ExpectedRoles $ExpectedRoles -TargetClient $TargetClient)) {
        $fullPath = Join-Path $TargetRoot $rule.Path
        if (-not (Test-Path $fullPath)) {
            continue
        }

        $content = Get-Content -Raw $fullPath
        foreach ($reference in @(Normalize-ToArray -Value $rule.References)) {
            if (-not $content.Contains([string]$reference)) {
                [void]$issues.Add("$($rule.Path) -> $reference")
            }
        }
    }

    @($issues | Sort-Object -Unique)
}

function Get-InvalidWorkflowContent {
    param([string]$TargetRoot)

    $issues = New-Object System.Collections.ArrayList
    $checklistPath = 'docs/workflow/current-stage-user-checklist.md'
    $fullChecklistPath = Join-Path $TargetRoot $checklistPath
    if (-not (Test-Path $fullChecklistPath)) {
        return @()
    }

    $content = Get-Content -Raw $fullChecklistPath
    foreach ($pattern in @(
        '当前初始化线程只负责补齐协作工程'
        '初始化收口清单'
            '初始化协作包已落盘'
            '先完成初始化落盘'
        '再进入实施线程'
        'postcheck'
        'bootstrap'
        '初始化落盘'
        '初始化线程'
    )) {
        if ($content.Contains($pattern)) {
            [void]$issues.Add("$checklistPath -> $pattern")
        }
    }

    $firstSprintContractPath = 'docs/workflow/first-sprint-contract.md'
    $sprintContractTemplatePath = 'docs/workflow/sprint-contract-template.md'
    $firstSprintContractFullPath = Join-Path $TargetRoot $firstSprintContractPath
    $sprintContractTemplateFullPath = Join-Path $TargetRoot $sprintContractTemplatePath
    if ((Test-Path $firstSprintContractFullPath) -and (Test-Path $sprintContractTemplateFullPath)) {
        $firstSprintContract = (Get-Content -Raw $firstSprintContractFullPath).Trim()
        $sprintContractTemplate = (Get-Content -Raw $sprintContractTemplateFullPath).Trim()
        if ($firstSprintContract -and $firstSprintContract -eq $sprintContractTemplate) {
            [void]$issues.Add("$firstSprintContractPath duplicates $sprintContractTemplatePath")
        }
        foreach ($pattern in @(
            'Sprint Contract 模板'
            '使用前请先复制'
            '任务级 Contract 脚手架'
            '不代表当前项目已经存在一个已签署'
            '待填写'
        )) {
            if ($firstSprintContract.Contains($pattern)) {
                [void]$issues.Add("$firstSprintContractPath -> $pattern")
            }
        }
        foreach ($requiredPhrase in @('首轮实施合同', '第一优先工作流')) {
            if (-not $firstSprintContract.Contains($requiredPhrase)) {
                [void]$issues.Add("$firstSprintContractPath missing $requiredPhrase")
            }
        }
    }

    @($issues | Sort-Object -Unique)
}

function Get-DuplicateMarkdownSectionNumberIssues {
    param(
        [string]$TargetRoot,
        [string[]]$RelativePaths
    )

    $issues = New-Object System.Collections.ArrayList
    foreach ($relativePath in $RelativePaths) {
        $fullPath = Join-Path $TargetRoot $relativePath
        if (-not (Test-Path $fullPath)) {
            continue
        }

        $content = Get-Content -Raw $fullPath
        $seen = @{}
        foreach ($match in [regex]::Matches($content, '(?m)^##\s+([0-9]+)\.')) {
            $number = [string]$match.Groups[1].Value
            if ($seen.ContainsKey($number)) {
                [void]$issues.Add("$relativePath duplicate heading number ## $number.")
            } else {
                $seen[$number] = $true
            }
        }
    }

    @($issues | Sort-Object -Unique)
}

function Get-AuthorshipQualityIssues {
    param(
        [string]$TargetRoot,
        [string]$TargetClient = 'codex'
    )

    $issues = New-Object System.Collections.ArrayList
    $requiredDocs = @(
        'docs/00-初始化结果索引.md',
        'docs/project_context.md',
        'docs/architecture/01-项目架构设计书.md',
        'docs/roadmap/01-实施路线图.md',
        'docs/skills/required-capabilities.md',
        'docs/workflow/implementation-kickoff.md',
        'docs/workflow/current-stage-user-checklist.md'
    )
    if ($TargetClient -eq 'claude-code') {
        $requiredDocs += 'CLAUDE.md'
    } else {
        $requiredDocs += 'AGENTS.md'
        $requiredDocs += '.codex/COORDINATOR-SUBAGENTS.md'
    }

    $forbiddenPhrases = @(
        'ExampleProject',
        'Build the first usable version quickly',
        '当前无自动分析假设',
        '当前无自动分析信号',
        '当前无外部参考源',
        '当前无明确延后能力',
        '当前无额外后续角色',
        '生成项目骨架',
        '当前目录已经生成 HE 协作工程',
        '业务项目成品已生成',
        '业务脚手架已生成'
    )

    foreach ($relativePath in $requiredDocs) {
        $fullPath = Join-Path $TargetRoot $relativePath
        if (-not (Test-Path $fullPath)) {
            continue
        }

        $content = Get-Content -Raw $fullPath
        foreach ($placeholderMatch in [regex]::Matches($content, '\{\{[a-zA-Z0-9_\-]+\}\}')) {
            [void]$issues.Add("$relativePath unresolved placeholder $($placeholderMatch.Value)")
        }
        foreach ($variableMatch in [regex]::Matches($content, '\$[a-zA-Z_][a-zA-Z0-9_]*')) {
            [void]$issues.Add("$relativePath unresolved script variable $($variableMatch.Value)")
        }
        foreach ($phrase in $forbiddenPhrases) {
            if ($content -like "*$phrase*") {
                [void]$issues.Add("$relativePath contains template phrase '$phrase'")
            }
        }
    }

    if ($TargetClient -eq 'codex') {
        if (Test-Path (Join-Path $TargetRoot 'CLAUDE.md')) {
            [void]$issues.Add('Codex target must not generate CLAUDE.md')
        }
        foreach ($relativePath in @('AGENTS.md', '.codex/COORDINATOR-SUBAGENTS.md', 'docs/00-初始化结果索引.md')) {
            $fullPath = Join-Path $TargetRoot $relativePath
            if ((Test-Path $fullPath) -and ((Get-Content -Raw $fullPath) -like '*.codex/skills*')) {
                [void]$issues.Add("$relativePath references invalid Codex skill path .codex/skills")
            }
        }
        foreach ($duplicateHeadingIssue in (Get-DuplicateMarkdownSectionNumberIssues -TargetRoot $TargetRoot -RelativePaths @('AGENTS.md'))) {
            [void]$issues.Add($duplicateHeadingIssue)
        }
    } elseif ($TargetClient -eq 'claude-code') {
        if (Test-Path (Join-Path $TargetRoot 'AGENTS.md')) {
            [void]$issues.Add('Claude Code target must not generate AGENTS.md')
        }
    }

    @($issues | Sort-Object -Unique)
}

function ConvertTo-BulletList {
    param(
        [object[]]$Items,
        [string]$Fallback = '- 无'
    )

    $normalized = @(
        foreach ($item in (Normalize-ToArray -Value $Items)) {
            $text = [string]$item
            if ($text) { $text }
        }
    )

    if ($normalized.Count -eq 0) {
        return $Fallback
    }

    ($normalized | ForEach-Object { "- $_" }) -join "`n"
}

function Get-ImplementationHandoffChecklistItems {
    param(
        [object[]]$ChecklistSeed,
        [string]$ClientEntryFile,
        [string]$WorkflowStageText
    )

    $filtered = New-Object System.Collections.ArrayList
    foreach ($item in (Normalize-ToArray -Value $ChecklistSeed)) {
        $text = ([string]$item).Trim()
        if (-not $text) {
            continue
        }
        if ($text -match '初始化落盘|初始化线程|postcheck|bootstrap|再进入实施线程') {
            continue
        }
        [void]$filtered.Add($text)
    }
    if ($filtered.Count -gt 0) {
        return @($filtered)
    }

    @(
        "阅读 $ClientEntryFile、docs/project_context.md、docs/architecture/01-项目架构设计书.md"
        '阅读 docs/workflow/implementation-kickoff.md、docs/workflow/first-sprint-contract.md、docs/workflow/first-task-pack.md'
        "确认首轮实施范围、验收口径与 $WorkflowStageText 调度顺序"
        '进入业务实现前，将任务合同、责任角色和验证证据要求写清楚'
    )
}

function Expand-TemplatePlaceholders {
    param(
        [string]$TemplateContent,
        [hashtable]$ReplacementTable
    )

    [regex]::Replace($TemplateContent, '\{\{([a-zA-Z0-9_\-]+)\}\}', {
        param($match)
        $key = $match.Groups[1].Value
        if ($ReplacementTable.ContainsKey($key)) {
            return [string]$ReplacementTable[$key]
        }
        $match.Value
    })
}

function Get-RequiredCapabilitiesFromDecision {
    param($Decision)

    $capabilities = New-Object System.Collections.ArrayList
    $sourceCapabilities = @(Get-SelectedCapabilityRecordsFromDecision -Decision $Decision)
    foreach ($capability in $sourceCapabilities) {
        $name = if ($capability.PSObject.Properties.Match('name').Count -gt 0) { [string]$capability.name } else { [string]$capability }
        $displayName = if ($capability.PSObject.Properties.Match('display_name').Count -gt 0 -and [string]$capability.display_name) { [string]$capability.display_name } else { $name }
        if ($name) {
            [void]$capabilities.Add([pscustomobject]@{
                name = $name
                display_name = $displayName
            })
        }
    }

    if (@($capabilities).Count -eq 0) {
        foreach ($probe in (Normalize-ToArray -Value (Get-DecisionPropertyValue -DecisionObject $Decision -PropertyName 'capability_probe_results' -DefaultValue @()))) {
            $name = if ($probe.PSObject.Properties.Match('name').Count -gt 0) { [string]$probe.name } else { '' }
            $displayName = if ($probe.PSObject.Properties.Match('display_name').Count -gt 0 -and [string]$probe.display_name) { [string]$probe.display_name } else { $name }
            if ($name) {
                [void]$capabilities.Add([pscustomobject]@{
                    name = $name
                    display_name = $displayName
                })
            }
        }
    }

    @($capabilities | Sort-Object name -Unique)
}

function ConvertTo-RequiredCapabilityListText {
    param([object[]]$Capabilities)

    $normalized = @(
        foreach ($capability in (Normalize-ToArray -Value $Capabilities)) {
            $displayName = if ($capability.PSObject.Properties.Match('display_name').Count -gt 0 -and [string]$capability.display_name) {
                [string]$capability.display_name
            } elseif ($capability.PSObject.Properties.Match('name').Count -gt 0) {
                [string]$capability.name
            } else {
                [string]$capability
            }
            if ($displayName) { "- $displayName" }
        }
    )

    if ($normalized.Count -eq 0) {
        return '- 当前无必需能力记录'
    }

    $normalized -join "`n"
}

function ConvertTo-CapabilityProbeSummaryText {
    param([object[]]$ProbeResults)

    $normalized = @(
        foreach ($result in (Normalize-ToArray -Value $ProbeResults)) {
            $displayName = if ($result.PSObject.Properties.Match('display_name').Count -gt 0 -and [string]$result.display_name) {
                [string]$result.display_name
            } elseif ($result.PSObject.Properties.Match('name').Count -gt 0) {
                [string]$result.name
            } else {
                [string]$result
            }
            if ($displayName) {
                $evidence = if ($result.PSObject.Properties.Match('evidence').Count -gt 0 -and [string]$result.evidence) {
                    " | $([string]$result.evidence)"
                } else {
                    ''
                }
                "- ${displayName}: $(if ([bool]$result.passed) { 'pass' } else { 'fail' })$evidence"
            }
        }
    )

    if ($normalized.Count -eq 0) {
        return '- 当前无 capability probe 记录'
    }

    $normalized -join "`n"
}

function Test-AgentCapabilityDeclaration {
    param(
        [string]$Path,
        [object[]]$RequiredCapabilities
    )

    if (-not (Test-Path $Path)) {
        return $false
    }

    $content = Get-Content -Raw $Path
    if ($content -notmatch '## 必须具备的能力') {
        return $false
    }

    foreach ($capability in (Normalize-ToArray -Value $RequiredCapabilities)) {
        $displayName = if ($capability.PSObject.Properties.Match('display_name').Count -gt 0 -and [string]$capability.display_name) {
            [string]$capability.display_name
        } elseif ($capability.PSObject.Properties.Match('name').Count -gt 0) {
            [string]$capability.name
        } else {
            [string]$capability
        }
        if ($displayName -and ($content -notlike "*$displayName*")) {
            return $false
        }
    }

    $true
}

function Get-PrecheckEvidenceSummary {
    param(
        $Decision,
        $StatusObject
    )

    $probeResults = @(Normalize-ToArray -Value (Get-DecisionPropertyValue -DecisionObject $Decision -PropertyName 'capability_probe_results' -DefaultValue @()))
    $requiredCapabilities = @(Get-RequiredCapabilitiesFromDecision -Decision $Decision)
    $failedProbeResults = @(
        foreach ($probe in $probeResults) {
            if (-not [bool]$probe.passed) {
                if ($probe.PSObject.Properties.Match('display_name').Count -gt 0 -and [string]$probe.display_name) {
                    [string]$probe.display_name
                } elseif ($probe.PSObject.Properties.Match('name').Count -gt 0) {
                    [string]$probe.name
                }
            }
        }
    )

    $precheckRecordPassed = $false
    $precheckRecordPath = Get-SessionPath 'precheck.json'
    if (Test-Path $precheckRecordPath) {
        $precheckRecord = Read-JsonFile -Path $precheckRecordPath
        $precheckRecordPassed = [bool]$precheckRecord.passed
    }

    $statusPrecheckPassed = [bool](Get-StatusPropertyValue -StatusObject $StatusObject -PropertyName 'precheck_passed' -DefaultValue $false)
    $hasProbeResults = ($probeResults.Count -gt 0)
    $hasRequiredCapabilities = ($requiredCapabilities.Count -gt 0)
    $passed = ($hasRequiredCapabilities -and $hasProbeResults -and ($failedProbeResults.Count -eq 0) -and ($precheckRecordPassed -or $statusPrecheckPassed))

    $missingEvidence = New-Object System.Collections.ArrayList
    if (-not $hasRequiredCapabilities) {
        [void]$missingEvidence.Add('decision.required_capabilities')
    }
    if (-not $hasProbeResults) {
        [void]$missingEvidence.Add('decision.capability_probe_results')
    }
    if (-not ($precheckRecordPassed -or $statusPrecheckPassed)) {
        [void]$missingEvidence.Add('.commonhe/session/precheck.json or status.precheck_passed')
    }

    [pscustomobject]@{
        Passed = $passed
        FailedProbeResults = @($failedProbeResults)
        MissingEvidence = @($missingEvidence)
    }
}

function Get-HandoffGeneratedFiles {
    param(
        [string]$TargetRoot,
        [object[]]$GenerationResults
    )

    if ($GenerationResults) {
        return @($GenerationResults | ForEach-Object { [string]$_.Target })
    }

    $decision = Read-JsonFile -Path (Get-SessionPath 'decision.json')
    $expectedRoles = @(Get-ExpectedRoleNames -Decision $decision)
    $targetClient = [string](Get-DecisionPropertyValue -DecisionObject $decision -PropertyName 'target_client' -DefaultValue 'codex')
    $speckitEnabled = @(
        foreach ($capability in (Get-RequiredCapabilitiesFromDecision -Decision $decision)) {
            $name = Get-CapabilityIdentity -Capability $capability
            if ($name -and $name.Equals('Speckit', [System.StringComparison]::OrdinalIgnoreCase)) { $name }
        }
    ).Count -gt 0
    $clientCoreFiles = if ($targetClient -eq 'claude-code') {
        @(
            'CLAUDE.md'
            '.claude/settings.json'
            '.claude/skills/required-capabilities.md'
        )
    } else {
        @(
            'AGENTS.md'
            '.codex/COORDINATOR-SUBAGENTS.md'
            '.agents/skills/required-capabilities.md'
        )
    }
    $files = New-Object System.Collections.ArrayList
    $agentRoot = if ($targetClient -eq 'claude-code') { '.claude/agents' } else { '.codex/agents' }
    $coreFiles = @(
        'docs/00-初始化结果索引.md'
        $clientCoreFiles
        'docs/project_context.md'
        'docs/architecture/01-项目架构设计书.md'
        'docs/roadmap/01-实施路线图.md'
        'docs/skills/required-capabilities.md'
        'docs/workflow/current-stage-user-checklist.md'
        'docs/workflow/archive-policy.md'
        'docs/workflow/acceptance-gates.md'
        'docs/workflow/evaluator-protocol.md'
        'docs/workflow/grading-criteria.md'
        'docs/workflow/sprint-contract-template.md'
        'docs/workflow/implementation-kickoff.md'
        'docs/workflow/first-sprint-contract.md'
        'docs/workflow/first-task-pack.md'
    )

    foreach ($relativePath in $coreFiles) {
        $fullPath = Join-Path $TargetRoot $relativePath
        if (Test-Path $fullPath) {
            [void]$files.Add([System.IO.Path]::GetFullPath($fullPath))
        }
    }

    foreach ($role in $expectedRoles) {
        $handbookPath = Join-Path $TargetRoot "docs/agents/$role-handbook.md"
        if (Test-Path $handbookPath) {
            [void]$files.Add([System.IO.Path]::GetFullPath($handbookPath))
        }
    }

    foreach ($agentName in (Get-ExpectedAgencyAgentNames -ExpectedRoles $expectedRoles)) {
        $agentPath = Join-Path $TargetRoot "$agentRoot/$agentName.md"
        if (Test-Path $agentPath) {
            [void]$files.Add([System.IO.Path]::GetFullPath($agentPath))
        }
    }

    @($files | Sort-Object -Unique)
}

function Write-BootstrapHandoff {
    param(
        [string]$TargetRoot,
        [object]$PostcheckSummary,
        [object[]]$GenerationResults = @()
    )

    if (-not (Test-Path $bootstrapHandoffTemplatePath)) {
        throw "Bootstrap handoff template not found: $bootstrapHandoffTemplatePath"
    }

    $decision = Read-JsonFile -Path (Get-SessionPath 'decision.json')
    $expectedRoles = @(Get-ExpectedRoleNames -Decision $decision)
    $generatedFiles = Get-HandoffGeneratedFiles -TargetRoot $TargetRoot -GenerationResults $GenerationResults
    $normalizedDecisionIntegrations = @(Normalize-ToArray -Value (Get-DecisionPropertyValue -DecisionObject $decision -PropertyName 'integrations' -DefaultValue @()))
    $decisionCurrentStage = [string](Get-DecisionPropertyValue -DecisionObject $decision -PropertyName 'current_stage' -DefaultValue 'implementation-v1')
    $targetClient = [string](Get-DecisionPropertyValue -DecisionObject $decision -PropertyName 'target_client' -DefaultValue 'codex')
    $targetClientName = Get-TargetClientName -TargetClient $targetClient
    $clientEntryFile = if ($targetClient -eq 'claude-code') { 'CLAUDE.md' } else { 'AGENTS.md' }
    $clientCoordinatorPath = if ($targetClient -eq 'claude-code') { '.claude/settings.json' } else { '.codex/COORDINATOR-SUBAGENTS.md' }
    $clientSkillPath = if ($targetClient -eq 'claude-code') { '.claude/skills/required-capabilities.md' } else { '.agents/skills/required-capabilities.md' }
    $integrationDisplays = @(
        foreach ($integration in $normalizedDecisionIntegrations) {
            $displayName = Get-IntegrationDisplayName -Integration $integration
            $integrationName = Get-IntegrationName -Integration $integration
            if ($integrationName) {
                if ($displayName -and $displayName -ne $integrationName) {
                    "integration-$integrationName ($displayName)"
                } else {
                    "integration-$integrationName"
                }
            }
        }
    )

    $defaultRoles = @('architect', 'backend', 'frontend', 'reviewer', 'qa', 'docs')
    $extraGeneratedItems = @(
        foreach ($role in $expectedRoles) {
            if ($defaultRoles -notcontains $role) {
                "追加角色：$role"
            }
        }
        foreach ($integrationDisplay in $integrationDisplays) {
            "集成角色：$integrationDisplay"
        }
    ) | Select-Object -Unique

    $postcheckItems = @("postcheck：$(if ([bool]$PostcheckSummary.Passed) { '通过' } else { '未通过' })")
    foreach ($fieldName in @('MissingCoreFiles', 'MissingAgentFiles', 'MissingHandbooks', 'UnexpectedAgentFiles', 'UnexpectedHandbooks', 'MissingCapabilityDeclarations', 'DanglingRoleReferences', 'BrokenWorkflowReferences', 'InvalidWorkflowContent', 'AuthorshipQualityIssues', 'TruthSourceGateIssues', 'MissingCapabilityEvidence', 'FailedCapabilityProbes')) {
        $items = @(Normalize-ToArray -Value $PostcheckSummary.$fieldName)
        if ($items.Count -gt 0) {
            foreach ($item in $items) {
                $postcheckItems += "$fieldName -> $item"
            }
        }
    }
    $postcheckItems += [string]$PostcheckSummary.RecommendedFix

    $truthSources = @(
        $clientEntryFile
        $clientCoordinatorPath
        $clientSkillPath
        'docs/project_context.md'
        'docs/architecture/01-项目架构设计书.md'
        'docs/roadmap/01-实施路线图.md'
        'docs/skills/required-capabilities.md'
        'docs/workflow/evaluator-protocol.md'
        'docs/workflow/grading-criteria.md'
        'docs/workflow/sprint-contract-template.md'
        'docs/workflow/implementation-kickoff.md'
        'docs/workflow/first-sprint-contract.md'
        'docs/workflow/first-task-pack.md'
    )

    $closureChecklistItems = if ([bool]$PostcheckSummary.Passed) {
        @(
            '[ ] postcheck 已通过'
            "[ ] 已提示用户在 $targetClientName 中新开会话或重启会话"
            '[ ] 已提示当前线程不继续业务实现'
            "[ ] 已说明不得删除 docs/$clientCoordinatorPath/$clientEntryFile/.commonhe/session"
        )
    } else {
        @(
            '[ ] 当前不得宣布初始化成功'
            '[ ] 请先修复缺失项、清理冗余角色文件，并移除未启用角色引用'
            '[ ] 修复后重新执行 postcheck'
            '[ ] 在 postcheck 通过前不要新开业务实现线程'
        )
    }

    $nextRecommendedSteps = if ([bool]$PostcheckSummary.Passed) {
        @(
            "当前初始化协作包已收口，可进入实施接手阶段：$decisionCurrentStage"
            "请在 $targetClientName 中新开会话或重启会话，让新生成的协作协议正式生效。"
            '在新的线程中按实施态真源继续项目工作。'
        )
    } else {
        @(
            '当前不得宣布初始化成功。'
            '请先修复缺失项、清理冗余角色文件，并移除未启用角色引用。'
            '修复后重新执行 postcheck，再决定是否收口初始化流程。'
        )
    }

    $stillAdjustableItems = @(
        '后续仍可按新一轮确认结果增删角色、集成或文档深度。'
        '业务系统实现、技术选型或 AI 框架设计不应继续在当前初始化线程中展开。'
    )

    $templateContent = Get-Content -Raw $bootstrapHandoffTemplatePath
    $expanded = Expand-TemplatePlaceholders -TemplateContent $templateContent -ReplacementTable @{
        generated_files = ConvertTo-BulletList -Items $generatedFiles
        enabled_roles = ConvertTo-BulletList -Items $expectedRoles
        extra_generated_items = ConvertTo-BulletList -Items $extraGeneratedItems
        postcheck_result = ConvertTo-BulletList -Items $postcheckItems
        truth_sources = ConvertTo-BulletList -Items $truthSources
        next_recommended_steps = ConvertTo-BulletList -Items $nextRecommendedSteps
        closure_checklist_items = ConvertTo-BulletList -Items $closureChecklistItems
        still_adjustable_items = ConvertTo-BulletList -Items $stillAdjustableItems
    }

    $handoffPath = Get-SessionPath 'bootstrap-handoff.md'
    Set-Content -Path $handoffPath -Value $expanded
    $handoffPath
}

function Invoke-PostBootstrapCheck {
    param(
        [string]$TargetRoot,
        [switch]$PersistStatus
    )

    if (-not $TargetRoot) {
        throw "TargetRoot is required for postcheck stage."
    }

    $targetRoot = [System.IO.Path]::GetFullPath($TargetRoot)
    $decision = Read-JsonFile -Path (Get-SessionPath 'decision.json')
    $statusBeforePostcheck = if (Test-Path (Get-SessionPath 'status.json')) {
        Read-JsonFile -Path (Get-SessionPath 'status.json')
    } else {
        $null
    }
    $wasClosedBeforePostcheck = Test-ClosureGateActive -StatusObject $statusBeforePostcheck
    $expectedRoles = @(Get-ExpectedRoleNames -Decision $decision)
    $targetClient = [string](Get-DecisionPropertyValue -DecisionObject $decision -PropertyName 'target_client' -DefaultValue 'codex')
    $requiredCapabilities = @(Get-RequiredCapabilitiesFromDecision -Decision $decision)
    $requiredCapabilityNames = @(
        foreach ($capability in $requiredCapabilities) {
            $name = Get-CapabilityIdentity -Capability $capability
            if ($name) { $name.ToLowerInvariant() }
        }
    )
    $speckitEnabled = ($requiredCapabilityNames -contains 'speckit')
    $clientCorePaths = if ($targetClient -eq 'claude-code') {
        @(
            'CLAUDE.md'
            '.claude/settings.json'
            '.claude/skills/required-capabilities.md'
        )
    } else {
        @(
            'AGENTS.md'
            '.codex/COORDINATOR-SUBAGENTS.md'
            '.agents/skills/required-capabilities.md'
        )
    }
    $agentRoot = if ($targetClient -eq 'claude-code') { '.claude/agents' } else { '.codex/agents' }
    $coreRelativePaths = @(
        'docs/project_context.md'
        'docs/architecture/01-项目架构设计书.md'
        'docs/roadmap/01-实施路线图.md'
        'docs/skills/required-capabilities.md'
        'docs/workflow/current-stage-user-checklist.md'
        'docs/workflow/archive-policy.md'
        'docs/workflow/acceptance-gates.md'
        'docs/workflow/evaluator-protocol.md'
        'docs/workflow/grading-criteria.md'
        'docs/workflow/sprint-contract-template.md'
        'docs/workflow/implementation-kickoff.md'
        'docs/workflow/first-sprint-contract.md'
        'docs/workflow/first-task-pack.md'
    ) + $clientCorePaths
    if ($speckitEnabled) {
        $coreRelativePaths += @(
            '.specify/templates/spec-template.md'
            '.specify/scripts/powershell/create-new-feature.ps1'
        )
    }

    $missingCoreFiles = @(
        foreach ($relativePath in $coreRelativePaths) {
            if (-not (Test-Path (Join-Path $targetRoot $relativePath))) {
                $relativePath
            }
        }
    )

    $expectedAgencyAgentNames = @(Get-ExpectedAgencyAgentNames -ExpectedRoles $expectedRoles)
    $expectedAgentFiles = @(
        foreach ($agentName in $expectedAgencyAgentNames) {
            "$agentRoot/$agentName.md"
        }
    )
    $expectedHandbooks = @(
        foreach ($role in $expectedRoles) {
            "docs/agents/$role-handbook.md"
        }
    )

    $missingAgentFiles = @(
        foreach ($relativePath in $expectedAgentFiles) {
            if (-not (Test-Path (Join-Path $targetRoot $relativePath))) {
                $relativePath
            }
        }
    )
    $missingHandbooks = @(
        foreach ($relativePath in $expectedHandbooks) {
            if (-not (Test-Path (Join-Path $targetRoot $relativePath))) {
                $relativePath
            }
        }
    )

    $actualAgentFiles = @()
    $agentDir = Join-Path $targetRoot $agentRoot
    if (Test-Path $agentDir) {
        $actualAgentFiles = @(
            Get-ChildItem -Path $agentDir -File -Filter *.md |
                ForEach-Object { "$agentRoot/$($_.Name)" } |
                Sort-Object
        )
    }

    $actualHandbooks = @()
    $handbookDir = Join-Path $targetRoot 'docs/agents'
    if (Test-Path $handbookDir) {
        $actualHandbooks = @(
            Get-ChildItem -Path $handbookDir -File -Filter *.md |
                ForEach-Object { "docs/agents/$($_.Name)" } |
                Sort-Object
        )
    }

    $unexpectedAgentFiles = @(
        foreach ($relativePath in $actualAgentFiles) {
            if ($expectedAgentFiles -notcontains $relativePath) {
                $relativePath
            }
        }
    )
    $unexpectedHandbooks = @(
        foreach ($relativePath in $actualHandbooks) {
            if ($expectedHandbooks -notcontains $relativePath) {
                $relativePath
            }
        }
    )

    $actualRoles = @(
        @(
            foreach ($relativePath in $actualAgentFiles) {
                [System.IO.Path]::GetFileNameWithoutExtension($relativePath)
            }
            foreach ($relativePath in $actualHandbooks) {
                [System.IO.Path]::GetFileNameWithoutExtension($relativePath) -replace '-handbook$'
            }
        ) | Sort-Object -Unique
    )

    $missingCapabilityDeclarations = @(
        foreach ($relativePath in $expectedAgentFiles) {
            $fullPath = Join-Path $targetRoot $relativePath
            if ((Test-Path $fullPath) -and -not (Test-AgentCapabilityDeclaration -Path $fullPath -RequiredCapabilities $requiredCapabilities)) {
                $relativePath
            }
        }
    )
    $precheckEvidence = Get-PrecheckEvidenceSummary -Decision $decision -StatusObject $statusBeforePostcheck
    $danglingRoleReferences = @(Get-DanglingRoleReferences `
        -TargetRoot $targetRoot `
        -Decision $decision `
        -ExpectedRoles $expectedRoles `
        -ExpectedAgentFiles $expectedAgentFiles `
        -ExpectedHandbooks $expectedHandbooks)
    $brokenWorkflowReferences = @(Get-BrokenWorkflowReferences -TargetRoot $targetRoot -ExpectedRoles $expectedRoles -TargetClient $targetClient)
    $invalidWorkflowContent = @(Get-InvalidWorkflowContent -TargetRoot $targetRoot)
    $authorshipQualityIssues = @(Get-AuthorshipQualityIssues -TargetRoot $targetRoot -TargetClient $targetClient)
    if (-not $speckitEnabled -and (Test-Path (Join-Path $targetRoot '.specify'))) {
        $authorshipQualityIssues += 'unselected_capability:.specify generated even though Speckit was not selected'
    }
    $truthSourceGate = Invoke-TruthSourceGate -TargetRoot $targetRoot -TargetClient $targetClient

    $passed = ($missingCoreFiles.Count -eq 0) -and
        ($missingAgentFiles.Count -eq 0) -and
        ($missingHandbooks.Count -eq 0) -and
        ($unexpectedAgentFiles.Count -eq 0) -and
        ($unexpectedHandbooks.Count -eq 0) -and
        ($missingCapabilityDeclarations.Count -eq 0) -and
        ($danglingRoleReferences.Count -eq 0) -and
        ($brokenWorkflowReferences.Count -eq 0) -and
        ($invalidWorkflowContent.Count -eq 0) -and
        ($authorshipQualityIssues.Count -eq 0) -and
        ([bool]$truthSourceGate.Passed) -and
        (@($precheckEvidence.MissingEvidence).Count -eq 0) -and
        (@($precheckEvidence.FailedProbeResults).Count -eq 0)

    $targetClientName = Get-TargetClientName -TargetClient $targetClient
    $recommendedFix = if ($passed) {
        "当前生成结果已符合已确认方案，可以收口初始化流程并提示用户在 $targetClientName 中新开会话或重启会话。"
    } else {
        '请先修复缺失项、补齐能力声明与 precheck 证据，清理冗余角色文件，修正 workflow 引用，移除模板残留/错误目标软件入口，并移除实施态文档中的 init-only 内容后，再重新执行 postcheck；未通过前不得宣布初始化成功。'
    }

    $summary = [pscustomobject]@{
        Passed = $passed
        MissingCoreFiles = $missingCoreFiles
        MissingAgentFiles = $missingAgentFiles
        MissingHandbooks = $missingHandbooks
        UnexpectedAgentFiles = $unexpectedAgentFiles
        UnexpectedHandbooks = $unexpectedHandbooks
        MissingCapabilityDeclarations = $missingCapabilityDeclarations
        DanglingRoleReferences = $danglingRoleReferences
        BrokenWorkflowReferences = $brokenWorkflowReferences
        InvalidWorkflowContent = $invalidWorkflowContent
        AuthorshipQualityIssues = $authorshipQualityIssues
        TruthSourceGatePassed = [bool]$truthSourceGate.Passed
        TruthSourceGateIssues = @($truthSourceGate.Issues)
        MissingCapabilityEvidence = @($precheckEvidence.MissingEvidence)
        FailedCapabilityProbes = @($precheckEvidence.FailedProbeResults)
        ExpectedRoles = $expectedRoles
        ActualRoles = $actualRoles
        RecommendedFix = $recommendedFix
    }

    if ($PersistStatus) {
        if ($passed) {
            Set-Status -StageName 'implementation_ready' -LastPostcheckPassed $true -InitClosed $true -LastPostcheckSummary $summary -ImplementationStagePromoted $true -ClosureGateActive $true -CapabilityGatePassed $true -TruthSourceGatePassed $true -TruthSourceGateFailed $false -TruthSourceGateIssues @()
        } elseif ($wasClosedBeforePostcheck) {
            Set-Status -StageName 'implementation_ready' -LastPostcheckPassed $false -InitClosed $true -LastPostcheckSummary $summary -ImplementationStagePromoted $true -ClosureGateActive $true -CapabilityGatePassed $false -TruthSourceGatePassed ([bool]$truthSourceGate.Passed) -TruthSourceGateFailed (-not [bool]$truthSourceGate.Passed) -TruthSourceGateIssues @($truthSourceGate.Issues)
        } else {
            Set-Status -StageName 'postcheck_failed' -LastPostcheckPassed $false -InitClosed $false -LastPostcheckSummary $summary -ClosureGateActive $false -CapabilityGatePassed $false -TruthSourceGatePassed ([bool]$truthSourceGate.Passed) -TruthSourceGateFailed (-not [bool]$truthSourceGate.Passed) -TruthSourceGateIssues @($truthSourceGate.Issues)
        }
    }

    $summary
}

function Get-SuccessClosureMessage {
    param([string]$TargetClient = '')

    $targetClientName = Get-TargetClientName -TargetClient $(if ($TargetClient) { $TargetClient } else { Get-CurrentTargetClient })
    "初始化协作包已生成并通过 postcheck。请在 $targetClientName 中新开会话或重启会话，让新生成的协作协议生效；当前初始化线程到此收口，不继续业务实现。"
}

function Get-PostcheckFailureMessage {
    '初始化协作包生成已完成，但 postcheck 未通过；当前不得宣布初始化成功。请先修复缺失项、清理冗余角色文件、补齐 workflow 引用，并移除实施态文档中的 init-only 内容后，再重新执行 postcheck。'
}

function Invoke-PrecheckStage {
    param([string]$ProjectRoot)

    if (-not (Test-Path (Get-SessionPath 'status.json'))) {
        Ensure-SessionSkeleton
    }

    $resolvedProjectRoot = if ($ProjectRoot) {
        [System.IO.Path]::GetFullPath($ProjectRoot)
    } else {
        Get-ResolvedProjectRoot
    }

    $summary = Invoke-Precheck -ProjectRoot $resolvedProjectRoot -PersistStatus -LegacyProjectDetected (Test-IsLegacyProject -ProjectRoot $resolvedProjectRoot) -LegacyZeroQuestionMode (Test-IsLegacyProject -ProjectRoot $resolvedProjectRoot)
    if ([bool]$summary.passed) {
        [pscustomobject]@{
            Stage = 'precheck'
            SessionRoot = $sessionRoot
            ProjectRoot = $resolvedProjectRoot
            Precheck = $summary
            Message = '五项能力已通过 precheck，可以继续初始化。'
        }
    } else {
        [pscustomobject]@{
            Stage = 'precheck_failed'
            SessionRoot = $sessionRoot
            ProjectRoot = $resolvedProjectRoot
            Precheck = $summary
            Message = Get-PrecheckFailureMessage -PrecheckSummary $summary
        }
    }
}

function Invoke-DoctorStage {
    param([string]$ProjectRoot)

    if (-not (Test-Path (Get-SessionPath 'status.json'))) {
        Ensure-SessionSkeleton
    }

    $resolvedProjectRoot = if ($ProjectRoot) {
        [System.IO.Path]::GetFullPath($ProjectRoot)
    } else {
        Get-ResolvedProjectRoot
    }
    $legacyDetected = Test-IsLegacyProject -ProjectRoot $resolvedProjectRoot
    $summary = Invoke-Doctor -ProjectRoot $resolvedProjectRoot -PersistStatus -LegacyProjectDetected $legacyDetected -LegacyZeroQuestionMode $legacyDetected
    if ([bool]$summary.passed) {
        [pscustomobject]@{
            Stage = 'doctor'
            SessionRoot = $sessionRoot
            ProjectRoot = $resolvedProjectRoot
            Doctor = $summary
            Message = 'doctor 已通过，可以继续 precheck。'
        }
    } else {
        [pscustomobject]@{
            Stage = 'doctor_failed'
            SessionRoot = $sessionRoot
            ProjectRoot = $resolvedProjectRoot
            Doctor = $summary
            Message = Get-DoctorFailureMessage -DoctorSummary $summary
        }
    }
}

function Start-LegacyProjectInitialization {
    param(
        [string]$ProjectRoot,
        $DoctorSummary = $null,
        $PrecheckSummary = $null
    )

    $resolvedProjectRoot = [System.IO.Path]::GetFullPath($ProjectRoot)
    if (-not $DoctorSummary) {
        $DoctorSummary = Invoke-Doctor -ProjectRoot $resolvedProjectRoot -PersistStatus -LegacyProjectDetected $true -LegacyZeroQuestionMode $true
    }
    if (-not [bool]$DoctorSummary.passed) {
        return [pscustomobject]@{
            Stage = 'doctor_failed'
            SessionRoot = $sessionRoot
            ProjectRoot = $resolvedProjectRoot
            Doctor = $DoctorSummary
            Message = Get-DoctorFailureMessage -DoctorSummary $DoctorSummary
        }
    }

    $precheckSummary = $PrecheckSummary
    if (-not $precheckSummary) {
        $precheckSummary = Invoke-Precheck -ProjectRoot $resolvedProjectRoot -PersistStatus -LegacyProjectDetected $true -LegacyZeroQuestionMode $true
    }
    if (-not [bool]$precheckSummary.passed) {
        return [pscustomobject]@{
            Stage = 'precheck_failed'
            SessionRoot = $sessionRoot
            ProjectRoot = $resolvedProjectRoot
            Precheck = $precheckSummary
            Message = Get-PrecheckFailureMessage -PrecheckSummary $precheckSummary
        }
    }

    Set-Status -StageName 'autodiscovery' -DoctorPassed $true -DoctorFailed $false -EntrycheckPassed $true -EntrycheckFailed $false -MisplacedPackageDetected $false -PrecheckPassed $true -PrecheckFailed $false -LegacyProjectDetected $true -LegacyZeroQuestionMode $true -MissingCapabilities @()
    $analysis = Get-LegacyProjectAnalysis -ProjectRoot $resolvedProjectRoot
    Write-ProjectAnalysisArtifacts -Analysis $analysis
    Write-AutodiscoveryDecision -Analysis $analysis -PrecheckSummary $precheckSummary | Out-Null
    Set-Status `
        -StageName 'autodiscovery_ready' `
        -DoctorPassed $true `
        -DoctorFailed $false `
        -EntrycheckPassed $true `
        -EntrycheckFailed $false `
        -MisplacedPackageDetected $false `
        -PrecheckPassed $true `
        -PrecheckFailed $false `
        -LegacyProjectDetected $true `
        -LegacyZeroQuestionMode $true `
        -MissingCapabilities @() `
        -CurrentDeliveryMode ([string]$analysis.delivery_mode) `
        -CurrentStageGoal ([string]$analysis.current_stage_goal)

    Invoke-BootstrapStage -TargetRoot $resolvedProjectRoot -Execute -Force
}

function Start-Conversation {
    $resolvedProjectRoot = Get-ResolvedProjectRoot
    Ensure-SessionSkeleton -ProjectRoot $resolvedProjectRoot -Provider $Provider -Model $Model -BaseUrl $BaseUrl -ApiKey $ApiKey

    $legacyDetected = Test-IsLegacyProject -ProjectRoot $resolvedProjectRoot
    $doctorSummary = Invoke-Doctor -ProjectRoot $resolvedProjectRoot -PersistStatus -LegacyProjectDetected $legacyDetected -LegacyZeroQuestionMode $legacyDetected
    if (-not [bool]$doctorSummary.passed) {
        return [pscustomobject]@{
            Stage = 'doctor_failed'
            SessionRoot = $sessionRoot
            ProjectRoot = $resolvedProjectRoot
            Doctor = $doctorSummary
            Message = Get-DoctorFailureMessage -DoctorSummary $doctorSummary
        }
    }

    $precheckSummary = Invoke-Precheck -ProjectRoot $resolvedProjectRoot -PersistStatus -LegacyProjectDetected $legacyDetected -LegacyZeroQuestionMode $legacyDetected
    if (-not [bool]$precheckSummary.passed) {
        return [pscustomobject]@{
            Stage = 'precheck_failed'
            SessionRoot = $sessionRoot
            ProjectRoot = $resolvedProjectRoot
            Precheck = $precheckSummary
            Message = Get-PrecheckFailureMessage -PrecheckSummary $precheckSummary
        }
    }

    if ($legacyDetected) {
        return Start-LegacyProjectInitialization -ProjectRoot $resolvedProjectRoot -DoctorSummary $doctorSummary -PrecheckSummary $precheckSummary
    }

    $question = Get-CurrentQuestion
    $questionSource = [string](Get-StatusPropertyValue -StatusObject (Read-JsonFile -Path (Get-SessionPath 'status.json')) -PropertyName 'question_source' -DefaultValue 'fallback template')
    Set-Status -StageName 'discovery' -DoctorPassed $true -DoctorFailed $false -EntrycheckPassed $true -EntrycheckFailed $false -MisplacedPackageDetected $false -PrecheckPassed $true -PrecheckFailed $false -LegacyProjectDetected $false -LegacyZeroQuestionMode $false -MissingCapabilities @()
    [pscustomobject]@{
        Stage = 'discovery'
        SessionRoot = $sessionRoot
        QuestionId = $question.id
        QuestionText = $question.prompt
        QuestionSource = $questionSource
    }
}

function Submit-Answer {
    param([string]$InputText)

    if (-not $InputText) { throw "InputText is required for answer stage." }

    $status = Read-JsonFile -Path (Get-SessionPath 'status.json')
    if ([string]$status.stage -ne 'discovery') {
        throw "Answer stage is only allowed while the session is in discovery."
    }

    $questions = Read-JsonFile -Path (Get-SessionPath 'questions.json')
    $answers = Read-JsonFile -Path (Get-SessionPath 'answers.json')
    $index = [int]$status.current_question_index
    $question = $questions[$index]

    $answerTable = @{}
    foreach ($property in $answers.PSObject.Properties) {
        $answerTable[$property.Name] = [string]$property.Value
    }
    $answerTable[$question.id] = $InputText
    Write-JsonFile -Path (Get-SessionPath 'answers.json') -Value $answerTable
    Append-DiscoveryMarkdown -Question $question.prompt -Answer $InputText

    $nextIndex = $index + 1
    if ($nextIndex -ge $questions.Count) {
        Set-Status -StageName 'proposal_ready'
        [pscustomobject]@{
            Stage = 'proposal_ready'
            SessionRoot = $sessionRoot
            Message = 'Discovery 已完成，可以进入 propose 阶段。'
        }
    } else {
        Set-Status -StageName 'discovery' -CurrentQuestionIndex $nextIndex
        $nextQuestion = $questions[$nextIndex]
        [pscustomobject]@{
            Stage = 'discovery'
            SessionRoot = $sessionRoot
            QuestionId = $nextQuestion.id
            QuestionText = $nextQuestion.prompt
            QuestionSource = [string](Get-StatusPropertyValue -StatusObject (Read-JsonFile -Path (Get-SessionPath 'status.json')) -PropertyName 'question_source' -DefaultValue 'fallback template')
        }
    }
}

function Build-Proposal {
    $status = Read-JsonFile -Path (Get-SessionPath 'status.json')
    if ([string]$status.stage -notin @('proposal_ready', 'proposal', 'confirmed')) {
        throw "Proposal can only be generated after discovery is complete."
    }

    $answers = Get-AnswerTable
    $proposalPack = Get-ProposalOptions -Answers $answers
    Write-JsonFile -Path (Get-SessionPath 'proposal-options.json') -Value $proposalPack.Options
    Write-ProposalMarkdown -Answers $answers -Options $proposalPack.Options -Recommended $proposalPack.Recommended
    Set-Status -StageName 'proposal'
    $recommendedOption = @($proposalPack.Options | Where-Object { [string]$_.id -eq [string]$proposalPack.Recommended } | Select-Object -First 1)[0]

    [pscustomobject]@{
        Stage = 'proposal'
        SessionRoot = $sessionRoot
        Options = @($proposalPack.Options)
        Recommended = $proposalPack.Recommended
        RecommendedOption = $recommendedOption
        ProposalPath = (Get-SessionPath 'proposal.md')
        Message = '候选方案已生成，请由用户明确拍板后再进入 confirm。'
    }
}

function Confirm-Proposal {
    param([string]$Choice)

    if (-not $Choice) { throw "Choice is required for confirm stage." }

    $status = Read-JsonFile -Path (Get-SessionPath 'status.json')
    if ([string]$status.stage -ne 'proposal') {
        throw "Confirm stage is only allowed after proposal generation."
    }

    $options = Read-JsonFile -Path (Get-SessionPath 'proposal-options.json')
    $selected = $options | Where-Object { [string]$_.id -eq $Choice } | Select-Object -First 1
    if (-not $selected) { throw "Unknown proposal choice: $Choice" }

    $answers = Get-AnswerTable
    $existingDecision = Read-JsonFile -Path (Get-SessionPath 'decision.json')
    $targetClient = [string](Get-DecisionPropertyValue -DecisionObject $existingDecision -PropertyName 'target_client' -DefaultValue 'codex')
    $requiredCapabilities = @(Get-SelectedCapabilityRecordsFromDecision -Decision $existingDecision)
    $requiredNames = @(
        foreach ($capability in $requiredCapabilities) {
            $name = Get-CapabilityIdentity -Capability $capability
            if ($name) { $name.ToLowerInvariant() }
        }
    )
    $capabilityProbeResults = @(
        foreach ($probe in (Normalize-ToArray -Value (Get-DecisionPropertyValue -DecisionObject $existingDecision -PropertyName 'capability_probe_results' -DefaultValue @()))) {
            $name = Get-CapabilityIdentity -Capability $probe
            if ($requiredNames.Count -eq 0 -or ($name -and ($requiredNames -contains $name.ToLowerInvariant()))) {
                $probe
            }
        }
    )
    $selectedCapabilities = @(Get-MandatorySelectedCapabilityRecords -RequiredCapabilities $requiredCapabilities -ProbeResults $capabilityProbeResults)
    $decision = @{
        user_confirmed = $true
        auto_confirmed = $false
        confirmation_mode = 'user_choice'
        discovery_mode = 'interactive_confirmed'
        project_name = if ($answers.ContainsKey('project_name')) { $answers['project_name'] } else { 'UnnamedProject' }
        target_client = $targetClient
        selected_capabilities = @($selectedCapabilities)
        selected_solution_id = $Choice
        selected_solution_title = [string](Get-DecisionPropertyValue -DecisionObject $selected -PropertyName 'name' -DefaultValue '')
        project_type = [string]$selected.project_type
        delivery_mode = [string]$selected.delivery_mode
        solution_mode = [string]$selected.solution_mode
        enabled_roles = @(Normalize-ToArray -Value $selected.enabled_roles)
        recommended_roles_now = @(Normalize-ToArray -Value $selected.recommended_roles_now)
        available_roles_later = @(Normalize-ToArray -Value $selected.available_roles_later)
        integrations = @(Get-NormalizedIntegrations -Integrations $selected.integrations)
        detected_integrations = @(Get-NormalizedIntegrations -Integrations $selected.detected_integrations)
        external_references = @(Normalize-ToArray -Value $selected.external_references)
        current_stage = [string]$selected.current_stage
        current_stage_goal = [string]$selected.current_stage_goal
        primary_workstream = [string]$selected.primary_workstream
        stage_constraints = @(Normalize-ToArray -Value $selected.stage_constraints)
        deferred_capabilities = @(Normalize-ToArray -Value $selected.deferred_capabilities)
        implementation_checklist_seed = @(Normalize-ToArray -Value $selected.implementation_checklist_seed)
        implementation_acceptance_seed = @(Normalize-ToArray -Value $selected.implementation_acceptance_seed)
        domain_modules = @(Normalize-ToArray -Value (Get-DecisionPropertyValue -DecisionObject $selected -PropertyName 'domain_modules' -DefaultValue @()))
        solution_architecture_summary = [string](Get-DecisionPropertyValue -DecisionObject $selected -PropertyName 'architecture_summary' -DefaultValue '')
        solution_team_composition = @(Normalize-ToArray -Value (Get-DecisionPropertyValue -DecisionObject $selected -PropertyName 'team_composition' -DefaultValue @()))
        solution_token_estimate = [string](Get-DecisionPropertyValue -DecisionObject $selected -PropertyName 'token_estimate' -DefaultValue '')
        solution_recommendation_text = [string](Get-DecisionPropertyValue -DecisionObject $selected -PropertyName 'recommendation_text' -DefaultValue '')
        required_capabilities = @($requiredCapabilities)
        capability_probe_results = @($capabilityProbeResults)
        legacy_analysis_version = ''
        signal_categories = @()
        role_rationale = (Get-DecisionPropertyValue -DecisionObject $selected -PropertyName 'role_rationale' -DefaultValue @{})
        omitted_role_rationale = (Get-DecisionPropertyValue -DecisionObject $selected -PropertyName 'omitted_role_rationale' -DefaultValue @{})
        confidence_breakdown = @{}
        dominant_workstreams = @()
        kickoff_pack = @{
            implementation_kickoff = 'docs/workflow/implementation-kickoff.md'
            first_sprint_contract = 'docs/workflow/first-sprint-contract.md'
            first_task_pack = 'docs/workflow/first-task-pack.md'
        }
        analysis_confidence = ''
        autodiscovery_signals = @()
        autodiscovery_assumptions = @()
        project_goal_summary = if ($answers.ContainsKey('project_goal')) { $answers['project_goal'] } else { '' }
        target_users_summary = if ($answers.ContainsKey('target_users')) { $answers['target_users'] } else { '' }
        core_features_summary = if ($answers.ContainsKey('core_features')) { $answers['core_features'] } else { '' }
        constraints_summary = if ($answers.ContainsKey('constraints')) { $answers['constraints'] } else { '' }
    }
    Write-JsonFile -Path (Get-SessionPath 'decision.json') -Value $decision
    Set-Status `
        -StageName 'confirmed' `
        -LastPostcheckPassed $null `
        -InitClosed $false `
        -LastPostcheckSummary $null `
        -ImplementationStagePromoted $false `
        -CurrentDeliveryMode ([string]$selected.delivery_mode) `
        -CurrentStageGoal ([string]$selected.current_stage_goal)

    [pscustomobject]@{
        Stage = 'confirmed'
        SessionRoot = $sessionRoot
        Choice = $Choice
        Message = '用户已确认方案，可以进入 bootstrap。'
    }
}

function Build-ValuesFromSession {
    $answers = Get-AnswerTable
    $decision = Read-JsonFile -Path (Get-SessionPath 'decision.json')
    $decision = Normalize-DecisionRolesForDeliveryMode -Decision $decision
    Write-JsonFile -Path (Get-SessionPath 'decision.json') -Value $decision
    $status = Read-JsonFile -Path (Get-SessionPath 'status.json')

    $normalizedRoles = @(Normalize-ToArray -Value $decision.recommended_roles_now)
    if ($normalizedRoles.Count -eq 0) {
        $normalizedRoles = @(Normalize-ToArray -Value $decision.enabled_roles)
    }
    $normalizedLaterRoles = @(Normalize-ToArray -Value (Get-DecisionPropertyValue -DecisionObject $decision -PropertyName 'available_roles_later' -DefaultValue @()))
    $normalizedLaterRoles = @(
        $normalizedLaterRoles | ForEach-Object { ([string]$_).Trim() } | Where-Object {
            $_ -and ($normalizedRoles -notcontains $_)
        } | Sort-Object -Unique
    )
    Set-ObjectPropertyValue -ObjectValue $decision -PropertyName 'available_roles_later' -PropertyValue @($normalizedLaterRoles)
    Write-JsonFile -Path (Get-SessionPath 'decision.json') -Value $decision
    $normalizedIntegrations = @(Normalize-ToArray -Value $decision.detected_integrations)
    if ($normalizedIntegrations.Count -eq 0) {
        $normalizedIntegrations = @(Normalize-ToArray -Value $decision.integrations)
    }
    $normalizedExternalReferences = @(Normalize-ToArray -Value $decision.external_references)
    $normalizedStageConstraints = @(Normalize-ToArray -Value $decision.stage_constraints)
    $normalizedDeferredCapabilities = @(Normalize-ToArray -Value $decision.deferred_capabilities)
    $normalizedChecklistSeed = @(Normalize-ToArray -Value $decision.implementation_checklist_seed)
    $normalizedAcceptanceSeed = @(Normalize-ToArray -Value $decision.implementation_acceptance_seed)
    $normalizedRequiredCapabilities = @(Get-RequiredCapabilitiesFromDecision -Decision $decision)
    $normalizedCapabilityProbeResults = @(Normalize-ToArray -Value (Get-DecisionPropertyValue -DecisionObject $decision -PropertyName 'capability_probe_results' -DefaultValue @()))
    $normalizedAutodiscoverySignals = @(Normalize-ToArray -Value (Get-DecisionPropertyValue -DecisionObject $decision -PropertyName 'autodiscovery_signals' -DefaultValue @()))
    $normalizedAutodiscoveryAssumptions = @(Normalize-ToArray -Value (Get-DecisionPropertyValue -DecisionObject $decision -PropertyName 'autodiscovery_assumptions' -DefaultValue @()))
    $normalizedDominantWorkstreams = @(Normalize-ToArray -Value (Get-DecisionPropertyValue -DecisionObject $decision -PropertyName 'dominant_workstreams' -DefaultValue @()))
    $workflowStageText = Get-WorkflowStageText -EnabledRoles $normalizedRoles
    $dispatchTriggerLines = ConvertTo-BulletList -Items (Get-DispatchTriggerItems -EnabledRoles $normalizedRoles) -Fallback '- 页面/UI -> frontend'
    $coreGateLines = ConvertTo-BulletList -Items (Get-CoreGateItems -EnabledRoles $normalizedRoles) -Fallback '- 真源文档可读'
    $finalGateLines = ConvertTo-BulletList -Items (Get-FinalGateItems -EnabledRoles $normalizedRoles) -Fallback '- 自动化检查与门禁'
    $blockedWorkflowStages = if ($workflowStageText) { $workflowStageText } else { 'implementation' }
    $implementationSequence = ConvertTo-BulletList -Items (Get-ImplementationSequenceItems -EnabledRoles $normalizedRoles) -Fallback '- implementation'
    $frontendCollaborationTrigger = Get-CollaborationRequirementText -LeadText '改关键页面链路、权限显示、导航时' -EvidenceText '关键验证证据' -EnabledRoles $normalizedRoles
    $backendCollaborationTrigger = Get-CollaborationRequirementText -LeadText '改接口、鉴权、核心业务流时' -EvidenceText '接口与鉴权验证证据' -EnabledRoles $normalizedRoles
    $databaseCollaborationTrigger = Get-CollaborationRequirementText -LeadText '涉及表结构、迁移策略、索引策略变化时' -EvidenceText '迁移验证与回滚证据' -EnabledRoles $normalizedRoles
    $devopsCollaborationTrigger = Get-CollaborationRequirementText -LeadText '涉及流水线、部署配置、环境策略变化时' -EvidenceText '部署验证与回滚证据' -EnabledRoles $normalizedRoles
    $frontendCollaborationNote = Get-CollaborationNotificationText -LeadText '涉及关键页面链路' -EvidenceText '关键验证证据' -EnabledRoles $normalizedRoles
    $backendCollaborationNote = Get-CollaborationNotificationText -LeadText '涉及接口、鉴权、关键业务流' -EvidenceText '接口与鉴权验证证据' -EnabledRoles $normalizedRoles

    $roleLines = if ($normalizedRoles.Count -gt 0) {
        ($normalizedRoles | ForEach-Object { "- $_" }) -join "`n"
    } else {
        '- docs'
    }

    $integrationText = if ($normalizedIntegrations.Count -gt 0) {
        ($normalizedIntegrations | ForEach-Object {
            $displayName = Get-IntegrationDisplayName -Integration $_
            if ($displayName) { $displayName } else { [string]$_ }
        }) -join '、'
    } else {
        '无'
    }

    $projectGoal = if ($answers.ContainsKey('project_goal')) { $answers['project_goal'] } else { '未填写项目目标' }
    $coreFeatures = if ($answers.ContainsKey('core_features')) { $answers['core_features'] } else { '未填写核心功能' }
    $constraints = if ($answers.ContainsKey('constraints')) { $answers['constraints'] } else { '未填写约束' }
    $deliveryMode = if ($decision.PSObject.Properties.Name -contains 'delivery_mode') { [string]$decision.delivery_mode } else { [string]$decision.project_type }
    $targetClient = [string](Get-DecisionPropertyValue -DecisionObject $decision -PropertyName 'target_client' -DefaultValue 'codex')
    $targetClientName = if ($targetClient -eq 'claude-code') { 'Claude Code' } else { 'Codex' }
    $clientEntryFile = if ($targetClient -eq 'claude-code') { 'CLAUDE.md' } else { 'AGENTS.md' }
    $clientCoordinatorPath = if ($targetClient -eq 'claude-code') { '.claude/settings.json' } else { '.codex/COORDINATOR-SUBAGENTS.md' }
    $clientAgentPath = if ($targetClient -eq 'claude-code') { '.claude/agents/*.md' } else { '.codex/agents/*.md' }
    $clientSkillPath = if ($targetClient -eq 'claude-code') { '.claude/skills/required-capabilities.md' } else { '.agents/skills/required-capabilities.md' }
    $currentStage = if ($decision.PSObject.Properties.Name -contains 'current_stage' -and [string]$decision.current_stage) { [string]$decision.current_stage } else { 'implementation-v1' }
    $currentStageGoal = if ($decision.PSObject.Properties.Name -contains 'current_stage_goal' -and [string]$decision.current_stage_goal) { [string]$decision.current_stage_goal } else { "完成面向 $targetClientName 的初始化协作包收口，并准备首轮实施接手" }
    $externalReferenceLines = if ($normalizedExternalReferences.Count -gt 0) {
        ($normalizedExternalReferences | ForEach-Object {
            $purpose = if ($_.PSObject.Properties.Match('purpose').Count -gt 0) { [string]$_.purpose } else { '外部参考' }
            $type = if ($_.PSObject.Properties.Match('type').Count -gt 0) { [string]$_.type } else { 'reference' }
            $mustRead = if ($_.PSObject.Properties.Match('must_read').Count -gt 0 -and [bool]$_.must_read) { 'must-read' } else { 'optional' }
            "- [$type/$mustRead] $($_.path) | $purpose"
        }) -join "`n"
    } else {
        '- 本轮未引入外部参考源；后续如新增参考材料，需要先写入真源。'
    }
    $stageConstraintLines = if ($normalizedStageConstraints.Count -gt 0) {
        ($normalizedStageConstraints | ForEach-Object { "- $_" }) -join "`n"
    } else {
        "- 当前阶段只生成面向 $targetClientName 的初始化协作包，不直接展开业务实现。`n- 后续实施必须从 $clientEntryFile 和 docs/ 真源重新接手。`n- 首轮范围围绕 $coreFeatures 建立可验证边界。"
    }
    $deferredCapabilityLines = if ($normalizedDeferredCapabilities.Count -gt 0) {
        ($normalizedDeferredCapabilities | ForEach-Object { "- $_" }) -join "`n"
    } else {
        '- 本轮未确认的能力不默认启用；需要后续主控重新评估后再写入真源。'
    }
    $laterRoleLines = if ($normalizedLaterRoles.Count -gt 0) {
        ($normalizedLaterRoles | ForEach-Object { "- $_" }) -join "`n"
    } else {
        '- 当前先不追加其他后续角色；如实施范围扩大，再由主控重新评估。'
    }
    $implementationChecklistLines = ConvertTo-BulletList -Items (Get-ImplementationHandoffChecklistItems -ChecklistSeed $normalizedChecklistSeed -ClientEntryFile $clientEntryFile -WorkflowStageText $workflowStageText) -Fallback "- 是否进入 $workflowStageText"
    $implementationAcceptanceLines = if ($normalizedAcceptanceSeed.Count -gt 0) {
        ($normalizedAcceptanceSeed | ForEach-Object { "- $_" }) -join "`n"
    } else {
        ConvertTo-BulletList -Items (Get-ImplementationAcceptanceSeed -DeliveryMode $deliveryMode -ExternalReferences $normalizedExternalReferences -EnabledRoles $normalizedRoles) -Fallback '- 初始化协作包已通过能力门禁、postcheck 与语义验收记录检查'
    }
    $closureChecklistLines = "- [ ] postcheck 已通过`n- [ ] 已提示用户在 $targetClientName 中新开会话或重启会话`n- [ ] 已提示当前线程不继续业务实现`n- [ ] 已说明不得删除 docs/$clientCoordinatorPath/$clientEntryFile/.commonhe/session"
    $safeRetainedPaths = "- docs/`n- .commonhe/session/`n- $clientEntryFile`n- $clientCoordinatorPath"
    $closureSummary = "初始化协作包已生成并通过 postcheck。请在 $targetClientName 中新开会话或重启会话，从 $clientEntryFile 和 docs/ 真源重新接手；当前初始化线程到此收口，不继续业务实现。"
    $postcheckStatus = '`postcheck` 已通过'
    $milestoneOverview = 'M1 初始化完成、M2 当前阶段业务交付、M3 验收与上线/归档'
    $milestoneDetails = "- M1 初始化完成`n- M2 当前阶段业务交付`n- M3 验收与上线/归档"
    $requiredCapabilitiesList = ConvertTo-RequiredCapabilityListText -Capabilities $normalizedRequiredCapabilities
    $capabilityProbeSummary = ConvertTo-CapabilityProbeSummaryText -ProbeResults $normalizedCapabilityProbeResults
    $capabilityScopeNotes = ConvertTo-CapabilityScopeNotesText -Capabilities $normalizedRequiredCapabilities
    $doctorPassed = [bool](Get-StatusPropertyValue -StatusObject $status -PropertyName 'doctor_passed' -DefaultValue $false)
    $doctorFailed = [bool](Get-StatusPropertyValue -StatusObject $status -PropertyName 'doctor_failed' -DefaultValue $false)
    $precheckPassed = [bool](Get-StatusPropertyValue -StatusObject $status -PropertyName 'precheck_passed' -DefaultValue $false)
    $precheckFailed = [bool](Get-StatusPropertyValue -StatusObject $status -PropertyName 'precheck_failed' -DefaultValue $false)
    $capabilityGatePassed = Get-StatusPropertyValue -StatusObject $status -PropertyName 'capability_gate_passed' -DefaultValue $null
    $capabilityGateStatus = if ([bool]$capabilityGatePassed) {
        '- 绿色：能力探测与 precheck 已通过'
    } elseif (($null -eq $capabilityGatePassed) -and $doctorPassed -and $precheckPassed) {
        '- 绿色：doctor 与 precheck 已通过'
    } elseif ($doctorFailed) {
        '- 红色：doctor 未通过，当前只能先修复环境或入口'
    } elseif ($precheckFailed) {
        '- 红色：precheck 未通过，当前只能修复依赖'
    } elseif ($doctorPassed) {
        '- 黄色：doctor 已通过，尚未记录 precheck 结果'
    } elseif ($precheckPassed) {
        '- 黄色：precheck 已通过，尚未记录 doctor 结果'
    } else {
        '- 黄色：当前尚未记录 doctor / precheck 结果'
    }
    $capabilityGateIsGreen = ([string]$capabilityGateStatus).StartsWith('- 绿色')
    $capabilityRemediationSection = if ($capabilityGateIsGreen) {
        ''
    } else {
        @"
## 0. 临时补救命令（能力门禁未通过时执行）

这是星星的vibecoding启动器的异常补救说明：只有能力门禁不是绿色时才执行。接手本初始化协作包的新 Agent 必须先完成以下步骤。

1. 检测本地是否存在 superpowers、agent-browser、chrome-devtools、GitNexus、Speckit。
2. 如果任一内容缺失，直接安装或配置；这些是 agent 工作流程必须用到的 Skill / MCP / CLI 能力。
3. 安装或配置完成后，提示需要重开服务或重启会话才能加载，然后重新执行能力检测。
"@
    }
    $autodiscoverySignalSummary = ConvertTo-BulletList -Items $normalizedAutodiscoverySignals -Fallback '- 主 Agent 已完成当前收口，本轮未保留额外自动分析信号。'
    $autodiscoveryAssumptions = ConvertTo-BulletList -Items $normalizedAutodiscoveryAssumptions -Fallback '- 当前没有额外未验证假设；后续范围变化需重新回写真源。'
    $implementationOwnerRole = if ($normalizedRoles -contains 'architect') {
        'architect'
    } elseif ($normalizedDominantWorkstreams -contains 'frontend' -and $normalizedRoles -contains 'frontend') {
        'frontend'
    } elseif ($normalizedDominantWorkstreams -contains 'backend' -and $normalizedRoles -contains 'backend') {
        'backend'
    } elseif ($normalizedRoles.Count -gt 0) {
        [string]$normalizedRoles[0]
    } elseif ([bool](Get-StatusPropertyValue -StatusObject $status -PropertyName 'precheck_failed' -DefaultValue $false)) {
        'docs'
    } else {
        'docs'
    }
    $implementationSupportRoles = @(
        foreach ($role in $normalizedRoles) {
            if ([string]$role -ne $implementationOwnerRole) { [string]$role }
        }
    ) | Select-Object -First 4
    $implementationSupportRoleList = ConvertTo-BulletList -Items $implementationSupportRoles -Fallback '- 当前无额外 support roles'
    $dominantWorkstreamSummary = ConvertTo-BulletList -Items $normalizedDominantWorkstreams -Fallback '- 当前按单一工作流推进'
    $kickoffRequiredReads = ConvertTo-BulletList -Items @(
        'docs/project_context.md'
        'docs/roadmap/01-实施路线图.md'
        'docs/workflow/implementation-kickoff.md'
        'docs/workflow/first-sprint-contract.md'
        'docs/workflow/first-task-pack.md'
    )
    $primaryDeliverableLabel = if ($normalizedDominantWorkstreams -contains 'frontend') {
        '首个页面/交互可交付'
    } elseif ($normalizedDominantWorkstreams -contains 'backend') {
        '首个接口/服务可交付'
    } else {
        '首个实施阶段可交付'
    }
    $firstSprintRiskGate = if (($normalizedRoles -contains 'architect') -or (($normalizedRoles -contains 'frontend') -and ($normalizedRoles -contains 'backend'))) {
        'medium'
    } else {
        'low'
    }
    $firstSprintEvaluatorRole = if ($normalizedRoles -contains 'reviewer') { 'reviewer' } elseif ($normalizedRoles -contains 'qa') { 'qa' } else { 'docs' }
    $validationOwnerRole = if ($normalizedRoles -contains 'reviewer') { 'reviewer' } elseif ($normalizedRoles -contains 'qa') { 'qa' } else { 'docs' }
    $validationTitle = if (($normalizedRoles -contains 'reviewer') -and ($normalizedRoles -contains 'qa')) {
        '完成首轮 review 与 qa 证据补齐'
    } elseif ($normalizedRoles -contains 'reviewer') {
        '完成首轮 review 证据补齐'
    } elseif ($normalizedRoles -contains 'qa') {
        '完成首轮 qa 证据补齐'
    } else {
        '完成首轮验证证据补齐'
    }
    $agentAuthoredDomainModules = @(Normalize-ToArray -Value (Get-DecisionPropertyValue -DecisionObject $decision -PropertyName 'domain_modules' -DefaultValue @()))
    $domainModules = if ($agentAuthoredDomainModules.Count -gt 0) { $agentAuthoredDomainModules } else { @(Get-DomainModuleItems -Answers $answers -DeliveryMode $deliveryMode) }
    $domainAcceptance = @(Get-DomainAcceptanceItems -Answers $answers -DeliveryMode $deliveryMode)
    $domainAcceptance = @($domainAcceptance + $normalizedAcceptanceSeed) | Select-Object -Unique
    $domainWorkflowItems = @(Get-DomainWorkflowItems -Answers $answers -EnabledRoles $normalizedRoles -DeliveryMode $deliveryMode)
    $domainModules = @($domainModules)
    $domainAcceptance = @($domainAcceptance)
    $domainWorkflowItems = @($domainWorkflowItems)
    $domainModuleLines = ConvertTo-BulletList -Items $domainModules -Fallback "- 围绕 $coreFeatures 建立首轮业务模块"
    $domainAcceptanceLines = ConvertTo-BulletList -Items $domainAcceptance -Fallback $implementationAcceptanceLines
    $packageAcceptanceLines = ConvertTo-BulletList -Items @(
        '初始化协作包结构、目标软件入口和 session 审计产物已通过 postcheck'
        'decision.json、meng-xingxing-output.json 与 final-acceptance.json 的选中方案一致'
        'capability_gate_passed 为 true，五个必选能力均记录为 selected=true'
        '当前验收只确认协作包初始化完成，不声称业务代码、业务功能、评审结论或质量验收结论已经完成'
    )
    $domainWorkflowSection = if ($domainWorkflowItems.Count -gt 0) {
        "### 主 Agent 业务协作工作流`n`n$(ConvertTo-BulletList -Items $domainWorkflowItems)"
    } else {
        ''
    }
    $confirmedFeatureTerms = @(Get-ConfirmedFeatureTerms -Answers $answers)
    $confirmedFeatureText = if ($confirmedFeatureTerms.Count -gt 0) { [string]::Join('、', $confirmedFeatureTerms) } else { $coreFeatures }
    $domainDataModelLines = if ($domainModules.Count -gt 0) {
        "- 核心对象：围绕 $confirmedFeatureText 提炼实体、状态、归属关系和权限边界`n- 跨端状态：记录来源端、操作人、更新时间和可追溯证据`n- 验收数据：每个关键路径至少保留可复现的输入、状态变化和输出结果"
    } else {
        "围绕 $coreFeatures 设计最小数据结构，并保证关键字段可验证"
    }
    $domainApiLines = if ($domainModules.Count -gt 0) {
        if ($deliveryMode -eq 'web-miniapp') {
            "- Web 管理后台：覆盖 $confirmedFeatureText 的后台端管理接口`n- 小程序端：覆盖 $confirmedFeatureText 的移动端查询、提交和状态同步接口`n- 统一鉴权：按角色、端侧和数据范围控制访问`n- 状态同步：Web 与小程序对关键状态保持一致"
        } else {
            "- 业务接口：覆盖 $confirmedFeatureText 的创建、查询、更新、状态流转与异常反馈`n- 鉴权边界：按角色和数据范围控制访问`n- 验证接口：保留关键路径可复现证据"
        }
    } else {
        '接口以清晰、稳定、可验证为目标'
    }
    $firstTaskPackItems = @(
        "### task_1`n- title: 锁定首轮实施范围与真源`n- owner_role: $implementationOwnerRole`n- support_roles: docs, reviewer`n- depends_on: docs/project_context.md, docs/roadmap/01-实施路线图.md`n- done_signal: 当前范围、验收口径、外部参考已在真源中收口，尚未声称业务实现完成"
        "### task_2`n- title: 准备 $primaryDeliverableLabel 的实施合同与接口边界`n- owner_role: $implementationOwnerRole`n- support_roles: $([string]::Join(', ', @($implementationSupportRoles)))`n- depends_on: docs/workflow/implementation-kickoff.md, docs/workflow/first-sprint-contract.md`n- done_signal: 当前第一优先工作流的实施合同、接口边界和验证计划已成文，等待后续实施"
        "### task_3`n- title: 定义 $validationTitle`n- owner_role: $validationOwnerRole`n- support_roles: $implementationOwnerRole`n- depends_on: docs/workflow/acceptance-gates.md, docs/workflow/grading-criteria.md`n- done_signal: 关键风险、证据采集方式与下一轮建议已经成文，等待后续实施后补证据"
    ) -join "`n`n"
    $domainTaskItems = @(Get-DomainTaskItems -Answers $answers -ImplementationOwnerRole $implementationOwnerRole -SupportRoles $implementationSupportRoles -ValidationOwnerRole $validationOwnerRole)
    if ($domainTaskItems.Count -gt 0) {
        $firstTaskPackItems = $domainTaskItems -join "`n`n"
    }
    $firstTaskPackGateNote = if (($normalizedRoles -contains 'reviewer') -and ($normalizedRoles -contains 'qa')) {
        '进入 review / qa 前先补齐可验证证据'
    } elseif ($normalizedRoles -contains 'reviewer') {
        '进入 review 前先补齐可验证证据'
    } elseif ($normalizedRoles -contains 'qa') {
        '进入 qa 前先补齐可验证证据'
    } else {
        '进入验收前先补齐可验证证据'
    }
    $projectGoalSummary = [string](Get-DecisionPropertyValue -DecisionObject $decision -PropertyName 'project_goal_summary' -DefaultValue '')
    $targetUsersSummary = [string](Get-DecisionPropertyValue -DecisionObject $decision -PropertyName 'target_users_summary' -DefaultValue '')
    $coreFeaturesSummary = [string](Get-DecisionPropertyValue -DecisionObject $decision -PropertyName 'core_features_summary' -DefaultValue '')
    $constraintsSummary = [string](Get-DecisionPropertyValue -DecisionObject $decision -PropertyName 'constraints_summary' -DefaultValue '')
    $selectedCapabilities = @(Normalize-ToArray -Value (Get-DecisionPropertyValue -DecisionObject $decision -PropertyName 'selected_capabilities' -DefaultValue @()))
    if ($selectedCapabilities.Count -eq 0) {
        $selectedCapabilities = @(Get-MandatorySelectedCapabilityRecords -RequiredCapabilities $normalizedRequiredCapabilities -ProbeResults $normalizedCapabilityProbeResults)
        Set-ObjectPropertyValue -ObjectValue $decision -PropertyName 'selected_capabilities' -PropertyValue @($selectedCapabilities)
        Write-JsonFile -Path (Get-SessionPath 'decision.json') -Value $decision
    }

    if (-not $projectGoalSummary) {
        $projectGoalSummary = if ($answers.ContainsKey('project_goal')) { $answers['project_goal'] } else { '未填写项目目标' }
    }
    if (-not $targetUsersSummary) {
        $targetUsersSummary = if ($answers.ContainsKey('target_users')) { $answers['target_users'] } else { '未填写目标用户' }
    }
    if (-not $coreFeaturesSummary) {
        $coreFeaturesSummary = if ($answers.ContainsKey('core_features')) { $answers['core_features'] } else { '未填写核心功能' }
    }
    if (-not $constraintsSummary) {
        $constraintsSummary = if ($answers.ContainsKey('constraints')) { $answers['constraints'] } else { '未填写约束' }
    }
    $confirmedProjectName = [string]$decision.project_name
    $selectedCapabilitiesSummary = ConvertTo-CapabilitySelectionSummaryText -SelectedCapabilities $selectedCapabilities
    $solutionArchitectureSummary = [string](Get-DecisionPropertyValue -DecisionObject $decision -PropertyName 'solution_architecture_summary' -DefaultValue '')
    $selectedSolutionIdText = [string](Get-DecisionPropertyValue -DecisionObject $decision -PropertyName 'selected_solution_id' -DefaultValue $decision.solution_mode)
    $selectedSolutionTitleText = [string](Get-DecisionPropertyValue -DecisionObject $decision -PropertyName 'selected_solution_title' -DefaultValue $decision.solution_mode)
    $channelScopeLines = if ($deliveryMode -eq 'web-miniapp') {
        "`n- Web 管理后台与微信小程序端的职责边界记录为后续实施输入，不作为当前初始化验收完成项"
    } else {
        ''
    }
    $packageIntro = "这是主 Agent 为 ``$confirmedProjectName`` 收口的初始化协作包：它负责沉淀方案、目标软件入口、能力门禁和第一轮接手方式，不代表业务系统已经实现。"
    $handoffSummary = "主 Agent 已完成方案与目标软件收口；后续请在 $targetClientName 中从 ``$clientEntryFile`` 重新接手，并以 docs/ 与 .commonhe/session/ 作为追溯真源。"

    $values = @{
        project_name = [string]$decision.project_name
        package_intro = $packageIntro
        handoff_summary = $handoffSummary
        target_client = $targetClient
        target_client_name = $targetClientName
        client_entry_file = $clientEntryFile
        client_coordinator_path = $clientCoordinatorPath
        client_agent_path = $clientAgentPath
        client_skill_path = $clientSkillPath
        selected_capabilities = @($selectedCapabilities)
        selected_capabilities_summary = $selectedCapabilitiesSummary
        project_type = [string]$decision.project_type
        delivery_mode = $deliveryMode
        current_phase = $currentStage
        core_goal = $projectGoalSummary
        agent_dispatch_matrix = $roleLines
        roles_and_manuals = $roleLines
        dispatch_triggers = $dispatchTriggerLines
        domain_workflow_section = $domainWorkflowSection
        current_goals = "- 将用户确认的方案收口为面向 $targetClientName 的初始化协作包`n- 沉淀 $projectGoalSummary`n- 为首轮实施准备清晰的真源、角色、能力门禁与接手入口"
        in_scope_items = "- 初始化协作包真源文档`n- $targetClientName 原生入口与角色调度文件`n- 已选 Skill / MCP / CLI 能力记录`n- 围绕 $coreFeaturesSummary 的首轮实施准备$channelScopeLines"
        out_of_scope_items = "- 本轮不生成业务项目成品、业务代码或业务脚手架`n- 未经用户确认的扩展能力不默认启用`n- 第三方系统对接、复杂报表和长期平台化能力留到后续实施阶段"
        out_of_scope = "- 本轮不生成业务项目成品、业务代码或业务脚手架`n- 未经用户确认的扩展能力不默认启用`n- 第三方系统对接、复杂报表和长期平台化能力留到后续实施阶段"
        enabled_roles = $roleLines
        recommended_roles_now = $roleLines
        available_roles_later = $laterRoleLines
        external_references = $externalReferenceLines
        stage_constraints = $stageConstraintLines
        deferred_capabilities = $deferredCapabilityLines
        acceptance_criteria = $packageAcceptanceLines
        core_gate_items = $coreGateLines
        final_gate_items = $finalGateLines
        blocked_workflow_stages = $blockedWorkflowStages
        implementation_checklist_items = $implementationChecklistLines
        closure_checklist_items = $closureChecklistLines
        closure_summary = $closureSummary
        safe_retained_paths = $safeRetainedPaths
        postcheck_status = $postcheckStatus
        current_phase_tasks = "- 读取本初始化协作包的真源入口`n- 根据 $clientEntryFile 启动 $targetClientName 新会话`n- 按 docs/workflow/first-task-pack.md 推进首轮实施"
        risks_and_constraints = "- $constraintsSummary"
        completed_items = "- 已完成方案选择、项目名确认、目标软件确认与能力选择记录`n- 接手包真源、目标入口与审计记录已准备完成，业务实施尚未开始"
        must_confirm_items = "- [ ] 待用户确认：是否以当前真源作为 $targetClientName 新会话的唯一接手依据"
        must_provide_items = if ($normalizedExternalReferences.Count -gt 0) { "- 当前无新增必须提供项；已有外部参考已写入真源。" } else { "- 若后续要加入外部风格、内容或接口参考，请提供稳定路径并回写真源。" }
        deferred_decisions = $deferredCapabilityLines
        blocking_points = "- 未确认首轮范围、责任角色与验证证据前，不得签收业务实现完成。"
        architecture_version = 'v0.1.0'
        architecture_date = (Get-Date -Format 'yyyy-MM-dd')
        architecture_owner = '星星的vibecoding启动器初始化编排器'
        architecture_status = 'draft'
        product_positioning = "面向 $projectGoalSummary 的初始化协作包"
        target_users = $targetUsersSummary
        core_value = $projectGoalSummary
        current_scope = $coreFeaturesSummary
        architecture_overview = "当前按方案 $selectedSolutionIdText（$selectedSolutionTitleText）初始化，先把 $projectGoalSummary 收口为可被 $targetClientName 接手的协作包，而不是提前生成业务工程。"
        tech_stack = if ($solutionArchitectureSummary) {
            "当前选定方案的架构建议：$solutionArchitectureSummary`n`n本轮仍只生成初始化协作包，不生成业务代码；该技术栈作为后续实施新会话的起点。"
        } else {
            "当前识别项目类型为 $($decision.project_type)；本轮只沉淀技术判断与首轮实施约束，具体栈由后续业务实施再落定。"
        }
        service_or_module_design = $domainModuleLines
        data_model_principles = $domainDataModelLines
        api_contract_principles = $domainApiLines
        security_and_compliance = "按当前项目角色和关键数据操作划分访问边界；首轮至少记录高风险操作、数据变更和异常处理证据。"
        deployment_and_ops = '优先确保当前阶段可落地'
        architecture_tradeoffs = "当前方案优先考虑 $constraintsSummary"
        current_architecture_conclusion = "已按 $($decision.solution_mode) 方案完成初始化协作包收口，后续进入 $targetClientName 新会话接手。"
        roadmap_version = 'v0.1.0'
        roadmap_date = (Get-Date -Format 'yyyy-MM-dd')
        roadmap_owner = '星星的vibecoding启动器初始化编排器'
        roadmap_status = 'draft'
        project_goal = $projectGoalSummary
        current_phase_goal = "完成初始化协作包收口；后续实施输入覆盖：$coreFeaturesSummary"
        success_criteria = $packageAcceptanceLines
        milestone_overview = "M1 初始化协作包收口、M2 首轮实施接手、M3 验证与归档"
        milestone_details = "- M1: 当前已完成面向 $targetClientName 的初始化协作包生成与 postcheck`n- M2: 新会话读取真源并推进首轮实施`n- M3: 按后续业务验收门禁补齐证据并归档"
        role_responsibilities = "- 主控负责调度与验收`n- 执行代理负责实现、文档、验证与复核"
        deliverables = "- $clientEntryFile 与目标软件配置`n- docs/ 真源文档`n- .commonhe/session/ 可追溯状态`n- 角色与能力门禁说明"
        project_goal_summary = $projectGoalSummary
        risks_and_mitigations = "- 范围漂移：由主控控制`n- 需求变化：通过真源收口"
        execution_sequence = $implementationSequence
        required_capabilities_list = $requiredCapabilitiesList
        capability_probe_summary = $capabilityProbeSummary
        capability_scope_notes = $capabilityScopeNotes
        capability_gate_status = $capabilityGateStatus
        capability_remediation_section = $capabilityRemediationSection
        implementation_owner_role = $implementationOwnerRole
        implementation_support_roles = $implementationSupportRoleList
        dominant_workstreams_summary = $dominantWorkstreamSummary
        kickoff_required_reads = $kickoffRequiredReads
        first_task_pack_items = $firstTaskPackItems
        first_task_pack_gate_note = $firstTaskPackGateNote
        task_id = 'first-sprint'
        implementer_role = $implementationOwnerRole
        evaluator_role = $firstSprintEvaluatorRole
        risk_gate = $firstSprintRiskGate
        requirement_summary = "- 当前阶段目标：$currentStageGoal`n- 第一优先工作流：$primaryDeliverableLabel"
        first_priority_workflow = "围绕 $confirmedFeatureText 准备 $primaryDeliverableLabel 的首轮实施边界、接口/页面契约和验证计划；第一轮只按真源推进，不提前签收业务实现。"
        first_sprint_deliverables = "- 首轮实施范围与责任角色确认记录`n- $primaryDeliverableLabel 的接口、页面或流程边界说明`n- 面向 reviewer / qa 的验证证据采集计划`n- 需要回写 docs/ 真源的范围变更记录"
        criteria_1 = "完成首轮实施合同、接口边界和验证计划收口，不声称业务代码已完成"
        verify_method_1 = '按 docs/workflow/acceptance-gates.md 确认初始化协作包门禁，并在后续实施后补齐证据'
        deliverable_ref_1 = 'docs/workflow/implementation-kickoff.md'
        additional_dimensions = if ($normalizedRoles -contains 'qa') { '| 回归验证 | >= 6 | 中 |' } else { '| 用户可见行为证据 | >= 6 | 中 |' }
        in_scope = $coreFeaturesSummary
        known_risks = "- 范围漂移：通过 kickoff pack 与真源收口`n- 证据不足：由 reviewer / qa 补齐验证"
        test_strategy = if ($normalizedRoles -contains 'qa') { '- 自动化测试 + agent-browser / chrome-devtools 运行态验证' } else { '- 关键验证命令 + 用户可见行为证据' }
        evidence_plan = "- 每个首轮任务完成时记录命令输出、截图或日志路径`n- reviewer 复核范围漂移与角色交接证据`n- qa 只在有可执行实现后补齐回归证据"
        frontend_collaboration_trigger = "- $frontendCollaborationTrigger"
        backend_collaboration_trigger = "- $backendCollaborationTrigger"
        database_collaboration_trigger = "- $databaseCollaborationTrigger"
        devops_collaboration_trigger = "- $devopsCollaborationTrigger"
        frontend_collaboration_note = "- $frontendCollaborationNote"
        backend_collaboration_note = "- $backendCollaborationNote"
        autodiscovery_assumptions = $autodiscoveryAssumptions
        autodiscovery_signal_summary = $autodiscoverySignalSummary
    }

    $valuesPath = Get-SessionPath 'generated-values.json'
    Write-JsonFile -Path $valuesPath -Value $values
    $valuesPath
}

function ConvertTo-FailureValues {
    param(
        [string]$ValuesPath,
        [object]$PostcheckSummary
    )

    if (-not (Test-Path $ValuesPath)) {
        throw "Values file not found for failure conversion: $ValuesPath"
    }

    $values = Get-Content -Raw $ValuesPath | ConvertFrom-Json
    $failureItems = @(
        '初始化文件已生成'
        'postcheck 未通过'
        '当前不得宣布初始化成功'
    )
    $repairItems = @(
        '[ ] 当前不得宣布初始化成功'
        '[ ] 请先修复缺失项、清理冗余角色文件，并移除未启用角色引用'
        '[ ] 修复后重新执行 postcheck'
        '[ ] 在 postcheck 通过前不要新开业务实现线程'
    )
    $missingAndUnexpected = @(
        foreach ($fieldName in @('MissingCoreFiles', 'MissingAgentFiles', 'MissingHandbooks', 'UnexpectedAgentFiles', 'UnexpectedHandbooks', 'MissingCapabilityDeclarations', 'DanglingRoleReferences', 'BrokenWorkflowReferences', 'InvalidWorkflowContent', 'AuthorshipQualityIssues', 'TruthSourceGateIssues', 'MissingCapabilityEvidence', 'FailedCapabilityProbes')) {
            foreach ($item in (Normalize-ToArray -Value $PostcheckSummary.$fieldName)) {
                "$fieldName -> $item"
            }
        }
    )
    if ($missingAndUnexpected.Count -gt 0) {
        foreach ($item in $missingAndUnexpected) {
            $repairItems += $item
        }
    }

    Set-ObjectPropertyValue -ObjectValue $values -PropertyName 'completed_items' -PropertyValue '- 当前阶段尚未进入实施'
    Set-ObjectPropertyValue -ObjectValue $values -PropertyName 'postcheck_status' -PropertyValue '`postcheck` 未通过'
    Set-ObjectPropertyValue -ObjectValue $values -PropertyName 'closure_summary' -PropertyValue '初始化协作包已生成，但 postcheck 未通过；当前线程不得进入业务实现，也不得宣布初始化成功。'
    Set-ObjectPropertyValue -ObjectValue $values -PropertyName 'closure_checklist_items' -PropertyValue (($repairItems | ForEach-Object { "- $_" }) -join "`n")
    Set-ObjectPropertyValue -ObjectValue $values -PropertyName 'blocking_points' -PropertyValue '- postcheck 未通过会阻断初始化成功收口'
    Set-ObjectPropertyValue -ObjectValue $values -PropertyName 'capability_gate_status' -PropertyValue '- 红色：capability gate 未通过'

    $failureValuesPath = Get-SessionPath 'generated-values.failure.json'
    Write-JsonFile -Path $failureValuesPath -Value $values
    $failureValuesPath
}

function Rewrite-FailureTruthDocs {
    param(
        [string]$TargetRoot,
        [string]$ValuesPath,
        [object]$PostcheckSummary
    )

    $failureValuesPath = ConvertTo-FailureValues -ValuesPath $ValuesPath -PostcheckSummary $PostcheckSummary
    & $initScript -TargetRoot $TargetRoot -ValuesPath $failureValuesPath -DecisionPath (Get-SessionPath 'decision.json') -Execute -Force | Out-Null
    $failureValuesPath
}

function Invoke-TruthSourceGate {
    param(
        [string]$TargetRoot,
        [string]$TargetClient
    )

    $gateScript = Join-Path $scriptDir 'assert-commonhe-truth-source.ps1'
    if (-not (Test-Path -LiteralPath $gateScript -PathType Leaf)) {
        return [pscustomobject]@{
            Passed = $false
            Issues = @("truth-source gate script missing: $gateScript")
        }
    }

    try {
        $output = @(& $gateScript -GeneratedRoot $TargetRoot -TargetClient $TargetClient -AsJson 2>&1)
        $lastExitCodeVariable = Get-Variable -Name LASTEXITCODE -ErrorAction SilentlyContinue
        $exitCode = if ($null -ne $lastExitCodeVariable) { [int]$lastExitCodeVariable.Value } else { 0 }
    } catch {
        return [pscustomobject]@{
            Passed = $false
            Issues = @("truth-source gate execution failed: $($_.Exception.Message)")
        }
    }
    $rawOutput = ($output | ForEach-Object { [string]$_ }) -join "`n"

    try {
        $parsed = $rawOutput | ConvertFrom-Json
        return [pscustomobject]@{
            Passed = [bool]$parsed.Passed
            Issues = @(Normalize-ToArray -Value $parsed.Issues)
        }
    } catch {
        return [pscustomobject]@{
            Passed = $false
            Issues = @("truth-source gate did not return valid JSON; exit=$exitCode; output=$rawOutput")
        }
    }
}

function Invoke-BootstrapStage {
    param(
        [string]$TargetRoot,
        [string]$ValuesPath,
        [switch]$Execute,
        [switch]$Force
    )

    if (-not $TargetRoot) { throw "TargetRoot is required for bootstrap stage." }

    $decision = Read-JsonFile -Path (Get-SessionPath 'decision.json')
    $isConfirmed = $false
    if ($decision.PSObject.Properties.Name -contains 'user_confirmed') {
        $isConfirmed = [bool]$decision.user_confirmed
    }
    if (-not $isConfirmed) {
        throw "Cannot bootstrap before the user has confirmed the proposal. Set 'user_confirmed' to true in session decision.json."
    }

    $targetRoot = [System.IO.Path]::GetFullPath($TargetRoot)
    $statusBeforeBootstrap = Read-JsonFile -Path (Get-SessionPath 'status.json')
    $precheckAlreadyPassed = [bool](Get-StatusPropertyValue -StatusObject $statusBeforeBootstrap -PropertyName 'precheck_passed' -DefaultValue $false)
    if (-not $precheckAlreadyPassed) {
        $precheckSummary = Invoke-Precheck -ProjectRoot $targetRoot -PersistStatus
        if (-not [bool]$precheckSummary.passed) {
            return [pscustomobject]@{
                Stage = 'precheck_failed'
                SessionRoot = $sessionRoot
                TargetRoot = $targetRoot
                Precheck = $precheckSummary
                Message = Get-PrecheckFailureMessage -PrecheckSummary $precheckSummary
            }
        }
    }

    $effectiveValuesPath = if ($ValuesPath) { $ValuesPath } else { Build-ValuesFromSession }
    $generationResult = @(& $initScript -TargetRoot $targetRoot -ValuesPath $effectiveValuesPath -DecisionPath (Get-SessionPath 'decision.json') -Execute:$Execute -Force:$Force)

    if (-not $Execute) {
        return [pscustomobject]@{
            Stage = 'bootstrap_preview'
            SessionRoot = $sessionRoot
            TargetRoot = $targetRoot
            GeneratedFiles = @($generationResult | ForEach-Object { [string]$_.Target })
            Results = $generationResult
            Message = 'Bootstrap 干跑已生成文件清单；正式执行后才会进入 postcheck 与初始化收口。'
        }
    }

    Set-Status -StageName 'bootstrapped' -LastPostcheckPassed $null -InitClosed $false
    $postcheckSummary = Invoke-PostBootstrapCheck -TargetRoot $targetRoot -PersistStatus
    $handoffPath = Write-BootstrapHandoff -TargetRoot $targetRoot -PostcheckSummary $postcheckSummary -GenerationResults $generationResult

    if ([bool]$postcheckSummary.Passed) {
        [pscustomobject]@{
            Stage = 'implementation_ready'
            SessionRoot = $sessionRoot
            TargetRoot = $targetRoot
            GeneratedFiles = @($generationResult | ForEach-Object { [string]$_.Target })
            Postcheck = $postcheckSummary
            HandoffPath = $handoffPath
            Message = Get-SuccessClosureMessage -TargetClient ([string](Get-DecisionPropertyValue -DecisionObject $decision -PropertyName 'target_client' -DefaultValue 'codex'))
        }
    } else {
        Rewrite-FailureTruthDocs -TargetRoot $targetRoot -ValuesPath $effectiveValuesPath -PostcheckSummary $postcheckSummary | Out-Null
        [pscustomobject]@{
            Stage = 'postcheck_failed'
            SessionRoot = $sessionRoot
            TargetRoot = $targetRoot
            GeneratedFiles = @($generationResult | ForEach-Object { [string]$_.Target })
            Postcheck = $postcheckSummary
            HandoffPath = $handoffPath
            Message = Get-PostcheckFailureMessage
        }
    }
}

function Invoke-PostcheckStage {
    param([string]$TargetRoot)

    $targetRoot = [System.IO.Path]::GetFullPath($TargetRoot)
    $postcheckSummary = Invoke-PostBootstrapCheck -TargetRoot $targetRoot -PersistStatus
    $handoffPath = Write-BootstrapHandoff -TargetRoot $targetRoot -PostcheckSummary $postcheckSummary

    if ([bool]$postcheckSummary.Passed) {
        [pscustomobject]@{
            Stage = 'implementation_ready'
            SessionRoot = $sessionRoot
            TargetRoot = $targetRoot
            Postcheck = $postcheckSummary
            HandoffPath = $handoffPath
            Message = Get-SuccessClosureMessage
        }
    } else {
        [pscustomobject]@{
            Stage = 'postcheck_failed'
            SessionRoot = $sessionRoot
            TargetRoot = $targetRoot
            Postcheck = $postcheckSummary
            HandoffPath = $handoffPath
            Message = Get-PostcheckFailureMessage
        }
    }
}

function Show-Status {
    $status = Read-JsonFile -Path (Get-SessionPath 'status.json')
    [pscustomobject]@{
        Stage = [string]$status.stage
        SessionRoot = [string]$status.session_root
        StartedAt = [string]$status.started_at
        LastPostcheckPassed = $status.last_postcheck_passed
        InitClosed = [bool]$status.init_closed
        LastPostcheckSummary = $status.last_postcheck_summary
        ImplementationStagePromoted = [bool]$status.implementation_stage_promoted
        CurrentDeliveryMode = [string]$status.current_delivery_mode
        CurrentStageGoal = [string]$status.current_stage_goal
        DoctorPassed = [bool](Get-StatusPropertyValue -StatusObject $status -PropertyName 'doctor_passed' -DefaultValue $false)
        DoctorFailed = [bool](Get-StatusPropertyValue -StatusObject $status -PropertyName 'doctor_failed' -DefaultValue $false)
        EntrycheckPassed = [bool](Get-StatusPropertyValue -StatusObject $status -PropertyName 'entrycheck_passed' -DefaultValue $false)
        EntrycheckFailed = [bool](Get-StatusPropertyValue -StatusObject $status -PropertyName 'entrycheck_failed' -DefaultValue $false)
        MisplacedPackageDetected = [bool](Get-StatusPropertyValue -StatusObject $status -PropertyName 'misplaced_package_detected' -DefaultValue $false)
        PrecheckPassed = [bool](Get-StatusPropertyValue -StatusObject $status -PropertyName 'precheck_passed' -DefaultValue $false)
        PrecheckFailed = [bool](Get-StatusPropertyValue -StatusObject $status -PropertyName 'precheck_failed' -DefaultValue $false)
        CapabilityGatePassed = Get-StatusPropertyValue -StatusObject $status -PropertyName 'capability_gate_passed' -DefaultValue $null
        LegacyProjectDetected = [bool](Get-StatusPropertyValue -StatusObject $status -PropertyName 'legacy_project_detected' -DefaultValue $false)
        LegacyZeroQuestionMode = [bool](Get-StatusPropertyValue -StatusObject $status -PropertyName 'legacy_zero_question_mode' -DefaultValue $false)
        MissingCapabilities = @(Normalize-ToArray -Value (Get-StatusPropertyValue -StatusObject $status -PropertyName 'missing_capabilities' -DefaultValue @()))
        TruthSourceGatePassed = Get-StatusPropertyValue -StatusObject $status -PropertyName 'truth_source_gate_passed' -DefaultValue $null
        TruthSourceGateFailed = Get-StatusPropertyValue -StatusObject $status -PropertyName 'truth_source_gate_failed' -DefaultValue $null
        TruthSourceGateIssues = @(Normalize-ToArray -Value (Get-StatusPropertyValue -StatusObject $status -PropertyName 'truth_source_gate_issues' -DefaultValue @()))
        ClosureGateActive = [bool](Get-StatusPropertyValue -StatusObject $status -PropertyName 'closure_gate_active' -DefaultValue $false)
        ClosureMessage = if (Test-ClosureGateActive -StatusObject $status) { Get-ClosureMessage } else { '' }
    }
}

if (Test-Path (Get-SessionPath 'status.json')) {
    $gateStatus = Read-JsonFile -Path (Get-SessionPath 'status.json')
    if ((Test-ClosureGateActive -StatusObject $gateStatus) -and ($Stage -notin @('status', 'postcheck'))) {
        return Get-ClosureGateResponse
    }
}

switch ($Stage) {
    'start' { Start-Conversation }
    'answer' { Submit-Answer -InputText $InputText }
    'propose' { Build-Proposal }
    'confirm' { Confirm-Proposal -Choice $Choice }
    'status' { Show-Status }
    'bootstrap' { Invoke-BootstrapStage -TargetRoot $TargetRoot -ValuesPath $ValuesPath -Execute:$Execute -Force:$Force }
    'postcheck' { Invoke-PostcheckStage -TargetRoot $TargetRoot }
    'precheck' { Invoke-PrecheckStage -ProjectRoot $ProjectRoot }
    'doctor' { Invoke-DoctorStage -ProjectRoot $ProjectRoot }
}
