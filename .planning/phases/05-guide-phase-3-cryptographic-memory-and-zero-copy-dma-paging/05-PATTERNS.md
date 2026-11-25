# Phase 5: Guide Phase 3 - Cryptographic Memory and Zero-Copy DMA Paging - Pattern Map

**Mapped:** 2026-11-25
**Files analyzed:** 62 new or modified files (grouped into 11 work areas)
**Analogs found:** 55 / 62 (7 have only a partial analog or none; see "No Analog Found")

All analog paths below were checked with `git ls-files` and are tracked source. Line numbers are at HEAD `1e8b2d7`. Excerpts are short on purpose; the planner should cite the path and range, and the executor reads the range.

How to read the match column: **exact** = same role and same data flow; **role** = same role, different data flow; **partial** = one aspect only (named in the entry).

---

## File Classification

### Area A: DEBT-01 to DEBT-05 (first, D-29)

| New/Modified File | Role | Data Flow | Closest Analog | Match |
|---|---|---|---|---|
| `xtask/src/main.rs` `TestStep::run` (mod) | config (test runner) | batch | itself, lines 3578-3694; `Test9pStep` call at 4958 | exact |
| `xtask/src/test9p.rs` (mod: doc, maybe count pins) | utility (test runner) | batch | itself, lines 1-136, tests 279-363 | exact |
| `.planning/milestones/v3.0-phases/04-.../04-VALIDATION.md` (mod) | doc | n/a | the 25 `04-NN-SUMMARY.md` files beside it | no code analog |
| `.planning/milestones/v3.0-phases/04-.../04-OWNER-STOP-2.md`, `evidence/owner-stop-2/RESULTS.md`, `evidence/kernel-bytes/KERNEL-BYTES-GAPS.md` (mod) | doc | n/a | none (dated correction note) | no code analog |
| `ds/src/tpht/measure.rs` (mod: new test case) | test | transform | `table_agrees_with_counting_model`, lines 238-273 | exact |
| `ds/src/tpht/flattened.rs` `default_banks` (mod, only if DEBT-04 changes sizing) | utility | transform | itself, lines 154-159 | exact |
| `docs/threat-model.md` line 181 (mod) | doc | n/a | line 176 of the same list (a "Settled in Phase 4 on <date>" entry) | exact |
| `xtask/tests/threat_model.rs` (mod: new assertion) | test | file-I/O | `threat_model_quotes_the_ac_boundary`, lines 148-175 | exact |

### Area B: Lint opt-in (D-25)

| New/Modified File | Role | Data Flow | Closest Analog | Match |
|---|---|---|---|---|
| `x86_64/Cargo.toml` (mod) | config | n/a | `aarch64/Cargo.toml:15-19` | exact |
| `Cargo.toml` lines 5-11 comment, check-cfg list 44-53 (mod) | config | n/a | itself, lines 15-53 (the predeclare rule) | exact |
| `x86_64/src/{cpu,pio,seg,main,trap,proc,uart16550,dat,allocator}.rs` (mod: 32 `// SAFETY:` comments) | utility | n/a | `x86_64/src/fpu.rs:180-211`, `x86_64/src/proc.rs:438-441` | exact |
| `aarch64/src/platform/virt/rtc.rs`, `aarch64/src/proc.rs`, `aarch64/src/textcopy.rs` (mod: 7 comments) | utility | n/a | `aarch64/src/fpu.rs:353-361`, `aarch64/src/fault.rs:146-149` | exact |
| `UNSAFE-AUDIT.md` (mod) | doc | n/a | itself: scope line 23-36, reconciliation 38-54, phase deltas 62-92, module-state tables 103-113 and 213-221, classification rows 271-311 | exact |
| `xtask/src/lintcontract.rs` (mod: R3 amendment, fixtures) | utility (static check) | file-I/O | itself: `check_ds_token_and_forbid_rules` 261-309, fixture 531-567, tests 613-635 | exact |

### Area C: AVX and XSAVE (D-13, D-30; owner stop 1)

| New/Modified File | Role | Data Flow | Closest Analog | Match |
|---|---|---|---|---|
| `x86_64/src/fpu.rs` (mod: dual-format `FpState`, save/restore selection) | service (CPU state) | request-response | itself: `FpState` 65-118, `save_raw`/`restore_raw` 180-211, `selftest` 476-523 | exact |
| `x86_64/src/cpu.rs` (mod: `has_xsave`, `has_avx`, `has_avx2`, `has_clflushopt`, `has_clflush`, XCR0 helpers) | utility | transform | `has_tsc_deadline`..`has_rdseed`, lines 222-258; `cpuid` 185-206 | exact |
| `x86_64/src/main.rs` (mod: XSAVE enable before `fpu::selftest`, one boot line naming the format) | config (boot) | event-driven | lines 294-304 (`fpu::selftest` call and its boot line) | exact |
| `x86_64/src/trap.rs:415-423` (mod: comment on stack `FpState` alignment) | doc comment | n/a | itself | exact |
| `x86_64/src/proc.rs`, `syscall.rs`, `exec.rs` FP call sites (type growth only) | model | CRUD | no code change per RESEARCH Q2 table | n/a |
| `xtask/src/main.rs` (mod: marker for the FP-format boot line, AVX2 TCG leg) | config (gate) | event-driven | `Q35_CHURN_*` marker consts 1010-1025 | role |

### Area D: `crypto/` additions (D-10, D-18, D-32)

| New/Modified File | Role | Data Flow | Closest Analog | Match |
|---|---|---|---|---|
| `crypto/src/key.rs` (mod: page-key derivation labels) | service (KDF) | transform | `k_file` 83-101, `derive_labelled` 103-123 | exact |
| `crypto/src/session.rs` (mod: cipher-half type for D-18) | model (key holder) | transform | `DirectionKeys` 101-111, `keystream` 361-368, `zeroize`/`is_zeroized` 297-330 | exact |
| `crypto/src/page.rs` (mod: page-cipher type, closure-scoped key accessor, seal/open reference, tag construction) | service | transform | `KeyCapability<S>` in `crypto/src/key.rs:129-235`; `chacha20_xor` in `page.rs:57-96` | role |
| `crypto/tests/keys.rs` or new `crypto/tests/pagekeys.rs` (KATs) | test | transform | `crypto/tests/keys.rs:1-49` | exact |
| in-crate tests for raw-byte pins | test | transform | `crypto/src/session.rs:371-452` | exact |

### Area E: Portable core in `port/` (D-23; no `unsafe`)

| New/Modified File | Role | Data Flow | Closest Analog | Match |
|---|---|---|---|---|
| `port/src/palloc.rs` (new) | service (allocator) | CRUD | `port/src/fid.rs` `ActiveSlots` + `FidShard` 510-566 (Roaring bitmaps under `Lock`); `port/src/bitmapalloc.rs` for the allocator role | role |
| `port/src/segcrypt.rs` (new: page records, `BackingStore`, seal/open, `StrideTracker`) | service + model | CRUD / transform | `port/src/fid.rs` (module doc 1-119, imports 121-130, `FidEntry` 280-319) | role |
| `port/src/superpage.rs` (new, exactly 64 bytes) | model | transform | `port/src/fid.rs:281-319` (`repr(C, align(64))` plus const asserts) | exact |
| `port/src/inflight.rs` (new) | store | event-driven | `port/src/fid.rs` table-on-TPHT shape 484-582 | partial (single-CPU write-once table; see note) |
| `port/src/pager.rs` (new: High-Water Pager, fault decision, pin counts) | service | event-driven | `aarch64/src/fault.rs:36-117` (decision), `x86_64/src/platform/q35/churngate.rs:106-177` (pure, host-tested policy halves) | role |
| `port/src/lib.rs` (mod: module declarations, re-exports) | config | n/a | itself, lines 20-28 and 88-96 | exact |
| `port/tests/common/` fakes (`FrameOps` recorder, `BackingStore` fake) + `mod.rs` | test | CRUD | `port/tests/common/aead_link.rs:1-112`, `port/tests/common/mod.rs` | role |
| `port/src/<module>_tests.rs` (only if a test needs `unsafe` or std fixtures) | test | n/a | `port/src/lib.rs:120-163` (crate-root test modules) | exact |

### Area F: Roaring SIMD kernels (D-16, D-38)

