# r9 pass-2 witness return (plans 01-09 / 01-32)

- date: 2026-11-23T15:24:06Z
- git HEAD: 8417a847cede1af06631b480188aaa55142b5989
- r9 root: /home/req/code/r9
- work root: /home/req/r9-witness-work
- steps: trees kernel gate gate_neg probe flipback profiles cpuauth_ref

## out/env.txt

~~~
# r9 pass-2 witness preflight
date=2026-11-23T15:12:45Z
user=req uid=1000
r9_root=/home/req/code/r9
git_head=8417a847cede1af06631b480188aaa55142b5989
work_dir=/home/req/r9-witness-work
steps=trees kernel gate gate_neg probe flipback profiles cpuauth_ref
harvey_pin=bd7a6982a3e4f465f8ed3da5e0c7290521514ff0
kerndate=1795382101
uname=Linux teeniebox 7.4.0-rc3 #1-NixOS SMP PREEMPT_DYNAMIC @315532800 x86_64 GNU/Linux
nproc=16
               total        used        free      shared  buff/cache   available
Mem:            30Gi       1.6Gi        28Gi       382Mi       1.9Gi        29Gi
Swap:          7.5Gi          0B       7.5Gi
--- free disk under /home/req/r9-witness-work ---
Filesystem        Size  Used Avail Use% Mounted on
/dev/mapper/root  946G  576G  367G  62% /
--- nix ---
nix (Nix) 2.35.6
experimental-features = dynamic-derivations fetch-closure fetch-tree flakes nix-command
sandbox = true
--- /dev/kvm ---
DEV_KVM=writable
crw-rw-rw- 1 root kvm 10, 232 Nov 23 10:02 /dev/kvm

~~~

## out/summary.tsv

~~~
trees	0	258
kernel	1	31
gate	1	87
gate_neg	1	34
probe	1	50
flipback	1	48
profiles	1	152
cpuauth_ref	1	20

~~~

## out/survivors.txt

~~~
# surviving qemu/Xvfb/drawterm processes owned by req
# at 2026-11-23T15:24:06Z
none

~~~

## out/secret-scan.txt

~~~
# credential scan of out/ at 2026-11-23T15:24:06Z
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
image: x86_64/auth /home/req/code/r9/target/images/x86_64/auth.paq sha256 a60482fe4485feacda2a8afa3c8c1d8f7b30237d46df097d46aafffd6332aa25 members 95
reference member list /nix/store/mxx5qyijchxaff7cg61y4nwvjvjbbmi4-harvey-reference-amd64-0/auth.members: No such file or directory (os error 2) — R9_HARVEY_REFERENCE_AMD64 names the reference bake's directory
CPUAUTH-REF auth exit=nonzero

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
    Finished `dev` profile [optimized + debuginfo] target(s) in 14.70s
mb2: header at file offset 0x1008 validated on the stripped ISO kernel (magic/arch/length/checksum/end tag)
iso: module /home/req/r9-witness-work/r9-probe-flipback/target/images/x86_64/cpu.paq staged at /boot/bootfs.paq
iso: kernel cmdline append "init=/amd64/init -c" (mb2 tag 1, the bootconf carrier)
xorriso 1.5.8.pl02 : RockRidge filesystem manipulator, libburnia project.

xorriso : NOTE : Environment variable SOURCE_DATE_EPOCH encountered with value 315532800
Drive current: -outdev 'stdio:target/boot-q35cpu.iso'
Media current: stdio file, overwriteable
Media status : is blank
Media summary: 0 sessions, 0 data blocks, 0 data,  365g free
Added to ISO image: directory '/'='/tmp/nix-shell.0i4gFN/grub.zGL9CC'
xorriso : UPDATE :     356 files added in 1 seconds
Added to ISO image: directory '/'='/home/req/r9-witness-work/r9-probe-flipback/target/iso-q35cpu'
xorriso : UPDATE :     361 files added in 1 seconds
xorriso : NOTE : Copying to System Area: 512 bytes from file '/nix/store/jniwmlg4zv2r4baccllbizddnh1scfjr-grub-2.14/lib/grub/i386-pc/boot_hybrid.img'
ISO image produced: 19721 sectors
Written to medium : 19721 sectors at LBA 0
Writing to 'stdio:target/boot-q35cpu.iso' completed successfully.

