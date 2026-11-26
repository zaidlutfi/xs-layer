# Diagnostic logs

Logs of short diagnostics runs on the test machine (Windows 11, Intel i7-14700KF, NVIDIA RTX 4070
12 GB, 32 GB RAM), for people who want to suggest changes from real numbers.

They are scrubbed before they are put here: the Windows user and machine names, folders, the
title's folder, file, product and pack names, its title id, the names the title asked the network
for, the title's own output lines, and the title's own shader and program names are replaced by placeholders (`<title folder>`,
`<program>`, ...). Shader and pipeline hashes are kept.

| File | Run | What it is |
|------|-----|------------|
| `timing-run-2026-11-25.txt` | 278 | The build of commit `c36971b2`, diagnostics profile, 60 s after the first world frame, with `VICTORY_GPU_TIMING=1` and `VICTORY_D3D12_DRED=1`. Slower than a normal run by design: the timing run sends each list on its own, and the breadcrumbs make recording about 2.4 times slower. Other switches: `VICTORY_LEVELS_ON_DEMAND=1 VICTORY_ASYNC_SUBMIT=1 VICTORY_QUEUE2=1 VICTORY_INDIRECT_WINDOW=1 VICTORY_TEXTURE_NO_WAIT=1 VICTORY_MERGE_GAP=64 VICTORY_MERGE_RUNS=4096 VICTORY_STAGING_POOL=0 VICTORY_COPY_PLAIN=1 VICTORY_JOIN_PARTS=1 VICTORY_BACK_OWN_VIEW=1 VICTORY_INDIRECT_GATHER=1 VICTORY_CARD_REHASH_RUNS=1 VICTORY_CARD_WINDOWS_MB=2048 VICTORY_CARD_BUFFERS_MIN_KB=64 VICTORY_CARD_BUFFERS_MB=1280 VICTORY_TEXTURE_BACK=1`. Lines to start from: `the card's time by command (timing run)`, `framelog: usual frame`, `copies per frame`, `host copies in the card's memory`. |
| `timing-run-295s-2026-11-25.txt` | 280 | The build of commit `44bff61b`, diagnostics profile, with `VICTORY_GPU_TIMING=1` and `VICTORY_D3D12_DRED=1` and the same switches as above plus `VICTORY_CARD_HASH_LEN=1`: 295 s after the first world frame, played by the tester (on the water and ashore). It ends with a crash inside the title's own code (an access violation reading address 0 in a memory copy; the layer's line `unhandled exception` and the stack are at the end), not with a lost card. The title's own output lines, its crash reporter's among them, are left out of this copy. |