| New/Modified File | Role | Data Flow | Closest Analog | Match |
|---|---|---|---|---|
| `ds/src/simd/x86_64.rs` (new, AVX2 inline asm) | utility (kernel) | transform | `x86_64/src/fpu.rs:388-433` (`xor_fold16` + scalar reference) | exact |
| `ds/src/simd/aarch64.rs` (new, NEON inline asm) | utility (kernel) | transform | `aarch64/src/fpu.rs:425-467` | exact |
| `ds/src/lib.rs` (mod: mount at crate root with `#[path]`) | config | n/a | `port/src/lib.rs:41-50` (`devip_waker`); `ds/src/lib.rs:13-32` | exact |
| `ds/src/setops.rs` (mod: dispatch on an optional proof token; `VectorSection` trait) | utility | transform | itself 19-80; tests 86-212 | exact |
| `port/src/lib.rs` re-export of `VectorSection`; `unsafe impl` in each arch `fpu.rs` | config | n/a | `KernelSimd` guard, `x86_64/src/fpu.rs:325-386` | role |

### Area G: x86_64 pager

| New/Modified File | Role | Data Flow | Closest Analog | Match |
|---|---|---|---|---|
| `x86_64/src/fpu.rs` (mod: restricted section guard) | service | request-response | `KernelSimd` + `kernel_begin_on` + `Drop`, lines 325-386; tests 601-691 | exact |
| `x86_64/src/cryptpage.rs` (new: AVX2 `global_asm!` routine + scalar glue) | utility | transform | `x86_64/src/fpu.rs:392-428` (asm conventions); `x86_64/src/main.rs:46` (`global_asm!` + `att_syntax`) | partial |
| `x86_64/src/shadow.rs` (new: frame bytes, PTE publish, zero + flush) | service | file-I/O (frame) | `x86_64/src/proc.rs:1400-1440` (`page_in_zeroed`, `map_user_page`), `x86_64/src/pagealloc.rs:106-110` | role |
| `x86_64/src/mmu.rs` (new: unmap, dirty bit, `invlpg`) | utility | CRUD | `x86_64/src/proc.rs:155-166` (PTE consts), 438-445 (`invlpg`), 1447+ (`free_user_mappings`) | role |
| `x86_64/src/trap.rs` (mod: `#PF` dispatch) | controller | event-driven | itself 820-850; decision shape from `aarch64/src/fault.rs:36-117` | role |
| `x86_64/src/proc.rs`, `exec.rs` (mod: install sealing, `mapping_log` walkers) | model | CRUD | `x86_64/src/proc.rs:195-203`, 1368-1418 | exact |
| `x86_64/src/usermem.rs` (mod: pin guard, MEM-10) | middleware | request-response | itself 39-70; mock tests 226-297 | exact |
| `x86_64/src/syscall.rs` (mod: pins at sleeping sites; page-aligned `read(2)`) | controller | request-response | existing `UserSliceMut::new` sites in the same file | exact |
| `x86_64/src/platform/q35/<sealgate>.rs` (new kernel-context driver for eviction, tamper, ingress probe) | controller (gate driver) | event-driven | `x86_64/src/platform/q35/churngate.rs` (whole-file precedent) | exact |
| `x86_64/src/main.rs` (mod: `mod` lines, cfg dispatch to the driver) | config | n/a | lines 298-310 | exact |

### Area H: aarch64 pager

| New/Modified File | Role | Data Flow | Closest Analog | Match |
|---|---|---|---|---|
| `aarch64/src/fpu.rs` (mod: restricted section, NEON) | service | request-response | itself 363-423 | exact |
| `aarch64/src/cryptpage.rs` (new, NEON) | utility | transform | `aarch64/src/fpu.rs:428-462` | partial |
| `aarch64/src/shadow.rs` (new) | service | file-I/O (frame) | `aarch64/src/fault.rs:244-276`; `aarch64/src/kmem.rs:132` (`physaddr_as_ptr_mut_offset_from_kzero`) | role |
| `aarch64/src/fault.rs` (mod: sealed path beside the demand path) | controller | event-driven | itself 36-117, 119-192, 291-338 | exact |
| `aarch64/src/usermem.rs`, `proc.rs` (mod) | middleware / model | CRUD | the x86_64 siblings (same API by design) | exact |
| `aarch64/src/platform/virt/<sealgate>.rs` (new) | controller (gate driver) | event-driven | `aarch64/src/platform/virt/churngate.rs` | exact |

### Area I: Ingress (D-17 to D-21, D-36)

| New/Modified File | Role | Data Flow | Closest Analog | Match |
|---|---|---|---|---|
| `port/src/chan/transport.rs` (mod: scatter open, retained key) | service | streaming | `Session::open` 844-936, `handle_marker` 1296-1319 | exact |
| `port/src/devmnt.rs` (mod: page-wise read into frames) | service | streaming | `MntIo` 238-259, `frame_one` 509-552 | exact |
| `port/tests/aead_ingress.rs` (new) | test | streaming | `port/tests/aead_transport.rs:1-40` | exact |
| `xtask/src/test9p.rs` `PORT_TESTS` (mod) | config | batch | itself 51-56 and tests 279-363 | exact |

### Area J: Profiles, probes, gates

