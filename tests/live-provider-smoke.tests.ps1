param(
    [string]$DeepSeekApiKey = '',
    [switch]$Enable
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

if (-not $Enable -and $env:COMMONHE_ENABLE_LIVE_PROVIDER_SMOKE -ne '1') {
    Write-Host 'SKIP live provider smoke: set COMMONHE_ENABLE_LIVE_PROVIDER_SMOKE=1 or pass -Enable to run credentialed network probes.'
    return
}

$repoRoot = Split-Path -Parent $PSScriptRoot
$tauriRoot = Join-Path $repoRoot 'apps\desktop\src-tauri'
$tmpRoot = Join-Path $repoRoot 'tmp\live-provider-smoke'
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

function Invoke-SanitizedProbe {
    param(
        [string]$Uri,
        [hashtable]$Headers
    )

    try {
        $response = Invoke-WebRequest -UseBasicParsing -Uri $Uri -Headers $Headers -Method Get -TimeoutSec 20
        return [pscustomobject]@{
            ok = $true
            status = [int]$response.StatusCode
        }
    }
    catch {
        $status = if ($_.Exception.Response) { [int]$_.Exception.Response.StatusCode.value__ } else { -1 }
        return [pscustomobject]@{
            ok = $false
            status = $status
            message = $_.Exception.Message
        }
    }
}

function Invoke-CheckedCargoTest {
    param(
        [string]$TestName
    )

    cargo test $TestName -- --ignored --nocapture | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "cargo test failed: $TestName"
    }
}

try {
    New-Item -ItemType Directory -Path $workspaceRoot -Force | Out-Null
    if (-not $DeepSeekApiKey) {
        $DeepSeekApiKey = $env:COMMONHE_DEEPSEEK_TEST_KEY
    }
    if (-not $DeepSeekApiKey) {
        $DeepSeekApiKey = $env:DEEPSEEK_API_KEY
    }

    $codexHome = Join-Path $env:USERPROFILE '.codex'
    $codexAuthPath = Join-Path $codexHome 'auth.json'
    $codexConfigPath = Join-Path $codexHome 'config.toml'
    $codexAuthExists = Test-Path -LiteralPath $codexAuthPath -PathType Leaf
    $codexConfigExists = Test-Path -LiteralPath $codexConfigPath -PathType Leaf
    $codexWireApi = if ($codexConfigExists) {
        (Select-String -Path $codexConfigPath -Pattern 'wire_api\s*=\s*"([^"]+)"').Matches.Groups[1].Value
    } else {
        ''
    }
    $summary = [ordered]@{
        deepseek = [ordered]@{
            provider = 'deepseek'
            model = 'deepseek-v4-flash'
            probeStatus = $null
            result = 'pending'
            blockedReason = $null
        }
        codex = [ordered]@{
            provider = 'codex'
            authSourceDetected = $codexAuthExists
            configSourceDetected = $codexConfigExists
            wireApi = if ($codexWireApi) { $codexWireApi } else { $null }
            result = 'pending'
            blockedReason = $null
        }
    }

    Push-Location $tauriRoot
    try {
        if ($DeepSeekApiKey) {
            $deepSeekProbe = Invoke-SanitizedProbe -Uri 'https://api.deepseek.com/models' -Headers @{
                Authorization = "Bearer $DeepSeekApiKey"
            }
            Assert-True ($deepSeekProbe.ok -and $deepSeekProbe.status -eq 200) 'DeepSeek live probe must return HTTP 200.'
            $summary.deepseek.probeStatus = $deepSeekProbe.status

            $env:COMMONHE_DEEPSEEK_TEST_KEY = $DeepSeekApiKey
            Invoke-CheckedCargoTest -TestName 'live_deepseek_provider_validation_and_model_discovery'
            Invoke-CheckedCargoTest -TestName 'live_deepseek_agent_flow_smoke'
            $summary.deepseek.result = 'passed'
        }
        else {
            $summary.deepseek.result = 'blocked'
            $summary.deepseek.blockedReason = 'missing_live_deepseek_key'
        }

        if ($codexAuthExists -and $codexConfigExists) {
            Invoke-CheckedCargoTest -TestName 'live_codex_provider_validation_and_model_discovery_from_local_auth'
            Invoke-CheckedCargoTest -TestName 'live_codex_agent_flow_from_local_auth_smoke'
            $summary.codex.result = 'passed'
        }
        else {
            $summary.codex.result = 'blocked'
            $summary.codex.blockedReason = 'missing_local_auth_or_config'
        }
    }
    catch {
        if ($summary.deepseek.result -eq 'pending') {
            $summary.deepseek.result = 'failed'
            $summary.deepseek.failure = $_.Exception.Message
        }
        elseif ($summary.codex.result -eq 'pending') {
            $summary.codex.result = 'failed'
            $summary.codex.failure = $_.Exception.Message
        }
        throw
    }
    finally {
        Remove-Item Env:COMMONHE_DEEPSEEK_TEST_KEY -ErrorAction SilentlyContinue
        Pop-Location
    }

    Set-Content -Path (Join-Path $fixtureRoot 'live-provider-summary.json') -Value ($summary | ConvertTo-Json -Depth 6)

    Assert-True (Test-Path -LiteralPath (Join-Path $fixtureRoot 'live-provider-summary.json') -PathType Leaf) 'Live smoke summary must be written to tmp/.'
}
finally {
    Write-Host "Live provider smoke artifacts: $fixtureRoot"
}
