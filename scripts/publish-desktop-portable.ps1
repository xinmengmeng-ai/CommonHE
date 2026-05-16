param(
    [string]$RepoRoot = (Split-Path -Parent $PSScriptRoot),
    [string]$ReleaseName = 'xingxing-vibecoding-launcher-v1.0-portable-current',
    [string]$SourceExePath,
    [string]$SourceResourcesRoot,
    [string]$ReleaseRoot
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

if ([string]::IsNullOrWhiteSpace($SourceExePath)) {
    $SourceExePath = Join-Path $RepoRoot 'apps\desktop\src-tauri\target\release\commonhe-desktop.exe'
}

if ([string]::IsNullOrWhiteSpace($SourceResourcesRoot)) {
    $SourceResourcesRoot = Join-Path $RepoRoot 'apps\desktop\src-tauri\resources\commonhe'
}

if ([string]::IsNullOrWhiteSpace($ReleaseRoot)) {
    $ReleaseRoot = Join-Path $RepoRoot 'release'
}

$destinationRoot = Join-Path $ReleaseRoot $ReleaseName
$zipPath = Join-Path $ReleaseRoot "$ReleaseName.zip"
$destinationResourcesRoot = Join-Path $destinationRoot 'resources\commonhe'

function Assert-PathExists {
    param(
        [string]$Path,
        [string]$Message
    )

    if (-not (Test-Path -LiteralPath $Path)) {
        throw $Message
    }
}

function New-PortableZip {
    param(
        [string]$SourceDirectory,
        [string]$DestinationPath
    )

    Add-Type -AssemblyName System.IO.Compression.FileSystem

    $maxAttempts = 5
    for ($attempt = 1; $attempt -le $maxAttempts; $attempt++) {
        try {
            if (Test-Path -LiteralPath $DestinationPath) {
                Remove-Item -LiteralPath $DestinationPath -Force
            }

            [System.IO.Compression.ZipFile]::CreateFromDirectory($SourceDirectory, $DestinationPath)
            return
        }
        catch {
            if (Test-Path -LiteralPath $DestinationPath) {
                Remove-Item -LiteralPath $DestinationPath -Force -ErrorAction SilentlyContinue
            }

            if ($attempt -eq $maxAttempts) {
                throw
            }

            $delayMs = 500 * $attempt
            Write-Warning "Portable zip attempt $attempt failed, retrying in ${delayMs}ms: $($_.Exception.Message)"
            Start-Sleep -Milliseconds $delayMs
        }
    }
}

Assert-PathExists -Path $SourceExePath -Message "Portable source exe is missing: $SourceExePath"
Assert-PathExists -Path $SourceResourcesRoot -Message "Portable source resources are missing: $SourceResourcesRoot"
Assert-PathExists -Path (Join-Path $SourceResourcesRoot 'references\product-manager.md') -Message 'Portable source resources are missing references/product-manager.md'
Assert-PathExists -Path (Join-Path $SourceResourcesRoot 'references\agency-agents-zh-README.md') -Message 'Portable source resources are missing references/agency-agents-zh-README.md'

if (Test-Path -LiteralPath $destinationRoot) {
    Remove-Item -LiteralPath $destinationRoot -Recurse -Force
}

New-Item -ItemType Directory -Path $destinationRoot -Force | Out-Null
Copy-Item -LiteralPath $SourceExePath -Destination (Join-Path $destinationRoot 'commonhe-desktop.exe') -Force
Copy-Item -LiteralPath $SourceResourcesRoot -Destination $destinationResourcesRoot -Recurse -Force

if (Test-Path -LiteralPath $zipPath) {
    Remove-Item -LiteralPath $zipPath -Force
}
New-PortableZip -SourceDirectory $destinationRoot -DestinationPath $zipPath

[pscustomobject]@{
    ReleaseName = $ReleaseName
    PortableRoot = $destinationRoot
    PortableExe = Join-Path $destinationRoot 'commonhe-desktop.exe'
    ZipPath = $zipPath
}
