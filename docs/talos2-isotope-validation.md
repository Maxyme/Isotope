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

The game window is unavailable through the app-inspection tool. The first short
test ended before the subsequent visual-feedback check; a report of no window
after that test does not establish a fullscreen failure. A later windowed test
was confirmed visible by the user, as described below. Interactive gameplay beyond
the menu remains unverified. The test uses a four-minute time limit
and disposable prefixes. It ends with the expected `bounded` receipt, and its
Wine server exits cleanly. Original recorded game-file hashes remain unchanged.

Validation: 38 Rust tests pass, three external-artifact tests remain ignored, and
strict Clippy plus formatting checks pass. `probe-security` fails on the previous
runtime and passes on the rebuild. Investigation logs and receipts are retained
under `/private/tmp/talos-isotope-20261020`; logs that can contain game credentials
remain inside its private `game-test` subdirectory.


## Visible-window confirmation

A follow-up launch uses `-dx12 -windowed -ResX=1280 -ResY=720 -WinX=100 -WinY=100`
with HUD disabled. Wine traces show the Talos window created at an on-screen
position, shown, and receiving client-surface presentation calls. The user confirms
that this game window is visible. Its receipt records a normal exit with code 0;
the game log reports `Closing by request`, followed by orderly engine shutdown.
There is no reproduced startup crash in this test.

Nucleon's launcher does not request a hidden window or change GUI sessions. An
owned native-window diagnostic also reports a visible, ordered, non-minimized
window on the active desktop. No Wine window-system patch was added. The working
windowed configuration is preserved in the isolated `talos2-isotope-windowed`
application profile, without an automatic test timeout. Original Talos/GPTK/FSR
settings, game files and saves remain unchanged. Fullscreen behavior has not been
independently verified.

For this local investigation, the preserved profile can be launched with:

```sh
NUCLEON_SUPPORT_DIR=/private/tmp/talos-isotope-20261020/game-test/support \
  nucleon app run talos2-isotope-windowed
```

This uses the disposable game clone and the rebuilt runtime in Isotope's `target/`
folder. It does not select or modify the original saved Talos profile.
