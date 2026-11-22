# Phase 14: Harvey amd64 Userspace on r9 - Pattern Map

**Mapped:** 2026-11-08
**Files analyzed:** 27 (16 r9-side, 11 Harvey-side)
**Analogs found:** 27 / 27 (24 exact/role-match re-keys of aarch64 or in-file precedent; 3 in-place extensions whose pattern is their own existing code)

**Repo roots:** `harvey/` = `/home/req/code/harvey`, `r9/` = `/home/req/code/r9` (HEAD `029e32f`, Phase-13 complete). Every analog below was verified git-tracked (`git ls-files` non-empty in its repo).

**One-line summary:** every artifact in this phase exists twice already — as an aarch64 precedent (exec/ABI/boot chain/net gates, re-key) and as an amd64 precedent (toolchain, tier builds, k10t1 venue). The only genuinely new code is the r9 x86_64 user-mode substrate, and it has a complete aarch64 sibling to port against.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `harvey/amd64/ABI.md` | config (contract doc) | n/a (frozen spec) | `harvey/aarch64/ABI.md` | exact |
| `r9/x86_64/src/dat.rs` | model (frame layout) | event-driven (trap) | `r9/aarch64/src/trap.rs:99-161` + 9k donors | role-match (rework-in-place) |
| `r9/x86_64/src/trap.rs` | controller (entry asm) | event-driven | `r9/aarch64/src/trap.rs` + `harvey/sys/src/9k/k10/l64syscall.s` | role-match (rework + new entry) |
| `r9/x86_64/src/seg.rs` | utility (segment/MSR) | request-response | itself (`star()`/`utextsel()` dead code, lines 260-273) + `harvey/sys/src/9k/k10/vsvm.c:174-185` | exact (revive in place) |
| `r9/x86_64/src/cpu.rs` | utility (CPU/MSR) | request-response | itself (`fmask()` line 25-30, `wrmsr` line 203) | exact (revive in place) |
| `r9/x86_64/src/exec.rs` | service (a.out loader) | transform (parse→map) | `r9/aarch64/src/exec.rs` | exact (cross-repo port) |
| `r9/x86_64/src/proc.rs` | model+service (proc/sched) | event-driven | `r9/aarch64/src/proc.rs` | exact (cross-repo port) |
| `r9/x86_64/src/syscall.rs` | controller (dispatch) | request-response | `r9/aarch64/src/syscall.rs` | exact (cross-repo port) |
| `r9/x86_64/src/usermem.rs` | utility (validated user access) | request-response | `r9/aarch64/src/usermem.rs` | exact (cross-repo port) |
| `r9/x86_64/src/main.rs` (exec_bootstrap) | controller (boot orchestration) | event-driven | `r9/aarch64/src/main.rs:434-560` | exact (cross-repo port) |
| `r9/x86_64/src/platform/q35/mb2.rs` | utility (firmware parse) | transform | itself (`parse_mmap`/`parse_acpi_root`, lines 255-301) | exact (extend in place) |
| `r9/xtask/src/uregcheck.rs` | test (cross-repo gate) | transform (parse→compare) | itself (aarch64 arm, lines 28-189) | exact (extend in place) |
| `r9/xtask/src/main.rs` | test harness (boot-test) | batch | itself (arch gate lines 1471-1485) | exact (extend in place) |
| `r9/x86_64/lib/config_q35exec.toml` | config | n/a | `r9/aarch64/lib/config_virtexec.toml` | exact |
| `r9/x86_64/lib/config_q35shell.toml` | config | n/a | `r9/aarch64/lib/config_virtshell.toml` | exact |
| `r9/x86_64/lib/grub.cfg` | config (loader) | n/a | itself (menuentry, lines 10-13) | exact (extend in place) |
| `r9/flake.nix` (boot-iso param) | config (build) | batch | itself (`boot-iso` FOD, lines 105-205) | exact (parameterize in place) |
| `harvey/build/x86_64/smokesleep.c` | component (marker binary) | request-response | `harvey/build/aarch64/smokesleep.c` | exact |
| `harvey/build/x86_64/subset-build.sh` | service (build driver) | batch | `harvey/build/aarch64/subset-build.sh` | exact (re-key) |
| `harvey/build/x86_64/stage-build.sh` | service (build driver) | batch | `harvey/build/aarch64/stage-build.sh` | exact (re-key; aarch64 leg is lima-specific — amd64 is native-unshare only) |
| `harvey/build/x86_64/mkimage.sh` | service (bake) | batch (file I/O) | `harvey/build/aarch64/mkimage.sh` | exact (re-key) |
| `harvey/build/x86_64/root-tree/` + net fixtures | config (image skeleton) | n/a | `harvey/build/aarch64/root-tree/` (+ `lib/netip-*.rc`, `netaddr.rc`) | exact (re-key) |
| `harvey/build/x86_64/smoke-r9.expect` | test (composed gate) | streaming (serial capture) | `harvey/build/aarch64/smoke-r9.expect` | exact (re-key) |
| `harvey/build/x86_64/smoke-r9-net.expect` | test (composed net gate) | streaming | `harvey/build/aarch64/smoke-r9-net.expect` | exact (re-key) |
| `harvey/sys/src/9k/boot/mkfile.amd64` | config (build) | batch | `harvey/sys/src/9k/boot/mkfile.aarch64` | exact |
| `harvey/build/x86_64/tiers/{tier0-libs,tier1-cmds}.txt` | config (data allowlists) | n/a | `harvey/build/aarch64/tiers/*.txt` | exact (re-key) |
| `harvey/sys/src/mkfile` + `sys/src/cmd/mkfile` (+ `cmd/ip`, `cmd/auth`, `cmd/auth/factotum` mkfiles) | config (build filter) | batch | itself (aarch64 gate, `sys/src/mkfile:68-75`) | exact (add amd64 arm) |
| `harvey/flake.nix` + `harvey/nix/boot-r9-x64.sh` (+ smoke app scripts) | config (compose app) | batch | `harvey/flake.nix` bootR9Pkg (106-110) + `harvey/nix/boot-r9.sh` | exact (re-key) |
| `harvey/build/mm/{build-9k10t1.sh,boot-9k10t1-sweep.sh}` | test (donor venue) | batch | themselves (extend: r9-x64 execution leg) | exact (extend in place) |

