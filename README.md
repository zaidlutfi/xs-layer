# xs-layer: a VIctory build for 8 GB NVIDIA cards

A merge of public [VIctory](https://github.com/Marvisak/VIctory) work, tuned and tested on a
laptop with an 8 GB NVIDIA card and 16 GB of RAM. VIctory is a compatibility layer that runs an
Xbox Series title natively on Windows from files the user supplies.

**This repository contains no game files, code or assets. You need your own dump of the
title (build 1.0.2.1).** It is not affiliated with or endorsed by the game's developer or
publisher.

## What is in it

- fxceless/victory-fork `main` at `32b31f75` (frame-rate work, `INDIRECT_GATHER`, window upload,
  `changed_runs` by masks, ocean texture read-back so boats float, cheaper card hashes)
- 2 MiB sections for the title's physical memory: issue 1089's patch by RyanCase94 (on by default;
  `VICTORY_SECTION_MB=0` turns it off). With the new `main` this took the open world here from
  2.8 to 3.8 fps
- Card copy census and `VICTORY_SKIP_WC_BACK`: pull request 868 (in the tree, left off)
- `VICTORY_CARD_AWAKE` from avatarmanz/VIctory-7900XTX (in the tree, left off)
- Series S picture: fxceless/victory-fork pull requests 188, 258, 883
- Indirect draws and fence waits: pull requests 1016, 1014, 1020
- Live ray tracing: pull request 814 (as merged in 905)
- Shader fixes from other forks (hair colour, 42 more programs translate): pull request 1112
- Compute loop limit (hair) 683, copy-back join 842

(Pull request numbers are fxceless/victory-fork's.) See [CREDITS.md](CREDITS.md).

## Tested on

RTX 4060 Laptop 8 GB (driver 32.0.16.2140), i9-12900H, 16 GB RAM, 32 GB page file,
Windows 11 26300, `--console series-s`, warm shader cache, 10-minute runs:

| Build | Where | Ray tracing | FPS (median) |
|---|---|---|---|
| v2 | Open world, safehouse, about 5,200 draws a frame | off | **3.8** (two runs) |
| v1 | same, back to back with v2 | off | 2.8 |
| v1 | same | on (`-RT`) | ~1.1-1.5 |

No crash in any of those runs. One machine only: treat it as a test build.

## Install

1. Download the zip from [Releases](../../releases) and unzip it anywhere.
2. Windows 11 with a recent NVIDIA driver. With 16 GB of RAM set the page file to 32 GB or
   more (System > About > Advanced system settings > Performance > Advanced > Virtual memory).
3. Close other heavy programs: the card and the RAM are both full while it runs.

## Run

From PowerShell in the unzipped folder:

```powershell
powershell -ExecutionPolicy Bypass -File .\run.ps1 -Game "D:\path\to\game_scarlett_master.exe"
```

Add `-RT` for live ray tracing (better light and shadows, slower).

The first launch translates every shader before the game starts (about 6 minutes on all
cores; `-SkipWarm` leaves that out). The game's first minutes can still be slow while the
driver makes pipelines. Cache and saves go to `%LOCALAPPDATA%\victory\cache` and `\data`.

## Build from source

```powershell
cargo build --profile diag -p victory --features diagnostics
```

The exe is `target\diag\victory.exe`; put it next to `run.ps1`, or pass `-Exe`.

## Known issues

- Series X profile does not fit in 8 GB: keep `--console series-s` (run.ps1 does).
- With ray tracing: grainy hair and cloth, a checkered shadow on some walls.
- Some compute programs are still refused on 32-lane NVIDIA cards (4 per run here).

## License

GPL-3.0-or-later, as VIctory. The upstream README is in [docs/UPSTREAM-README.md](docs/UPSTREAM-README.md).
