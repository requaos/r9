---
phase: 15-userspace-tiers-bootfs-payload-amd64
plan: 05
subsystem: kernel-venue
tags: [draw, vga, dispi, devdraw, multiboot, initrd, paqfs, kbin, mousein, qemu, amd64, uspace-08]
duration: 123 min
completed: 2026-11-10
requires:
  - "15-04-complete tier machinery (the libdraw family fresh in /amd64/lib — the kernel links exactly those archives; the GUI binaries the paq harvests)"
provides:
  - "the draw donor venue (D-04): k10draw boots under qemu -vga std with kernel-local devdraw serving — /dev/draw/new opens against a DISPI-set 1024x768x32 screen"
  - "sys/src/9k/k10/vgadispi.c — the qemu-native DISPI VGAdev (route 1 proven under KVM: A1 confirmed, no realmode fallback needed)"
  - "the -initrd payload channel: qemu -initrd registers as an mb1 module, multiboot.c exposes the bytes via addbootfile, in-guest paqfs mounts them (A2 confirmed; the -append -v pass prints the mod evidence)"
  - "the bootfs-draw.paq bake: the WHOLE lib/font/bit tree (4303 files) + the tier-3 GUI harvest computed from the tier data + mntgen/paqfs/drawsmoke; 20240113 bytes (the A3 measurement — -m 512 holds)"
  - "the eve-gated /dev/kbin (devcons.c) — synthetic keyboard through the standard kbdputc path — plus the proven /dev/mousein position readback: the 15-06 replay surface"
  - "a meminit fix (asm.c) making k10 tolerate reserved spans above the kernel image — the identity window now maps [vmstart, vmend) with the extension base derived arithmetically (no page-table walk)"
  - "/lib and /tmp devroot mount points (the 15-02 /rc precedent — the embedded root is read-only)"
affects:
  - "15-06 (the trace rig keys off this venue: fonts, GUI binaries, kbin/mousein replay, mntgen /tmp scratch)",
  - "15-08 (the evidence bundle records the A1/A2 outcomes)", "15-09 (compile green unchanged; its libmemdraw audit still precedes the trace capture)"
tech-stack:
  added: []
  patterns:
    - "the draw wire byte order rides draw.h's own BG/BP macros — this tree flipped them LITTLE-endian vs stock Plan 9's big-endian; raw-protocol exercisers must use the header macros, never hand-rolled packing"
    - "venue boot scripts bind #i AFTER the size verb (devdraw's attach installs the screen image at bind time) and always with union binds onto /dev (plain bind is MREPL over initcode's #c)"
    - "device probes with bounded waits live in the C exerciser (alarm+atnotify), never in rc console-interaction forms — the 15-02 dialect's least reliable corner wedges the whole boot"
key-files:
  created:
    - sys/src/9k/k10/k10draw
    - sys/src/9k/k10/vgadispi.c
    - sys/src/9k/k10/root/k10draw.rc
    - build/x86_64/drawsmoke.c
    - build/mm/build-9kdraw.sh
  modified:
    - sys/src/9k/k10/multiboot.c
    - sys/src/9k/k10/asm.c
    - sys/src/9k/port/devroot.c
    - sys/src/9k/port/devcons.c
    - .gitignore
key-decisions:
  - "A1 confirmed: the qemu std-vga VBE DISPI interface (ports 0x1CE/0x1CF) sets 1024x768x32 under KVM on qemu 11.1.2 — route 1 stands, the realmode fallback (route 2) never needed"
  - "A2 confirmed: -initrd registers as an mb1 module under -kernel; the evidence is the `mod ...` boot print from the -append -v verbose pass (main.c's second multiboot walk) — zero changes to main.c"
  - "A3 measured: the 36MB font tree deflates to 20240113 bytes (~19.3MB, above the ~15MB guess) — the venue holds at -m 512 with kernel 230M/user 257M reported at boot"
  - "OQ-2 resolved: the 15-06 replay scripts are committed in-guest data driven through kbin/mousein (the eve-gated files); serial chardev injection remains only as the bring-up bootstrap"
  - "the input probes live in drawsmoke (C, alarm-bounded) rather than k10draw.rc — the rc console-interaction forms wedge the boot (measured twice: the @{} background block died silently; the flat write-then-read ladder blocked on the console echo interplay)"
  - "the kbin handler feeds each byte as a rune (the sibling-9front shape) — a chartorune walk produced shifted staged runes through an interaction this plan did not root-cause (documented in Deviations #5)"