| New/Modified File | Role | Data Flow | Closest Analog | Match |
|---|---|---|---|---|
| `x86_64/lib/config_q35lowmem.toml`, `config_q35crypt.toml` (new) | config | n/a | `x86_64/lib/config_q35churn.toml` | exact |
| `aarch64/lib/config_virt<lowmem>.toml`, `config_virt<crypt>.toml` (new; names are the planner's) | config | n/a | `aarch64/lib/config_virtchurn.toml` | exact |
| `Cargo.toml` check-cfg (mod: new `dev_*` keys) | config | n/a | lines 15-53 | exact |
| `xtask/src/main.rs` (mod: marker consts, `BootStage`, `BootProfile`, name-selected profile bools, fail-fast, `finish` arms, post-pass) | config (gate) | event-driven | the churn gate: 1010-1025, 1544-1557, 1749-1764, 2171-2184, 2255-2267, 2499-2526, 2967-2984, 4318-4384 | exact |
| `xtask/src/<probe generator>.rs` (new: generated `a.out` ymm/NEON probes, pinned by an xtask test) | utility (generator) | transform | `xtask/src/rforkh.rs` (render, write, check, tests) | role |
| `xtask/src/image.rs` (mod: register the generator) | utility | file-I/O | `GENERATORS` 73-77, validate arm 404-435, stage arm 868-906, test 2875-2896 | exact |
| `xtask/manifests/amd64/*.toml`, `xtask/manifests/arm64/*.toml` (mod or new image) | config | n/a | `xtask/manifests/amd64/marker.toml`; schema doc `xtask/src/image.rs:14-51` | exact |

### Area K: Documents and evidence

| New/Modified File | Role | Closest Analog | Match |
|---|---|---|---|
| `docs/threat-model.md` (N9, N10, N11 entries, lines 177-180) | doc | line 176 ("Settled in Phase 4 on 2026-11-24 (Phase 4 CONTEXT D-15): ...") | exact |
| `.planning/PROJECT.md`, `REQUIREMENTS.md`, `ROADMAP.md`, `rfc-9p2000-aead.md`, guide ledger row 5 | doc | prior dated rows in the same files | exact |
| phase evidence (`evidence/PINS-05-NN.md`, kernel-bytes, owner-stop files) | doc | `.planning/milestones/v3.0-phases/04-.../evidence/PINS-04-03.md`, `04-OWNER-STOP.md`, `04-OWNER-STOP-RUNNER.sh` | exact |

---

## Pattern Assignments

### Area A: DEBT

#### `xtask/src/main.rs` `TestStep::run` (DEBT-01)

**Analog:** itself. Legs are `(Vec<String>, Option<Configuration>)` pushed onto `all_cmd_args`; static checks run before any leg.

**Pre-leg static checks** (lines 3578-3588):
```rust
    fn run(self) -> Result<()> {
        // D-12: a stale or missing generated header fails the project's
        // test command before any cargo leg runs.
        xtask::rforkh::check(&workspace())?;
        // D-09, D-10, D-16, D-21: the static lint contract ...
        xtask::lintcontract::check(&workspace())?;
        let mut all_cmd_args: Vec<(Vec<String>, Option<Configuration>)> = Vec::new();
```

**A leg** (lines 3618-3626):
```rust
        all_cmd_args.push((
            vec![
                "test".to_string(),
                "--package".to_string(),
                "crypto".to_string(),
                "--doc".to_string(),
            ],
            None,
        ));
```

**The existing `test-9p` entry to reuse, not re-implement** (line 4958): `xtask::test9p::run(&workspace(), Path::new(&cargo()))`. `cargo test -p xtask` becomes one more `all_cmd_args` leg in the form above.

**Arch-leg selection the x86_64 tests depend on** (lines 3653-3683): the arch crate leg is chosen by `std::env::consts::ARCH`, with a second aarch64 leg under the `virtshell` cfg set via `load_named_config(Arch::Aarch64, "virtshell")`. A sealed-profile cfg leg for host tests copies that second leg.

#### `xtask/src/test9p.rs` (DEBT-01, and later Area I)

**Analog:** itself.

**Module doc that DEBT-01 reverses** (lines 4-9 and 43-45): "it is not added to `cargo xtask test`" and "the step is still not part of `cargo xtask test`". Both sentences change in the same commit.

**Count pins that break when `PORT_TESTS` grows** (Area I adds `aead_ingress`):
- line 283 and 313: `assert_eq!(specs.len(), 14, "seven legs, each with a scalar twin: ...")`
- line 317: `assert_eq!(defaults.len(), 7, ...)`
- lines 342-347: `test9p_lists_the_five_port_files_and_the_crypto_session_pins` compares `PORT_TESTS` to the literal five-name list.

DEBT-01 itself changes no count. The plan that adds `aead_ingress` advances these three pins (RED first: the new name in the literal list, then the constant).

**List constant** (lines 51-53):
```rust
pub const PORT_TESTS: &[&str] =
    &["aead_wire", "aead_tlv", "aead_transport", "aead_fid", "aead_negotiate"];
```

#### `ds/src/tpht/measure.rs` (DEBT-04)

**Analog:** `table_agrees_with_counting_model`, lines 238-273. Copy the Miri scaling and the loop over `Distribution::ALL` and `GRID_SEEDS`.

```rust
        let blocks = miri_scaled(1_024, 4);
        let seed_count = miri_scaled(GRID_SEEDS.len(), 1);
        let dist_count = miri_scaled(Distribution::ALL.len(), 1);
        let percent = 100;
        let entries = entries_at(blocks, percent);
        let banks = banks_for(entries);
```

**The bar and the two sizing rules being compared:**
- `exceeds_bar(hits, lookups)` = `hits * 10 > lookups * 9` (lines 119-122)
- pin sizing: `banks_for(entries)` = `(2 * entries.div_ceil(BANK_NODES)).max(2)` (lines 124-128)
- production sizing: `default_banks(num_blocks)` = `num_blocks.div_ceil(64).max(2)` (`ds/src/tpht/flattened.rs:157-159`); `FidTable` uses zero banks (`port/src/fid.rs:618-623`).

Generator-style runs that are not pins use `#[ignore = "..."]` (lines 275-278).

#### `docs/threat-model.md` + `xtask/tests/threat_model.rs` (DEBT-05)

**Document entry form to copy** (line 176): a bold lead, the source citations in backticks, then "Settled in Phase 4 on 2026-11-24 (Phase 4 CONTEXT D-15): ...". Every bullet in sections 2 to 7 must carry a source tag (`[AC `, `[RFC `, `[IG `, `[PT `) or `**[new]**`; the test at lines 194-222 enforces it.

**Test form** (lines 148-175): read the doc at run time through `doc()`, then `assert!(doc.contains(...), "<message naming the missing text>")`. A missing document is a failing test with a message, not a compile error (module doc lines 1-8).

---

### Area B: Lint opt-in

#### `x86_64/Cargo.toml`

**Analog:** `aarch64/Cargo.toml:15-19`.
```toml
# Opt into the workspace lints (clippy undocumented_unsafe_blocks
# = deny + the check-cfg declarations above).  ...
[lints]
workspace = true
```
Delete `x86_64/Cargo.toml:18-38` (the `[lints.rust]` table and its comment). Cargo forbids both tables together; the workspace `Cargo.toml:40-42` already records that rule from the aarch64 move. Every cfg in the deleted table is already in `Cargo.toml:44-53`.

#### SAFETY comments in `x86_64/src/*` and the 7 aarch64 sites

**Analog:** `x86_64/src/fpu.rs`. One comment per block, immediately before `unsafe {`, stating the invariant, not restating the code.

Block inside an `unsafe fn` (lines 180-191):
```rust
unsafe fn save_raw(dst: *mut FpState) {
    // SAFETY: the caller's contract — FXSAVE64 writes at most the 512
    // bytes at dst (16-byte aligned, as FXSAVE requires).  CR4.OSFXSR is
    // set at boot (l.S), so FXSAVE saves the XMM state too.
    unsafe {
```
Safe wrapper (lines 214-217):
```rust
pub fn save(dst: &mut FpState) {
    // SAFETY: dst is a live, exclusively borrowed, aligned FpState.
    unsafe { save_raw(dst) }
}
```
Pure instruction (`x86_64/src/proc.rs:438-441`):
```rust
    // SAFETY: invlpg is a pure TLB-management instruction on the
    ...
    unsafe { core::arch::asm!("invlpg ({0})", in(reg) va, options(att_syntax, nostack, preserves_flags)) };
```
`unsafe fn` items carry a `# Safety` doc section (lines 175-179), which the lint does not check; the audit lists them per file.

Undocumented today, as examples of what the pass fixes: `x86_64/src/cpu.rs:188` (`cpuid`) and `216` (`rdtsc`) have a doc comment but no `// SAFETY:` line.

#### `UNSAFE-AUDIT.md`

**Analog:** itself. Four things to copy:
1. **Scope line** (23-36): rewrite the sentence "x86_64 + riscv64 are NEVER compiled by the gates ... NO lint opt-in" and keep the old text as dated history, as the "Phase 4 update (2026-11-24)" sentence at lines 29-32 does.
2. **Reconciliation table** (38-54): one row per crate of lint-visible blocks plus impls, a bold SAFETY-annotation total, and the sentence that the sum reconciles. Add an x86_64 row.
3. **Phase delta paragraph** (62-72 and 74-92): `**Phase N delta (<what>, <date>):**`, counts by kind, the re-measured line numbers, and whether kernel bytes moved.
4. **Module-state table** (103-113 for `ds`, 213-221 for `port`) and **classification rows** (271-311): `| File + identifier | Kind | Wrapper / invariant |`, grouped as `encapsulated` or `floor`.

The reserved-module row to update when Area F lands (line 109): "`ds` reserved Roaring SIMD module | **approved by D-09 and D-20, NO CODE** until Phase 5".

Stale content to replace, not extend: the "Out of scope" table (lines 313-322) lists `x86_64` files `node0.rs`, `vsvm.rs` and `devcons.rs`, which are not in `x86_64/src` at HEAD, and gives 81 tokens. The x86_64 inventory must be re-measured from the tree.

#### `xtask/src/lintcontract.rs` (R3 amendment, D-38)

**Analog:** itself.

**The skip to extend by one named path** (lines 263-269):
```rust
    let src = workspace.join("ds/src");
    let tpht = src.join("tpht");
    for path in sorted_files(&src) {
        if path.starts_with(&tpht) {
            continue;
        }
```
**The forbid-layout rule that the new `mod` line must satisfy** (lines 287-306): every module declared in `ds/src/lib.rs` other than `tpht` must have `#[forbid(unsafe_code)]` on the same or the previous line. The SIMD module declaration needs a named exception here, as `tpht` has.

**Module doc rule text** (lines 14-17) and the error text (line 279, "only ds/src/tpht/ may hold unsafe in ds/ (D-09)") change in the same plan.

**Fixture and test form** (lines 569-576, 613-635): forbidden tokens are built with `["uns", "afe"].concat()` so the module's own source never matches a scan; each case writes one file into a scratch fixture and asserts the error names file and line:
```rust
        let dir = fixture("unsafe-tpht-extra");
        write(&dir, "ds/src/tpht_extra.rs", &unsafe_block());
        let text = check(&dir)
            .expect_err("ds/src/tpht_extra.rs is outside the allowed directory")
            .to_string();
        assert!(text.contains("ds/src/tpht_extra.rs:2"), "names the file and line: {text}");
```
Add: the reserved path accepted; a sibling path next to it rejected.

**R6** (lines 439-452) checks `[lints] workspace = true` for `crypto` and `ds` only. Adding `x86_64/Cargo.toml` to that list is the mechanical lock for D-25.

---

### Area C: AVX and XSAVE

#### `x86_64/src/fpu.rs` (format selection)

**Analog:** itself.

**State type with const asserts** (lines 65-72); the asserts become 832 and 64:
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C, align(16))]
pub struct FpState {
    pub fx: [u8; 512],
}

const _: () = assert!(core::mem::size_of::<FpState>() == 512);
const _: () = assert!(core::mem::align_of::<FpState>() == 16);
```
**`DEFAULT` as a `const` block** (lines 82-89): extend it with the zeroed XSAVE header; do not build it at run time.

**Compare through `same_user_state`, not `==`** (lines 107-117). RESEARCH Pitfall 3: add the AVX bytes with `XSTATE_BV` honoured.

**Always initialise a buffer before saving into it** (lines 289-292):
```rust
        let mut st = FpState::DEFAULT;
        save(&mut st);
        st
