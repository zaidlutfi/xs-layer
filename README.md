# xs-layer: a VIctory build for 8 GB NVIDIA cards

A build of public [VIctory](https://github.com/Marvisak/VIctory) work, tuned and tested on a
laptop with an 8 GB NVIDIA card and 16 GB of RAM. VIctory is a compatibility layer that runs an
Xbox Series title natively on Windows from files the user supplies.

**This repository contains no game files, code or assets. You need your own dump of the
title (build 1.0.2.1).** It is not affiliated with or endorsed by the game's developer or
publisher.

## What is in it (v3)

- The tree of [codec-d/cool_fox](https://github.com/codec-d/cool_fox) at `f4b48ad`, which is
  built on [avatarmanz/VIctory-7900XTX](https://github.com/avatarmanz/VIctory-7900XTX): shader
  writes come back on a thread of their own, indirect draws are clamped in batches, textures go
  up on their own queue, wave64 programs are lowered for 32-lane NVIDIA cards.
- `run.ps1` with the settings measured for an 8 GB laptop card by
  [lordon1a/victory-laptop-8gb](https://github.com/lordon1a/victory-laptop-8gb): 2 GB of the
  title's memory windows kept in the card's own memory, no card copies of small buffers, and
  textures capped at 2 GB (its set uses 1 GB; see below).

v1 and v2 were a merge on fxceless/victory-fork; their sources are in this repository's history.
See [CREDITS.md](CREDITS.md).

## Tested on

RTX 4060 Laptop 8 GB (driver 621.40), i9-12900H, 16 GB RAM, 32 GB page file, Windows 11 26300,
`--console series-s`, warm shader cache, open-world save at the safehouse, ray tracing off.

v3, one 15-minute run, medians of the 5-second log lines by how much the frame draws:

| Draws a frame | v3 fps | v2 fps |
|---|---|---|
| under 4,500 | **6.4** | about 5.7 |
| 4,500 to 6,500 | **6.3** | about 4.1 |
| 6,500 to 9,000 | **5.6** | about 3.2 |
| above 9,000 | **4.7 to 5.5** | about 2.6 |

Those runs had the texture cap at 1 GB. `run.ps1` ships with 2 GB: every texture the open world
asks for then gets a copy (1.4 GB used, where 1 GB left about 4,500 requests without one) for
about 5 % of the frame rate (5.9, 5.4 and 4.9 fps in the last three rows, one run). Set
`VICTORY_TEXTURE_BUDGET_MB` back to 1024 in `run.ps1` for the faster numbers.

Whole runs of v3: 4.6 and 4.9 fps (10 minutes each, about 7,500 draws), 5.8 fps (15 minutes,
about 6,000 draws). The v2 column is computed from v2's own earlier runs (frame time against
draws), not run side by side. No crash in any of these runs. One machine only: treat it as a
test build.

Why it is faster on 8 GB: the card stays under its memory budget (5.3 of 7.2 GB) and the
submitting thread waits 0 to 15 ms a frame for it; in v2 the card was the limit at about
31 microseconds a draw. The limit is now the processor's side (preparing 115 to 145 ms and
copy-back 50 to 65 ms a frame).

## Frame generation (optional)

NVIDIA's Smooth Motion can be turned on for `victory.exe` in the NVIDIA app (Graphics > Program
Settings > add the exe > Smooth Motion > On; RTX 40 series, driver 621.40 here). Measured with
PresentMon over 88 s: the title drew 5.4 fps and 10.2 frames a second were presented, so it
does double them, and the real frame rate did not drop. The pacing is uneven (a tenth of the
presents follow the one before at once, a tenth come 250 ms or more later), and how it looks at
so low a base rate has not been judged here: input still answers at the real rate.

## Install

1. Download the zip from [Releases](../../releases) and unzip it anywhere.
2. Windows 11 with a recent NVIDIA driver. With 16 GB of RAM set the page file to 32 GB or
   more (System > About > Advanced system settings > Performance > Advanced > Virtual memory).
3. Close other heavy programs: the RAM is full while it runs.

## Run

From PowerShell in the unzipped folder:

```powershell
powershell -ExecutionPolicy Bypass -File .\run.ps1 -Game "D:\path\to\game_scarlett_master.exe"
```

The first launch translates every shader before the game starts (several minutes on all
cores; `-SkipWarm` leaves that out). The game's first minutes can still be slow while the
driver makes pipelines.

## Build from source

```powershell
cargo build --locked --profile diag -p victory --features diagnostics
```

The exe is `target\diag\victory.exe`; put it next to `run.ps1`, or pass `-Exe`.

## Known issues

- Anything in shadow is darker than in v2.
- About 900 indirect draws a frame are dropped on NVIDIA cards (the log's `dropped ... indirect`):
  small objects can be missing.
- Ray tracing is not set up in `run.ps1` for v3 (not tested here).
- Series X profile does not fit in 8 GB: keep `--console series-s` (run.ps1 does).

## Measured with no gain on this machine

So nobody has to repeat them: a 720p display mode (the title ignores it and renders
1920x1080), variable rate shading at 4x4 (the card's time does not change, so an upscaler such
as DLSS or FSR would not help either), leaving out the barrier waits, keeping the process on the
fast cores, lordon1a's own build (5.2 fps at 7,400 draws), `VICTORY_THIN=1` on top of these
settings (4.2 fps), 3 GB of windows in the card's memory instead of 2 (no change).

## License

GPL-3.0-or-later, as VIctory. The upstream README is in [docs/UPSTREAM-README.md](docs/UPSTREAM-README.md).
