# Scale audit — owner targets of 2026-11-27

Audit only: no code changed. "measured" means run on the KVM host against the
branch tip's kernel (`481a330`); "read" means taken from the source;
"computed" means arithmetic on values read from the source.

## Targets (owner, 2026-11-27)

- **RAM:** more than 100 GiB.
- **Storage:** hundreds of terabytes.
- **File paths:** expected to exceed 2048 characters.

> "Let's make sure we are planning things accordingly at this fundamental
> level."

## Summary

1. None of the three targets is stated in the plans. ROADMAP, REQUIREMENTS,
   PROJECT, the intel notes and the architecture and protocol documents carry
   no memory-size, direct-map, name-length or path-length statement, and the
   verification plan configures its VMs with 2 GiB
   (`r9-permissions-and-test-plan.md:88,118,139`).
2. The x86_64 kernel cannot use more than about 4 GiB of RAM today, and it
   fails silently: 2 GiB and 4 GiB boot to the prompt; 8 GiB hangs before its
   first line of output (measured).
3. The kernel rejects every path argument longer than 127 bytes (read).
4. No storage stack exists yet; the page cache's page index is 32-bit, a
   16 TiB ceiling per image (read).

## 1. Memory

| # | Finding | Source | How |
|---|---|---|---|
| M1 | RAM is reachable only through one 1 GiB page. Every physical access the kernel makes (`phys_ptr_mut`) asserts `pa < 1 GiB`. | `x86_64/src/l.S:613-616`, `x86_64/src/proc.rs:136`, `:455` | read |
| M2 | 8 GiB: panics `PA … outside the KZERO window` before the console exists and spins in the panic handler forever. 2 GiB and 4 GiB: boot to the prompt. Cause: the allocator pre-zeroes 15% of all frames and zeroes them through that window — 0.6 GiB at 4 GiB, 1.2 GiB at 8 GiB. | `proc.rs:455`, `port/src/palloc.rs:51,362`, `x86_64/src/pagealloc.rs:229-232` | measured |
| M3 | The alias cannot simply be enlarged where it is: `KZERO` sits 2 GiB below the top of the 64-bit space. | `l.S:32` | computed |
| M4 | 127 TiB of unused kernel virtual space in PML4 slots 256..509 (only slot 400 is taken, by the per-process kernel-stack window). | `proc.rs:184` | computed |
| M5 | One 4-byte pin count per frame of the machine's whole address span, allocated at boot from the kernel heap: 100 MiB at 100 GiB of RAM, 1 GiB at 1 TiB. | `port/src/palloc.rs:263,347-349` | read |
| M6 | The kernel heap is a fixed 32 MiB `.bss` arena and exhaustion panics. M5 alone exhausts it at a 32 GiB address span. | `port/src/globalalloc.rs:153`, `x86_64/src/runtime.rs:27` | read |
| M7 | Pool targets are percentages of RAM: 15% of frames kept pre-zeroed and 5% low water — 15 GiB held and zeroed at boot on a 100 GiB machine. | `palloc.rs:51,362-363` | read |
| M8 | Device memory at or above 4 GiB panics, all PCI and virtio MMIO goes through that path, and firmware-assigned BAR addresses are kept as assigned. | `proc.rs:121-127`, `x86_64/src/platform/q35/pci.rs:227-689,828-839` | read |
| M9 | The whole firmware memory map is capped at 32 entries. | `x86_64/src/platform/q35/mb2.rs:181,220,252` | read |
| M10 | Static caps: process table 512 (2048 under the churn profile), never reaped; 32 descriptors per process; 64 CPUs; 64 MiB boot-module stash in `.bss`; image page cache of 1024 blocks, sized "for the boot image". | `proc.rs:151-163`, `port/src/smp.rs:57`, `l.S:85`, `pagealloc.rs:32-35` | read |
| M11 | Frame numbers are `u32`: fine to 16 TiB, a ceiling to record. | `palloc.rs:68,79` | read |
| M12 | No IOMMU; DMA uses physical addresses directly. Whether every DMA path is 64-bit was not checked. | | unverified |
| M13 | Boot today is BIOS + GRUB. Large machines boot via UEFI; GRUB provides multiboot2 there too. Not tested. | | unverified |

Reproduce M2 (the ISO is the composed chain ISO; `-smp 1` keeps the kernel-stack
wedge out of the picture):

```
qemu-system-x86_64 -M q35 -cpu host -smp 1 -m 8192M -accel kvm -cdrom boot.iso \
    -nographic -no-reboot        # nothing after GRUB's "Booting `r9'"
