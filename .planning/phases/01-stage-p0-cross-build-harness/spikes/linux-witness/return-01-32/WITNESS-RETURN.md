# r9 pass-2 witness return (plans 01-09 / 01-32)

- date: 2026-11-23T06:03:54Z
- git HEAD: 7a0f6d2bee2f6da2c04a5c34a3fdd2e0a51d1565
- r9 root: /home/req/code/r9
- work root: /home/req/r9-witness-work
- steps: rio-diag

## out/env.txt

~~~
# r9 pass-2 witness preflight
date=2026-11-23T06:03:17Z
user=req uid=1000
r9_root=/home/req/code/r9
git_head=7a0f6d2bee2f6da2c04a5c34a3fdd2e0a51d1565
work_dir=/home/req/r9-witness-work
steps=rio-diag
harvey_pin=bd7a6982a3e4f465f8ed3da5e0c7290521514ff0
kerndate=1795382101
uname=Linux teeniebox 7.4.0-rc3 #1-NixOS SMP PREEMPT_DYNAMIC @315532800 x86_64 GNU/Linux
nproc=16
               total        used        free      shared  buff/cache   available
Mem:            30Gi       2.4Gi        11Gi       393Mi        18Gi        28Gi
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
crw-rw-rw- 1 root kvm 10, 232 Nov 23 00:55 /dev/kvm

~~~

## out/summary.tsv

~~~
rio-diag	1	37

~~~

## out/survivors.txt

~~~
# surviving qemu/Xvfb/drawterm processes owned by req
# at 2026-11-23T06:03:54Z
none

~~~

## out/secret-scan.txt

