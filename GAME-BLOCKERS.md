# Game startup blockers — 2026-10-20

Historical pre-fix evidence. See [FONT-VALIDATION.md](FONT-VALIDATION.md) for the implementation and repaired-runtime results.
The first fix is the Wine runtime's missing font backend, followed by prefix repair and Nucleon's cache path. The current evidence does not justify replacing Mesa to solve these startup failures. Neither game is certified playable.

## 1. Enable and package FreeType and usable fonts (confirmed runtime defect)

`assets/wine/recipes/arm64-fex.toml:35` explicitly sets `--without-freetype` and `--without-fontconfig`. The actual build's `include/config.h` leaves `SONAME_LIBFREETYPE` undefined, so this is not just an unused recipe setting. The runtime contains no `.ttf`/`.fon` files and `share/wine/fonts` is absent, although the Wine source includes its replacement Tahoma and other fonts.

A small x64 Windows probe executed through the same FEX runtime reports:

```text
ncm_ok=1 caption=32759 small=32759 menu=32759 fontHeight=-13 screen=1512x982
text_metrics_ok=0 height=1431655765 ascent=1431655765 descent=1431655765 external=1431655765
adjust_1280x720=-4,-32764,1284,724
created=<HWND> client=32857x0
```

1431655765 is the probe's `0x55555555` sentinel, not a real font measurement: the failed text-metrics API leaves its output untouched. Both games' prefixes persist CaptionHeight, MenuHeight and SmCaptionHeight as 32759. Resetting those registry values to Wine defaults in a backed-up disposable prefix did not cure the failure; the same bad metrics recurred. This is an active initialization defect, not solely stale registry data.

Source chain:

- `wine/dlls/win32u/freetype.c:3913`: without FreeType, `init_freetype_lib()` returns NULL.
- `wine/dlls/win32u/font.c:6750`: font initialization returns without a font backend.
- `wine/dlls/win32u/driver.c:370`: fallback text-metrics implementation returns FALSE without writing output.
- `wine/dlls/win32u/sysparams.c:5467`: metric retrieval failure returns zero.
- `wine/dlls/win32u/sysparams.c:5492`: the caller ignores that failure, leaving its TEXTMETRIC uninitialized.
- `wine/dlls/win32u/sysparams.c:5531`: normalization reads the invalid height to enlarge menu/caption metrics.
- `wine/dlls/uxtheme/system.c:404,492`: theme setup gets and persists those metrics. The ThemeManager NonClientMetrics backup is poisoned too.

**Fix:** build Wine with native ARM64 FreeType, bundle its runtime dependencies and Wine's font assets, and ensure installed font discovery works. Fontconfig is optional on macOS; do not add it solely to fix this. Fail the build or validation if the font backend is absent. A defensive Wine error-handling patch should also initialize/check failed metrics, but cannot replace a working font backend.

After that fix, regenerate disposable prefixes. For existing user prefixes, back up and repair both WindowMetrics and cached ThemeManager metrics; preserve saves and restart the Wine server. Do not delete production prefixes as part of this investigation.

## 2. DiRT window visibility: corrupt geometry, plus a focus issue to isolate

Focused macdrv traces show a main window initially reaching `(240,0)-(33105,33536)` with its client top at 32764. This independently matches the bad caption metrics. It later switches to a sane fullscreen 1512x982 surface and repeatedly calls `macdrv_client_surface_present`. A native thread sample shows the Cocoa event loop, DirectInput/audio work and graphics activity. Thus process and shader activity do not establish visible gameplay, but the new driver is not simply failing to create/present every surface.

The user confirmed no game window appeared during either earlier launch. That observation remains authoritative; this later trace has no new visual confirmation.

`Nucleon/crates/nucleon-runner/src/lib.rs:265` also explicitly activates **Steam** after a 2.5-second delay (`:1181`), rather than the game. This is a confirmed focus-target problem and a candidate contributor to a hidden game. Its causal effect on the reported invisibility is not yet proven. Repair geometry first, then remove or correctly target that activation and verify actual on-screen visibility/input.

## 3. Make managed shader-cache paths safe (confirmed independent crash)

The original generated DXVK cache directory was 256 characters before adding its filename. DXVK 3.1 `src/util/util_file.cpp:41-47` uses a fixed `MAX_PATH + 1` conversion buffer and writes the terminator at the returned length. The DiRT crash dump points to its bounds trap. A short work directory avoided that crash.

**Fix in Nucleon:** retain persistent graphics compatibility keys, but expose each managed cache through a short path inside the Wine prefix (or a managed Wine drive). Shortening only the SHA digest or adding a Windows long-path prefix does not solve an arbitrarily long host directory and this fixed buffer. Add a long-host-path regression test that exercises a real cache-file create/open. The devtools launcher currently strips inherited cache overrides, so an environment-only test override was ineffective.

## 4. DMC5: retest geometry before modifying Mesa

Both the new and previous Mesa drivers reproduce a `32859 x 2` swapchain and Metal's width-limit assertion. Temporary windowed settings did not fix it.

This log prints the **incoming DXGI descriptor**, before Vulkan surface creation: VKD3D `libs/vkd3d/swapchain.c:3955`. DXVK `src/dxgi/dxgi_factory.cpp:532-563` copies the incoming descriptor and fills zero dimensions from GetClientRect, then forwards it. The standalone probe's similarly oversized/zero client geometry provides a concrete common failure mechanism. The exact game's descriptor/GetClientRect transition has not been traced, so font repair is the first necessary fix, not a proven complete DMC5 cure.

If the bad descriptor survives a font-correct runtime and clean prefix, log original DXGI input dimensions, GetClientRect, the forwarded descriptor, and VKD3D input. No source-level mismatch was found in the ARM64EC private factory signatures/layouts. Reject invalid resource sizes with an error rather than clamping dimensions or permitting a Metal assertion.

VKD3D separately warns that `VK_EXT_dynamic_rendering_unused_attachments` is missing. Sparse-depth support also remains incomplete. These need graphics validation after normal windows and swapchain dimensions work; enabling an extension flag without implementing its behavior is not a fix.

## Verification order

1. Runtime dependency/font inventory: FreeType enabled and packaged, usable replacement fonts discoverable.
2. Font probe: GetTextMetricsW succeeds with sane positive dimensions; nonclient heights stay small across theme reload and Wine restart.
3. Window probe: known-size window produces a nonempty, correctly sized client rectangle; confirm visibility and input.
4. Real cache file open with a deliberately long host path, using Nucleon's short Wine alias.
5. DiRT DX11 and DMC5 DX12 menu/input checks, then actual driving/combat; verify renderer errors and frame output.

Evidence, probe source/executable, macdrv trace, runner receipt and thread sample: `target/mesa-game-validation/evidence/window-font-investigation/`. `GAME-BLOCKERS.json` records machine-readable results and source paths. No permanent code changes, commits or pushes were made during this investigation. Production game settings and the working runtime were preserved; the disposable registry experiment was backed up and restored.
