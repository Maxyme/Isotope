# Isotope notices

Builder orchestration and portable process/archive helpers were extracted from
Maxyme/Nucleon at b171067 (2026-10-19), under GPL-3.0-only. LICENSE retains those
terms. Downstream source modifications are dated in exported compilation inputs.

Pinned upstream Wine retains LGPL-2.1-or-later. Tolkara Wine/FEX changes and the
shared-data helper retain their upstream notices. FEX retains MIT; its submodules
and LLVM/MinGW runtime components retain their own licenses. The packager exports
patched Wine, FEX, initialized pinned submodules, helper origins and builder
sources, and includes upstream and compiler/runtime notices in the binary tree.

The experimental reference-app, graphics and SDK probes do not grant permission
to redistribute their inputs. Core releases exclude those components, Apple GPTK,
Microsoft DLLs and Valve/Steam binaries. Consult each pinned source's notices.
