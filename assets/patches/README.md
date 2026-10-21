# Upstream Darwin overlay

Base: Wine 11.20 (`7298a818b79446bae153b8d5aa27235ab3703bf9`),
FEX-2610 (`14c92681f4d62cf84d901460e0358de09c8847a7`).

- `wine-upstream-darwin.patch`: TSD TEB access, relocatable shared data, 32-bit
  address-window marshalling, Darwin executable-memory pool, ARM64EC stack helper
  alias and cursor registry compatibility. Uses upstream Wine syscall wrappers.
- `wine-upstream-integration.patch`: shared-page initialization in the PE loader,
  the existing opt-in Steam export redirection, and the DXMT window ABI adapter.
- `wine-runtime-tools.patch`: optional signal-safe native fault records,
  builtin DLL search precedence and per-process macOS launcher supervision.
  Versioned ntdll exports allow consumers to detect these extensions without
  changing the runtime descriptor schema. No Voile runtime code is incorporated.
- `fex-upstream-darwin.patch`: TSD TEB access, 32-bit guest-window translation,
  16 KB host guards and the native Unix companion. Applied before either FEX
  emulator is compiled, not just when building the companion.

The overlay adapts Darwin compatibility work from Tolkara Wine/FEX by Vasilii
Kharitonov and contributors. Attribution remains in [NOTICE.md](../../NOTICE.md);
the builds use the upstream Wine/FEX pins above. WoW64 comments describe the
guest-to-host address window directly. The shared-data instruction decoder is
copied from its separately pinned MacRunner source.

Excluded: iPad process hosting, single-process Wine server, alias JIT buffers,
game-specific fixes, generated syscall wrappers and debug-only exit/message
traces. CrossOver and Apple GPTK/D3DMetal are not inputs.
