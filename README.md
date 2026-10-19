# Isotope

Native ARM64 Wine/FEX runtimes for macOS.

Standalone Rust tooling for the pinned Tolkara ARM64 Wine/FEX implementation on
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
explicitly; build inputs and receipts must still match pinned recipes.
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
