# Runs your own dump of the title through this build, Series S profile.
# Usage (PowerShell, from this folder):
#   powershell -ExecutionPolicy Bypass -File .\run.ps1 -Game "D:\path\to\game_scarlett_master.exe"
#   the first launch builds every shader first (minutes); -SkipWarm leaves that out
param(
    [Parameter(Mandatory = $true)][string]$Game,
    [switch]$SkipWarm,
    [string]$Exe
)
# The folder is not known yet where a parameter's default is worked out (Windows PowerShell 5.1,
# with a mandatory parameter above): the default used to come out as "\victory.exe".
if (-not $Exe) { $Exe = Join-Path $PSScriptRoot 'victory.exe' }
if (-not (Test-Path $Game)) { Write-Error "Game not found: $Game"; exit 1 }
if (-not (Test-Path $Exe)) { Write-Error "victory.exe not found next to run.ps1 (or pass -Exe)"; exit 1 }

# The 8 GB laptop set (after lordon1a/victory-laptop-8gb), as measured here on an
# RTX 4060 Laptop 8 GB with 16 GB of RAM: 5 to 6 fps in the open world.
$settings = [ordered]@{
    # Shader writes come back beside the write guards, on a thread of their own
    VICTORY_RELAY             = '2'
    VICTORY_RELAY_COPY        = '1'
    VICTORY_WIDE_BACK         = '1'
    VICTORY_JOIN_COPIES       = '1'
    VICTORY_UNDER_WIDE        = '0'
    VICTORY_LAND_ECHO         = '0'
    # Indirect draws clamped in batches
    VICTORY_INDIRECT_BATCH    = '2'
    VICTORY_INDIRECT_WINDOW   = '1'
    VICTORY_DYNAMIC_SLOTS     = '524288'
    # The card's 8 GB: textures capped, 2 GB of the title's memory windows in the card's
    # own memory (through the resizable BAR), the card left to sleep between frames.
    # 2048 holds every texture the open world asks for (1.4 GB used); 1024 is about 5 %
    # faster but leaves some 4,500 texture requests without a copy.
    VICTORY_TEXTURE_BUDGET_MB = '2048'
    VICTORY_TEXTURE_NO_WAIT   = '1'
    VICTORY_WINDOWS_BAR_MB    = '2048'
    VICTORY_WINDOW_KB         = '256'
    VICTORY_WINDOW_TAILS      = '1'
    VICTORY_CARD_AWAKE        = '0'
    VICTORY_CARD_REHASH_RUNS  = '1'
    VICTORY_CARD_STEPS        = '2'
    VICTORY_SHOW_AFTER        = '1'
    # Threads and bookkeeping
    VICTORY_MAP_THREADS       = '1'
    VICTORY_TITLE_PRIORITIES  = '0'
    VICTORY_MEMORY_MAP        = '1'
    VICTORY_PIPELINE_BOOK     = '1'
    # A stand-in for the bounce light that does not run yet (shadows are still dark)
    VICTORY_AMBIENT           = '0.8'
    # Crash data that is cheap enough to leave on
    VICTORY_D3D12_DRED        = 'light'
}
# Left out on purpose, each slower on an 8 GB card: VICTORY_CARD_BUFFERS_MIN_KB=64 (card copies
# of small buffers), VICTORY_THIN=1 and VICTORY_JOIN_PARTS=8 (the preset for 32 GB cards: 2.4 to
# 2.6 fps here against 5 to 6). Also measured and left as they are: VICTORY_THIN=1 alone
# (4.2 fps), VICTORY_WINDOWS_BAR_MB=3072 (no gain).
foreach ($k in $settings.Keys) { Set-Item "Env:$k" $settings[$k] }

# First launch: translate every shader up front on all cores instead of one at a time in game
# (black screens otherwise). The cache keys on the switches above, so this runs after them.
$pack = Join-Path (Split-Path $Game) 'shaders.rpf'
$ready = Join-Path $PSScriptRoot ('.shaders-ready-' + (Get-Item $Exe).LastWriteTimeUtc.Ticks)
if (-not $SkipWarm -and -not (Test-Path $ready) -and (Test-Path $pack)) {
    Write-Host 'First launch: building all shaders before the game starts (several minutes)...'
    & $Exe shaders "$pack"
    if ($LASTEXITCODE -eq 0) { New-Item -ItemType File $ready -Force | Out-Null }
}

& $Exe run "$Game" --execute --console series-s