```
**Per-CPU split that keeps host tests possible** (lines 155-173): `this_cpu()` has a `#[cfg(not(test))]` and a `#[cfg(test)]` body, and every public function has an `_on(cpu, ...)` twin that tests call with a leaked `CpuFp`. The boot-time format flag follows the same split (a static in the kernel build, an injectable value in tests).

**Module doc** (lines 1-54) states the FXSAVE-only rule and "CR4.OSXSAVE is never set"; it is rewritten in the same plan.

**Boot self-test** (lines 465-523): `image(seed)` builds a distinct valid state, the test installs it, changes it, runs a section, and compares with `same_user_state`. The AVX leg adds `ymm` content and runs only when CPUID reports AVX.

#### `x86_64/src/cpu.rs` (feature helpers)

**Analog:** lines 222-258. Pure functions of the raw CPUID register values, host-testable, each with the doc sentence naming what TCG `qemu64` reports.
```rust
pub fn has_tsc_deadline(cpuid1_ecx: u32) -> bool {
    cpuid1_ecx & (1 << 24) != 0
}
```
Leaf 7 helpers must take the max leaf first (lines 249-258):
```rust
pub fn has_rdseed(max_leaf: u32, cpuid7_ebx: u32) -> bool {
    max_leaf >= 7 && cpuid7_ebx & (1 << 18) != 0
}
```
`has_avx2` (leaf 7 EBX bit 5) and `has_clflushopt` (leaf 7 EBX bit 23) copy this second form. `cpuid(leaf, subleaf)` at lines 185-206 saves and restores `%rbx` around the instruction; reuse it, do not write another.

#### `x86_64/src/main.rs` (enable + boot line)

**Analog:** lines 298-304. The XSAVE enable goes before this call; the format line copies its shape so a gate can match one literal prefix.
```rust
    #[cfg(dev_exec)]
    match fpu::selftest() {
        Ok(sum) => println!(
            "fpu: kernel-SIMD selftest ok (user state parked and restored across the section; xor_fold16 {sum:#034x})"
        ),
        Err(why) => panic!("fpu: kernel-SIMD selftest FAILED: {why}"),
    }
```
Note the `#[cfg(dev_exec)]`: the selftest runs only in exec profiles. The format line must print in every boot (D-30), so it is not under that cfg.

---

### Area D: `crypto/`

#### `crypto/src/key.rs` (page-key derivation)

**Analog:** `k_file` (lines 83-101) for a derivation with a fixed-width context, and `derive_labelled` (103-123) as the primitive to call.
```rust
pub fn k_file(k_user: &Key, qid_path: u64, qid_vers: u32) -> Key {
    // The fixed-width `info` (T-03-17): 8 bytes little-endian path, then
    // 4 bytes little-endian vers, exactly 12 bytes with no padding.
    let mut info = [0u8; 12];
    info[..8].copy_from_slice(&qid_path.to_le_bytes());
    info[8..].copy_from_slice(&qid_vers.to_le_bytes());
```
`derive_labelled` concatenates label and context with no framing, so labels must be fixed length (doc lines 105-109). Separate cipher and MAC keys per domain: `KeyCapability::new(k_dec, k_mac)` (lines 152-156) already takes two.

#### `crypto/src/session.rs` (cipher half for D-18)

**Analog:** `DirectionKeys` (lines 101-111) and its `keystream` (361-368). The new type holds `k_enc` and `prefix` only, has no `mac()`, no accessor, no `Debug`.
```rust
#[derive(Clone)]
pub struct DirectionKeys {
    k_enc: Key,
    k_mac: Key,
    prefix: Zeroizing<[u8; PREFIX_LEN]>,
}
```
```rust
    pub fn keystream(&self, seq: u64, counter: u32, buf: &mut [u8]) -> crate::Result<()> {
        let mut nonce = [0u8; 12];
        nonce[..PREFIX_LEN].copy_from_slice(&self.prefix[..]);
        nonce[PREFIX_LEN..].copy_from_slice(&seq.to_le_bytes());
        crate::page::chacha20_xor(self.k_enc.bytes(), &nonce, counter, buf)
    }
```
**Zeroize observation without an accessor** (lines 310-330): `is_zeroized()` folds every byte into one accumulator and returns a `bool`. The retained-key lifetime pins use the same form.

#### `crypto/src/page.rs` (page cipher, D-32 accessor)

**Analog:** the typestate in `crypto/src/key.rs:129-235`. Private state, no accessor, `seal` present on `ReadWrite` only, and a `compile_fail` doctest proving the absence (lines 197-219). The D-32 closure-scoped accessor is the first function in `crypto` that lends key bytes outside the crate; today the rule is stated at `key.rs:76-80`:
```rust
    /// The key bytes, for derivation inside `crypto/` only.  Deliberately
    /// `pub(crate)`: no public function or method returns key bytes.
    pub(crate) fn bytes(&self) -> &[u8; KEY_LEN] {
```
That doc comment, the `Key` doc at lines 48-52 and the `DirectionKeys` doc at `session.rs:101-105` all need the dated T-04-03 amendment.

The scalar reference the vector routine must equal is `chacha20_xor(key, nonce, counter, buf)` at `page.rs:65-96`; it rejects block index `u32::MAX` before writing anything (lines 81-84).

#### Known-answer tests

**Analog:** `crypto/tests/keys.rs:1-49` (public API, compare with `Key::ct_eq`) and `crypto/src/session.rs:429-452` (in-crate, raw bytes). The expected value is computed outside the crate and the command is recorded in evidence:
```rust
/// The known answer for `k_user()`, `KAT_PATH`, `KAT_VERS`, computed
/// independently of this crate (Python `hmac` + `hashlib`, ...).
/// The command and its output are recorded in `evidence/PINS-03-04.md`
/// (`KAT k_file`).
const KAT_EXPECTED: &str = "ce90009aebce083580333aeaf1420104c25b12b6a1151dfe8fd5b2420cf10afe";
```
Fixed inputs are the byte ramps `0x00..=0x1f` (secret) and `0x20..=0x3f` (transcript); `hex32` is the local decoder (lines 19-29).

---

### Area E: `port/` core

#### Module declaration (`port/src/lib.rs`)

**Analog:** lines 88-96. Each new module gets a comment naming the phase, the decision, what it holds, and why it keeps the forbid, then the attribute.
```rust
// fid (Phase 4, 04-09, D-22): the `9P2000.aead` hybrid handle layouts — ...
// Safe code only: the
// 64-byte entry is packed field by field with `to_le_bytes`, never transmuted,
// so the module keeps the forbid (COMPENDIUM listing 9.6's `PodSlot` +
// `transmute` is not copied).
#[forbid(unsafe_code)]
pub mod fid;
```
The header comment at lines 10-19 lists the unsafe-holding modules by name; it stays true only if all five new modules carry the forbid. The crate result type is `pub type Result<T> = core::result::Result<T, &'static str>;` (line 165); error strings are prefixed with the module name (`"fid: a table holds 1 to 256 shards ..."`, `fid.rs:611-615`).

#### `port/src/segcrypt.rs`, `palloc.rs`, `pager.rs` (module shape)

**Analog:** `port/src/fid.rs`.

**Module doc** (lines 1-119): one `//!` title line with requirement and decision ids, then `# ` sections for layouts, invariants, lock order, and what the module does not claim.

**Imports** (lines 121-130):
```rust
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crypto::key::Key;
use ds::roaring::RoaringBitmap;
use ds::tpht::flattened::FlattenedTphtShard;
use ds::tpht::{HOME_SLOTS, NoProbe, Probe, TinyPtr, TphtError};

use crate::mcslock::{Lock, LockNode};
```

**Roaring sets under a named lock** (lines 510-566), the form for the clean, dirty and pinned pools:
```rust
struct ActiveSlots {
    ids: RoaringBitmap,
    keys: BTreeMap<u32, u64>,
}
...
pub struct FidShard<P: Probe = NoProbe> {
    tpht: FlattenedTphtShard<FID_ENTRY_WORDS, P>,
    active: Lock<ActiveSlots>,
    reclaimed: Lock<RoaringBitmap>,
}
```
`Lock::new("fid-active", ...)` (line 629) names each lock; the lock order is written in the type's doc comment (lines 554-561). `RoaringBitmap` API available: `insert`, `remove`, `contains`, `len`, `insert_range`, `union`, `intersection`, `difference`, `andnot` (`ds/src/roaring.rs:71-187`). `Evictable = Dirty \ (Pinned | Active)` is `dirty.difference(&pinned.union(&active))`.

**Test seam by generic parameter** (lines 562, 584-610): `FidTable<P: Probe = NoProbe>` with `new(...)` delegating to `with_probe(..., NoProbe)`. `FrameOps` and `BackingStore` follow the same form: a trait parameter with the real implementor supplied by the arch crate and a recording fake in tests.