Data-flow vocabulary is approximate here — this phase is kernel/build/gate work; the meaningful flows are trap/syscall (event/request), build/bake (batch, file I/O), and gate expects (streaming serial capture).

## Pattern Assignments

### A. ABI freeze (XEXEC-01)

#### `harvey/amd64/ABI.md` (contract doc)

**Analog:** `harvey/aarch64/ABI.md` (143 lines) — the publication template, structure-for-structure.

Copy the template's section order verbatim: title ("frozen") → provenance (which donor pins the facts; for amd64: 9k `sys/src/9k/k10/`, NOT 9front) → "Wire contract" → "User-side stub shape" → "Syscall table" → "Trap frame" → "What does not cross the wire" → "Kernel-side enforcement (appendix)".

**Wire-contract section pattern** (`aarch64/ABI.md:17-31`) — re-key per the amd64 facts:
```markdown
- **Syscall number:** register `R0`          →  register `BP` (6c RARG = D_BP)
- **Arguments:** 8-byte slots at user `SP+8` →  SAME formula, different reason:
  the stub spills arg0 (BP) to `a0+0(FP)`; SP+0 is the return-address slot
- **Trap instruction:** `SVC`                →  `SYSCALL` (MSR-routed, no hw frame push)
- **Result:** register `R0`                  →  register `AX`
```

**Stub-shape section** must quote the real generator, `harvey/sys/src/libc/9syscall/mkfile:81-95` (`case amd64`):
```make
	if(! ~ $i nsec)
		echo MOVQ RARG, 'a0+0(FP)'
	echo MOVQ '$'$n, RARG
	echo SYSCALL
	echo RET
```
plus the per-name quirks: `seek` → `TEXT _seek`, `exits` → `_exits`, and `nsec` emits NO arg0 spill (a generic SP+8 read for nsec consumes the return address — freeze this as a named ABI rule).

**Enforcement appendix** (`aarch64/ABI.md:83-143`) pins the two-layer check; the amd64 appendix adds the negative control (D-02b) and the byte-offset table from 14-RESEARCH.md §"pinned byte-offset table" (ax@0 … r15@112, u16 ds/es/fs/gs@120-126, type@128, error@136, ip@144, cs@152, flags@160, sp@168, ss@176, sizeof=184).

**Frozen source of truth** (quote verbatim, it is the whole file): `harvey/amd64/include/ureg.h:1-30` — bare `struct Ureg {` (NO typedef), 15 × `u64int` GP, then `u16int ds; u16int es; u16int fs; u16int gs;` (packed into one qword at 120), then type/error/ip/cs/flags/sp/ss.