~~~
# credential scan of out/ at 2026-11-23T06:03:54Z
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
== PASS: baked /home/req/r9-witness-work/harvey/build/x86_64/out/bootfs-cpu.paq (22002513 bytes; chain + the auth suite + cmd/cpu + aux/listen + the real cpurc + tcp17010 + the font payload + the §15 p9sk1 domain + the 19-10 desktop suite (test/rio/plumber/plumb/ramfs/syscall/awd + glenda's home + the plumb rules); header 529ab12b trailer 6b46e688; KERNDATE 1795382101)

~~~

### out/rio-diag/rio-diag.log

~~~
churngate: heap exit 3430592 33554432
devmnt: server error: file does not exist
process 38 exited: ""
churngate: heap exit 3430784 33554432
 Feb  7 01:28:16 started on tcp
process 39 exited: ""
churngate: heap exit 3440960 33554432
devmnt: server error: file does not exist
devmnt: server error: file does not exist
process 41 exited: ""
churngate: heap exit 4145152 33554432
process 43 exited: ""
churngate: heap exit 4145152 33554432
process 44 exited: ""
churngate: heap exit 4145216 33554432
process 45 exited: ""
churngate: heap exit 4145216 33554432
cpurc: LISTEN-17010
rio-gate: stage ok — listen-17010
process 46 exited: ""
churngate: heap exit 4149248 33554432
cpurc: DONErio-gate: stage ok — ladder-done
spawn bash -c R9_RUN_ID=r9riogate-dt-99543-1795413797 exec -a r9riogate-dt-99543-1795413797 /nix/store/y8h5ana2drnzs2fdx6fxgr74bwmzi8y9-drawterm-19.06/bin/drawterm -s '' -O cpu=127.0.0.1,auth=tcp!127.0.0.1!5670,user=glenda
rio-gate: spawned drawterm on a pty (mode rio-diag, argv0 tag r9riogate-dt-99543-1795413797, user glenda)
RIO-FAIL diag-capture — the before-typing PNG capture failed (convert-failed: WARNING: The convert command is deprecated in IMv7, use "magick" instead of "convert" or "magick convert"
)
RIO-ERROR: the rio-diag gate FAILED (rc=1 — the driver named the failure on stderr; capture: /home/req/r9-witness-work/out/rio-diag/rio-gate-rio-diag.log)
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
churngate: heap exit 2613248 33554432
process 29 exited: ""
churngate: heap exit 2613248 33554432
process 30 exited: ""
churngate: heap exit 2613376 33554432
syscall 10 unimplemented
factotum warning: cannot chmod 666 /srv/factotum: wstat: device does not implement wstat
process 31 exited: ""
churngate: heap exit 3427712 33554432
process 33 exited: ""
churngate: heap exit 3427968 33554432
process 34 exited: ""
churngate: heap exit 3427968 33554432
process 35 exited: ""
churngate: heap exit 3430400 33554432
process 36 exited: ""
churngate: heap exit 3430528 33554432
cpurc: FACTOTUM-OK
process 37 exited: ""
churngate: heap exit 3430592 33554432
devmnt: server error: file does not exist
process 38 exited: ""
churngate: heap exit 3430784 33554432
 Feb  7 01:28:16 started on tcp
process 39 exited: ""
churngate: heap exit 3440960 33554432
devmnt: server error: file does not exist
devmnt: server error: file does not exist
process 41 exited: ""
churngate: heap exit 4145152 33554432
process 43 exited: ""
churngate: heap exit 4145152 33554432
process 44 exited: ""
churngate: heap exit 4145216 33554432
process 45 exited: ""
churngate: heap exit 4145216 33554432
cpurc: LISTEN-17010
process 46 exited: ""
churngate: heap exit 4149248 33554432
cpurc: DONEspawn bash -c R9_RUN_ID=r9riogate-dt-99543-1795413797 exec -a r9riogate-dt-99543-1795413797 /nix/store/y8h5ana2drnzs2fdx6fxgr74bwmzi8y9-drawterm-19.06/bin/drawterm -s '' -O cpu=127.0.0.1,auth=tcp!127.0.0.1!5670,user=glenda

~~~

### out/rio/rio-gate-rio.log

~~~
/srv/boot
/srv/factotum
process 73 exited: ""
churngate: heap exit 4693248 33554432
% ls /srv
/srv/boot
/srv/factotum
process 74 exited: ""
churngate: heap exit 4693248 33554432
% ls /srv
/srv/boot
/srv/factotum
process 75 exited: ""
churngate: heap exit 4693248 33554432
% ls /srv
/srv/boot
/srv/factotum
process 76 exited: ""
churngate: heap exit 4693248 33554432
% ls /srv
/srv/boot
/srv/factotum
process 77 exited: ""
churngate: heap exit 4693248 33554432
% ls /srv
/srv/boot
/srv/factotum
process 78 exited: ""
churngate: heap exit 4693248 33554432
% ls /srv
/srv/boot
/srv/factotum
process 79 exited: ""
churngate: heap exit 4693248 33554432
% ls /srv
/srv/boot
/srv/factotum
process 80 exited: ""
churngate: heap exit 4693248 33554432
% 
~~~

### out/rio/rio.log

~~~
/srv/factotum
process 73 exited: ""
churngate: heap exit 4693248 33554432
% ls /srv
/srv/boot
/srv/factotum
process 74 exited: ""
churngate: heap exit 4693248 33554432
% ls /srv
/srv/boot
/srv/factotum
process 75 exited: ""
churngate: heap exit 4693248 33554432
% ls /srv
/srv/boot
/srv/factotum
process 76 exited: ""
churngate: heap exit 4693248 33554432
% ls /srv
/srv/boot
/srv/factotum
process 77 exited: ""
churngate: heap exit 4693248 33554432
% ls /srv
/srv/boot
/srv/factotum
process 78 exited: ""
churngate: heap exit 4693248 33554432
% ls /srv
/srv/boot
/srv/factotum
process 79 exited: ""
churngate: heap exit 4693248 33554432
% ls /srv
/srv/boot
/srv/factotum
process 80 exited: ""
churngate: heap exit 4693248 33554432
% RIO-FAIL rio-not-serving — no rio.<user>.<pid> + riowctl.<user>.<pid> pair in the serial /srv listing within the budget
RIO-ERROR: the rio gate FAILED (rc=1 — the driver named the failure on stderr; capture: /home/req/r9-witness-work/out/rio/rio-gate-rio.log)

~~~

