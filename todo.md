# TODO

## 32-bit game compatibility

The x86 CPU smoke test passes, but this does not establish graphics compatibility.
GoNNER currently fails Direct3D initialization through WineD3D: the OpenGL context
call `wglCreateContextAttribsARB` returns `0xc0000005`, and Unity cannot initialize
D3D9 or its DXGI factory (`0x887a0004`). The exact fault location remains unknown;
these results do not establish that the defect is exclusive to 32-bit execution.

### Preferred path: upstream x86 DXVK with KosmicKrisp

- [ ] Add a minimal x86 Vulkan probe that creates a device, surface and swapchain,
      renders a frame and presents through Wine's WoW64 Vulkan bridge.
- [ ] Add an x86 Direct3D device-and-present probe using upstream DXVK. Verify
      actual frame output rather than treating a live process as a graphics pass.
- [ ] Assemble pinned upstream x86 DXVK DLLs with the validated native Vulkan
      loader and Mesa/KosmicKrisp driver. Preserve hashes, licenses and provenance.
- [ ] Define an explicit x86 graphics pairing and validate Nucleon's selection and
      staging. The current ARM64EC graphics contract accepts only x64 guests;
      do not relax its checks or reuse ARM64EC DLLs for x86 games.
- [ ] Test GoNNER through Nucleon with an isolated prefix. Confirm visible menu,
      input, gameplay and normal exit with the user while the game is running.
- [ ] Document any WoW64 Vulkan or Mesa blockers exposed by these tests. This
      proposed path is not yet verified and may require further fixes.

### WineD3D/OpenGL compatibility

- [ ] Reproduce context creation with minimal x86 and x64 WGL probes to determine
      whether the failure is specific to WoW64/FEX or also affects native Wine GL.
- [ ] Capture the fault location and inspect pointer/handle conversion and native
      context creation before choosing a Wine or FEX patch.
- [ ] Add a regression probe for the confirmed defect and retest GoNNER's
      WineD3D path after repair.

Keep the x86 release CPU test optional for now. Track CPU execution, graphics
presentation and user-confirmed gameplay as separate validation results.
Prefer maintained upstream DXVK and Wine bridges over custom graphics code.