**9k donor contracts to restate** (all verified tracked; cite by line):
- `harvey/sys/src/9k/k10/l64syscall.s` — `touser` lines 8-22, `syscallentry` 26-48, `syscallreturn` 50-68 (whole file is 68 lines; the ABI.md should restate its register contract: CX=user ip, R11=flags, BP(RARG)=user sp, result AX, segment stores at `(15*8+n)(SP)`).
- `harvey/sys/src/9k/k10/vsvm.c:174-185` — the MSR arming: `Star = user_csel<<48 | kernel_csel<<32`, `Lstar = syscallentry`, `Sfmask = If` (If-only — this overrides r9's dead `fmask()` which has IF|TF|DF; D-02a says follow 9k).
- `harvey/sys/src/9k/k10/syscall.c` — arg copy at `sp+BY2SE` (line 251), return `ureg->ax = ar0.p` (line 280), note delivery via `ureg->bp` (lines 182-192, `nf->arg0 = &nf->ureg` at 187).
- `harvey/sys/src/9k/k10/mem.h:57-61` — `UTZERO (0+2*MiB)`, `USTKTOP 0x00007ffffffff000ull`, `USTKSIZE (16*1024*1024)` (donor reference; r9 owns its own stack-top policy — discretion item).
- `harvey/sys/src/libc/9syscall/sys.h` — the frozen number table 0-54 (NOTE: research cited `9syscall/amd64/sys.h`; the actual tracked authoritative file is the shared `sys/src/libc/9syscall/sys.h` — ABI.md must cite that path).

#### `r9/x86_64/src/dat.rs` (Ureg rework to 184 bytes)

**Analog:** the CURRENT struct in this same file (lines 21-66) is the anti-pattern to replace; the target shape is `harvey/amd64/include/ureg.h` + the const-assert idiom of `r9/aarch64/src/trap.rs:144-161`.

**What changes** (lines 10-11, 50-66 today):
```rust
pub const UREG_TRAPNO_OFFSET: usize = 19 * core::mem::size_of::<u64>();   // → 16*8 = 128 (the `type` slot)
pub const UREG_CS_OFFSET: usize = 22 * core::mem::size_of::<u64>();       // → 19*8 = 152
```
Replace the four `u64` segment fields (current lines 50-53, "we waste a few bytes to keep the stack aligned") with the packed u16 quartet:
```rust
    ds: u16,
    es: u16,
    fs: u16,
    gs: u16,
    // (one qword at offset 120 — no padding before `type`: 120+8 == 128)
    pub type_: u64,      // was trapno@152 — ureg.h names it `type`, slot @128
    pub ecode: u64,      // @136
    pub pc: u64,         // @144 (ureg.h `ip`)
    cs: u64,             // @152
    ...
```
Then append the aarch64-style always-on asserts (`r9/aarch64/src/trap.rs:148-155` is the exact idiom to copy):
```rust
const _: () = assert!(core::mem::offset_of!(Ureg, bp) == 48);
const _: () = assert!(core::mem::offset_of!(Ureg, gs) == 126);
const _: () = assert!(core::mem::offset_of!(Ureg, type_) == 128);
const _: () = assert!(core::mem::offset_of!(Ureg, cs) == 152);
const _: () = assert!(core::mem::size_of::<Ureg>() == 184);
```
Note `dat.rs:225-226` already uses `static_assertions::const_assert_eq!` for Mach — either idiom is in-family; prefer the `const _: () = assert!` form to mirror aarch64's appendix text 1:1.

#### `r9/x86_64/src/trap.rs` (intrcommon re-key + syscallentry)

**Analog (rework):** its own `intrcommon` (lines 302-399). Today: `subq $((4 + 15) * 8), %rsp` (line 307) and four separate qword segment stores at `15*8..18*8` (lines 331-338). Re-key to the 9k shape: `SUBQ $(16*8)` (15 GP + 1 packed-dseg qword below the vector slot) and pack the four `movw` stores into one qword at `15*8+0..6` — exactly 9k's `syscallentry` stores (`harvey/sys/src/9k/k10/l64syscall.s:36-39`, `MOVW $SSEL(...), (15*8+0)(SP)` … `(15*8+6)(SP)`).

**CRITICAL (Pitfall 2 / CR-01 class):** the SWAPPED witness rides `SWAPPED_CS_OFFSET = 7` (line 19) applied at three sites — `intrspecial` line 251/258 (`+24` = the stub-pushed CS word), `intrregular` line 280/286 (same `+24`, with the "saved CS word is at +24 … +16 would be the saved RIP's high byte" comment at 245-250), and `intrcommon`'s restore `btr` at line 360/395 (`+ UREG_CS_OFFSET`). When `UREG_CS_OFFSET` moves 176→152, re-derive every one of these and keep the fixed comment shape in sync.

**Analog (new syscall path):** `harvey/sys/src/9k/k10/l64syscall.s:26-48` is the entry to port — SYSCALL pushes NOTHING, so the hardware-frame half is software-built (the RESEARCH Pattern 2 quote is verbatim-accurate; re-read the file at implementation time). Key r9 deltas: SWAPGS is unconditional (SYSCALL only fires from ring 3 — no CS-check witness here); the stack switch goes through GS:Mach (`MOVQ 16(RMACH), RUSER` → `m->proc` kstack or TSS rsp0); dispatch convention is BP=number, args at saved-user-SP+8, result AX (write into `frame.ax`).

#### `r9/x86_64/src/seg.rs` + `r9/x86_64/src/cpu.rs` (MSR arming)

**Analog:** their own dead code, already SYSRET-correct. `seg.rs:189-199` GdtData order is `null, ktext, kdata, udata, utext, unused, task` — already the SYSRET +16/+8 layout. Revive `utextsel()` (260-263) and `star()` (265-273):
```rust
    pub fn star() -> u64 {
        const RPL_USER: u64 = MachMode::User as u64;
        const UTEXTSEL: u64 = Gdt::utextsel() as u64;
        const KTEXTSEL: u64 = Gdt::ktextsel() as u64;
        ((UTEXTSEL - 16) | RPL_USER) << 48 | KTEXTSEL << 32
    }
```
`cpu.rs:25-30` `fmask()` currently returns IF|TF|DF — **9k donor says `Sfmask = If` only** (`vsvm.c:185`); follow the donor (D-02a) and note the deviation from the dead code in a comment. `cpu.rs:203` `wrmsr()` already exists. Arming sequence to mirror: `vsvm.c:174-185` (GSbase/KernelGSbase, Efer.Sce, Star, Lstar, Sfmask; user GS base = 0).

#### `r9/xtask/src/uregcheck.rs` (amd64 arm + parser fix + negative control)

**Analog:** its own aarch64 arm. Structure to mirror:
- `PINNED_FIELDS: [&str; 35]` (lines 28-32) → an amd64 table of 22 slots with the u16 quartet `ds, es, fs, gs` at slots 15-18 (packed one-qword rule documented next to the table).
- `extract_ureg_fields` (138-189) — two gaps to fix for amd64: it matches only `typedef struct Ureg` (amd64's header is a bare `struct Ureg {`, no typedef) and only `u64int` prefixes (amd64 declares `u16int ds;` etc.). Extend the prefix match to `u16int` with the four-fields-one-qword packing rule; keep the "deliberately narrow parser" doctrine (doc comment lines 15-18) — pin the quartet EXPLICITLY, never skip it (Pitfall 9).
- `harvey_dir()` (53-62) resolves ONE checkout; the amd64 arm needs `amd64/include/ureg.h` under the same `R9_HARVEY_DIR` (update the error string at 56-59 which currently names only `aarch64/include/ureg.h`).
- Unit-test shape to mirror (lines 254-284): `frozen_header_matches_the_pinned_table`, `drift_names_the_divergent_field`, `field_count_drift_is_reported` — amd64 equivalents plus the D-02b **negative control** (mutate a header copy → expect the named-field diff + a "NEGATIVE CONTROL FAILED" line). NOTE: no `NEGATIVE CONTROL` string exists in the xtask today — this is new, precedent is the drift tests above and the always-on negative panic scan in `xtask/src/main.rs:1097-1115` (`negative_scan_hit_for`).

#### `r9/xtask/src/main.rs` (boot-test gating)

**Analog:** its own arch gate, lines 1471-1485:
```rust
        // ... x86_64 skips it: the amd64 Ureg ABI is Phase 14's
        // XEXEC-01 (out of scope this phase).
        if self.arch == Arch::Aarch64 {
            uregcheck::check_for_boot_test()?;
        }
```
Change to run the check for both arches (aarch64 arm + the new amd64 arm). The `Arch::Riscv64` rejection at 1472-1474 shows the shape for arch-specific validation before qemu spawns.

### B. Exec substrate port (XEXEC-02) — r9 x86_64

#### `r9/x86_64/src/exec.rs` (CREATE)

**Analog:** `r9/aarch64/src/exec.rs` (499 lines) — port with constants re-keyed.

**Module-doc pattern** (lines 1-12): state eager-vs-demand mapping policy and the exec_into/exec_replace split in the same terms.

**Constants to re-key** (aarch64 lines 40-62 → amd64):
```rust
const R_MAGIC_AARCH64: u32 = 0x8C47;   → S_MAGIC amd64 = 0x8A97 (_MAGIC(0x8A, 26) per sys/include/a.out.h)
INITTEXT = 0x10000 + 40                → 0x200000 + 40  (6l obj.c INITTEXT = 0x200000+HEADR, HEADR=40)
TEXT_MAP_BASE = 0x10000                → 0x200000
data_va alignment: next_multiple_of(0x10000)  → next_multiple_of(0x200000)  // 6l INITRND = 2 MiB (Pitfall 4 / A1 — assert against a real 6.out's header bytes at bring-up)
```
USTACKTOP/USTKSIZE policy block (lines 28-36) — r9 owns it; 9k donor reference is `USTKTOP 0x00007ffffffff000`, `USTKSIZE 16 MiB` (`harvey/sys/src/9k/k10/mem.h:59-60`); aarch64 chose `0x0000_8000_0000_0000`. Discretion item — record the choice.

**Validation pattern to keep verbatim** (lines 89-119): `parse_aout` — checked `be32`/`be64` readers (79-85), magic check, empty-text check, entry-in-text check, data_va+data+bss < USTOP check. Every violation a loud `Err` before any mapping (V5 input-validation control).

**Frame seeding:** the forkret-shaped TrapFrame becomes the forkret-shaped 184-byte Ureg; entry comes from the a.out `entry` field (BE u64 at bytes 32..39) — on amd64 the seeded frame's ip/flags/sp/ss/dseg fields are set for the SYSRETQ/touser return (9k `touser`, `l64syscall.s:8-22`: CX=ip, R11=flags, BP=sp; r9's analog writes them into the frame instead).

#### `r9/x86_64/src/proc.rs` (CREATE)

**Analog:** `r9/aarch64/src/proc.rs` (2,737 lines) — the largest port. Key excerpts to carry:

**Proc struct** (lines 637-714): pid/name/state, `kstack_top` + `kstack_pas` (x86_64: kstack reached via TSS rsp0 instead of TTBR0-analog — the per-process page table field `root_table: *mut RootPageTable` becomes the per-process PML4/CR3), `fd_group: Arc<Lock<FdGroup>>`, `mapping_log`, `segments: Option<Segments>`, `text_source`, `frame: *mut TrapFrame` (→ `*mut Ureg`), `context: swtch::Context`, note machinery (`note_queue`, `notify_handler`, `note_ureg`, `last_note`), wait records.

**validate_user_range** (lines 1057-1072) — copy shape-verbatim (segment-membership proof):
```rust
    pub fn validate_user_range(&self, va: u64, len: u64, _writable: bool) -> Result<()> {
        let Some(end) = va.checked_add(len) else { return Err("user range overflows") };
        if len == 0 { return Ok(()); }
        let Some(segs) = &self.segments else { return Err("proc has no segments") };
        let (start, end) = (va as usize, end as usize);
        let contained = [&segs.text, &segs.data, &segs.bss, &segs.stack, &segs.tos]
            .iter().any(|r| r.0.start <= start && start < end && end <= r.0.end);
        if contained { Ok(()) } else { Err("bad user address") }
    }
```
Keep its single-process-era caveat comment (1050-1056). On x86_64 the "TTBR0 live" justification becomes "the calling proc's CR3 is loaded / user half visible in kernel context" — PROJECT.md's locked per-process live-user-memory model (NO Linux-style copy layer).

#### `r9/x86_64/src/syscall.rs` (CREATE)

**Analog:** `r9/aarch64/src/syscall.rs` (2,124 lines).

**Header invariants doc** (lines 1-20) — copy and re-key the wire facts: number in `frame.bp` (was `frame.x0`), args at `frame.sp + 8` (same formula!), result into `frame.ax`. Keep the locking invariant note: devcons write path takes the Console MCS lock from syscall context; **the tick println must be cfg-gated off under dev_exec** (Pitfall 8).

**Frozen number consts** (lines 42-72): the aarch64 set (BIND..NSEC, ~30 numbers of 0-54) carries over unchanged — same `sys.h` table (A5). rfork flag consts (79-95) verbatim.

**Dispatch pattern** (lines 164-193):
```rust
pub fn dispatch(frame: &mut TrapFrame) {
    let Some(mut proc) = proc::current() else { panic!(...) };
    let n = frame.x0;                       // amd64: frame.bp
    let result = match DISPATCH.get(n as usize) {
        Some(Some(handler)) => handler(frame, &mut proc),
        _ => { println!("syscall {n} unimplemented"); proc.set_errstr("syscall not implemented"); -1i64 as u64 }
    };
    if n == NOTED && result != -1i64 as u64 { return; }   // donor dosyscall special case
    frame.x0 = result;                       // amd64: frame.ax
}
```

**sargs pattern** (lines 215-227): args at `frame.sp + 8` via `UserSlice::validated_only` — identical formula, PLUS the amd64 special cases (Pitfall 5): `nsec` (53) has no spilled arg0 → dispatch it from the frame with NO stack read; `seek` is `_seek`, `exits` is `_exits` (TEXT renames only). Keep the 32-bit-arg masking discipline comment (199-214).

#### `r9/x86_64/src/usermem.rs` (CREATE)

**Analog:** `r9/aarch64/src/usermem.rs` (185 lines) — near-verbatim port. Only the SAFETY comments change (TTBR0-live → CR3/per-process-PML4-live). The fused constructor idiom (lines 42-46), `validated_only` variants (52-55), per-struct SAFETY comments on every raw materialization, and `UserString::copy_from`'s validate-each-byte loop (165-184) all carry as-is. This is the V5 escalation mitigation — every user-VA argument flows through here.

#### `r9/x86_64/src/main.rs` exec_bootstrap (extend)

**Analog:** `r9/aarch64/src/main.rs:434-560`. The whole hook-registration block (473-536: devproc post/proc hooks, devcons pid hook, devpipe hooks, devmnt contention hook, devether/devip hooks under cfg, devdup/devsrv hooks) carries — port/ is arch-neutral and expects these. `BOOT_ENTRY` cfg selection (443-446: `dev_bootchain` → `/boot/boot`, else `/init`) carries for q35exec (marker `/init`-style) vs q35shell (`/boot/boot`).

**x86_64 swap point:** Phase 13's arming window ends at the netgate trigger (RESEARCH: `../r9/x86_64/src/main.rs:130-200`); exec_bootstrap replaces/extends it. **IRQ-masked discipline** (Pattern 4): IF=0 through the whole bootstrap, `sti`/unmask only at the ring-3 handoff — aarch64 precedent lines 400-415 show the cfg-gated unmask placement.

### C. Bootfs delivery (XEXEC-02/03)

#### `r9/x86_64/src/platform/q35/mb2.rs` (+TAG_MODULE)

**Analog:** its own tag parses. Add `const TAG_MODULE: u32 = 3;` beside lines 29-32 and a match arm in `parse` (217-238) following `parse_acpi_root`'s shape (255-276): checked `le_u32` gets, size-vs-body minimum check, named `ParseError` variant. Body: `{ u32 mod_start; u32 mod_end; u8 cmdline[] }` — 32-bit PHYSICAL addresses, one tag per module (may repeat — collect into a Vec like `Facts`' ranges). Advance rule already correct (line 242: `off = (off + size + 7) & !7`). Bounds-check the module range against the parsed memory map before mapping (V5 firmware-input control). Add unit tests in the file's existing synthetic-builder `mod tests` (303+, `b.tag(...)` helper at 482+).