qemu-system-x86_64 ... -m 4096M  # reaches the "% " prompt
```

## 2. Storage

| # | Finding | Source | How |
|---|---|---|---|
| S1 | No block device driver exists (no NVMe, AHCI or virtio-blk); the storage engine is unplanned work (ROADMAP Phase 13). | | read |
| S2 | The 9P2000.aead wire carries file offsets and cursors as `u64` and Qid paths as `u64`; `iounit` is `u32`; bodies are 4 KiB pages. | `rfc-9p2000-aead.md:277-280,14` | read |
| S3 | The page cache key's page index is `u32`: 16 TiB per image. | `port/src/pagecache.rs:55-57` | read |
| S4 | The boot RAM-disk filesystem uses `u32` block offsets. Boot-only; not the path a disk-backed root takes. | `port/src/paqfs.rs:69,170` | read |

## 3. Paths

| # | Finding | Source | How |
|---|---|---|---|
| P1 | Every path or name argument is copied from user memory with a 128-byte bound and a longer one fails with "string too long" — 13 call sites across exec, open, create, stat, wstat, remove, bind, mount, unmount and chdir. | `x86_64/src/syscall.rs:313,633,675,679,714,752,759,869,873,937,1046,1137,1539`; `ERRMAX` at `proc.rs:141`; `x86_64/src/usermem.rs:331` | read |
| P2 | exec accepts at most 32 arguments, each at most 128 bytes, and a path at most 128. | `syscall.rs:311-313` | read |
| P3 | The process name is a 28-byte field, silently cut to 27 bytes: `ps` and `#p` show truncated names for long elements. | `proc.rs:1312,1549` | read |
| P4 | Kernel device servers bound element names individually; one found: environment names. Each server needs checking. | `port/src/devenv.rs:180` | read |
| P5 | The protocol documents state no element-name maximum. Standard 9P strings are length-counted; the server-side limit becomes a Fossil/Venti design decision. | `rfc-9p2000-aead.md:309` | read |

## 4. What already fits

- Physical addresses are `u64` (`port::mem::PhysAddr`).
- File offsets and cursors on the wire are `u64` (S2).
- The user address space is 128 TiB (`x86_64/src/exec.rs:647`).
- Path resolution holds no whole-path buffer and sends one element per Twalk,
  so a long path is never limited by the 16-element walk bound
  (`port/src/namec.rs:186-262`, `port/src/devmnt.rs:856-872`,
  `port/src/chan/msg.rs:29`).

## 5. Planning items proposed (owner decisions)

1. **Record the targets as requirements**, each with a number and a
   verification leg, so later phases plan against them instead of the gate
   guest's 1 GiB.
2. **Kernel memory model (x86_64).** A direct map of all RAM, built at boot
   from the firmware memory map into free PML4 slots (M3, M4), covering RAM
   ranges only; keep the kernel image mapped where it is linked; retire the
   1 GiB assert (M1, M2). Device memory through a mapping facility instead of
   the fixed 4 GiB alias (M8).
3. **Frame metadata and heap.** No per-frame arrays from a fixed heap: sparse
   pin counts, or metadata carved from RAM at boot (M5, M6). A kernel heap
   that grows from the frame allocator. Pool targets as absolute bounds, not
   percentages (M7).
4. **Caps.** Decide which of M9 and M10 become dynamic or are raised: memory
   map entries, process table and descriptor tables, CPU count, module stash,
   page-cache sizing.
5. **Paths.** A named path bound chosen for the target (a single constant, set
   well above 2048, or unbounded up to a memory limit) in place of `ERRMAX`
   (P1); exec limited by total size rather than 32 × 128 (P2); a
   variable-length process name (P3); every kernel device server's element
   bound checked (P4); an element-name maximum and length-counted names
   required in the storage design (P5).
6. **Storage.** 64-bit block addresses, offsets and sizes end to end; the page
   cache key's page index to `u64` (S3); page-cache and index structures
   sized for RAM rather than the boot image.
