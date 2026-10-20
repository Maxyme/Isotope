# Isotope

Native ARM64 Wine/FEX runtimes for macOS.

Standalone Rust tooling for the pinned upstream Wine 11.20 / FEX-2610 implementation on
Apple Silicon macOS. This repository owns compilation, patches, experimental
CPU/graphics probes, runtime assembly and source-inclusive release publishing.
Nucleon owns installation, ABI/integrity checks, Steam integration and game
launching. This builder has no Cargo dependency on Nucleon or a sibling checkout.

## Build and validate

Install Rust, Xcode Command Line Tools, Bison/Flex, CMake, Ninja and Python 3.9+
(required by upstream FEX). The optional graphics experiments also need Meson,
its pinned Python dependencies and the SDK specified by their recipe.

```sh
cargo build --release --locked
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings

target/release/isotope fex fetch
target/release/isotope fex doctor
target/release/isotope fex build-fex
target/release/isotope fex build-wine
```

`--work` defaults to `target/arm64-fex`. Existing local workspaces can be supplied
explicitly; build inputs and receipts must still match pinned recipes. The upstream
migration uses `core/`, `sources/wine`, `sources/fex` and `sources/shared-data`; old
Tolkara workspaces remain on disk and are not adopted or deleted. Use a fresh
`--work` for this version.

Wine configuration is recorded in `wine-build/isotope-configure.json` and copied
into runtime provenance. Reuse requires the same source path/revision/diff,
patches, configuration arguments, compiler identities, toolchain pin, SDK identity
and explicit configure environment. Unknown shell variables and user Autoconf
site scripts are excluded from configure. A changed input, missing/corrupt receipt
or incomplete build directory stops before compilation; use a fresh `--work`
directory. Existing build products are preserved. Pre-receipt workspaces must also
start fresh; Isotope never adopts their Makefile as proof of configuration.

Builder recipes reject unknown fields, misspelled source/archive slots and wrong
value types. Historical unused recipe settings have been removed. Persisted Wine,
FEX companion, frontend and source-overlay receipts use typed Serde contracts;
release validation uses explicit status enums and typed guest outcomes. Unsupported
receipt schemas, missing required fields and malformed manifests fail before they
can authorize assembly or packaging. Historical schema-less component receipts
require rebuilding the affected components; they are never implicitly upgraded.
JSON errors include the input filename.
Nucleon's independently versioned inspection response accepts additional fields
while still requiring its schema and runtime fields to have the expected types.

`fex probe`, `fex probe-unixlib`, `fex check-reference` and the `dx12`/`frontend`
commands retain the experimental probes and assembly workflows. Reference app
components are local diagnostics, never inputs to the core release publisher.
GPU compiler DLLs must be explicitly supplied with `NUCLEON_PROBE_D3DCOMPILER`.
The Steam-route probe and game launch wrappers remain in Nucleon's devtools.

Package validation uses a separately built Nucleon executable through its public
CLI, not duplicated installer/ABI code. Supply `--nucleon /path/to/nucleon` or
`ISOTOPE_NUCLEON`; otherwise `nucleon` is resolved from PATH. It must support
`wine install` and schema-1 `wine inspect --json`. Build and fetch commands do
not need Nucleon. Assembly invokes its descriptor validator. Release validation
installs into private support storage, verifies the installed files and native
dependencies, then runs CPU probes from that relocated installation.

```sh
target/release/isotope --nucleon ../Nucleon/target/debug/nucleon \
  release build --version 0.1.0-rc.1 --output target/release-assets
```

`release package` requires unchanged sources/products and the publisher build
receipt. `release validate` exercises existing built inputs without creating
publishable release assets. Its new output directory retains logs and a
`diagnostic-runtime.tar.gz` for local installation tests. This diagnostic archive
has no corresponding-sources bundle and is not a distribution release.

## Temporary 32-bit policy

**Both x64 and x86 tests always run.** x64, archive import/integrity/native closure,
and server cleanup must pass. A failed x86 test is temporarily non-blocking,
recorded as `FAILED_OPTIONAL` with its error and log; overall validation becomes
`PASSED_WITH_OPTIONAL_FAILURE`.

The original `CreateProcessW` error 998 was traced to a missing image-name pointer
conversion in Wine and is corrected by the downstream patch described in
[WOW64-FIX.md](WOW64-FIX.md). The preview policy remains optional until deliberately
changed; `--require-x86` still forces the check. CPU tests do not qualify gameplay.

Pass `--require-x86` to any `release` action to restore mandatory 32-bit validation.
Interruption still aborts publication. This temporary policy is visible in
release metadata and notes; failures are not reported as test passes.

## Releases and Nucleon installation

Core artifacts contain native Wine, both FEX PE emulators and Unix companions,
built-in WineD3D, provenance and notices. They exclude DXMT/DXVK/VKD3D, Vulkan
loaders/drivers, Steam libraries, Apple GPTK and Microsoft game DLLs.

Each release includes:

- `isotope-<version>-macos-arm64.tar.gz`, with `nucleon-runtime.toml` schema 1.
- Matching patched Wine/FEX/helper and builder sources, initialized pinned
  submodules, per-file source manifest, build scripts and notices.
- `runtime-release.json` schema 2, `SHA256SUMS.txt`, release notes and validation logs.

Nucleon's named source resolves GitHub release tag `v<version>` in
`Maxyme/isotope`; it checks the exact version, asset filename and SHA256 before
using its normal immutable-generation importer. No automatic latest update.

