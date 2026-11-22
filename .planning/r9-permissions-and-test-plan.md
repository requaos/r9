# Plan r9 Permissions, Cluster Topology & Automated Test Specification
## Automated Verification Harness for Fossil, Venti, 9P2000.aead, and Multi-Domain Security Using `minimega`

---

### 1. Executive Overview & Testbed Architecture

This specification formalizes the **user permissions hierarchy, distributed storage topology, and automated verification suite** for **r9**. To rigorously evaluate access controls, group dynamics, cache coherence, and cryptographic memory isolation across a multi-node cluster, we leverage **`minimega`** (Sandia National Laboratories).

`minimega` is an open-source distributed virtual machine management engine built specifically for large-scale QEMU/KVM network emulation. It allows us to:
1. **Define Declarative Node Templates (`vm config`):** Provision memory, vCPUs, VirtIO devices, and disk overlays cleanly for heterogeneous node roles.
2. **Simulate Isolated Virtual Fabrics (`tap` / `mesh`):** Emulate isolated VLANs for storage backend traffic, 9P grid communications, and interactive terminal display streams.
3. **Automate Cluster-Wide Test Execution (`vm cc`):** Coordinate headless guest commands, file injection, and distributed assertion gathering without manual operator intervention.

```
==================================================================================================
                               r9 CLUSTER TESTBED TOPOLOGY (minimega)
==================================================================================================

                          [Terminal 01: term01]          [Terminal 02: term02]
                          User: larry (rio / chat)       User: moe (rio / chat)
                          VirtIO-GPU (#G Venus)          VirtIO-GPU (#G Venus)
                                    │                              │
  VLAN 30 (Terminal Fabric)         └──────────────┬───────────────┘
  ─────────────────────────────────────────────────┼──────────────────────────────────────────────
                                                   │
     [cpu01]     [cpu02]     [cpu03]     [cpu04]   │ [cpu05]     [cpu06]
     larry       curly       moe         bob       │ eve         alice (arch)
     (compute)   (compute)   (compute)   (auditor) │ (adversary) (federated)
        │           │           │           │      │    │           │
        └───────────┴───────────┼───────────┴──────┴────┴───────────┘
                                │ (9P2000.aead: QTLEASED, Tcompound, ExtMask)
  VLAN 20 (Grid Fabric)         │
  ──────────────────────────────┼─────────────────────────────────────────────────────────────────
                                ▼
                   ┌──────────────────────────────┐
                   │    Fossil File Server 01     │
                   │    Hostname: fossil01        │
                   │    • Centralized /adm/users  │
                   │    • Live Buffer Cache       │
                   │    • GFS Snapshot Scheduler  │
                   └──────────────┬───────────────┘
                                  │ (Venti Block Protocol / 9P2000.aead: Zstd Level 3)
  VLAN 10 (Storage Fabric)        │
  ────────────────────────────────┼───────────────────────────────────────────────────────────────
             ┌────────────────────┼────────────────────┐
             ▼                    ▼                    ▼
     [Venti Node 01]      [Venti Node 02]      [Venti Node 03]
     Hostname: venti01    Hostname: venti02    Hostname: venti03
     BLAKE3 CAS Arenas    BLAKE3 CAS Arenas    BLAKE3 CAS Arenas
     Chained-TPHT Index   Chained-TPHT Index   Chained-TPHT Index
     64KB Super-Blocks    64KB Super-Blocks    64KB Super-Blocks
     1KB Tag Slabs        1KB Tag Slabs        1KB Tag Slabs
```

---

### 2. Heterogeneous Node Roles & minimega Configuration Templates

The testbed defines four discrete machine roles. Because r9 boots Harvey userland on `amd64` using QEMU `q35`, all templates configure modern VirtIO devices and hardware-accelerated CPU features.

