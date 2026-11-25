# Runs your own dump of the title through this build, Series S profile.
# Usage (PowerShell, from this folder):
#   powershell -ExecutionPolicy Bypass -File .\run.ps1 -Game "D:\path\to\game_scarlett_master.exe"
#   add -RT to turn on live ray tracing (better light and shadows, slower)
param(
    [Parameter(Mandatory = $true)][string]$Game,
    [switch]$RT,
    [string]$Exe = "$PSScriptRoot\victory.exe"
)
if (-not (Test-Path $Game)) { Write-Error "Game not found: $Game"; exit 1 }
if (-not (Test-Path $Exe)) { Write-Error "victory.exe not found next to run.ps1 (or pass -Exe)"; exit 1 }

# Speed switches (measured on an 8 GB NVIDIA card)
$env:VICTORY_COPY_PLAIN = '1'
$env:VICTORY_JOIN_PARTS = '1'
$env:VICTORY_BACK_OWN_VIEW = '1'
$env:VICTORY_ASYNC_SUBMIT = '1'
$env:VICTORY_QUEUE2 = '1'
$env:VICTORY_INDIRECT_WINDOW = '1'
$env:VICTORY_INDIRECT_GATHER = '1'
$env:VICTORY_INDIRECT_SINGLE = '1'
$env:VICTORY_MONITOR_FENCE_WAIT = '1'
$env:VICTORY_BACK_JOIN = '256'
$env:VICTORY_FPS_COUNTER = '0'
# (VICTORY_TEXTURE_NO_WAIT=1 is left off: objects flashed black and it was not faster)

# Memory limits for 8 GB cards (raise VICTORY_TEXTURE_BUDGET_MB on 12 GB+ cards)
$env:VICTORY_TEXTURE_BUDGET_MB = '4096'
$env:VICTORY_TEXTURE_DROP_MIPS = '1'

# Hair and compute loop limits
$env:VICTORY_SHADER_LOOP_LIMIT = '32'
$env:VICTORY_SHADER_LOOP_LIMIT_CS = '128'

if ($RT) {
    # Live ray tracing (lighting, shadows, reflections)
    $env:VICTORY_BVH_BUILDER = '1'
    $env:VICTORY_BVH_QUARANTINE = '0'
    $env:VICTORY_CARD_FETCH_INFLIGHT = '1'
}

& $Exe run "$Game" --execute --console series-s
