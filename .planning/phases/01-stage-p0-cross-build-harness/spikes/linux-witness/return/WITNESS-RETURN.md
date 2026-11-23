# r9 Linux witness return (Plan 01-06)

- date: 2026-11-23T01:51:20Z
- git HEAD: c92a3f4bddedc7ed299f72c5393866b15c7c2516
- staging root: /home/req/r9-spike-work/witness-bundle
- kernel sha256: ecdcdaddea8d3759d74303b869ddfae8efeba1d4d160382577cfc52b9a32c45b (2030226 bytes; matches s5/evidence/kernel-provenance.txt)
- probe sha256: db94057f04c751c59a648de5c8505f452bb225b6a276714e7ee5544396a5dcd5
- probe recorded sha256: db94057f04c751c59a648de5c8505f452bb225b6a276714e7ee5544396a5dcd5
- probe match: yes
- run-all exit: 1

## out/env.txt

~~~
# r9 Linux witness preflight
date=2026-11-23T01:44:51Z
user=req uid=1000
bundle_root=/home/req/r9-spike-work/witness-bundle
work_dir=/home/req/r9-spike-work
uname=Linux teeniebox 7.4.0-rc3 #1-NixOS SMP PREEMPT_DYNAMIC @315532800 x86_64 GNU/Linux
nproc=16
               total        used        free      shared  buff/cache   available
