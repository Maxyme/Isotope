# Runtime capabilities and launch preparation

Nucleon resolves a selected Wine into a `ResolvedRuntime`, and prepares a
`LaunchPlan` containing its prefix, graphics components and Steam launch protocol.
Graphics availability checks, staging and the game environment use this selection.
Wine version strings do not select the execution mode.

## Native runtime descriptor

An ARM64/FEX runtime requires `nucleon-runtime.toml` at its root. The
[core descriptor](assets/wine/runtime-arm64-fex.toml) defines the shipped core
layout; graphics assembly adds its bundled component paths. Ordinary Intel Wine installations can still be inferred from
their binaries. A native runtime without a descriptor fails with a configuration
error; Nucleon does not guess which emulator to use.

The descriptor records:

- Schema version, execution mode (`intel` or `arm64-fex`) and guest ABIs
  (`x64`, `x86`).
- Relative loader, wineserver, Unix and Windows library paths. Paths must exist,
  stay inside the canonical runtime root and match the selected native ABI.
  Universal loaders execute the explicitly selected slice.
- FEX emulator files for every declared guest ABI. The x64 emulator must have
  an ARM64EC PE ABI; the x86 emulator must have an ARM64 PE ABI. File-level
  checks establish a component contract, not successful CPU/game validation.
- Compatible graphics backends and optional bundled DXMT, DXVK, VKD3D, Vulkan
  ICD and Vulkan loader locations. Bundled ICDs use a relative library path to
  an architecture-compatible library inside the runtime. The Vulkan loader
  must export `vkGetInstanceProcAddr`.
- Steam bridge protocol (`bridge` or `valve-redirect`) and whether to use the
  Windows Steam helper by default. An explicit per-game helper choice wins.
- Required native prefix family and ABI (`[prefix] family = "isotope-fex"`,
  `abi = 1`). Families contain lowercase letters, digits, `_` or `-`; the ABI
  is positive and changes when an existing prefix cannot be safely reused.
- Correctness settings `FEX_TSOENABLED=1` and, for Wine builds that need the
  downstream shared-data decoder, `NUCLEON_EMULATE_KUSD=1`. The descriptor cannot
  override runtime search paths, prefix selection or disable memory ordering.

The descriptor describes a complete runtime directory. It cannot reference
build-tree siblings using `../`. The Rust developer tools can assemble a complete local developer generation.
The managed installer imports them outside the build tree; compilation stays a developer action.

`nucleon wine inspect /path/to/runtime --json` reports resolved execution
capabilities or a descriptor error. Checking status does not initialize a prefix.

## Build, install and select

Isotope builds complete runtime directories and source-inclusive core archives.
Use the [builder commands](README.md#build) and [packaging workflow](README.md#package-and-install).
Nucleon imports the output as an immutable generation; installation and selection
are separate actions:

```sh
nucleon wine install /path/to/runtime-or-archive
nucleon wine fex select /path/to/installed/runtime
nucleon wine fex verify
nucleon run /path/to/program.exe --emulator fex
nucleon launch <appid> --emulator fex
```

The selected directory is printed by installation. A one-launch `--wine <exact-id-or-path>`
selection leaves the saved default unchanged. Nucleon starts the program directly.
Steam games use Nucleon's managed game installation and a signed-in native Steam
client for ownership and Steamworks services.

Nucleon owns game profiles, prefix locking/preparation, bridge staging, shader
caches, launch logs and process cleanup. See its [setup guide](https://github.com/Maxyme/Nucleon/blob/main/SETUP.md)
and [architecture](https://github.com/Maxyme/Nucleon/blob/main/ARCHITECTURE.md)
for those consumer policies. Isotope owns runtime build inputs, package integrity,
ABI declarations, corresponding sources and its owned runtime probes.

## Experimental graphics pairing

An experimental descriptor can declare:

```toml
[graphics]
frontend_abi = "arm64ec"
frontend_manifest = "share/nucleon/arm64ec/frontend.json"
dxvk = "share/nucleon/arm64ec/dxvk"
vkd3d = "share/nucleon/arm64ec/vkd3d"
vulkan_icd = "lib/vulkan/kosmickrisp_icd.json"
vulkan_loader = "lib/vulkan/libvulkan.1.dylib"
```

Omitting `frontend_abi` selects `x64`, preserving the existing guest-specific
x86/x64 checks. `arm64ec` requires ARM64/FEX, only x64 guests, one Windows library
directory, and the pinned DXVK/VKD3D Vulkan component set. DXMT/GPTK and DX9/DX10
are outside this initial contract. The guest executable, Steam helper and bridge
retain their existing ABI requirements.

The schema-1 frontend JSON contains `frontend_abi`, `files` (runtime-relative
paths to SHA256 digests), and complete `dxvk`/`vkd3d` build receipts in `builds`.
Coverage includes the selected DLLs, their original receipts, Darwin CRT/TLS
objects/header, Wine/FEX PE and native boundaries, and Vulkan loader/driver/ICD.
Receipt sources must match the pinned frontend recipe; C and C++ options must
declare the paired emulated-TLS/thread-pointer contract. Runtime inspection and
launch selection verify identities and reject unpaired overrides before prefix
preparation. Managed import verifies the same contract after relocation.
These local provenance records establish pairing and integrity; they are not a
signed certification of arbitrary binaries or gameplay compatibility.

`isotope frontend prepare-runtime --base-runtime /path/to/baseline`
copies an existing descriptor-backed runtime into a separate
`target/arm64-fex/arm64ec-integrated/<generation>` directory. It packages retained
ARM64EC Vulkan builds and gives them an experimental prefix family. With an
explicit baseline, assembly executes no Wine, Steam or game command and does not
register or select a Steam compatibility tool. Run receipts report the frontend
ABI and pairing-manifest path; they do not claim to identify loaded modules.

Vulkan paths require matching loader/driver/Wine components. A native FEX runtime
must declare its Vulkan loader; availability includes the native Wine Vulkan
companion. The resolved driver directories precede the loader directory to avoid
loading an SDK's different same-name driver. The legacy MoltenVK alias is private
to the prefix and points to that same resolved driver.

## Runtime extensions

The native ntdll exports `isotope_dll_search_prepend_v1` and
`isotope_guest_supervisor_v1` for consumers to detect optional DLL-search and
launcher-death supervision support. `WINEDLLPATH_PREPEND` retains Wine's builtin
DLL rules. `ISOTOPE_SUPERVISOR_PID` identifies the owning launcher; it does not
identify Steam. These features also serve standalone Windows applications.
`NUCLEON_EMULATE_KUSD` remains a required runtime correctness setting for builds
using the downstream shared-data decoder.

## Validation boundaries

Core releases validate their fixed package layout, relocated execution, native
closure, fonts, permissions and checksums independently of Nucleon. Experimental
graphics assembly validates local paths, native/guest images and pairing hashes.
`isotope check-consumer --runtime <path> --nucleon <executable> --output <report>`
checks Nucleon's descriptor interpretation explicitly after assembly; ordinary
assembly records this separate check as `NOT_RUN`.
Neither static inspection nor a passing owned probe establishes game compatibility.
Steam ownership, rendering, input and gameplay are separate consumer checks.
Prefix families and ABI versions describe saved-state compatibility; their names
must not be changed as a cosmetic cleanup.
