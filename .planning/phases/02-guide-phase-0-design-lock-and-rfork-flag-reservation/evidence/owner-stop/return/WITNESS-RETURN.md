# r9 pass-2 witness return (plans 01-09 / 01-32)

- date: 2026-11-24T03:42:10Z
- git HEAD: 4624ea2f0cb45c81870dcc881d544d7d36a2e0bf
- r9 root: /home/req/code/r9
- work root: /home/req/r9-witness-work
- steps: full

## out/env.txt

~~~
# r9 pass-2 witness preflight
date=2026-11-24T03:39:33Z
user=req uid=1000
r9_root=/home/req/code/r9
git_head=4624ea2f0cb45c81870dcc881d544d7d36a2e0bf
work_dir=/home/req/r9-witness-work
steps=full
harvey_pin=bd7a6982a3e4f465f8ed3da5e0c7290521514ff0
kerndate=1795382101
uname=Linux teeniebox 7.4.0-rc3 #1-NixOS SMP PREEMPT_DYNAMIC @315532800 x86_64 GNU/Linux
nproc=16
               total        used        free      shared  buff/cache   available
Mem:            30Gi       3.1Gi        14Gi       723Mi        14Gi        27Gi
Swap:          7.5Gi          0B       7.5Gi
--- free disk under /home/req/r9-witness-work ---
Filesystem        Size  Used Avail Use% Mounted on
/dev/mapper/root  946G  579G  363G  62% /
--- nix ---
nix (Nix) 2.35.6
experimental-features = dynamic-derivations fetch-closure fetch-tree flakes nix-command
sandbox = true
--- /dev/kvm ---
DEV_KVM=writable
crw-rw-rw- 1 root kvm 10, 232 Nov 23 22:27 /dev/kvm

~~~

## out/summary.tsv

~~~
full	0	156

~~~

## out/survivors.txt

~~~
# surviving qemu/Xvfb/drawterm processes owned by req
# at 2026-11-24T03:42:09Z
none

~~~

## out/secret-scan.txt

~~~
# credential scan of out/ at 2026-11-24T03:42:09Z
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
image: x86_64/auth /home/req/code/r9/target/images/x86_64/auth.paq sha256 dfcc9734c4c9e5bc7ac48a920e28d960f5c9e72a0c164aa82cd5be0d2843c259 members 95
image: x86_64/auth matches the reference member list (95 members)
CPUAUTH-REF auth exit=0

~~~

### out/flipback/flipback.log

