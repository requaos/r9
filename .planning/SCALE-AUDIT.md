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
