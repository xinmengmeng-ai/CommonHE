param(
    [switch]$SkipTests,
    [switch]$SkipResourceSync,
    [switch]$SkipInstall,
    [switch]$SkipTauriBuild,
    [switch]$SkipFrontendBuild,
    [switch]$BundleInstaller,
    [switch]$SkipPortablePublish
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot
$testsRoot = Join-Path $repoRoot 'tests'
$desktopRoot = Join-Path $repoRoot 'apps\desktop'
$syncScriptPath = Join-Path $PSScriptRoot 'sync-desktop-resources.ps1'
$publishPortableScriptPath = Join-Path $PSScriptRoot 'publish-desktop-portable.ps1'

function Get-ToolPath {
    param(
        [string[]]$Names,
        [string]$MissingMessage
    )

    foreach ($name in $Names) {
        $command = Get-Command $name -ErrorAction SilentlyContinue
        if ($null -ne $command) {
            return $command.Source
        }
    }

    throw $MissingMessage
}

function Invoke-ExternalCommand {
    param(
        [string]$FilePath,
        [string[]]$Arguments,
        [string]$FailureMessage
    )

    & $FilePath @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw $FailureMessage
    }
}

function Assert-DesktopScaffold {
    $requiredScaffoldFiles = @(
        'package.json',
        'src-tauri\Cargo.toml',
        'src-tauri\tauri.conf.json'
    )

    foreach ($relativePath in $requiredScaffoldFiles) {
        $fullPath = Join-Path $desktopRoot $relativePath
        if (-not (Test-Path -LiteralPath $fullPath -PathType Leaf)) {
            throw "Desktop scaffold file is missing: apps\desktop\$relativePath"
        }
    }
}

if (-not $SkipTests) {
    $powershellCommand = Get-Command pwsh -ErrorAction SilentlyContinue
    if ($null -eq $powershellCommand) {
        $powershellCommand = Get-Command powershell -ErrorAction Stop
    }

    $releasePackageTestPath = Join-Path $testsRoot 'release-package.tests.ps1'
    $testFiles = Get-ChildItem -LiteralPath $testsRoot -Filter '*.tests.ps1' -File |
        Where-Object { $_.FullName -ne $releasePackageTestPath } |
        Sort-Object FullName
    foreach ($testFile in $testFiles) {
        Write-Host "Running PowerShell test: $($testFile.FullName)"
        Invoke-ExternalCommand `
            -FilePath $powershellCommand.Source `
            -Arguments @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $testFile.FullName) `
            -FailureMessage "PowerShell test failed: $($testFile.FullName)"
    }
}

if (-not $SkipResourceSync) {
    Write-Host 'Syncing CommonHE resources for desktop build.'
    & $syncScriptPath | Out-Host
}

$nodePath = Get-ToolPath -Names @('node') -MissingMessage 'Node.js was not found on PATH. Install Node.js before running the CommonHE Desktop build.'
$npmPath = Get-ToolPath -Names @('npm.cmd', 'npm') -MissingMessage 'npm was not found on PATH. Install npm before running the CommonHE Desktop build.'

Write-Host "Using node: $nodePath"
Write-Host "Using npm: $npmPath"
if (-not $SkipTauriBuild) {
    $cargoPath = Get-ToolPath -Names @('cargo') -MissingMessage 'Cargo was not found on PATH. Install Rust/Cargo before running the Tauri desktop build. The Tauri build was not started.'
    Write-Host "Using cargo: $cargoPath"
}

Assert-DesktopScaffold

Push-Location $desktopRoot
try {
    if (-not $SkipInstall) {
        if (Test-Path -LiteralPath (Join-Path $desktopRoot 'package-lock.json') -PathType Leaf) {
            Invoke-ExternalCommand -FilePath $npmPath -Arguments @('ci') -FailureMessage 'npm ci failed for CommonHE Desktop.'
        }
        else {
            Invoke-ExternalCommand -FilePath $npmPath -Arguments @('install') -FailureMessage 'npm install failed for CommonHE Desktop.'
        }
    }

    if (-not $SkipTauriBuild) {
        if ($SkipFrontendBuild) {
            $distIndexPath = Join-Path $desktopRoot 'dist\index.html'
            if (-not (Test-Path -LiteralPath $distIndexPath -PathType Leaf)) {
                throw 'Frontend dist is missing. Run without -SkipFrontendBuild once before building the Tauri desktop app.'
            }
        }
        else {
            Invoke-ExternalCommand -FilePath $npmPath -Arguments @('run', 'build') -FailureMessage 'Vite frontend build failed for CommonHE Desktop.'
        }

        $tauriConfigPath = Join-Path 'src-tauri' 'tauri.portable.conf.json'
        $tauriBuildArgs = if ($BundleInstaller) {
            @('run', 'tauri', '--', 'build', '--config', $tauriConfigPath)
        } else {
            @('run', 'tauri', '--', 'build', '--no-bundle', '--config', $tauriConfigPath)
        }

        Invoke-ExternalCommand -FilePath $npmPath -Arguments $tauriBuildArgs -FailureMessage 'Tauri desktop build failed.'
    }
}
finally {
    Pop-Location
}

if (-not $SkipTauriBuild -and -not $SkipPortablePublish) {
    $publishResult = & $publishPortableScriptPath -RepoRoot $repoRoot
    $publishResult | Out-Host

    if (-not $SkipTests) {
        $releasePackageTestPath = Join-Path $testsRoot 'release-package.tests.ps1'
        Write-Host "Running PowerShell release package test: $releasePackageTestPath"
        Invoke-ExternalCommand `
            -FilePath $powershellCommand.Source `
            -Arguments @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $releasePackageTestPath, '-ReleaseZipPath', $publishResult.ZipPath, '-ReleaseName', $publishResult.ReleaseName) `
            -FailureMessage "PowerShell release package test failed: $releasePackageTestPath"
    }
}