**Byte packing without `unsafe`** (lines 321-329): fields go through `to_le_bytes` into a byte array; nothing is transmuted. Page records and tag-binding inputs use this.

#### `port/src/superpage.rs`

**Analog:** `port/src/fid.rs:281-319`.
```rust
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FidEntry {
    ...
}

const _: () = {
    assert!(core::mem::size_of::<FidEntry>() == 64);
    assert!(core::mem::align_of::<FidEntry>() == 64);
};
```
With eight atomic words (RESEARCH Open Question 7) the derive list changes (atomics are not `Copy` or `Eq`); the const assert block is copied as is.

#### `port/src/pager.rs` (decision logic, host-tested)

**Analog for the decision:** `aarch64/src/fault.rs:91-116`: resident means a protection fault and a kill; otherwise classify by segment and page in.
```rust
    let already_mapped = proc.mapping_log.iter().any(|m| m.va == page_va);

    match segments.classify(va) {
        Some(SegKind::Text) if !already_mapped => {
            page_in_from_image(&mut proc, page_va, SegKind::Text)
        }
        ...
        _ => unresolvable(frame, class, far, access),
    }
```
**Analog for keeping policy pure and host-tested:** `x86_64/src/platform/q35/churngate.rs:106-135`. Small pure functions return a value or `Result<(), String>`; the kernel-only code calls them.
```rust
#[cfg(any(test, dev_churn))]
fn slope_ok(measured: u64, ceiling: u64) -> Result<(), String> {
    if measured <= ceiling {
        return Ok(());
    }
    Err(alloc::format!(
        "per-exec cursor delta {measured} over ceiling {ceiling} (unbounded drain — the #44 shape)"
    ))
}
```

#### Test fixtures under `port/tests/common/`

**Analog:** `port/tests/common/aead_link.rs:1-112` and `port/tests/common/mod.rs` (the `mod.rs` is required so Cargo does not build the fixture as its own test target). A test file starts with `mod common;` then `use common::aead_link::{...}` (`port/tests/aead_transport.rs:13-20`). Fixtures record what happened in public fields or enums (`Stage`, line 73) so a test asserts the sequence of calls, which is what the "frame never mapped before its tag verifies" pin needs from a recording `FrameOps`.

Tests that need `unsafe` or std-only scaffolding and cannot live inside a forbidden module go at the crate root as `port/src/<name>_tests.rs` with a `#[cfg(test)] mod` line and a comment giving the reason (`port/src/lib.rs:120-163`).

---

### Area F: Roaring SIMD

#### `ds/src/lib.rs` (mounting the reserved module)

**Analog:** `port/src/lib.rs:41-50`.
```rust
// devip_waker (the file is port/src/devip/waker.rs) — the phase-08 ONE
// named unsafe site in the devip work: ...  It is mounted at the crate root via #[path]
// precisely BECAUSE the forbid on `devip` above propagates into
// submodules ...
#[path = "devip/waker.rs"]
pub mod devip_waker;
```
`ds/src/lib.rs:13-25` already reserves the place and says the module "will be mounted at the crate root with #[path]". That comment ("no code in this phase") is rewritten when the module lands. Two arch files need `#[cfg(target_arch = "...")]` on their `mod` lines.

#### `ds/src/simd/{x86_64,aarch64}.rs` (inline-asm kernels)

**Analog:** `xor_fold16` on each architecture.

x86_64 (`x86_64/src/fpu.rs:392-428`): the guard is a parameter that proves the section; AT&T syntax; numeric local labels; operands by explicit register; `clobber_abi("C")` so host builds declare the vector registers; a SAFETY comment that states the bytes read, the registers written, and what the guard proves.
```rust
pub fn xor_fold16(_simd: &KernelSimd, data: &[u8]) -> u128 {
    assert!(data.len().is_multiple_of(16), ...);
    let (lo, hi): (u64, u64);
    // SAFETY: reads exactly data.len() bytes from data (rcx = the block
    // count); writes only XMM0/XMM1, rflags and the declared GPRs.
    // `_simd` proves a live section: ...
    unsafe {
        core::arch::asm!(
            "pxor %xmm0, %xmm0",
            ...
            inout("rsi") data.as_ptr() => _,
            inout("rcx") data.len() / 16 => _,
            out("rax") lo,
            out("rdx") hi,
            clobber_abi("C"),
            options(att_syntax, nostack, readonly),
        );
    }
```
aarch64 (`aarch64/src/fpu.rs:440-460`): the same shape with `".arch_extension simd"` as the first line and `{{v1.16b}}` brace escaping.

Each kernel has a public scalar twin (`xor_fold16_scalar`, x86_64 line 431) and a test that compares the two over several lengths and rejects a trivially zero result (`x86_64/src/fpu.rs:732-745`).

Do not use `#[target_feature]`: module doc lines 50-54 and RESEARCH question 1.

#### `ds/src/setops.rs` (dispatch + tests)

**Analog:** itself. The four signatures at lines 20-65 do not change (the module doc, lines 5-10, promises a replacement body needs no caller change). The module doc's last line, "No SIMD code lands here in this phase", is rewritten.

**Test form to extend for vector-equals-scalar** (lines 86-188): eight literal seeds, a local `SplitMix64`, a bit-by-bit `naive_bitmap` that shares no code with the kernels, `random_words` that mixes all-zero, all-one and random words, and assertions that name the seed:
```rust
            assert_eq!(out, naive, "bitmap_and words differ (seed {seed:#018x})");
            assert_eq!(card, naive_card, "bitmap_and cardinality differs (seed {seed:#018x})");
```
Asm cannot run under Miri: gate the vector tests with `cfg(not(miri))` (RESEARCH Pitfall 6); `ds/src/tpht/measure.rs:236` shows the crate's existing `miri_scaled` helper for the tests that do run there.

---

### Area G and H: the pagers

#### Restricted section in `x86_64/src/fpu.rs` and `aarch64/src/fpu.rs` (D-12)

**Analog:** `KernelSimd`, identical on both architectures (x86_64 lines 325-386, aarch64 363-423).

**Guard type** (x86_64 lines 327-334): private fields, a `PhantomData<*mut ()>` to make it `!Send`.
```rust
pub struct KernelSimd {
    parked: *mut FpState,
    cpu: &'static CpuFp,
    _not_send: PhantomData<*mut ()>,
}
```
**Begin** (lines 342-363): set the per-CPU in-section flag first and panic by name if it was already set; assert interrupts disabled under `#[cfg(not(test))]`.
```rust
    if cpu.set_in_section(true) {
        panic!(
            "fpu: nested kernel_begin (a KernelSimd section is already open; sections are not reentrant)"
        );
    }
    #[cfg(not(test))]
    assert!(
        interrupts_disabled(),
        "fpu: kernel_begin with interrupts enabled (a section must not be interruptible)"
    );
```
**End in `Drop`** (lines 365-379), then clear the flag last. **Scoped form** (lines 383-386): `with_simd(|simd| ...)`.

The restricted section shares the same `in_section` flag, so the scheduler hooks' existing guard (`assert_outside_kernel_simd`, lines 235-240) covers it with no change, and nesting either kind inside the other panics by name. It does not touch `cpu.owner()` and does not call `save_raw`.

**Tests to copy** (lines 601-691): the section test, the per-CPU bookkeeping test with two leaked `CpuFp`, and the three `#[should_panic(expected = "fpu: ...")]` tests. Panic messages are matched by prefix, so new messages keep the `fpu: ` prefix and a distinct name.

Host-test caveat on aarch64 (lines 469-474): a host build's own code uses vector registers, so register-preservation tests put restore and save in one asm block. The restricted-section preservation test needs the same care.

#### `cryptpage.rs` (both architectures)

**Partial analog:** asm conventions from `xor_fold16` above. For `global_asm!`, the tree's only precedent is `x86_64/src/main.rs:46`: `core::arch::global_asm!(include_str!("l.S"), options(att_syntax));`. RESEARCH "Code Examples" has a routine verified to assemble on the kernel target; that is the reference for the body, since no vector cipher exists in the tree.

Rules taken from the code as it is:
- The routine takes the restricted-section guard as its proof parameter (`_simd: &KernelSimd` precedent).
- Key bytes arrive as a borrowed `&[u8; 32]` from `port` through the `FrameOps` trait (D-32); the arch crate may not name `crypto` (lint rule R4, `xtask/src/lintcontract.rs:18-20`, checked at 336+).
- A public scalar twin and an equality test over random inputs, as `xor_fold16_scalar` and its test.
- The boot self-test pattern (`fpu::selftest`, lines 476-523) is the form for a "vector equals scalar" boot line under the AVX2 TCG model.

#### `x86_64/src/shadow.rs`, `mmu.rs`

**Analog:** `x86_64/src/proc.rs`.

