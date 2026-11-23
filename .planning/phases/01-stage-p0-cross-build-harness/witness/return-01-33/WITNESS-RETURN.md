# r9 pass-2 witness return (plans 01-09 / 01-32)

- date: 2026-11-23T16:29:51Z
- git HEAD: 9af05f8b71e57a422c36b3a23e168399bb1c88b6
- r9 root: /home/req/code/r9
- work root: /home/req/r9-witness-work
- steps: kernel gate gate_neg probe flipback profiles cpuauth_ref

## out/env.txt

~~~
# r9 pass-2 witness preflight
date=2026-11-23T16:17:30Z
user=req uid=1000
r9_root=/home/req/code/r9
git_head=9af05f8b71e57a422c36b3a23e168399bb1c88b6
work_dir=/home/req/r9-witness-work
steps=kernel gate gate_neg probe flipback profiles cpuauth_ref
harvey_pin=bd7a6982a3e4f465f8ed3da5e0c7290521514ff0
kerndate=1795382101
uname=Linux teeniebox 7.4.0-rc3 #1-NixOS SMP PREEMPT_DYNAMIC @315532800 x86_64 GNU/Linux
nproc=16
               total        used        free      shared  buff/cache   available
Mem:            30Gi       1.8Gi        21Gi       441Mi       8.1Gi        29Gi
Swap:          7.5Gi          0B       7.5Gi
--- free disk under /home/req/r9-witness-work ---
Filesystem        Size  Used Avail Use% Mounted on
/dev/mapper/root  946G  577G  365G  62% /
--- nix ---
nix (Nix) 2.35.6
experimental-features = dynamic-derivations fetch-closure fetch-tree flakes nix-command
sandbox = true
--- /dev/kvm ---
DEV_KVM=writable
crw-rw-rw- 1 root kvm 10, 232 Nov 23 10:23 /dev/kvm

~~~

## out/summary.tsv

~~~
kernel	1	32
gate	1	272
gate_neg	0	95
probe	1	46
flipback	1	48
profiles	0	150
cpuauth_ref	0	98

~~~

## out/survivors.txt

~~~
# surviving qemu/Xvfb/drawterm processes owned by req
# at 2026-11-23T16:29:51Z
none

~~~

## out/secret-scan.txt

~~~
# credential scan of out/ at 2026-11-23T16:29:51Z
scanned out/ for the password value of harvey/build/x86_64/out/auth-creds.txt
clean: no credential artifact and no DEMO password value in out/

~~~

## last 40 lines of each out/<step>/*.log

### out/a13/a13.log

~~~
/nix/store/y8h5ana2drnzs2fdx6fxgr74bwmzi8y9-drawterm-19.06

~~~

