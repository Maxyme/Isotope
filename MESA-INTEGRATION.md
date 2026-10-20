# Isotope Mesa integration

Implemented and locally validated 2026-10-20 on Apple M5 Max / macOS 27.2.

## Source

[Maxyme/Mesa](https://github.com/Maxyme/mesa/tree/codex/isotope-kosmickrisp) is based on athei `0b83cb23450da5ecdebf775b0ac7789f45b75748` (Mesa 27.0.0-devel). Isotope now pins tested commit `e3d0c46f52479f16da810bfbccfbc123ec1674c0`.

The four source commits separate linear attachments, wireframe, query allocation, and native testing/CI. They retain upstream import messages and include the repository's required AI trailers. The source provenance and imported licenses are committed alongside the driver.

- Linear colour/input attachment changes from Nilton's fork preserve athei's sparse-residency descriptors and 3D image views. An unused descriptor-padding slot and backend flag bit carry the 2D input view without changing descriptor size.
- Wireframe uses Metal's triangle fill mode. Polygon point mode retains the existing wireframe fallback.
- Query changes adapt Steamac's lazy occlusion allocation and process-shared, command-buffer-owned timestamp chunks. At the same time, athei's pipeline-statistics reports are retained.
- Native probes are retained with their Apache-2.0 license in the Mesa repository. The deliberately invalid R16G16B16_UINT probe request is replaced with an explicit null descriptor; the supported texel operations remain tested.

## Isotope changes

The graphics recipe no longer contains a Steamac input. The builder fetches the pinned Mesa repository directly and requires a clean matching checkout; it does not apply a separate Mesa patch stack. Staged drivers include Mesa's license, Steamac's retained license, and source-provenance metadata.

Mesa checkouts, compiler/driver builds, staged drivers and probes use revision-specific paths, preserving older artifacts. Driver fetching no longer needs Wine sources; Wine cloning is deferred until needed by a Wine build. `MESA_KK_EXPERIMENTAL=image_view_min_lod` is explicit in the recipe and carried into probe environments and assembled Vulkan runtime descriptors.

## Validation

A fresh workspace, `target/mesa-integration-verification`, successfully completed the actual Isotope CLI sequence: `dx12 fetch`, `dx12 build-driver`, `dx12 probe-driver`. Its source directory contains only the pinned Mesa checkout: no Steamac or Wine checkout was needed.

| Native check | Result |
| --- | --- |
| Required capabilities, including linear colour attachments | Pass |
| Geometry and wireframe rendering | Pass |
| Robust buffer/image access | Pass |
| Buffer device address | Pass |
| Supported texel-buffer operations and null descriptors | Pass |
| Queries, including all five previous stress failures | Pass |
| Sparse resources with explicit minimum-LOD setting | Pass, sparse D32 depth skipped |
| Linear colour/input attachment GPU readback | Pass |

The last test is imported from Nilton's fork and runs through Mesa's test runner. All eight tests passed after the final source commit. The same suite is configured in the fork's macOS 26 CI. That hosted job was still queued at the final check; local success is not a claim that hosted CI completed.

Rust formatting and strict Clippy pass. The publisher tests report 32 passed, zero failed, one existing optional test ignored. The existing recipe contract test also rejects retired Steamac settings and mistyped driver environment values.

## Remaining scope

No Wine or game was launched. Native probe success does not establish gameplay or complete Vulkan conformance. Sparse D32 images remain unsupported and their depth tests are skipped, so this does not fix or reproduce the old driver's sparse depth rendering errors. Experimental minimum LOD remains explicit. The previously working installed/tested runtime is unchanged.

The Mesa source commits are published because Isotope's pin must be fetchable. Isotope changes remain uncommitted. `MESA-INTEGRATION.json` records exact source and driver hashes, test results, and local artifact paths.