**PTE constants** (lines 155-166). Text is mapped writable today, which RESEARCH Pitfall 7 says the sealed profiles must not do; a read-only executable text kind and bit 6 (dirty) are new constants beside these:
```rust
const PTE_P: u64 = 1 << 0;
const PTE_RW: u64 = 1 << 1;
const PTE_US: u64 = 1 << 2;
const PTE_NX: u64 = 1 << 63;
pub(crate) const PTE_USER_TEXT: u64 = PTE_P | PTE_RW | PTE_US;
pub(crate) const PTE_USER_DATA: u64 = PTE_P | PTE_RW | PTE_US | PTE_NX;
```
**Map with rollback on failure** (lines 1423-1440):
```rust
        let pa = alloc_phys_page()?;
        let created = match pml4_map_user(self.pml4_pa, va, pa, kind.pte()) {
            Ok(created) => created,
            Err(msg) => {
                // The walk already freed its own intermediates; the
                // fresh leaf page never mapped — free it too or it
                // leaks (17-01, the WR-03 OOM-release family).
                free_phys_page(pa);
                return Err(msg);
            }
        };
        for c in created.into_iter().flatten() {
            self.mapping_tables.push(c);
        }
        self.mapping_log.push(MappingRecord { va, phys: pa as usize, size: PAGE_SIZE_4K, kind });
```
**Do not copy** `page_in_zeroed`'s allocation (line 1404): `alloc_phys_page().unwrap_or_else(|_| panic!(...))`. RESEARCH Pitfall 8: a low-memory profile must get an error, not a halt.

**Unmapped frame access:** `x86_64/src/pagealloc.rs:106-110`: "the identity map of the first 4 GiB ... already covers every page this allocator can hand out, so the returned PA doubles as its kernel VA." That is how a frame is filled before any PTE exists (MEM-01).

**`invlpg` with a host-test stub** (lines 438-445): a `#[cfg(not(test))]` body with the asm and a `#[cfg(test)] fn invlpg(_va: usize) {}`. `CLFLUSHOPT`, `CLFLUSH` and `SFENCE` wrappers copy this pair.

**`mapping_log` record** (lines 195-203); every walker of it is a call site for the sealed profile (RESEARCH Pitfall 13): `ensure_user_resident_inherent` 1368-1386, `ensure_stack_resident` 1390-1398, `free_user_mappings` 1447+, `rfork_copy` 2708+, and the RFMEM `SHARED_PAGES` table at 634-701.

#### `x86_64/src/trap.rs` (`#PF`)

**Analog:** itself, lines 826-850. The pager call goes ahead of this block for vector 14 from ring 3; the kill path is unchanged and already routes through the exit tail, so one process dies and the kernel lives.
```rust
            let cr2 = {
                let v: u64;
                // SAFETY: a register read with no memory effect.
                unsafe { core::arch::asm!("mov %cr2, %rax", out("rax") v, options(att_syntax, nomem, nostack, preserves_flags)) };
                v
            };
            ...
            match crate::proc::current() {
                Some(mut p) => crate::syscall::exit_tail(&mut p, b"sys: trap"),
                None => crate::println!("user trap with no current process"),
            }
```
The tamper kill uses the same `exit_tail(&mut p, b"...")` call with its own status string. The comment at lines 820-825 ("this era has no trap-context pager") is rewritten.

#### `aarch64/src/fault.rs`

**Analog:** itself.

- **Trap-context invariants** (module doc lines 7-21): IRQs stay masked, no locks on entry, the byte source never RPCs. The sealed path keeps all of them.
- **Kill** (lines 329-338): `kill_current_process()` routes through `crate::syscall::exit_tail(&mut proc, b"fault")`.
- **Evidence markers** (lines 181-185, 253-257): one `println!` per page-in with a fixed prefix (`fault: text page-in va=...`). Sealed faults get their own prefix so default boots can be asserted to print none (MEM-11).
- **What the sealed path must not copy** (lines 132, 146-150): `map_active_user(page_va, entry)` first, then a write through the user VA. Fill through the KZERO alias instead (`aarch64/src/kmem.rs:132`, `physaddr_as_ptr_mut_offset_from_kzero`).
- Line 128 maps text with `Entry::rw_user_text()`; sealed profiles need a read-only text entry (Pitfall 7).
- Lines 124-125 and 246-247 panic on allocation failure (Pitfall 8). Line 246 also has two statements on one line (`fn page_in_zeroed(...) {    let pa = ...`), which is one of the pre-existing fmt failures STATE.md mentions; a plan that edits this function should expect the diff to include that line.

#### `x86_64/src/usermem.rs` / `aarch64/src/usermem.rs` (MEM-10)

**Analog:** itself. The trait is the seam (lines 39-48):
```rust
pub trait UserAddressSpace {
    fn validate_user_range(&self, va: u64, len: u64, writable: bool) -> Result<(), &'static str>;
    fn ensure_user_resident(&mut self, start: usize, end: usize);
}
```
The fused constructor (lines 62-70) is where a pin is taken. `UserSlice` holds only `{ ptr, len }` (lines 53-56), so release needs a separate guard that records the frame range (RESEARCH Pitfall 9).

**Mock for tests** (lines 237-273): a struct with a real byte buffer and a `resident_calls: Vec<(usize, usize)>` log; pin and unpin calls are recorded the same way.
```rust
        fn ensure_user_resident(&mut self, start: usize, end: usize) {
            self.resident_calls.push((start, end));
        }
```
The SAFETY comment at lines 89-95 states "its user pages are never unmapped under a live syscall"; eviction makes that false unless the range is pinned, so the comment is rewritten with the pin as the invariant.

#### Kernel-context gate driver (`platform/q35/<name>.rs`, `platform/virt/<name>.rs`)

**Analog:** `x86_64/src/platform/q35/churngate.rs` (aarch64 mirror: `aarch64/src/platform/virt/churngate.rs`).

- **File header** (lines 1-55): what the driver proves, the marker vocabulary as a list, and the concurrency note (a real kernel-context proc, never ring 3, always IRQ-masked).
- **Constants with their derivation in the doc comment** (lines 69-94).
- **cfg split** (lines 59-67, 108, 186): pure halves under `#[cfg(any(test, dev_churn))]`, kernel-only items under `#[cfg(dev_churn)]`. The module itself is declared unconditionally (`x86_64/src/platform/q35/mod.rs:19`) so the host battery compiles the pure halves.
- **Marker builders as pure functions** (lines 140-177), with a fail-closed terminal marker: DONE is printed only on success, and a failure prints `<gate>: FAIL <reason>`.
- **Bootstrap** (lines 234-280): `crate::exec_prologue(facts)`, walk the binaries by path, `crate::proc::new_user_proc("...")`, `crate::proc::seed_kernel_context(&mut driver, main_fn)`, stash inputs in a `SyncUnsafeCell<Option<Inputs>>`, then `crate::proc::enter_sched_loop_from_boot()`.
- **Dispatch in `main.rs`** (lines 306-310):
```rust
    #[cfg(all(dev_exec, any(not(dev_churn), dev_bootchain)))]
    exec_bootstrap(facts);

    #[cfg(all(dev_exec, dev_churn, not(dev_bootchain)))]
    platform::q35::churngate::bootstrap_and_run(facts);
```
A third arm for the sealed drivers must keep these three mutually exclusive.

---

### Area I: Ingress

#### `port/src/devmnt.rs`

**Analog:** `frame_one`, lines 517-552. The read-until-full loop with EOF handling is copied once per destination (head buffer, then each frame); the `alloc::vec![0u8; size - 4]` at line 541 is the allocation that ingress removes.
```rust
        while got < 4 {
            let n = io.0.read_some(&mut hdr[got..4])? as usize;
            if n == 0 {
                return Err("devmnt: eof waiting for a reply");
            }
            got += n;
        }
        let size = u32::from_le_bytes(hdr) as usize;
        // mntchk: the claimed size is untrusted until it passes
        // BOTH bounds ...
        if size < 7 { ... return Err("devmnt: runt reply message"); }
        if size > msize as usize { ... return Err("devmnt: reply exceeds negotiated msize"); }
```
The size bounds come before any body byte is read; D-36's page-count derivation sits after these checks and before placement. The reader-token rule in the doc comment (lines 509-516) covers the whole call, so the per-page reads stay inside the same token.

`MntIo::read_some(&mut self, buf: &mut [u8]) -> Result<u64>` (lines 245-250) already takes the destination slice. `ChanIo` accepts only `DEV_PIPE` and `DEV_SSL` (lines 271-285).

#### `port/src/chan/transport.rs`

