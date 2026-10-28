# Talos Principle 2 startup investigation — 2026-10-28

The Epic Windows edition was tested through Nucleon with Isotope in a disposable
APFS game clone, an isolated support directory and fresh prefixes. Original game
files, the existing GPTK/FSR4 profile and its prefix were preserved.

## Graphics baseline

The initial test accidentally used the older `upstream-game-runtime` Mesa build.
With the current recipe's Mesa `e3d0c46f52479f16da810bfbccfbc123ec1674c0`, Talos reports
shader model 6.6 and supported 64-bit atomics, then creates its SM6 Direct3D 12
renderer. The earlier shader-model rejection does not apply to this driver.

## Reproduced startup defect

The game then fails to load `EOSSDK-Win64-Shipping.dll` and later dereferences a
null interface. An owned x64 `LoadLibraryW` probe reproduces the SDK load failure
with error 998. Wine tracing identifies an access violation during secur32's
security-provider/RPC initialization. No missing eager DLL imports were found.

Instrumenting an isolated rpcrt4 rebuild shows that a delayed ARM64EC variadic
call passes its stack pointer and byte count as the arguments themselves during
an EC → x64 delay resolver → EC round trip. A process-local diagnostic route
straight to the EC RPC function permits the unchanged Epic SDK to load and report
version `1.15.5-24099393`. The diagnostic route is not a shipped workaround.

LLVM fixed this exact compiler defect after the previously pinned May 5 toolchain:
[copy ARM64EC variadic payloads into the x64 stack](https://github.com/llvm/llvm-project/pull/190933),
merged May 17, 2026. Isotope now pins the official October 20 LLVM-MinGW release
(LLVM 23.1.4), verified against its published SHA-256.

`isotope fex probe-security` provides a game-independent regression: a cold
secur32 load, security-package array enumeration/freeing, and service-manager
context-handle creation/closing. Each run uses a fresh prefix, records the probe
hash and result, and cleans up its Wine server. It is also a mandatory x64 check
in release validation. The old runtime fails this probe at secur32 loading.

Wine's separate warning about missing schannel support comes from the current
`--without-gnutls` recipe. It is not the immediate RPC crash; TLS/package changes
need separate runtime evidence.

## Rebuilt runtime and game result

The fresh Wine rebuild with LLVM 23.1.4 passes `probe-security`. The same unchanged
Epic SDK loads normally and reports its version, without the diagnostic bypass.
The newly paired runtime also passes Nucleon's explicit consumer validation and
inspection after relocation to `target/talos2-llvm23-runtime`.

The test retains Wine 11.20, FEX-2610, the current Mesa driver, DXVK 3.1 and
VKD3D-Proton 3.0.1. Existing ARM64EC graphics binaries were reused; their historical
receipts were normalized in the disposable workspace and a new pairing manifest
records the rebuilt Wine components. No graphics source upgrade or GnuTLS package
was needed to resolve the reproduced startup failure.

A four-minute Nucleon launch of the isolated game clone creates its SM6 Direct3D
12 renderer, initializes the engine, loads `/Game/Maps/Talos2/Menu/MainMenu2`, and
completes Epic login with `EOS_Success`. The previous SDK loading error and null
interface crash do not recur. Optional Epic friends/presence requests report
insufficient access scopes, while login and achievement queries succeed.

The game window is unavailable through the app-inspection tool. Visual rendering
and interactive gameplay remain unverified; reaching the menu in logs is startup
evidence, not a gameplay certification. The test uses a four-minute time limit
and disposable prefixes. It ends with the expected `bounded` receipt, and its
Wine server exits cleanly. Original recorded game-file hashes remain unchanged.

Validation: 38 Rust tests pass, three external-artifact tests remain ignored, and
strict Clippy plus formatting checks pass. `probe-security` fails on the previous
runtime and passes on the rebuild. Investigation logs and receipts are retained
under `/private/tmp/talos-isotope-20261020`; logs that can contain game credentials
remain inside its private `game-test` subdirectory.
