Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot
$syncScriptPath = Join-Path $repoRoot 'scripts\sync-desktop-resources.ps1'
$buildScriptPath = Join-Path $repoRoot 'scripts\build-desktop.ps1'
$publishPortableScriptPath = Join-Path $repoRoot 'scripts\publish-desktop-portable.ps1'
$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("CommonHE-DesktopSmoke-" + [System.Guid]::NewGuid().ToString('N'))

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

try {
    Assert-True (Test-Path -LiteralPath $syncScriptPath -PathType Leaf) 'Desktop resource sync script is missing.'
    Assert-True (Test-Path -LiteralPath $buildScriptPath -PathType Leaf) 'Desktop build script is missing.'
    Assert-True (Test-Path -LiteralPath $publishPortableScriptPath -PathType Leaf) 'Desktop portable publish script is missing.'

    $resourceRoot = Join-Path $tempRoot 'apps\desktop\src-tauri\resources\commonhe'
    & $syncScriptPath -DestinationRoot $resourceRoot | Out-Null

    foreach ($expectedDirectory in @('config', 'core', 'init', 'templates', 'tools', 'specs')) {
        Assert-True (Test-Path -LiteralPath (Join-Path $resourceRoot $expectedDirectory) -PathType Container) "Synced payload is missing $expectedDirectory."
    }

    foreach ($excludedDirectory in @('release', 'tests', '.git', '.superpowers')) {
        Assert-False (Test-Path -LiteralPath (Join-Path $resourceRoot $excludedDirectory)) "Synced payload must not include $excludedDirectory."
    }

    Assert-True (Test-Path -LiteralPath (Join-Path $resourceRoot 'config\init-values.sample.json') -PathType Leaf) 'Synced payload is missing config/init-values.sample.json.'
    Assert-True (Test-Path -LiteralPath (Join-Path $resourceRoot 'config\target-client-capabilities.json') -PathType Leaf) 'Synced payload is missing config/target-client-capabilities.json.'
    Assert-True (Test-Path -LiteralPath (Join-Path $resourceRoot 'config\commonhe-truth-source-gates.json') -PathType Leaf) 'Synced payload is missing config/commonhe-truth-source-gates.json.'
    Assert-True (Test-Path -LiteralPath (Join-Path $resourceRoot 'specs\004-dual-agent-semantic-review\plan.md') -PathType Leaf) 'Synced payload is missing current Speckit 004 plan.'
    Assert-True (Test-Path -LiteralPath (Join-Path $resourceRoot 'core\HE-MANIFEST.md') -PathType Leaf) 'Synced payload is missing core/HE-MANIFEST.md.'
    Assert-True (Test-Path -LiteralPath (Join-Path $resourceRoot 'init\init-flow.md') -PathType Leaf) 'Synced payload is missing init/init-flow.md.'
    Assert-True (Test-Path -LiteralPath (Join-Path $resourceRoot '.specify\templates\spec-template.md') -PathType Leaf) 'Synced payload is missing .specify templates.'
    Assert-True (Test-Path -LiteralPath (Join-Path $resourceRoot '.specify\extensions\git\scripts\powershell\create-new-feature.ps1') -PathType Leaf) 'Synced payload is missing .specify PowerShell scripts.'
    Assert-True (Test-Path -LiteralPath (Join-Path $resourceRoot 'agency-agents-zh\README.md') -PathType Leaf) 'Synced payload is missing full agency-agents-zh README.'
    Assert-True (Test-Path -LiteralPath (Join-Path $resourceRoot 'agency-agents-zh\engineering\engineering-frontend-developer.md') -PathType Leaf) 'Synced payload is missing real agency-agents-zh agent files.'
    Assert-True (Test-Path -LiteralPath (Join-Path $resourceRoot 'templates\AGENTS.template.md') -PathType Leaf) 'Synced payload is missing templates/AGENTS.template.md.'
    Assert-True (Test-Path -LiteralPath (Join-Path $resourceRoot 'templates\CLAUDE.template.md') -PathType Leaf) 'Synced payload is missing templates/CLAUDE.template.md.'
    Assert-True (Test-Path -LiteralPath (Join-Path $resourceRoot 'tools\init-common-he.ps1') -PathType Leaf) 'Synced payload is missing tools/init-common-he.ps1.'
    Assert-True (Test-Path -LiteralPath (Join-Path $resourceRoot 'tools\assert-commonhe-truth-source.ps1') -PathType Leaf) 'Synced payload is missing tools/assert-commonhe-truth-source.ps1.'
    Assert-True (Test-Path -LiteralPath (Join-Path $resourceRoot 'references\product-manager.md') -PathType Leaf) 'Synced payload is missing references/product-manager.md.'
    Assert-True (Test-Path -LiteralPath (Join-Path $resourceRoot 'references\agency-agents-zh-README.md') -PathType Leaf) 'Synced payload is missing references/agency-agents-zh-README.md.'

    $fakeExePath = Join-Path $tempRoot 'commonhe-desktop.exe'
    Set-Content -Path $fakeExePath -Value 'fake exe payload'
    $portableReleaseRoot = Join-Path $tempRoot 'release'
    & $publishPortableScriptPath `
        -RepoRoot $repoRoot `
        -ReleaseName 'desktop-smoke-portable' `
        -SourceExePath $fakeExePath `
        -SourceResourcesRoot $resourceRoot `
        -ReleaseRoot $portableReleaseRoot | Out-Null

    $portableRoot = Join-Path $portableReleaseRoot 'desktop-smoke-portable'
    Assert-True (Test-Path -LiteralPath (Join-Path $portableRoot 'commonhe-desktop.exe') -PathType Leaf) 'Portable publish output is missing commonhe-desktop.exe.'
    Assert-True (Test-Path -LiteralPath (Join-Path $portableRoot 'resources\commonhe\references\product-manager.md') -PathType Leaf) 'Portable publish output is missing references/product-manager.md.'
    Assert-True (Test-Path -LiteralPath (Join-Path $portableRoot 'resources\commonhe\references\agency-agents-zh-README.md') -PathType Leaf) 'Portable publish output is missing references/agency-agents-zh-README.md.'
    Assert-True (Test-Path -LiteralPath (Join-Path $portableRoot 'resources\commonhe\.specify\templates\spec-template.md') -PathType Leaf) 'Portable publish output is missing .specify templates.'
    Assert-True (Test-Path -LiteralPath (Join-Path $portableRoot 'resources\commonhe\agency-agents-zh\engineering\engineering-frontend-developer.md') -PathType Leaf) 'Portable publish output is missing real agency-agents-zh agent files.'
    Assert-True (Test-Path -LiteralPath (Join-Path $portableRoot 'resources\commonhe\config\commonhe-truth-source-gates.json') -PathType Leaf) 'Portable publish output is missing truth-source gate manifest.'
    Assert-True (Test-Path -LiteralPath (Join-Path $portableRoot 'resources\commonhe\specs\004-dual-agent-semantic-review\plan.md') -PathType Leaf) 'Portable publish output is missing current Speckit 004 plan.'
    Assert-True (Test-Path -LiteralPath (Join-Path $portableRoot 'resources\commonhe\tools\assert-commonhe-truth-source.ps1') -PathType Leaf) 'Portable publish output is missing truth-source gate script.'
    Assert-True (Test-Path -LiteralPath (Join-Path $portableReleaseRoot 'desktop-smoke-portable.zip') -PathType Leaf) 'Portable publish output is missing the zip archive.'

    foreach ($scaffoldFile in @(
        'apps\desktop\package.json',
        'apps\desktop\src-tauri\Cargo.toml',
        'apps\desktop\src-tauri\tauri.conf.json'
    )) {
        Assert-True (Test-Path -LiteralPath (Join-Path $repoRoot $scaffoldFile) -PathType Leaf) "Desktop scaffold file is missing: $scaffoldFile"
    }
}
finally {
    if (Test-Path -LiteralPath $tempRoot) {
        Remove-Item -LiteralPath $tempRoot -Recurse -Force
    }
}