#### `r9/x86_64/lib/grub.cfg` (+module2)

**Analog:** itself (13 lines). Add inside the existing menuentry AFTER `multiboot2 /boot/kernel`:
```
menuentry "r9" {
    multiboot2 /boot/kernel
    module2 /boot/bootfs.paq
    boot
}
```
`module2` (not mb1 `module`) — mixing them silently drops modules (Pitfall 3). Keep the serial-console header comment discipline (lines 1-6: why serial-first matters for gates). Open Question 4 (committed unconditional line + loud "no module tag" dev behavior vs builder templating) is the planner's call.

#### `r9/x86_64/lib/config_q35exec.toml`, `config_q35shell.toml` (CREATE)

**Analogs:** `r9/aarch64/lib/config_virtexec.toml` (49 lines) and `config_virtshell.toml` (45 lines), section-for-section: `[build]` target `lib/x86_64-unknown-none-elf.json`, `[link]` script/load-address (Phase-13 q35 values — copy from existing `x86_64/lib/config_q35.toml`), `[config]` platform `q35` with the dev list — q35exec: `dev = ["exec"]` (staged leg 1; gate tick println off under dev_exec per Pitfall 8), q35shell: `dev = ["exec", "bootchain", "ether", "ip"]` (virtshell line 27 re-keyed; ether/ip stay per Phase-13 netgate). `[qemu]` machine q35 + accel kvm-native per Phase-13 D-theme — copy from `x86_64/lib/config_q35net.toml` (40 lines, the Phase-13 net profile). NO `initrd` key (aarch64's fixed-addr loader trick, virtexec.toml:34-45, does NOT carry — D-03's module tag replaces it; note this in the config comments).

