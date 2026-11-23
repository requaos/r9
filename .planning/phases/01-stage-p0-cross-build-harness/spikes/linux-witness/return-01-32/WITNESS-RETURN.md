# r9 pass-2 witness return (plans 01-09 / 01-32)

- date: 2026-11-23T06:49:04Z
- git HEAD: 46d0677ee4fcc7ef64b961dfdaf9d4bd1b793e9d
- r9 root: /home/req/code/r9
- work root: /home/req/r9-witness-work
- steps: rio-diag rio

## out/env.txt

~~~
# r9 pass-2 witness preflight
date=2026-11-23T06:43:41Z
user=req uid=1000
r9_root=/home/req/code/r9
git_head=46d0677ee4fcc7ef64b961dfdaf9d4bd1b793e9d
work_dir=/home/req/r9-witness-work
steps=rio-diag rio
harvey_pin=bd7a6982a3e4f465f8ed3da5e0c7290521514ff0
kerndate=1795382101
uname=Linux teeniebox 7.4.0-rc3 #1-NixOS SMP PREEMPT_DYNAMIC @315532800 x86_64 GNU/Linux
nproc=16
               total        used        free      shared  buff/cache   available
Mem:            30Gi       2.4Gi        10Gi       393Mi        18Gi        28Gi
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
crw-rw-rw- 1 root kvm 10, 232 Nov 23 01:29 /dev/kvm

~~~

## out/summary.tsv

~~~
rio-diag	0	161
rio	0	162

~~~

## out/survivors.txt

~~~
# surviving qemu/Xvfb/drawterm processes owned by req
# at 2026-11-23T06:49:04Z
none

~~~

## out/secret-scan.txt

~~~
# credential scan of out/ at 2026-11-23T06:49:04Z
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