~~~
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
% GATE-PASS control-no-rio
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
drawterm-gate: spawned drawterm on a pty (mode control-noG, argv0 tag r9x64dtgate-dt-320886-1795453021, user glenda)
drawterm-gate: feed ok — dt-password
drawterm-gate: stage ok — session-prompt (the remote interactive shell)
drawterm-gate: stage ok — serial console rc prompt (init's handoff after cpurc: DONE)
drawterm-gate: stage ok — rio budget elapsed (60 s)
drawterm-gate: stage ok — serial /srv shows neither rio nor riowctl (the negative control)
drawterm-gate: post-pass ok — zero cpurc: FAIL in the capture
drawterm-gate: post-pass ok — zero kernel panic markers (drawterm's own sysfatal exit pair excluded)
drawterm-gate: post-pass ok — the per-bake DEMO password redacted from the retained capture (T-01-58)
drawterm-gate: ALL STAGES PASS — cpu boot to cpurc: DONE, -G control session, serial /srv shows neither rio entry (GATE-PASS control-no-rio)
GATE PASS quick x86_64 run=320886-1795453021

~~~

### out/full/full.log

~~~
secstore
 Feb  7 01:28:16 AUTH norbert
 Feb  7 01:28:16 [norbert] GET ssmoke-r9-auth-x64: stage ok — secstored-get
ecstore-obj.txt
process 81 exited: ""
churngate: heap exit 5320704 33554432
 Feb  7 01:28:16 [norbert] BYE
process 82 exited: ""
churngate: heap exit 5320704 33554432
devmnt: server error: file does not exist
AUTH-SECOBS Asmoke-r9-auth-x64: stage ok — secstore-bytesback
UTH-ROUNDTRIP-PAYLOAD-18-05
process 83 exited: ""
churngate: heap exit 5320704 33554432
devmnt: server error: file does not exist
AUTH-OK secstore-roundtrip
smoke-r9-auth-x64: stage ok — roundtrip-verdict
process 84 exited: ""
churngate: heap exit 5320704 33554432
devmnt: server error: file does not exist
AUTH-DONE
smoke-r9-auth-x64: stage ok — sweep-done
process 85 exited: ""
churngate: heap exit 5320704 33554432
process 5 exited: ""
churngate: heap exit 5320704 33554432
% smoke-r9-auth-x64: stage ok — prompt
smoke-r9-auth-x64: stage ok — nvram-prompt-absent (the getpass string never appeared)
smoke-r9-auth-x64: stage ok — zero AUTH-FAIL in the capture
smoke-r9-auth-x64: stage ok — pairing banner `authdom: PAIRED DEMO' present
smoke-r9-auth-x64: stage ok — secstored ANNOUNCE line present (the syslog console fallback carrying AUTH-07)
smoke-r9-auth-x64: post-pass ok — SPEAKSFOR-DENIED x 1 (the deny control fired exactly once)
smoke-r9-auth-x64: post-pass ok — tr-ok present (authsrv -d's per-exchange syslog line: AUTH-07's real-exchange evidence)
smoke-r9-auth-x64: post-pass ok — the deny control's tr-fail 'no speaks for' line present (the fail-closed speaksfor gate proven)
smoke-r9-auth-x64: post-pass ok — the secstore round-trip PRESENCE-demanded: saved + AUTH norbert + [norbert] PUT/GET syslog tokens + the bytes-back AUTH-SECOBS + the verdict all in the capture
smoke-r9-auth-x64: post-pass ok — the per-bake DEMO password redacted from the retained capture (T-18-13; asserts ran over the true capture first)
smoke-r9-auth-x64: ALL STAGES PASS — chain boot, hostfwd-5670 observed host-side, D-02 boot pairing (DEMO label), non-interactive keyfs (N keys read), AUTH-OK keyfs, loopback up, the writable secstore venue (ramfs, live file probe), authsrv connect-accept on 567, secstored announced on 5356, cs answering authdom from the baked local, factotum carrying the p9sk1 key, auth/debug's p9sk1 success as norbert, hauthprobe's SPEAKSFOR-OK (bootes for norbert), the deny control (SPEAKSFOR-DENIED x1 + 'no speaks for'), console restored, secuser's account through the venue, the secstore round-trip EXERCISED and presence-demanded (secstore: saved + AUTH norbert + [norbert] PUT/GET syslog tokens + the bytes-back AUTH-SECOBS + the AUTH-OK secstore-roundtrip verdict — the 18-05 SKIP arm retired), AUTH-DONE, ANNOUNCE on serial, tr-ok present, nvram-prompt-absent, zero AUTH-FAIL, the capture redacted
== composed auth gate PASS — capture retained at /home/req/code/r9/target/gates/smoke-r9-auth-x64/smoke-r9-auth.log
GATE PASS full x86_64-linux run=685446-1795491574
FULL-RUN-DIR /home/req/code/r9/target/gate/685446-1795491574

~~~

### out/gate/gate.log

~~~
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
% GATE-PASS control-no-rio
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
drawterm-gate: spawned drawterm on a pty (mode control-noG, argv0 tag r9x64dtgate-dt-540452-1795468407, user glenda)
drawterm-gate: feed ok — dt-password
drawterm-gate: stage ok — session-prompt (the remote interactive shell)
drawterm-gate: stage ok — serial console rc prompt (init's handoff after cpurc: DONE)
drawterm-gate: stage ok — rio budget elapsed (60 s)
drawterm-gate: stage ok — serial /srv shows neither rio nor riowctl (the negative control)
drawterm-gate: post-pass ok — zero cpurc: FAIL in the capture
drawterm-gate: post-pass ok — zero kernel panic markers (drawterm's own sysfatal exit pair excluded)
drawterm-gate: post-pass ok — the per-bake DEMO password redacted from the retained capture (T-01-58)
drawterm-gate: ALL STAGES PASS — cpu boot to cpurc: DONE, -G control session, serial /srv shows neither rio entry (GATE-PASS control-no-rio)
GATE PASS quick x86_64 run=540452-1795468407
GATE-RUN 2 exit=0

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
drawterm-gate: spawned drawterm on a pty (mode force-no-rio, argv0 tag r9x64dtgate-dt-550222-1795468601, user glenda)
drawterm-gate: feed ok — dt-password
drawterm-gate: stage ok — session-prompt (the remote interactive shell)
drawterm-gate: stage ok — serial console rc prompt (init's handoff after cpurc: DONE)
drawterm-gate: stage ok — rio budget elapsed (60 s)
drawterm-gate: GATE-FAIL rio-not-serving — no rio.<user>.<pid> + riowctl.<user>.<pid> pair in the serial /srv listing within the budget
GATE-FAIL force-no-rio-leg-failed (leg force-no-rio; capture /home/req/code/r9/target/gate/550222-1795468601/force-no-rio.log)
GATE-NEG-EXPECTED-FAIL

~~~

### out/kernel/kernel.log

~~~
KERNEL-STORE /nix/store/wwm4a44fhyxhjvbzpwybyi53h8y6vdwd-r9-kernel-x64-af46615
2377dc2fb599a838425659ce7fe52f118ee4bfc9ac8f8a877091fab0cb111018  /nix/store/wwm4a44fhyxhjvbzpwybyi53h8y6vdwd-r9-kernel-x64-af46615/kernel-x64.elf
this derivation will be built:
  /nix/store/8dpjymiwp76qqb6s0fyp6kkyylvlkv8y-r9-boot-iso-af46615.drv
building '/nix/store/8dpjymiwp76qqb6s0fyp6kkyylvlkv8y-r9-boot-iso-af46615.drv'...
KERNEL-BOOTISO-STORE /nix/store/793z5bgjllnnq16gwlfhc21dcb6wry26-r9-boot-iso-af46615
KERNEL-BOOTISO-BUILD-OK
checking outputs of '/nix/store/8dpjymiwp76qqb6s0fyp6kkyylvlkv8y-r9-boot-iso-af46615.drv'...
KERNEL-BOOTISO-REBUILD-OK

~~~

### out/probe/probe.log

~~~
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
% GATE-PASS control-no-rio
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
drawterm-gate: spawned drawterm on a pty (mode control-noG, argv0 tag r9x64dtgate-dt-307565-1795452813, user glenda)
drawterm-gate: feed ok — dt-password
drawterm-gate: stage ok — session-prompt (the remote interactive shell)
drawterm-gate: stage ok — serial console rc prompt (init's handoff after cpurc: DONE)
drawterm-gate: stage ok — rio budget elapsed (60 s)
drawterm-gate: stage ok — serial /srv shows neither rio nor riowctl (the negative control)
drawterm-gate: post-pass ok — zero cpurc: FAIL in the capture
drawterm-gate: post-pass ok — zero kernel panic markers (drawterm's own sysfatal exit pair excluded)
drawterm-gate: post-pass ok — the per-bake DEMO password redacted from the retained capture (T-01-58)
drawterm-gate: ALL STAGES PASS — cpu boot to cpurc: DONE, -G control session, serial /srv shows neither rio entry (GATE-PASS control-no-rio)
GATE PASS quick x86_64 run=307565-1795452813

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
    Finished `dev` profile [optimized + debuginfo] target(s) in 14.74s
mb2: header at file offset 0x1008 validated on the stripped ISO kernel (magic/arch/length/checksum/end tag)
xtask: image x86_64/substrate missing or stale — assembling
image: x86_64/substrate /home/req/code/r9/target/images/x86_64/substrate.paq sha256 5f5f393ee47fecb716245d4b2a0e5e3434bc3cb008e6f8e7d0200937acb31920 members 27
iso: module /home/req/code/r9/target/images/x86_64/substrate.paq staged at /boot/bootfs.paq
xorriso 1.5.8.pl02 : RockRidge filesystem manipulator, libburnia project.

xorriso : NOTE : Environment variable SOURCE_DATE_EPOCH encountered with value 315532800
Drive current: -outdev 'stdio:target/boot-q35auth.iso'
Media current: stdio file, overwriteable
Media status : is blank
Media summary: 0 sessions, 0 data blocks, 0 data,  363g free
Added to ISO image: directory '/'='/tmp/nix-shell.Ovf6E5/grub.a8jxeF'
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
boot-test: captured 2206 bytes

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
    Finished `dev` profile [optimized + debuginfo] target(s) in 15.14s
mb2: header at file offset 0x1008 validated on the stripped ISO kernel (magic/arch/length/checksum/end tag)
iso: module /home/req/code/r9/target/images/x86_64/marker.paq staged at /boot/bootfs.paq
xorriso 1.5.8.pl02 : RockRidge filesystem manipulator, libburnia project.

xorriso : NOTE : Environment variable SOURCE_DATE_EPOCH encountered with value 315532800
Drive current: -outdev 'stdio:target/boot-q35churn.iso'
Media current: stdio file, overwriteable
Media status : is blank
Media summary: 0 sessions, 0 data blocks, 0 data,  363g free
Added to ISO image: directory '/'='/tmp/nix-shell.vqdT6b/grub.MOOJh0'
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
boot-test: captured 124245 bytes

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
    Finished `dev` profile [optimized + debuginfo] target(s) in 14.78s
mb2: header at file offset 0x1008 validated on the stripped ISO kernel (magic/arch/length/checksum/end tag)
iso: module /home/req/code/r9/target/images/x86_64/chain.paq staged at /boot/bootfs.paq
xorriso 1.5.8.pl02 : RockRidge filesystem manipulator, libburnia project.

xorriso : NOTE : Environment variable SOURCE_DATE_EPOCH encountered with value 315532800
Drive current: -outdev 'stdio:target/boot-q35churnshell.iso'
Media current: stdio file, overwriteable
Media status : is blank
Media summary: 0 sessions, 0 data blocks, 0 data,  363g free
Added to ISO image: directory '/'='/tmp/nix-shell.HfUEbZ/grub.SySgZq'
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
    Finished `dev` profile [optimized + debuginfo] target(s) in 14.89s
mb2: header at file offset 0x1008 validated on the stripped ISO kernel (magic/arch/length/checksum/end tag)
iso: module /home/req/code/r9/target/images/x86_64/cpu.paq staged at /boot/bootfs.paq
iso: kernel cmdline append "init=/amd64/init -c" (mb2 tag 1, the bootconf carrier)
xorriso 1.5.8.pl02 : RockRidge filesystem manipulator, libburnia project.

xorriso : NOTE : Environment variable SOURCE_DATE_EPOCH encountered with value 315532800
Drive current: -outdev 'stdio:target/boot-q35cpu.iso'
Media current: stdio file, overwriteable
Media status : is blank
Media summary: 0 sessions, 0 data blocks, 0 data,  363g free
Added to ISO image: directory '/'='/tmp/nix-shell.JpXXxz/grub.mkKi79'
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
boot-test: captured 4029 bytes

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
    Finished `dev` profile [optimized + debuginfo] target(s) in 15.06s
mb2: header at file offset 0x1008 validated on the stripped ISO kernel (magic/arch/length/checksum/end tag)
xtask: image x86_64/marker missing or stale — assembling
image: x86_64/marker /home/req/code/r9/target/images/x86_64/marker.paq sha256 3d95e0acd54e38fa2d183e69da0c7a750f0c817c433648fcfd58ab945965e7f9 members 30
iso: module /home/req/code/r9/target/images/x86_64/marker.paq staged at /boot/bootfs.paq
xorriso 1.5.8.pl02 : RockRidge filesystem manipulator, libburnia project.

xorriso : NOTE : Environment variable SOURCE_DATE_EPOCH encountered with value 315532800
Drive current: -outdev 'stdio:target/boot-q35exec.iso'
Media current: stdio file, overwriteable
Media status : is blank
Media summary: 0 sessions, 0 data blocks, 0 data,  363g free
Added to ISO image: directory '/'='/tmp/nix-shell.lti0GM/grub.c3Mtuy'
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
boot-test: captured 2067 bytes

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
    Finished `dev` profile [optimized + debuginfo] target(s) in 13.46s
mb2: header at file offset 0x1008 validated on the stripped ISO kernel (magic/arch/length/checksum/end tag)
xorriso 1.5.8.pl02 : RockRidge filesystem manipulator, libburnia project.

xorriso : NOTE : Environment variable SOURCE_DATE_EPOCH encountered with value 315532800
Drive current: -outdev 'stdio:target/boot-q35.iso'
Media current: stdio file, overwriteable
Media status : is blank
Media summary: 0 sessions, 0 data blocks, 0 data,  363g free
Added to ISO image: directory '/'='/tmp/nix-shell.67fG2b/grub.GbARYq'
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
    Finished `dev` profile [optimized + debuginfo] target(s) in 14.16s
mb2: header at file offset 0x1008 validated on the stripped ISO kernel (magic/arch/length/checksum/end tag)
xorriso 1.5.8.pl02 : RockRidge filesystem manipulator, libburnia project.

xorriso : NOTE : Environment variable SOURCE_DATE_EPOCH encountered with value 315532800
Drive current: -outdev 'stdio:target/boot-q35lowmcfg.iso'
Media current: stdio file, overwriteable
Media status : is blank
Media summary: 0 sessions, 0 data blocks, 0 data,  363g free
Added to ISO image: directory '/'='/tmp/nix-shell.Cl1SRM/grub.CtAHuO'
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
    Finished `dev` profile [optimized + debuginfo] target(s) in 14.69s
mb2: header at file offset 0x1008 validated on the stripped ISO kernel (magic/arch/length/checksum/end tag)
xorriso 1.5.8.pl02 : RockRidge filesystem manipulator, libburnia project.

xorriso : NOTE : Environment variable SOURCE_DATE_EPOCH encountered with value 315532800
Drive current: -outdev 'stdio:target/boot-q35net.iso'
Media current: stdio file, overwriteable
Media status : is blank
Media summary: 0 sessions, 0 data blocks, 0 data,  363g free
Added to ISO image: directory '/'='/tmp/nix-shell.MuvYYb/grub.TEWxHF'
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
boot-test: captured 2299 bytes

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
    Finished `dev` profile [optimized + debuginfo] target(s) in 14.68s
mb2: header at file offset 0x1008 validated on the stripped ISO kernel (magic/arch/length/checksum/end tag)
xtask: image x86_64/chain missing or stale — assembling
image: x86_64/chain /home/req/code/r9/target/images/x86_64/chain.paq sha256 73042f219db624fbcc10ffe4e127b99510b6a6c01871f33744dc4df0e03235c5 members 49
iso: module /home/req/code/r9/target/images/x86_64/chain.paq staged at /boot/bootfs.paq
xorriso 1.5.8.pl02 : RockRidge filesystem manipulator, libburnia project.

xorriso : NOTE : Environment variable SOURCE_DATE_EPOCH encountered with value 315532800
Drive current: -outdev 'stdio:target/boot-q35shell.iso'
Media current: stdio file, overwriteable
Media status : is blank
Media summary: 0 sessions, 0 data blocks, 0 data,  363g free
Added to ISO image: directory '/'='/tmp/nix-shell.9U5tbw/grub.rftuxZ'
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

### out/soak/soak.log

~~~
RecoveryDone: 0
RecoveryRTO: 0
RecoveryNoSeq: 0
RecoveryCwind: 0
RecoveryPA: 0
process 51 exited: ""
churngate: heap exit 2823424 33554432
process 53 exited: ""
churngate: heap exit 2823424 33554432
process 54 exited: ""
churngate: heap exit 2823424 33554432
process 52 exited: ""
churngate: heap exit 2823424 33554432
process 56 exited: ""
churngate: heap exit 2823424 33554432
process 57 exited: ""
churngate: heap exit 2823424 33554432
process 55 exited: ""
churngate: heap exit 2823424 33554432
netip: soak stats oknetsoak-r9-x64: stage ok — soak-stats

process 58 exited: ""
churngate: heap exit 2823424 33554432
netip: soak ok
netsoak-r9-x64: stage ok — soak-ok
process 59 exited: ""
churngate: heap exit 2823424 33554432
netip: NETIP-DONE
netsoak-r9-x64: stage ok — soak-done
process 60 exited: ""
churngate: heap exit 2823424 33554432
process 5 exited: ""
churngate: heap exit 2823424 33554432
% netsoak-r9-x64: stage ok — soak-prompt
netsoak-r9-x64: post-pass ok — tcpblast: done x 9 (1 baseline + 8 burst)
netsoak-r9-x64: post-pass ok — zero netip: FAIL occurrences
netsoak-r9-x64: post-pass ok — ARP L2-truth marker + L2-TRUTH tap venue label present
netsoak-r9-x64: ALL STAGES PASS — chain boot, netip-tap fixture (static + ARP L2-truth + venue label), baseline 2048 rounds + burst 8 x 512 rounds (counted 9 tcpblast: done), stats reconciliation, zero FAIL, completion
== soak gate PASS — capture retained at /home/req/code/r9/target/gates/soak-r9-net-x64/netsoak-r9.log; pcap + peer log at /home/req/code/r9/target/gates/soak-r9-net-x64/evidence.OH2DpW
== soak evidence scratch retained: /home/req/code/r9/target/gates/soak-r9-net-x64/evidence.OH2DpW (netsoak.pcap + hostecho.log — this run's evidence)

~~~

### out/store-verify/store-verify.log

~~~
STORE-VERIFY-PATH /nix/store/002vay29nmgw2d81nvmi2i6adgz7h1md-harvey-tree-amd64-0
checking '/nix/store/002vay29nmgw2d81nvmi2i6adgz7h1md-harvey-tree-amd64-0'...
STORE-VERIFY-OK /nix/store/002vay29nmgw2d81nvmi2i6adgz7h1md-harvey-tree-amd64-0
STORE-VERIFY-PATH /nix/store/b2i0hcbxxmyjlh7drl94s0lp3iq7n04a-harvey-tree-arm64-0
checking '/nix/store/b2i0hcbxxmyjlh7drl94s0lp3iq7n04a-harvey-tree-arm64-0'...
STORE-VERIFY-OK /nix/store/b2i0hcbxxmyjlh7drl94s0lp3iq7n04a-harvey-tree-arm64-0

~~~

### out/stress/stress.log

~~~
ReseqPktlim: 0
Delayack: 0
Wopenack: 0
Recovery: 0
RecoveryDone: 0
RecoveryRTO: 0
RecoveryNoSeq: 0
RecoveryCwind: 0
RecoveryPA: 0
process 49 exited: ""
churngate: heap exit 2835712 33554432
process 51 exited: ""
churngate: heap exit 2835712 33554432
process 52 exited: ""
churngate: heap exit 2835712 33554432
process 50 exited: ""
churngate: heap exit 2835712 33554432
process 54 exited: ""
churngate: heap exit 2835712 33554432
process 55 exited: ""
churngate: heap exit 2835712 33554432
process 53 exited: ""
churngate: heap exit 2835712 33554432
netstress: stats ok
pstress-r9-x64: stage ok — stress-stats
rocess 56 exited: ""
churngate: heap exit 2835712 33554432
netstress: STRESS-DONEstress-r9-x64: stage ok — stress-done

process 57 exited: ""
churngate: heap exit 2835712 33554432
process 5 exited: ""
churngate: heap exit 2835712 33554432
% stress-r9-x64: stage ok — stress-prompt
stress-r9-x64: post-pass ok — tcpblast: done x 9 (1 baseline + 8 burst)
stress-r9-x64: post-pass ok — zero netstress: FAIL occurrences
stress-r9-x64: post-pass ok — venue label present (tap)
stress-r9-x64: ALL STAGES PASS — chain boot, netstress fixture (tap: static + venue label + baseline 1 + burst 8 x 512, counted 9 tcpblast: done), stats reconciliation, zero FAIL, completion
== stress gate PASS — capture retained at /home/req/code/r9/target/gates/stress-r9-net-x64/stress-r9.log; cascade + peer log at /home/req/code/r9/target/gates/stress-r9-net-x64/evidence.FbyMSC
== stress evidence scratch retained: /home/req/code/r9/target/gates/stress-r9-net-x64/evidence.FbyMSC (qemu -d cascade: qemu-int-*.log + netstress.pcap + hostecho.log)

~~~

### out/trees/trees.log

~~~
unpacking 'github:requaos/harvey/c697666ba315f137ca2fd11768f835d04bd9f018?narHash=sha256-VhB2RKggOVWGNjiMHC0s6OMK7q3khwskByCyaTFbKwg%3D' into the Git cache...
these 4 derivations will be built:
  /nix/store/hgny4f6s6rznqy3sqkfzas1cr7mayd4g-source.drv
  /nix/store/pw42zwrqg56072j12nlc1q43nji4padz-harvey-hostbin-0.drv
  /nix/store/9p3i39hrvi6brykkzhb9jj356kvc69a7-harvey-tree-arm64-0.drv
  /nix/store/dfiqi4g071n2v9sw80l7n9n9my6fkxrg-harvey-tree-amd64-0.drv
building '/nix/store/hgny4f6s6rznqy3sqkfzas1cr7mayd4g-source.drv'...
building '/nix/store/pw42zwrqg56072j12nlc1q43nji4padz-harvey-hostbin-0.drv'...
building '/nix/store/dfiqi4g071n2v9sw80l7n9n9my6fkxrg-harvey-tree-amd64-0.drv'...
building '/nix/store/9p3i39hrvi6brykkzhb9jj356kvc69a7-harvey-tree-arm64-0.drv'...
TREES-STORE /nix/store/002vay29nmgw2d81nvmi2i6adgz7h1md-harvey-tree-amd64-0
--- /nix/store/002vay29nmgw2d81nvmi2i6adgz7h1md-harvey-tree-amd64-0/HARVEY-TREE ---
  rev=c697666ba315f137ca2fd11768f835d04bd9f018
  narHash=sha256-VhB2RKggOVWGNjiMHC0s6OMK7q3khwskByCyaTFbKwg=
  arch=amd64
  system=x86_64-linux
checking '/nix/store/002vay29nmgw2d81nvmi2i6adgz7h1md-harvey-tree-amd64-0'...
TREES-VERIFY-OK /nix/store/002vay29nmgw2d81nvmi2i6adgz7h1md-harvey-tree-amd64-0
TREES-STORE /nix/store/b2i0hcbxxmyjlh7drl94s0lp3iq7n04a-harvey-tree-arm64-0
--- /nix/store/b2i0hcbxxmyjlh7drl94s0lp3iq7n04a-harvey-tree-arm64-0/HARVEY-TREE ---
  rev=c697666ba315f137ca2fd11768f835d04bd9f018
  narHash=sha256-VhB2RKggOVWGNjiMHC0s6OMK7q3khwskByCyaTFbKwg=
  arch=arm64
  system=x86_64-linux
checking '/nix/store/b2i0hcbxxmyjlh7drl94s0lp3iq7n04a-harvey-tree-arm64-0'...
TREES-VERIFY-OK /nix/store/b2i0hcbxxmyjlh7drl94s0lp3iq7n04a-harvey-tree-arm64-0

~~~