### D. Compose (XEXEC-03, D-05)

#### `r9/flake.nix` (boot-iso parameterization)

**Analog:** its own `boot-iso` derivation (lines 105-205). The parameterization must keep in r9, un-forked: the xorriso-date wrapper (126-153), the grub moddir scrub (154-161), strip-all staging + store-hash zero-zap (162-184), and the FOD discipline (193-205: outputHash wired only after the double-build byte-identity proof). Shape decision (Open Question 1 — flake lib function `r9.lib.bootIso {kernel, modules}` vs overlay package) is the planner's; either way Harvey passes `kernel-x64` + its baked bootfs.paq, and grub.cfg's `module2` line references the staged module path. Pitfall 6: the composed ISO's bytes now depend on Harvey's bake — carry mkimage's determinism (KERNDATE, fixed staging order) and decide where the FOD boundary sits.

#### `harvey/flake.nix` (boot-r9 x86_64 app) + `harvey/nix/boot-r9-x64.sh` (CREATE)

**Analogs:** `harvey/flake.nix` `bootR9Pkg` (lines 106-110) / `smokeR9Pkg` (116-120) / `netSmokeR9Pkg` (133-137) / `tier1R9Pkg` (143-147) — all four are the SAME `mkScriptApp` shape (26-39) exporting `R9_KERNEL_PKG` from `r9.packages.${system}.kernel-virt`; the x64 apps swap the input to the x64 kernel/ISO package and point at `build/x86_64/` scripts. The `optionalAttrs (builtins.hasAttr system r9.packages)` gate (160-165) carries. `harvey/nix/boot-r9.sh` (210 lines) is the app script template: its step structure (0 kernel-package contract → KERNDATE provenance 127-142 → 1 userland build 144-151 → 2 bake 153-158 → 3 launch 160-209) re-keys with step 3 becoming "call r9's ISO builder + `qemu-system-x86_64 -M q35 -accel kvm -cdrom ISO -nographic`" — NO DTB patch, NO `-device loader` (D-03 replaces the aarch64 delivery, boot-r9.sh:171-199 is what NOT to carry). The smoke apps' caller discipline (rm -f capture, R9_RUN_TAG survivor sweep) is documented in smoke-r9.expect:26-31,79 — keep.