```sh
nucleon wine install isotope --version 0.1.0-rc.1 --register
nucleon wine install ./isotope-0.1.0-rc.1-macos-arm64.tar.gz --register
nucleon wine install https://github.com/Maxyme/isotope/releases/download/v0.1.0-rc.1/isotope-0.1.0-rc.1-macos-arm64.tar.gz --register
```

A local directory is also accepted. URLs use adjacent `runtime-release.json`
unless `--sha256` is supplied. Local archives can optionally supply a checksum.
`--register` affects only the experimental FEX selection, leaving Intel Wine alone.

The workflow accepts a full Nucleon commit through manual input `nucleon_ref` or
repository variable `NUCLEON_VALIDATOR_REVISION` for tags. This pin must contain
the consumer CLI described above. Configure it after the consumer change is
committed and pushed. Tags `v*` produce prereleases; manual dispatch only produces
CI artifacts. The local repository creation does not create a GitHub repository,
push tags or publish assets.

## Provenance and licenses

The builder was extracted from Nucleon commit b171067, including its tested
process ownership and bounded artifact helpers. The small portable helpers now
belong to this independent builder; runtime acceptance policy remains exclusively
in Nucleon. Wine/FEX and bundled toolchain notices retain their original terms.
Builder code retains GPL-3.0-only; Wine retains LGPL-2.1-or-later and FEX MIT.
Corresponding patched compilation inputs and build scripts accompany releases.
See LICENSE, NOTICE.md and the source archives for attribution and rebuild inputs.

## Local split verification (2026-10-19)

18 standalone tests and strict Clippy/formatting passed. Packaging existing pinned
build products, real Nucleon CLI import/verification, x64 execution and cleanup
passed. x86 reproduced error 998 and was recorded as optional; validation exited
zero. Direct archive installation/registration, checksum rejection and Intel
selection preservation passed in an isolated home. No Steam/game launch, fresh
full source build or remote release run was performed. See VALIDATION.json.

The split verification above predates the 32-bit process fix. The current
[WoW64 fix](WOW64-FIX.md) and [verification record](WOW64-VALIDATION.json) show both
guests passing a strict local `--require-x86` validation run.

## Upstream migration (2026-10-20)

Wine is pinned to the official 11.20 release and FEX to FEX-2610. The builder no
longer checks out either Tolkara fork. Three reviewed patches carry the Darwin
TEB slot, guest address window, MAP_JIT pool and shared-page/Steam/graphics glue.
Wine 11.20's existing syscall wrappers replace the older generated wrapper layer;
FEX is selected through Wine's registry configuration rather than loader changes.
The iPad hosting, aliased-JIT and application-specific fork changes are omitted.
The separate graphics recipe no longer adds exit/message tracing patches.

Both guest workloads pass with a fresh prefix, including 32-bit child process
creation; the x64 shared-page/protection and self-modifying-code checks pass.
Nucleon accepts the assembled Vulkan runtime. DiRT Rally 2.0 launched with DXVK
and KosmicKrisp, with menu and input confirmed by the user. No CrossOver, GPTK or
D3DMetal components were used. Core release contents still exclude graphics and
Steam binaries; game validation used the separately assembled graphics runtime.
A second fresh build through the Isotope CLI passed the real Nucleon managed
archive import, integrity/native dependency audit, both guest tests and cleanup
with `release validate --require-x86`. 32 Rust tests, strict Clippy and formatting
pass; one archive-fixture test remains intentionally ignored. The DiRT-tested
graphics runtime is retained locally in `target/upstream-game-runtime`.
See [UPSTREAM-VALIDATION.json](UPSTREAM-VALIDATION.json) for the precise scope.

### Mesa driver source

The graphics recipe pins [Maxyme/Mesa](https://github.com/Maxyme/mesa), based on
`athei/mesa` with focused wireframe, linear attachment and query allocation fixes.
Isotope fetches this single Mesa source and does not clone or apply patches from
Steamac. Native probes and their licenses live with the Mesa source. Mesa
checkouts and compiler/driver build directories are separated by source revision.

`MESA_KK_EXPERIMENTAL=image_view_min_lod` is explicit in the graphics recipe and
is applied to native probes and assembled Vulkan runtimes. The fork's macOS CI
runs the seven native probe groups and a linear attachment GPU readback test.
Sparse D32 images remain unsupported and their depth cases are skipped; polygon
point mode retains the wireframe fallback. Native tests are not game validation.

## Font support and startup validation

Wine requires the pinned FreeType source archive in the core recipe. Isotope builds
an ARM64 shared library with optional external codec/shaping dependencies disabled,
packages it beside Wine's native modules, and includes Wine's replacement fonts.
No Homebrew font library is needed at runtime. The archive checksum, library/font
hashes, upstream license texts and complete dependency source archive accompany
binary/source releases.

`fex probe` now always checks real Tahoma text metrics and an exact 480×240 client
rectangle, then repeats after restarting Wine. `--window` additionally exercises a
visible GDI window. Relocated release validation requires the same checks; x86
CPU testing remains optional unless `--require-x86` is selected.

Changing from the old fontless recipe requires a fresh build workspace. Test with
fresh prefixes: old prefixes may contain corrupt WindowMetrics and cached theme
metrics. Preserve user prefixes and saves; resetting the registry alone cannot
repair a runtime that still has no font backend.