### out/control/bake.log

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
== PASS: baked /home/req/r9-witness-work/harvey/build/x86_64/out/bootfs-cpu.paq (22002486 bytes; chain + the auth suite + cmd/cpu + aux/listen + the real cpurc + tcp17010 + the font payload + the §15 p9sk1 domain + the 19-10 desktop suite (test/rio/plumber/plumb/ramfs/syscall/awd + glenda's home + the plumb rules); header 529ab12b trailer 6b46e688; KERNDATE 1795382101)

~~~

### out/control/control.log

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
% RIO-CONTROL-NO-RIO
rio-gate: post-pass ok — zero cpurc: FAIL in the capture
rio-gate: post-pass ok — zero kernel panic markers (drawterm's own sysfatal exit pair excluded)
rio-gate: post-pass ok — the per-bake DEMO password redacted from the retained capture (0 occurrence(s) mapped)
rio-gate: ALL STAGES PASS — cpu boot to cpurc: DONE, -G control session, serial /srv shows no rio entry (RIO-CONTROL-NO-RIO)
== rio control-noG gate PASS — capture retained at /home/req/r9-witness-work/out/control/rio-gate-control-noG.log

~~~

### out/control/rio-gate-control-noG.log

~~~
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

% 
rio-gate: redaction contract held — <demo-pw-redacted> (0 occurrence(s) of the per-bake DEMO password mapped; T-01-30)

~~~

### out/cpuauth-ref/cpuauth-ref.log

~~~
    |
566 | pub struct Icmpv6Frame {
    |            ----------- fields in this struct
...
569 |     pub dst_mac: [u8; 6],
    |         ^^^^^^^
...
579 |     pub frame_len: usize,
    |         ^^^^^^^^^
    |
    = note: `Icmpv6Frame` has a derived impl for the trait `Debug`, but this is intentionally ignored during dead code analysis

warning: fields `src_mac`, `dst_mac`, and `frame_len` are never read
   --> xtask/src/pcap.rs:723:9
    |
721 | pub struct Tcpv4Frame {
    |            ---------- fields in this struct
722 |     /// Ethernet source/destination MAC.
723 |     pub src_mac: [u8; 6],
    |         ^^^^^^^
724 |     pub dst_mac: [u8; 6],
    |         ^^^^^^^
...
737 |     pub frame_len: usize,
    |         ^^^^^^^^^
    |
    = note: `Tcpv4Frame` has a derived impl for the trait `Debug`, but this is intentionally ignored during dead code analysis

warning: `xtask` (bin "xtask") generated 4 warnings
    Finished `dev` profile [optimized + debuginfo] target(s) in 0.02s
     Running `target/debug/xtask image --arch x86-64 --image auth --compare-reference`
authdomain-gen: generated /home/req/code/r9/target/images/x86_64/auth.authdom: label=DEMO authdom=DEMO-harvey authid=bootes users=3
authdomain-gen: DEMO material is throwaway — never reuse outside a gate image
authdomain-gen: no authdomain.local — generating ephemeral DEMO material
image: authdomain generated a fresh p9sk1 domain for "auth" at /home/req/code/r9/target/images/x86_64/auth.authdom (authdom DEMO-harvey)
image: authdomain pairing verified over the staged copies for "auth"
image: creds /home/req/code/r9/target/images/x86_64/auth.creds (mode 0600)
image: x86_64/auth /home/req/code/r9/target/images/x86_64/auth.paq sha256 adb7b108e1bd06a2838c11ad2361d440431ae83511b418e438b0f026d8b0f26f members 95
image: x86_64/auth matches the reference member list (95 members)
CPUAUTH-REF auth exit=0

~~~

### out/flipback/flipback.log

~~~
    |        ^^^^^

warning: function `splx` is never used
   --> x86_64/src/trap.rs:790:8
    |
790 | pub fn splx(x: IntrStatus) -> IntrStatus {
    |        ^^^^

warning: `x86_64` (bin "x86_64") generated 25 warnings (run `cargo fix --bin "x86_64" -p x86_64` to apply 2 suggestions)
    Finished `dev` profile [optimized + debuginfo] target(s) in 14.37s
mb2: header at file offset 0x1008 validated on the stripped ISO kernel (magic/arch/length/checksum/end tag)
iso: module /home/req/r9-witness-work/r9-probe-flipback/target/images/x86_64/cpu.paq staged at /boot/bootfs.paq
iso: kernel cmdline append "init=/amd64/init -c" (mb2 tag 1, the bootconf carrier)
xorriso 1.5.8.pl02 : RockRidge filesystem manipulator, libburnia project.

xorriso : NOTE : Environment variable SOURCE_DATE_EPOCH encountered with value 315532800
Drive current: -outdev 'stdio:target/boot-q35cpu.iso'
Media current: stdio file, overwriteable
Media status : is blank
Media summary: 0 sessions, 0 data blocks, 0 data,  365g free
Added to ISO image: directory '/'='/tmp/nix-shell.DumawO/grub.OboYh9'
xorriso : UPDATE :     356 files added in 1 seconds
Added to ISO image: directory '/'='/home/req/r9-witness-work/r9-probe-flipback/target/iso-q35cpu'
xorriso : UPDATE :     361 files added in 1 seconds
xorriso : NOTE : Copying to System Area: 512 bytes from file '/nix/store/jniwmlg4zv2r4baccllbizddnh1scfjr-grub-2.14/lib/grub/i386-pc/boot_hybrid.img'
ISO image produced: 19721 sectors
Written to medium : 19721 sectors at LBA 0
Writing to 'stdio:target/boot-q35cpu.iso' completed successfully.

gate: iso /home/req/r9-witness-work/r9-probe-flipback/target/boot-q35cpu.iso 
gate: leg rio (capture /home/req/r9-witness-work/r9-probe-flipback/target/gate/197244-1795451108/rio.log)
can't read "spawn_id": no such variable
    while executing
"expect_before -i $spawn_id {
	-re {panic|Unhandled interrupt} {
		puts stderr "drawterm-gate: kernel panic marker on the wire — aborting"
		cleanup
..."
    (file "/home/req/r9-witness-work/r9-probe-flipback/gates/x86_64/drawterm-gate.expect" line 336)
GATE-FAIL rio-leg-failed (leg rio; capture /home/req/r9-witness-work/r9-probe-flipback/target/gate/197244-1795451108/rio.log)

~~~

### out/gate/gate.log

~~~
/srv/factotum
process 69 exited: ""
% process 71 exited: ""
process 70 exited: ""
devmnt: server error: './dev/label' file does not exist
devmnt: server error: './dev/winname' file does not exist
ls /srv
/srv/boot
/srv/factotum
/srv/plumb.glenda.71
/srv/rio.glenda.74
/srv/riowctl.glenda.74
process 80 exited: ""
% GATE-PASS rio-serving
drawterm-gate: stage ok — banner
drawterm-gate: stage ok — bootconf
drawterm-gate: stage ok — paqfs
drawterm-gate: stage ok — version
drawterm-gate: stage ok — hostfwd-17010-listening (127.0.0.1:17010 present in the host listening table)
drawterm-gate: stage ok — hostfwd-5670-listening (127.0.0.1:5670 present in the host listening table)
drawterm-gate: stage ok — pairing-ok
drawterm-gate: stage ok — ether-ok
drawterm-gate: stage ok — keyfs-ok
drawterm-gate: stage ok — authsrv-listen
drawterm-gate: stage ok — factotum-ok
drawterm-gate: stage ok — listen-17010
drawterm-gate: stage ok — ladder-done
drawterm-gate: Xvfb ready (DISPLAY=:0.0, -nolisten tcp)
drawterm-gate: spawned drawterm on a pty (mode rio, argv0 tag r9x64dtgate-dt-164734-1795450821, user glenda)
drawterm-gate: feed ok — XTEST password entry into the drawterm window (48234497)
drawterm-gate: stage ok — session window quiet (rio draws; no pty text expected)
drawterm-gate: stage ok — serial console rc prompt (init's handoff after cpurc: DONE)
drawterm-gate: stage ok — rio budget elapsed (60 s)
drawterm-gate: no pair after poll 1 — sent the documented manual start (plumber; rio)
drawterm-gate: stage ok — serial /srv shows rio and riowctl (D-28 marker)
drawterm-gate: post-pass ok — zero cpurc: FAIL in the capture
drawterm-gate: post-pass ok — zero kernel panic markers (drawterm's own sysfatal exit pair excluded)
drawterm-gate: post-pass ok — the per-bake DEMO password redacted from the retained capture (T-01-58)
GATE-FAIL rio-leg-failed (leg rio; capture /home/req/code/r9/target/gate/164734-1795450821/rio.log)
GATE-RUN 2 exit=nonzero

~~~

### out/gate-neg/gate-neg.log

~~~
/srv/factotum
process 76 exited: ""
% ls /srv
/srv/boot
/srv/factotum
process 77 exited: ""
% ls /srv
/srv/boot
/srv/factotum
process 78 exited: ""
% ls /srv
/srv/boot
/srv/factotum
process 79 exited: ""
% ls /srv
/srv/boot
/srv/factotum
process 80 exited: ""
% GATE-FAIL rio-not-serving
drawterm-gate: stage ok — banner
drawterm-gate: stage ok — bootconf
drawterm-gate: stage ok — paqfs
drawterm-gate: stage ok — version
drawterm-gate: stage ok — hostfwd-17010-listening (127.0.0.1:17010 present in the host listening table)
drawterm-gate: stage ok — hostfwd-5670-listening (127.0.0.1:5670 present in the host listening table)
drawterm-gate: stage ok — pairing-ok
drawterm-gate: stage ok — ether-ok
drawterm-gate: stage ok — keyfs-ok
drawterm-gate: stage ok — authsrv-listen
drawterm-gate: stage ok — factotum-ok
drawterm-gate: stage ok — listen-17010
drawterm-gate: stage ok — ladder-done
drawterm-gate: spawned drawterm on a pty (mode force-no-rio, argv0 tag r9x64dtgate-dt-174412-1795450955, user glenda)
drawterm-gate: feed ok — dt-password
drawterm-gate: stage ok — session-prompt (the remote interactive shell)
drawterm-gate: stage ok — serial console rc prompt (init's handoff after cpurc: DONE)
drawterm-gate: stage ok — rio budget elapsed (60 s)
drawterm-gate: GATE-FAIL rio-not-serving — no rio.<user>.<pid> + riowctl.<user>.<pid> pair in the serial /srv listing within the budget
GATE-FAIL force-no-rio-leg-failed (leg force-no-rio; capture /home/req/code/r9/target/gate/174412-1795450955/force-no-rio.log)
GATE-NEG-EXPECTED-FAIL

~~~

### out/kernel/kernel.log

~~~
this derivation will be built:
  /nix/store/nyiky00zwfs8cbd9mv253n52y6bbj49j-r9-kernel-x64-9af05f8.drv
building '/nix/store/nyiky00zwfs8cbd9mv253n52y6bbj49j-r9-kernel-x64-9af05f8.drv'...
KERNEL-STORE /nix/store/imb1n1k67mpiv0v2cn0wr0vj8nw1pcc0-r9-kernel-x64-9af05f8
4228a1bf1408a7f06d942fa847b6ca25d7bfbd5a400ce9190b602d3b365f32e1  /nix/store/imb1n1k67mpiv0v2cn0wr0vj8nw1pcc0-r9-kernel-x64-9af05f8/kernel-x64.elf
this derivation will be built:
  /nix/store/c09d6n0f2n5hlsmv65250i9mnjljpf5j-r9-boot-iso-9af05f8.drv
building '/nix/store/c09d6n0f2n5hlsmv65250i9mnjljpf5j-r9-boot-iso-9af05f8.drv'...
error: hash mismatch in fixed-output derivation '/nix/store/c09d6n0f2n5hlsmv65250i9mnjljpf5j-r9-boot-iso-9af05f8.drv':
         specified: sha256-TIzwBaHF0R4m31l6EbW2IU3rkUl73g6ljlNE0S5hGFY=
            got:    sha256-xFt7GUshPv+CQNry48fBOAiCsCIuqGZfeC0eX11/8mc=
KERNEL-BOOTISO-BUILD-FAIL

~~~

### out/probe/probe.log

~~~
    |        ^^^^^

warning: function `splx` is never used
   --> x86_64/src/trap.rs:790:8
    |
790 | pub fn splx(x: IntrStatus) -> IntrStatus {
    |        ^^^^

warning: `x86_64` (bin "x86_64") generated 25 warnings (run `cargo fix --bin "x86_64" -p x86_64` to apply 2 suggestions)
    Finished `dev` profile [optimized + debuginfo] target(s) in 14.67s
mb2: header at file offset 0x1008 validated on the stripped ISO kernel (magic/arch/length/checksum/end tag)
iso: module /home/req/r9-witness-work/r9-probe-flip/target/images/x86_64/cpu.paq staged at /boot/bootfs.paq
iso: kernel cmdline append "init=/amd64/init -c" (mb2 tag 1, the bootconf carrier)
xorriso 1.5.8.pl02 : RockRidge filesystem manipulator, libburnia project.

xorriso : NOTE : Environment variable SOURCE_DATE_EPOCH encountered with value 315532800
Drive current: -outdev 'stdio:target/boot-q35cpu.iso'
Media current: stdio file, overwriteable
Media status : is blank
Media summary: 0 sessions, 0 data blocks, 0 data,  365g free
Added to ISO image: directory '/'='/tmp/nix-shell.jOXxCJ/grub.eD7Q9O'
xorriso : UPDATE :     356 files added in 1 seconds
Added to ISO image: directory '/'='/home/req/r9-witness-work/r9-probe-flip/target/iso-q35cpu'
xorriso : UPDATE :     361 files added in 1 seconds
xorriso : NOTE : Copying to System Area: 512 bytes from file '/nix/store/jniwmlg4zv2r4baccllbizddnh1scfjr-grub-2.14/lib/grub/i386-pc/boot_hybrid.img'
ISO image produced: 19704 sectors
Written to medium : 19704 sectors at LBA 0
Writing to 'stdio:target/boot-q35cpu.iso' completed successfully.

gate: iso /home/req/r9-witness-work/r9-probe-flip/target/boot-q35cpu.iso 
gate: leg rio (capture /home/req/r9-witness-work/r9-probe-flip/target/gate/184084-1795451060/rio.log)
can't read "spawn_id": no such variable
    while executing
"expect_before -i $spawn_id {
	-re {panic|Unhandled interrupt} {
		puts stderr "drawterm-gate: kernel panic marker on the wire — aborting"
		cleanup
..."
    (file "/home/req/r9-witness-work/r9-probe-flip/gates/x86_64/drawterm-gate.expect" line 336)
GATE-FAIL rio-leg-failed (leg rio; capture /home/req/r9-witness-work/r9-probe-flip/target/gate/184084-1795451060/rio.log)

~~~

### out/profiles/profiles.log

~~~
PROFILE q35 exit=0
PROFILE q35lowmcfg exit=0
PROFILE q35net exit=0
PROFILE q35exec exit=0
PROFILE q35churn exit=0
PROFILE q35shell exit=0
PROFILE q35churnshell exit=0
PROFILE q35cpu exit=0
PROFILE q35auth exit=0

~~~

### out/profiles/q35auth.log

~~~
    |
778 | pub fn spllo() -> IntrStatus {
    |        ^^^^^

warning: function `splhi` is never used
   --> x86_64/src/trap.rs:784:8
    |
784 | pub fn splhi() -> IntrStatus {
    |        ^^^^^

warning: function `splx` is never used
   --> x86_64/src/trap.rs:790:8
    |
790 | pub fn splx(x: IntrStatus) -> IntrStatus {
    |        ^^^^

warning: `x86_64` (bin "x86_64") generated 12 warnings (run `cargo fix --bin "x86_64" -p x86_64` to apply 1 suggestion)
    Finished `dev` profile [optimized + debuginfo] target(s) in 14.45s
mb2: header at file offset 0x1008 validated on the stripped ISO kernel (magic/arch/length/checksum/end tag)
iso: module /home/req/code/r9/target/images/x86_64/substrate.paq staged at /boot/bootfs.paq
xorriso 1.5.8.pl02 : RockRidge filesystem manipulator, libburnia project.

xorriso : NOTE : Environment variable SOURCE_DATE_EPOCH encountered with value 315532800
Drive current: -outdev 'stdio:target/boot-q35auth.iso'
Media current: stdio file, overwriteable
Media status : is blank
Media summary: 0 sessions, 0 data blocks, 0 data,  365g free
Added to ISO image: directory '/'='/tmp/nix-shell.OhwF6x/grub.2EspD5'
xorriso : UPDATE :     356 files added in 1 seconds
Added to ISO image: directory '/'='/home/req/code/r9/target/iso-q35auth'
xorriso : UPDATE :     361 files added in 1 seconds
xorriso : NOTE : Copying to System Area: 512 bytes from file '/nix/store/jniwmlg4zv2r4baccllbizddnh1scfjr-grub-2.14/lib/grub/i386-pc/boot_hybrid.img'
ISO image produced: 8989 sectors
Written to medium : 8989 sectors at LBA 0
Writing to 'stdio:target/boot-q35auth.iso' completed successfully.

qemu-system-x86_64: -chardev socket,id=cons,path=target/boot-test-q35auth.sock,server=on,wait=on: info: QEMU waiting for connection on: disconnected:unix:target/boot-test-q35auth.sock,server=on
boot-test: PASS -- SeaBIOS -> GRUB2 -> mb2 kernel + module boot: banner, memory facts, discovery, the module/paqfs mount notes, the exec-entry note, the armed source "random: source=rdrand" (venue "kvm"), then the staged driver's OWN ring-3 prints (two 64-byte /dev/random reads, full-length and pairwise distinct -> READS-OK; the #¤ fixture legs: post, consume with the /dev/user readback, one-shot reject, hostowner gate, hostowner write refused -> the five cap markers; DONE via WRITE syscalls) — AUTH-02's substrate re-key + AUTH-03's devcap live with CR-01's eve-base gate closed (D-05/D-06/D-07, 18-06)
boot-test: serial log written to /home/req/code/r9/target/boot-test-q35auth.log
boot-test: captured 2207 bytes

~~~

### out/profiles/q35churn.log

~~~
    |
778 | pub fn spllo() -> IntrStatus {
    |        ^^^^^

warning: function `splhi` is never used
   --> x86_64/src/trap.rs:784:8
    |
784 | pub fn splhi() -> IntrStatus {
    |        ^^^^^

warning: function `splx` is never used
   --> x86_64/src/trap.rs:790:8
    |
790 | pub fn splx(x: IntrStatus) -> IntrStatus {
    |        ^^^^

warning: `x86_64` (bin "x86_64") generated 13 warnings
    Finished `dev` profile [optimized + debuginfo] target(s) in 14.47s
mb2: header at file offset 0x1008 validated on the stripped ISO kernel (magic/arch/length/checksum/end tag)
iso: module /home/req/code/r9/target/images/x86_64/marker.paq staged at /boot/bootfs.paq
xorriso 1.5.8.pl02 : RockRidge filesystem manipulator, libburnia project.

xorriso : NOTE : Environment variable SOURCE_DATE_EPOCH encountered with value 315532800
Drive current: -outdev 'stdio:target/boot-q35churn.iso'
Media current: stdio file, overwriteable
Media status : is blank
Media summary: 0 sessions, 0 data blocks, 0 data,  365g free
Added to ISO image: directory '/'='/tmp/nix-shell.45cLsw/grub.kAegef'
xorriso : UPDATE :     356 files added in 1 seconds
Added to ISO image: directory '/'='/home/req/code/r9/target/iso-q35churn'
xorriso : UPDATE :     361 files added in 1 seconds
xorriso : NOTE : Copying to System Area: 512 bytes from file '/nix/store/jniwmlg4zv2r4baccllbizddnh1scfjr-grub-2.14/lib/grub/i386-pc/boot_hybrid.img'
ISO image produced: 10798 sectors
Written to medium : 10798 sectors at LBA 0
Writing to 'stdio:target/boot-q35churn.iso' completed successfully.

qemu-system-x86_64: -chardev socket,id=cons,path=target/boot-test-q35churn.sock,server=on,wait=on: info: QEMU waiting for connection on: disconnected:unix:target/boot-test-q35churn.sock,server=on
boot-test: PASS -- SeaBIOS -> GRUB2 -> mb2 kernel + module boot: banner, memory facts, discovery, the module-tag/paqfs mount notes, then the kernel-context churn driver's 1400 exec iterations (1361 churnmark ring-3 prints counted) + 39 RFMEM contention generations, drain, and the heap slope held: steady=466 measured=128 <= ceiling=1772 bytes/exec (host re-derived from the kernel's own samples) — D-06 tier (a) green, CR-03's in-kernel contention leg landed
boot-test: serial log written to /home/req/code/r9/target/boot-test-q35churn.log
boot-test: captured 124268 bytes

~~~

### out/profiles/q35churnshell.log

~~~
    |
778 | pub fn spllo() -> IntrStatus {
    |        ^^^^^

warning: function `splhi` is never used
   --> x86_64/src/trap.rs:784:8
    |
784 | pub fn splhi() -> IntrStatus {
    |        ^^^^^

warning: function `splx` is never used
   --> x86_64/src/trap.rs:790:8
    |
790 | pub fn splx(x: IntrStatus) -> IntrStatus {
    |        ^^^^

warning: `x86_64` (bin "x86_64") generated 48 warnings (run `cargo fix --bin "x86_64" -p x86_64` to apply 1 suggestion)
    Finished `dev` profile [optimized + debuginfo] target(s) in 14.57s
mb2: header at file offset 0x1008 validated on the stripped ISO kernel (magic/arch/length/checksum/end tag)
iso: module /home/req/code/r9/target/images/x86_64/chain.paq staged at /boot/bootfs.paq
xorriso 1.5.8.pl02 : RockRidge filesystem manipulator, libburnia project.

xorriso : NOTE : Environment variable SOURCE_DATE_EPOCH encountered with value 315532800
Drive current: -outdev 'stdio:target/boot-q35churnshell.iso'
Media current: stdio file, overwriteable
Media status : is blank
Media summary: 0 sessions, 0 data blocks, 0 data,  365g free
Added to ISO image: directory '/'='/tmp/nix-shell.InrNQA/grub.juUatV'
xorriso : UPDATE :     356 files added in 1 seconds
Added to ISO image: directory '/'='/home/req/code/r9/target/iso-q35churnshell'
xorriso : UPDATE :     361 files added in 1 seconds
xorriso : NOTE : Copying to System Area: 512 bytes from file '/nix/store/jniwmlg4zv2r4baccllbizddnh1scfjr-grub-2.14/lib/grub/i386-pc/boot_hybrid.img'
ISO image produced: 10942 sectors
Written to medium : 10942 sectors at LBA 0
Writing to 'stdio:target/boot-q35churnshell.iso' completed successfully.

qemu-system-x86_64: -chardev socket,id=cons,path=target/boot-test-q35churnshell.sock,server=on,wait=on: info: QEMU waiting for connection on: disconnected:unix:target/boot-test-q35churnshell.sock,server=on
boot-test: PASS -- SeaBIOS -> GRUB2 -> mb2 kernel + chain-module boot: banner, memory facts, discovery, the NIC bind, the module-tag/paqfs mount notes, the exec-entry note, then the REAL boot(8)→init→rc chain's own ring-3 prints (paqfs/version handoffs, the init rc handoff) through to the interactive `% ` prompt, observed in order — XEXEC-03 leg 2 (D-08's staged chain on the proven exec substrate; the composed Harvey-side gate is 14-07's)
boot-test: serial log written to /home/req/code/r9/target/boot-test-q35churnshell.log
boot-test: captured 2689 bytes

~~~

### out/profiles/q35cpu.log

~~~
778 | pub fn spllo() -> IntrStatus {
    |        ^^^^^

warning: function `splhi` is never used
   --> x86_64/src/trap.rs:784:8
    |
784 | pub fn splhi() -> IntrStatus {
    |        ^^^^^

warning: function `splx` is never used
   --> x86_64/src/trap.rs:790:8
    |
790 | pub fn splx(x: IntrStatus) -> IntrStatus {
    |        ^^^^

warning: `x86_64` (bin "x86_64") generated 25 warnings (run `cargo fix --bin "x86_64" -p x86_64` to apply 2 suggestions)
    Finished `dev` profile [optimized + debuginfo] target(s) in 14.82s
mb2: header at file offset 0x1008 validated on the stripped ISO kernel (magic/arch/length/checksum/end tag)
iso: module /home/req/code/r9/target/images/x86_64/cpu.paq staged at /boot/bootfs.paq
iso: kernel cmdline append "init=/amd64/init -c" (mb2 tag 1, the bootconf carrier)
xorriso 1.5.8.pl02 : RockRidge filesystem manipulator, libburnia project.

xorriso : NOTE : Environment variable SOURCE_DATE_EPOCH encountered with value 315532800
Drive current: -outdev 'stdio:target/boot-q35cpu.iso'
Media current: stdio file, overwriteable
Media status : is blank
Media summary: 0 sessions, 0 data blocks, 0 data,  365g free
Added to ISO image: directory '/'='/tmp/nix-shell.dxOsuL/grub.KwP3ra'
xorriso : UPDATE :     356 files added in 1 seconds
Added to ISO image: directory '/'='/home/req/code/r9/target/iso-q35cpu'
xorriso : UPDATE :     361 files added in 1 seconds
xorriso : NOTE : Copying to System Area: 512 bytes from file '/nix/store/jniwmlg4zv2r4baccllbizddnh1scfjr-grub-2.14/lib/grub/i386-pc/boot_hybrid.img'
ISO image produced: 19720 sectors
Written to medium : 19720 sectors at LBA 0
Writing to 'stdio:target/boot-q35cpu.iso' completed successfully.

qemu-system-x86_64: -chardev socket,id=cons,path=target/boot-test-q35cpu.sock,server=on,wait=on: info: QEMU waiting for connection on: disconnected:unix:target/boot-test-q35cpu.sock,server=on
boot-test: PASS -- SeaBIOS -> GRUB2 -> mb2 kernel + cpu-module boot with the append seam live: banner, memory facts, discovery, the module/paqfs notes, the exec-entry note, the bootconf-merge line (devenv: bootconf: the cmdline's key=value pairs in the ROOT env seed), boot(8)'s own ring-3 prints, then init -c's cpustart #!-exec'ing the REAL /rc/bin/cpurc — the six-rung fail-loud ladder (pairing -> ether -> keyfs -> authsrv listen -> factotum -> tcp17010 listen) green to cpurc: DONE with the pairing rung (cpurc: PAIR-OK) and ZERO FAIL words, then init's post-cpustart terminal handoff to the `% ` prompt (the retired 19-01 loop-prohibition's expected successor) — CPU-02's service=cpu architecture live from ring 3 on 19-07's §15 image (19-01/19-03/19-08)
boot-test: serial log written to /home/req/code/r9/target/boot-test-q35cpu.log
boot-test: captured 4018 bytes

~~~

### out/profiles/q35exec.log

~~~
    |
778 | pub fn spllo() -> IntrStatus {
    |        ^^^^^

warning: function `splhi` is never used
   --> x86_64/src/trap.rs:784:8
    |
784 | pub fn splhi() -> IntrStatus {
    |        ^^^^^

warning: function `splx` is never used
   --> x86_64/src/trap.rs:790:8
    |
790 | pub fn splx(x: IntrStatus) -> IntrStatus {
    |        ^^^^

warning: `x86_64` (bin "x86_64") generated 12 warnings (run `cargo fix --bin "x86_64" -p x86_64` to apply 1 suggestion)
    Finished `dev` profile [optimized + debuginfo] target(s) in 14.50s
mb2: header at file offset 0x1008 validated on the stripped ISO kernel (magic/arch/length/checksum/end tag)
iso: module /home/req/code/r9/target/images/x86_64/marker.paq staged at /boot/bootfs.paq
xorriso 1.5.8.pl02 : RockRidge filesystem manipulator, libburnia project.

xorriso : NOTE : Environment variable SOURCE_DATE_EPOCH encountered with value 315532800
Drive current: -outdev 'stdio:target/boot-q35exec.iso'
Media current: stdio file, overwriteable
Media status : is blank
Media summary: 0 sessions, 0 data blocks, 0 data,  365g free
Added to ISO image: directory '/'='/tmp/nix-shell.xovB7E/grub.iyANfV'
xorriso : UPDATE :     356 files added in 1 seconds
Added to ISO image: directory '/'='/home/req/code/r9/target/iso-q35exec'
xorriso : UPDATE :     361 files added in 1 seconds
xorriso : NOTE : Copying to System Area: 512 bytes from file '/nix/store/jniwmlg4zv2r4baccllbizddnh1scfjr-grub-2.14/lib/grub/i386-pc/boot_hybrid.img'
ISO image produced: 9018 sectors
Written to medium : 9018 sectors at LBA 0
Writing to 'stdio:target/boot-q35exec.iso' completed successfully.

qemu-system-x86_64: -chardev socket,id=cons,path=target/boot-test-q35exec.sock,server=on,wait=on: info: QEMU waiting for connection on: disconnected:unix:target/boot-test-q35exec.sock,server=on
boot-test: PASS -- SeaBIOS -> GRUB2 -> mb2 kernel + module boot: banner, memory facts, discovery, the module-tag/paqfs mount notes, the exec-entry note, the marker binary's OWN ring-3 prints (notify got-note + interrupted via WRITE syscalls) and the all-exited park observed in order — XEXEC-02 leg 1 (D-03 delivery, D-04 format, D-08 staging, D-09 marker)
boot-test: serial log written to /home/req/code/r9/target/boot-test-q35exec.log
boot-test: captured 2068 bytes

~~~

### out/profiles/q35.log

~~~
   --> x86_64/src/trap.rs:778:8
    |
778 | pub fn spllo() -> IntrStatus {
    |        ^^^^^

warning: function `splhi` is never used
   --> x86_64/src/trap.rs:784:8
    |
784 | pub fn splhi() -> IntrStatus {
    |        ^^^^^

warning: function `splx` is never used
   --> x86_64/src/trap.rs:790:8
    |
790 | pub fn splx(x: IntrStatus) -> IntrStatus {
    |        ^^^^

warning: `x86_64` (bin "x86_64") generated 10 warnings (run `cargo fix --bin "x86_64" -p x86_64` to apply 1 suggestion)
    Finished `dev` profile [optimized + debuginfo] target(s) in 14.74s
mb2: header at file offset 0x1008 validated on the stripped ISO kernel (magic/arch/length/checksum/end tag)
xorriso 1.5.8.pl02 : RockRidge filesystem manipulator, libburnia project.

xorriso : NOTE : Environment variable SOURCE_DATE_EPOCH encountered with value 315532800
Drive current: -outdev 'stdio:target/boot-q35.iso'
Media current: stdio file, overwriteable
Media status : is blank
Media summary: 0 sessions, 0 data blocks, 0 data,  365g free
Added to ISO image: directory '/'='/tmp/nix-shell.xbjetL/grub.TWDoTm'
xorriso : UPDATE :     356 files added in 1 seconds
Added to ISO image: directory '/'='/home/req/code/r9/target/iso-q35'
xorriso : UPDATE :     360 files added in 1 seconds
xorriso : NOTE : Copying to System Area: 512 bytes from file '/nix/store/jniwmlg4zv2r4baccllbizddnh1scfjr-grub-2.14/lib/grub/i386-pc/boot_hybrid.img'
ISO image produced: 8962 sectors
Written to medium : 8962 sectors at LBA 0
Writing to 'stdio:target/boot-q35.iso' completed successfully.

qemu-system-x86_64: -chardev socket,id=cons,path=target/boot-test-q35.sock,server=on,wait=on: info: QEMU waiting for connection on: disconnected:unix:target/boot-test-q35.sock,server=on
boot-test: PASS -- SeaBIOS -> GRUB2 -> mb2 kernel boot: banner, >= 1 q35 memory-map fact line, the discovery markers (MCFG parse, pciexbar audit, device found, route), >= 3 tick lines and the typed echo probe observed in order on the serial socket (X64-01 platform gate + X64-03 discovery contract)
boot-test: serial log written to /home/req/code/r9/target/boot-test-q35.log
boot-test: captured 1582 bytes

~~~

### out/profiles/q35lowmcfg.log

~~~
   --> x86_64/src/trap.rs:778:8
    |
778 | pub fn spllo() -> IntrStatus {
    |        ^^^^^

warning: function `splhi` is never used
   --> x86_64/src/trap.rs:784:8
    |
784 | pub fn splhi() -> IntrStatus {
    |        ^^^^^

warning: function `splx` is never used
   --> x86_64/src/trap.rs:790:8
    |
790 | pub fn splx(x: IntrStatus) -> IntrStatus {
    |        ^^^^

warning: `x86_64` (bin "x86_64") generated 10 warnings (run `cargo fix --bin "x86_64" -p x86_64` to apply 1 suggestion)
    Finished `dev` profile [optimized + debuginfo] target(s) in 14.36s
mb2: header at file offset 0x1008 validated on the stripped ISO kernel (magic/arch/length/checksum/end tag)
xorriso 1.5.8.pl02 : RockRidge filesystem manipulator, libburnia project.

xorriso : NOTE : Environment variable SOURCE_DATE_EPOCH encountered with value 315532800
Drive current: -outdev 'stdio:target/boot-q35lowmcfg.iso'
Media current: stdio file, overwriteable
Media status : is blank
Media summary: 0 sessions, 0 data blocks, 0 data,  365g free
Added to ISO image: directory '/'='/tmp/nix-shell.hn8G1E/grub.ASlsHE'
xorriso : UPDATE :     356 files added in 1 seconds
Added to ISO image: directory '/'='/home/req/code/r9/target/iso-q35lowmcfg'
xorriso : UPDATE :     360 files added in 1 seconds
xorriso : NOTE : Copying to System Area: 512 bytes from file '/nix/store/jniwmlg4zv2r4baccllbizddnh1scfjr-grub-2.14/lib/grub/i386-pc/boot_hybrid.img'
ISO image produced: 8962 sectors
Written to medium : 8962 sectors at LBA 0
Writing to 'stdio:target/boot-q35lowmcfg.iso' completed successfully.

qemu-system-x86_64: -chardev socket,id=cons,path=target/boot-test-q35lowmcfg.sock,server=on,wait=on: info: QEMU waiting for connection on: disconnected:unix:target/boot-test-q35lowmcfg.sock,server=on
boot-test: PASS -- SeaBIOS -> GRUB2 -> mb2 kernel boot: banner, >= 1 q35 memory-map fact line, the discovery markers (MCFG parse, pciexbar audit, device found, route), >= 3 tick lines and the typed echo probe observed in order on the serial socket (X64-01 platform gate + X64-03 discovery contract)
boot-test: serial log written to /home/req/code/r9/target/boot-test-q35lowmcfg.log
boot-test: captured 1582 bytes

~~~

### out/profiles/q35net.log

~~~
   --> x86_64/src/trap.rs:778:8
    |
778 | pub fn spllo() -> IntrStatus {
    |        ^^^^^

warning: function `splhi` is never used
   --> x86_64/src/trap.rs:784:8
    |
784 | pub fn splhi() -> IntrStatus {
    |        ^^^^^

warning: function `splx` is never used
   --> x86_64/src/trap.rs:790:8
    |
790 | pub fn splx(x: IntrStatus) -> IntrStatus {
    |        ^^^^

warning: `x86_64` (bin "x86_64") generated 10 warnings (run `cargo fix --bin "x86_64" -p x86_64` to apply 2 suggestions)
    Finished `dev` profile [optimized + debuginfo] target(s) in 14.80s
mb2: header at file offset 0x1008 validated on the stripped ISO kernel (magic/arch/length/checksum/end tag)
xorriso 1.5.8.pl02 : RockRidge filesystem manipulator, libburnia project.

xorriso : NOTE : Environment variable SOURCE_DATE_EPOCH encountered with value 315532800
Drive current: -outdev 'stdio:target/boot-q35net.iso'
Media current: stdio file, overwriteable
Media status : is blank
Media summary: 0 sessions, 0 data blocks, 0 data,  365g free
Added to ISO image: directory '/'='/tmp/nix-shell.AmnbKr/grub.035wen'
xorriso : UPDATE :     356 files added in 1 seconds
Added to ISO image: directory '/'='/home/req/code/r9/target/iso-q35net'
xorriso : UPDATE :     360 files added in 1 seconds
xorriso : NOTE : Copying to System Area: 512 bytes from file '/nix/store/jniwmlg4zv2r4baccllbizddnh1scfjr-grub-2.14/lib/grub/i386-pc/boot_hybrid.img'
ISO image produced: 8971 sectors
Written to medium : 8971 sectors at LBA 0
Writing to 'stdio:target/boot-q35net.iso' completed successfully.

qemu-system-x86_64: -chardev socket,id=cons,path=target/boot-test-q35net.sock,server=on,wait=on: info: QEMU waiting for connection on: disconnected:unix:target/boot-test-q35net.sock,server=on
boot-test: PASS -- SeaBIOS -> GRUB2 -> mb2 kernel boot: banner, discovery markers, the bind markers (negotiation audit + #l0: + MAC), the kernel-side netgate ping-pong xN/tx-bulk stages and X64-NET-DONE in order; the pcap assertion holds (4352 ARP requests, 4352 gateway replies, 60-byte frames from the guest MAC) — X64-04 closed and X64-02's load proof landed
boot-test: serial log written to /home/req/code/r9/target/boot-test-q35net.log
boot-test: captured 2301 bytes

~~~

### out/profiles/q35shell.log

~~~
    |
778 | pub fn spllo() -> IntrStatus {
    |        ^^^^^

warning: function `splhi` is never used
   --> x86_64/src/trap.rs:784:8
    |
784 | pub fn splhi() -> IntrStatus {
    |        ^^^^^

warning: function `splx` is never used
   --> x86_64/src/trap.rs:790:8
    |
790 | pub fn splx(x: IntrStatus) -> IntrStatus {
    |        ^^^^

warning: `x86_64` (bin "x86_64") generated 25 warnings (run `cargo fix --bin "x86_64" -p x86_64` to apply 2 suggestions)
    Finished `dev` profile [optimized + debuginfo] target(s) in 14.67s
mb2: header at file offset 0x1008 validated on the stripped ISO kernel (magic/arch/length/checksum/end tag)
iso: module /home/req/code/r9/target/images/x86_64/chain.paq staged at /boot/bootfs.paq
xorriso 1.5.8.pl02 : RockRidge filesystem manipulator, libburnia project.

xorriso : NOTE : Environment variable SOURCE_DATE_EPOCH encountered with value 315532800
Drive current: -outdev 'stdio:target/boot-q35shell.iso'
Media current: stdio file, overwriteable
Media status : is blank
Media summary: 0 sessions, 0 data blocks, 0 data,  365g free
Added to ISO image: directory '/'='/tmp/nix-shell.bxK0KY/grub.C50B7b'
xorriso : UPDATE :     356 files added in 1 seconds
Added to ISO image: directory '/'='/home/req/code/r9/target/iso-q35shell'
xorriso : UPDATE :     361 files added in 1 seconds
xorriso : NOTE : Copying to System Area: 512 bytes from file '/nix/store/jniwmlg4zv2r4baccllbizddnh1scfjr-grub-2.14/lib/grub/i386-pc/boot_hybrid.img'
ISO image produced: 9166 sectors
Written to medium : 9166 sectors at LBA 0
Writing to 'stdio:target/boot-q35shell.iso' completed successfully.

qemu-system-x86_64: -chardev socket,id=cons,path=target/boot-test-q35shell.sock,server=on,wait=on: info: QEMU waiting for connection on: disconnected:unix:target/boot-test-q35shell.sock,server=on
boot-test: PASS -- SeaBIOS -> GRUB2 -> mb2 kernel + chain-module boot: banner, memory facts, discovery, the NIC bind, the module-tag/paqfs mount notes, the exec-entry note, then the REAL boot(8)→init→rc chain's own ring-3 prints (paqfs/version handoffs, the init rc handoff) through to the interactive `% ` prompt, observed in order — XEXEC-03 leg 2 (D-08's staged chain on the proven exec substrate; the composed Harvey-side gate is 14-07's)
boot-test: serial log written to /home/req/code/r9/target/boot-test-q35shell.log
boot-test: captured 2652 bytes

~~~

### out/rio/bake.log

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
== PASS: baked /home/req/r9-witness-work/harvey/build/x86_64/out/bootfs-cpu.paq (22002476 bytes; chain + the auth suite + cmd/cpu + aux/listen + the real cpurc + tcp17010 + the font payload + the §15 p9sk1 domain + the 19-10 desktop suite (test/rio/plumber/plumb/ramfs/syscall/awd + glenda's home + the plumb rules); header 529ab12b trailer 6b46e688; KERNDATE 1795382101)

~~~

### out/rio-diag/bake.log

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
== PASS: baked /home/req/r9-witness-work/harvey/build/x86_64/out/bootfs-cpu.paq (22002536 bytes; chain + the auth suite + cmd/cpu + aux/listen + the real cpurc + tcp17010 + the font payload + the §15 p9sk1 domain + the 19-10 desktop suite (test/rio/plumber/plumb/ramfs/syscall/awd + glenda's home + the plumb rules); header 529ab12b trailer 6b46e688; KERNDATE 1795382101)

~~~

### out/rio-diag/rio-diag.log

~~~
churngate: heap exit 4701440 33554432
% rio-gate: no pair after poll 1 — sent the documented manual start (plumber; rio)
process 73 exited: ""
churngate: heap exit 5016384 33554432
process 72 exited: ""
churngate: heap exit 5016384 33554432
devmnt: server error: './dev/label' file does not exist
devmnt: server error: './dev/winname' file does not exist
ls /srv
/srv/boot
/srv/factotum
/srv/plumb.glenda.73
/srv/r9-login-ok
/srv/r9-login-pid.51
/srv/rio.glenda.76
/srv/riowctl.glenda.76
process 82 exited: ""
churngate: heap exit 5890944 33554432
% rio-gate: diag marker — /srv/r9-login-ok and /srv/r9-login-pid.51 present in the serial /srv listing (profile shell pid 51)
RIO-DIAG login-ok
rio-gate: stage ok — serial /srv shows rio and riowctl (D-28 marker)
RIO-SERVING
rio-gate: post-pass ok — zero cpurc: FAIL in the capture
rio-gate: post-pass ok — zero kernel panic markers (drawterm's own sysfatal exit pair excluded)
rio-gate: post-pass ok — the per-bake DEMO password redacted from the retained capture (0 occurrence(s) mapped)
rio-gate: ALL STAGES PASS — cpu boot to cpurc: DONE, encrypted drawterm session, serial /srv shows the rio-diag login marker and rio + riowctl (RIO-SERVING); diagnostic evidence in /home/req/r9-witness-work/out/rio-diag
== rio rio-diag gate PASS — capture retained at /home/req/r9-witness-work/out/rio-diag/rio-gate-rio-diag.log
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

### out/rio-diag/rio-gate-rio-diag.log

~~~

churngate: heap exit 4701440 33554432

ls /srv

/srv/boot
/srv/factotum
/srv/r9-login-ok
/srv/r9-login-pid.51
process 71 exited: ""

churngate: heap exit 4701440 33554432

% process 73 exited: ""

churngate: heap exit 5016384 33554432

process 72 exited: ""

churngate: heap exit 5016384 33554432

devmnt: server error: './dev/label' file does not exist

devmnt: server error: './dev/winname' file does not exist

ls /srv

/srv/boot
/srv/factotum
/srv/plumb.glenda.73
/srv/r9-login-ok
/srv/r9-login-pid.51
/srv/rio.glenda.76
/srv/riowctl.glenda.76
process 82 exited: ""

churngate: heap exit 5890944 33554432

% 
rio-gate: redaction contract held — <demo-pw-redacted> (0 occurrence(s) of the per-bake DEMO password mapped; T-01-30)

~~~

### out/rio/rio-gate-rio.log

~~~

churngate: heap exit 4693248 33554432

process 68 exited: ""

churngate: heap exit 4693248 33554432

ls /srv

/srv/boot
/srv/factotum
process 69 exited: ""

churngate: heap exit 4693248 33554432

% process 71 exited: ""

churngate: heap exit 5008320 33554432

process 70 exited: ""

churngate: heap exit 5008320 33554432

devmnt: server error: './dev/label' file does not exist

devmnt: server error: './dev/winname' file does not exist

ls /srv

/srv/boot
/srv/factotum
/srv/plumb.glenda.71
/srv/rio.glenda.74
/srv/riowctl.glenda.74
process 80 exited: ""

churngate: heap exit 5882752 33554432

% 
rio-gate: redaction contract held — <demo-pw-redacted> (0 occurrence(s) of the per-bake DEMO password mapped; T-01-30)

~~~

### out/rio/rio.log

~~~
ls /srv
/srv/boot
/srv/factotum
process 69 exited: ""
churngate: heap exit 4693248 33554432
% rio-gate: no pair after poll 1 — sent the documented manual start (plumber; rio)
process 71 exited: ""
churngate: heap exit 5008320 33554432
process 70 exited: ""
churngate: heap exit 5008320 33554432
devmnt: server error: './dev/label' file does not exist
devmnt: server error: './dev/winname' file does not exist
ls /srv
/srv/boot
/srv/factotum
/srv/plumb.glenda.71
/srv/rio.glenda.74
/srv/riowctl.glenda.74
process 80 exited: ""
churngate: heap exit 5882752 33554432
% rio-gate: stage ok — serial /srv shows rio and riowctl (D-28 marker)
RIO-SERVING
rio-gate: post-pass ok — zero cpurc: FAIL in the capture
rio-gate: post-pass ok — zero kernel panic markers (drawterm's own sysfatal exit pair excluded)
rio-gate: post-pass ok — the per-bake DEMO password redacted from the retained capture (0 occurrence(s) mapped)
rio-gate: ALL STAGES PASS — cpu boot to cpurc: DONE, encrypted drawterm session, serial /srv shows rio + riowctl (RIO-SERVING)
== rio rio gate PASS — capture retained at /home/req/r9-witness-work/out/rio/rio-gate-rio.log
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

### out/trees/trees.log

~~~
these 3 derivations will be built:
  /nix/store/iycgzwqgmsi85wm7x3qqhsqln7cj4h09-harvey-hostbin-0.drv
  /nix/store/pmgg2zfrx68qdq9ds7h9940angvd0msa-harvey-tree-amd64-0.drv
  /nix/store/w8a148l5wgz73fpz05kmzgqw6kx0c89q-harvey-tree-arm64-0.drv
this path will be fetched (152.0 KiB download, 439.9 KiB unpacked):
  /nix/store/pa0pzpgmzancbxqhxl6fr1k0ix1xd5y1-proot-5.5.0
copying path '/nix/store/pa0pzpgmzancbxqhxl6fr1k0ix1xd5y1-proot-5.5.0' from 'https://cache.nixos.org'...
building '/nix/store/iycgzwqgmsi85wm7x3qqhsqln7cj4h09-harvey-hostbin-0.drv'...
building '/nix/store/pmgg2zfrx68qdq9ds7h9940angvd0msa-harvey-tree-amd64-0.drv'...
building '/nix/store/w8a148l5wgz73fpz05kmzgqw6kx0c89q-harvey-tree-arm64-0.drv'...
TREES-STORE /nix/store/arrvrysn8l6bagwv95nbzlng8s1y7wyf-harvey-tree-amd64-0
--- /nix/store/arrvrysn8l6bagwv95nbzlng8s1y7wyf-harvey-tree-amd64-0/HARVEY-TREE ---
  rev=bd7a6982a3e4f465f8ed3da5e0c7290521514ff0
  narHash=sha256-N6VjnVI69pcxSCpQ9BUxrlpTqnp9LgjKfGcpO3F6Ma0=
  arch=amd64
  system=x86_64-linux
checking '/nix/store/arrvrysn8l6bagwv95nbzlng8s1y7wyf-harvey-tree-amd64-0'...
TREES-VERIFY-OK /nix/store/arrvrysn8l6bagwv95nbzlng8s1y7wyf-harvey-tree-amd64-0
TREES-STORE /nix/store/5rq6h22qxhifg6v78q7pf4bbphc9mnjp-harvey-tree-arm64-0
--- /nix/store/5rq6h22qxhifg6v78q7pf4bbphc9mnjp-harvey-tree-arm64-0/HARVEY-TREE ---
  rev=bd7a6982a3e4f465f8ed3da5e0c7290521514ff0
  narHash=sha256-N6VjnVI69pcxSCpQ9BUxrlpTqnp9LgjKfGcpO3F6Ma0=
  arch=arm64
  system=x86_64-linux
checking '/nix/store/5rq6h22qxhifg6v78q7pf4bbphc9mnjp-harvey-tree-arm64-0'...
TREES-VERIFY-OK /nix/store/5rq6h22qxhifg6v78q7pf4bbphc9mnjp-harvey-tree-arm64-0

~~~

