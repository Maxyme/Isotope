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

New runtime descriptors advertise supported Steam protocols as capabilities;
Nucleon chooses game launch policy. Adding Vulkan does not select a Steam protocol
or enable a Windows Steam helper. Core release builds require only the first recipe. The
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

## Runtime diagnostics and lifecycle

The optional regressions run small owned workloads without Steam or installed
games. Each command keeps separate logs and a `result.json` in its workspace:

```sh
isotope fex probe-graphics --work target/arm64-fex
isotope fex probe-supervision --work target/arm64-fex
isotope fex probe-dll-search --work target/arm64-fex
isotope fex probe-security --work target/arm64-fex
isotope fex probe-vulkan --work target/probes --runtime /path/to/graphics-runtime
```

`probe-security` checks cold security-library loading and RPC output handling.
Release validation also requires this x64 check; it catches older compilers that
miscompile ARM64EC variadic calls through delayed imports.

`--runtime` tests an existing installation instead of the workspace's built core.
Vulkan additionally needs a descriptor naming its driver and loader, and accepts
`--sdk`. These graphics regressions are separate from the release CPU gate. WGL
checks legacy/core/shared contexts, readback and presentation. Direct3D and Vulkan
check a cleared frame through GPU readback and a swapchain present; they do not
certify game compatibility. The Vulkan probe can also diagnose the x86 Wine bridge
in an isolated graphics runtime; it does not enable x86 ARM64EC frontend pairing.

`ISOTOPE_FAULT_DIAGNOSTICS=1` emits a bounded native syscall-fault record with host
registers, fault address, guest address window and return address. Formatting uses
no allocation or symbol lookup in the signal handler. Nucleon enables it for
explicit diagnostic launches; ordinary launches leave it disabled.

Consumers can set `ISOTOPE_SUPERVISOR_PID` to their own PID. A native thread inside
each Wine process, and the newly started wineserver, waits for that owner's exit
through macOS process notifications. It terminates only itself if the owner dies,
including SIGKILL. An absent variable leaves normal Wine lifetime behavior intact;
invalid settings or failed registration stop startup. Consumers should retain their
normal prefix/process cleanup. The supervision regression kills a dedicated owner
after a guest and its Windows child are ready, waits for prefix shutdown and checks
that an unrelated control process survives. A separate phase starts an unsupervised
server and observer, proving that both guests exit through their own watchers.

`WINEDLLPATH_PREPEND` searches external Wine-built DLL directories before the
runtime's own builtins. Empty entries are ignored. Wine's rejection of non-builtin
DLLs remains in place: DXVK and VKD3D still require native DLL staging. Nucleon detects
the versioned native ntdll capabilities before selecting search-based DXMT loading
or guest supervision, retaining its existing path for older runtimes. The DLL probe
checks both guest architectures and verifies the selected image, including the
native-DLL rejection case.

## Package and install

Core releases validate the relocated archive independently: file hashes,
permissions, symlinks, native dependencies, CPU execution, fonts and cleanup.
Nucleon managed import is recorded as `NOT_RUN` and checked separately after
publication. Graphics assembly validates its local paths, ABI images and pairing hashes without
requiring a Nucleon executable. Its provenance records consumer validation as
`NOT_RUN`. Check compatibility explicitly after assembly, keeping the report
outside the immutable runtime:

```sh
isotope check-consumer --runtime /path/to/assembled/runtime \
  --nucleon /path/to/nucleon --output /new/path/consumer-validation.json
```

The report records the consumer, resolved runtime and inspection result. Static
consumer inspection does not execute Wine, Steam or a game.

```sh
target/release/isotope release build \
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
nucleon wine install isotope --version 0.1.0-rc.1
# Alternatively: nucleon wine install ./isotope-0.1.0-rc.1-macos-arm64.tar.gz
nucleon wine fex select /path/to/installed/runtime
nucleon wine fex verify
```

Nucleon also accepts a runtime directory or archive URL. The named source resolves
the exact `v<version>` release in `Maxyme/isotope` and verifies its checksum.
Installation prints the installed directory and leaves runtime selection unchanged.
`wine fex select` saves that directory as the FEX default without changing Intel
Wine. Launch through `nucleon run <exe> --emulator fex` or
`nucleon launch <appid> --emulator fex`.

The [release workflow](.github/workflows/release.yml) builds and validates Isotope
without a Nucleon checkout or executable. Portable Rust checks and release
publishing run on `ubuntu-26.04-arm`; native runtime builds and probes run on
`macos-latest`. Manual dispatch retains CI artifacts; `v*` tags publish prereleases
after standalone validation. Nucleon's separate manual integration workflow can
verify a published Isotope version afterward.

## Licenses

The builder is GPL-3.0-only. Wine, FEX, FreeType, graphics components and compiler
runtimes retain their own licenses and attribution. See [LICENSE](LICENSE),
[NOTICE.md](NOTICE.md) and [the patch notes](assets/patches/README.md).