### E. Harvey build drivers + boot(8) (XEXEC-03/04)

#### `harvey/build/x86_64/{stage-build.sh, subset-build.sh}` (CREATE)

**Analogs:** `harvey/build/aarch64/stage-build.sh` (151 lines) and `subset-build.sh` (469 lines).

- `stage-build.sh` re-key: `STAGE_SRCS="init smokefork smokenote stacksmoke smokesleep"` (line 41) → the amd64 marker set (D-09: smokesleep is leg 1's marker); `objtype=aarch64 7c/7l` (line 110) → `objtype=amd64 6c/6l`; the magic assert (147-148, `00008c47`) → amd64 S_MAGIC `00008a97` (BE). Note the aarch64 script drives a lima guest — this host is linux, and amd64 builds natively under `sudo unshare` (subset-build.sh's `NATIVE_LINUX` split, lines 52-82, is the driver shape to prefer for both x64 scripts).
- `subset-build.sh` re-key: keep the tier-file loop discipline, the D-07 fresh-archive rule (`rm -f` the .a before `mk install`), T0-ART/T1-ART sha256 markers + `TIER-BUILD-OK` (never trust mk exit status), the nm-shim comment block (115-120), and the entry-form dispatch per tier1-cmds (per-target install / dir install / nested / dir:target / 9k/boot + stage driver-only blocks). Paths re-key `build/aarch64/` → `build/x86_64/`, `aarch64/bin` → `amd64/bin`, objtype, toolchain 7c→6c.

#### `harvey/build/x86_64/mkimage.sh` (CREATE)

**Analog:** `harvey/build/aarch64/mkimage.sh` (291 lines) — the closest single template in the repo. Keep the whole skeleton: header image-contract comment (2-31, re-key paths `/aarch64/init` → `/amd64/init` etc.), `$PLAN9/bin` LAST PATH fixup (55-59), mkpaqfs-host bootstrap (61-66), loud named-input asserts (68-119), staging via tar from committed root-tree (127-132), empty mount-point dirs incl. `/net` (135-142), binary staging (144-161), pre-bake manifests incl. the init-identity grep guard (227-239) and the **`/boot/factotum` absence assert** (267, auth-stub-by-absence — Phase-14 keeps it, auth is Phase 18), rcmain verbatim `cmp` (271), and the byte-format contract check — header magic `529ab12b`, trailer `6b46e688` via `od -j` (281-289, D-04 carried verbatim). Determinism (Pitfall 6): keep ordering stable; KERNDATE flows in from the caller (boot-r9.sh:127-142).

#### `harvey/sys/src/9k/boot/mkfile.amd64` (CREATE)

**Analog:** `harvey/sys/src/9k/boot/mkfile.aarch64` (62 lines) — near-verbatim, objtype-keyed. Same `BOOTLIB`/`BOOTFILES` archive pattern (30-51), same `boot: bootr9.$O printstub.$O $BOOTLIB` link with printstub named explicitly (58-59). The aarch64 comment block (3-28) documents exactly why the file exists (no kernel config for r9 → hand-written `bootr9.c` supplies bootconf) — amd64 has kernel configs (k10cpu) but the r9 boot has none, so the same hand-written-bootconf rationale applies; bootr9.c is already arch-neutral enough for the aarch64 link (verify its embed method table compiles under 6c — A4; trim BOOTFILES to the real 6l link closure the same way the aarch64 file documents its trims).

#### `harvey/build/x86_64/root-tree/` + tiers + net fixtures (CREATE)

