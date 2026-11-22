# Phase 18: In-Guest p9sk1 Auth Domain - Pattern Map

**Mapped:** 2026-11-13
**Files analyzed:** 24 (10 Harvey new, 8 Harvey modified, 6 r9 new/modified groups)
**Analogs found:** 24 / 24 (every file has an in-tree analog; 2 are composites)

Dual-repo note: Harvey paths are relative to `/home/req/code/harvey`; r9 paths are relative to `/home/req/code/r9` (written `r9/...` below). All analogs verified git-tracked (`git ls-files` non-empty in the owning repo; r9 HEAD @ 8f3ff11).

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `r9/port/src/devcap.rs` (new) | kernel device | CRUD (post/consume hash table) | `r9/port/src/devsrv.rs` + donor `sys/src/9k/port/devcap.c` | exact (structure + semantics composite) |
| `r9/port/src/*hmacsha1*.rs` (new, hand-rolled SHA-1/HMAC) | kernel utility | transform | `r9/port/src/devcons.rs:963-1006` (hand-rolled ChaCha20) | role-match |
| `r9/port/src/namec.rs` (modify: `'¤'` arm) | kernel route | request-response | `r9/port/src/namec.rs:60-82` (registry_attach arms) | exact (self, additive) |
| `r9/x86_64/src/entropy.rs` (new; rng_seed/jitter impls) | kernel service | streaming (entropy) | `r9/aarch64/src/platform/virt/devcons.rs:89-131` + `r9/x86_64/src/cpu.rs:164-216` | exact composite |
| `r9/x86_64/src/main.rs` (modify: hooks registration + armed-source print) | kernel config (boot) | event-driven | `r9/aarch64/src/main.rs:562-568` + `r9/x86_64/src/main.rs:425,472` | exact |
| `r9/x86_64/src/proc.rs` + `r9/port/src/dat.rs` (modify: per-proc user) | kernel model | — | `r9/x86_64/src/proc.rs:978-1000` (Proc struct) | exact (self, additive field) |
| `r9/port/src/devcons.rs` (modify: `/dev/user` from per-proc user, eve gate) | kernel device | — | `r9/port/src/devcons.rs:177-181,317-323` (HOSTOWNER, USER_CONTENT) | exact (self) |
| `r9/x86_64/lib/config_q35*.toml` (new substrate profile) | config | — | `r9/x86_64/lib/config_q35net.toml` | exact |
| `r9/Cargo.toml` (modify: check-cfg predeclare) | config | — | `r9/Cargo.toml:35-39` (cfg(dev_*) list) | exact (self, additive) |
| `r9/xtask/src/main.rs` (modify: substrate boot profile + markers) | test harness | event-driven | `r9/xtask/src/main.rs:803-832,895-970` (marker consts + budgets) | exact (self, additive) |
| `r9` host pins (`port/src/devcap_tests.rs` etc.) | test | transform | `r9/port/src/devip_tests.rs:1-30` (crate-root `*_tests.rs` pin homes) | exact |
| `build/x86_64/authdomain-gen.c` (new; name planner's) | utility (host build tool) | transform (key material → paired artifacts) | `sys/src/cmd/auth/keyfs.c:714-795` (byte format) + `build/x86_64/svctcpecho.c` (host-C harness shape) | exact composite |
| committed interface for gitignored key file + `.gitignore` entry (new) | config | file-I/O | `build/x86_64/root-tree/lib/netaddr.rc` (committed key=value fixture family) | role-match |
| `build/x86_64/authprobe.c` (new; speaksfor leg-2 client) | utility (guest C client) | request-response (p9sk1 Treq) | `sys/src/cmd/auth/debug.c:209-267` (authfutz) + `build/x86_64/svctcpecho.c` | exact composite |
| `build/x86_64/mkimage.sh` (modify: sixth auth contract) | config/build script | batch (bake) | `build/x86_64/mkimage.sh:762-1062` (services contract §11) | exact |
| `build/x86_64/tiers/tier2-services.txt` (modify: `cmd/aux:listen1` leaf, maybe ramfs) | config (tier data) | — | `build/x86_64/tiers/tier2-services.txt:52` (`cmd/aux:listen`) | exact |
| `build/x86_64/auth-termrc` (new; termrc override) | config (boot script) | event-driven | `build/x86_64/services-termrc` | exact |
| `build/x86_64/auth-sweep.rc` (new; bring-up + probe, baked at /lib) | test (in-guest probe) | request-response | `build/x86_64/services-sweep.rc:133-376` (loopback/cs/factotum/keyfs/authsrv blocks) | exact |
| `build/x86_64/root-tree/lib/ndb/local.template` (modify: AUTH-05 rows) | config (data template) | — | itself — header lines 16-20 document the Phase-18 ipnet= extension point | exact (self) |
| `build/x86_64/root-tree/lib/netaddr.rc` (modify: ether key, if seam-homed) | config (data seam) | — | itself — leg-prefixed key sections (lo_/slirp_/tap_) | exact (self) |
| `build/x86_64/smoke-r9-auth.expect` (new; auth expect gate) | test (gate) | request-response | `build/x86_64/smoke-r9-net.expect` | exact |
| `nix/smoke-r9-auth-x64.sh` (new; gate driver) | config (gate driver) | batch | `nix/smoke-r9-net-x64.sh` | exact |
| `flake.nix` (modify: auth gate app) | config | — | `flake.nix:237-242` (netSmokeR9X64Pkg) | exact |
| `.github/workflows/r9-shell.yml` (modify: auth CI leg) | config (CI) | — | `.github/workflows/r9-shell.yml` net-smoke-x64 job (`env R9_ACCEL=tcg nix run .#boot-r9-net-smoke-x64`) | exact |
| `build/x86_64/README.md` AUTH section + `.planning/PROJECT.md` row (modify) | doc | — | `build/x86_64/README.md` §"The net modes" (16-07 extend-in-place home) | exact |

## Pattern Assignments

### `r9/port/src/devcap.rs` (kernel device, CRUD)

**Structure analog:** `r9/port/src/devsrv.rs` (300 lines — the small-device port pattern). **Semantics donor:** `sys/src/9k/port/devcap.c` (287 lines, read in full this session).

Copy the devsrv module skeleton: donor-mapping doc header, `REGISTRY_RUNE`, `Lock<(Vec<Entry>, u64)>` static state, free functions (attach/walk/open/read/write/stat) dispatched via namec, `write_stat_record` helper. Donor semantics to port (devcap.c lines cited):

**Dirtab — caphash must be LAST** (devcap.c:42-49) — `capremove` by eve on Qhash sets `ncapdir = nelem-1`, permanently hiding the file:
```c
Dirtab capdir[] =
{
	".",		{Qdir,0,QTDIR},	0,		DMDIR|0500,
	"capuse",	{Quse},		0,		0222,
	"caphash",	{Qhash},	0,		0200,
};
```

**FIFO trim at Maxhash=256** (devcap.c:145-175) — drop OLDEST (head of list), append at tail; ticks recorded but NEVER checked (no time-based expiry — do not invent one):
```c
static void
addcap(uchar *hash)
{
	...
	p->ticks = m->ticks;
	qlock(&capalloc);
	/* trim extras */
	while(capalloc.nhash >= Maxhash){
		t = capalloc.first;
		if(t == nil)
			panic("addcap");
		capalloc.first = t->next;
		free(t);
		capalloc.nhash--;
	}
	/* add new one */
	for(l = &capalloc.first; *l != nil; l = &(*l)->next)
		;
	*l = p;
	capalloc.nhash++;
	qunlock(&capalloc);
}
```

**capuse write — consume-on-match + user mutation** (devcap.c:215-257) — the AUTH-03 core; `strrchr` LAST `@` splits key, `hmac_sha1(data="from@to", key=random)`, `remcap` consumes, FIRST `@` splits from/to, `strcmp(from, up->user)` gate, `kstrdup(&up->user, to)`:
```c
	from = cp;
	key = strrchr(cp, '@');
	if(key == nil)
		error(Eshort);
	*key++ = 0;

	hmac_sha1((uchar*)from, strlen(from), (uchar*)key, strlen(key), hash, nil);

	p = remcap(hash);
	if(p == nil){
		snprint(err, sizeof err, "invalid capability %s@%s", from, key);
		error(err);
	}

	/* if a from user is supplied, make sure it matches */
	to = strchr(from, '@');
	if(to == nil){
		to = from;
	} else {
		*to++ = 0;
		if(strcmp(from, up->user) != 0)
			error("capability must match user");
	}

	/* set user id */
	kstrdup(&up->user, to);
	up->basepri = PriNormal;
```

**devsrv structure excerpt to copy** (devsrv.rs:33-59, 87-93):
```rust
extern crate alloc;
use alloc::vec::Vec;
use crate::dat::{Chan, Qid, DEV_SRV, OREAD, ORDWR, OTRUNC, OWRITE};
use crate::mcslock::{Lock, LockNode};

pub const REGISTRY_RUNE: u8 = b's';
const QTDIR: u8 = 0x80;
const QTFILE: u8 = 0x00;

static SRV: Lock<(Vec<SrvEntry>, u64)> = Lock::new("devsrv", (Vec::new(), 1));

pub fn attach() -> Chan {
    let mut c = Chan::new(DEV_SRV, Qid { path: 0, vers: 0, typ: QTDIR }, 0);
    c.path = b"#s".to_vec();
    c
}
```

Kernel prerequisites (verified absent, this phase adds): per-proc user (Proc has none — see below), eve comparison (devcons `HOSTOWNER` static vs current user), HMAC-SHA1 (see next assignment). Errors: donor strings `Eperm`/`Eshort`/`"invalid capability %s@%s"`/`"capability must match user"` carried verbatim into r9 `Err(...)` strings (the donor-fidelity rule).

---

### `r9/port/src/*hmacsha1*.rs` (kernel utility, transform)

**Analog:** the hand-rolled ChaCha20 in `r9/port/src/devcons.rs:939-1006` — the "no new crate" kernel discipline (`forbid`-attribute module, pure safe Rust, RFC-cited doc comments):
```rust
/// The ChaCha20 state constants — "expand 32-byte k" (RFC 8439 §2.3).
const CHACHA20_CONST: [u32; 4] = [0x6170_7865, 0x3320_646e, 0x7962_2d32, 0x6b20_6574];

/// The ChaCha20 quarter-round (RFC 8439 §2.1): add-xor-rotate on the
/// four named state words.
fn quarter_round(s: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize) {
    s[a] = s[a].wrapping_add(s[b]);
    s[d] = (s[d] ^ s[a]).rotate_left(16);
    ...
```
Mirror this exactly: RFC 3174 (SHA-1) / RFC 2104 (HMAC) doc cites, `#[cfg(test)]` host pins for the known-answer vectors beside the implementation (or a crate-root `*_tests.rs` sibling — see the test assignment below). SHA-1 here is the donor cap-format contract, not a crypto choice.

---

### `r9/x86_64/src/entropy.rs` (kernel service, streaming)

**Impl analog:** `r9/aarch64/src/platform/virt/devcons.rs:85-131` — the existing RandomHooks implementations this phase re-keys (the x86_64 version replaces DTB read with RDRAND/RDSEED/TSC):
```rust
pub fn rng_seed_read(buf: &mut [u8]) -> usize {
    ... // copies /chosen/rng-seed bytes; 0 when absent
}

/// devcons's jitter hook: CNTFRQ ^ CNTPCT ^ the 1 ms tick count. ...
pub fn random_jitter() -> u64 {
    use aarch64_cpu::registers::{CNTFRQ_EL0, CNTPCT_EL0, Readable};
    CNTFRQ_EL0.get() ^ CNTPCT_EL0.get() ^ super::timer::ticks()
}
```
x86_64 jitter equivalent: `rdtsc()` (cpu.rs:192-199) sampled twice with a fold (mirroring `drbg_mix_jitter`'s two-sample decorrelation, devcons.rs:1065-1072) — TCG's qemu64 timing variance is the entropy source there (load-bearing in CI).

**Gating analog:** `r9/x86_64/src/cpu.rs:201-216` — the 13-03 CPUID-gated decision helpers, host-testable without a CPU. Mirror as `has_rdrand(cpuid1_ecx)` (bit 30) / `has_rdseed(cpuid7_ebx)` (bit 18, check max-leaf first):
```rust
/// Pure decision helper: does leaf-1 ECX bit 21 (`CPUID_EXT_X2APIC`)
/// advertise x2APIC mode?  Host-testable without a CPU — lapic's mode
/// gate routes through this.  KVM's `-cpu host` passes the host's x2APIC
/// through ...; TCG's qemu64 never exposes the bit ...
pub fn has_x2apic(cpuid1_ecx: u32) -> bool {
    cpuid1_ecx & (1 << 21) != 0
}
```
The `cpuid()` asm (cpu.rs:164-185, rbx-rescued) and `rdtsc()` already exist — reuse; only an `rdrand`/`rdseed` asm wrapper is new (Intel retry-loop guidance; no in-tree wrapper exists).

**Registration analog:** `r9/aarch64/src/main.rs:562-568` — the exact call shape, plus x86_64's existing hook call sites `r9/x86_64/src/main.rs:425` (`set_pid_hook`) and `:472` (`set_cons_hooks`) for placement:
```rust
#[cfg(platform = "virt")]
{
    port::devcons::set_random_hooks(port::devcons::RandomHooks {
        rng_seed: platform::devcons::rng_seed_read,
        jitter: platform::devcons::random_jitter,
    });
}
```
The seam itself (`RandomHooks`, `set_random_hooks` single-registration assert, `random_read`'s None-arm) at `r9/port/src/devcons.rs:1029-1156` stays untouched; add the armed-source print beside the existing once-marker (`RANDOM_SEEDED_PRINTED`, devcons.rs:1057-1060, print at :1125-1127 — vocabulary planner's, e.g. `random: source=rdrand|jitter`).

---

### `r9/x86_64/src/proc.rs` + `r9/port/src/dat.rs` + `r9/port/src/devcons.rs` (per-proc user / eve)

**Analog (self, additive):** `r9/x86_64/src/proc.rs:978-1000` — the real Proc struct (pid/name/state/kstack/pml4/...; no user field today). Add `user: Vec<u8>` (or fixed `[u8; ANAMELEN]`), seeded `glenda`, copied at fork. `r9/port/src/dat.rs:340` `pub struct Proc {}` is the port stub — keep the port consuming an injected hook (the devsrv `ResolveHook` dependency-injection idiom, devsrv.rs:61-85), since port cannot see the arch proc table.

**`/dev/user` serve site:** `r9/port/src/devcons.rs:318-323` — replace the static const with the current proc's user (keep the NO-trailing-newline rule the comment documents):
```rust
/// The "user" file's content (the sane default: glenda — the auth stub
/// never runs a real handshake).  NO trailing newline (the
/// donor's Quser read is `readstr(up->user)` — the bare name): init's
/// readenv("#c/user") feeds newns's `$user` → `/usr/$user` verbatim, so a
/// newline would poison the home path.
const USER_CONTENT: &[u8] = b"glenda";
```
**eve comparison:** `r9/port/src/devcons.rs:177-181` `static HOSTOWNER: Lock<Vec<u8>>` (behind `#c/hostowner`, QID 4, 0664) — devcap's iseve gate reads this vs the current user.

---

### `r9/x86_64/lib/config_q35*.toml` + `r9/Cargo.toml` + `r9/xtask/src/main.rs` (substrate profile)

**Config analog:** `r9/x86_64/lib/config_q35net.toml` — whole shape: `[build]` target/buildflags, `[link]`, `[config] platform/dev list`, `[qemu]` machine/accel/cpu/smp/memory, `[dist]`. The substrate profile's `dev` list gains whatever keys gate devcap/entropy (`dev = ["cap"]` class), e.g.:
```toml
[config]
platform = "q35"
dev = ["ether"]
```
**check-cfg:** `r9/Cargo.toml:35-39` — the `unexpected_cfgs` list (`cfg(dev_panic)`, `cfg(dev_exec)`, `cfg(dev_bootchain)`, ...); the new dev key joins here (same commit as the TOML, the declared rule).

**xtask profile analog:** `r9/xtask/src/main.rs:803-832` (marker consts + one total wall-clock budget `BOOT_TEST_TIMEOUT_SECS`, TCG multipliers `TCG_FIRST_MARKER_MULT`/`TCG_OVERALL_MULT`, first-marker budget) and :895-970 (per-profile marker const blocks with the presence-never-order doc discipline). D-07's profile adds: random markers (read N bytes, distinct across reads, armed-source line) + devcap markers (post hash, consume via capuse, one-shot/hostowner asserts), both KVM + TCG legs:
```rust
/// Assertion markers, named so every profile reuses them.
const BOOT_BANNER_MARKER: &str = "r9 from the Internet";
...
/// Overall wall-clock budget for the whole marker sequence (overridable via
/// R9_BOOT_TEST_TIMEOUT); one total budget, never per-marker timings.
const BOOT_TEST_TIMEOUT_SECS: u64 = 300;
```
**Host pins analog:** `r9/port/src/devip_tests.rs:1-30` — crate-root `*_tests.rs` sibling files (`allocator_tests.rs`, `devether_tests.rs`, `rx_chain_tests.rs`), pin map enumerated in the module doc, STUB HOOKS over test-module statics. devcap's hash-table semantics and the entropy mixing logic get deterministic pins there BEFORE any boot gate depends on them.

---

### `build/x86_64/authdomain-gen.c` (utility, transform — the D-01/D-04 generator)

**Byte-format source of record (do not hand-roll):** `sys/src/cmd/auth/keyfs.c:713-795`. `oldCBCencrypt` (:714-724) and `writeusers` (:740-795) define `/adm/keys` exactly:
```c
void
oldCBCencrypt(char *key7, uchar *p, int len)
{
	uchar ivec[8];
	uchar key[8];
	DESstate s;

	memset(ivec, 0, 8);
	des56to64((uchar*)key7, key);
	setupDESstate(&s, key, ivec);
	desCBCencrypt((uchar*)p, len, &s);
}
```
```c
	buf = malloc(KEYDBOFF + nu*KEYDBLEN);
	...
	p = buf;
	randombytes(p, KEYDBOFF);
	p += KEYDBOFF;
	for(i = 0; i < Nuser; i++)
		for(u = users[i]; u; u = u->link){
			strncpy((char*)p, u->name, Namelen);
			p += Namelen;
			memmove(p, u->key, DESKEYLEN);
			p += DESKEYLEN;
			*p++ = u->status;
			*p++ = u->warnings;
			expire = u->expire;
			*p++ = expire;
			*p++ = expire >> 8;
			*p++ = expire >> 16;
			*p++ = expire >> 24;
			memmove(p, u->secret, SECRETLEN);
			p += SECRETLEN;
		}

	/* encrypt */
	oldCBCencrypt(authkey, buf, p - buf);
```
Compile/link the donor algorithm files (`sys/src/libauthsrv/passtokey.c`, `sys/src/libauthsrv/nvcsum.c`, `sys/src/libsec/port/desCBC.c` + des) rather than reimplementing — the Wave-0 A1 probe decides plan9port `-lauthsrv -lsec` linking vs vendoring these four self-contained files. Nvrsafe layout: `sys/include/authsrv.h:140-157` (109 bytes, five nvcsum-seed-9 sums; emit ALL five valid). Pairing invariant: `/adm/keys` encrypted under nvram `machkey` (`sys/src/cmd/auth/lib/getauthkey.c:14`).

**Host-C harness shape analog:** `build/x86_64/svctcpecho.c:1-26` — the existing host-side-built C tool's include block and style:
```c
#include <u.h>
#include <libc.h>
...
int connectonly;

void
usage(void)
{
	fprint(2, "usage: %s [-c] net!addr!port\n", argv0);
	exits("usage");
}
```
(The generator runs on the HOST at bake time — host randomness, never the guest DRBG. DEMO label lands in nvram authdom / keys header / ndb token / manifest row, D-04.)

---

### Committed interface for the gitignored key file (config, file-I/O)

**Analog:** `build/x86_64/root-tree/lib/netaddr.rc` — the committed key=value fixture family (single-source-as-data): one committed file defines named keys, every consumer derives from it, `#`-comment sections document each key group, and a mechanical grep assert enforces no literals elsewhere. The interface definition (field names/types for seed/passwords/users/label) follows this "committed data file + documented shape + bake assert" pattern; `.gitignore` gains the local-file entry (existing entries are bare paths/patterns, e.g. `/result`, `*.6`, `amd64/*bin`).

---

### `build/x86_64/authprobe.c` (utility, request-response — the D-13 speaksfor leg)

**Wire-exchange donor:** `sys/src/cmd/auth/debug.c:209-267` (`authfutz`) — the working p9sk1 client reference. The probe is the delta: `tr.hostid = <speaker>` DIFFERS from `tr.uid = <third user>` (debug.c:233-234 always self-speaks — that is why this client exists):
```c
	tr.type = AuthTreq;
	strecpy(tr.authid, tr.authid+sizeof tr.authid, user);
	strecpy(tr.authdom, tr.authdom+sizeof tr.authdom, dom);
	strecpy(tr.hostid, tr.hostid+sizeof tr.hostid, user);
	strecpy(tr.uid, tr.uid+sizeof tr.uid, user);
	memset(tr.chal, 0xAA, sizeof tr.chal);
	convTR2M(&tr, trbuf);
	if(_asgetticket(fd, trbuf, tbuf) < 0){
		...
	}
	convM2T(tbuf, &t, key);
	if(t.num != AuthTc){
		...
	}
	...
	print("\tticket request using %s@%s key succeeded\n", user, dom);
```
Assert shape per RESEARCH: `t.num == AuthTc && t.suid == thirduser` after decrypting with the SPEAKER's key; negative control `uid=!<denied>` row must fail decrypt. **Harness/flag style:** `svctcpecho.c` above (ARGBEGIN, alarm, `%r` diagnostics, `exits`). **Build precedent:** mkimage.sh §11k host-side 6c/6l (below).

---

### `build/x86_64/mkimage.sh` sixth auth contract (config/build script, batch)

**Analog:** the services contract, `build/x86_64/mkimage.sh:762-1062` (§11; also §12 churn at :1064-1145 as the minimal additive-contract shape). Section-by-section: 11a input asserts with remediation (`err "...missing — run sh build/x86_64/subset-build.sh first"`), 11b tier-data-computed staging (`svc_stage` duplicate-h* guard), 11e skeleton tar, 11g chain binaries from THIS build's products, 11i computed payload copy, 11k host-side 6c/6l builds, 11l sweep at /lib + termrc override, 11o printed manifest, 11r KERNDATE determinism + mkpaqfs cap assert, 11s header/trailer magic checks.

**Host-side 6c/6l build to copy for authprobe** (:960-973):
```sh
(	cd "$HARVEY" && \
		"$SVC_NS_HBIN/6c" -FTVw -o "$STAGE5/.svctcpecho.6" -Isys/include -Iamd64/include build/x86_64/svctcpecho.c && \
		"$SVC_NS_HBIN/6l" -o "$STAGE5/amd64/bin/hsvctcpecho" -L "$HARVEY/amd64/lib" "$STAGE5/.svctcpecho.6" && \
		rm -f "$STAGE5/.svctcpecho.6"
	) || err "hsvctcpecho host-side build failed"
magic=$(od -An -tx1 -N4 "$STAGE5/amd64/bin/hsvctcpecho" | tr -d ' \n')
[ "$magic" = "00008a97" ] || err "hsvctcpecho: bad magic $magic (want 00008a97)"
```

**Tier-entry presence asserts to copy for the auth subjects** (:821-824):
```sh
for e in cmd/mntgen cmd/ndb cmd/exportfs cmd/srv cmd/aux:listen cmd/auth/authsrv cmd/auth/keyfs; do
	"$GREP" -q "^$e[[:space:]#]" "$SVC_T2" || err "services bake: tier-2 data lacks the probe-subject entry $e (the sweep and the data diverged)"
done
```
New pairing assert (D-02 bake arm): manifest hashes vs staged nvram/keys/ndb artifacts — refuse on mismatch; join the existing assert families (tier-entry presence, staged==source, literal grep with negative controls). mkpaqfs per-file cap + fail-on-any-diagnostic (:1048-1052) and KERNDATE pinning stay (deterministic layers pinned, content per-bake — both true at once, Pitfall 10).

---

### `build/x86_64/tiers/tier2-services.txt` leaf line (config)

**Analog:** line 52 — `cmd/aux:listen		# USPACE-07/D-02: the services listener; ...` — the aux leaf-filter discipline (bare `cmd/aux` container line is FORBIDDEN, header lines 28-31). The new line is one leaf:
```text
cmd/aux:listen1		# 18: the flag-passing per-connection runner (authsrv -d rides); donor aux/listen1.c
```
(`cmd/ramfs` same shape IF the Wave-0 probe picks it for writable /adm/secstore — A3.)

---

### `build/x86_64/auth-termrc` + `build/x86_64/auth-sweep.rc` (boot script + in-guest probe)

**termrc analog — `build/x86_64/services-termrc` (whole file, 18 lines):**
```rc
#!/bin/rc
# services-termrc — the SERVICES image's /rc/bin/termrc ...

rc /lib/services-sweep.rc r9

exec rc -i
```
The auth image's termrc override is this shape with the auth sweep; the sweep runs in a CHILD rc (its trailing exit never kills init's shell).

**sweep analogs — `build/x86_64/services-sweep.rc` blocks (flat if-ladders, `$B/h*` helper vocabulary, SVC-OK/FAIL markers):**
- loopback + cs bring-up (:133-216): `bind -a '#I' /net`, ipifc clone + `bind loopback` + TWO addresses (127.0.0.1 AND a non-loopback — myipaddr skips loopback), `hcs -f <file>` with trailing newline
- factotum start + ctl key write (:297-304):
```rc
if($B/hfactotum -s factotum){
	if(svc_waitpath 10 /mnt/factotum/ctl){
		if($B/hecho 'key proto=pass service=test user=probe !password=probe-only-test' > /mnt/factotum/ctl){
			kout=`{$B/hcat /mnt/factotum/ctl}
			...
			if(~ $"kout *proto*pass*) echo SVC-OK factotum
```
- the keyfs sentinel-bind technique (:334-348) — READ-ONLY file bound over /dev/cons so readln's fresh OREADs return the fixed first line (auth/debug's password prompts are drivable this way; secuser is NOT — getpassm opens ORDWR and sysfatals):
```rc
if(! $B/htest -e /env/svckeys){
	if(bind /boot/sentinel.txt /dev/cons){
		if($B/hkeyfs -m /mnt/keys /env/svckeys){
```
- the listener probe (:356-365) — connect-accept shape via hsvctcpecho.

Boot-flow ordering locked by D-02/D-03: pairing re-verify (banner/abort) BEFORE `nvram=/adm/nvram` assignment BEFORE keyfs. rc landmines: plain `nvram=...` assignment on its own line is SAFE; never inside an argv word; flat status ladders; no braced if-not bodies; sourced-not-typed.

---

### `build/x86_64/root-tree/lib/ndb/local.template` + `netaddr.rc` (data templates, modify in place)

**local.template** — its own header (:16-20) documents the Phase-18 extension point; AUTH-05 rows compose from seam tokens, zero venue literals:
```
# Content model: the baked host entry is the same ip=/dom=/sys= tuple
# shape the 15-02 services probes measured cs answering from
# (`!ipinfo dom=host attr').  The mask and gateway ride as their own
# attribute tuples for the Phase-18 domain generator to compose into
# ipnet= blocks (authdom/auth entries) when a plan owns that layer.

ip=@GUESTV4@ sys=harvey dom=harvey.local
ipmask=@MASKV4@
ipgw=@GWV4@
```
New AUTH-05 content (authdom=/auth= — numeric addresses, the v2.0 Pitfall 12(d) rule) and the separate `/lib/ndb/auth` speaksfor rows (`hostid=<speaker> uid=!sys uid=!adm uid=*` — authsrv.c:791-821 consumes; missing file = fail CLOSED) ride the same token-substitution + literal-grep-assert discipline (15-D-16).

**netaddr.rc** — if the `ether=` value is seam-homed (OQ 4), it joins the leg-prefixed sections (:30-57: `lo_guestv4=...`, `slirp_guestv4=...`, `tap_guestv4=...`); the bake's venue-literal grep over root-tree/lib enforces the single home.

---

### `build/x86_64/smoke-r9-auth.expect` + `nix/smoke-r9-auth-x64.sh` + `flake.nix` + CI (gate, app-per-gate)

**expect analog — `build/x86_64/smoke-r9-net.expect` (505 lines):** copy the whole chassis: `set timeout 600` TCG-sized (:156), required-env foreach with exit 2 (:163-169), fresh-capture `exec rm -f` + `log_file` (:173-210), `stage` proc (:227-234), `cleanup` process-group kill (:217-221), `expect_before` panic fast-fail (:279-290), presence-never-order + two-occurrence marker discipline for typed-echo collisions (:104-120), whole-capture post-passes with `log_file` flush (:480-500). Auth gate additions: the D-02 banner stage BEFORE keyfs markers, the never-appears assert for `can't read NVRAM, please enter machine key` (the Pitfall 1 wedge marker), marker vocabulary from RESEARCH Pattern 3 (`tr-ok`/`tr-fail`/`cr-ok`/`cr-fail`, `keys read`, `ANNOUNCE`, `AUTH `, `secstore: saved`, the pairing banner) — match message tokens never dates (syslog's `<sysname> <ctime> ` prefix).
```tcl
proc stage {name re} {
    global expect_out
    expect {
        -re $re { puts stderr "smoke-r9-net-x64: stage ok — $name" }
        timeout { puts stderr "smoke-r9-net-x64: TIMEOUT at stage: $name"; cleanup; exit 1 }
        eof { puts stderr "smoke-r9-net-x64: qemu exited (EOF) at stage: $name"; cleanup; exit 1 }
    }
}
```

**driver analog — `nix/smoke-r9-net-x64.sh` (253 lines):** env contract checks (:113-130), accel selection with no-silent-downgrade (:134-144), KERNDATE provenance (:170-176), stage-build/subset-build steps with R9_SKIP_BUILD hatch (:180-187), bake + compose via R9_ISO_BUILDER (:191-226), unique `R9_RUN_TAG` + EXIT-trap pkill survivor sweep (:206-220), expect invocation with RC check (:244-251). The auth driver swaps the paq payload (bootfs-auth.paq), the expect file, the log default, and the tag prefix (the sibling-gate no-cross-kill rule, :90-93).

**flake app analog — `flake.nix:237-242` (netSmokeR9X64Pkg), with servicesR9X64Pkg :259-263 as the minimal-shape variant:**
```nix
netSmokeR9X64Pkg = mkScriptApp pkgs "harvey-boot-r9-net-smoke-x64" ./nix/smoke-r9-net-x64.sh ''
  export R9_KERNEL_X64_PKG="${r9.packages.${system}.kernel-x64}"
  export R9_ISO_BUILDER="${r9.packages.${system}.iso-builder}/bin/r9-iso-builder"
  export R9_NETDEV_EMITTER="${r9.packages.${system}.netdev-args}/bin/r9-netdev-args"
'' ([ pkgs.plan9port pkgs.bash pkgs.qemu pkgs.git pkgs.expect pkgs.iproute2 pkgs.procps ]
  ++ pkgs.lib.optionals isLinux [ pkgs.util-linux pkgs.gnused pkgs.mount pkgs.binutils ]);
```

**CI analog — `.github/workflows/r9-shell.yml` net-smoke-x64 job:** `timeout-minutes: 90` TCG scale, pinned checkout/nix-installer SHAs, `run: env R9_ACCEL=tcg nix run .#boot-r9-net-smoke-x64`. The auth leg is the same job shape with the auth app name (placement: join this leg vs a new leg is planner discretion).

---

## Shared Patterns

### Fail-loud build asserts (bake refuses, never warns)
**Source:** `build/x86_64/mkimage.sh:115` (`err() { echo "error: $*" >&2; exit 1; }`) + the assert families at :821-824, :965-973, :990-991, :1014, :1048-1052.
**Apply to:** the sixth contract's pairing assert, generator diagnostics, gate driver errors. Every `err` names the REMEDIATION ("run sh build/x86_64/subset-build.sh first").

### Single-source-as-data (committed data file + mechanical assert)
**Source:** `build/x86_64/root-tree/lib/netaddr.rc` (the D-11 seam; venue-literal grep), `build/x86_64/tiers/tier2-services.txt` (tier membership; bake-computed staging), `build/x86_64/bootfs-payload.txt` (manifest).
**Apply to:** the generator's committed interface file, the pairing manifest, AUTH-05 ndb content (tokens only, 15-D-16).

### Presence-based, timing-tolerant markers (never order, never timings)
**Source:** `r9/xtask/src/main.rs:811-832` (one total wall-clock budget, TCG multipliers) + :941-970 (per-profile marker consts, "PRESENCE, never interleaving"); `build/x86_64/smoke-r9-net.expect:104-120` (two-occurrence discipline for typed-echo collisions).
**Apply to:** the substrate boot profile (D-07), the auth expect gate, the boot pairing banner.

### The sentinel-bind (isolate interactive prompts)
**Source:** `build/x86_64/services-sweep.rc:312-348` — `bind /boot/sentinel.txt /dev/cons` makes readln's fresh OREADs return a fixed first line.
**Apply to:** auth/debug password prompts ONLY. NOT secuser (getpassm opens /dev/cons ORDWR + sysfatals — feed console via expect, redirect stdin for the rest); NOT the keyfs path in the auth image (D-02's check + nvram= make the happy path non-interactive; the sentinel is the negative-control tool).

### r9 hook registration (dependency-injection, single-registration assert)
**Source:** `r9/port/src/devcons.rs:1046-1055` (`set_random_hooks` + assert), `r9/port/src/devsrv.rs:63-85` (`ResolveHook`), `r9/x86_64/src/main.rs:425,472` (call sites).
**Apply to:** the per-proc user resolver port needs, the eve gate, the x86_64 entropy hooks registration.

### Donor-code-not-reimplementation (byte formats are contracts)
**Source:** RESEARCH "Don't Hand-Roll" table — `sys/src/libauthsrv/passtokey.c`, `sys/src/libauthsrv/nvcsum.c`, `sys/src/libsec/port/desCBC.c` (lacy partial-tail rule), `sys/src/cmd/auth/keyfs.c:714-795`, `sys/src/cmd/auth/debug.c:209-267`.
**Apply to:** the generator (compile the donor algorithm files), the speaksfor probe (extend authfutz, use libauthsrv codecs), devcap (port devcap.c semantics verbatim; FIFO aging not one-shot-per-post phrasing).

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `r9` RDRAND/RDSEED asm wrapper | kernel utility | streaming | No `rdrand` inline-asm wrapper exists in-tree (cpuid/rdtsc/wrmsr do, cpu.rs:164-231) — mirror the `rdtsc()` two-register asm shape; retry-loop semantics from the Intel DRNG guide (cited in RESEARCH) |
| Boot-time pairing re-verify (D-02 boot arm) | boot script leg | transform | No in-guest hash-verify precedent (bake-time digest asserts exist in mkimage.sh; the boot arm is new) — plan from RESEARCH's rc skeleton + the fail-loud abort idiom; in-guest `sha1sum` is already a tier member (mkimage.sh:950-951) |
| The gitignored local key file itself | config (data) | file-I/O | Deliberately uncommitted (D-01) — only its committed INTERFACE has analogs (netaddr.rc family above); nothing to copy, everything to define |

All three have RESEARCH.md coverage (Code Examples + Assumptions) — planner should use RESEARCH patterns for them.

## Metadata

**Analog search scope:** Harvey `build/x86_64/` (mkimage.sh, sweeps, termrc, tiers, root-tree, expect files), `nix/`, `flake.nix`, `.github/workflows/`, `sys/src/{9k/port,cmd/auth,libauthsrv,libsec,cmd/aux}`; r9 `port/src/`, `x86_64/src/`, `x86_64/lib/`, `aarch64/src/`, `xtask/src/`, root `Cargo.toml`.
**Files scanned:** ~30 read (full or targeted); all analog paths verified git-tracked in their owning repo (r9 @ 8f3ff11 = origin/main, matching STATE's Phase-17 terminus).
**Pattern extraction date:** 2026-11-13
