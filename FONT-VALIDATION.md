# Font-enabled runtime validation — 2026-10-20

Implemented the startup repairs diagnosed in GAME-BLOCKERS.md. These results supersede the earlier fontless-runtime startup failures; they do not certify gameplay.

## Implementation

- Build pinned FreeType 2.14.3 from a SHA-256-verified upstream source archive. Disable optional external codec/shaping dependencies, package a signed ARM64 library with system-only dependencies, and include Wine's replacement fonts.
- Preserve FreeType notices and the complete dependency source archive in releases. Require font assets when staging a core runtime, and bind FreeType/font hashes into build provenance.
- Make actual Tahoma text measurements and an exact 480×240 client rectangle mandatory in the native and relocated-release probes. Repeat after restarting Wine. Keep the visible GDI paint probe available with `--window`.
- Relink only the build-time font converter with its private dependency path; packaged Wine/native libraries do not inherit that build rpath. Preserve bitmap `.fon` containers by using the existing object parser to select actual PE images for stripping.
- Retain the pinned Mesa fork integration and revision-separated driver build/probe outputs from the preceding investigation.
- Nucleon changes are intentionally uncommitted: short `C:\nucleon-shader-cache\<namespace>\...` aliases retain persistent caches and compatibility keys; remove delayed activation of Steam; allow the exact minimum-LOD runtime setting only for bundled KosmicKrisp.

## Verification

Isotope: 33 unit tests pass, one explicitly supplied-archive test remains ignored, strict Clippy passes. Nucleon core/runner tests and strict Clippy pass, including long-host-path cache mapping, namespace separation and preservation of conflicting existing files.

The enhanced probe rejects the previous fontless runtime with exit 11 (`FONT_METRICS_FAILED`). The rebuilt runtime reports caption=25, menu=18, small caption=17 and measured text height=16; a requested client area is exactly 480×240. These checks pass again after Wine restart, and the GDI window paints successfully.

Relocated archive/import validation passes: managed install, native integrity/dependencies, x64 CPU smoke, x86 CPU smoke, mandatory font/window metrics across restart, and prefix cleanup. The x86 test remains optional by policy (`x86_required=false`), but passed in this run. No release was published.

## Games through Nucleon

Both games were tested with fresh isolated prefixes and the repaired Wine core, the pinned `e3d0c46f52479f16da810bfbccfbc123ec1674c0` Mesa driver and the existing paired ARM64EC DXVK/VKD3D frontends.

- DiRT Rally 2.0 / DX11: completed a 180-second bounded launch, retained its game process, generated shader cache through a 93-character Wine path, and had no logged Metal assertion or `VK_ERROR`. Its persisted caption/menu metrics are sane. The former long-cache-path crash did not recur.
- DMC5 / DX12: completed a 240-second bounded launch and creates a normal 1518×962 swapchain. The previous 32859×2 request and Metal size assertion did not recur. VKD3D still warns that `VK_EXT_dynamic_rendering_unused_attachments` is missing; this remains a graphics compatibility limitation.

The user confirmed **no game window appeared for either repaired-runtime run**. Both games therefore fail visible launch validation. Menu, input, driving and combat were not reached. A live process, shader cache, normal swapchain or bounded exit zero is not a gameplay pass. The font/geometry and cache fixes remain verified independently; native window visibility/activation needs further investigation.

DMC5 settings were restored byte-for-byte. The previous runtime driver is unchanged. User prefixes/saves were preserved; only isolated test prefixes were regenerated. Full receipts, hashes and paths are recorded in FONT-VALIDATION.json. Logs remain under target/font-validation/.

Follow-up: the runs automatically closed after 180/240 seconds. Confirmation of whether the user watched during those intervals is pending. Native UI inspection also fails for Steam with ScreenCaptureKit error -3811, so the capture-tool failure itself is not specific evidence about Wine window registration.