**Analogs:** `harvey/build/aarch64/root-tree/` (contains `lib/` with `netip-*.rc` + `net-sweep.rc`, `rc/bin/termrc`, `usr/glenda/lib/profile`), `build/aarch64/netaddr.rc` (42 lines — the D-11 single addressing seam: lo_/slirp_/blastport keys; NO hardcoded 10.0.2.x anywhere else), `build/aarch64/tiers/tier0-libs.txt` (60 lines) + `tier1-cmds.txt` (134 lines) — the append-only plain-data discipline with the `#`-comment/whitespace keep-rule documented in their headers, re-keyed to the amd64 tier-0/tier-1 scope (this phase: tier-0 libs + tier-1 core commands as execution proof; the k10t1 43-file embed set is the ready-made amd64 list).

#### `harvey/sys/src/mkfile` + `sys/src/cmd/mkfile` (+ container mkfiles) — MODIFY

**Analog:** the existing filter, `harvey/sys/src/mkfile:50-75`:
```make
TIER0=$HARVEY/build/aarch64/tiers/tier0-libs.txt
TIER1=$HARVEY/build/aarch64/tiers/tier1-cmds.txt
T0KEEP=`{awk '/^[a-z0-9]/{ sub(/[\/: 	].*$/,""); print }' $TIER0}
...
LIBS=`{if(~ $objtype aarch64 && ! ~ $"COVERAGE_SWEEP 1) for(i in $ALLLIBS) if(~ $i $T0KEEP) echo $i; if not echo $ALLLIBS}
```
Add the amd64 arm: `if(~ $objtype amd64 ...)` pointing at `$HARVEY/build/x86_64/tiers/*.txt`. The same one-line change applies at every filter site (per tier1-cmds.txt header: `sys/src/mkfile`, `sys/src/cmd/mkfile` lines 22-33, and the container mkfiles `cmd/ip`, `cmd/auth`, `cmd/auth/factotum`). Keep the `$HARVEY`-absolute addressing (live under unshare -m) and the COVERAGE_SWEEP escape.

### F. Gates (XEXEC-03/05, D-06/D-07)

#### `harvey/build/x86_64/smoke-r9.expect` (CREATE)

**Analog:** `harvey/build/aarch64/smoke-r9.expect` (273 lines). Re-key the spawn argv (93-101): drop `-dtb`/`-device loader`; use `-cdrom <composed ISO>` (+ `-M q35 -accel kvm|tcg -m 1024M`, virtio-net NIC stays). Keep EVERYTHING else stage-for-stage — the discipline is load-bearing (measured): env contract with exit 2 (68-79), fresh-capture `exec rm -f` (84), `stage` proc naming each failure (119-126), `expect_before` panic fast-fail (130-136), the stage vocabulary (139-273: banner → paqfs → stacksmoke → `% ` → two-occurrence echo round-trip → ls/ps/cat sweep with `strip_echoed_line` → DEL safety → init restart → whole-capture pid-warning-absent + nic-bind post-passes). "Green twice" (XEXEC-03) = run the gate twice, both captures retained (driver-level, e.g. the flake app or CI loop).

#### `harvey/build/x86_64/smoke-r9-net.expect` + `root-tree/lib/netip-*.rc` (CREATE)

**Analogs:** `harvey/build/aarch64/smoke-r9-net.expect` (403 lines) + `root-tree/lib/netip-loopback.rc` (200 lines), `netip-static.rc`, `netip-dhcp.rc`, `netaddr.rc`. The fixture pattern (netip-loopback.rc): sourced-not-typed, `test -e` before every redirect-open (uncatchable rc Xerror), per-step `netip: FAIL <stage>` + exit, ok markers + `NETIP-DONE`, no sleeps/timing assumptions, addresses ONLY from `/lib/netaddr.rc`. The composed net smoke's header (1-100) documents the load-bearing STAGE ORDER (ether legs first — multi-waiter RX wake is a known r9 defect; ether unbind before loopback; singleton mediums) and the **venue-honesty label** (`netip: NOT-L2-TRUTH slirp venue` asserted as a stage — Pitfall 10). Carry both verbatim into the x64 gate.

### G. Tiers + donor witness (XEXEC-04)

#### `harvey/build/mm/{build-9k10t1.sh, boot-9k10t1-sweep.sh}` — EXTEND

**Analogs:** themselves. `build-9k10t1.sh` (203 lines): the precondition block naming remediation (47-61: amd64/bin + amd64/lib populated, 6c/6l/6a in hostbin, sudo, PLAN9), the staging discipline with sentinel digest pins (70-95), the nuke-sandwich namespace build via `RUN_BUILD_IN_PRIVATE_NAMESPACE` (101-174), and the embedded-count + git-clean asserts (176-199) — extend with the r9-x64 execution leg (same 6.outs run on r9: the ABI witness). `boot-9k10t1-sweep.sh` (375 lines): the marker contract — expected marker set COMPUTED from the tier data (lines 34-52), boundary-anchored `T1-OK m([^A-Za-z0-9.-]|$)` / `T1-SKIP m`, the converse assertion (every emitted T1-SKIP was expected), marker-state digest on failure. The r9 leg reuses `build/aarch64/tier1-r9.expect`'s composed-sweep shape (the tier1R9Pkg app at harvey/flake.nix:143-147).

## Shared Patterns

### 1. Two-layer ABI enforcement + negative control (D-02b) — XEXEC-01
**Source:** `r9/aarch64/src/trap.rs:148-155` (const asserts), `r9/xtask/src/uregcheck.rs:28-189` (pinned table + narrow parser + drift diff), `r9/xtask/src/main.rs:1471-1485` (boot-test precondition).
**Apply to:** `r9/x86_64/src/dat.rs`, `r9/xtask/src/uregcheck.rs` (amd64 arm), `harvey/amd64/ABI.md` (the published table they both pin). Any drift = build failure with a named-field diff, never a runtime mystery.

