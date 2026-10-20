# Isotope notices

Builder orchestration and portable process/archive helpers were extracted from
Maxyme/Nucleon at b171067 (2026-10-19), under GPL-3.0-only. LICENSE retains those
terms. Downstream source modifications are dated in exported compilation inputs.

Pinned upstream Wine retains LGPL-2.1-or-later. The Darwin overlay is derived in
part from Tolkara Wine/FEX changes (Vasilii Kharitonov and contributors); their
copyright and license notices remain in the derived patches. Neither Tolkara fork
is a build dependency. The pinned MacRunner shared-data helper retains its
upstream notices. FEX retains MIT; its submodules
and LLVM/MinGW runtime components retain their own licenses. The packager exports
patched Wine, FEX, initialized pinned submodules, helper origins and builder
sources, and includes upstream and compiler/runtime notices in the binary tree.

The experimental reference-app, graphics and SDK probes do not grant permission
to redistribute their inputs. Core releases exclude those components, Apple GPTK,
Microsoft DLLs and Valve/Steam binaries. Consult each pinned source's notices.

The experimental KosmicKrisp driver is built from the pinned Maxyme/Mesa fork
based on athei/Mesa. Its source provenance is recorded in
`src/kosmickrisp/isotope-sources.json` in that repository. Adapted Steamac query
and wireframe changes and native probes retain Steamac's Apache-2.0 license;
the driver stage includes that license alongside Mesa's license and provenance.

Native font support uses FreeType 2.14.3 under the FreeType License (FTL), with
its upstream license texts included under `share/licenses/freetype`. Portions of
this software are copyright © 1996–2026 The FreeType Project (www.freetype.org).
All rights reserved. Wine's replacement fonts retain their original notices;
they are built/package inputs from the pinned Wine source, not Microsoft fonts.
The corresponding FreeType source archive is included in source releases.