patterns-established:
  - "Venue boot scripts: union binds onto /dev; #i last (after size); paqfs rides an EXISTING devroot mount point (read-only root); C exercisers carry anything needing bounded waits"
  - "Kernel identity window: the extension base is vmunmapped's own KZERO offset; the whole [vmstart, vmend) window maps at meminit — reserved spans above the kernel image are reachable"
requirements-completed: [USPACE-08]
actuals:
  tokens: 11800   # chars/4 over the realized diff (47307 chars, 10 files, 1278 insertions — estimate was 78000: the same over-projection direction as 15-01's 60000->10700 and 15-04's 38000->6100)
  tasks: 3
  commits: 3      # measured: git rev-list --count b8f9ea33e..HEAD
status: complete
coverage:
  - id: D1
    description: "The draw donor venue boots (D-04): k10draw config + the DISPI VGAdev + the boot leg — /dev/draw/new opens through the userspace-driven mode-set, proven by drawsmoke's alloc/paint/readback"
    requirement: USPACE-08
    verification:
      - kind: command
        ref: "nix develop -c sh build/mm/build-9kdraw.sh: exit 0, DRAW-SMOKE-OK + DRAW-VENUE-DONE on the serial capture, amd64/9kdraw.elf landed, k10cpu re-proved, k10 tree git-clean; frozen configs k10cpu/k10t1/k10f diff-quiet at HEAD"
        status: pass
      - kind: command
        ref: "tracer feedback gate (end-of-phase mode): both <automated> verifies re-run post-commit green (exit 0, byte-stable marker set; static greps all pass)"
        status: pass
  - id: D2
    description: "The -initrd font channel: the multiboot addbootfile delta exposes the module, in-guest paqfs mounts it, rio's and acme's default fonts stat-able — FONT-CHANNEL-OK with the mod boot print as A2 evidence"
    requirement: USPACE-08
    verification:
      - kind: command
        ref: "venue capture: FONT-CHANNEL-OK + DRAW-SMOKE-OK + one `mod 0x317000 0x16646ed <...bootfs-draw.paq>` line; bootfs-draw.paq 20240113 bytes, header 529ab12b/trailer 6b46e688; no-initrd regression boot green after the meminit change"
        status: pass
  - id: D3
    description: "The eve-gated synthetic-input surface: /dev/kbin behind the iseve() gate feeding kbdputc, /dev/mousein driving mousetrack — the complete four-marker bring-up gate on one boot"
    requirement: USPACE-08
    verification:
      - kind: command
        ref: "final plan-level run at HEAD: DRAW-SMOKE-OK + FONT-CHANNEL-OK + KBIN-OK + MOUSEIN-OK + DRAW-VENUE-DONE on one capture, zero panic/trap, exit 0; grep: kbin count 5 in devcons.c, the Qkbin open case carries iseve()+error(Eperm)"
        status: pass
---

# Phase 15 Plan 5: Userspace Tiers + Bootfs Payload (amd64) — Draw Donor Venue Summary

**The kernel-local devdraw donor venue: a DISPI VGAdev sets 1024x768x32 under qemu -vga std with /dev/draw proven alloc/paint/readback end-to-end, the 36MB font payload reaches the guest through -initrd→addbootfile→paqfs (no srv9), and the eve-gated kbin/mousein synthetic-input surface is proven on one four-marker boot — including a meminit fix that makes the k10 kernel tolerate module spans above the kernel image.**

