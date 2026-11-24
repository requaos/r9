# Kernel bytes before and after the rfork flag move

Phase 2 changed no executable code. This file records the measurement (D-23).

- Base commit (before, from `before.sha256`): `c002feb2379949ea50bf7e02248c0d21f4edf85e`
- After commit (`git rev-parse HEAD` when the after builds were made): `09e6901f24e5f1479f8657fc9b5809632d874a16`
- Venue: `Darwin arm64` (both the before and after builds were made on this one machine)

A Mac-built kernel is not comparable with the Linux `kernel-x64.elf` hash recorded at the
owner stop (`nix build .#kernel-x64` on the x86_64 Linux host): the kernel `.rodata` carries
toolchain store-path strings, and the toolchain derivation differs per system. Only the
before/after comparison below, made on one venue, is evidence.

`.text` was extracted with the devshell's `$OBJCOPY -O binary --only-section=.text`
(`.text.boot` likewise on aarch64); the whole-ELF and section values are sha256.

## Before and after

| Config | Section | Before (`c002feb`) | After (`09e6901`) | Same or different |
|--------|---------|--------------------|-------------------|-------------------|
| `q35churnshell` | whole ELF | `8f2d485e375937a764006273bbd6caf1f9bdb178dd307ad973abe981ac616cce` | `c9dcef0b32b77e13f30f8a6ba7b448948e684720dc794de003d06a1e4988b78b` | different |
| `q35churnshell` | `.text` | `38eed81462c8bf248f48310a08c0ebd9e4e62b6a8244a05a8e5df0d180820330` | `38eed81462c8bf248f48310a08c0ebd9e4e62b6a8244a05a8e5df0d180820330` | same |
| `q35cpu` | whole ELF | `31bc6156a84d489122867244291afcd70b59885728bcd0890d144c9d4eb2c1d2` | `610804e62dcbf9f5e3691320224e1d8b386fbb2d44e327e3ee834c5becff0cf9` | different |
| `q35cpu` | `.text` | `3618c3304bf0c8d8f8ef6a3aad96ef0da5404688897a9c9f6041c4c33232544f` | `3618c3304bf0c8d8f8ef6a3aad96ef0da5404688897a9c9f6041c4c33232544f` | same |
| `virtshell` | whole ELF | `76e6e153f578b7022da74570802729e51d3105b9e3ffab0c31ef12917fec56d2` | `9218b998327c49b6ea06e79761d421328ad3451702719a0a0af8b79c59fa083e` | different |
| `virtshell` | `.text` | `b904ec4e5ae8dc76e4b2edce5f20e5fd5adc331bbddd203cc3f060ec9ff540fb` | `b904ec4e5ae8dc76e4b2edce5f20e5fd5adc331bbddd203cc3f060ec9ff540fb` | same |
| `virtshell` | `.text.boot` | `3d396fda638dd564a10b9f752e8fca12490440ab29f88838c617f7ad333ff454` | `3d396fda638dd564a10b9f752e8fca12490440ab29f88838c617f7ad333ff454` | same |
| `virtcpu` | whole ELF | `10fd33201b83e8b22cb0ba780a9a8ccb6a416195ea6e6a2c214723a06c1b012a` | `c7f4171d3e7e27c943c8ff93a059927fabaeb4daaff92cb351cbd264bd81b934` | different |
| `virtcpu` | `.text` | `ea9b4bcd901f07c942153f1e0c8c02c83a4acaee5b86b2ca10c5af0230d8dc8c` | `ea9b4bcd901f07c942153f1e0c8c02c83a4acaee5b86b2ca10c5af0230d8dc8c` | same |
| `virtcpu` | `.text.boot` | `920bdd94c82b88fe94a76ab50daf27456410b67ec9edaf11d3e1f7ef4773dd4c` | `920bdd94c82b88fe94a76ab50daf27456410b67ec9edaf11d3e1f7ef4773dd4c` | same |

## Verdicts

VERDICT text-q35churnshell: IDENTICAL
VERDICT text-q35cpu: IDENTICAL
VERDICT text-virtshell: IDENTICAL
VERDICT text-virtcpu: IDENTICAL
VERDICT textboot-virtshell: IDENTICAL
VERDICT textboot-virtcpu: IDENTICAL
VERDICT elf-q35churnshell: DIFFERENT
VERDICT elf-q35cpu: DIFFERENT
VERDICT elf-virtshell: DIFFERENT
VERDICT elf-virtcpu: DIFFERENT

The whole ELFs differ as the research measured: the panic `Location` line numbers moved in
x86_64 `.rodata` and aarch64 `.data` (the refactor shifted source lines), plus the
non-loaded debug and symbol sections. No executable section changed, so D-23's stop
condition is met and the phase proceeds. Because the x86_64 ELF differs, the `boot-iso`
`outputHash` still has to be re-derived at the owner stop; a same-ELF result would have
meant the hash needed no change.

## Stripped-ELF differential (information, not a criterion)

Both ELFs were stripped with `$OBJCOPY --strip-all` and compared with `cmp -l`:

| Config | Differing bytes |
|--------|-----------------|
| `q35churnshell` | 118 |
| `q35cpu` | 118 |
| `virtshell` | 44 |
| `virtcpu` | 44 |

These match the research's measurement on the scratch edit (118 for the x86_64 configs and
44 for the aarch64 ones), which read the differing words as panic line-number fields only.