#### 2.1 Machine Role Definitions
* **1x Central Fossil File Server (`fossil01`):** Serves as the authoritative mutable file system for `/`. Manages `/adm/users`, tracks active working sets, handles `QTLEASED` distributed lease revocations, and commits GFS snapshots.
* **3x Venti CAS Storage Nodes (`venti01`, `venti02`, `venti03`):** Provide the append-only, content-addressed block tier. Store 64KB Data Super-Blocks with adjacent 1KB Out-of-Band Tag Slabs, indexed by L3-resident Chained-TPHT head arrays.
* **6x Stateless Compute Nodes (`cpu01` to `cpu06`):** Diskless compute instances that mount Fossil at `/` via `9P2000.aead`. Run user applications (`chat-rs`, compilers, pipelines) and exercise local cache leases and process cryptographic isolation.
* **2x Interactive Terminal Nodes (`term01`, `term02`):** Run `rio`, `libdraw`, and `chat-rs` over VirtIO-GPU (#G) Venus with hardware blitting. Used for GUI multi-domain visibility tests.

#### 2.2 Declarative minimega Configuration Script (`cluster_topology.mm`)

Save the following configuration script to `/opt/r9/minimega/cluster_topology.mm`:

```text
# minimega v2.x cluster deployment script for r9
# Target: 1x Fossil, 3x Venti, 6x CPU, 2x Terminal

# -----------------------------------------------------------------------------
# 1. Virtual Switch & VLAN Partitioning
# -----------------------------------------------------------------------------
# VLAN 10: Dedicated Storage Fabric (Fossil <-> Venti CAS traffic)
# VLAN 20: 9P2000.aead Grid Fabric (CPU <-> Fossil, Peer-to-Peer)
# VLAN 30: Terminal Display Fabric (Terminal <-> CPU / Gateway)

clear vm config

# -----------------------------------------------------------------------------
# 2. Template: Venti CAS Storage Node (venti-tmpl)
# -----------------------------------------------------------------------------
vm config memory 2048
vm config vcpus 2
vm config disk /var/lib/minimega/images/r9-venti-base.qc2
vm config net 10
vm config qemu-append "-cpu host -machine q35,accel=kvm -device virtio-net-pci,netdev=net0 -drive file=/var/lib/minimega/images/venti-arena.raw,if=none,id=arenadisk,format=raw -device nvme,drive=arenadisk,serial=VENTIARENA"
vm config snapshot true

# Launch 3 Venti instances
vm launch kvm venti01
vm launch kvm venti02
vm launch kvm venti03

# -----------------------------------------------------------------------------
# 3. Template: Fossil File Server (fossil-tmpl)
# -----------------------------------------------------------------------------
clear vm config
vm config memory 4096
vm config vcpus 4
vm config disk /var/lib/minimega/images/r9-fossil-base.qc2
# Dual-homed: NIC 0 on Storage Fabric (VLAN 10), NIC 1 on Grid Fabric (VLAN 20)
vm config net 10 20
vm config qemu-append "-cpu host -machine q35,accel=kvm -device virtio-net-pci,netdev=net0 -device virtio-net-pci,netdev=net1 -drive file=/var/lib/minimega/images/fossil-cache.raw,if=none,id=fossilcache,format=raw -device virtio-blk-pci,drive=fossilcache"
vm config snapshot true

vm launch kvm fossil01

# -----------------------------------------------------------------------------
# 4. Template: Stateless CPU Compute Nodes (cpu-tmpl)
# -----------------------------------------------------------------------------
clear vm config
vm config memory 2048
vm config vcpus 2
# Stateless netboot / ramdisk kernel: mounts Fossil over 9P2000.aead
vm config kernel /var/lib/minimega/images/r9-9load
vm config initrd /var/lib/minimega/images/r9-bootproto.cpio
vm config net 20
vm config qemu-append "-cpu host -machine q35,accel=kvm -device virtio-net-pci,netdev=net0"
vm config snapshot false

# Launch 6 compute instances
vm launch kvm cpu01
vm launch kvm cpu02
vm launch kvm cpu03
vm launch kvm cpu04
vm launch kvm cpu05
vm launch kvm cpu06

# -----------------------------------------------------------------------------
# 5. Template: Interactive Terminal Nodes (term-tmpl)
# -----------------------------------------------------------------------------
clear vm config
vm config memory 2048
vm config vcpus 2
vm config disk /var/lib/minimega/images/r9-term-base.qc2
# Dual-homed: NIC 0 on Grid Fabric (VLAN 20), NIC 1 on Terminal Fabric (VLAN 30)
vm config net 20 30
# VirtIO-GPU Venus acceleration enabled
vm config qemu-append "-cpu host -machine q35,accel=kvm -device virtio-vga-gl -display egl-headless -device virtio-net-pci,netdev=net0 -device virtio-net-pci,netdev=net1"
vm config snapshot true

vm launch kvm term01
vm launch kvm term02

# -----------------------------------------------------------------------------
# 6. Start All Instances
# -----------------------------------------------------------------------------
vm start all
```

---

### 3. Formalized User Registry & Group Hierarchy

In Plan 9, user authentication and group permissions are resolved centrally via Fossil's `/adm/users`. Groups are modeled as users that contain member names, governed by a designated group leader.

#### 3.1 Persona Definitions
| User / UID | Persona Name | Assigned Node | Default Group | Persona Role in Test Suite |
| :--- | :--- | :--- | :--- | :--- |
| **1001** | `larry` | `cpu01`, `term01` | `dev` | **Primary Collaborator A:** Creates private pair-shares with Curly; initiates messaging. |
| **1002** | `curly` | `cpu02` | `dev` | **Targeted Peer B:** Recipient of Larry's pairwise shares; collaborator on team code. |
| **1003** | `moe` | `cpu03`, `term02` | `stooges` | **Team Lead C:** Creates broad team shares for A and B; group leader of `stooges`. |
| **1004** | `bob` | `cpu04` | `audit` | **Restricted Auditor:** Has read-only rights to system logs; excluded from private dev trees. |
| **1005** | `eve` | `cpu05` | `none` | **Adversary / Probe:** Probes unauthorized reads, attempts note spoofing and memory snooping. |
| **2001** | `alice` | `cpu06` | `arch-team` | **Federated Peer:** Authenticates via `archive.r9` Factotum tickets to test cross-domain 9P. |
| **0** | `glenda` | `fossil01` | `sys` | **System Administrator:** Provisions `/adm/users` and root infrastructure. |
| **9999** | `none` | (Global) | `none` | **Anonymous World:** Unauthenticated fallback for public reads. |

#### 3.2 Authoritative Fossil `/adm/users` File

Save to Fossil's root administrative directory `/adm/users`:

```text
# /adm/users on fossil01 (lab.r9)
# Format: uid:name:leader:members

0:adm:adm:glenda,sys
1:sys:sys:glenda
9999:none:none:

# Individual User Identifiers (self-led)
1001:larry:larry:
1002:curly:curly:
1003:moe:moe:
1004:bob:bob:
1005:eve:eve:
2001:alice:alice:

# Functional & Collaborative Security Groups
1010:stooges:moe:larry,curly,moe
1020:pair_ab:larry:larry,curly
1030:dev:glenda:larry,curly,moe,bob
1040:audit:glenda:bob,glenda
```

---

### 4. Comprehensive End-to-End Test Matrix

The automated verification suite exercises five distinct architectural layers across the `minimega` cluster:

```
[Layer 5: Messaging & Aliases]     ── upas/send, chat-rs thread filtering, CC visibility
[Layer 4: Distributed 9P2000.aead] ── QTLEASED coherence, StatelessCap, ExtMask
[Layer 3: Cross-Domain Federation] ── Factotum multi-domain tickets, lab.r9 vs archive.r9
[Layer 2: Fossil VFS Permissions]  ── 9P mode bits (owner/group/other), leader mutations
[Layer 1: Cryptographic Memory]    ── rfork(RFCKEY|RFROBASE), PKEYs, ShadowFrame
```

---

#### Test Suite 1: Discretionary File Sharing (Fossil VFS)

##### Test 1.1: Asymmetric Pairwise Sharing (A shares with B, strictly excludes C)
* **Goal:** Verify that User A (`larry`) can share read/write files with User B (`curly`) while User C (`moe`) and Adversary (`eve`) are completely denied access.
* **Pre-Conditions:** `fossil01` is active; `/adm/users` contains `pair_ab:larry:larry,curly`.
* **Execution:**
  1. On `cpu01`, `larry` runs:
     ```rc
     mkdir -p /usr/larry/shared_ab
     echo "Confidential_AB_Payload_9481" > /usr/larry/shared_ab/spec.txt
     chgrp pair_ab /usr/larry/shared_ab
     chgrp pair_ab /usr/larry/shared_ab/spec.txt
     chmod 750 /usr/larry/shared_ab          # drwxr-x---
     chmod 640 /usr/larry/shared_ab/spec.txt # -rw-r-----
     ```
  2. On `cpu02`, `curly` executes `cat /usr/larry/shared_ab/spec.txt`.
  3. On `cpu03`, `moe` executes `ls /usr/larry/shared_ab` and `cat /usr/larry/shared_ab/spec.txt`.
  4. On `cpu05`, `eve` executes `cat /usr/larry/shared_ab/spec.txt`.
* **Assertions:**
  * `curly` read succeeds and returns exact string `Confidential_AB_Payload_9481`.
  * `curly` write (`echo "x" >> /usr/larry/shared_ab/spec.txt`) fails with `permission denied` (mode is `640`).
  * `moe` directory listing and file reads fail with `permission denied` (`Eperm`).
  * `eve` read fails with `permission denied` (`Eperm`).

##### Test 1.2: Symmetric Team Collaboration (C shares with both A and B)
* **Goal:** Verify that Team Lead C (`moe`) can provision a shared project directory with full read/write collaboration for `larry` and `curly`.
* **Execution:**
  1. On `cpu03`, `moe` runs:
     ```rc
     mkdir -p /usr/moe/collab
     echo "Project_Milestone_Alpha" > /usr/moe/collab/plan.txt
     chgrp stooges /usr/moe/collab
     chgrp stooges /usr/moe/collab/plan.txt
     chmod 770 /usr/moe/collab          # drwxrwx---
     chmod 660 /usr/moe/collab/plan.txt # -rw-rw----
     ```
  2. On `cpu01`, `larry` appends: `echo "Larry_Signed" >> /usr/moe/collab/plan.txt`.
  3. On `cpu02`, `curly` appends: `echo "Curly_Signed" >> /usr/moe/collab/plan.txt`.
  4. On `cpu04`, `bob` attempts to read `/usr/moe/collab/plan.txt`.
* **Assertions:**
  * Both `larry` and `curly` writes succeed.
  * Reading the file displays all three signatures in chronological order.
  * `bob` read is rejected with `Eperm`.

##### Test 1.3: Group Leadership Authority Enforcement
* **Goal:** Verify that only group leaders can add or remove group members in `/adm/users`.
* **Execution:**
  1. `larry` (leader of `pair_ab`) issues `fossil/conf` command to add `bob` to `pair_ab`.
  2. `curly` (member, non-leader) attempts to remove `larry` from `pair_ab`.
  3. `curly` attempts to add `eve` to `stooges` (led by `moe`).
* **Assertions:**
  * `larry` mutation succeeds.
  * `curly` mutations are rejected with `not group leader`.

---

#### Test Suite 2: Distributed `9P2000.aead` Leases & Cache Coherence

##### Test 2.1: Concurrent Read Leases on Multi-Core Compute Nodes
* **Goal:** Verify that when `cpu01`, `cpu02`, and `cpu03` read the same file, Fossil grants `QTLEASED` read leases and registers them in an in-memory `ClientSet`.
* **Execution:**
  1. All three nodes read `/usr/moe/collab/plan.txt`.
  2. Observe `fossil01` kernel logs via `minimega` serial console.
* **Assertions:**
  * `fossil01` registers node IDs in `LeaseRecord.readers`.
  * `ClientSet` remains in `Inline { count: 3, ids: [...] }` without allocating heap memory.
  * Subsequent reads on `cpu01`–`cpu03` are serviced entirely from local DRAM cache with 0 network packets.

##### Test 2.2: Asynchronous Conflicting Write Invalidation (`Tlease_revoke`)
* **Goal:** Verify cache consistency when an exclusive write occurs on a leased file.
* **Execution:**
  1. On `cpu02`, `curly` opens `/usr/moe/collab/plan.txt` for writing (`O_WRONLY`).
  2. `fossil01` upgrades Curly’s lease to `ExclusiveWrite` and emits `Tlease_revoke` packets to `cpu01` and `cpu03`.
  3. On `cpu01`, `larry` reads `/usr/moe/collab/plan.txt`.
* **Assertions:**
  * `cpu01` intercepts `Tlease_revoke` and clears the corresponding block in its `valid_blocks` Roaring Bitmap.
  * `larry`’s read triggers an on-demand `9P2000.aead` fetch, immediately reflecting Curly’s modifications.

##### Test 2.3: Wire Anti-Splicing & Replay Defense
* **Goal:** Validate that packets modified or replayed across channels fail cryptographic verification.
* **Execution:**
  1. Using an in-line packet tap on VLAN 20, duplicate a valid `Twrite` packet from `cpu01`.
  2. Replay the captured packet with a stale sequence nonce.
  3. Inject a valid payload from Channel $X$ into Channel $Y$ by modifying the FID in the cleartext header.
* **Assertions:**
  * The replayed packet fails monotonic nonce check (`SeqNonce <= last_seen`) and is dropped.
  * The spliced packet fails Keyed BLAKE3 AAD MAC verification because the FID is bound in the MAC.

---

#### Test Suite 3: Multi-Domain Namespace Federation (`lab.r9` vs `archive.r9`)

##### Test 3.1: Factotum Multi-Key Negotiation
* **Goal:** Validate that a user holding credentials for two independent authentication servers can mount both filesystems into disjoint namespaces simultaneously.
* **Execution:**
  1. On `cpu01`, `larry` loads `factotum` with two keys:
     ```text
     key proto=p9sk1 dom=lab.r9 user=larry !password=secret_lab
     key proto=p9sk1 dom=archive.r9 user=larry !password=secret_arch
     ```
  2. Mount local domain: `mount -c /net/tcp!fossil01!564 /n/lab`.
  3. Mount remote domain: `mount -c /net/tcp!venti03!564 /n/archive`.
* **Assertions:**
  * Both mounts establish separate `9P2000.aead` links with distinct ephemeral keys ($K_{\text{transient\_A}}$ and $K_{\text{transient\_B}}$).
  * `larry` accesses `/n/lab` and `/n/archive` concurrently without credential collisions.

##### Test 3.2: Unregistered Cross-Domain User Attenuation
* **Goal:** Verify that a remote user without a local `/adm/users` record is attenuated to `none`.
* **Execution:**
  1. `alice` (from `archive.r9`) mounts `fossil01` on `lab.r9`.
  2. `alice` attempts to read `/usr/larry/shared_ab/spec.txt`.
* **Assertions:**
  * `fossil01` authenticates her ticket for `archive.r9` but finds no `alice` in local `/adm/users`.
  * Her channel maps to UID `9999` (`none`).
  * Access to private files returns `Eperm`.

---

#### Test Suite 4: Kernel Memory Micro-Isolation (`rfork` & PKEYs)

##### Test 4.1: Sibling Memory Cryptographic Partitioning (`RFCKEY`)
* **Goal:** Verify that sibling child processes cannot read or infer each other's private heap frames even under the same UID.
* **Execution:**
  1. A harness binary spawns two children with `rfork(RFPROC | RFMEM | RFCKEY)`.
  2. Child 1 receives $K_{RW\_B1}$; Child 2 derives $K_{RW\_B2}$.
  3. Child 2 attempts to dereference Child 1's private heap address.
* **Assertions:**
  * Hardware MMU traps the access via independent `CR3` page tables.
  * If the physical frame is read directly, ChaCha20 decryption under $K_{RW\_B2}$ yields random bytes, failing the BLAKE3 MAC check.
  * Kernel terminates Child 2 with a `sys: crypto integrity violation` note.

##### Test 4.2: Hardware Read-Only Base Segment Enforcement (`RFROBASE`)
* **Goal:** Verify that a process cannot mutate its own code or shared read-only base.
* **Execution:**
  1. Process invokes `rfork(RFROBASE)`.
  2. Process writes to an address inside `Block A` (`SG_TEXT`).
* **Assertions:**
  * Hardware PKEY 1 / `WRPKRU` traps the write in ~20 cycles without touching DRAM.

---

#### Test Suite 5: Mail & Chat Thread Visibility (`chat-rs`)

##### Test 5.1: Direct Message Isolation
* **Execution:** On `term01`, `larry` sends a direct message to `curly` (`upas/send curly`).
* **Assertions:**
  * Message appears in `curly`'s transcript under `[USERS] -> larry`.
  * Message is completely absent from `moe`'s, `bob`'s, and `eve`'s mail spools.

##### Test 5.2: Dual-Visibility Engine Verification
* **Execution:** `moe` sends a message to `larry` with a CC to `stooges` (`upas/send larry stooges`).
* **Assertions:**
  * **Larry's View:** Message appears in `[USERS] -> moe` (badged `(to you, cc: stooges)`) AND in `[PERSONAL GROUPS] -> stooges`.
  * **Curly's View:** Message appears in `[PERSONAL GROUPS] -> stooges` (badged `moe -> stooges (to: larry)`).
  * **Bob / Eve:** Message is completely absent.

---

### 5. In-Guest Automated Test Script (`tests/perm_test.rc`)

This executable Plan 9 `rc` script executes inside the guest instances to conduct automated assertions:

```rc
#!/bin/rc
# /sys/src/tests/perm_test.rc - Automated r9 Permissions Verification Suite

fn assert_ok {
    desc=$1
    shift
    if(! $*){
        echo 'FAIL:' $desc
        exit 1
    }
    echo 'PASS:' $desc
}

fn assert_fail {
    desc=$1
    shift
    if($* >/dev/null >[2]/dev/null){
        echo 'FAIL (unexpected success):' $desc
        exit 1
    }
    echo 'PASS (correctly denied):' $desc
}

echo '=== Starting r9 Permissions & Security Suite ==='

# 1. Asymmetric Pairwise Sharing Tests
echo '--- Test Suite 1: Discretionary Sharing ---'
assert_ok 'Larry creates shared directory'     mkdir -p /usr/larry/shared_ab

assert_ok 'Larry creates secret payload'     echo 'Payload_AB_7721' > /usr/larry/shared_ab/spec.txt

assert_ok 'Larry sets pair_ab group'     chgrp pair_ab /usr/larry/shared_ab && chgrp pair_ab /usr/larry/shared_ab/spec.txt

assert_ok 'Larry sets restrictive permissions'     chmod 750 /usr/larry/shared_ab && chmod 640 /usr/larry/shared_ab/spec.txt

assert_ok 'Curly reads pair_ab file'     rfork u; user=curly; cat /usr/larry/shared_ab/spec.txt

assert_fail 'Curly cannot write to 640 file'     rfork u; user=curly; echo 'bad' >> /usr/larry/shared_ab/spec.txt

assert_fail 'Moe cannot list pair_ab directory'     rfork u; user=moe; ls /usr/larry/shared_ab

assert_fail 'Moe cannot read pair_ab file'     rfork u; user=moe; cat /usr/larry/shared_ab/spec.txt

assert_fail 'Eve cannot read pair_ab file'     rfork u; user=eve; cat /usr/larry/shared_ab/spec.txt

# 2. Team Collaboration Tests
echo '--- Test Suite 2: Team Collaboration ---'
assert_ok 'Moe creates collab workspace'     mkdir -p /usr/moe/collab && echo 'Plan_Alpha' > /usr/moe/collab/plan.txt

assert_ok 'Moe assigns stooges group'     chgrp stooges /usr/moe/collab && chgrp stooges /usr/moe/collab/plan.txt

assert_ok 'Moe enables team write'     chmod 770 /usr/moe/collab && chmod 660 /usr/moe/collab/plan.txt

assert_ok 'Larry writes to team plan'     rfork u; user=larry; echo 'Larry_Review' >> /usr/moe/collab/plan.txt

assert_ok 'Curly writes to team plan'     rfork u; user=curly; echo 'Curly_Review' >> /usr/moe/collab/plan.txt

assert_fail 'Bob cannot read team plan'     rfork u; user=bob; cat /usr/moe/collab/plan.txt

assert_fail 'Eve cannot write to team plan'     rfork u; user=eve; echo 'Malicious' >> /usr/moe/collab/plan.txt

echo '=== All In-Guest Assertion Suites Passed Successfully ==='
```

---

### 6. Cluster Orchestration & Test Execution Driver

To execute this entire pipeline headlessly from the host, the following driver shell script coordinates `minimega`, injects guest scripts, and gathers assertion results:

```bash
#!/usr/bin/env bash
# /opt/r9/scripts/run_cluster_tests.sh
set -euo pipefail

echo "[*] Deploying 12-node r9 cluster via minimega..."
minimega -e "read /opt/r9/minimega/cluster_topology.mm"

echo "[*] Waiting for nodes to reach interactive state..."
sleep 15

echo "[*] Verifying Venti CAS cluster connectivity..."
minimega -e "vm cc fossil01 exec venti/ping venti01"
minimega -e "vm cc fossil01 exec venti/ping venti02"
minimega -e "vm cc fossil01 exec venti/ping venti03"

echo "[*] Injecting /adm/users into Fossil server..."
minimega -e "vm cc fossil01 copy /opt/r9/conf/users /adm/users"

echo "[*] Triggering automated permissions test suite on cpu01..."
minimega -e "vm cc cpu01 exec rc -c /sys/src/tests/perm_test.rc"

echo "[*] Triggering cross-domain federation suite on cpu06..."
minimega -e "vm cc cpu06 exec rc -c /sys/src/tests/federation_test.rc"

echo "[*] All cluster test runs completed successfully!"
```
