# r9 Linux witness return (Plan 01-06)

- date: 2026-11-23T02:31:44Z
- git HEAD: e53230ff6bbed847e6460b8fd745762aafbd9429
- staging root: /home/req/r9-spike-work/witness-bundle
- kernel sha256: ecdcdaddea8d3759d74303b869ddfae8efeba1d4d160382577cfc52b9a32c45b (2030226 bytes; matches s5/evidence/kernel-provenance.txt)
- probe sha256: db94057f04c751c59a648de5c8505f452bb225b6a276714e7ee5544396a5dcd5
- probe recorded sha256: db94057f04c751c59a648de5c8505f452bb225b6a276714e7ee5544396a5dcd5
- probe match: yes
- run-all exit: 1

## out/env.txt

~~~
# r9 Linux witness preflight
date=2026-11-23T02:14:55Z
user=req uid=1000
bundle_root=/home/req/r9-spike-work/witness-bundle
work_dir=/home/req/r9-spike-work
uname=Linux teeniebox 7.4.0-rc3 #1-NixOS SMP PREEMPT_DYNAMIC @315532800 x86_64 GNU/Linux
nproc=16
               total        used        free      shared  buff/cache   available
Mem:            30Gi       1.8Gi        24Gi       284Mi       4.9Gi        29Gi
Swap:          7.5Gi          0B       7.5Gi
--- free disk under /home/req/r9-spike-work ---
Filesystem        Size  Used Avail Use% Mounted on
/dev/mapper/root  946G  620G  324G  66% /
--- nix ---
nix (Nix) 2.35.6
experimental-features = dynamic-derivations fetch-closure fetch-tree flakes nix-command
sandbox = true
--- /dev/kvm ---
DEV_KVM=writable
crw-rw-rw- 1 root kvm 10, 232 Nov 22 21:14 /dev/kvm
--- sysctls (unprivileged userns is what Harvey's x86_64 venue needs) ---
user.max_user_namespaces=126494
kernel.unprivileged_userns_clone=unavailable
kernel.apparmor_restrict_unprivileged_userns=unavailable

~~~

## out/summary.tsv

~~~
s3	1	857
s6	0	152

~~~

## out/survivors.txt

~~~
# surviving qemu/Xvfb/drawterm processes owned by req
# at 2026-11-23T02:31:44Z
88800 /nix/store/y8h5ana2drnzs2fdx6fxgr74bwmzi8y9-drawterm-19.06/bin/drawterm -s  -O cpu=127.0.0.1,auth=tcp!127.0.0.1!5670,user=glenda

~~~

## out/secret-scan.txt

~~~
# credential scan of out/ at 2026-11-23T02:31:44Z
scanned for the DEMO password value from the scratch tree's auth-creds.txt
clean: no auth-creds artifact and no DEMO password value in out/

~~~

## last 40 lines of each out/<step>/*.log

### out/s3/control-noG.log

~~~
/srv/boot
/srv/factotum
process 74 exited: ""
churngate: heap exit 4693504 33554432
% ls /srv
/srv/boot
/srv/factotum
process 75 exited: ""
churngate: heap exit 4693504 33554432
% ls /srv
/srv/boot
/srv/factotum
process 76 exited: ""
churngate: heap exit 4693504 33554432
% ls /srv
/srv/boot
/srv/factotum
process 77 exited: ""
churngate: heap exit 4693504 33554432
% ls /srv
/srv/boot
/srv/factotum
process 78 exited: ""
churngate: heap exit 4693504 33554432
% ls /srv
/srv/boot
/srv/factotum
process 79 exited: ""
churngate: heap exit 4693504 33554432
% ls /srv
/srv/boot
/srv/factotum
process 80 exited: ""
churngate: heap exit 4693504 33554432
% S3-CONTROL-NO-RIO
rio-gate: post-pass ok — zero cpurc: FAIL in the capture
rio-gate: post-pass ok — zero kernel panic markers (drawterm's own sysfatal exit pair excluded)
rio-gate: post-pass ok — the per-bake DEMO password redacted from the retained capture (0 occurrence(s) mapped)
rio-gate: ALL STAGES PASS — cpu boot to cpurc: DONE, -G control session, serial /srv shows no rio entry (S3-CONTROL-NO-RIO)
== S3 control-noG gate PASS — capture retained at /home/req/r9-spike-work/witness-bundle/out/s3/control-noG/rio-gate-control-noG.log

~~~

### out/s3/determinism.log

~~~
building '/nix/store/bn3ms6pwv7sgisynphaylf46z63svn6y-harness-0.1.0.drv'...
building '/nix/store/bsg061wyparnv6wcq1vy4jwa5rq9dvby-nix-shell-env.drv'...
harvey devshell: HARVEY=/home/req/r9-spike-work/harvey PLAN9=/nix/store/dzvnbi2n1x9jkmzvmwxdly607bwaxsbv-plan9port-0-unstable-2025-11-10/plan9
  host bin: /home/req/r9-spike-work/harvey/.build/hostbin-linux-x86_64/bin
  harness: /nix/store/xjlym8x1fqqp6n8ai6nwn5q459sj95pz-harness-0.1.0/bin/harness
apps: nix run .#setup -> .#toolchain -> .#build | .#iso
(. #build uses sudo unshare; run from the repo root)
== determinism: scratch=/home/req/r9-spike-work/harvey KERNDATE=1795382101
== bake 1 hashes -> /home/req/r9-spike-work/witness-bundle/out/s3/determinism/bake1.sha256
== bake 2: sh build/x86_64/mkimage.sh (log: /home/req/r9-spike-work/witness-bundle/out/s3/determinism/determinism-bake2.log)
S3-ERROR: bake2-failed — the second bake did not complete; tail of /home/req/r9-spike-work/witness-bundle/out/s3/determinism/determinism-bake2.log follows
services manifest: /amd64/bin/test staged
services manifest: /amd64/bin/bind staged
services manifest: /amd64/bin/hsvctcpecho staged
services manifest: /amd64/bin/hmkdir staged
services manifest: /rc/lib/rcmain staged
services manifest: /rc/bin/termrc staged
services manifest: /lib/namespace staged
services manifest: /adm/timezone/local staged
services manifest: /usr/glenda/lib/profile staged
services manifest: /lib/netaddr.rc staged
services manifest: /lib/netip-loopback.rc staged
services manifest: /lib/netip-static.rc staged
services manifest: /lib/netip-dhcp.rc staged
services manifest: /lib/netip-tcp.rc staged
services manifest: /lib/services-sweep.rc staged
services manifest: 127 h* sweep binaries staged (tier-1 helpers + the tier-2 set computed from tiers/tier2-services.txt: 90 tier-2 members)
== PASS: baked /home/req/r9-spike-work/harvey/build/x86_64/out/bootfs-services.paq (26671161 bytes; 127 h* binaries; header 529ab12b trailer 6b46e688; KERNDATE 1795382101)
== PASS: baked /home/req/r9-spike-work/harvey/build/x86_64/out/bootfs-churn.paq (492105 bytes; chain + test/dc + churncats + /lib/churn-cats.rc; header 529ab12b trailer 6b46e688; KERNDATE 1795382101)
== PASS: baked /home/req/r9-spike-work/harvey/build/x86_64/out/bootfs-substrate.paq (43674 bytes; skeleton + randread as /amd64/init; header 529ab12b trailer 6b46e688; KERNDATE 1795382101)
error: auth bake: sweep subject hsecstored missing from the computed auth-suite set (the resolver lost it)

~~~

### out/s3/rio.log

~~~
process 46 exited: ""
churngate: heap exit 4149248 33554432
cpurc: DONErio-gate: stage ok — ladder-done
spawn bash -c exec -a r9s3riogate-dt-63949-1795400202 /nix/store/y8h5ana2drnzs2fdx6fxgr74bwmzi8y9-drawterm-19.06/bin/drawterm -s '' -O cpu=127.0.0.1,auth=tcp!127.0.0.1!5670,user=glenda
rio-gate: spawned drawterm on a pty (mode rio, argv0 tag r9s3riogate-dt-63949-1795400202, user glenda)

process 47 exited: ""
churngate: heap exit 4149248 33554432
process 5 exited: ""
churngate: heap exit 4149248 33554432
process 4 exited: ""
churngate: heap exit 4149248 33554432

init: starting /bin/rc
% devmnt: server error: file does not exist
 Feb  7 01:28:16 tcp call for tcp17010 on chan /net/tcp/2 (10.0.2.2)
devmnt: server error: file does not exist
devmnt: server error: file does not exist
 Feb  7 01:28:16 tcp call for tcp17010 on chan /net/tcp/3 (10.0.2.2)
devmnt: server error: file does not exist
devmnt: server error: not found
devmnt: server error: not found
 Feb  7 01:28:16 tr-ok glenda@glenda(unknown) -> glenda@bootes
process 51 exited: ""
churngate: heap exit 4569344 33554432
rio-gate: TIMEOUT at feed stage: dt-password
S3-ERROR: the S3 rio gate FAILED (rc=1 — the driver named the failure on stderr; capture: /home/req/r9-spike-work/witness-bundle/out/s3/rio/rio-gate-rio.log)
The XKEYBOARD keymap compiler (xkbcomp) reports:
> Warning:          Multiple symbols for level 1/group 1 on key <FK23>
>                   Using F23, ignoring XF86TouchpadOff
> Warning:          Symbol map for key <FK23> redefined
>                   Using last definition for conflicting fields
> Warning:          Symbol map for key <FK24> redefined
>                   Using last definition for conflicting fields
> Warning:          Could not resolve keysym XF86ElectronicPrivacyScreenOn
> Warning:          Could not resolve keysym XF86ElectronicPrivacyScreenOff
> Warning:          Could not resolve keysym XF86ActionOnSelection
> Warning:          Could not resolve keysym XF86ContextualInsert
> Warning:          Could not resolve keysym XF86ContextualQuery
Errors from xkbcomp are not fatal to the X server

~~~

### out/s6/bake-control.log

~~~
cpu manifest (§15): /boot/boot staged
cpu manifest (§15): /boot/paqfs staged
cpu manifest (§15): /amd64/init staged
cpu manifest (§15): /amd64/bin/rc staged
cpu manifest (§15): /amd64/bin/ls staged
cpu manifest (§15): /amd64/bin/cat staged
cpu manifest (§15): /amd64/bin/echo staged
cpu manifest (§15): /amd64/bin/ps staged
cpu manifest (§15): /amd64/bin/bind staged
cpu manifest (§15): /amd64/bin/authsrv staged
cpu manifest (§15): /amd64/bin/cpu staged
cpu manifest (§15): /amd64/bin/aux/listen staged
cpu manifest (§15): /amd64/bin/ip/ipconfig staged
cpu manifest (§15): /amd64/bin/test staged
cpu manifest (§15): /amd64/bin/rio staged
cpu manifest (§15): /amd64/bin/plumber staged
cpu manifest (§15): /amd64/bin/plumb staged
cpu manifest (§15): /amd64/bin/ramfs staged
cpu manifest (§15): /amd64/bin/syscall staged
cpu manifest (§15): /amd64/bin/awd staged
cpu manifest (§15): /rc/lib/rcmain staged
cpu manifest (§15): /rc/bin/cpurc staged
cpu manifest (§15): /rc/bin/service/tcp17010 staged
cpu manifest (§15): /bin/service/tcp17010 staged
cpu manifest (§15): /rc/bin/termrc staged
cpu manifest (§15): /lib/namespace staged
cpu manifest (§15): /lib/netaddr.rc staged
cpu manifest (§15): /lib/ndb/auth staged
cpu manifest (§15): /lib/ndb/local staged
cpu manifest (§15): /adm/timezone/local staged
cpu manifest (§15): /adm/nvram staged
cpu manifest (§15): /adm/keys staged
cpu manifest (§15): /adm/domain.manifest staged
cpu manifest (§15): /boot/authid-creds.txt staged
cpu manifest (§15): /usr/glenda/lib/profile staged
cpu manifest (§15): /usr/glenda/lib/plumbing staged
cpu manifest (§15): /sys/lib/plumb/basic staged
cpu manifest (§15): /sys/lib/plumb/fileaddr staged
cpu manifest (§15): 26 h* sweep binaries staged (§14's computed auth-suite set + the tier-1 helpers incl. hfactotum/hsed)
== PASS: baked /home/req/r9-spike-work/harvey/build/x86_64/out/bootfs-cpu.paq (22002515 bytes; chain + the auth suite + cmd/cpu + aux/listen + the real cpurc + tcp17010 + the font payload + the §15 p9sk1 domain + the 19-10 desktop suite (test/rio/plumber/plumb/ramfs/syscall/awd + glenda's home + the plumb rules); header 529ab12b trailer 6b46e688; KERNDATE 1795382101)

~~~

### out/s6/bake-flipback.log

~~~
cpu manifest (§15): /boot/boot staged
cpu manifest (§15): /boot/paqfs staged
cpu manifest (§15): /amd64/init staged
cpu manifest (§15): /amd64/bin/rc staged
cpu manifest (§15): /amd64/bin/ls staged
cpu manifest (§15): /amd64/bin/cat staged
cpu manifest (§15): /amd64/bin/echo staged
cpu manifest (§15): /amd64/bin/ps staged
cpu manifest (§15): /amd64/bin/bind staged
cpu manifest (§15): /amd64/bin/authsrv staged
cpu manifest (§15): /amd64/bin/cpu staged
cpu manifest (§15): /amd64/bin/aux/listen staged
cpu manifest (§15): /amd64/bin/ip/ipconfig staged
cpu manifest (§15): /amd64/bin/test staged
cpu manifest (§15): /amd64/bin/rio staged
cpu manifest (§15): /amd64/bin/plumber staged
cpu manifest (§15): /amd64/bin/plumb staged
cpu manifest (§15): /amd64/bin/ramfs staged
cpu manifest (§15): /amd64/bin/syscall staged
cpu manifest (§15): /amd64/bin/awd staged
cpu manifest (§15): /rc/lib/rcmain staged
cpu manifest (§15): /rc/bin/cpurc staged
cpu manifest (§15): /rc/bin/service/tcp17010 staged
cpu manifest (§15): /bin/service/tcp17010 staged
cpu manifest (§15): /rc/bin/termrc staged
cpu manifest (§15): /lib/namespace staged
cpu manifest (§15): /lib/netaddr.rc staged
cpu manifest (§15): /lib/ndb/auth staged
cpu manifest (§15): /lib/ndb/local staged
cpu manifest (§15): /adm/timezone/local staged
cpu manifest (§15): /adm/nvram staged
cpu manifest (§15): /adm/keys staged
cpu manifest (§15): /adm/domain.manifest staged
cpu manifest (§15): /boot/authid-creds.txt staged
cpu manifest (§15): /usr/glenda/lib/profile staged
cpu manifest (§15): /usr/glenda/lib/plumbing staged
cpu manifest (§15): /sys/lib/plumb/basic staged
cpu manifest (§15): /sys/lib/plumb/fileaddr staged
cpu manifest (§15): 26 h* sweep binaries staged (§14's computed auth-suite set + the tier-1 helpers incl. hfactotum/hsed)
== PASS: baked /home/req/r9-spike-work/harvey/build/x86_64/out/bootfs-cpu.paq (22002514 bytes; chain + the auth suite + cmd/cpu + aux/listen + the real cpurc + tcp17010 + the font payload + the §15 p9sk1 domain + the 19-10 desktop suite (test/rio/plumber/plumb/ramfs/syscall/awd + glenda's home + the plumb rules); header 529ab12b trailer 6b46e688; KERNDATE 1795382101)

~~~

### out/s6/bake-probe.log

~~~
cpu manifest (§15): /boot/boot staged
cpu manifest (§15): /boot/paqfs staged
cpu manifest (§15): /amd64/init staged
cpu manifest (§15): /amd64/bin/rc staged
cpu manifest (§15): /amd64/bin/ls staged
cpu manifest (§15): /amd64/bin/cat staged
cpu manifest (§15): /amd64/bin/echo staged
cpu manifest (§15): /amd64/bin/ps staged
cpu manifest (§15): /amd64/bin/bind staged
cpu manifest (§15): /amd64/bin/authsrv staged
cpu manifest (§15): /amd64/bin/cpu staged
cpu manifest (§15): /amd64/bin/aux/listen staged
cpu manifest (§15): /amd64/bin/ip/ipconfig staged
cpu manifest (§15): /amd64/bin/test staged
cpu manifest (§15): /amd64/bin/rio staged
cpu manifest (§15): /amd64/bin/plumber staged
cpu manifest (§15): /amd64/bin/plumb staged
cpu manifest (§15): /amd64/bin/ramfs staged
cpu manifest (§15): /amd64/bin/syscall staged
cpu manifest (§15): /amd64/bin/awd staged
cpu manifest (§15): /rc/lib/rcmain staged
cpu manifest (§15): /rc/bin/cpurc staged
cpu manifest (§15): /rc/bin/service/tcp17010 staged
cpu manifest (§15): /bin/service/tcp17010 staged
cpu manifest (§15): /rc/bin/termrc staged
cpu manifest (§15): /lib/namespace staged
cpu manifest (§15): /lib/netaddr.rc staged
cpu manifest (§15): /lib/ndb/auth staged
cpu manifest (§15): /lib/ndb/local staged
cpu manifest (§15): /adm/timezone/local staged
cpu manifest (§15): /adm/nvram staged
cpu manifest (§15): /adm/keys staged
cpu manifest (§15): /adm/domain.manifest staged
cpu manifest (§15): /boot/authid-creds.txt staged
cpu manifest (§15): /usr/glenda/lib/profile staged
cpu manifest (§15): /usr/glenda/lib/plumbing staged
cpu manifest (§15): /sys/lib/plumb/basic staged
cpu manifest (§15): /sys/lib/plumb/fileaddr staged
cpu manifest (§15): 26 h* sweep binaries staged (§14's computed auth-suite set + the tier-1 helpers incl. hfactotum/hsed)
== PASS: baked /home/req/r9-spike-work/harvey/build/x86_64/out/bootfs-cpu.paq (21974317 bytes; chain + the auth suite + cmd/cpu + aux/listen + the real cpurc + tcp17010 + the font payload + the §15 p9sk1 domain + the 19-10 desktop suite (test/rio/plumber/plumb/ramfs/syscall/awd + glenda's home + the plumb rules); header 529ab12b trailer 6b46e688; KERNDATE 1795382101)

~~~

### out/s6/clone.log

~~~
Cloning into '/home/req/r9-spike-work/r9'...
Updating files:   0% (1/1126)Updating files:   1% (12/1126)Updating files:   2% (23/1126)Updating files:   3% (34/1126)Updating files:   4% (46/1126)Updating files:   5% (57/1126)Updating files:   6% (68/1126)Updating files:   7% (79/1126)Updating files:   8% (91/1126)Updating files:   9% (102/1126)Updating files:  10% (113/1126)Updating files:  11% (124/1126)Updating files:  12% (136/1126)Updating files:  13% (147/1126)Updating files:  14% (158/1126)Updating files:  15% (169/1126)Updating files:  16% (181/1126)Updating files:  17% (192/1126)Updating files:  18% (203/1126)Updating files:  19% (214/1126)Updating files:  20% (226/1126)Updating files:  21% (237/1126)Updating files:  22% (248/1126)Updating files:  23% (259/1126)Updating files:  24% (271/1126)Updating files:  25% (282/1126)Updating files:  26% (293/1126)Updating files:  27% (305/1126)Updating files:  28% (316/1126)Updating files:  29% (327/1126)Updating files:  30% (338/1126)Updating files:  31% (350/1126)Updating files:  32% (361/1126)Updating files:  33% (372/1126)Updating files:  34% (383/1126)Updating files:  35% (395/1126)Updating files:  36% (406/1126)Updating files:  37% (417/1126)Updating files:  38% (428/1126)Updating files:  39% (440/1126)Updating files:  40% (451/1126)Updating files:  41% (462/1126)Updating files:  42% (473/1126)Updating files:  43% (485/1126)Updating files:  44% (496/1126)Updating files:  45% (507/1126)Updating files:  46% (518/1126)Updating files:  47% (530/1126)Updating files:  48% (541/1126)Updating files:  49% (552/1126)Updating files:  50% (563/1126)Updating files:  51% (575/1126)Updating files:  52% (586/1126)Updating files:  53% (597/1126)Updating files:  54% (609/1126)Updating files:  55% (620/1126)Updating files:  56% (631/1126)Updating files:  57% (642/1126)Updating files:  58% (654/1126)Updating files:  59% (665/1126)Updating files:  60% (676/1126)Updating files:  61% (687/1126)Updating files:  62% (699/1126)Updating files:  63% (710/1126)Updating files:  64% (721/1126)Updating files:  65% (732/1126)Updating files:  66% (744/1126)Updating files:  67% (755/1126)Updating files:  68% (766/1126)Updating files:  69% (777/1126)Updating files:  70% (789/1126)Updating files:  71% (800/1126)Updating files:  72% (811/1126)Updating files:  73% (822/1126)Updating files:  74% (834/1126)Updating files:  75% (845/1126)Updating files:  76% (856/1126)Updating files:  77% (868/1126)Updating files:  78% (879/1126)Updating files:  79% (890/1126)Updating files:  80% (901/1126)Updating files:  81% (913/1126)Updating files:  82% (924/1126)Updating files:  83% (935/1126)Updating files:  84% (946/1126)Updating files:  85% (958/1126)Updating files:  86% (969/1126)Updating files:  87% (980/1126)Updating files:  88% (991/1126)Updating files:  88% (1001/1126)Updating files:  89% (1003/1126)Updating files:  90% (1014/1126)Updating files:  91% (1025/1126)Updating files:  92% (1036/1126)Updating files:  93% (1048/1126)Updating files:  94% (1059/1126)Updating files:  95% (1070/1126)Updating files:  96% (1081/1126)Updating files:  97% (1093/1126)Updating files:  98% (1104/1126)Updating files:  99% (1115/1126)Updating files: 100% (1126/1126)Updating files: 100% (1126/1126), done.
Note: switching to '116bcc23c8052a1728992cdbc7b625143260d683'.

You are in 'detached HEAD' state. You can look around, make experimental
changes and commit them, and you can discard any commits you make in this
state without impacting any branches by switching back to a branch.

If you want to create a new branch to retain commits you create, you may
do so (now or later) by using -c with the switch command. Example:

  git switch -c <new-branch-name>

Or undo this operation with:

  git switch -

Turn off this advice by setting config variable advice.detachedHead to false

HEAD is now at 116bcc2 bump boot-iso hash for the x86_64 entry-flags kernel changes

~~~

### out/s6/linux-x86_64.log

~~~
S6-X86: leg control
S6-X86: control sha256=3ae4127abe1d8466617d22877f1171f4a30086f05e8ef7c1406c1d5dd8fa9de8
S6-X86: leg probe
S6-X86: probe sha256=7af2a8e07e4c969907727b3b31964bfddb3cad642fda643fc20766bda1d05f12
S6-X86: leg flipback
S6-X86: flipback sha256=3ae4127abe1d8466617d22877f1171f4a30086f05e8ef7c1406c1d5dd8fa9de8
S6-X86 control=PASS probe=PASS flipback=PASS sha-equal=yes

~~~

### out/s6/x86-control.driver.log

~~~
q35: intsrc bus 0 source 11 -> gsi 11 flags 0xd
q35: hpet 0xfed00000
pciexbar: 0xb0000001 -> 0xb0000001
pci: virtio-net 1af4:1041 at 00:02.0 rev 01 pin 1 (inta) bars 2
pci: inta -> gsi 22 (q35 root slot 2, pirq G: 16+6)
Memory usage:
  Used:		0x00000006800000
  Total:	0x0000003ffdf000
random: source=rdrand
lapic: cpuid leaf-1 ecx: x2apic=true tsc-deadline=true
lapic: x2apic live (msr 0x800 map; lint0/1 masked, no pic)
timer: 2496189480 Hz (pit 1193182 Hz reference), TscDeadline mode, armed
ioapic: id 0 version 0x11 rtes 23 gsib 0 (madt 0xfec00000)
pci: bar4 pa 0xfe000000 size 0x4000 (firmware) readback 0xfe000000
pci: caps common@0x0+0x1000 notify@0x3000 isr@0x1000 device@0x2000 mult 4
virtio-net: features offered 0x10130bf8024 negotiated 0x100010020
virtio-net: rx pre-posted 128 buffers qsize 256
ioapic: gsi 22 -> vector 0x21 (level)
#l0: virtio: 1000Mbps: 52:54:00:12:34:56
fpu: kernel-SIMD selftest ok (user state parked and restored across the section; xor_fold16 0x1983c6d50a18d707c9d74d9a42c59f27)
q35: module @0x14c000 +0x5f1ad (mb2 tag 3, stashed)
devenv: seeded cputype=amd64 nobootprompt=embed readparts=0
q35: paqfs root mounted from module (389549 bytes)
devroot: /boot/boot open (159349 bytes)
exec: entering ring 3 pc=0x2054fa sp=0x7fffffffef78 path=/boot/boot argv=[boot]
paqfs...version...root-chain: Sun Nov 22 21:15:01 GMT 2026
fingerprint: e7e84ec22c9cf702c8c76fe5a777428b9aa4d0a7
process 2 exited: ""
syscall 10 unimplemented
time...
boot: opening #c/swap: namec: does not exist
init: can't open #c/sysname: namec: does not exist
syscall 10 unimplemented

init: starting /bin/rc
% /bin/echo R9-ECHO-CONTROL
R9-ECHO-CONTROL
process 5 exited: ""
% QEMU: Terminated
q35shell-probe: leg complete — /bin/echo R9-ECHO-CONTROL

~~~

### out/s6/x86-control.log

~~~
q35: intsrc bus 0 source 11 -> gsi 11 flags 0xd
q35: hpet 0xfed00000
pciexbar: 0xb0000001 -> 0xb0000001
pci: virtio-net 1af4:1041 at 00:02.0 rev 01 pin 1 (inta) bars 2
pci: inta -> gsi 22 (q35 root slot 2, pirq G: 16+6)
Memory usage:
  Used:		0x00000006800000
  Total:	0x0000003ffdf000
random: source=rdrand
lapic: cpuid leaf-1 ecx: x2apic=true tsc-deadline=true
lapic: x2apic live (msr 0x800 map; lint0/1 masked, no pic)
timer: 2496189480 Hz (pit 1193182 Hz reference), TscDeadline mode, armed
ioapic: id 0 version 0x11 rtes 23 gsib 0 (madt 0xfec00000)
pci: bar4 pa 0xfe000000 size 0x4000 (firmware) readback 0xfe000000
pci: caps common@0x0+0x1000 notify@0x3000 isr@0x1000 device@0x2000 mult 4
virtio-net: features offered 0x10130bf8024 negotiated 0x100010020
virtio-net: rx pre-posted 128 buffers qsize 256
ioapic: gsi 22 -> vector 0x21 (level)
#l0: virtio: 1000Mbps: 52:54:00:12:34:56
fpu: kernel-SIMD selftest ok (user state parked and restored across the section; xor_fold16 0x1983c6d50a18d707c9d74d9a42c59f27)
q35: module @0x14c000 +0x5f1ad (mb2 tag 3, stashed)
devenv: seeded cputype=amd64 nobootprompt=embed readparts=0
q35: paqfs root mounted from module (389549 bytes)
devroot: /boot/boot open (159349 bytes)
exec: entering ring 3 pc=0x2054fa sp=0x7fffffffef78 path=/boot/boot argv=[boot]
paqfs...version...root-chain: Sun Nov 22 21:15:01 GMT 2026
fingerprint: e7e84ec22c9cf702c8c76fe5a777428b9aa4d0a7
process 2 exited: ""
syscall 10 unimplemented
time...
boot: opening #c/swap: namec: does not exist
init: can't open #c/sysname: namec: does not exist
syscall 10 unimplemented

init: starting /bin/rc
% /bin/echo R9-ECHO-CONTROL
R9-ECHO-CONTROL
process 5 exited: ""
% QEMU: Terminated

~~~

### out/s6/x86-flipback.driver.log

~~~
q35: intsrc bus 0 source 11 -> gsi 11 flags 0xd
q35: hpet 0xfed00000
pciexbar: 0xb0000001 -> 0xb0000001
pci: virtio-net 1af4:1041 at 00:02.0 rev 01 pin 1 (inta) bars 2
pci: inta -> gsi 22 (q35 root slot 2, pirq G: 16+6)
Memory usage:
  Used:		0x00000006800000
  Total:	0x0000003ffdf000
random: source=rdrand
lapic: cpuid leaf-1 ecx: x2apic=true tsc-deadline=true
lapic: x2apic live (msr 0x800 map; lint0/1 masked, no pic)
timer: 2496401940 Hz (pit 1193182 Hz reference), TscDeadline mode, armed
ioapic: id 0 version 0x11 rtes 23 gsib 0 (madt 0xfec00000)
pci: bar4 pa 0xfe000000 size 0x4000 (firmware) readback 0xfe000000
pci: caps common@0x0+0x1000 notify@0x3000 isr@0x1000 device@0x2000 mult 4
virtio-net: features offered 0x10130bf8024 negotiated 0x100010020
virtio-net: rx pre-posted 128 buffers qsize 256
ioapic: gsi 22 -> vector 0x21 (level)
#l0: virtio: 1000Mbps: 52:54:00:12:34:56
fpu: kernel-SIMD selftest ok (user state parked and restored across the section; xor_fold16 0x1983c6d50a18d707c9d74d9a42c59f27)
q35: module @0x14c000 +0x5f1ad (mb2 tag 3, stashed)
devenv: seeded cputype=amd64 nobootprompt=embed readparts=0
q35: paqfs root mounted from module (389549 bytes)
devroot: /boot/boot open (159349 bytes)
exec: entering ring 3 pc=0x2054fa sp=0x7fffffffef78 path=/boot/boot argv=[boot]
paqfs...version...root-chain: Sun Nov 22 21:15:01 GMT 2026
fingerprint: e7e84ec22c9cf702c8c76fe5a777428b9aa4d0a7
process 2 exited: ""
syscall 10 unimplemented
time...
boot: opening #c/swap: namec: does not exist
init: can't open #c/sysname: namec: does not exist
syscall 10 unimplemented

init: starting /bin/rc
% /bin/echo R9-ECHO-CONTROL
R9-ECHO-CONTROL
process 5 exited: ""
% QEMU: Terminated
q35shell-probe: leg complete — /bin/echo R9-ECHO-CONTROL

~~~

### out/s6/x86-flipback.log

~~~
q35: intsrc bus 0 source 11 -> gsi 11 flags 0xd
q35: hpet 0xfed00000
pciexbar: 0xb0000001 -> 0xb0000001
pci: virtio-net 1af4:1041 at 00:02.0 rev 01 pin 1 (inta) bars 2
pci: inta -> gsi 22 (q35 root slot 2, pirq G: 16+6)
Memory usage:
  Used:		0x00000006800000
  Total:	0x0000003ffdf000
random: source=rdrand
lapic: cpuid leaf-1 ecx: x2apic=true tsc-deadline=true
lapic: x2apic live (msr 0x800 map; lint0/1 masked, no pic)
timer: 2496401940 Hz (pit 1193182 Hz reference), TscDeadline mode, armed
ioapic: id 0 version 0x11 rtes 23 gsib 0 (madt 0xfec00000)
pci: bar4 pa 0xfe000000 size 0x4000 (firmware) readback 0xfe000000
pci: caps common@0x0+0x1000 notify@0x3000 isr@0x1000 device@0x2000 mult 4
virtio-net: features offered 0x10130bf8024 negotiated 0x100010020
virtio-net: rx pre-posted 128 buffers qsize 256
ioapic: gsi 22 -> vector 0x21 (level)
#l0: virtio: 1000Mbps: 52:54:00:12:34:56
fpu: kernel-SIMD selftest ok (user state parked and restored across the section; xor_fold16 0x1983c6d50a18d707c9d74d9a42c59f27)
q35: module @0x14c000 +0x5f1ad (mb2 tag 3, stashed)
devenv: seeded cputype=amd64 nobootprompt=embed readparts=0
q35: paqfs root mounted from module (389549 bytes)
devroot: /boot/boot open (159349 bytes)
exec: entering ring 3 pc=0x2054fa sp=0x7fffffffef78 path=/boot/boot argv=[boot]
paqfs...version...root-chain: Sun Nov 22 21:15:01 GMT 2026
fingerprint: e7e84ec22c9cf702c8c76fe5a777428b9aa4d0a7
process 2 exited: ""
syscall 10 unimplemented
time...
boot: opening #c/swap: namec: does not exist
init: can't open #c/sysname: namec: does not exist
syscall 10 unimplemented

init: starting /bin/rc
% /bin/echo R9-ECHO-CONTROL
R9-ECHO-CONTROL
process 5 exited: ""
% QEMU: Terminated

~~~

### out/s6/x86-probe.driver.log

~~~
q35: intsrc bus 0 source 11 -> gsi 11 flags 0xd
q35: hpet 0xfed00000
pciexbar: 0xb0000001 -> 0xb0000001
pci: virtio-net 1af4:1041 at 00:02.0 rev 01 pin 1 (inta) bars 2
pci: inta -> gsi 22 (q35 root slot 2, pirq G: 16+6)
Memory usage:
  Used:		0x00000006800000
  Total:	0x0000003ffdf000
random: source=rdrand
lapic: cpuid leaf-1 ecx: x2apic=true tsc-deadline=true
lapic: x2apic live (msr 0x800 map; lint0/1 masked, no pic)
timer: 2496216000 Hz (pit 1193182 Hz reference), TscDeadline mode, armed
ioapic: id 0 version 0x11 rtes 23 gsib 0 (madt 0xfec00000)
pci: bar4 pa 0xfe000000 size 0x4000 (firmware) readback 0xfe000000
pci: caps common@0x0+0x1000 notify@0x3000 isr@0x1000 device@0x2000 mult 4
virtio-net: features offered 0x10130bf8024 negotiated 0x100010020
virtio-net: rx pre-posted 128 buffers qsize 256
ioapic: gsi 22 -> vector 0x21 (level)
#l0: virtio: 1000Mbps: 52:54:00:12:34:56
fpu: kernel-SIMD selftest ok (user state parked and restored across the section; xor_fold16 0x1983c6d50a18d707c9d74d9a42c59f27)
q35: module @0x14c000 +0x58371 (mb2 tag 3, stashed)
devenv: seeded cputype=amd64 nobootprompt=embed readparts=0
q35: paqfs root mounted from module (361329 bytes)
devroot: /boot/boot open (159349 bytes)
exec: entering ring 3 pc=0x2054fa sp=0x7fffffffef78 path=/boot/boot argv=[boot]
paqfs...version...root-chain: Sun Nov 22 21:15:01 GMT 2026
fingerprint: ec6b0005213377e925717a609b1700539836ffdf
process 2 exited: ""
syscall 10 unimplemented
time...
boot: opening #c/swap: namec: does not exist
init: can't open #c/sysname: namec: does not exist
syscall 10 unimplemented

init: starting /bin/rc
% /bin/echo x
R9-RUST-AOUT-PROBE-OK
process 5 exited: ""
% QEMU: Terminated
q35shell-probe: leg complete — /bin/echo x

~~~

### out/s6/x86-probe.log

~~~
q35: intsrc bus 0 source 11 -> gsi 11 flags 0xd
q35: hpet 0xfed00000
pciexbar: 0xb0000001 -> 0xb0000001
pci: virtio-net 1af4:1041 at 00:02.0 rev 01 pin 1 (inta) bars 2
pci: inta -> gsi 22 (q35 root slot 2, pirq G: 16+6)
Memory usage:
  Used:		0x00000006800000
  Total:	0x0000003ffdf000
random: source=rdrand
lapic: cpuid leaf-1 ecx: x2apic=true tsc-deadline=true
lapic: x2apic live (msr 0x800 map; lint0/1 masked, no pic)
timer: 2496216000 Hz (pit 1193182 Hz reference), TscDeadline mode, armed
ioapic: id 0 version 0x11 rtes 23 gsib 0 (madt 0xfec00000)
pci: bar4 pa 0xfe000000 size 0x4000 (firmware) readback 0xfe000000
pci: caps common@0x0+0x1000 notify@0x3000 isr@0x1000 device@0x2000 mult 4
virtio-net: features offered 0x10130bf8024 negotiated 0x100010020
virtio-net: rx pre-posted 128 buffers qsize 256
ioapic: gsi 22 -> vector 0x21 (level)
#l0: virtio: 1000Mbps: 52:54:00:12:34:56
fpu: kernel-SIMD selftest ok (user state parked and restored across the section; xor_fold16 0x1983c6d50a18d707c9d74d9a42c59f27)
q35: module @0x14c000 +0x58371 (mb2 tag 3, stashed)
devenv: seeded cputype=amd64 nobootprompt=embed readparts=0
q35: paqfs root mounted from module (361329 bytes)
devroot: /boot/boot open (159349 bytes)
exec: entering ring 3 pc=0x2054fa sp=0x7fffffffef78 path=/boot/boot argv=[boot]
paqfs...version...root-chain: Sun Nov 22 21:15:01 GMT 2026
fingerprint: ec6b0005213377e925717a609b1700539836ffdf
process 2 exited: ""
syscall 10 unimplemented
time...
boot: opening #c/swap: namec: does not exist
init: can't open #c/sysname: namec: does not exist
syscall 10 unimplemented

init: starting /bin/rc
% /bin/echo x
R9-RUST-AOUT-PROBE-OK
process 5 exited: ""
% QEMU: Terminated

~~~