### 2. Measured-set dispatch + validate-before-touch — V5 escalation control
**Source:** `r9/aarch64/src/syscall.rs:8-15,164-227` (loud-unimplemented; args via `UserSlice::validated_only` at SP+8), `r9/aarch64/src/usermem.rs` (fused constructors), `r9/aarch64/src/proc.rs:1057-1072` (`validate_user_range`).
**Apply to:** `r9/x86_64/src/{syscall,usermem,proc,exec}.rs`. amd64 deltas: number in BP, result in AX, `nsec` special case, stack-range validation precedes the arg copy (9k `syscall.c:248-251` precedent).

### 3. IRQ-masked exec bootstrap + cfg-gated tick print — Pattern 4/Pitfall 8
**Source:** `r9/aarch64/src/main.rs:400-415,457-560` + the heisenbug record `.planning/milestones/v1.0-phases/04-r9-userspace-exec-harvey-r9-abi/deferred-items.md`.
**Apply to:** `r9/x86_64/src/main.rs` (IF=0 through bootstrap, unmask only at ring-3 handoff), `config_q35exec.toml`/`config_q35shell.toml` (tick println off under dev_exec).

### 4. Expect-marker gate discipline — XEXEC-03/05
**Source:** `harvey/build/aarch64/smoke-r9.expect` (two-occurrence echo, strip-echoed-line, presence-not-timing, fresh capture, stage-named failures, whole-capture post-passes) + `smoke-r9-net.expect` (stage order, venue labels).
**Apply to:** every `harvey/build/x86_64/*.expect` and the flake app scripts that drive them.

### 5. Reproducible-ISO FOD discipline — D-05/Pitfall 6
**Source:** `r9/flake.nix:105-205` (xorriso-date wrapper, strip-all + store-hash zap, hash wired only after byte-identity proof) + `harvey/build/aarch64/mkimage.sh` (deterministic staging) + `harvey/nix/boot-r9.sh:127-142` (KERNDATE).
**Apply to:** the parameterized builder (r9) and the composed ISO (Harvey-side boundary decision).

### 6. Append-only tier data + objtype-gated mk filters — XEXEC-04
**Source:** `harvey/build/aarch64/tiers/*.txt` headers + `harvey/sys/src/mkfile:50-75` + `sys/src/cmd/mkfile:22-33`.
**Apply to:** `harvey/build/x86_64/tiers/*.txt` and the amd64 filter arms; coverage reports assert against the files.

### 7. Loud named-input asserts in build drivers
**Source:** every `harvey/build/aarch64/*.sh` — `err()` with the remediation named (e.g. mkimage.sh:69-71 "run sh build/aarch64/subset-build.sh first"; build-9k10t1.sh:54 "run 'nix run .#build' first"), host-grep resolution D-02 (mkimage.sh:47, stage-build.sh:48, build-9k10t1.sh:66), magic/marker asserts at every hand-off.
**Apply to:** all `harvey/build/x86_64/*.sh` and the nix app scripts.

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| mb2 module-tag CONSUMPTION (mapping the module phys range into devroot) | service | transform | The walker-extend has an in-file analog, but no prior instance of GRUB-delivered module consumption exists (aarch64 used qemu `-device loader` + DTB /chosen patch — deliberately not carried, D-03). Consumption-side analog: `r9/port/src/devroot.rs` `mount_initrd` (arch-neutral, reused as-is) + aarch64 main.rs's initrd discovery/reserve/map sequence. |
| `ureg-check` NEGATIVE CONTROL harness leg | test | batch | No `NEGATIVE CONTROL` machinery exists in the xtask today; precedent = the drift unit tests (`uregcheck.rs:269-284`) + the always-on negative panic scan (`xtask/src/main.rs:1097-1115`). Build fresh from those shapes (D-02b). |
| `boot-r9` x86_64 flake APP composition calling r9's exported builder | config | batch | The aarch64 app consumes a package (`kernel-virt` store path); no prior flake-to-flake builder invocation exists. Nearest shapes: `bootR9Pkg` (harvey/flake.nix:106-110) for the app wiring + r9's `boot-iso` derivation for what gets called. Open Question 1 governs the API shape. |

## Metadata

**Analog search scope:** `harvey/build/aarch64/`, `harvey/build/mm/`, `harvey/build/x86_64/` (README only), `harvey/nix/`, `harvey/sys/src/9k/{boot,k10}`, `harvey/sys/src/libc/9syscall/`, `harvey/sys/src/cmd/6l/`, `harvey/sys/src/mkfile`, `harvey/sys/src/cmd/mkfile`, `harvey/{aarch64,amd64}/`, `r9/aarch64/{src,lib}/`, `r9/x86_64/{src,lib}/`, `r9/xtask/src/`, `r9/port/src/` (grep), `r9/flake.nix`.
**Files scanned:** ~40 read (full or targeted); line numbers in excerpts verified against HEAD (`r9` @ `029e32f`, `harvey` working tree).
**Corrections to upstream docs found while mapping:** (1) the ABI.md template lives at `harvey/aarch64/ABI.md`, NOT `../r9/aarch64/ABI.md` as 14-RESEARCH.md's source list says; (2) the authoritative syscall table is the shared `harvey/sys/src/libc/9syscall/sys.h`, NOT `9syscall/amd64/sys.h` (does not exist); (3) `uregcheck.rs`'s `harvey_dir()` error text names only `aarch64/include/ureg.h` — the amd64 arm must update it.
**Pattern extraction date:** 2026-11-08
