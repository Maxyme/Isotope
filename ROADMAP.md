# TODO

## 32-bit game compatibility

The x86 CPU smoke test passes, but this does not establish graphics compatibility.
GoNNER currently fails Direct3D initialization through WineD3D: the OpenGL context
call `wglCreateContextAttribsARB` returns `0xc0000005`, and Unity cannot initialize
D3D9 or its DXGI factory (`0x887a0004`). The standalone WGL probe reproduces
context creation failure on x86 while its
x64 counterpart passes. The Direct3D 11 WineD3D probe currently fails on both
architectures; track that separately from the x86 context failure.

### Preferred path: upstream x86 DXVK with KosmicKrisp

- [x] Add an x86/x64 Vulkan probe with device, surface, swapchain, GPU clear,
      synchronized pixel readback and present (`fex probe-vulkan`).
- [ ] Fix the x86 Vulkan command-pool allocation bridge: the paired Mesa probe
      creates a device and swapchain, then faults in `vkCreateCommandPool` on an
      untranslated low guest address. The same probe passes x64 GPU readback and
      presentation. Establish a passing x86 baseline before adding DXVK.
- [ ] Add an x86 Direct3D device-and-present probe using upstream DXVK. Verify
      actual frame output rather than treating a live process as a graphics pass.
- [ ] Assemble pinned upstream x86 DXVK DLLs with the validated native Vulkan
      loader and Mesa/KosmicKrisp driver. Preserve hashes, licenses and provenance.
- [ ] Define an explicit x86 graphics pairing and validate Nucleon's selection and
      staging. The current ARM64EC graphics contract accepts only x64 guests;
      do not relax its checks or reuse ARM64EC DLLs for x86 games.
- [ ] Test GoNNER through Nucleon with an isolated prefix. Confirm visible menu,
      input, gameplay and normal exit with the user while the game is running.
- [x] Document the initial WoW64 Vulkan command-pool blocker exposed by these
      tests. The proposed x86 path remains unverified and may need further fixes.

### WineD3D/OpenGL compatibility

- [x] Add x86/x64 WGL reproductions with core/shared contexts and readback/present
      (`fex probe-graphics`); baseline: x86 context creation fails, x64 passes.
- [x] Add opt-in signal-safe syscall-fault records with guest-window coordinates.
- [ ] Repair the OpenGL client-object pointer conversion in `make_opengl`.
      Fault records show a low raw guest address while the guest window is mapped
      at a higher host address; the generated context-creation bridge currently
      treats its client-memory output as an opaque handle. Check shared-context,
      pbuffer and sync-object conversions as part of the fix.
- [ ] Add a regression probe for the confirmed defect and retest GoNNER's
      WineD3D path after repair.

Keep the x86 release CPU test optional for now. Track CPU execution, graphics
presentation and user-confirmed gameplay as separate validation results.
Prefer maintained upstream DXVK and Wine bridges over custom graphics code.
