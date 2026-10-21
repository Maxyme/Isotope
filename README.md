# Isotope

Build native ARM64 Wine/FEX runtimes for Apple Silicon macOS.

Isotope compiles pinned Wine 11.20 and FEX-2610 sources with a Darwin compatibility
patch set, bundles FreeType and Wine fonts, and produces runtime archives with
corresponding sources. Optional graphics workflows build Mesa/KosmicKrisp and
paired DXVK, VKD3D-Proton or DXMT frontends.

[Nucleon](https://github.com/Maxyme/Nucleon) installs and validates runtimes,
integrates with Steam and launches games. Isotope has no Cargo dependency on
Nucleon or a sibling checkout.

## Build

Use a native Apple Silicon host with Rust, Xcode Command Line Tools, Bison, Flex,
CMake, Ninja and Python 3.9+ for FEX. Graphics builds also use LLVM, Meson, pinned
Python packages and a Vulkan SDK. The default SDK path is
`~/VulkanSDK/1.4.363.0/macOS`; graphics commands accept `--sdk` to override it.
The frontend recipe pins the native LLVM version required by the DXMT experiment.

```sh
cargo build --release --locked
cargo fmt -- --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings

target/release/isotope fex fetch
target/release/isotope fex doctor
target/release/isotope fex build-fex
target/release/isotope fex build-wine
target/release/isotope fex probe
```

`--work` defaults to `target/arm64-fex`; `--jobs` defaults to 6. Component builds
and probes use that workspace. Use `release build` below to create an installable
core package.

Build reuse requires matching sources, patches, configuration, compilers,
toolchain, SDK and recorded environment. Missing or mismatched receipts stop the
build and preserve existing products. Use a fresh `--work` after changing inputs;
old build directories are not adopted automatically. Recipes and receipts reject
unknown fields, unsupported schemas and malformed values.

## Recipes and graphics

The three recipes describe separate build layers, not three competing Wine bases.

| Recipe | Purpose | Commands |
| --- | --- | --- |
| [arm64-fex.toml](assets/wine/recipes/arm64-fex.toml) | Wine/FEX sources, Darwin patches, compiler, FreeType and optional reference archive | `fex`, `release`; shared by graphics builds |
| [arm64-fex-dx12.toml](assets/wine/recipes/arm64-fex-dx12.toml) | Mesa, its build dependencies, Vulkan-enabled Wine and reference DXVK/VKD3D archives | `dx12` |
| [arm64ec-graphics.toml](assets/wine/recipes/arm64ec-graphics.toml) | Source-built graphics frontends, MinGW and the paired Darwin TEB/CRT settings | `frontend` |

All three are used. Core release builds require only the first recipe. The
`dx12` name also covers Vulkan/DX11 support; `frontend` builds the separately
paired graphics DLLs. These workflows retain distinct build and provenance
records because their Wine configuration, sources and ABI requirements differ.
Use `isotope dx12 --help` and `isotope frontend --help` for available actions.

Mesa is pinned to [Maxyme/Mesa](https://github.com/Maxyme/mesa), based on
athei/Mesa with focused compatibility fixes. Isotope builds this source directly.
The recipe's `MESA_KK_EXPERIMENTAL=image_view_min_lod` setting is included in native
probes and assembled runtimes. Sparse D32 support remains incomplete, and polygon
point mode uses a wireframe fallback.

Reference-app imports are local diagnostics. Fetch them explicitly with
`fex fetch --reference`; they are excluded from core releases. Shader probes
require an explicitly supplied `NUCLEON_PROBE_D3DCOMPILER`. Steam and game launch
helpers live in Nucleon's devtools.

## Validation and compatibility

Core probes cover x64/x86 execution, shared data and memory protection. Font
validation checks real Tahoma text metrics and an exact 480×240 client rectangle,
then repeats after Wine restart. `fex probe --window` adds a visible GDI test.
FreeType and Wine's replacement fonts are bundled; no host font library is needed.
Use fresh test prefixes when moving from a fontless runtime: persisted window
and theme metrics may already be corrupt. Preserve existing user prefixes/saves.

Both x64 and x86 release CPU tests always run. x64 execution, archive import,
integrity, native dependencies, font/window metrics and server cleanup must pass.
For this preview, an x86 failure is recorded as `FAILED_OPTIONAL` and the overall
result becomes `PASSED_WITH_OPTIONAL_FAILURE`. Add `--require-x86` to any `release`
action to make it mandatory. Interruption still aborts publication.

Local testing has passed both CPU workloads, including 32-bit child creation,
and relocated Nucleon archive installation. The separately assembled graphics
runtime reached user-confirmed menus and input in DiRT Rally 2.0 (DX11/DXVK) and
DMC5 (DX12/VKD3D). Driving, combat and sustained performance remain untested.
The ARM64EC graphics pairing currently accepts x64 games only; GoNNER's x86
WineD3D path fails OpenGL context creation. VKD3D also reports the missing
`VK_EXT_dynamic_rendering_unused_attachments` extension. See [ROADMAP.md](ROADMAP.md)
for the 32-bit graphics plan. CPU and native graphics probes do not certify games.

## Package and install

Package validation invokes Nucleon's public CLI. Supply `--nucleon /path/to/nucleon`
or `ISOTOPE_NUCLEON`; otherwise `nucleon` is resolved from PATH. The executable must
support `wine install` and schema-1 `wine inspect --json`. Fetching and component
builds do not require Nucleon; runtime assembly and release validation do.

```sh
target/release/isotope --nucleon /path/to/nucleon release build \
  --version 0.1.0-rc.1 --work target/runtime-release \
  --output target/release-assets
```

The output directory must be new. `release build` fetches, compiles, packages and
validates the core. `release package` reuses an unchanged publisher build receipt.
`release validate` tests existing products through a relocated installation,
retaining logs and a diagnostic archive in a new output directory. The diagnostic
archive has no corresponding source bundle and is not a distribution release.

Core releases contain Wine, x64/x86 FEX emulators and native companions, built-in
WineD3D, fonts, notices and provenance. Graphics add-ons, Vulkan loaders/drivers,
Steam binaries, GPTK and Microsoft game DLLs are excluded. Each release includes
a binary archive, corresponding patched sources and build scripts,
`runtime-release.json`, `SHA256SUMS.txt` and validation logs.

```sh
nucleon wine install isotope --version 0.1.0-rc.1 --register
nucleon wine install ./isotope-0.1.0-rc.1-macos-arm64.tar.gz --register
```

Nucleon also accepts a runtime directory or archive URL. The named source resolves
the exact `v<version>` release in `Maxyme/isotope` and verifies its checksum.
Registration selects the experimental FEX runtime without changing Intel Wine.

The [release workflow](.github/workflows/release.yml) requires a full
Nucleon commit via manual input `nucleon_ref` or repository variable
`NUCLEON_VALIDATOR_REVISION`. Use a pushed commit containing the required consumer
CLI. Manual dispatch retains CI artifacts; `v*` tags publish prereleases after
validation.

## Licenses

The builder is GPL-3.0-only. Wine, FEX, FreeType, graphics components and compiler
runtimes retain their own licenses and attribution. See [LICENSE](LICENSE),
[NOTICE.md](NOTICE.md) and [the patch notes](assets/patches/README.md).
