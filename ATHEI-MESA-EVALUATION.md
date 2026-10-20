# athei/Mesa native evaluation

2026-10-20, Apple M5 Max, macOS 27.2. No Wine or game launches were performed.

## Outcome

**Build succeeds without local changes. This is a credible Mesa-only source, but not yet a drop-in replacement for Isotope's working Steamac-patched driver.** The remaining blockers are unsupported wireframe, missing linear colour attachments for the tested formats, and five failing query stress cases.

The raw texel-probe crash is attributable to a deliberately invalid request, not a failure in the supported texel operations. A diagnostic probe that substitutes an explicit null descriptor for that invalid request passes. Sparse testing passes with the driver's experimental minimum-LOD feature enabled, but skips unsupported sparse D32 depth images. Those distinctions matter when comparing drivers.

## Source and build

- Repository: [athei/mesa](https://github.com/athei/mesa/tree/macos-patched).
- Branch: `macos-patched`; exact revision: `0b83cb23450da5ecdebf775b0ac7789f45b75748`.
- Mesa version: `27.0.0-devel`.
- Matching Mesa CLC/precomp tools built first with LLVM, followed by a release arm64 KosmicKrisp build with runtime LLVM disabled and deployment target macOS 26.0. The same build settings/toolchain as the previous evaluations were used.
- Driver signature verifies and its imports are system libraries/frameworks only; no MoltenVK or Homebrew runtime dependency.
- Driver SHA-256: `4784572249e77083899a6767c8c59fb9980d03e5100934f974adac31a735c56a`.
- Source checkout remains clean. No patches, source fixes, feature spoofing, Wine changes or recipe changes were made.

## Identical baseline probes

The same existing probe binaries and shaders used for the unpatched and SteamARM evaluations were run with explicit ICD selection and Metal validation enabled (`MTL_DEBUG_LAYER=1`, error mode `assert`). Their required-feature gates are runtime policy checks, not general Vulkan conformance verdicts.

| Check | Result |
| --- | --- |
| Capability gate | Fail: linear colour attachment support missing |
| Geometry suite | Geometry-shader cases pass; unsupported wireframe case fails |
| Robust access | Pass |
| Buffer device address | Pass |
| Texel buffers, original probe | Crashes on deliberately invalid R16G16B16_UINT view |
| Query suite | Five failing stress cases |
| Sparse suite, default settings | Four missing minimum-LOD feature checks |

The driver advertises geometry shaders, depth-clip control, transform feedback and geometry streams. Transform-feedback advertisement was observed; this experiment does not independently establish complete stream-output correctness. `fillModeNonSolid=0` remains a DXVK compatibility concern. The line-mode probe produces filled interiors; because wireframe is explicitly unsupported, that result is not a conformance claim about supported usage.

### Texel crash investigation

An unchanged copy of the original probe was rebuilt with debug symbols. LLDB identifies the crash in `kk_CreateBufferView`, called with `VK_FORMAT_R16G16B16_UINT`, offset 0, range 48, at the original probe's line 416. That probe explicitly labels the operation invalid usage and expects Steamac-specific null-descriptor handling.

Before the crash, single-texel alignment, uniform/storage loads and stores, atomics, bounds handling, descriptor arrays and push descriptors pass. A separate diagnostic copy replaces only the invalid view creation with `VK_NULL_HANDLE` and adjusts its output labels. All remaining checks, including supported R32G32B32 formats, then pass. The driver itself and original probe were not changed. The diagnostic source and diff are retained with the artifacts.

### Query failures

The 8192-entry timestamp pool works, resolving the upstream single-heap abort. Five broader checks fail:

- 48 one-query timestamp pools on two devices: only 30 created.
- Occlusion pools of 32760, 8 and 40000 queries: only one created.
- 300 occlusion pools of 4096 queries: only seven created.
- 40 timestamp pools of 8192: only 15 created.
- 16 back-to-back timestamps: only four available.

The existing Steamac-patched baseline passes these checks. This fork's split-per-pool timestamp heaps do not remove the broader process-wide heap pressure tested here.

### Sparse follow-up

Setting `MESA_KK_EXPERIMENTAL=image_view_min_lod` makes the sparse probe exit zero, without source changes. However, the log explicitly says **D32 sparse images are unsupported; sparse depth is not tested**. This must not be presented as fixing the working Steamac driver's two sparse depth failures: those cases were not exercised here. The experimental feature's broader correctness/performance is not established by this one probe suite.

## Recommendation

Keep this fork as a candidate foundation for reducing the Steamac checkout dependency. Before selecting it for Isotope, address wireframe, linear colour attachments and query scalability, then rerun the native gates. Do not port SteamARM's entire patch stack into it without identifying overlap and necessity. A later Nucleon/game evaluation remains required and is currently on hold at the user's request.

The known-working driver hash is unchanged: `1cd31de2e7d51792aba62ebececf001cca7c30b038f6e7b19d0968bb53cbdf40`. No production recipe changes, registrations, commits or pushes were made.

## Local evidence

`ATHEI-MESA-EVALUATION.json` contains source/build metadata and results. All artifacts are under ignored `target/mesa-unpatched-evaluation/`: `sources/athei`, `clc-athei`, `athei-build`, `driver-athei`, `logs/athei`, and the diagnostic texel-probe source/diff. The local build helper references the existing workspace toolchain; it is not a standalone public build interface.
