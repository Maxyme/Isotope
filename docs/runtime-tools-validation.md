# Runtime tools validation — 2026-10-21

Local Apple Silicon validation used the pinned Wine 11.20, FEX-2610 and
FreeType 2.14.3 recipes. Wine was built in a fresh workspace with the runtime-tools
overlay; the earlier validated installation was preserved. The Vulkan fixture
pairs this core with Mesa `e3d0c46f52479f16da810bfbccfbc123ec1674c0`, already used by
the game's graphics runtime, with `MESA_KK_EXPERIMENTAL=image_view_min_lod`.
No installed games were launched for these checks.

| Check | x86 | x64 |
| --- | --- | --- |
| Builtin DLL search: appended, prepended, native rejection | All pass | All pass |
| WGL legacy/core/shared context, GPU readback and present | Fails context creation | Pass |
| WineD3D D3D11 device, GPU readback and present | Fails device creation | Fails device creation |
| Vulkan device, swapchain, GPU readback and present | Fails command-pool creation | Pass |

The fault record reproduces the x86 OpenGL access violation against a low guest
address despite a high host guest-window base. Context creation's generated bridge
uses an opaque-handle conversion for its client-memory output; inspect the other
context, pbuffer and sync-object conversions before repairing it. Vulkan separately
reports the same low-pointer pattern in `vkCreateCommandPool`, after device and
swapchain creation. These failures remain compatibility work in [ROADMAP.md](../ROADMAP.md).

The CPU/shared-data probe and font/window-metrics restart check pass with the new
extensions disabled by default. Launcher supervision passes both tests:

- A new server, an x86 guest and its Windows child exit after owner SIGKILL; an
  unrelated native control process survives.
- An existing server and independent Windows observer run without supervision.
  The observer opens handles to the supervised guest and its child before owner
  SIGKILL, then confirms both exited. Server-side shutdown cannot satisfy this test.

The DLL probe loads identifiable copies of real Wine builtins. It confirms that
ordinary `WINEDLLPATH` loses to the runtime's builtins, `WINEDLLPATH_PREPEND` wins,
and removing the builtin marker makes Wine reject the external candidate. It also
checks empty path entries. This validates search precedence, not a DXMT rendering
certification. Nucleon's tests cover capability detection, old-runtime fallback,
restoration of previously copied DXMT files, no-op repeat preparation, and rejecting
search-based loading for a mixed builtin/native bundle. DXVK/VKD3D retain staging.

Reproduce the owned workloads with the `fex probe-graphics`, `probe-vulkan`,
`probe-dll-search` and `probe-supervision` commands documented in the README. Their
logs and structured results remain in each command's workspace under `core/`.
Graphics failures do not change the existing optional x86 CPU release policy.