gate: iso /home/req/r9-witness-work/r9-probe-flipback/target/boot-q35cpu.iso 
gate: leg rio (capture /home/req/r9-witness-work/r9-probe-flipback/target/gate/120659-1795447239/rio.log)
can't read "spawn_id": no such variable
    while executing
"expect_before -i $spawn_id {
	-re {panic|Unhandled interrupt} {
		puts stderr "drawterm-gate: kernel panic marker on the wire — aborting"
		cleanup
..."
    (file "/home/req/r9-witness-work/r9-probe-flipback/gates/x86_64/drawterm-gate.expect" line 336)
GATE-FAIL rio-leg-failed (leg rio; capture /home/req/r9-witness-work/r9-probe-flipback/target/gate/120659-1795447239/rio.log)

~~~

### out/gate/gate.log

~~~

warning: function `splx` is never used
   --> x86_64/src/trap.rs:790:8
    |
790 | pub fn splx(x: IntrStatus) -> IntrStatus {
    |        ^^^^

warning: `x86_64` (bin "x86_64") generated 25 warnings (run `cargo fix --bin "x86_64" -p x86_64` to apply 2 suggestions)
    Finished `dev` profile [optimized + debuginfo] target(s) in 14.05s
mb2: header at file offset 0x1008 validated on the stripped ISO kernel (magic/arch/length/checksum/end tag)
iso: module /home/req/code/r9/target/images/x86_64/cpu.paq staged at /boot/bootfs.paq
iso: kernel cmdline append "init=/amd64/init -c" (mb2 tag 1, the bootconf carrier)
xorriso 1.5.8.pl02 : RockRidge filesystem manipulator, libburnia project.

xorriso : NOTE : Environment variable SOURCE_DATE_EPOCH encountered with value 315532800
Drive current: -outdev 'stdio:target/boot-q35cpu.iso'
Media current: stdio file, overwriteable
Media status : is blank
Media summary: 0 sessions, 0 data blocks, 0 data,  367g free
Added to ISO image: directory '/'='/tmp/nix-shell.174IuM/grub.3z5SL0'
xorriso : UPDATE :     356 files added in 1 seconds
Added to ISO image: directory '/'='/home/req/code/r9/target/iso-q35cpu'
xorriso : UPDATE :     361 files added in 1 seconds
xorriso : NOTE : Copying to System Area: 512 bytes from file '/nix/store/jniwmlg4zv2r4baccllbizddnh1scfjr-grub-2.14/lib/grub/i386-pc/boot_hybrid.img'
ISO image produced: 19721 sectors
Written to medium : 19721 sectors at LBA 0
Writing to 'stdio:target/boot-q35cpu.iso' completed successfully.

gate: iso /home/req/code/r9/target/boot-q35cpu.iso 
gate: leg rio (capture /home/req/code/r9/target/gate/88271-1795447108/rio.log)
can't read "spawn_id": no such variable
    while executing
"expect_before -i $spawn_id {
	-re {panic|Unhandled interrupt} {
		puts stderr "drawterm-gate: kernel panic marker on the wire — aborting"
		cleanup
..."
    (file "/home/req/code/r9/gates/x86_64/drawterm-gate.expect" line 336)
GATE-FAIL rio-leg-failed (leg rio; capture /home/req/code/r9/target/gate/88271-1795447108/rio.log)
GATE-RUN 2 exit=nonzero

~~~

### out/gate-neg/gate-neg.log

~~~

warning: function `splx` is never used
   --> x86_64/src/trap.rs:790:8
    |
790 | pub fn splx(x: IntrStatus) -> IntrStatus {
    |        ^^^^

warning: `x86_64` (bin "x86_64") generated 25 warnings (run `cargo fix --bin "x86_64" -p x86_64` to apply 2 suggestions)
    Finished `dev` profile [optimized + debuginfo] target(s) in 13.62s
mb2: header at file offset 0x1008 validated on the stripped ISO kernel (magic/arch/length/checksum/end tag)
iso: module /home/req/code/r9/target/images/x86_64/cpu.paq staged at /boot/bootfs.paq
iso: kernel cmdline append "init=/amd64/init -c" (mb2 tag 1, the bootconf carrier)
xorriso 1.5.8.pl02 : RockRidge filesystem manipulator, libburnia project.

xorriso : NOTE : Environment variable SOURCE_DATE_EPOCH encountered with value 315532800
Drive current: -outdev 'stdio:target/boot-q35cpu.iso'
Media current: stdio file, overwriteable
Media status : is blank
Media summary: 0 sessions, 0 data blocks, 0 data,  366g free
Added to ISO image: directory '/'='/tmp/nix-shell.GdEgr1/grub.7etY9S'
xorriso : UPDATE :     356 files added in 1 seconds
Added to ISO image: directory '/'='/home/req/code/r9/target/iso-q35cpu'
xorriso : UPDATE :     361 files added in 1 seconds
xorriso : NOTE : Copying to System Area: 512 bytes from file '/nix/store/jniwmlg4zv2r4baccllbizddnh1scfjr-grub-2.14/lib/grub/i386-pc/boot_hybrid.img'
ISO image produced: 19721 sectors
Written to medium : 19721 sectors at LBA 0
Writing to 'stdio:target/boot-q35cpu.iso' completed successfully.

gate: iso /home/req/code/r9/target/boot-q35cpu.iso 
gate: leg force-no-rio (capture /home/req/code/r9/target/gate/97884-1795447142/force-no-rio.log)
can't read "spawn_id": no such variable
    while executing
"expect_before -i $spawn_id {
	-re {panic|Unhandled interrupt} {
		puts stderr "drawterm-gate: kernel panic marker on the wire — aborting"
		cleanup
..."
    (file "/home/req/code/r9/gates/x86_64/drawterm-gate.expect" line 336)
GATE-FAIL force-no-rio-leg-failed (leg force-no-rio; capture /home/req/code/r9/target/gate/97884-1795447142/force-no-rio.log)
GATE-NEG-WRONG-FAILURE (GATE-FAIL rio-not-serving absent)

~~~

### out/kernel/kernel.log

~~~
this derivation will be built:
  /nix/store/7dvc4brcrqa2ak5mndqidrvz3304njgh-r9-kernel-x64-8417a84.drv
building '/nix/store/7dvc4brcrqa2ak5mndqidrvz3304njgh-r9-kernel-x64-8417a84.drv'...
KERNEL-STORE /nix/store/2279qlywyhk9zfya9qjaqn004vj95jl6-r9-kernel-x64-8417a84
3535848bdab363f226360c172caf95dc699a493e2c7a9ebf6af68ad10cb77ad4  /nix/store/2279qlywyhk9zfya9qjaqn004vj95jl6-r9-kernel-x64-8417a84/kernel-x64.elf
this derivation will be built:
  /nix/store/832rq51gnqzbzawl7yhk3dxj0bq03302-r9-boot-iso-8417a84.drv
error: some outputs of '/nix/store/832rq51gnqzbzawl7yhk3dxj0bq03302-r9-boot-iso-8417a84.drv' are not valid, so checking is not possible
       Hint: --rebuild and --check error if the derivation was not previously built and cannot be substituted.
             Remove it to perform a fresh build, or use --repair to rewrite missing or corrupted builds in the store.
KERNEL-BOOTISO-REBUILD-FAIL

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
    Finished `dev` profile [optimized + debuginfo] target(s) in 14.60s
mb2: header at file offset 0x1008 validated on the stripped ISO kernel (magic/arch/length/checksum/end tag)
iso: module /home/req/r9-witness-work/r9-probe-flip/target/images/x86_64/cpu.paq staged at /boot/bootfs.paq
iso: kernel cmdline append "init=/amd64/init -c" (mb2 tag 1, the bootconf carrier)
xorriso 1.5.8.pl02 : RockRidge filesystem manipulator, libburnia project.

xorriso : NOTE : Environment variable SOURCE_DATE_EPOCH encountered with value 315532800
Drive current: -outdev 'stdio:target/boot-q35cpu.iso'
Media current: stdio file, overwriteable
Media status : is blank
Media summary: 0 sessions, 0 data blocks, 0 data,  366g free
Added to ISO image: directory '/'='/tmp/nix-shell.6C2KC4/grub.e48v3C'
xorriso : UPDATE :     356 files added in 1 seconds
Added to ISO image: directory '/'='/home/req/r9-witness-work/r9-probe-flip/target/iso-q35cpu'
xorriso : UPDATE :     361 files added in 1 seconds
xorriso : NOTE : Copying to System Area: 512 bytes from file '/nix/store/jniwmlg4zv2r4baccllbizddnh1scfjr-grub-2.14/lib/grub/i386-pc/boot_hybrid.img'
ISO image produced: 19704 sectors
Written to medium : 19704 sectors at LBA 0
Writing to 'stdio:target/boot-q35cpu.iso' completed successfully.

gate: iso /home/req/r9-witness-work/r9-probe-flip/target/boot-q35cpu.iso 
gate: leg rio (capture /home/req/r9-witness-work/r9-probe-flip/target/gate/107501-1795447192/rio.log)
can't read "spawn_id": no such variable
    while executing
"expect_before -i $spawn_id {
	-re {panic|Unhandled interrupt} {
		puts stderr "drawterm-gate: kernel panic marker on the wire — aborting"
		cleanup
..."
    (file "/home/req/r9-witness-work/r9-probe-flip/gates/x86_64/drawterm-gate.expect" line 336)
GATE-FAIL rio-leg-failed (leg rio; capture /home/req/r9-witness-work/r9-probe-flip/target/gate/107501-1795447192/rio.log)

~~~

### out/profiles/profiles.log

~~~
PROFILE q35 exit=0
PROFILE q35lowmcfg exit=0
PROFILE q35net exit=0
PROFILE q35exec exit=0
PROFILE q35churn exit=0
PROFILE q35shell exit=0
PROFILE q35churnshell exit=nonzero
PROFILE q35cpu exit=0
PROFILE q35auth exit=0

~~~

### out/profiles/q35auth.log

~~~
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
    Finished `dev` profile [optimized + debuginfo] target(s) in 15.48s
mb2: header at file offset 0x1008 validated on the stripped ISO kernel (magic/arch/length/checksum/end tag)
xtask: image x86_64/substrate missing or stale — assembling
image: x86_64/substrate /home/req/code/r9/target/images/x86_64/substrate.paq sha256 3d9e31f905a7115f74ecd07964ff4d74e48d937135ebad54611eca9d821969d7 members 27
iso: module /home/req/code/r9/target/images/x86_64/substrate.paq staged at /boot/bootfs.paq
xorriso 1.5.8.pl02 : RockRidge filesystem manipulator, libburnia project.

xorriso : NOTE : Environment variable SOURCE_DATE_EPOCH encountered with value 315532800
Drive current: -outdev 'stdio:target/boot-q35auth.iso'
Media current: stdio file, overwriteable
Media status : is blank
Media summary: 0 sessions, 0 data blocks, 0 data,  365g free
Added to ISO image: directory '/'='/tmp/nix-shell.yScaoX/grub.KW1L9x'
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
    Finished `dev` profile [optimized + debuginfo] target(s) in 14.80s
mb2: header at file offset 0x1008 validated on the stripped ISO kernel (magic/arch/length/checksum/end tag)
iso: module /home/req/code/r9/target/images/x86_64/marker.paq staged at /boot/bootfs.paq
xorriso 1.5.8.pl02 : RockRidge filesystem manipulator, libburnia project.

xorriso : NOTE : Environment variable SOURCE_DATE_EPOCH encountered with value 315532800
Drive current: -outdev 'stdio:target/boot-q35churn.iso'
Media current: stdio file, overwriteable
Media status : is blank
Media summary: 0 sessions, 0 data blocks, 0 data,  365g free
Added to ISO image: directory '/'='/tmp/nix-shell.yYvP6l/grub.j4RRkj'
xorriso : UPDATE :     356 files added in 1 seconds
Added to ISO image: directory '/'='/home/req/code/r9/target/iso-q35churn'
xorriso : UPDATE :     361 files added in 1 seconds
xorriso : NOTE : Copying to System Area: 512 bytes from file '/nix/store/jniwmlg4zv2r4baccllbizddnh1scfjr-grub-2.14/lib/grub/i386-pc/boot_hybrid.img'
ISO image produced: 10799 sectors
Written to medium : 10799 sectors at LBA 0
Writing to 'stdio:target/boot-q35churn.iso' completed successfully.

qemu-system-x86_64: -chardev socket,id=cons,path=target/boot-test-q35churn.sock,server=on,wait=on: info: QEMU waiting for connection on: disconnected:unix:target/boot-test-q35churn.sock,server=on
boot-test: PASS -- SeaBIOS -> GRUB2 -> mb2 kernel + module boot: banner, memory facts, discovery, the module-tag/paqfs mount notes, then the kernel-context churn driver's 1400 exec iterations (1361 churnmark ring-3 prints counted) + 39 RFMEM contention generations, drain, and the heap slope held: steady=466 measured=128 <= ceiling=1772 bytes/exec (host re-derived from the kernel's own samples) — D-06 tier (a) green, CR-03's in-kernel contention leg landed
boot-test: serial log written to /home/req/code/r9/target/boot-test-q35churn.log
boot-test: captured 124257 bytes

~~~

### out/profiles/q35churnshell.log

~~~
q35: acpi rsdt 0x3ffe2345 via mb2 acpi tag (5 entries)
q35: mcfg 0xb0000000 buses 0x0-0xff (seg 0)
q35: lapic 0xfee00000 (madt local intc)
q35: ioapic 0xfec00000 gsib 0 (id 0)
q35: intsrc bus 0 source 0 -> gsi 2 flags 0x0
q35: intsrc bus 0 source 5 -> gsi 5 flags 0xd
q35: intsrc bus 0 source 9 -> gsi 9 flags 0xd
q35: intsrc bus 0 source 10 -> gsi 10 flags 0xd
q35: intsrc bus 0 source 11 -> gsi 11 flags 0xd
q35: hpet 0xfed00000
pciexbar: 0xb0000001 -> 0xb0000001
pci: virtio-net 1af4:1041 at 00:02.0 rev 01 pin 1 (inta) bars 2
pci: inta -> gsi 22 (q35 root slot 2, pirq G: 16+6)
Memory usage:
  Used:		0x00000006c00000
  Total:	0x0000003ffdf000
random: source=rdrand
lapic: cpuid leaf-1 ecx: x2apic=true tsc-deadline=true
lapic: x2apic live (msr 0x800 map; lint0/1 masked, no pic)
timer: 2496091280 Hz (pit 1193182 Hz reference), TscDeadline mode, armed
ioapic: id 0 version 0x11 rtes 23 gsib 0 (madt 0xfec00000)
pci: bar4 pa 0xfe000000 size 0x4000 (firmware) readback 0xfe000000
pci: caps common@0x0+0x1000 notify@0x3000 isr@0x1000 device@0x2000 mult 4
virtio-net: features offered 0x10130bf8024 negotiated 0x100010020
virtio-net: rx pre-posted 128 buffers qsize 256
ioapic: gsi 22 -> vector 0x21 (level)
#l0: virtio: 1000Mbps: 52:54:00:12:34:56
fpu: kernel-SIMD selftest ok (user state parked and restored across the section; xor_fold16 0x1983c6d50a18d707c9d74d9a42c59f27)
q35: module @0x14c000 +0x5f1e7 (mb2 tag 3, stashed)
devenv: seeded cputype=amd64 nobootprompt=embed readparts=0
q35: paqfs root mounted from module (389607 bytes)
devroot: /boot/boot open (159349 bytes)
exec: entering ring 3 pc=0x2054fa sp=0x7fffffffef78 path=/boot/boot argv=[boot]
paqfs...version...chain.stage: Mon Nov 23 15:22:57 GMT 2026
fingerprint: 61e4919df9f20e803e13ef682bf1217d12df8500
process 2 exited: ""
churngate: heap exit 96512 33554432
syscall 10 unimplemented
--- end capture (2505 bytes) ---
boot-test: >= 3 lines beginning "tick" not observed (negative scan hit "unimplemented syscall")

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
    Finished `dev` profile [optimized + debuginfo] target(s) in 14.68s
mb2: header at file offset 0x1008 validated on the stripped ISO kernel (magic/arch/length/checksum/end tag)
iso: module /home/req/code/r9/target/images/x86_64/cpu.paq staged at /boot/bootfs.paq
iso: kernel cmdline append "init=/amd64/init -c" (mb2 tag 1, the bootconf carrier)
xorriso 1.5.8.pl02 : RockRidge filesystem manipulator, libburnia project.

xorriso : NOTE : Environment variable SOURCE_DATE_EPOCH encountered with value 315532800
Drive current: -outdev 'stdio:target/boot-q35cpu.iso'
Media current: stdio file, overwriteable
Media status : is blank
Media summary: 0 sessions, 0 data blocks, 0 data,  365g free
Added to ISO image: directory '/'='/tmp/nix-shell.QsPqKn/grub.DMJu4X'
xorriso : UPDATE :     356 files added in 1 seconds
Added to ISO image: directory '/'='/home/req/code/r9/target/iso-q35cpu'
xorriso : UPDATE :     361 files added in 1 seconds
xorriso : NOTE : Copying to System Area: 512 bytes from file '/nix/store/jniwmlg4zv2r4baccllbizddnh1scfjr-grub-2.14/lib/grub/i386-pc/boot_hybrid.img'
ISO image produced: 19721 sectors
Written to medium : 19721 sectors at LBA 0
Writing to 'stdio:target/boot-q35cpu.iso' completed successfully.

qemu-system-x86_64: -chardev socket,id=cons,path=target/boot-test-q35cpu.sock,server=on,wait=on: info: QEMU waiting for connection on: disconnected:unix:target/boot-test-q35cpu.sock,server=on
boot-test: PASS -- SeaBIOS -> GRUB2 -> mb2 kernel + cpu-module boot with the append seam live: banner, memory facts, discovery, the module/paqfs notes, the exec-entry note, the bootconf-merge line (devenv: bootconf: the cmdline's key=value pairs in the ROOT env seed), boot(8)'s own ring-3 prints, then init -c's cpustart #!-exec'ing the REAL /rc/bin/cpurc — the six-rung fail-loud ladder (pairing -> ether -> keyfs -> authsrv listen -> factotum -> tcp17010 listen) green to cpurc: DONE with the pairing rung (cpurc: PAIR-OK) and ZERO FAIL words, then init's post-cpustart terminal handoff to the `% ` prompt (the retired 19-01 loop-prohibition's expected successor) — CPU-02's service=cpu architecture live from ring 3 on 19-07's §15 image (19-01/19-03/19-08)
boot-test: serial log written to /home/req/code/r9/target/boot-test-q35cpu.log
boot-test: captured 3969 bytes

~~~

### out/profiles/q35exec.log

~~~
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
    Finished `dev` profile [optimized + debuginfo] target(s) in 14.86s
mb2: header at file offset 0x1008 validated on the stripped ISO kernel (magic/arch/length/checksum/end tag)
xtask: image x86_64/marker missing or stale — assembling
image: x86_64/marker /home/req/code/r9/target/images/x86_64/marker.paq sha256 9d9f8894b841f0ec8180dbd5af253d60b509447ed551c3e463a4858e6af4a6b5 members 30
iso: module /home/req/code/r9/target/images/x86_64/marker.paq staged at /boot/bootfs.paq
xorriso 1.5.8.pl02 : RockRidge filesystem manipulator, libburnia project.

xorriso : NOTE : Environment variable SOURCE_DATE_EPOCH encountered with value 315532800
Drive current: -outdev 'stdio:target/boot-q35exec.iso'
Media current: stdio file, overwriteable
Media status : is blank
Media summary: 0 sessions, 0 data blocks, 0 data,  365g free
Added to ISO image: directory '/'='/tmp/nix-shell.ga4ro7/grub.AVmhS3'
xorriso : UPDATE :     356 files added in 1 seconds
Added to ISO image: directory '/'='/home/req/code/r9/target/iso-q35exec'
xorriso : UPDATE :     361 files added in 1 seconds
xorriso : NOTE : Copying to System Area: 512 bytes from file '/nix/store/jniwmlg4zv2r4baccllbizddnh1scfjr-grub-2.14/lib/grub/i386-pc/boot_hybrid.img'
ISO image produced: 9019 sectors
Written to medium : 9019 sectors at LBA 0
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
    Finished `dev` profile [optimized + debuginfo] target(s) in 14.43s
mb2: header at file offset 0x1008 validated on the stripped ISO kernel (magic/arch/length/checksum/end tag)
xorriso 1.5.8.pl02 : RockRidge filesystem manipulator, libburnia project.

xorriso : NOTE : Environment variable SOURCE_DATE_EPOCH encountered with value 315532800
Drive current: -outdev 'stdio:target/boot-q35.iso'
Media current: stdio file, overwriteable
Media status : is blank
Media summary: 0 sessions, 0 data blocks, 0 data,  365g free
Added to ISO image: directory '/'='/tmp/nix-shell.6SJEk2/grub.AyrVsL'
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
    Finished `dev` profile [optimized + debuginfo] target(s) in 14.39s
mb2: header at file offset 0x1008 validated on the stripped ISO kernel (magic/arch/length/checksum/end tag)
xorriso 1.5.8.pl02 : RockRidge filesystem manipulator, libburnia project.

xorriso : NOTE : Environment variable SOURCE_DATE_EPOCH encountered with value 315532800
Drive current: -outdev 'stdio:target/boot-q35lowmcfg.iso'
Media current: stdio file, overwriteable
Media status : is blank
Media summary: 0 sessions, 0 data blocks, 0 data,  365g free
Added to ISO image: directory '/'='/tmp/nix-shell.SsHyJz/grub.FnBhBT'
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
    Finished `dev` profile [optimized + debuginfo] target(s) in 14.43s
mb2: header at file offset 0x1008 validated on the stripped ISO kernel (magic/arch/length/checksum/end tag)
xorriso 1.5.8.pl02 : RockRidge filesystem manipulator, libburnia project.

xorriso : NOTE : Environment variable SOURCE_DATE_EPOCH encountered with value 315532800
Drive current: -outdev 'stdio:target/boot-q35net.iso'
Media current: stdio file, overwriteable
Media status : is blank
Media summary: 0 sessions, 0 data blocks, 0 data,  365g free
Added to ISO image: directory '/'='/tmp/nix-shell.PSWcqX/grub.nOPL5O'
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
boot-test: captured 2300 bytes

~~~

### out/profiles/q35shell.log

~~~
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
    Finished `dev` profile [optimized + debuginfo] target(s) in 14.35s
mb2: header at file offset 0x1008 validated on the stripped ISO kernel (magic/arch/length/checksum/end tag)
xtask: image x86_64/chain missing or stale — assembling
image: x86_64/chain /home/req/code/r9/target/images/x86_64/chain.paq sha256 d07209e4df8da591af199922e01f21eb8af03b2445a48afb3d2ab3c176d61007 members 49
iso: module /home/req/code/r9/target/images/x86_64/chain.paq staged at /boot/bootfs.paq
xorriso 1.5.8.pl02 : RockRidge filesystem manipulator, libburnia project.

xorriso : NOTE : Environment variable SOURCE_DATE_EPOCH encountered with value 315532800
Drive current: -outdev 'stdio:target/boot-q35shell.iso'
Media current: stdio file, overwriteable
Media status : is blank
Media summary: 0 sessions, 0 data blocks, 0 data,  365g free
Added to ISO image: directory '/'='/tmp/nix-shell.b0x0xS/grub.yta6JG'
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
boot-test: captured 2626 bytes

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

