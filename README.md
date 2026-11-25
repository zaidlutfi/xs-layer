# xs-layer: a VIctory build for 8 GB NVIDIA cards

A merge of public [VIctory](https://github.com/Marvisak/VIctory) work, tuned and tested on a
laptop with an 8 GB NVIDIA card and 16 GB of RAM. VIctory is a compatibility layer that runs an
Xbox Series title natively on Windows from files the user supplies.

**This repository contains no game files, code or assets. You need your own dump of the
title (build 1.0.2.1).** It is not affiliated with or endorsed by the game's developer or
publisher.

## What is in it

- fxceless/victory-fork `main` at `085ee7c` (frame-rate work, `INDIRECT_GATHER`, window upload)
- Series S picture: fxceless/victory-fork pull requests 188, 258, 883
- Indirect draws and fence waits: pull requests 1016, 1014, 1020
- Live ray tracing: pull request 814 (as merged in 905)
- Shader fixes from other forks (hair colour, 42 more programs translate): pull request 1112
- Compute loop limit (hair) 683, copy-back join 842

(Pull request numbers are fxceless/victory-fork's.) See [CREDITS.md](CREDITS.md).

## Tested on

RTX 4060 Laptop 8 GB (driver 32.0.16.2140), i9-12900H, 16 GB RAM, 32 GB page file,
Windows 11 26300, `--console series-s`, warm shader cache, 10-minute runs:

| Where | Ray tracing | FPS (median) |
|---|---|---|
| Open world, safehouse | off | 2.7-2.9 (best 3.2) |
| Open world, safehouse | on (`-RT`) | ~1.1-1.5 |

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

The first launch translates and builds thousands of shaders: several minutes, slow or black
screens. It can crash once in that time; start it again, the cache is kept. Cache and saves go
to `%LOCALAPPDATA%\victory\cache` and `\data`.

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