**Analog:** `Session::open`, lines 851-936. Every rejection is the same two lines, and the order is fixed: authenticated, not closed, deadline, minimum size, MAC, then sequence, then header checks, then TLVs, then the body.
```rust
        // Verify before acting on any field but Size (D-07).
        let mut mac = self.recv_keys.mac();
        mac.update(&buf[..size - MAC_LEN]);
        ...
        if mac.verify(&tag).is_err() {
            self.fail();
            return Err(Closed);
        }
```
A scatter variant feeds `mac.update(part)` once per part (`PacketMac::update`, `crypto/src/session.rs:268`) in wire order and skips `crypt_body` for data pages. The doc sentence at lines 847-850 ("`Size` is the only field read before the MAC verifies") is the text D-36 amends.

**Re-key cutover** (lines 1312-1317): the session's own copy is zeroized at the marker and stays so; D-18's retained cipher half is a separate object owned by page records.
```rust
        let old = core::mem::replace(&mut self.recv_keys, new_keys);
        self.rekey.recv_superseded = Some(old);
        if let Some(keys) = &mut self.rekey.recv_superseded {
            keys.zeroize();
        }
        self.recv_seq = 0;
```

#### `port/tests/aead_ingress.rs`

**Analog:** `port/tests/aead_transport.rs:1-40`. Module doc names the requirement ids and states that expected values are computed in the test, not by calling the code under test ("The wire pins never call `Session::open` ... to obtain an expected value: they hold their own `DirectionKeys` ... The page size 4096, the 64 blocks per page and the offsets are literals here."). Imports go through `mod common;` and `common::aead_link::{AeadLink, ClientEnd, ServerEnd, connect, ...}`; the fixed secret is the `0x00..=0x1f` ramp.

---

### Area J: Profiles, probes, gates

#### Profile TOMLs

**Analog (x86_64):** `x86_64/lib/config_q35churn.toml`. Header comment explains what the profile proves and which cfg its `dev` key compiles; then the fixed stanzas.
```toml
[config]
platform = "q35"
dev = ["exec", "churn"]

[qemu]
machine = "q35"
accel = "kvm"
cpu = "host"
smp = 1
memory = "1024M"
```
The `[qemu.netdev]` stanza (lines 45-64) is carried even by net-free profiles, because every q35 profile's `finish()` asserts the virtio-net discovery line. `[dist] module = "image:marker"` (line 73) names the manifest whose image is baked on demand; a profile with generated probes names its own image.

The committed q35 profiles say `accel = "kvm"`, `cpu = "host"`; TCG runs override with `--accel tcg --cpu <model>` (or `R9_ACCEL`, `R9_CPU`). The AVX2 model string from RESEARCH question 3 is passed that way; no TOML carries it today.

**Analog (aarch64):** `aarch64/lib/config_virtchurn.toml`: `dev = ["exec", "ether", "churn"]` (line 42), `cpu = "cortex-a72"`, `accel = "tcg"`, `initrd = { file = "target/bootfs.paq", addr = 0x48000000 }`.

**check-cfg predeclare** (`Cargo.toml:15-53`): every new `dev_<key>` is added to the workspace list, with a comment sentence in the block above saying which TOML emits it and that the declaration lands before or with the first code use.

#### `xtask/src/main.rs` (a new boot profile, end to end)

**Analog:** the churn gate. Seven places, in file order:

1. **Marker constants** with a doc block (lines 1010-1025):
```rust
const Q35_CHURN_HEAP_MARKER: &str = "churngate: heap ";
const Q35_CHURN_DONE_MARKER: &str = "churngate: DONE iterations=";
const Q35_CHURN_FAIL_MARKER: &str = "churngate: FAIL";
```
2. **`BootStage` variant** for the terminal marker (lines 1544-1557).
3. **`BootProfile` variant** with a doc comment listing the stages and the post-pass (lines 1749-1764).
4. **Name-selected profile bool** (lines 2171-2184). The profile's `dev` list contains `exec`, so the bool is checked before the generic exec dispatch:
```rust
        let q35churn_profile = self.arch == Arch::X86_64 && self.config_name == "q35churn";
```
5. **Fail-fast on the FAIL marker** from any stage (lines 2255-2267).
6. **Stage transition and terminal match** (lines 2499-2526). Wait for the complete line, not the prefix:
```rust
                    if let Some(line) =
                        line_containing(&capture[bind_after..], Q35_CHURN_DONE_MARKER.as_bytes())
                    {
                        if line.contains("semgens=") {
                            return self.finish(log_abs, &capture, BootProfile::X64Churn);
                        }
                    }
```
7. **`finish` arm and shared post-pass** (lines 2967-2984 and 4318-4384). One marker-vocabulary struct with a const per architecture, and a post-pass that parses every number from the kernel's own lines:
```rust
struct ChurnMarkers { heap: &'static str, slope: &'static str, done: &'static str, ... }
const Q35_CHURN_MARKERS: ChurnMarkers = ChurnMarkers { ... };
const VIRT_CHURN_MARKERS: ChurnMarkers = ChurnMarkers { ... };

fn churn_post_pass(capture: &[u8], m: &ChurnMarkers) -> Result<ChurnVerdict> {
    let field = |line: &str, key: &str| -> Option<u64> {
        line.split_whitespace()
            .find_map(|t| t.strip_prefix(key).and_then(|v| v.parse().ok()))
    };
```
The four sealed profiles are two gates times two architectures, which is exactly this shared-vocabulary shape. A host test pins that the aarch64 vocabulary mirrors the x86_64 one (`virtchurn_markers_mirror_q35churn`, line 5539).

`q35lowmcfg` (lines 1231, 2903) is an unrelated profile with a similar name; `q35lowmem` must not be matched by any `starts_with`-style check.

#### Generated `a.out` probes (D-34)

**Analog for the generator module:** `xtask/src/rforkh.rs`. A pure `render()` that returns the whole artifact, a `write(workspace)`, a `check(workspace)` that fails with the regenerate command, and tests that pin exact content.
```rust
pub fn check(workspace: &Path) -> Result<()> {
    let path = workspace.join(HEADER_PATH);
    let committed = fs::read_to_string(&path).map_err(|e| {
        format!("rfork-header: read {}: {e} (run `cargo xtask rfork-header`)", path.display())
    })?;
    if committed != render() {
        return Err(format!(
            "rfork-header: {} is stale against port/src/rfork.rs (run `cargo xtask rfork-header`)",
            path.display()
        ).into());
    }
    Ok(())
}
```
Tests pin individual output lines and stability (`render() == render()`), lines 105-150. For a binary artifact the pins are byte literals: the 40-byte header fields and the instruction encodings.

**Analog for registration:** `xtask/src/image.rs`.
- registry (lines 73-77): `const GENERATORS: &[&str] = &["authdomain", "ndb-template"];`
- manifest validation arm (lines 404-435): `require_field(file, image, "generator")?; known_generator(...)`, then a per-generator `match` that rejects fields the generator does not take.
- staging arm (lines 868-906): `match generator { "authdomain" => { ... copy_with_mode(&src, &dest, mode)?; } ... }`.
- rejection test (lines 2875-2896): an unknown name is `IMAGE-GENERATOR`.

**Manifest entry form** (`xtask/src/image.rs:34-37` and `xtask/manifests/amd64/marker.toml:31-35`):
```toml
[[file]]
path = "/amd64/churnmark"
source = "harvey"
from = "build/x86_64/smoke-out/churnmark"
mode = "0755"
```
A generated probe replaces `source`/`from` with `source = "generated"` and `generator = "<name>"`; `mode` is required for `generated` (doc lines 116-118), and `replaces` is rejected (lines 407-414). `marker.toml` is gated by `--compare-reference` against Harvey's reference bake (header lines 8-12), so generated probes go in a separate image manifest, not in `marker.toml`.

---

## Shared Patterns

### 1. Unsafe placement (applies to every new file)

| Crate | Rule | Source |
|---|---|---|
| `port` | per-module `#[forbid(unsafe_code)]`; three named exceptions | `port/src/lib.rs:10-20`; `UNSAFE-AUDIT.md:213-221` |
| `ds` | forbid everywhere except `tpht` and the one reserved SIMD module | `ds/src/lib.rs:13-32`; lint rule R3 |
| `crypto` | crate-level `#![forbid(unsafe_code)]`; no `unsafe` token at all | lint rule R2, `xtask/src/lintcontract.rs:235-259` |
| arch crates | `unsafe` allowed; one `// SAFETY:` per block, enforced by `clippy::undocumented_unsafe_blocks = "deny"` | `Cargo.toml:12-13`; `x86_64/src/fpu.rs:180-217` |
| arch crates | may not name `crypto` or `ds` in any dependency table | lint rule R4, `xtask/src/lintcontract.rs:18-20` |

Every plan that adds or removes an `unsafe` site updates `UNSAFE-AUDIT.md` in the same commit (the reconciliation is a sum that must hold, lines 51-54).

