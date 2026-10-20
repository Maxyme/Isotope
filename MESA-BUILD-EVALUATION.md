# Plain Mesa / KosmicKrisp build investigation

2026-10-20, Apple M5 Max. Scope after the user's latest instruction: build and native Vulkan probes only; no further DiRT launches.

## Plain upstream build: successful

KosmicKrisp is part of Mesa, not a separate project to fetch or build on top of an installed Mesa. A driver-only build selects `-Dvulkan-drivers=kosmickrisp -Dplatforms=macos`, with OpenGL and Gallium drivers disabled. Mesa's standard [KosmicKrisp build documentation](https://docs.mesa3d.org/drivers/kosmickrisp.html) describes this path and requires Metal 4 / macOS 26 or newer.

Built pristine **Mesa 26.2.4**, matching SteamARM's base for a direct comparison. The release archive SHA-256 is `bce5f7fbebb934373b86c999a064d52fb5065878dc57f287f95346648ec832e9`. After building, all 12807 original regular source files were compared against the verified release archive and match exactly. No third-party patches or local fixes were applied.

The build has two stages:

1. Build and install matching `mesa_clc` and precomp compiler tools, using LLVM at build time.
2. Build the release KosmicKrisp driver with `mesa-clc=system`, `llvm=disabled`, `spirv-tools=disabled`, `platforms=macos`, and `vulkan-drivers=kosmickrisp`. Disable OpenGL, Gallium, GLX, EGL, GBM and Zstd. Set deployment target macOS 26.0.

This is the same general build configuration already used by Isotope. Upstream does not need Steamac, Wine, FEX or MoltenVK to build the driver. The probes are separate test programs; reusing the existing Steamac-origin probes does not patch the driver.

Output: `target/mesa-unpatched-evaluation/driver-plain-26.2.4/`, containing the arm64 dylib, relative-path ICD manifest, Mesa license and build metadata. Ad hoc signature verification passed. The dylib links only system libraries/frameworks; there is no runtime dependency on LLVM, Homebrew libraries or MoltenVK. A Vulkan loader is still needed by a Vulkan application to load this ICD.

Driver SHA-256: `c2af88ad9b43623d59f80d1a304cb9a0160e9ddee4a83c48096eef06f94ea5d2`.

## Full SteamARM patch set: also builds

Before the plain-build request, all **27** SteamARM KosmicKrisp patches at `f8f729aa0cd4bf669210f18bd21d19629212da0c` were applied to a separate Mesa 26.2.4 extraction. Every patch applied without rejected hunks, offsets or fuzz, and the driver built without local source changes. This includes geometry, query, memory, performance and MetalFX changes; it is not solely a compatibility patch set.

Its matching compiler tools were built separately. The final driver also has no runtime LLVM dependency, but adds the system MetalFX framework. It was tested directly, without SteamARM's feature-advertising shim.

## Native comparison

All drivers use identical offscreen probe binaries and shaders, explicit ICD selection, and Metal validation (`MTL_DEBUG_LAYER=1`, error mode `assert`). These probes check the needs of this runtime; their feature gates are not general Vulkan conformance verdicts.

| Check | Plain 26.2.4 | SteamARM 26.2.4 + all 27 patches | Current Steamac-patched driver |
| --- | --- | --- | --- |
| Build | Pass | Pass | Pass, prior build |
| Capability gate | Fail | Fail | Pass |
| Geometry suite | Feature unavailable | Two rendering checks fail | Pass |
| Robust access | Pass | Pass | Pass |
| Buffer device address | Pass | Pass | Pass |
| Texel-buffer alignment | Fail | Fail | Pass |
| Query suite | Metal validation abort | Four checks fail; no abort | Pass |
| Sparse resources | Required features unavailable | Required features unavailable | Two existing sparse depth errors |

Plain 26.2.4 reports `geometryShader=0`, `fillModeNonSolid=0`, `depthClipEnable=0`, `transformFeedback=0` and no linear colour attachment support for the tested formats. Its timestamp query probe aborts because a Metal counter heap cannot exceed 4096 entries. The unpatched pinned 26.3 development base and upstream 27.0 development revision were already built in the preceding [evaluation](MESA-EVALUATION.md); newer upstream builds still have compatibility gaps, so successfully compiling an older plain release does not resolve them.

SteamARM supplies geometry support and its 8192-entry timestamp pool works, but geometry line primitive IDs fail a rendering check. Its wireframe test produces filled output; wireframe is explicitly unsupported, so this is evidence of missing behaviour rather than a conformance claim for supported usage. Four query checks fail: too many independent timestamp pools, a large occlusion-pool stress case, many 8192-entry timestamp pools, and back-to-back timestamp availability. Depth clip, transform feedback, linear colour attachments and sparse features remain unavailable.

## Implications for Isotope

A plain driver build is straightforward and useful as a baseline. The existing Isotope command currently imposes Steamac through `prepare_mesa`, fetching, provenance, license and patch packaging, and probe-source lookup; these are build-tool policy choices, not upstream build requirements.

If adding an upstream-only build mode, use an explicit patch selection with an empty list for plain Mesa, record that empty list in provenance, and keep probe sources independent. Keep patched and unpatched source/build directories separate. That mode should build a driver without claiming it passes the runtime's compatibility gate.

The plain build is currently unsuitable as a replacement for the known-working gaming driver. Do not switch the production recipe based on build success alone. Likewise, the complete SteamARM stack is not equivalent to the current compatibility stack, despite being smaller in patch bytes (347,904 vs 725,683).

No further Wine/game launch was performed after the user asked to hold DiRT. An earlier SteamARM launch had already started and ended; it is excluded from this build-only comparison and supplies no gameplay verdict here. The working runtime and production recipe are unchanged.

## Evidence

`MESA-BUILD-EVALUATION.json` records build metadata, exact SteamARM input hashes, per-probe results, source verification and driver hashes. Full sources, patches, compiler tools, driver outputs, logs and local build scripts are retained in ignored `target/mesa-unpatched-evaluation/`. The local scripts reference this workspace's existing toolchain and are not a standalone public build interface. Nothing was committed or pushed.
