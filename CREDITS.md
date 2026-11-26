# Credits

This build is a merge; the work is theirs. All GPL-3.0-or-later unless noted.

- **VIctory** by Marvisak and its contributors: the base of everything (loader, runtime
  services, Direct3D 12 back end, shader recompiler). https://github.com/Marvisak/VIctory
- **victory-fork** by fxceless (hyphennn): frame-rate and rendering work this build follows.
  https://github.com/fxceless/victory-fork
- Pull requests to victory-fork merged here, by their authors:
  - 814 live ray tracing, Series S overlay/upscaler, 8 GB headroom: staronthehorizon (merged as 905)
  - 188 Series S second present plane and upscale stretch
  - 258 image stores keep unwritten channels
  - 883 certified shared-memory accesses for paired wave64 compute
  - 1016 indirect argument clamp for single records
  - 1014 card compare with 64-thread groups
  - 1020 monitor waits parked on layer-owned GPU fences
  - 1112 shader fixes collected from other forks (Marvisak/VIctory c0228d9 by tjthebigbearcat-hub;
    DexNom/VIctory 921d8d3, ddb7332, 76749ce)
  - 683 loop limit for compute programs
  - 842 copy-back join (VICTORY_BACK_JOIN)
  - 868 card copy census, VICTORY_SKIP_WC_BACK
  - 999 ocean texture read-back (cxy0072019-sketch, porting Geckomacabre's patch)
- Issue 1089: 2 MiB sections, the patch by RyanCase94 (with data points from HLVM04 and
  Exopus-0x65f7ad0).
- avatarmanz/VIctory-7900XTX: VICTORY_CARD_AWAKE (commit 5f86fa30).
- The upstream README in docs/UPSTREAM-README.md keeps the full credit list of victory-fork.

Merge and tests on an 8 GB laptop: zaidlutfi, with Claude Code.

## v3 (2026-11-26)

v3 moves to another tree; the list above is what v1 and v2 were made of.

- **cool_fox** by codec_d: the tree v3 is exported from, at its commit `f4b48ad`
  (NVIDIA Wave32 lowering, Series S two-plane present, residency budgets).
  https://github.com/codec-d/cool_fox
- **VIctory-7900XTX** by avatarmanz, which cool_fox is built on (relay and landing of shader
  writes, card steps, batched indirect clamps, textures on a copy queue of their own), and the
  work it carries from DexNom/VIctory, Belobug/VIctory-Belobug-AMD, FQY55kai/ZJOU-VICTORY and
  myokex/victory-amd. https://github.com/avatarmanz/VIctory-7900XTX
- **victory-laptop-8gb** by lordon1a: the settings in `run.ps1` are its measured set for an
  8 GB laptop card with 16 GB of RAM, and its test log is why three switches are left out.
  https://github.com/lordon1a/victory-laptop-8gb
- docs/UPSTREAM-README.md is cool_fox's README with its own credit list.

Changes here against cool_fox `f4b48ad`: the title's name taken out of five comments, its
launcher and bug journal left out, `run.ps1`, this file and the README.