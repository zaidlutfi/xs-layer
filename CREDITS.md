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