7. **Verification.** A large-memory boot leg (cheap: QEMU only backs guest
   memory the guest writes — today's 32 GiB run cost nothing) and long-path
   and large-file cases in the test harness, whose VMs are 2 GiB today.

## 6. Effect on the pending kernel-stack decision

The first-GiB alias is not a foundation to build on: it is the limit that
stops the kernel at 4 GiB today. The `-smp 2` fix should not wait for the
memory-model work, but it should be built so that work does not undo it:

- either a kernel-stack arena reserved at boot in low memory next to the
  kernel image, reached through whatever direct map exists — only the
  physical-to-virtual translation moves when item 2 lands, since the kernel
  image's own mapping stays where it is linked;
- or the shared virtual window with per-process entries, which does not depend
  on the direct map at all.

Recommendation unchanged in kind: the arena, as the smaller change; and item
2 opened as planning work in its own right.

## 7. Status (2026-11-28)

Done on branch `feat/x86_64-memory-model` (stacked on the two-CPU fix and the
sleep fix; local, nothing pushed):

- **Item 2, the direct map (M1, M2, M3, M4).** `x86_64/src/directmap.rs` maps
  the RAM above the first gibibyte at `0xffff_8000_0000_0000 + pa`, into the
  PML4 slots 256..=509 the boot tables leave free (127 TiB of virtual space,
  physical addresses to 128 TiB), at boot from the firmware map before the
  frame allocator is seeded: 1 GiB pages for aligned gibibytes, 2 MiB pages at
  range edges, and the sub-2 MiB edges withheld from the pools. Its table
  pages (two on every shape measured) come from a low-RAM reservation beside
  the kernel-stack arena. `phys_ptr_mut` reaches the first gibibyte through
  KZERO as before and the rest through the map; every process PML4 copies the
  kernel half from the boot table (slots 256..=511). The kernel image and the
  stacks keep their addresses, so a 1 GiB guest builds no map and runs as
  before.
- **Item 3, frame metadata and pool bounds (M5, M7).** `port/src/palloc.rs`
  keeps the managed set and a pin count only for frames that have one (a map,
  not a table sized by the machine). The clean target and the low-water mark
  are capped at 64 MiB and 16 MiB; the percentages hold below that.
- **Item 4, two caps raised.** The firmware memory map keeps 128 entries (was
  32) and the boot-information stash is 32 KiB (was 4 KiB).
- **Found on the way, M14: the demand pager's mapping log.** Every mapped page
  had a 32-byte record in a `Vec` on the 32 MiB kernel heap, and every page
  fault asked "is this page mapped?" by scanning it. A process mapping 1 GiB
  needed an 8 MiB heap vector, then a 16 MiB one, where the heap ran out
  (`oom: Layout { size: 16777216 }`), after a quarter of a million linear
  scans. It is now a radix tree keyed by address (`x86_64/src/maplog.rs`):
  four levels of 512-entry nodes, each one page from the frame allocator;
  insert, lookup and removal take a few table reads; about one page per 512
  mapped pages; no `unsafe` of its own.
- **Found on the way, M15: a console line per fault.** Every demand page-in
  printed a line (a console lock and thirty-odd UART bytes a page). The lines
  are now behind the `faulttrace` boot token.
- **M8 on this venue.** QEMU keeps the whole PCI hole below 4 GiB at every RAM
  size measured from 8 GiB to 128 GiB (2 GiB of RAM below the boundary, the
  rest above), and the NIC driver already moves a firmware BAR assigned above
  4 GiB into the low window. Device memory at or above 4 GiB stays unsupported
  on a host whose firmware places BARs high: recorded, not built, since it
  cannot be exercised here.

Measured on the KVM host (a 100 GiB anonymous guest is refused by the host's
overcommit heuristic; a sparse file-backed one is not):

| Guest | Frames managed | Map: spans, bytes, table pages | First output | Leg |
|---|---|---|---|---|
| 8 GiB | 2,060,286 | 2, 0x1bfe00000, 2 | 0.63 s | pass, 15.6 s |
| 32 GiB | 8,351,742 | 2, 0x7bfe00000, 2 | 1.14 s | pass, 15.6 s |
| 100 GiB | 26,177,534 | 2, 0x18bfe00000, 2 | 1.62 s | pass, 19.1 s |
| 128 GiB | 33,517,566 | 2, 0x1fbfe00000, 2 | 2.15 s | pass, 19.1 s |

The leg: a tier image whose sweep is replaced by one process that doubles a
string to 1 GiB and compares its halves. About 2 GiB of heap is mapped, frames
above 1 GiB and above 4 GiB are handed out (the kernel prints the first of
each), and the bytes read back intact (`MEM-OK 1073741824`). The kernel heap
stays under 3 MiB of its 32. Before this work an 8 GiB guest panicked before
its first line of output.

Limits that are not the kernel's, recorded for the owner:

- The user library's malloc caps one process's heap at (3840 − 1) MiB and one
  allocation below 2 GiB (`sys/src/libc/port/malloc.c`, `pool.c`).
- exec takes at most 32 arguments of 128 bytes, and the kernel caps each
  path-like argument at 128 bytes (`ERRMAX`). The leg's first workload, an awk
  program as one argument, failed on that. Item 5.
- `brk` maps every grown page eagerly rather than on fault.
- The frame number is `u32` (16 TiB, M11); the direct map covers 128 TiB.

Still open from section 5: the rest of item 3 (the kernel heap is still one
fixed 32 MiB arena: nothing in it now grows with RAM, but it does not grow
either, and the QuickFit side table holds 4096 headers); the rest of item 4;
item 6; item 7's large-file cases.

## 8. Paths (2026-11-28)

Done on branch `feat/x86_64-paths` (stacked on the memory branch; local,
nothing pushed):

- **P1, the path bound.** `port::namec::PATHMAX` (16 KiB) replaces
  `ERRMAX` at every path or name argument the syscall layer copies from
  user memory: 13 call sites on x86_64 (exec's path, open, create, chdir,
  remove, stat, wstat's name form, bind's two, unmount's two, mount's two)
  and the same 13 on aarch64. `ERRMAX` stays what it is for error strings
  and note messages.
- **P2, exec's arguments.** Bounded by bytes, not by count: each string
  with its NUL and slot must fit the eager stack under the Tos
  (`EXEC_ARGS_BYTES`, 256 KiB less the Tos). The seed writes down through
  the eager stack pages instead of one page (was 32 arguments of 128
  bytes, all in the top page).
- **P3, the process name.** A variable-length field; it was cut at 27
  characters.
- **P4, device servers.** The environment device bounded element names at
  127 bytes (the donor's genbuf); it now uses `PATHMAX`. The others were
  checked: `devsrv` and `devip` bound numeric strings, `devssl` bounds a
  control line (128 bytes, a verb and its arguments, not a name), `devmnt`
  passes names through, and the 9P message layer counts walk names (16 per
  message) but bounds each only by the wire's 16-bit length.
- **P5** is left to the storage design: an element name is at most 64 KiB
  on the wire and the kernel accepts `PATHMAX`.

Measured, in the guest (a tier image whose sweep passes a 3000-character
path, as one element and as 1500 elements, and a 3000-character argument,
through `echo`, `ls`, `cd` and `bind`): before, each call failed with
`string too long`; after, each path resolves and answers `namec: does not
exist` (the boot filesystem is read-only, so nothing long exists to find)
and the argument comes back whole (`LONGARG 3000`). Host tests pin the copy
to `PATHMAX`, the exact fit of the initial stack, a seed that runs through
four pages with a 5000-byte first argument, and the name.

Limits that are not the kernel's, recorded for the owner:

- The userland shell builds a command name in a 1024-byte buffer and
  reports no reason when a name does not fit; a script cannot exec a
  longer path by name.
- aarch64's exec still takes 32 arguments of 128 bytes and seeds one page:
  its stack layout differs (one seed page, the Tos in the page below), so
  the multi-page seed was not mirrored. Its host tests and a cross build
  pass with the path bound.

## 9. Storage: what the target requires of Phase 13 (planning; nothing built)

The third target is hundreds of terabytes of storage.  There is no block
device driver and no storage engine yet (S1; ROADMAP Phase 13), so this is
what that phase has to carry, not a change made now.

Holds today, and must keep holding end to end:

- 64-bit block numbers, offsets and sizes wherever a file's bytes are
  named.  The wire carries offsets and cursors as `u64` and Qid paths as
  `u64` (S2), and `Chan.offset` is `u64`.
- `iounit` is `u32` (S2): a bound per message, not per file.

Does not hold, and belongs to the engine:

- **The page cache's key is `(image_id: u32, page_idx: u32)`**
  (`port/src/pagecache.rs`): 16 TiB of 4 KiB pages per image (S3).  The
  page index wants `u64`, in the key and in the TPHT's packed word; a
  contained change, and the first to make.
- **The page cache is sized for the boot image, not the machine**: 1024
  TPHT home blocks (`PAGE_CACHE_BLOCKS`, `x86_64/src/pagealloc.rs`), for
  the image's text pages.  It wants a size set at boot from the machine's
  RAM, an absolute one as the frame pools' targets now are.
- **Element names on disk**: 9P strings and directory entries carry a
  `u16` length, and the kernel takes paths to `PATHMAX` (16 KiB, section
  8); the on-disk format must store a name at least that long (P5).
- The boot RAM-disk filesystem's `u32` fields (S4) are boot-only; a
  disk-backed root does not go through them.

What a large machine adds (from section 1):

- **Device memory above 4 GiB (M8).**  On this QEMU venue the whole PCI
  hole stays below 4 GiB at every RAM size measured, so nothing exercised
  it and `kaddr` still panics there.  A real server's firmware may place
  NVMe BARs high, and the storage phase then needs a mapping facility for
  device memory; the direct map's table helpers (`x86_64/src/directmap.rs`)
  are the place to build it.
- **64-bit DMA (M12)**: there is no IOMMU, and whether every DMA path is
  64-bit was not checked.
- **Booting from disk (M13)**: today's boot is BIOS + GRUB from an ISO;
  large machines boot via UEFI, untested.
- The frame number is `u32` (M11): 16 TiB of RAM; not a storage bound, but
  the page cache's backing on a machine that large.

Verification to plan with the engine: a file past 4 GiB read and written
back whole; a path and an element name beyond 2048 characters created,
found and removed on a writable filesystem (none exists today: the boot
filesystem is read-only, which is why the paths leg of section 8 stops at
a lookup); a directory with more entries than fit one block.