## Performance

- **Duration:** 123 min
- **Started:** 2026-11-10T03:24:03Z
- **Completed:** 2026-11-10T05:26:53Z
- **Tasks:** 3/3 (Task 1 tracer)
- **Files modified:** 10 (5 created)

## Accomplishments

- **The venue exists (Task 1, tracer):** k10draw (k10t1's device set + k10f's draw deltas, vgadispi in the vga section, the libdraw family ahead of k10cpu's libs) boots text-mode, the boot script drives `type dispi`/`linear`/`size`/`drawinit` through #v/vgactl, the LFB maps via vgalinearpci with the vesa soft-screen discipline, and drawsmoke proves the whole client-side draw loop — including reading the painted pixels back. The tracer gate's post-commit re-run reproduced the green.
- **The 36MB problem is solved without srv9 (Task 2):** qemu -initrd registers the paq as an mb1 module (the mod boot print is on the capture), multiboot.c's ~3-line delta exposes the bytes through addbootfile, and the tier-1-built paqfs mounts them over /mnt — the fonts bind at /lib, the GUI binaries union into /bin, mntgen serves /tmp. FONT-CHANNEL-OK with both apps' default fonts stat-able.
- **The input surface is complete and eve-gated (Task 3):** /dev/kbin (the mousein discipline verbatim — dirtab row + iseve() open gate + a write handler feeding kbdputc) round-trips a line to a /dev/cons reader; 'M 512 384 0 0' through /dev/mousein reads back through /dev/mouse. All four markers on one boot.
- **A latent kernel bug fixed (the biggest surprise):** stock meminit derived the kernel-window extension base from a page-table walk of vmunused and mapped only [vmunmapped, vmend) — both wrong once a reserved span sits above the kernel image. The fix is arithmetic (vmunmapped's own KZERO offset) plus mapping the whole window; the no-initrd boot was re-proven green after the change.

## Task Commits

1. **Task 1: the draw donor boots** — `ed1a0af37` (feat)
2. **Task 2: the -initrd font channel** — `5fddc8d7f` (feat)
3. **Task 3: synthetic input** — `6a648bd2b` (feat)

## Files Created/Modified

- `sys/src/9k/k10/k10draw` — the draw donor config (k10t1 set + draw deltas; DISPI route; 11-file embed)
- `sys/src/9k/k10/vgadispi.c` — the qemu-native VGAdev (DISPI mode-set, vgalinearpci LFB, soft-screen flush)
- `build/x86_64/drawsmoke.c` — the raw #i exerciser + the bounded kbin/mousein probes (plain libc; draw.h's LE BPLONG macros)
- `sys/src/9k/k10/root/k10draw.rc` — the venue boot script (union binds, paqfs channel, mode-set choreography)
- `build/mm/build-9kdraw.sh` — build (CONF=k10draw nuke sandwich + k10cpu re-proof) + the bootfs-draw.paq bake + the draw boot leg with the four-marker classify
- `sys/src/9k/k10/multiboot.c` — the addbootfile module exposure (the Fmods else branch)
- `sys/src/9k/k10/asm.c` — meminit: arithmetic extension base + full-window identity mapping
- `sys/src/9k/port/devroot.c` — /lib and /tmp mount points (the /rc precedent)
- `sys/src/9k/port/devcons.c` — the eve-gated kbin file
- `.gitignore` — .draw-stage/ and the parse-generated k10draw.c

## Decisions Made

See key-decisions in the frontmatter. The load-bearing ones: A1/A2 both confirmed (no route pivot), the OQ-2 in-guest replay resolution, the probes-in-C decision after the rc forms wedged twice, and the byte-per-rune kbin handler.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The plan's mk target name does not exist**
- **Found during:** Task 1 (first build)
- **Issue:** The plan says `mk CONF=k10draw 9kdraw`; the k10 mkfile's target is `$p$CONF` with `p=9` — the product is 9k10draw (the 9k10cpu/9k10t1 family naming); `9kdraw` reds with "don't know how to make".
- **Fix:** Build `mk CONF=k10draw 9k10draw`; the venue artifact lands at /amd64/9kdraw.elf (the plan's named product) via the script's copy step.
- **Files modified:** build/mm/build-9kdraw.sh
- **Verification:** the venue gate green end-to-end.
- **Committed in:** ed1a0af37

**2. [Rule 3 - Blocking] The userns scaffold resolved GNU sed and the $CONF.c rule reded**
- **Found during:** Task 1 (first build)
- **Issue:** build-9k10t1.sh passes the devshell's HARVEY_LINUX_BIN (the hostbin dir carrying the Plan 9 sed/awk dialects) into the chroot PATH head; my script hardcoded a nonexistent linux_amd64/bin path, sed fell through to GNU sed, and the kernel's sed recipe died with "invalid reference \1" (GNU BRE lacks the plan9 dialect).
- **Fix:** Use the devshell-exported HARVEY_LINUX_BIN (computed NS_HBIN fallback), exactly the t1 script's discipline.
- **Files modified:** build/mm/build-9kdraw.sh
- **Verification:** the namespace build green from the next run on.
- **Committed in:** ed1a0af37

**3. [Rule 1 - Venue correctness] Plain device binds are MREPL and the #i bind order matters**
- **Found during:** Task 1 (first green build's boot)
- **Issue:** `bind '#i' /dev` (a) REPLACES initcode's #c mount (the /dev/time write then failed and pid 1 died), and (b) devdraw's ATTACH installs the screen image — binding #i before the size verb fails the bind itself ("#i: no frame buffer").
- **Fix:** Union binds (-a) for #v/#m/#i onto /dev, and #i bound only after the size/drawinit choreography.
- **Files modified:** sys/src/9k/k10/root/k10draw.rc
- **Verification:** the capture shows the full verb chain then drawsmoke output.
- **Committed in:** ed1a0af37

**4. [Rule 1 - Kernel bug] meminit cannot fit or map reserved spans above the kernel image**
- **Found during:** Task 2 (the first -initrd boot wedged in meminit; then paqfs faulted on unmapped module bytes)
- **Issue:** (a) the extension base came from mmuphysaddr(vmunused) — the walk returns a wrong-level entry for spans beyond the early 2MiB pages, so the asmalloc fit loop could never satisfy its address and the boot wedged; (b) the mapping loop started at vmunmapped on the belief that the early pages covered below — the module bytes at [0x400000, 0x1600000) were unmapped and readers faulted.
- **Fix:** the extension base is vmunmapped's own KZERO offset (identical arithmetic for healthy walks — a no-op for the classic boot), and the identity map covers the whole [vmstart, vmend) window.
- **Files modified:** sys/src/9k/k10/asm.c
- **Verification:** the 20MB-module venue boots green; a manual no-initrd boot re-proved the classic path (kernel alloc space + DRAW-SMOKE-OK); k10cpu relinks green in every run.
- **Committed in:** 5fddc8d7f

**5. [Rule 1 - Protocol bug] The draw wire is LITTLE-endian on this tree; the rc input probes wedge; the chartorune walk shifted runes**
- **Found during:** Tasks 1 and 3 (bring-up iterations)
- **Issue:** (a) drawsmoke's hand-rolled big-endian packing byte-reversed every field (the kernel parsed id 0x02000000 — caught by a temporary kernel diagnostic print, since reverted); (b) the rc @{} background reader died silently and the flat write-then-read ladder blocked on the console-echo interplay (a U+FFFD led the echoed bytes and the newline never surfaced — not root-caused beyond that); (c) a chartorune walk in the kbin handler staged shifted runes in the same echo.
- **Fix:** (a) use draw.h's own BPLONG macros (single source of truth); (b) the probes moved into drawsmoke with alarm+atnotify bounded reads; (c) the handler feeds each byte as a rune (the sibling-9front shape) — the round trip is green and deterministic.
- **Files modified:** build/x86_64/drawsmoke.c, sys/src/9k/port/devcons.c, sys/src/9k/k10/root/k10draw.rc
- **Verification:** KBIN-OK + MOUSEIN-OK on the four-marker boot.
- **Committed in:** 6a648bd2b

**6. [Rule 3 - Venue reality] The embedded root is read-only and /lib, /tmp did not exist**
- **Found during:** Task 2 (mkdir denied: "/mnt/paq permission denied")
- **Issue:** devroot serves a read-only tree — no mkdir — and /lib (the font bind target) and /tmp (the mntgen scratch) were not rootdirs.
- **Fix:** paqfs mounts OVER /mnt (an existing mount point); devroot gains /lib and /tmp (the 15-02 /rc precedent, additive and harmless to every venue); mkdir dropped from the embed (11 files).
- **Files modified:** sys/src/9k/port/devroot.c, sys/src/9k/k10/root/k10draw.rc, sys/src/9k/k10/k10draw, build/mm/build-9kdraw.sh
- **Verification:** FONT-CHANNEL-OK with the fonts at their literal /lib/font/bit paths.
- **Committed in:** 5fddc8d7f

**7. [Rule 3 - Verify environment] The bare `sh build/...` verify cannot run in the ambient shell**
- **Found during:** Task 1 verify
- **Issue:** The script needs the nix devshell (plan9port rc/mk, the hostbin toolchain, qemu) — the 15-04 deviation #4 class.
- **Fix:** The verify executed as `nix develop -c sh build/mm/build-9kdraw.sh` (the script's own documented invocation); the greps ran against the same /tmp logs.
- **Verification:** every gate run in this summary.
- **Committed in:** ed1a0af37

---

**Total deviations:** 7 auto-fixed (2 kernel bugs, 2 venue correctness, 3 blocking/environment). **Impact on plan:** all inside the plan's own disciplines — the venue's shape, channel, and gates are exactly as planned; the meminit fix is the one piece of unplanned kernel surgery (documented, regression-proven, and load-bearing for every future module-bearing k10 boot).

## Authentication Gates

None.

## Known Stubs

None — every marker is earned by a real runtime path; the four-marker gate runs on every boot of the venue.

## Issues Encountered

- The bring-up took eight boot cycles (bind order, wire byte order, meminit, mount points, rc dialect, rune walk) — each diagnosed from the serial capture, two with one-shot kernel diagnostics (reverted before commit). The chartorune-vs-echo interaction (deviation #5b/#5c) is the one observed-but-not-root-caused behavior; the committed byte-per-rune handler sidesteps it and the round trip is deterministic across the final runs.

## Next Plan Readiness

READY for 15-06 (the trace rig): the venue boots with fonts at their literal paths, the GUI binaries in /bin (rio/acme/sam/samterm/plumb/plumber + the utility class), the mntgen /tmp scratch live, and the kbin/mousein replay surface proven — the committed replay scripts drive those files; tracesup builds against this venue next. NOTE for 15-08: the A1/A2 outcomes (both confirmed, no pivot) and the A3 measurement (20240113 bytes) are recorded here and in the key-decisions. NOTE for 15-09: sequenced before 15-06's capture per D-07; nothing in this plan touched libmemdraw sources.

## Self-Check: PASSED

- files exist: k10draw, vgadispi.c, k10draw.rc, drawsmoke.c, build-9kdraw.sh + the 5 modified ✓
- commits are ancestors of HEAD: ed1a0af37, 5fddc8d7f, 6a648bd2b ✓
- plan-level verification green at HEAD: fresh full run exit 0, all four markers + DRAW-VENUE-DONE on one capture, zero panic/trap ✓

---
*Phase: 15-userspace-tiers-bootfs-payload-amd64*
*Completed: 2026-11-10*