Mem:            30Gi       2.0Gi        27Gi       267Mi       2.5Gi        28Gi
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
crw-rw-rw- 1 root kvm 10, 232 Nov 22 20:40 /dev/kvm
--- sysctls (unprivileged userns is what Harvey's x86_64 venue needs) ---
user.max_user_namespaces=126494
kernel.unprivileged_userns_clone=unavailable
kernel.apparmor_restrict_unprivileged_userns=unavailable

~~~

## out/summary.tsv

~~~
s1	1	78
s5	0	6

~~~

## out/survivors.txt

(missing: /home/req/r9-spike-work/witness-bundle/out/survivors.txt)

## out/secret-scan.txt

(missing: /home/req/r9-spike-work/witness-bundle/out/secret-scan.txt)

## last 40 lines of each out/<step>/*.log

### out/s1/libc-amd64.log

~~~
s1-libc-amd64> 6a  rdpmc.s
s1-libc-amd64> 6a  setjmp.s
s1-libc-amd64> 6a  sqrt.s
s1-libc-amd64> 6a  strcat.s
s1-libc-amd64> 6a  strchr.s
s1-libc-amd64> 6a  strcpy.s
s1-libc-amd64> 6a  strlen.s
s1-libc-amd64> 6a  tas.s
s1-libc-amd64> ar vu /amd64/lib/libc.a _seek.6 getcallerpc.6 notejmp.6 argv0.6 atom.6 cycles.6 getfcr.6 main9.6 main9p.6 memccpy.6 memchr.6 memcmp.6 memcpy.6 memmove.6 memset.6 muldiv.6 rdpmc.6 setjmp.6 sqrt.6 strcat.6 strchr.6 strcpy.6 strlen.6 tas.6
s1-libc-amd64> a - _seek.6
s1-libc-amd64> a - getcallerpc.6
s1-libc-amd64> a - notejmp.6
s1-libc-amd64> a - argv0.6
s1-libc-amd64> a - atom.6
s1-libc-amd64> a - cycles.6
s1-libc-amd64> a - getfcr.6
s1-libc-amd64> a - main9.6
s1-libc-amd64> a - main9p.6
s1-libc-amd64> a - memccpy.6
s1-libc-amd64> a - memchr.6
s1-libc-amd64> a - memcmp.6
s1-libc-amd64> a - memcpy.6
s1-libc-amd64> a - memmove.6
s1-libc-amd64> a - memset.6
s1-libc-amd64> a - muldiv.6
s1-libc-amd64> a - rdpmc.6
s1-libc-amd64> a - setjmp.6
s1-libc-amd64> a - sqrt.6
s1-libc-amd64> a - strcat.6
s1-libc-amd64> a - strchr.6
s1-libc-amd64> a - strcpy.6
s1-libc-amd64> a - strlen.6
s1-libc-amd64> a - tas.6
s1-libc-amd64> S1-RUNG2-LIBC-SECONDS=4
s1-libc-amd64> --rw-r--r-- M 0 0 0 601170 Nov 23 01:46 /amd64/lib/libc.a
s1-libc-amd64> S1-RUNG2-LIBC-MEMBERS=259
s1-libc-amd64> S1-RUNG2-LIBC-OK
s1-libc-amd64> S1-VENUE-RC=0
s1-libc-amd64> S1-VENUE-OK
/nix/store/mfvy2a00xf01p410i11q9lxya1pcvi37-s1-libc-amd64-0

~~~

### out/s1/nsprobe.log

~~~
unpacking 'github:requaos/harvey/bd7a6982a3e4f465f8ed3da5e0c7290521514ff0?narHash=sha256-N6VjnVI69pcxSCpQ9BUxrlpTqnp9LgjKfGcpO3F6Ma0%3D' into the Git cache...
this derivation will be built:
  /nix/store/3sp3xvr6psh8swb49fpwj44mwy4dncny-s1-nsprobe-0.drv
this path will be fetched (152.0 KiB download, 439.9 KiB unpacked):
  /nix/store/pa0pzpgmzancbxqhxl6fr1k0ix1xd5y1-proot-5.5.0
copying path '/nix/store/pa0pzpgmzancbxqhxl6fr1k0ix1xd5y1-proot-5.5.0' from 'https://cache.nixos.org'...
building '/nix/store/3sp3xvr6psh8swb49fpwj44mwy4dncny-s1-nsprobe-0.drv'...
s1-nsprobe> SANDBOX-EVIDENCE id-u=1000
s1-nsprobe> SANDBOX-EVIDENCE uid-map=      1000      30001          1;
s1-nsprobe> SANDBOX-EVIDENCE user-ns=user:[4026533080]
s1-nsprobe> SANDBOX-EVIDENCE uname=Linux localhost 7.4.0-rc3 #1-NixOS SMP PREEMPT_DYNAMIC @315532800 x86_64 GNU/Linux
s1-nsprobe> SANDBOX-EVIDENCE usr-bin-env=absent
s1-nsprobe> S1-VENUE-BUILD-START mode=unshare
s1-nsprobe> S1-VENUE-PATH=/nix/store/kwyi4bq5s2qgwbhz6cs4gjy57c2zdxws-bash-interactive-5.3p20/bin:/nix/store/cb987zkxvsv0490753ilrvn5yk7ylgxi-coreutils-9.12/bin:/nix/store/2lgg8jqyzjp60svh7kmp39kqwnig31p2-util-linux-2.42.4-bin/bin:/nix/store/dzvnbi2n1x9jkmzvmwxdly607bwaxsbv-plan9port-0-unstable-2025-11-10/bin:/nix/store/9wc73n5izv0m7ppjbibi4hbr68rba1lj-gnused-4.10/bin:/nix/store/h96s16mjssibkvk62pvmfm3lwfc3wnwp-gawk-5.4.1/bin:/nix/store/c3f45pxlb81gzz4w1nym7lpznfypva7s-gnugrep-3.12/bin:/nix/store/nzsflqrk0zdljly9jhx82l784hl5rmc4-diffutils-3.12/bin:/nix/store/dl38pix4jvabi5smb9wywhp6pimmpva6-patchelf-0.19.2/bin:/nix/store/cb987zkxvsv0490753ilrvn5yk7ylgxi-coreutils-9.12/bin:/nix/store/99v13ig4382mngi6ggf0csqi7rfa04j5-findutils-4.11.0/bin:/nix/store/9mpja553cbkr2rc37kzdg8zbbg2dyyvl-diffutils-3.12/bin:/nix/store/9wc73n5izv0m7ppjbibi4hbr68rba1lj-gnused-4.10/bin:/nix/store/c3f45pxlb81gzz4w1nym7lpznfypva7s-gnugrep-3.12/bin:/nix/store/h96s16mjssibkvk62pvmfm3lwfc3wnwp-gawk-5.4.1/bin:/nix/store/n318yf1q5vysgh2qn6fd1q8yjy9vmwvh-gnutar-1.35/bin:/nix/store/rpswfv0ff4wap9anx7k1224x2m94pfcc-gzip-1.15/bin:/nix/store/ai2km55ba8bxrn64xx7p9lgvrrzdzv43-bzip2-1.0.8-bin/bin:/nix/store/8xz4d9wp4jy49mhl306wyg55096wis3p-gnumake-4.4.1/bin:/nix/store/nnm9gdr70ihc6zc9298mzz8jpcgaciql-bash-5.3p20/bin:/nix/store/d5jz315vxbphknfawsrjs192ci9bc6df-patch-2.8/bin:/nix/store/nflpm51il93hxh2rba6dvz59kaff0i7v-xz-5.8.4-bin/bin:/nix/store/2qpmak7162vhqxjwfik4aqs9lff6sn7w-file-5.48/bin:/nix/store/dzvnbi2n1x9jkmzvmwxdly607bwaxsbv-plan9port-0-unstable-2025-11-10/plan9/bin
s1-nsprobe> S1-VENUE-NSX=/build/nsroot.nHa578 (private tmpfs)
s1-nsprobe> mount: /build/nsroot.nHa578/nix/store: wrong fs type, bad option, bad superblock on /nix/store, missing codepage or helper program, or other error.
s1-nsprobe>        dmesg(1) may have more information after failed mount system call.
s1-nsprobe> S1-VENUE-RC=32
s1-nsprobe> S1-VENUE-FAIL the guest run exited 32
error: Cannot build '/nix/store/3sp3xvr6psh8swb49fpwj44mwy4dncny-s1-nsprobe-0.drv'.
       Reason: builder failed with exit code 32.
       Output paths:
         /nix/store/ii8pjgnqn7581g59g1icd36m9na5wnk2-s1-nsprobe-0
       Last 12 log lines:
       > SANDBOX-EVIDENCE id-u=1000
       > SANDBOX-EVIDENCE uid-map=      1000      30001          1;
       > SANDBOX-EVIDENCE user-ns=user:[4026533080]
       > SANDBOX-EVIDENCE uname=Linux localhost 7.4.0-rc3 #1-NixOS SMP PREEMPT_DYNAMIC @315532800 x86_64 GNU/Linux
       > SANDBOX-EVIDENCE usr-bin-env=absent
       > S1-VENUE-BUILD-START mode=unshare
       > S1-VENUE-PATH=/nix/store/kwyi4bq5s2qgwbhz6cs4gjy57c2zdxws-bash-interactive-5.3p20/bin:/nix/store/cb987zkxvsv0490753ilrvn5yk7ylgxi-coreutils-9.12/bin:/nix/store/2lgg8jqyzjp60svh7kmp39kqwnig31p2-util-linux-2.42.4-bin/bin:/nix/store/dzvnbi2n1x9jkmzvmwxdly607bwaxsbv-plan9port-0-unstable-2025-11-10/bin:/nix/store/9wc73n5izv0m7ppjbibi4hbr68rba1lj-gnused-4.10/bin:/nix/store/h96s16mjssibkvk62pvmfm3lwfc3wnwp-gawk-5.4.1/bin:/nix/store/c3f45pxlb81gzz4w1nym7lpznfypva7s-gnugrep-3.12/bin:/nix/store/nzsflqrk0zdljly9jhx82l784hl5rmc4-diffutils-3.12/bin:/nix/store/dl38pix4jvabi5smb9wywhp6pimmpva6-patchelf-0.19.2/bin:/nix/store/cb987zkxvsv0490753ilrvn5yk7ylgxi-coreutils-9.12/bin:/nix/store/99v13ig4382mngi6ggf0csqi7rfa04j5-findutils-4.11.0/bin:/nix/store/9mpja553cbkr2rc37kzdg8zbbg2dyyvl-diffutils-3.12/bin:/nix/store/9wc73n5izv0m7ppjbibi4hbr68rba1lj-gnused-4.10/bin:/nix/store/c3f45pxlb81gzz4w1nym7lpznfypva7s-gnugrep-3.12/bin:/nix/store/h96s16mjssibkvk62pvmfm3lwfc3wnwp-gawk-5.4.1/bin:/nix/store/n318yf1q5vysgh2qn6fd1q8yjy9vmwvh-gnutar-1.35/bin:/nix/store/rpswfv0ff4wap9anx7k1224x2m94pfcc-gzip-1.15/bin:/nix/store/ai2km55ba8bxrn64xx7p9lgvrrzdzv43-bzip2-1.0.8-bin/bin:/nix/store/8xz4d9wp4jy49mhl306wyg55096wis3p-gnumake-4.4.1/bin:/nix/store/nnm9gdr70ihc6zc9298mzz8jpcgaciql-bash-5.3p20/bin:/nix/store/d5jz315vxbphknfawsrjs192ci9bc6df-patch-2.8/bin:/nix/store/nflpm51il93hxh2rba6dvz59kaff0i7v-xz-5.8.4-bin/bin:/nix/store/2qpmak7162vhqxjwfik4aqs9lff6sn7w-file-5.48/bin:/nix/store/dzvnbi2n1x9jkmzvmwxdly607bwaxsbv-plan9port-0-unstable-2025-11-10/plan9/bin
       > S1-VENUE-NSX=/build/nsroot.nHa578 (private tmpfs)
       > mount: /build/nsroot.nHa578/nix/store: wrong fs type, bad option, bad superblock on /nix/store, missing codepage or helper program, or other error.
       >        dmesg(1) may have more information after failed mount system call.
       > S1-VENUE-RC=32
       > S1-VENUE-FAIL the guest run exited 32
       For full logs, run:
         nix log /nix/store/3sp3xvr6psh8swb49fpwj44mwy4dncny-s1-nsprobe-0.drv

~~~

### out/s3/rio.log

~~~
churngate: heap exit 3430528 33554432
cpurc: FACTOTUM-OKrio-gate: stage ok — factotum-ok

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
rio-gate: stage ok — listen-17010
process 46 exited: ""
churngate: heap exit 4149248 33554432
cpurc: DONE
rio-gate: stage ok — ladder-done
spawn bash -c exec -a r9s3riogate-dt-8431-1795398552 /nix/store/y8h5ana2drnzs2fdx6fxgr74bwmzi8y9-drawterm-19.06/bin/drawterm -s '' -O cpu=127.0.0.1,auth=tcp!127.0.0.1!5670,user=glenda
rio-gate: spawned drawterm on a pty (mode rio, argv0 tag r9s3riogate-dt-8431-1795398552, user glenda)
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

~~~

### out/s5/q35-kvm.log

~~~

Plan 9
mmuinit: vmstart 0xfffffffff0000000 vmunused 0xfffffffff02f6000 vmunmapped 0xfffffffff0400000 vmend 0xfffffffffe000000
sys->pd 0x108003 0x108023
l 3 0xffffff7fbfdfeff8 107023
l 2 0xffffff7fbfdffff8 108023
l 1 0xffffff7fbffffc00 e3
l 1 0xffffff7fbffffc00 e3
kernel alloc space = -268,435,456
base 0xfffffffff02fa000 ptr 0xfffffffff02fa000 nunits 14484992
apic0: hz 997351810 max 9973518 min 99735 div 2
pcirouting: ignoring south bridge PCI.0.1.0 1234/1111
intrenable: nil handler for 11, tbdf 0xc001000 for ether0
#l0: virtio: 1Gbps port 0xc040 irq 11: 525400123456
508M memory: 1944K+230M kernel, 276M user
root is from (tcp)[tcp]: 
~~~

### out/s5/q35-kvm-stdout.log

~~~
S5-CMD: /nix/store/bp9li432ba8adp47dr9z9y0hx56wn8v2-qemu-11.1.2/bin/qemu-system-x86_64 -M q35 -accel kvm -cpu host -m 512 -kernel ./s5/9k10cpu.elf -display none -no-reboot -serial file:/home/req/r9-spike-work/witness-bundle/out/s5/q35-kvm.log -netdev user,id=n0 -device virtio-net-pci,netdev=n0
qemu-system-x86_64: terminating on signal 15 from pid 8351 (sh)
BANNER=yes
PROMPT=root is from (tcp)[tcp]: 
SIGS=none
NIC=#l0: virtio: 1Gbps port 0xc040 irq 11: 525400123456
BANNER_SECONDS=1
PROMPT_SECONDS=1
S5-RESULT machine=q35 accel=kvm net=virtio banner=yes prompt=yes sigs=none

~~~

