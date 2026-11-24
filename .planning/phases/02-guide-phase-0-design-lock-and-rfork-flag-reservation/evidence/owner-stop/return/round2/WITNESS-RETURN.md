# r9 pass-2 witness return (plans 01-09 / 01-32)

- date: 2026-11-24T04:48:35Z
- git HEAD: 4624ea2f0cb45c81870dcc881d544d7d36a2e0bf
- r9 root: /home/req/r9-p02-stop/run
- work root: /home/req/r9-p02-stop/full
- steps: full

## out/env.txt

~~~
# r9 pass-2 witness preflight
date=2026-11-24T04:45:19Z
user=req uid=1000
r9_root=/home/req/r9-p02-stop/run
git_head=4624ea2f0cb45c81870dcc881d544d7d36a2e0bf
work_dir=/home/req/r9-p02-stop/full
steps=full
harvey_pin=bd7a6982a3e4f465f8ed3da5e0c7290521514ff0
kerndate=1795382101
uname=Linux teeniebox 7.4.0-rc3 #1-NixOS SMP PREEMPT_DYNAMIC @315532800 x86_64 GNU/Linux
nproc=16
               total        used        free      shared  buff/cache   available
Mem:            30Gi       3.1Gi        13Gi       725Mi        15Gi        27Gi
Swap:          7.5Gi          0B       7.5Gi
--- free disk under /home/req/r9-p02-stop/full ---
Filesystem        Size  Used Avail Use% Mounted on
/dev/mapper/root  946G  581G  362G  62% /
--- nix ---
nix (Nix) 2.35.6
experimental-features = dynamic-derivations fetch-closure fetch-tree flakes nix-command
sandbox = true
--- /dev/kvm ---
DEV_KVM=writable
crw-rw-rw- 1 root kvm 10, 232 Nov 23 23:41 /dev/kvm

~~~

## out/summary.tsv

~~~
full	0	196

~~~

## out/survivors.txt

~~~
# surviving qemu/Xvfb/drawterm processes owned by req
# at 2026-11-24T04:48:35Z
none

~~~

## out/secret-scan.txt

~~~
# credential scan of out/ at 2026-11-24T04:48:35Z
clean: no credential artifact and no DEMO password value in out/

~~~

## last 40 lines of each out/<step>/*.log

### out/full/full.log

~~~
secstore
 Feb  7 01:28:16 AUTH norbert
 Feb  7 01:28:16 [norbert] GET secstsmoke-r9-auth-x64: stage ok — secstored-get
ore-obj.txt
process 81 exited: ""
churngate: heap exit 5322752 33554432
 Feb  7 01:28:16 [norbert] BYE
process 82 exited: ""
churngate: heap exit 5322752 33554432
devmnt: server error: file does not exist
AUTH-SECOBS Asmoke-r9-auth-x64: stage ok — secstore-bytesback
UTH-ROUNDTRIP-PAYLOAD-18-05
process 83 exited: ""
churngate: heap exit 5322752 33554432
devmnt: server error: file does not exist
AUTH-OK secstore-roundtripsmoke-r9-auth-x64: stage ok — roundtrip-verdict

process 84 exited: ""
churngate: heap exit 5322752 33554432
devmnt: server error: file does not exist
AUTH-DONE
prsmoke-r9-auth-x64: stage ok — sweep-done
ocess 85 exited: ""
churngate: heap exit 5322752 33554432
process 5 exited: ""
churngate: heap exit 5322752 33554432
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
== composed auth gate PASS — capture retained at /home/req/r9-p02-stop/run/target/gates/smoke-r9-auth-x64/smoke-r9-auth.log
GATE PASS full x86_64-linux run=785121-1795495519
FULL-RUN-DIR /home/req/r9-p02-stop/run/target/gate/785121-1795495519

~~~

