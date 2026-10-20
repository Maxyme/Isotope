# 32-bit process creation fix (2026-10-19)

The x86 workload did start and pass memory/thread/exception tests. Its call to
`CreateProcessW` failed with error 998 when it tried to start a child process.
The same owned workload passed on x64.

## Cause and change

Tolkara relocates 32-bit guest memory into a 4 GB-aligned host window, above the
low addresses unavailable to a native ARM64 Darwin process. Wine's WoW64 bridge
normally widens guest pointers using `ULongToPtr`, redefined in the fork to include
that window. Scalar process attributes must remain scalar values.

In `dlls/wow64/process.c`, `ps_attributes_32to64` copied each attribute's 32-bit
`Value` into its native union. The `PS_ATTRIBUTE_IMAGE_NAME` case then used that
raw number as a host pointer, both for file redirection and for `NtCreateUserProcess`.
It bypassed the fork's pointer conversion. The control trace shows an ARM64 read
fault at `0x16fb60`, with the relocated window at `0x7000000000`.

The image-name fix, now carried in `wine-upstream-darwin.patch`, converts the image-name
pointer with `ULongToPtr` before either consumer reads it. It changes that pointer
case only, preserving scalar and handle semantics. The original investigation kept the Wine revision unchanged. The fix is now
preserved in the upstream Wine 11.20 Darwin overlay.

The fork's [32-bit window design](https://github.com/tolkara/tolkara/blob/main/docs/WINDOWS.md)
explains the same address mapping. The diagnosis above comes from the local
source, native exception trace and controlled module replacement.

## Regression checks

The owned Windows CPU workload now exercises two child-creation forms: explicit
`lpApplicationName` and a NULL application name with the executable parsed from
the command line. Each child must actually run and exit with code 37. The final
success marker is emitted only after both complete. x64-only shared-data helpers
are scoped to x64 so the x86 workload also compiles with `-Wall -Wextra -Werror`.

A control runtime and cloned runtime with only `wow64.dll` replaced were run in
separate private prefixes. The control reproduces error 998. The fixed module
passes the original workload and the expanded workload, including an executable
path containing spaces and Unicode. No exception handling, memory checks, error
checks or child exit-code checks were disabled.

Wine was then incrementally rebuilt through `fex build-wine` from a private cloned
source/build workspace with the recipe patch applied. The existing pinned
compiler and FEX products were reused; Nucleon's checkout and cached Wine sources
were left unchanged. The rebuilt runtime passed the real Nucleon importer, full integrity/native
dependency verification, x64 and x86 CPU workloads and server cleanup with
`release validate --require-x86`. Its receipt records `x86_required: true`, both
guests `PASSED`, and overall `PASSED`. Both child launch forms passed for both
guests. Portable builder tests (18 passed), strict Clippy and formatting passed.

The raw evidence is under `target/wow64-investigation/` and
`target/wow64-strict-validation/`; the final summary is `WOW64-VALIDATION.json`.
This is a local CPU/process regression fix, not a Steam/gameplay qualification.
No release publication or source commit is part of this investigation.
