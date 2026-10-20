# Unpatched Mesa / KosmicKrisp evaluation

Evaluated 2026-10-20 on Apple M5 Max, macOS 27.2.

## Decision

Keep the patched KosmicKrisp driver for the current Isotope runtime. Both unpatched builds compile, but neither provides the tested graphics compatibility or launches DiRT Rally 2.0 successfully through Nucleon. Updating to current upstream main does not resolve the blockers.

This does **not** establish that every Steamac patch is necessary. The next reduction experiment should isolate a minimal compatibility patch set, retaining functional probes and the game test as gates. Simply deleting the Steamac recipe entry currently removes both patch and probe sources and breaks the build workflow.

## Sources and method

- Existing Mesa base, unpatched: `ce576c29ed90822ea15564e1e60cb48e455881bb` (26.3.0-devel).
- Upstream main fetched from the official Mesa repository: `7f13762f3fae332deee4c58b3ac00d1ac7a45153` (27.0.0-devel).
- Both source checkouts were clean after their builds.
- Baseline: existing Mesa base with Steamac `6ef3992d869e41291bb2841bef2e71ae318d2a02` patches.
- Release builds with native clang, KosmicKrisp/macOS only, Mesa CLC/precomp tools, LLVM disabled in the driver, deployment target macOS 26. Current main used its matching CLC build; the pinned version reused the existing matching CLC tools.
- Identical existing native probes and shaders across all three drivers, with Metal validation enabled (`MTL_DEBUG_LAYER=1`, error mode `assert`) and the ICD explicitly selected.
- Each candidate runtime cloned the previously game-tested Wine 11.20 / FEX-2610 runtime, replaced only KosmicKrisp, and updated the assembly manifest hashes. Separate prefix families and shader caches avoided shared state. Both passed `nucleon wine inspect`.
- Nucleon launched DiRT Rally 2.0, appid 690790, through DXVK/DX11 in each candidate runtime with a 90-second runtime bound. No DX12 game validation was performed in this experiment.

## Native results

| Probe | Patched baseline | Unpatched pinned base | Unpatched upstream main |
| --- | --- | --- | --- |
| Required capability gate | Pass | Fail | Fail |
| Geometry shader | Pass | Feature unavailable | Feature unavailable |
| Robust access | Pass | Pass | Pass |
| Device address | Pass | Pass | Pass |
| Texel buffer | Pass | Alignment failure | Alignment failure |
| Queries | Pass | Metal validation abort | Metal validation abort |
| Sparse resources | Fail, two depth errors | Unsupported features | Unsupported features |

Both unpatched drivers report `geometryShader=0`, `transformFeedback=0`, `fillModeNonSolid=0`, and no linear colour attachment support for the probe's selected formats. Geometry device creation returns `VK_ERROR_FEATURE_NOT_PRESENT`. The capability gate reflects this runtime's requirements; it is not a general Vulkan conformance verdict.

The texel probe reports no single-texel alignment support (16-byte storage and uniform alignment). The query stress test aborts because Metal rejects a counter heap entry count greater than 4096. This is a reproducible probe regression under validation, not evidence that every game hits that path.

Sparse failures also exist in the patched baseline: D32 sparse depth test/write produces 16,384 incorrect colours and depths among 65,536 pixels, in both one- and two-render-pass cases. Unpatched drivers fail earlier because sparse capabilities are unavailable. The baseline must not be described as fully passing the suite.

## Nucleon and DiRT results

Both unpatched candidate launches log:

```text
DXVK: No adapters found. Please check your device filter settings
and Vulkan drivers. A Vulkan 1.3 capable setup is required.
EXCEPTION_ACCESS_VIOLATION exception (code=c0000005) raised
```

Neither achieved confirmed gameplay. The native probes independently show missing functionality; the DXVK warning alone does not identify every adapter-rejection requirement. The launcher reports a completed bounded run and exit code zero because it reached its timeout and cleaned up; this is **not** a successful game result.

The patched baseline's menu/input success was confirmed by the user in the preceding upstream Wine/FEX validation, not retested visually during this experiment. Its driver hash remains unchanged: `1cd31de2e7d51792aba62ebececf001cca7c30b038f6e7b19d0968bb53cbdf40`.

An initial pinned launch used a relative work path and failed when the diagnostic path was resolved from the game directory. It was retried with absolute paths. That harness failure is excluded from the Mesa result.

## Evidence and reproduction

`MESA-EVALUATION.json` records exact driver hashes, per-probe exit codes, and game receipt paths. Local build trees, source checkouts, full logs, candidate runtimes, and evaluation scripts are retained under `target/mesa-unpatched-evaluation/` (ignored build artifacts). The scripts reproduce this local setup and reference the existing SDK, probe binaries and working runtime; they are not a standalone public build interface.

No runtime recipe, default selection, or working driver was changed. No commits or pushes were made by this evaluation.
