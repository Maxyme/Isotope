# Nucleon game validation — 2026-10-20

Historical pre-fix evidence. See [FONT-VALIDATION.md](FONT-VALIDATION.md) for the implementation and repaired-runtime results.
Tested Isotope with Mesa `e3d0c46f52479f16da810bfbccfbc123ec1674c0` using the current Nucleon CLI and runner. Neither game passed visible launch validation. The user confirmed that no DiRT game window appeared on either the initial launch or the short-path retry. Wine windows could not be attached by the UI tool.

## DiRT Rally 2.0 — DirectX 11 / ARM64EC DXVK 3.1

The first launch crashed in `d3d11.dll` at RVA `0x385124`, a trap in `dxvk::util::Win32File::Win32File`. The DXVK cache directory alone was 256 characters; its filename exceeds the fixed `MAX_PATH + 1` conversion buffer in DXVK 3.1. A second launch using an environment override reproduced this because the diagnostic launcher clears inherited overrides.

Retried through Nucleon using `/private/tmp/ig-dirt` as the work directory. This reduced the cache directory to 202 characters. The game process stayed alive for 119.673 seconds, created approximately 9.4 MB of DXVK shader cache, did not spawn its crash reporter, and logged no `VK_ERROR`, DXVK `err: `, or Metal assertion. The test was then deliberately stopped through the runner for cleanup, so its receipt correctly says `cancelled`; this is not an exited-zero or bounded gameplay pass. The user confirmed that no game window appeared on either attempt, including the short-path retry. DiRT therefore failed visible launch validation despite its live process and shader-cache activity; menu, input, rendering quality, and driving were not reached.

The long generated cache path is a Nucleon/DXVK integration issue. A permanent fix should expose managed caches through a short Wine path while retaining their graphics compatibility namespace. The short test directory is only a validation workaround.

## Devil May Cry 5 — DirectX 12 / ARM64EC VKD3D-Proton 3.0.1

The first launch initialized VKD3D and Shader Model 6.6, then requested a `32859 x 2` swapchain. Metal rejected width 32859 because the maximum is 32768, and Wine exited with status 1.

A retry with a short work directory and temporary `WindowMode=Normal`, `FullScreenMode=false`, and `FullScreenDisplayMode=0` reproduced the identical dimensions and assertion. A control run with the previous Steamac-patched driver also reproduced the identical failure and exit status 1. Therefore this failure is not specific to the new Mesa driver; the originating bad dimensions in the game/DXGI/VKD3D/Wine path remain to be traced. Windowed settings did not resolve it. DMC5 did not pass menu or gameplay validation.

VKD3D also reports missing `VK_EXT_dynamic_rendering_unused_attachments`, which it says is required for correct operation. This is an additional compatibility concern, not established as the cause of this assertion.

## Consumer compatibility and preservation

Nucleon initially rejected the new runtime's `MESA_KK_EXPERIMENTAL=image_view_min_lod` descriptor environment setting. Its validator now accepts exactly that value only when the runtime declares KosmicKrisp and includes a bundled Vulkan ICD. Ten focused runtime tests pass, strict core Clippy passes, and the CLI/devtools/runner rebuilt successfully.

DMC5 settings were restored byte-for-byte (SHA-256 `66eaae2c9280db9c73de8e7d245e9d09723230c9fc97322fddfa7a91175bbff3`). The original runtime driver remains unchanged (SHA-256 `1cd31de2e7d51792aba62ebececf001cca7c30b038f6e7b19d0968bb53cbdf40`). Game tests used isolated prefixes and caches. No commits or pushes were made.

See `MESA-GAME-VALIDATION.json` for machine-readable receipts, runtime hashes, restoration checks, and paths. Logs and launch receipts are retained under `target/mesa-game-validation/evidence/`.