### 2. Kernel SIMD goes through an `fpu` guard

**Source:** `x86_64/src/fpu.rs:31-54` (rules), `392-428` (routine form).
**Apply to:** `cryptpage.rs` on both architectures, `ds/src/simd/*`, both restricted sections.
The guard is a function parameter; vector instructions are inline or global asm; `clobber_abi("C")` is named for host builds; no `#[target_feature]`; a public scalar twin and an equality test accompany every routine.

### 3. Pure decision helper, then hardware

**Source:** `x86_64/src/cpu.rs:222-258`; `x86_64/src/platform/q35/churngate.rs:106-177`.
**Apply to:** CPUID feature selection (FP format, cipher path, flush instruction), pager policy, marker text.
Anything that decides is a pure function of plain integers with a host test; the function that touches hardware is a thin caller. This is also how x86_64 logic gets tested on the aarch64 research host where `-p x86_64` tests do not run.

### 4. Host-test split inside arch crates

**Source:** `x86_64/src/fpu.rs:155-173` (`this_cpu` twin bodies), `314-323` (`#[cfg(not(test))]` hardware probe), `x86_64/src/proc.rs:438-445` (`invlpg` with a `#[cfg(test)]` no-op).
**Apply to:** every new privileged instruction wrapper (`xsetbv`, `clflushopt`, `sfence`, PTE writes).

### 5. Layout pins as const asserts

**Source:** `port/src/fid.rs:316-319`; `x86_64/src/fpu.rs:71-72`; `aarch64/src/fpu.rs:76-80` (with `offset_of!`).
**Apply to:** `FpState` (832, align 64), `SuperpageResidency` (64, align 64), page records.

### 6. Known-answer vectors computed outside the crate

**Source:** `crypto/tests/keys.rs:31-49`; `crypto/src/session.rs:429-452`; evidence file `.planning/milestones/v3.0-phases/04-.../evidence/PINS-04-03.md`.
**Apply to:** page-key derivation labels, tag construction, vector-equals-scalar reference outputs.
The constant is copied from a recorded command's output and "never re-derived here".

### 7. Seeded randomized tests with no dependency

**Source:** `ds/src/setops.rs:86-116` (eight literal seeds, local `SplitMix64`); the same finalizer is `mix64` in `port/src/fid.rs:503-508`.
**Apply to:** pool algebra against a `BTreeSet` model, vector kernels, cipher equality. No `rand`, no `proptest`.

### 8. Boot markers and fail-closed gates

**Source:** `x86_64/src/platform/q35/churngate.rs:39-44, 159-177`; `xtask/src/main.rs:1010-1025, 2255-2267, 4359-4369`.
**Apply to:** all four sealed profiles, the FP-format boot line, the tamper and ingress probes.
The kernel prints `<gate>: FAIL <reason>` and withholds DONE; xtask fails fast on FAIL, waits for the complete DONE line, and re-derives the verdict from raw markers.

### 9. Error and kill conventions

- `port`: `crate::Result<T>` with `&'static str`, message prefixed by the module name (`port/src/lib.rs:165`, `port/src/fid.rs:611-615`).
- `crypto`: `crate::Result` with `crate::Error::{TagMismatch, CounterOverflow}` (`crypto/src/session.rs:287`, `crypto/src/page.rs:83`).
- Transport: any failure is `self.fail(); return Err(Closed);` (`port/src/chan/transport.rs:873-876`).
- A user process is killed through the exit tail, never by halting: `crate::syscall::exit_tail(&mut p, b"<status>")` (`x86_64/src/trap.rs:845-848`, `aarch64/src/fault.rs:329-338`).
- A panic in kernel paths names its module: `panic!("fault: ...")`, `panic!("proc: ...")`, `panic!("fpu: ...")`.

### 10. Comment and doc conventions

Module docs open with one line naming requirement and decision ids (`port/src/fid.rs:1-4`, `crypto/src/page.rs:1`). Comments cite the decision that justifies a line (`(D-12)`, `(T-04-29)`, `(RESEARCH Pitfall 4)`). When a statement stops being true it is kept as dated history beside the new text (`UNSAFE-AUDIT.md:29-32`).

---

## No Analog Found

Planner should use RESEARCH.md patterns for these; the nearest partial analog is named so the conventions still carry over.

| File | Role | Data Flow | Reason | Nearest partial |
|---|---|---|---|---|
| `x86_64/src/cryptpage.rs`, `aarch64/src/cryptpage.rs` (vector ChaCha20 body, non-temporal stores, `SFENCE`) | utility | transform | No vector cipher and no standalone `global_asm!` routine exists; `main.rs:46` only includes `l.S` | `xor_fold16` for asm conventions; RESEARCH "Code Examples" and Pattern 1 |
| XSAVE enable and the XSAVE half of `fpu.rs` (`xsetbv`, header, `XSTATE_BV`) | service | request-response | The tree is FXSAVE-only; `CR4.OSXSAVE` is never set | `save_raw`/`restore_raw`; RESEARCH question 2 table and "Boot-time XSAVE enable, in order" |
| `port/src/segcrypt.rs` `BackingStore` and eviction, dirty tracking | service | CRUD | No eviction, swap, dirty bit use or shared page object exists on either architecture | `fid.rs` for module shape only; RESEARCH Patterns 2 and 3, Pitfall 10 |
| `port/src/inflight.rs` | store | event-driven | No write-once de-duplication table exists; Funnel Hashing is absent by rule (`ds/src/lib.rs:7-10`, lint rule R1) | `fid.rs` table shape; RESEARCH Open Question 9 (host-tested only) |
| Generated `a.out` machine-code probe | utility | transform | No r9-side binary generator; every image binary is `source = "harvey"` | `rforkh.rs` (render/check/tests) + `image.rs` generator registry |
| Page-aligned `read(2)` frame swap in `syscall.rs` | controller | request-response | Every read copies through `UserSliceMut` today | existing `UserSliceMut::new` sites; rules are the planner's (CONTEXT discretion) |
| MEM-13 KVM benchmark harness and report | test (manual) | batch | No benchmark profile exists | `churngate.rs` sampling and marker lines; `04-OWNER-STOP-RUNNER.sh` for an owner-run script |

DEBT-02 and DEBT-03 are document reconciles with no code analog; their inputs are the 25 `04-NN-SUMMARY.md` files and the returned owner-stop logs in the archived Phase 4 directory.

---

## Notes for the Planner

Facts found while mapping that change what a plan must touch:

1. **`test9p` count pins.** Adding `aead_ingress` to `PORT_TESTS` breaks three assertions in `xtask/src/test9p.rs` (lines 283, 313 and 317, and the literal list at 342-347). After DEBT-01 these run inside `cargo xtask test`, so the ingress plan must advance them.
2. **`UNSAFE-AUDIT.md` x86_64 table is stale** (lines 313-322 name files that no longer exist). The D-25 plan re-measures; it cannot add to the existing rows.
3. **`fpu::selftest` runs only under `dev_exec`** (`x86_64/src/main.rs:298`). The FP-format boot line (D-30) must be outside that cfg to appear in every boot.
4. **One `in_section` flag per CPU** (`CpuFp`) serves both section kinds, so the restricted section inherits the scheduler guards and the no-nesting rule without new bookkeeping.
5. **`marker.toml` is compared against Harvey's reference bake**, so generated probes need their own image manifest and their own `[dist] module = "image:<name>"`.
6. **Profile `dev` lists contain `exec`**, so each new profile bool in `xtask/src/main.rs` must be checked before the generic exec dispatch (the ordering rule stated at lines 2171-2184).
7. **Lint rule R6 names only `crypto` and `ds`** (`lintcontract.rs:442`). Extending the list to `x86_64` makes D-25 a mechanical lock.
8. **The `ds` forbid-layout rule** (`lintcontract.rs:287-306`) rejects any module line in `ds/src/lib.rs` without a forbid except `tpht`; the SIMD module needs a named exception in the same plan as the R3 path exception.

---

## Metadata

**Analog search scope:** `x86_64/src`, `x86_64/lib`, `aarch64/src`, `aarch64/lib`, `port/src`, `port/tests`, `crypto/src`, `crypto/tests`, `ds/src`, `xtask/src`, `xtask/tests`, `xtask/manifests`, `Cargo.toml`, `UNSAFE-AUDIT.md`, `docs/threat-model.md`
**Files read (whole or in ranges):** 34
**Project instructions:** no `CLAUDE.md` and no `.claude/skills` or `.agents/skills` directory in the repository; the owner's standing rules (no AI attribution, no push without approval, Harvey read-only) apply
**Knowledge graph:** queried once for orientation; it lags HEAD, so every path and line above was confirmed by reading the file
**Pattern extraction date:** 2026-11-25
