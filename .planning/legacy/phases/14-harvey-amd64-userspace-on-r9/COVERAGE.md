# API Coverage — Phase 14

> Full coverage by default. Opt-outs are explicit, reasoned decisions.

No external API integration: the phase-integrating-"API" signal the detector matched is r9's flake
boot-iso builder surface that Harvey's flake consumes (D-05) — a first-party cross-repo nix build
interface between two sibling checkouts of this same project, not an external API/SDK/service.
Everything else in the phase is in-tree kernel/build/gate work (Plan 9 C toolchain, Rust no_std
kernel, expect gates).
