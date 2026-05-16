param(
    [string]$RepoRoot = (Split-Path -Parent $PSScriptRoot),
    [string]$DestinationRoot
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$sourceDirectoryNames = @('config', 'core', 'init', 'templates', 'tools', 'specs', '.specify', 'agency-agents-zh')
$excludedDirectoryNames = @('release', 'tests', '.git', '.superpowers')
$extraFileMappings = @(
    @{ Source = 'product-manager.md'; Target = 'references\product-manager.md' },
    @{ Source = 'agency-agents-zh\README.md'; Target = 'references\agency-agents-zh-README.md' }
)

function Get-NormalizedFullPath {
    param([string]$Path)

    return ([System.IO.Path]::GetFullPath($Path)).TrimEnd('\', '/').Replace('/', '\')
}

function Test-IsPathInside {
    param(
        [string]$ChildPath,
        [string]$ParentPath
    )

    $child = Get-NormalizedFullPath $ChildPath
    $parent = Get-NormalizedFullPath $ParentPath

    if ($child.Length -le $parent.Length) {
        return $false
    }

    return $child.StartsWith("$parent\", [System.StringComparison]::OrdinalIgnoreCase)
}

function Get-RelativePath {
    param(
        [string]$BasePath,
        [string]$FullPath
    )

    $base = (Get-NormalizedFullPath $BasePath) + '\'
    $full = Get-NormalizedFullPath $FullPath
    return $full.Substring($base.Length)
}

function Test-IsExcludedRelativePath {
    param([string]$RelativePath)

    $segments = $RelativePath -split '[\\/]+' | Where-Object { $_ -ne '' }
    foreach ($segment in $segments) {
        if ($excludedDirectoryNames -contains $segment) {
            return $true
        }
    }

    return $false
}

$repoRootFull = Get-NormalizedFullPath $RepoRoot
if ([string]::IsNullOrWhiteSpace($DestinationRoot)) {
    $DestinationRoot = Join-Path $repoRootFull 'apps\desktop\src-tauri\resources\commonhe'
}

$destinationFull = Get-NormalizedFullPath $DestinationRoot
if ([System.IO.Path]::GetFileName($destinationFull) -ne 'commonhe') {
    throw "Refusing to sync desktop resources into '$destinationFull'. Destination folder must be named 'commonhe'."
}

foreach ($sourceDirectoryName in $sourceDirectoryNames) {
    $sourcePath = Join-Path $repoRootFull $sourceDirectoryName
    if (-not (Test-Path -LiteralPath $sourcePath -PathType Container)) {
        throw "Required source directory is missing: $sourceDirectoryName"
    }

    if ($destinationFull -eq (Get-NormalizedFullPath $sourcePath) -or (Test-IsPathInside -ChildPath $destinationFull -ParentPath $sourcePath)) {
        throw "Refusing to sync desktop resources into source directory: $destinationFull"
    }
}

if ($destinationFull -eq $repoRootFull) {
    throw "Refusing to sync desktop resources into repository root."
}

if (Test-Path -LiteralPath $destinationFull) {
    Remove-Item -LiteralPath $destinationFull -Recurse -Force
}

New-Item -ItemType Directory -Path $destinationFull -Force | Out-Null

$filesCopied = 0
foreach ($sourceDirectoryName in $sourceDirectoryNames) {
    $sourcePath = Join-Path $repoRootFull $sourceDirectoryName
    $targetRoot = Join-Path $destinationFull $sourceDirectoryName
    New-Item -ItemType Directory -Path $targetRoot -Force | Out-Null

    foreach ($item in Get-ChildItem -LiteralPath $sourcePath -Recurse -Force) {
        $relativePath = Get-RelativePath -BasePath $sourcePath -FullPath $item.FullName
        if (Test-IsExcludedRelativePath -RelativePath $relativePath) {
            continue
        }

        $targetPath = Join-Path $targetRoot $relativePath
        if ($item.PSIsContainer) {
            New-Item -ItemType Directory -Path $targetPath -Force | Out-Null
            continue
        }

        $targetParent = Split-Path -Parent $targetPath
        New-Item -ItemType Directory -Path $targetParent -Force | Out-Null
        Copy-Item -LiteralPath $item.FullName -Destination $targetPath -Force
        $filesCopied++
    }
}

foreach ($mapping in $extraFileMappings) {
    $sourcePath = Join-Path $repoRootFull $mapping.Source
    if (-not (Test-Path -LiteralPath $sourcePath -PathType Leaf)) {
        continue
    }

    $targetPath = Join-Path $destinationFull $mapping.Target
    $targetParent = Split-Path -Parent $targetPath
    New-Item -ItemType Directory -Path $targetParent -Force | Out-Null
    Copy-Item -LiteralPath $sourcePath -Destination $targetPath -Force
    $filesCopied++
}

[pscustomobject]@{
    Destination = $destinationFull
    SourceDirectories = $sourceDirectoryNames
    FilesCopied = $filesCopied
}
