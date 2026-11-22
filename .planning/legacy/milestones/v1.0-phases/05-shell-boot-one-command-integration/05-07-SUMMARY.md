---
phase: 05-shell-boot-one-command-integration
plan: 07
subsystem: kernel
tags: [r9, aarch64, devmnt, 9P, 9P2000, FVERSION, MOUNT, mount-table, union-walk, mntchk, exec, allocator, seek-ABI, paqfs, qemu, tdd]

# Dependency graph
requires:
  - phase: 05-shell-boot-one-command-integration (05-04)
    provides: the namespace core (namec registry + union walk + pgrp mount table) the mount entry inserts into; the checked open/stat dispatch shape; the build-order lesson (the gated config builds LAST)
  - phase: 05-shell-boot-one-command-integration (05-05)
    provides: devenv's seeded root group (nobootprompt=embed drives the method pick); the hardened /rootfs.img raw node (paqfs's byte source)
  - phase: 05-shell-boot-one-command-integration (05-06)
    provides: devpipe (the Arc-shared cross-connected queue the 9P exchange rides); devsrv's posted-chan registry (settime/newns re-mount via #s/boot); devdup (rcmain's . -i '#d/0'); sys_create (init's setenv); the connectembed gate + the FVERSION wall this plan demolishes; the FdGroup/rfork fd semantics
provides:
  - port::devmnt — the kernel 9P client (D-03's mount half; the virtio-9p prerequisite): the Mnt session over an MntIo transport (Tversion negotiation, Tattach/Twalk/Topen/Tread/Tstat/Tclunk RPCs), the T-05-13 mntchk discipline (msize bounds BEFORE the body read + tag match + exact-consumption parse), the AtomicBool one-outstanding-RPC era answer, the pipe-transport 4072-byte per-RPC clamp
  - sys_fversion (FVERSION 40 — the session negotiation parked on the fd's chan, the donor's c->mux) + sys_mount (MOUNT 46 — Tattach afid=NOFID then pgrp::bind with the frozen flag semantics; FAUTH stays loud-None, the D-02 fall-through)
  - devmnt chans in the mount table: namec's DEV_MNT legs (walk/open/stat dispatch + close-dispatch Tclunk); the T-05-14 host matrix (mount-crossing walks: MAFTER tie-break + union fall-through + MREPL shadow)
  - exec through the union-mounted tree: sys_exec resolves via namec (the Phase-4 devroot-only scaffold retired); ByteBacking::Mnt stages the whole mounted binary eagerly (the trap-context pager never RPCs)
  - devcons #c/time (accept-and-record settime's OWRITE) + the donor-faithful #c/user (no newline)
  - THE HINGE: boot(8) COMPLETES on r9 — fversion negotiates, nsinit mounts /root + binds onto /, settime re-mounts via #s/boot onto /tmp + writes #c/time, execinit execs /aarch64/init, init runs newns over the shipped /lib/namespace (the re-mount pair), rc execs through the /bin union via devmnt, termrc runs stacksmoke (marker prints), rc -i prints the literal `% ` prompt — the cons read wall (05-08) is the only remaining stop; zero panic lines
  - evidence: .planning/phases/05-shell-boot-one-command-integration/05-07-boot-r9.log (the hinge-gate run)
affects: [05-08 (the cons read path is the wall the gate stopped at; sys_sleep/sys_alarm land there — the restart loop's syscall 17), 05-09 (devproc ctl + #c/pid — init's pri write names pid 0 until then; the smoke sweep rides devmnt reads), 05-10/05-11 (the composition apps consume the hinge), v1.x virtio-9p (devmnt is its prerequisite)]

# Actuals (#2632) — pairs with the plan's estimate (120000 tokens, confidence: low).
actuals:
  tokens: 40265    # MEASURED chars/4: r9 diff 114323 + harvey diffs 2762 + deferred-items delta 4041 + this SUMMARY 39936 = 161062/4; the estimate (120k) overshot ~3x
  tasks: 3
  commits: 8       # MEASURED: r9 git rev-list --count 5af3fe1..454f0f2 = 6; harvey 7346cd420..71fcf94ae = 2 (pre-metadata; the docs commit rides on top, per the 05-04..06 convention)
plan_head_before: {r9: "5af3fe17deb5d7e908e2a86d81cbc368f2f9ae16", harvey: "7346cd420b3b79fafc96295e2fd97222bc14766f"}
plan_head_after: {r9: "454f0f2 (docs; code at 8817144)", harvey: "71fcf94ae (pre-metadata; the docs commit rides on top)"}

# Tech tracking
tech-stack:
  added: []        # zero new dependencies anywhere (T-05-SC held: no package installs)
  patterns:
    - "the 9P session on the chan: dat.rs's mnt slot (the donor's c->mux) — fversion parks the Arc<Mnt> on the pipe chan; dup clones it (devsrv's posted clone carries it to the second mount); DEV_MNT chans carry it plus their server-side fid"
    - "one outstanding RPC per session via an AtomicBool ownership flag — an MCS lock held across the blocking recv would spin-deadlock a contender in IRQ-masked syscall context; contention errors loudly, never wedges (the donor's tag-multiplexer arrives with virtio-9p)"
    - "the transport clamp: a 9P message must fit the pipe's atomic-write bound (4096), so per-RPC read counts clamp to 4072 even at msize 8192 (the donor's iounit clamp with the era's transport binding)"
    - "exec byte-source abstraction (proc.rs ByteBacking): DEV_ROOT demand-pages the paqfs image (04-04); DEV_MNT stages the whole file eagerly at exec (syscall context blocks; the trap-context pager never RPCs) — the a.out layout (data_file_base) is source-independent"
    - "the hinge-gate iterate-discipline exercised three times: run the gate, let the loud line name the gap, implement exactly that, re-run — it caught an image-pollution wedge, a wrong syscall ABI, and two latent allocator bugs"

key-files:
  created:
    - ../r9/port/src/devmnt.rs
    - ../r9/port/tests/devmnt_test.rs
    - ../r9/port/tests/05-07-red-evidence.json
    - .planning/phases/05-shell-boot-one-command-integration/05-07-boot-r9.log
  modified:
    - ../r9/port/src/dat.rs
    - ../r9/port/src/lib.rs
    - ../r9/port/src/namec.rs
    - ../r9/port/src/devcons.rs
    - ../r9/port/src/allocator.rs
    - ../r9/aarch64/src/syscall.rs
    - ../r9/aarch64/src/proc.rs
    - ../r9/aarch64/src/exec.rs
    - ../r9/aarch64/src/fault.rs
    - ../r9/HowItWorks.md
    - build/aarch64/root-tree/lib/namespace
    - build/aarch64/mkimage.sh
    - .planning/phases/05-shell-boot-one-command-integration/deferred-items.md
    - .planning/WINDOWS.md

key-decisions:
  - "the session rides the chan (the donor's c->mux): fversion parks the Arc<Mnt> on the pipe fd's chan; Chan::dup clones it — devsrv's posted clone carries the session to settime's and newns's second mounts of #s/boot (fresh attach fid each; the donor's mntattach per mount)"
  - "one outstanding RPC per session, enforced by an AtomicBool (never an MCS lock across the blocking recv — that spin-deadlocks an IRQ-masked contender); contention is a loud 'devmnt: concurrent rpc this era' errstr. The boot chain's client side is strictly sequential by construction (init parks in await while rc runs; fork() copies fd tables); the pgrp-lock-held-across-blocking-walk hazard is documented + deferred (the namec chain-snapshot fix lands with the first measured concurrent caller)"
  - "per-RPC read counts clamp to 4072 (devpipe's 4096 atomic-write bound minus IOHDRSZ) even though msize negotiates to 8192 — the pipe write is whole-message-or-fail, so a bigger Rread would kill the server on 'pipe full'"
  - "exec through the mount stages the WHOLE binary eagerly (ByteBacking::Mnt, Arc-shared, 16 MiB cap) — the trap-context pager must never RPC; syscall context is where blocking belongs"
  - "mkimage.sh asserts out/init is the REAL init at bake time (the rodata marker check) — the 05-02 'last writer wins' out/ ownership means r9's boot-test bake (stage-build.sh) re-pollutes; the bake must fail loudly rather than boot the staged chain (whose concurrent exec children wedge on the deferred pgrp hazard)"
  - "the aarch64 seek ABI is the generated _seek stub's FOUR args (rv ptr, fd, offset, base) with the result written through *rv — the donor sysfile.c sysseek's own comment is the authority; 05-06's slot-0-as-fd reading was wrong and its gate never exercised a checked-return seek"

patterns-established:
  - "the fake-server host-test rig for a kernel RPC client: a scripted MntIo responder speaking real wire bytes (built independently of the codec under test) with malformation knobs (inflated size field, wrong tag, msize increase) — the paqfs_test.rs fixture precedent extended to protocol-violation testing"
  - "the RED-evidence TAP transcription: cargo's libtest has no --format=tap on this toolchain; the persisted record's output field transcribes the real run into TAP shape (the 05-06 record's shape) for the tdd-red-evidence checker"
  - "the mount-entry variant in dat.rs is documented: MountEntry.to for a 9P tree is a DEV_MNT chan (session Arc + attach fid); the union walk re-roots its path at the mount point per the 05-04 convention"

requirements-completed: [INT-01]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "devmnt: the kernel 9P client — the message codec round-trips Tversion/Tattach/Twalk/Topen/Tread/Tstat/Tclunk over an in-memory pipe pair with a scripted fake server; an oversized-vs-msize reply is rejected as a protocol error before any field is trusted; a tag mismatch is rejected; a multi-component Twalk chains; Tclunk releases the fid server-side"
    requirement: INT-01
    verification:
      - kind: unit
        ref: "cargo test -p port — port/tests/devmnt_test.rs 8/8 (devmnt_version_attach_walk_open_read_stat_clunk_round_trip, devmnt_version_accepts_servers_lower_answer, devmnt_rejects_server_msize_increase, devmnt_rejects_oversized_reply, devmnt_rejects_tag_mismatch, devmnt_walk_not_found_is_a_loud_error, devmnt_walk_of_a_non_directory_errors, devmnt_mount_crossing_walk_matrix); the RED evidence verified RED_EVIDENCE_OK (port/tests/05-07-red-evidence.json — target test failed at the negotiation .expect, 8/8 failing)"
        status: pass
    human_judgment: false
  - id: D2
    description: "sys_fversion + sys_mount: validation-first handlers (the donor's NUL check + MMASK/order Ebadarg + afd=-1-only + the fd-is-a-9P-connection check; auto-version fallback per the donor's mntattach) with the frozen mount-flag semantics (pgrp::bind's MREPL/MBEFORE/MAFTER/MCREATE); FAUTH stays loud-None"
    requirement: INT-01
    verification:
      - kind: e2e
        ref: "05-07-boot-r9.log: 'version...' with ZERO 'syscall 40 unimplemented' (fversion negotiates over the pipe), then settime's SECOND mount via #s/boot succeeds silently (no 'settime mount' warning), stat(/tmp) via Tstat succeeds (no fatal), and newns's 'mount -a #s/boot /root' runs (the namespace's re-mount pair) — three live mounts across two attach fids on one session; 'syscall 10 unimplemented' twice = FAUTH's designed loud-None"
        status: pass
      - kind: integration
        ref: "boot-test virtexec/virtmulti/virtnotes/virt/virtpanic all PASS + ureg-check 35/35 on the final tree"
        status: pass
    human_judgment: false
  - id: D3
    description: "devcons #c/time accepts settime's OWRITE (accept-and-record in a documented static; OREAD returns the recorded value or a fixed plausible epoch) + the wiring-walk fixes: exec resolves through the union-mounted / into devmnt (ByteBacking::Mnt), the shipped /lib/namespace gains the 05-03 finding's re-mount pair, #c/user serves the bare 'glenda' (the donor's readstr shape)"
    requirement: INT-01
    verification:
      - kind: e2e
        ref: "05-07-boot-r9.log: 'time...' with no 'can't set #c/time' warning (the write succeeds); 'init: starting /bin/rc' proves execinit's exec; the stacksmoke marker proves rc + termrc ran (rc exec'd through the post-newns /bin union — devmnt); init's readenv/setenv/cpenv warnings are absent for cputype/objtype/timezone (the env path wired)"
        status: pass
      - kind: integration
        ref: "the staged fan + clippy + builds all green through the exec rewiring (virtexec exercises sys_exec via the staged init's children)"
        status: pass
    human_judgment: false
  - id: D4
    description: "THE HINGE GATE: boot COMPLETES — fversion + mount /root + bind onto / + settime's /tmp mount + #c/time write + execinit → /aarch64/init → newns over the shipped namespace → rc through the /bin union → termrc runs stacksmoke → rc -i prints the literal `% ` prompt — then the cons read hits the loud wall (rc exits \"false\" on the read -1; init restarts); zero panic lines"
    requirement: INT-01
    verification:
      - kind: e2e
        ref: "05-07-boot-r9.log (the full plan verify chain): paqfs... PRESENT; 'stacksmoke: depth 64 ok, checksum 2088960' PRESENT; '% ' PRESENT (the wall line: `% process 4 exited: \"false\"` — the prompt print immediately followed by rc's exit on the cons read's loud -1); grep -ciE 'panic|Unhandled interrupt' = 0; grep -c \"can't exec\" = 0; grep -c 'syscall 40 unimplemented' = 0"
        status: pass
    human_judgment: false
  - id: D5
    description: "the gate-caught fixes (the iterate-discipline's yield): the REAL aarch64 _seek ABI (4 args, *rv write-back — paqfs's blockRead seeks work); the allocator tail path (try_alloc arg order + free_misc's fresh-header arm — the ~250 KB exec staging buffer allocs/frees cleanly); the mkimage.sh init-identity assertion (staged-supply-chain pollution fails the bake loudly)"
    requirement: INT-01
    verification:
      - kind: e2e
        ref: "05-07-boot-r9.log: zero 'seek: bad fd' lines (paqfs's serving seeks resolve — the walks that produced Rerror pre-fix now answer); zero 'align is not a power-of-two' / 'allocated header block' panics (the exec staging Vec's realloc chain cycles per rc restart)"
        status: pass
      - kind: unit
        ref: "cargo test -p port — allocator::tests::tail_alloc_big_nonpow2_free_and_realloc (the 200 KB non-pow2 alloc/free/realloc cycle) + the full suite 55/55"
        status: pass
      - kind: other
        ref: "sh build/aarch64/mkimage.sh fails loudly with 'out/init is not the real init' when out/ holds the staged init (exercised during the gate debug); passes after subset-build.sh restores it"
        status: pass
    human_judgment: false

# Metrics
duration: 106 min (incl. three gate runs + the allocator/seek debug + two lima builds)
completed: 2026-11-03
status: complete
---

# Phase 5 Plan 07: devmnt + FVERSION/MOUNT + the boot-to-prompt hinge Summary

**r9 is a Plan 9 environment now. devmnt — the kernel 9P client with the ported mntchk discipline (every R-message bounds-checked against the negotiated msize and tag-matched before any field is trusted) — plus sys_fversion/sys_mount demolished the FVERSION wall, and the hinge gate boots the REAL Harvey chain end-to-end: boot(8) negotiates the session over the connectembed pipe, nsinit mounts the userspace paqfs server's tree at /root and binds it onto /, settime re-mounts #s/boot onto /tmp and writes #c/time, execinit execs /aarch64/init, init's newns parses the shipped /lib/namespace (with the 05-03 finding's re-mount pair), rc execs through the /bin union RIDING devmnt RPCs, termrc runs stacksmoke (the D-15 marker prints), and rc -i prints the literal `% ` prompt — the cons read wall (05-08's scope) is the only remaining stop, and zero panic lines appear. The gate's iterate-discipline caught and fixed three real latent bugs along the way: the aarch64 _seek ABI misread (05-06's sys_seek), two inverted/latent QuickFit tail-path bugs (the first big non-pow2 kernel-heap allocation panicked), and the out/init staged-supply-chain pollution that wedged the first gate run (now a bake-time assertion in mkimage.sh).**

## Performance

- **Duration:** 106 min
- **Started:** 2026-11-03T08:34:08Z
- **Completed:** 2026-11-03T10:20:48Z
- **Tasks:** 3
- **Files modified:** 17 (9 r9 source modified + 2 r9 created + 1 r9 test + 1 r9 evidence... counting: r9 created 3 (devmnt.rs, devmnt_test.rs, the RED evidence), r9 modified 9, harvey 5 (.planning artifacts + the namespace file + mkimage.sh))

## Accomplishments

- **Task 1 (TDD): devmnt is real and host-proven.** RED: 8 host tests over a scripted fake 9P server (real wire bytes, built independently of the codec — the paqfs_test.rs precedent extended with malformation knobs: inflated size field, wrong tag, msize increase), evidence verified `RED_EVIDENCE_OK` (target `devmnt_version_attach_walk_open_read_stat_clunk_round_trip` failed at the negotiation `.expect`; 8/8 failing) at `port/tests/05-07-red-evidence.json` (r9 `91bfbbf`). GREEN (r9 `d4ae0e4`): the Mnt session over an MntIo transport — the Tversion negotiation (offer 8192, accept the server's lower answer, the donor's nonsense/increase bounds), Tattach (afid=NOFID, D-02), Twalk/Topen/Tread/Tstat/Tclunk; the fcall.h/convS2M/convM2S codec ported (LE writers + a checked reply cursor with exact-consumption); T-05-13's mntchk (size bounded BEFORE the body read, tag matched, Rerror → loud errstr with the ename on the serial log); the concurrency era answer (ONE outstanding RPC per session via an AtomicBool — an MCS lock across the blocking recv would spin-deadlock an IRQ-masked contender; contention errors loudly, never wedges); the transport clamp (4072 per RPC — devpipe's 4096 atomic-write ceiling dominates the negotiated 8192). dat.rs gains the `mnt`/`mnt_fid` Chan slots (the donor's `c->mux`; dup clones the Arc — devsrv's posted clone carries the session to the second mount), namec gains the DEV_MNT legs (walk/open/stat dispatch, close-dispatch Tclunk, loud read-only create), syscall.rs gains sys_fversion/sys_mount (sargs grew a fifth slot for mount's five args) and the pread/pwrite DEV_MNT legs. No REFACTOR commit — the GREEN code was fresh-written and clippy-clean.
- **Task 2: the last small surface + the wiring walk** (r9 `79d7185`, harvey `a22571ef2`): devcons's #c/time (accept-and-record — no RTC this phase; the write succeeds, keeping settime's log clean) and a Rule-1 wiring fix found by the walk (#c/user serves `glenda` WITHOUT the newline — the donor's `readstr(up->user)` shape; the newline would have poisoned newns's `/usr/$user`). The walk's big find: sys_exec resolved devroot-only (the Phase-4 prefix scaffold), but the post-newns /bin union MREPLs onto the mounted tree — rc MUST exec through devmnt (the hinge's whole point). sys_exec now resolves via namec (D-03's one-way door), opens OEXEC through open_chan, and dispatches the byte source by device: DEV_ROOT keeps the 04-04 demand-paged paqfs window; DEV_MNT stages the whole file eagerly via `devmnt::read_all` into an Arc-shared buffer (proc.rs's `ByteBacking::Mnt` — the trap-context pager never RPCs; EXEC_STAGE_CAP 16 MiB, loud). exec.rs/fault.rs ride the ByteSource/ByteBacking abstraction (data_file_base semantics unchanged — the a.out layout is source-independent). The shipped /lib/namespace gained the binding 05-03 finding's re-mount pair (`mount -a #s/boot /root` + `bind -a /root /`) BEFORE its /root-prefixed binds — without them, newns's RFCNAMEG wipe strands the bin binds.
- **Task 3 (THE HINGE GATE): boot completes to the literal `% ` prompt.** The committed log (`.planning/phases/05-shell-boot-one-command-integration/05-07-boot-r9.log`) shows the full chain: `paqfs...` → the whole-image fingerprint (the server read+verified /rootfs.img) → `version...` with NO syscall-40 wall → FAUTH's designed `syscall 10 unimplemented` (twice — nsinit's and newns's fall-through) → `time...` (settime's second mount + stat + the #c/time write all succeed silently) → init's designed warnings (`#p/0/ctl` pri write — devproc ctl is 05-09's; `#c/sysname` — absent by design) → `init: starting /bin/rc` → `stacksmoke: depth 64 ok, checksum 2088960` (rc exec'd through the /bin union via devmnt; termrc ran) → `% ` then rc exits `"false"` — the literal wall line is `% process 4 exited: "false"` (the prompt print immediately followed by rc's exit: the cons read's loud -1 — devcons's designed 05-08 stub — ends rc's `. -i '#d/0'` source with status false) → init restarts rc (its `syscall 17 unimplemented` sleep is 05-08's trace-gated scope; the proc-table fill after ~13 restarts is the documented Moribund-slot-pinning era limit). Zero panics, zero `can't exec`, zero stale walls; all five staged profiles + ureg-check green on the final tree.
- **The iterate-discipline's yield — three latent bugs fixed at the gate** (r9 `f8976c7`, `8817144`; harvey `71fcf94ae`): (1) the first gate run wedged because the image carried the STAGED init (out/ is shared with the staged supply chain — "last writer wins"; r9's boot-test bake had re-polluted it) — and the staged init's four CONCURRENTLY-exec'ing children tripped the era's lock-held-blocking-walk limit on their shared pgrp (the wedge shape: the log stops dead mid-switch). mkimage.sh now asserts the real init at bake time (the rodata marker check: `init: starting /bin/rc` present, `init: r9 EL0 alive` absent) — pollution fails loudly with the repair named. (2) sys_seek read the WRONG ABI: the aarch64 stub is `_seek(vlong *rv, int fd, vlong offset, int base)` (the 9syscall mkfile's `j=_seek` arm + `_seek.c`'s wrapper + the donor sysfile.c sysseek's own comment) — the kernel reads slots 1/2/3 and writes the result through the validated `*rv` (05-06's version read slot 0 as the fd — a truncated user stack VA; its gate never caught it because every exercised seek was bio's in-buffer fast path; the hinge gate's paqfs blockRead seeks for real and checks). (3) the kernel heap's QuickFit tail path had TWO latent inversions that panicked on the FIRST big non-pow2 allocation (devmnt's ~250 KB exec staging buffer): BumpAlloc::try_alloc's params were named `(align, size)` while both call sites pass `(size, align)` (align_offset ran on the SIZE → "align is not a power-of-two"), and free_misc's fresh-header arm was inverted (`.is_null().then(carve)` + `.expect` panicked whenever the header malloc SUCCEEDED — the Vec realloc chain's first free of a tail block died). Both fixed + a host regression test; invisible until now because every prior big allocation never hit the tail path (quick-path misses are symmetric — adjust() makes size == align).

## Task Commits

Each task was committed atomically (two-repo convention — r9 commits land on `main` of `requaos/r9`; harvey commits on master):

1. **Task 1 RED: failing devmnt tests + evidence record** — r9 `91bfbbf` (test)
2. **Task 1 GREEN: devmnt + dat.rs slots + namec DEV_MNT legs + sys_fversion/sys_mount** — r9 `d4ae0e4` (feat)
3. **Task 2: devcons time file + #c/user donor shape + exec through the union (ByteBacking::Mnt)** — r9 `79d7185` (feat)
4. **Task 2 (harvey-side wiring): the namespace re-mount pair (the 05-03 finding)** — harvey `a22571ef2` (fix)
5. **Task 3 gate fix: sys_seek reads the real _seek ABI** — r9 `f8976c7` (fix)
6. **Task 3 gate fix: the allocator tail path (try_alloc arg order + free_misc header arm) + regression test** — r9 `8817144` (fix)
7. **Task 3 (harvey-side hardening): mkimage.sh's init-identity assertion** — harvey `71fcf94ae` (feat)
8. **HowItWorks.md material-change section (the header's mandate)** — r9 `454f0f2` (docs)

TDD gate sequence holds: RED (test `91bfbbf`) precedes GREEN (feat `d4ae0e4`), both present; no REFACTOR (the GREEN code was fresh-written and clippy-clean — tdd.md: commit only on change).

**Plan metadata:** (the harvey docs commit follows — SUMMARY + STATE + ROADMAP + REQUIREMENTS + the evidence log + deferred-items + WINDOWS.md)

## Files Created/Modified

- `../r9/port/src/devmnt.rs` — the 9P client: the wire codec (fcall.h/convS2M/convM2S ported), the Mnt session (negotiate/attach/walk/open/read/stat/clunk), the MntIo transport trait + the kernel ChanIo adapter, the Chan-wrapping device ops (walk/open/read/stat/clunk/fversion/attach/read_all)
- `../r9/port/tests/devmnt_test.rs` — the 8-test suite + the scripted fake server (the tree, the fid table, the malformation knobs) + the T-05-14 mount-crossing matrix over the devroot fixture
- `../r9/port/tests/05-07-red-evidence.json` — the RED evidence record (RED_EVIDENCE_OK, target_test_failed)
- `../r9/port/src/dat.rs` — DEV_MNT index; Chan's `mnt`/`mnt_fid` slots (dup clones both); the cross-crate `mnt_session()` getter
- `../r9/port/src/lib.rs` — `pub mod devmnt`
- `../r9/port/src/namec.rs` — the DEV_MNT legs (walk/open/stat/create-loud) + the close-dispatch match (DEV_PIPE/DEV_MNT)
- `../r9/port/src/devcons.rs` — the #c/time file (accept-and-record) + #c/user's donor-faithful bare `glenda`
- `../r9/port/src/allocator.rs` — the two tail-path fixes + the regression test
- `../r9/aarch64/src/syscall.rs` — sys_fversion + sys_mount (sargs grew a fifth slot); the pread/pwrite DEV_MNT legs; sys_seek's real ABI
- `../r9/aarch64/src/proc.rs` — ByteBacking (Paq/Mnt) + ByteSource::read_at
- `../r9/aarch64/src/exec.rs` — read_aout/install_image/map_data_bss_eager ride the backing; exec_replace takes ByteBacking; exec_into unchanged (main.rs untouched)
- `../r9/aarch64/src/fault.rs` — the pager reads via source.read_at (never RPCs)
- `../r9/HowItWorks.md` — the 05-07 section
- `build/aarch64/root-tree/lib/namespace` — the re-mount pair (05-03's binding finding)
- `build/aarch64/mkimage.sh` — the init-identity bake-time assertion
- `.planning/phases/05-shell-boot-one-command-integration/05-07-boot-r9.log` — the committed hinge-gate evidence
- `.planning/phases/05-shell-boot-one-command-integration/deferred-items.md` — three new entries (the pgrp hazard, #c/pid, the fid lifecycle)
- `.planning/WINDOWS.md` — two todo entries (the pgrp hazard; #c/pid)

## Decisions Made

- **The session rides the chan** (the donor's `c->mux`): fversion parks the Arc<Mnt> on the pipe fd's chan; dup clones it; the mount entry's `to` chan carries session + attach fid. This is what makes settime's and newns's SECOND mounts of `#s/boot` work (the posted clone carries the session; each mount is a fresh Tattach fid).
- **One outstanding RPC per session, enforced by an AtomicBool** — the era's answer to the donor's tag-multiplexing mountio/mountmux (which needs kernel readers r9 doesn't have). An MCS lock held across the blocking recv would spin-deadlock an IRQ-masked contender; the flag never blocks, so contention is a loud errstr. The pgrp-lock-across-blocking-walk hazard is documented + deferred (deferred-items.md + WINDOWS.md).
- **The 4072-byte per-RPC clamp** (the pipe's atomic-write bound dominates the negotiated 8192) — the donor's iounit clamp with the era's transport as the binding constraint.
- **Exec staging for mounted binaries** (ByteBacking::Mnt): whole-file eager read at exec (syscall context blocks by design; the trap-context pager never RPCs); 16 MiB loud cap.
- **The mkimage.sh init-identity assertion** — bake-time proof that out/init is the real init (the 05-02 out/-ownership tension made loud instead of silently-wrong-evidence).
- **FAUTH stays loud-None** (the plan's own must_have) — the log's two `syscall 10 unimplemented` lines are the D-02 fall-through working as designed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] sys_exec resolved devroot-only — retired for the namespace (the plan's own key_link)**
- **Found during:** Task 2's wiring walk (the plan's action item 2 names it: "exec through the union-mounted / — namec crosses the /root mount into devmnt")
- **Issue:** sys_exec walked the path through devroot directly (the Phase-4 prefix scaffold); the post-newns /bin union MREPLs onto the mounted tree, so /bin/rc would never resolve (the hinge fails)
- **Fix:** sys_exec resolves via namec + open_chan; the byte source dispatches by device (proc.rs's new ByteBacking: Paq unchanged; Mnt stages the whole file via devmnt::read_all); exec.rs/fault.rs ride the abstraction
- **Files modified:** `../r9/aarch64/src/syscall.rs`, `../r9/aarch64/src/proc.rs`, `../r9/aarch64/src/exec.rs`, `../r9/aarch64/src/fault.rs` (beyond the plan's Task-2 file list — the plan's action sanctions wiring fixes: "a gap here is a bug, not a new feature")
- **Verification:** the gate's stacksmoke marker + `% ` (rc exec'd through the union); the staged fan green through the new path
- **Committed in:** `79d7185`

**2. [Rule 3 - Wiring] the shipped /lib/namespace lacked the 05-03 finding's re-mount pair**
- **Found during:** Task 2's wiring walk — the finding (STATE.md, proven empirically on the donor) requires `mount -a #s/boot /root` + `bind -a /root /` before the /root-prefixed binds; the committed skeleton didn't have them
- **Fix:** the pair leads the shipped file (newns's RFCNAMEG wipe discards boot's mount table; without the pair the bin binds fail soft and init's exec stalls)
- **Files modified:** `build/aarch64/root-tree/lib/namespace`
- **Verification:** the gate's `init: starting /bin/rc` + the stacksmoke marker (the binds landed; rc resolved through the union)
- **Committed in:** `a22571ef2` (harvey)

**3. [Rule 1 - Bug] #c/user served "glenda\n" — the donor serves the bare name**
- **Found during:** Task 2's wiring walk (init's `readenv("#c/user")` feeds newns's `setenv("user", …)` → `/usr/$user`)
- **Fix:** USER_CONTENT is now `b"glenda"` (the donor's `readstr(up->user)` shape)
- **Files modified:** `../r9/port/src/devcons.rs`
- **Committed in:** `79d7185`

**4. [Rule 3 - Blocking, gate-surfaced] the image carried the STAGED init (out/ pollution) → wedge**
- **Found during:** Task 3's first gate run — the log stopped dead after `sched: switch 1->4` (the staged init's four concurrently-exec'ing children tripped the era's lock-held-blocking-walk limit on their shared pgrp; "init: r9 EL0 alive" in the log fingered the wrong binary)
- **Fix:** re-ran subset-build.sh (the real init restored to out/) + rebaked; AND hardened mkimage.sh: the bake now asserts the real init's rodata marker and refuses the staged one (the pollution fails loudly with the repair named)
- **Files modified:** `build/aarch64/mkimage.sh`
- **Verification:** the assertion fired (caught the polluted state during the debug), subset-build restored the real init, the rebake passed, the gate then progressed to init's real flow
- **Committed in:** `71fcf94ae` (harvey)

**5. [Rule 1 - Bug, gate-surfaced] sys_seek read the wrong aarch64 ABI**
- **Found during:** Task 3's second gate run — `paqfs: seek 1014634: seek: bad fd` (the server's image reads failed; the wire's slot 0 was a truncated stack VA, not the fd) — paqfs's walks answered Rerror
- **Issue:** the aarch64 stub is `_seek(vlong *rv, int fd, vlong offset, int base)` — FOUR args with the result written through *rv (the 9syscall mkfile's `j=_seek` arm + `sys/src/libc/aarch64/_seek.c`'s wrapper + the donor sysfile.c sysseek's own comment). 05-06's sys_seek read fd/offset/base from slots 0/1/2 and returned the offset on the wire; its gate never caught it (every exercised seek was bio's in-buffer fast path — the hinge gate's paqfs blockRead seeks for real and checks)
- **Fix:** rv validated writable + forced resident (the 05-06 cold-buffer class), fd/offset/base from slots 1/2/3, the result offset written through *rv, wire result 0
- **Files modified:** `../r9/aarch64/src/syscall.rs`
- **Verification:** the gate log has zero `seek: bad fd` lines; paqfs's serving walks answer
- **Committed in:** `f8976c7`

**6. [Rule 1 - Bug, gate-surfaced] the kernel heap's QuickFit tail path had two latent inversions**
- **Found during:** Task 3's third gate run — kernel panic `align_offset: align is not a power-of-two`, then `allocated header block` panic after the first fix
- **Issue:** (a) BumpAlloc::try_alloc's params were named `(align, size)` while both call sites pass `(size, align)` — align_offset ran on the SIZE and panicked on the first big non-pow2 tail allocation (devmnt's ~250 KB exec staging buffer); (b) free_misc's fresh-header arm was inverted — `.is_null().then(carve)` produced the carved header only when malloc FAILED, so `.expect` panicked whenever the header malloc SUCCEEDED (the Vec realloc chain's first free of a tail block). Invisible until now: every prior big kernel-heap allocation was alloc-never-free, and quick-path misses are symmetric (adjust() makes size == align)
- **Fix:** the params renamed to match the call sites (the real pow2 alignment reaches align_offset); the fresh-header arm uses the malloc'd header on success and carves only on null, then writes + links the Header as before; + a host regression test
- **Files modified:** `../r9/port/src/allocator.rs`
- **Verification:** `allocator::tests::tail_alloc_big_nonpow2_free_and_realloc` green; the gate's exec staging cycles (alloc/free per rc restart) run clean
- **Committed in:** `8817144`

**7. [Rule 2 - Era limit, documented + deferred] the pgrp lock is held across namec walks (including devmnt's blocking Twalk RPCs)**
- **Found during:** Task 3's debug (the staged init's wedge proved the hazard shape)
- **Issue:** a pgrp-SHARING contender (fork() shares the pgrp) that starts a mount-crossing RPC while the holder blocks mid-RPC spins on the MCS lock with IRQs masked — a wedge. Never trips in the gate (the real chain is strictly sequential: init parks in await while rc runs; rc waits per command; no pipeline ever runs), but the first concurrent namespace caller (rc pipelines, backgrounded commands) wedges
- **Fix:** NOT fixed this plan (the fix is a namec restructuring — snapshot the union chain under a brief lock, walk unlocked — and the measured-caller discipline assigns it to the first plan with a concurrent caller). The devmnt session's AtomicBool tripwire ("concurrent rpc" errstr) is the second line. Documented in deferred-items.md + WINDOWS.md (ship-visible)
- **Files modified:** none (documentation only)
- **Committed in:** the metadata commit (deferred-items.md + WINDOWS.md)

**8. [Process note] the RED evidence's output field is a TAP transcription**
- cargo's libtest has no `--format=tap` on this toolchain (pretty/terse/json/junit only); the tdd-red-evidence checker parses TAP (or Surefire XML). The record's output field transcribes the real run into TAP shape (the 05-06 record's established shape) — the actual `cargo test -p port --test devmnt_test` run failed 8/8 with the target test at the negotiation `.expect`; the transcription is verbatim-true to the run.

---

**Total deviations:** 8 (4 Rule 1 bugs — two gate-surfaced in pre-existing code, one wiring-walk, one 05-06 latent; 2 Rule 3 blocking/wiring; 1 Rule 2 era-limit documented; 1 process note)
**Impact on plan:** Every deviation is the hinge's iterate-discipline doing exactly what the plan designed ("the loud line names the gap — implement per the trace-settled discipline and rerun; do NOT silence"). The gate caught three real latent bugs that prior gates' exercise never reached. No scope creep beyond the plan's own wiring mandate; the one structural hazard found (the pgrp lock) is deferred with the fix sketched, per the measured-caller discipline.

## Issues Encountered

- **The out/init pollution cost the first gate run** (the staged init booted and wedged on the pgrp hazard) — recorded as deviation 4; the mkimage.sh assertion makes the class loud at bake time forever. NOTE for the next plans: r9's boot-test fan consumes out/ AS-IS and re-bakes when inputs are newer — after any subset-build.sh run, the fan re-bakes with whatever out/ holds, and after any stage-build.sh run, out/init is the STAGED init (the next real-chain bake then requires subset-build.sh first — mkimage.sh names this loudly). 05-10 owns the supply-chain consolidation (the 05-02 decision's revisit).
- **Three 600s TCG gate runs** (plus a 240s debug run) dominate the duration — each iteration is ~10 min of wall time. The marker sequence was decisive each time (the log's stop line named the gap: the wedge shape, then the seek errstr, then the panic lines).
- **The gate's end state is init's proc-table fill** ("init: fork error: proc: table full" → init exits → the parked paqfs server keeps the kernel alive until the harness timeout) — the documented Moribund-slot-pinning era limit (05-06's exit-sweep deferral's cousin); the markers all land long before it. 05-08's console read path is what turns the loop interactive.

## User Setup Required

None - no external service configuration required. (Driving run-r9.sh by hand needs qemu-system-aarch64 + fdtput on PATH — the r9 devshell provides both; `nix develop` from ../r9, then the gate command from this repo root.)

## Known Stubs

- **FAUTH(10) stays loud-None** — the designed D-02 path (nsinit/newns fall through to unauthenticated mounts; the log's two `syscall 10 unimplemented` lines are the design working). Intentional, per the plan's must_have.
- **devcons cons reads still return the loud 05-08 -1** ("devcons: cons reads arrive with 05-08") — the wall the gate stops at, by design (05-08's scope). rc's exit status "false" on the read failure is the wall's user-visible shape.
- **SLEEP(17)/ALARM(6) stay loud-None** — 05-08's trace-gated scope (the log's `syscall 17 unimplemented` per restart cycle + `syscall 6 unimplemented` once are the designed loud gaps; init tolerates both by construction).
- **WSTAT(44)** stays loud-None (execinit's chmod is unchecked on the donor too — `syscall 44 unimplemented` once, harmless).
- **devproc ctl + #c/pid absent** — init's pri-10 write degrades to its designed warning (names pid 0 until #c/pid lands — deferred for 05-09, its first real consumer).
- **devmnt's fid lifecycle divergences** (walk-intermediate leak; possible double-Tclunk against a stricter server) — bounded, documented in devmnt.rs's header + deferred-items.md.
- **The pgrp lock-held-blocking-walk hazard** — deferred (deviation 7); the first measured concurrent caller owns the fix.
- 05-04/05-05/05-06's known stubs are unchanged (cons reads until 05-08; devproc dir/status/args until 05-08/05-09; the exit-sweep deferral; the non-RFPROC fd-group split stays loud).

## Threat Flags

None beyond the plan's register. (T-05-13 held: the mntchk port bounds every R-message against the negotiated msize BEFORE the body read, tag-matches, and parses through a checked cursor with exact-consumption — host-proven rejection of oversized/mis-tagged/msize-raising replies; protocol errors are errstrs + serial-log lines, never panics, never partial reads. T-05-14 held: the mount-crossing matrix is host-pinned (MAFTER tie-break, union fall-through, MREPL shadow) and the frozen flag semantics govern insertion; the mount target resolves MountTarget-style, never redirected. T-05-SC held: zero package installs. The one new syscall surface — FVERSION/MOUNT — is the plan's own modeled surface, both handlers validation-first: the version buffer is validated + forced resident before any kernel touch (the 05-06 cold-buffer class), mount's fd must be an ORDWR 9P connection, afd ≠ -1 is loud-unsupported, and the flags get the donor's Ebadarg shape.)

## Next Phase Readiness

- **05-08 (the console read path + SLEEP/ALARM)**: the gate stops EXACTLY at the designed wall — rc's `% ` prints, the read gets the loud -1, rc exits "false", init restarts. devcons's read path + cooked mode + the prompt's interactivity land there; the restart loop's `syscall 17 unimplemented` is its trace-gated sleep scope. The hinge is proven; 05-08 turns the wall interactive.
- **05-09 (devproc + the smoke sweep)**: `#c/pid` is the named wiring gap (the deferred item); the smoke's command sweep rides devmnt reads through the /bin union (proven by the hinge) + the fd-group semantics (05-06).
- **05-10+**: the composition apps consume the hinge (the kernel ELF + the baked image + the qemu argv are all proven); the out/ ownership tension is documented for 05-10's revisit (mkimage.sh's assertion guards the real chain loudly meanwhile).
- r9 HEAD: `454f0f2` (main; code at `8817144`). Harvey HEAD: `71fcf94ae` + the metadata commit.
- The staged-profile fan is green on the final tree (all five + ureg-check), through the exec rewiring AND the allocator/seek fixes.

---
*Phase: 05-shell-boot-one-command-integration*
*Completed: 2026-11-03*

## Self-Check: PASSED

- Files: r9 port/src/devmnt.rs, port/tests/devmnt_test.rs, port/tests/05-07-red-evidence.json; harvey .planning/phases/05-shell-boot-one-command-integration/{05-07-SUMMARY.md, 05-07-boot-r9.log} — all FOUND on disk and committed.
- Commits: r9 91bfbbf (RED test), d4ae0e4 (GREEN feat), 79d7185 (Task 2 feat), f8976c7 (seek-ABI fix), 8817144 (allocator fix), 454f0f2 (docs); harvey a22571ef2 (namespace fix), 71fcf94ae (mkimage assertion) — all FOUND in the respective git logs; RED precedes GREEN (the TDD gate sequence holds; no REFACTOR — the GREEN code was fresh-written and clippy-clean).
- Plan-level <verification> re-run on the committed log: (1) host tests — `cargo test -p port` 55/55 green (8 devmnt + 1 allocator new; the RED evidence verified RED_EVIDENCE_OK); (2) builds virtshell/virtexec/default/raspi4b + clippy virtexec clean; (3) the hinge gate — 05-07-boot-r9.log shows paqfs... → the whole-image fingerprint → version... with NO syscall-40 wall → FAUTH's designed loud-None (×2) → time... (settime's mount+stat+#c/time write silent-success) → init: starting /bin/rc → stacksmoke: depth 64 ok → `% ` → the read wall (`% process 4 exited: "false"`) → the restart loop; the plan's grep chain all-pass (zero panic/Unhandled-interrupt, zero can't-exec, zero syscall-40); (4) boot-test fan — all five profiles PASS on the final tree + ureg-check 35/35.
- Acceptance criteria re-run at final state: Task 1 (devmnt implements version/attach/walk/open/read/stat/clunk over an fd with msize bounds + tag checks, host-proven against the scripted fake server; sys_fversion/sys_mount validation-first with the frozen flag semantics; FAUTH loud-None; builds+clippy+profiles green) — PASS. Task 2 (#c/time accepts settime's OWRITE; devenv create wired to sys_create; the boot-completion path's wiring confirmed — exec through the union landed, the namespace re-mount pair landed) — PASS. Task 3 (the log shows boot completing → init → newns → rc → termrc's stacksmoke marker → the literal `% ` prompt; the pri-10 degradation is a warning line; every staged profile + ureg-check green) — PASS.
