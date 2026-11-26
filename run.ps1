# Runs your own dump of the title through this build, Series S profile.
# Usage (PowerShell, from this folder):
#   powershell -ExecutionPolicy Bypass -File .\run.ps1 -Game "D:\path\to\game_scarlett_master.exe"
#   add -RT to turn on live ray tracing (better light and shadows, slower)
#   the first launch builds every shader first (minutes); -SkipWarm leaves that out
param(
    [Parameter(Mandatory = $true)][string]$Game,
    [switch]$RT,
    [switch]$SkipWarm,
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
$env:VICTORY_CARD_HASH_WIDE = '1'   # card compares with 64-thread groups (pull request 1014)
$env:VICTORY_CARD_HASH_LEN = '1'    # hash a buffer's own bytes, not a whole 64 KiB piece
# Left off: they were slower here (2.0 fps against 3.8): VICTORY_TEXTURE_NO_WAIT (also black
# flashes), VICTORY_SKIP_WC_BACK, VICTORY_CARD_AWAKE, VICTORY_JOIN_COPIES, VICTORY_CARD_REHASH_RUNS

# Ocean textures the title reads back: boats float on the first boat mission
$env:VICTORY_TEXTURE_BACK = '1'

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

# First launch: translate every shader up front on all cores (about 6 minutes) instead of one
# at a time in game (black screens, a possible crash). The cache keys on the switches above, so
# this runs after them; an interrupted pass goes on from where it stopped next time.
$pack = Join-Path (Split-Path $Game) 'shaders.rpf'
$ready = Join-Path $PSScriptRoot ('.shaders-ready-' + (Get-Item $Exe).LastWriteTimeUtc.Ticks)
if (-not $SkipWarm -and -not (Test-Path $ready) -and (Test-Path $pack)) {
    Write-Host 'First launch: building all shaders before the game starts (several minutes)...'
    & $Exe translate "$pack" --warm all --jobs $env:NUMBER_OF_PROCESSORS
    if ($LASTEXITCODE -eq 0) { New-Item -ItemType File $ready -Force | Out-Null }
}

& $Exe run "$Game" --execute --console series-s
