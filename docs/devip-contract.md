# The devip Contract — r9's `/net` File-Semantics Specification

This is the D-03 proof instrument and the D-08 cost-center deliverable: the complete enumerated
devip surface that r9's `#I` device implements over smoltcp 0.14, and the r9-side design decisions
the phase gates assert.  Gates assert against THIS document; if an implementation and this document
disagree, one of them is wrong and the disagreement is a gate red.

**Authority rule.** The enumeration derives from `sys/man/3/ip`, `sys/man/2/ip`,
`sys/man/8/ipconfig` (the documented surface) CROSSED WITH the Harvey donor source
`sys/src/9k/ip/` (the behavioral truth).  Source beats man wherever they disagree; every such
disagreement is flagged **[man-vs-src]** rather than smoothed over.  Donor line citations are
`sys/src/9k/ip/` paths in the Harvey checkout.  smoltcp claims are verified against the 0.14.0
crate (docs.rs + crate source + the 2026-11-05 no_std compile probe on r9's pinned toolchain).

**No implementation code here** — format strings, constants, error strings, and decision records
only.

---

## 1. Stack topology and the qid scheme

Attach: `bind -a #I<spec> /net`.  The man documents `<spec>` as an integer 0..15 identifying a
stack.  The source allocates `Nfs = 128` stack slots (`devip.c:44`): `devno = atoi(spec)`,
`error("bad specification")` when `devno >= Nfs` (`devip.c:298-316`).  **[man-vs-src]** — the
source's bound is 128, not 15.

* **r9 bound:** r9 implements ONE stack this era — only spec `0` (or the empty spec, which
  `atoi` maps to 0) attaches; any other spec errors `"bad specification"`.  The qid packing keeps
  the donor's field widths so a multi-stack era changes only the stack-slot count.

Each stack implements IPv4 AND IPv6; stacks are independent.  All addresses are 16-byte IPv6
form; v4 is carried mapped under the 12-byte `v4prefix` (`00..00 ff ff`).

The tree is three levels: the top directory, one directory per protocol, one directory per
conversation.  Qids are packed (`devip.c:35-49`):

```
QID(p,c,y) = ((p)<<Shiftproto) | ((c)<<Shiftconv) | (y)
Logtype  = 5   (Masktype  = 31;   32 file types)
Logconv  = 12  (Maskconv  = 4095; 4096 convs per proto)
Logproto = 8   (Maskproto = 255;  256 protos)
Shiftconv  = 5
Shiftproto = 17  (= Logtype + Logconv)
```

Tag constants (`devip.c:9-33`):

```
Qtopdir    = 1                     # top-level directory
Qtopbase   = 2
  Qarp     = 2   Qndb      = 3   Qiproute = 4   Qipselftab = 5   Qlog = 6
Qprotodir  = 7                     # a protocol directory
Qprotobase = 8
  Qclone   = 8   Qstats    = 9
Qconvdir   = 10                    # a conversation directory
Qconvbase  = 11
  Qctl     = 11  Qdata     = 12  Qerr   = 13  Qlisten = 14
  Qlocal   = 15  Qremote   = 16  Qstatus = 17  Qsnoop = 18
```

Extraction: `TYPE(q) = q.path & Masktype`, `CONV(q) = (q.path >> 5) & Maskconv`,
`PROTO(q) = (q.path >> 17) & Maskproto`.  devip's packing is its OWN scheme — it differs from
devether/netif's `NETQID`; r9's devip module keeps this packing and never reuses netif's.

---

## 2. Top-level files (per stack)

| File | Mode | Owner | Read | Write |
|------|------|-------|------|-------|
| `arp` | 0664 | network | one 90-byte line per entry (below) | `flush`, `add`, `del` |
| `iproute` | 0664 | network | one line per route (below) | `flush`, `tag`, `add`, `remove`, `route` |
| `ipselftab` | 0444 | network | one line per local address (below) | — (write → `Eperm`) |
| `ndb` | 0666 | network | up to 1024 bytes of free-form text | append-at-offset semantics (below) |
| `log` | 0666 | network | blocks until netlog output | `set` / `clear` / `only` |
| `bootp` | — | — | **ABSENT — [man-vs-src]** | — |

### `arp`

Read: fixed 90-byte lines (`arp.c:518-556`):

```
Alinelen = 90
aformat  = "%-6.6s %-8.8s %-40.40I %-32.32s\n"   # medium, state, ip, mac
```

State is one of `OK`, `WAIT` (`arpstate[]`, `arp.c:24-29`; state 0/`UNUSED` entries are skipped).
A read with `offset % 90 != 0` returns 0.  The table is `NCACHE = 256` entries, LRU replacement,
**no timeout**; hash `NHASH = 64`.  Entries do not expire.

Write verbs (`arp.c:425-516`):

* `flush` — zero every entry, drop held packets and the retransmit/drop chains.
* `add ether <ip> <mac>` — the 4-field form names the medium (`ipfindmedium`, nil → `Ebadarp`);
  a 3-field `add <ip> <mac>` form exists in the source and resolves the medium via a route lookup
  (no route → `error("Destination unreachable")`).  r9 implements the documented 4-field form
  (`add ether <ip> <mac>`) plus the 3-field form with the same route-mediated medium pick.
* `del <ip>` — remove one entry.

Bad verb / bad arg count → `Ebadarp` (`"bad arp"`) / `Ebadarg`; unparseable address → `Ebadip`.
See §12 for the r9 mapping: the file is a snoop-fed VIEW over smoltcp's (accessor-less) neighbor
cache; the static-add divergence is recorded there.

### `iproute`

Read: one line per route (`iproute.c:612`, `rformat`):

```
"%-15I %-4M %-15I %4.4s %4.4s %3s\n"   # target mask nexthop flags tag ifc
```

Flags (`routetype`, `iproute.c:592-610`; `4`/`6` is the version arm, `u`/`b`/`m` are exclusive):
`4` IPv4 route, `6` IPv6 route, `i` local interface (Rifc), `u` local unicast (Runi),
`b` broadcast (Rbcast), `m` multicast (Rmulti), `p` point-to-point (Rptpt).  Tag is an arbitrary
≤4-character string naming the originating protocol.  Longest-mask match wins; lookup key is
`dst AND mask == target`.

Write verbs (`iproute.c:802-880`): `flush` (remove all routes) / `tag <string>` (bind the tag to
routes added via this fd) / `add <target> <mask> <nexthop>` (replace on same target+mask) /
`remove <target> <mask>` / `route <target>` (print the chosen route on the console — a debugging
aid).

### `ipselftab`

Read-only; one line per IP address considered local (`ipifc.c:1028-1052`):

```
"%-44.44I %2.2d %4.4s\n"   # address, interface use-count, flags (same letters as iproute)
```

The `4` flag is never set (self entries are version-agnostic).  Write → `Eperm`.

### `ndb`

The stack's free-form configuration note, `Fs.ndb[1024]` (`ip.h:442`, `devip.c:367-371`,
`devip.c:1429-1441`).  Read is a plain string read of up to 1024 bytes; the directory entry's
`qid.vers` is `f->ndbvers` (bumps on every write — readers detect change).  Opening
`OWRITE|OTRUNC` zeroes the buffer first.  Writes `memmove` at the given offset and NUL-terminate;
a write with `off > strlen(ndb)` or `off+n >= 1024` errors `Eio`.  The donor gates
`OWRITE|OTRUNC` on `iseve()` (hostowner); **r9's single-user collapse drops that check** — the
era's documented shape (same collapse as devether).  ipconfig writes the ndb record on the first
configured interface.

### `log`

The netlog ring (`Nlog = 16*1024`, `netlog.c`).  Read BLOCKS until netlog output is available
(bad-packet and error messages; content is implementation-defined — the man says so).  Writes
(`netlogctl`, `netlog.c:37-69`):

* `set <flags…>` / `clear <flags…>` — flag names: `ppp ip fs tcp icmp udp compress gre tcpwin
  tcprxmt udpmsg ipmsg esp`.  (`tcpwin` = `Logtcp|Logtcpwin`, `tcprxmt` = `Logtcp|Logtcprxmt`,
  `udpmsg` = `Logudp|Logudpmsg`, `ipmsg` = `Logip|Logipmsg` — the message flags OR into their
  protocol bit.)
* `only <addr>` — restrict debug output to packets whose source or destination is `<addr>`
  (zero address clears the restriction).

Too few arguments → `Ebadnetctl` = `"too few arguments for netlog control message"`.

### `bootp` — **[man-vs-src]** documented-but-absent

`/net/bootp` appears in the man's SYNOPSIS but has no qid in the source — devip.c defines no
bootp file (the vestigial `bootpread` prototype in `ip.h` has no file surface).  Recorded ABSENT:
r9 implements no `/net/bootp` and no stub; a walk to the name fails with the ordinary
file-does-not-exist error.

---

## 3. Protocol directories and conversation semantics

Protocols present this phase: **`tcp`, `udp`, `icmp`, `icmpv6`, `ipifc`**.

Dropped per D-08 (simply ABSENT — no stub directories): **`gre`, `esp`, `rudp`, `igmp`,
`ipmux`**, and the **`pktmedium` / `netdevmedium`** mediums (the man's `bind pkt` /
`bind netdev <path>` forms error `"unknown interface type"`).  A walk to `/net/gre` fails with
the ordinary file-does-not-exist error; nothing in the tree hints the name ever existed.

Each protocol directory contains:

* `clone` — 0666, owner `network`.  Opening it allocates a conversation (`Fsprotoclone`,
  `devip.c:1273-1343`): the first free conv slot, or — under pressure — a protocol `gc` pass and
  retry.  The returned fd IS the new conv's `ctl` file.  Exhaustion: the donor panics when
  compiled as a cpu server (`cpuserver`; `scalednconv` = `4*Nchans` = 4096 there, `Nchans` = 1024
  otherwise) and returns nil otherwise, which `ipopen` maps to **`Enodev`**.  **r9 has no
  cpuserver flag: clone exhaustion is always `Enodev`** — recorded divergence, and
  `setlport`'s `"panic: out of ports"` tail maps the same way (see §4).
* `stats` — 0444, proto-specific tagged lines (per-proto chapters below).
* One numbered directory per allocated conversation.

Each conversation directory contains `ctl`, `data`, `err`, `listen`, `local`, `remote`,
`status` — and **`snoop` only for ipifc convs** (0400; `ip3gen` rejects snoop for any other
proto, `devip.c:95-98`).  Snoop is ipifc-only and **ungated this phase** (RESEARCH A4): it exists
in the tree with correct qid/perms, but no milestone consumer exercises it and no gate asserts
its data path.

Conversation lifecycle:

* **Clone/open:** `open(.../clone)` allocates the conv; `Fsprotoclone` sets owner = the cloner,
  perm = 0660, state = Idle, laddr/raddr = `IPnoaddr`, lport/rport = 0, ttl = `MAXTTL`,
  maxfragsize = 0, queues reopened.  Reading `ctl` returns the conv number as `"%.lud"` decimal
  text (`devip.c:642-647`; the man's "a text string representing the number").
* **Owner collapse:** the `clone` file itself is 0666/`network`; the new conv's files are 0660
  with the cloner as owner, and the FIRST open of `ctl`/`data`/`err` re-affirms owner =
  attacher, perm 0660 (`devip.c:436-440`).  The donor's `iseve()` perm checks collapse in r9's
  single-user era.  `ipwstat` on `ctl`/`data` may change owner/mode (`devip.c:515-544`).
* **Close:** last close runs `closeconv` (`devip.c:546-577`): pending incall convs are closed,
  multicast memberships dropped, state = Idle, owner = `"network"`, perm 0660, protocol close,
  queues reopened for the next cloner.
* **Conv states** (`ip.h:71-75`): `Idle=0, Announcing=1, Announced=2, Connecting=3, Connected=4`.

Per-file read semantics:

* `ctl` → the conv number (above).  `local` / `remote` → `"%I!%d\n"` (address!port), or the
  proto's override.  `status` → proto-specific lines (chapters below).
* `err` → the conv's error queue (`eq`, Qmsg — datagram semantics).  UDP ICMP advice hangups land
  here (§6).  On error or EOF on `data`, `err` carries the reason (the man).
* `listen` — `open(N/listen)` requires `state == Announced`, else `error("not announced")`;
  blocks until an incoming call arrives (`sleep(&cv->listenr, incoming, cv)`); a hangup of the
  waiting conv's read queue errors `"listen hungup"`; on success the returned chan points at the
  NEW conv's `ctl` (`devip.c:445-495`).  Incoming-call queue depth is **`Maxincall = 128`** per
  announced conv (`ip.h:40`) — the smoltcp no-backlog bridge is §12's accept pool decision.
* `data` — TCP: byte stream.  UDP/ICMP/ICMPv6: Qmsg datagram-at-once; a short read discards what
  doesn't fit (the man's UDP text).

---

## 4. Generic conversation ctl verbs

Dispatch (`devip.c:1115-1176`): an empty write errors `"short control request"`; an unrecognized
verb errors `"unknown control request"` (also the proto-ctl fallthrough string, `tcp.c:3344`).
All verbs parse through `parsecmd` (whitespace-separated fields).

* `connect <ip>!<port>[!r] [<local>]` — active open.  State must be Idle else `Econinuse`;
  state → Connecting, the proto connect runs, then the write **BLOCKS** on `c->cr` until
  Connected (`devip.c:951-977`); failure arrives via `c->cerr` (`Econrefused`, `Etimedout`, …).
  Local port selection (`setlport`, `devip.c:778-825`): with `!r`, a RESTRICTED port cycling
  600..1024; otherwise a **random port in [32768, 65536)** via `nrand` — **[man-vs-src]**: the
  man says "a free port number starting at 5000 is chosen" and "port numbers must be in the range
  1 to 32767"; the source randomizes in the high range.  A local/remote address+port quad already
  assigned to another conv fails `"address in use"`.
* `announce <X>` — X = decimal port or `*` (wildcard: calls to any port no process explicitly
  announced; the donor gates `*` on iseve — single-user collapse).  Sets laddr = `IPnoaddr`.
  State must be Idle else `Econinuse`; state → Announcing, proto announce, BLOCKS on `c->cr`
  until Announced (`devip.c:1004-1030`).  "Announce fails if the connection is already announced
  or connected" (the man; source: the `Econinuse` state check).
* `bind <X>` — set local port only (`Fsstdbind`); exists for APE sockets emulation, unused
  natively (the man).  Bad arg count → `"bad args to bind"`; the announce/connect equivalents are
  `"bad args to announce"` / `"bad args to connect"`.
* `ttl <n>` / `tos <n>` — set the IP ttl / tos fields; with no argument, ttl resets to `MAXTTL`
  and tos to 0 (`devip.c:1061-1074`).  **`MAXTTL = 255` in this tree** (`ip.h:44`) — note the
  phase research's "64" parenthetical is wrong; the SOURCE is 255, and smoltcp's
  `set_hop_limit` default (64) must NOT leak through: the bridge sets hop limit 255 at conv
  creation (§12).
* `ignoreadvice` — don't break (UDP) conversations on ICMP errors (§6).
* `addmulti <ifc-ip> [mcast-ip]` — with 2 fields the conv's raddr must be multicast
  (`"addmulti for a non multicast address"`) and f[1] is the interface address; with 3 fields
  f[1] is the multicast address and f[2] the interface.  Errors: `"addmulti needs interface
  address"`, `Ebadip`.  `remmulti <ip>` — remove; errors: `"remmulti needs interface address"`,
  `"remmulti for a non multicast address"`, `Ebadip` (`devip.c:1140-1164`).
* `maxfragsize <n>` — **[man-vs-src]** source-only verb, not in the man page
  (`devip.c:1165-1169`): sets `c->maxfragsize`, used for fragmentation when set.  No argument →
  `"maxfragsize needs size"`.

---

## 5. TCP

`tcp.c` (3,486 lines) is the donor's Reno state machine; r9 replaces the engine with smoltcp
`socket-tcp` + `socket-tcp-reno` (D-01) and keeps the file surface exact.

### 5.1 ctl verbs

Generic verbs plus (`tcpctl`, `tcp.c:3330-3345`):

* `close` — graceful close: FIN (`tcpclose2`).  smoltcp: `socket.close()`.
* `hangup` — immediate close, RST (`tcphangup`).  smoltcp: `socket.abort()`.  The aan gate's
  scripted drop writes exactly this (`echo hangup > /net/tcp/N/ctl`).
* `hangupxmit` — **[man-vs-src]** source-only verb (`tcpxmitclose`): close AFTER the transmit
  queue drains.  smoltcp shape: shutdown-of-TX-then-drain — the bridge closes once the TX buffer
  empties (`close()` semantics adjacent; §12 records it).
* `keepalive [n]` — start keepalives, n in milliseconds, **default 30000** (the man;
  `tcpstartka` requires the Established state else `"connection must be in Establised state"` —
  the donor's typo is the string).  smoltcp: `set_keep_alive(Some(Duration::millis(n)))`,
  default 30000 ms.
* `checksum <n>` — n == 0 emits zero checksums (`tcpsetchecksum` sets `nochecksum`).
  **No smoltcp mechanism** (checksums are always computed).  OQ-4 resolution (D-08 full-fidelity
  stance): **accepted-and-recorded** — the verb succeeds, is recorded on the conv, and a netlog
  note (`tcp` flag) names it a no-op.  Not loud-unimplemented.
* `tcpporthogdefense on|off` — a GLOBAL toggle in the donor (random-sequence keepalives shut
  hijacked connections; `tcpporthogdefensectl`, `"unknown value for tcpporthogdefense"` on other
  values).  **No smoltcp mechanism.**  OQ-4 resolution: **accepted-and-recorded** with a netlog
  note, same shape as `checksum`.

Unknown tcp verb → `"unknown control request"`.

### 5.2 status format

`tcpstate` (`tcp.c:493-509`), one line:

```
"%s qin %d qout %d rq %d.%d srtt %d mdev %d sst %lud cwin %lud swin %lud>>%d rwin %lud>>%d qscale %d timer.start %d timer.count %d rerecv %d katimer.start %d katimer.count %d\n"
```

The first field is the state name from **`tcpstates[]`** (`tcp.c:94-98`).  The 11-state 1:1
rename table to smoltcp's `tcp::State` — the status file's first field is a pure rename:

| # | devip `tcpstates[]` | smoltcp `tcp::State` |
|---|---------------------|----------------------|
| 0 | `Closed` | `Closed` |
| 1 | `Listen` | `Listen` |
| 2 | `Syn_sent` | `SynSent` |
| 3 | `Syn_received` | `SynReceived` |
| 4 | `Established` | `Established` |
| 5 | `Finwait1` | `FinWait1` |
| 6 | `Finwait2` | `FinWait2` |
| 7 | `Close_wait` | `CloseWait` |
| 8 | `Closing` | `Closing` |
| 9 | `Last_ack` | `LastAck` |
| 10 | `Time_wait` | `TimeWait` |

Field mapping (P-M honesty rule — real where smoltcp exposes it, fixed/zero where not, never
fabricated): state (real), qin/qout = conv queue bytes at the bridge (real), rq/reseq, srtt,
mdev, sst (ssthresh), cwin, swin>>scale, rwin>>scale, qscale, timer.start/count, rerecv,
katimer.start/count — smoltcp exposes none of the timer/reseq/window-scale internals in 0.14;
those fields print 0 and the doc records them as zero-mapped.  Gates assert the FORMAT and the
state/qin/qout fields, not the zero-mapped tail.

### 5.3 stats (`/net/tcp/stats`)

The source emits **28 tagged lines** (`tcp.c:348-375`, `statnames[]`), `"Name: %llud\n"` each —
**[man-vs-src]**: the man documents 11 fields.  The contract carries the source's list:

```
MaxConn MaxSegment ActiveOpens PassiveOpens EstabResets CurrEstab InSegs OutSegs
RetransSegs RetransSegsSent RetransTimeouts InErrs OutRsts CsumErrs HlenErrs LenErrs
OutOfOrder Resequenced ReseqBytelim ReseqPktlim Delayack Wopenack Recovery RecoveryDone
RecoveryRTO RecoveryNoSeq RecoveryCwind RecoveryPA
```

smoltcp exposes no stats counters: these are **bridge-maintained counters** incremented at every
packet transition the bridge observes (P-M).  A stats file that reads all zeros forever is a
contract violation (D-08).

### 5.4 gc

The donor's tcpgc reaps `Syn_received` convs older than 5 s and `Finwait2` older than 5 min under
clone pressure (`tcp.c:3362-3405`).  smoltcp has its own timeouts; r9's devip gc maps to dropping
fully-Closed convs' sockets (§3's clone-pressure path).  smoltcp RTO bounds (0.14):
initial 1000 ms, min 1000 ms, max 60 s — vs donor `tcpsettimer` 0.3–64 s.  The 1 s floor sizes
the TCG-scaled gate timeouts.

---

## 6. UDP

### 6.1 ctl verbs

Generic verbs plus one (`udpctl`, `udp.c:513-526`): **`headers`** — sets `headers = 7` ("new
headers format").  Default is point-to-point.

### 6.2 semantics

* After `announce`, a datagram from a NEW remote address/port quad spawns a new incoming conv
  (`Fsnewcall`, `udp.c:430-454`) — the listen/accept path.
* In `headers` mode all datagrams land on the announced conv prefixed with the 52-byte `Udphdr`
  (man/3/ip, `<ip.h>`):

```
struct Udphdr {
	uchar	raddr[16];	/* V6 remote address */
	uchar	laddr[16];	/* V6 local address */
	uchar	ifcaddr[16];	/* V6 interface address (receive only) */
	uchar	rport[2];
	uchar	lport[2];
};
```

  On write the user prefixes the same structure; the kernel overrides the user-specified local
  port with the announced one, and a non-local user-specified laddr is overridden too (the man).
  In headers mode no listen/accept is needed.  The bridge synthesizes the prefix from smoltcp's
  `UdpMetadata` (remote endpoint, local address) with v4 mapped into v6 form (`v4prefix`).
* Reads are datagram-at-once (Qmsg): a short read discards the rest of the datagram.
* **advise:** an ICMP error for the conv's quad `qhangup`s the conv's rq+wq with the message —
  the `err` file carries it — unless `ignoreadvice` (`udpadvise`, `udp.c:530-580`).  smoltcp's
  icmp socket `Endpoint::Udp(port)` receives associated ICMP errors (0.13+): the advice wiring
  exists; whether the gates exercise the err-file ICMP advice path is recorded in §12
  (listed-but-ungated corner candidate).

### 6.3 status and stats

status (`udpstate`, `udp.c:124-132`):

```
"%s qin %d qout %d\n"    # "Open" when the conv is in use, "Closed" otherwise
```

stats (`/net/udp/stats`, `udpstats`, `udp.c:585-596`) — four tagged lines:

```
InDatagrams: %llud
NoPorts: %lud
InErrors: %lud
OutDatagrams: %llud
```

(The man's prose: datagrams received / for bad ports / malformed / sent — the same four counters;
source tag names govern.)  Bridge-maintained per P-M.

---

## 7. ICMP (v4)

* ctl: **no proto verbs** (`icmp->ctl = nil`) — the generic verbs only.  NO headers mode for v4
  (the man documents headers only for icmpv6 and udp).
* The conv's `lport` IS the ICMP ident: on write the kernel OVERWRITES the packet's ident field
  with `c->lport` (`icmpkick`: `hnputs(p->icmpid, c->lport)`, `icmp.c:186`); RX dispatch matches
  the reply's ident against `lport` (`icmp.c:280-284`).  smoltcp mapping:
  `icmp::Endpoint::Ident(lport)` — accepts EchoRequest AND EchoReply with matching ident.
* ping(8) exercises exactly this: `netmkaddr(host, "icmp", "1")` → `connect <host>!1` → raw
  Icmphdr writes/reads on `data`; ping filters `type != echoreply` itself, which tolerates the
  looped-back request (loopback deliver + smoltcp's `auto-icmp-echo-reply` both verified
  compatible).
* status (`icmpstate`, `icmp.c:128-137`): `"Datagram qin %d qout %d\n"`.
* stats (`/net/icmp/stats`): the man documents 26 tagged fields (messages received/sent per
  type: unreachables, time-exceededs, parameter problems, source quenches, redirects, echo
  requests/replies, timestamps/replies, address-mask requests/replies, plus totals and errors).
  The source counts per-type in/out (`Icmppriv` `in[Maxtype+1]`/`out[]`).  Bridge-maintained
  counters per P-M; the tagged-line shape and the per-type names are the contract.

---

## 8. ICMPv6 — the mapping corner

* ctl: `headers` only — sets `headers = 6` (`icmpctl6`, `icmp6.c:281-292`).
* **`data` carries FULL IPv6 packets.**  Userspace builds/parses the IP6 header itself: on write
  the buffer starts at the IPv6 header (`padblock(bp, IP6HDR)`; the kernel fills src/dst and —
  unless headers mode — the ident), and reads deliver the packet WITH the IPv6 header.  ping's v6
  path skips `IPV6HDR_LEN = 40` on both directions (`ping.c:131,143`; `icmp6.c:236-282`).
  smoltcp's icmp socket is payload-level → **the bridge PARSES the user's IPv6 packet on write**
  (extract src/dst/payload; honor the user src in headers mode) **and SYNTHESIZES an IP6 header
  on read** (Pitfall P-E).  A payload-level passthrough silently breaks `ping -6` and ipconfig's
  RS/RA.
* `headers` mode prefix: the 40-byte `Icmp6hdr`/`ICMP_USEAD6` structure (man/3/ip ICMPV6):

```
struct Icmp6hdr {
	uchar	unused[8];
	uchar	laddr[16];	/* local address */
	uchar	raddr[16];	/* remote address */
};
```

* ipconfig's RA path uses it: `connect <addr>!<port>!r <port>` on an icmpv6 clone ctl, then
  `headers`, then RS/RA I/O on data (`ipv6.c:346-384` dialicmp).

---

## 9. ipifc

The interface protocol: `clone` / `stats` / numbered conv dirs with `ctl data err listen local
remote status snoop` (snoop 0400, ipifc-only — §3).  `data` is unused for I/O this era (the
packet-interface `bind pkt` path is dropped per D-08).

### 9.1 ctl verbs

(`ipifcctl`, `ipifc.c:760-795`, plus the devip bind path.)  Unknown verb → `"unsupported ctl"`.

* `bind ether <path>` — treat the device at `<path>` as an Ethernet medium carrying IP/ARP/IPv6;
  the donor dials `path!0x800`, `path!0x806`, `path!0x86dd` (`ethermedium.c:161-213`).  r9 maps
  to netif conv_alloc + conv_connect per ethertype against `#l0` (no file walk kernel-side).
  Errors: `"unknown interface type"`, `"interface already bound"`, `Ebadarg`.
* `bind loopback` — local loopback medium (§10).
* `bind pkt` / `bind netdev <path>` — **ABSENT per D-08** (pktmedium/netdevmedium dropped): they
  hit `"unknown interface type"` like any unknown medium.
* `unbind` — disassociate the device; removes all logical interfaces, closes the conv's queues.
* `add <local> [mask [remote [mtu [proxy]]]]` / `try <…>` — add a local address; `try` adds a v6
  address as TENTATIVE (DAD; ipconfig's ipv6.c:404 uses it).  Defaults: class mask for `mask`,
  `local AND mask` for `remote`, mtu 1514 ether / 4096 packet.  `proxy` = answer ARP for the
  remote address (ppp(8)'s trick).
* `remove <local> <mask>` — remove a local address.
* `mtu <n>` — set the interface mtu (the maximum packet INCLUDING medium headers).
* `reassemble` — reassemble IP fragments before forwarding.
* `iprouting [n]` — forwarding toggle (missing or non-zero n = on).
* `add6 <v6addr> <pfx-len> [onlink auto validlt preflt]` — add an IPv6 address with RFC 2461
  §6.2.1 parameters.
* `ra6 <keyword> <value> ...` — set RA parameters; keywords: `recvra sendra mflag oflag maxraint
  minraint linkmtu reachtime rxmitra ttl routerlt` (man/3/ip + ipconfig ipv6.c issuerara6 /
  issuebasera6).
* The man's ether-side list (`bridge`, `promiscuous`, `connect <type>`, `addmulti`, `remmulti`,
  `scanbs`, `headersonly`) is devether's own conv-ctl surface (Phase 7 locked depth), NOT
  ipifc's: the contract assigns each verb its exact home — those seven live at
  `/net/ether0/.../ctl`, and the ipifc ctl rejects them as `"unsupported ctl"`.

### 9.2 status and local read

status (`ipifcstate`, `ipifc.c:218-249`): the first line is

```
"device %s maxtu %d sendra %d recvra %d mflag %d oflag %d maxraint %d minraint %d linkmtu %d reachtime %d rxmitra %d ttl %d routerlt %d pktin %lud pktout %lud errin %lud errout %lud\n"
```

then one line per address (`slineformat`):

```
"\t%-40I %-10M %-40I %-12lud %-12lud\n"   # local, mask, remote, validlt, preflt
```

**[man-vs-src]**: the man's "9 white-space-separated fields" first-line description is the older,
simplified shape; the source format above is the contract.

`local` read (`ipifclocal`, `ipifc.c:250-275`): per-address `"%-40.40I ->"` plus the self-IPs
linked to it.

### 9.3 bind defaults

`ipifcbind` sets (`ipifc.c:143-153`): `mflag=0, oflag=0, maxraint=600000, minraint=200000,
linkmtu=0, reachtime=0, rxmitra=0, ttl=MAXTTL (=255, §4), routerlt=3*maxraint (=1800000)`.

### 9.4 stats (`/net/ipifc/stats`)

The MIB II counters (`ip.h:78-102`, Nipstats = 19 tagged lines): Forwarding, DefaultTTL,
InReceives, InHdrErrors, InAddrErrors, ForwDatagrams, InUnknownProtos, InDiscards, InDelivers,
OutRequests, OutDiscards, OutNoRoutes, ReasmTimeout, ReasmReqds, ReasmOKs, ReasmFails, FragOKs,
FragFails, FragCreates.  Bridge-maintained per P-M.

---

## 10. Mediums

* **ether** — hsize=14, mintu=60, maxtu=1514, maclen=6.  **The MTU INCLUDES the 14-byte Ethernet
  header** (and excludes FCS) — the smoltcp convention trap: smoltcp's Ethernet
  `DeviceCapabilities.max_transmission_unit` is the same 1514, matching devether's
  `ETHERMAXTU`/`mtu=1514` exactly; never report 1500.  `ares` = arpenter (resolution records),
  `areg` = sendgarp: **a gratuitous ARP is emitted whenever a new address is configured onto an
  Ethernet** (the man; ethermedium's Medium struct).  smoltcp does NOT send gratuitous ARP on
  `update_ip_addrs` — **the ether adapter emits it devip-side**, crafted with smoltcp's
  `wire::ArpPacket` codec (an allowed use: wire codec, not a protocol engine).  Gateable via pcap.
* **loopback** — maxtu **16 KiB** (`loopbackmedium.c:11-13`; the donor queues 1 MB Qmsg and runs
  a kproc reader — r9 has no kprocs: the seam's synchronous TX-loop drive replaces it, §12).  r9's
  loopback Device: a small fixed-pool `Medium::Ip` Device with Device MTU 16384 to match.  Do NOT
  use smoltcp's `phy::Loopback` — its TxToken allocates per packet.
* **null** — a registered medium NAME whose bind always errors **`"cannot bind null device"`**
  (`nullmedium.c:9-13`).  That error IS the null-medium contract: `ipfindmedium("null")` resolves
  (so the error is the null-specific one, never "unknown interface type") and the bind fails.
  There is no Device behind it.

---

## 11. Error vocabulary — every string, its trigger, and its gate

The file surface's exact strings.  Gates assert these verbatim (D-08).  `Eperm`/`Enodev`/… are
the `sys/src/9k/port/error.h` names; quoted literals are the inline `error("…")` strings.

| String | Trigger | Gate |
|--------|---------|------|
| `Eperm` | write to a read-only file (`ipselftab`, `status`, `local`, `remote`, `stats`, dirs); `create`/`remove` anywhere; `snoop` non-OREAD | tree-perms sweep |
| `Enodev` | clone exhaustion (§3); no interface for an operation | clone-pressure test |
| `Econinuse` | `connect`/`announce` on a non-Idle conv | ctl-verbs gate |
| `Econrefused` | connect failure via `c->cerr` (RST to SYN) | connect-to-closed-port gate |
| `Etimedout` | connect failure via `c->cerr` (timeout) | connect-timeout gate (TCG-scaled) |
| `Ebadip` (`"bad ip address syntax"`) | unparseable address in arp/addmulti/remmulti/del | arp + ctl-verbs gates |
| `Ebadarp` (`"bad arp"`) | unknown arp verb / unknown medium in `add` | arp gate |
| `Ebadarg` | bad argument count (arp add/del, ipifc add family) | arp + ipifc gates |
| `Eio` | ndb write past end (`off > strlen` or `off+n >= 1024`) | ndb gate |
| `Ebadnetctl` (`"too few arguments for netlog control message"`) | bare `set`/`clear`/`only` to `/net/log` | log gate |
| `"bad specification"` | attach spec ≥ Nfs (§1: r9's Nfs = 1) | attach gate |
| `"short control request"` | empty ctl write | ctl-verbs gate |
| `"unknown control request"` | unknown generic verb; unknown tcp verb (proto fallthrough) | ctl-verbs gate |
| `"unsupported ctl"` | unknown ipifc verb (incl. the dropped `bind pkt`/`bind netdev` → see below) | ipifc gate |
| `"connect not supported"` | proto with no connect (ipifc) | ctl-verbs gate |
| `"announce not supported"` | proto with no announce (ipifc) | ctl-verbs gate |
| `"bad args to connect"` / `"bad args to announce"` / `"bad args to bind"` | wrong field counts | ctl-verbs gate |
| `"address in use"` | quad-uniqueness violation on connect | connect gate |
| `"not a local IP address"` | local address not in ipselftab where one is required | ipifc/connect gates |
| `"malformed address"` | malformed `[addr!]port[!r]` parse | ctl-verbs gate |
| `"not announced"` | `listen` open on a non-Announced conv | listen gate |
| `"listen hungup"` | the waiting conv's rq closed mid-listen | listen gate |
| `"stats not implemented"` | proto with no stats (none this phase — all five protos have stats; the string is the nil-stats fallback) | (defensive only) |
| `"unknown interface type"` | `bind` to an unregistered medium name (incl. `pkt`/`netdev` per D-08) | ipifc gate |
| `"interface already bound"` | re-`bind` of a bound ipifc | ipifc gate |
| `"cannot bind null device"` | `bind null` (§10 — the null-medium contract) | null-medium gate |
| `"addmulti needs interface address"` | bare `addmulti` | ctl-verbs gate |
| `"addmulti for a non multicast address"` / `"remmulti for a non multicast address"` | non-multicast target | ctl-verbs gate |
| `"remmulti needs interface address"` | bare `remmulti` | ctl-verbs gate |
| `"maxfragsize needs size"` | bare `maxfragsize` | ctl-verbs gate |
| `"connection must be in Establised state"` | `keepalive` on a non-Established tcp conv (donor's typo verbatim) | tcp-ctl gate |
| `"unknown value for tcpporthogdefense"` | `tcpporthogdefense` with a non-`on`/`off` value | tcp-ctl gate |
| `"Destination unreachable"` | 3-field arp `add` with no route to the address | arp gate |

**The panic-boundary rule (load-bearing):** no smoltcp panic ever crosses the file surface.
smoltcp panics on non-unicast `update_ip_addrs`, on hop_limit = 0, on SocketSet type mismatch,
and on full fixed storage — every smoltcp call site in the bridge validates FIRST and maps
failure onto one of the strings above.  A kernel panic from a userspace ctl write or from a
network packet is a gate red (V5).

---

## 12. The r9 mapping — design decisions the gates assert

This chapter is the record the later gates pin.  Each entry names its decision ID.

### 12.1 The event-driven seam (D-05)

No periodic polling, no kprocs, no tick requirement (smoltcp 0.14 verified: poll is entirely
caller-driven).  `Interface::poll` runs in exactly three legs:

1. **RX-IRQ wake** — the virtio-net IRQ retires frames; `rx_ingress` demuxes them into the ether
   adapter's per-ethertype conv queues (0x800/0x806/0x86dd); the adapter's RX-pending interest
   wakes the registered devip waiter(s); the WOKEN context runs `drive_stack()` (poll, then
   re-arm the deadline) before re-testing its own file-op condition.  On loopback there is no
   IRQ: the TX-write loops the packet synchronously and the poll runs in the same file-op
   context (leg 3 subsumes it — this replaces the donor's loopback kproc reader).
2. **One-shot deadline armed from `poll_at()`** — after each poll, `poll_at(now, sockets)` gives
   the next soft deadline; it is recorded as an AtomicU64 in tick units and checked by the
   existing 1 ms generic-timer tick sweep (`proc::tick_timeouts`), which wakes the devip waiters.
   This is one-shot ARMING over the existing periodic tick — 1 ms granularity against smoltcp's
   1 s RTO floor, and the tick runs regardless as the preemption source.  **Considered and
   REJECTED (RESEARCH A5 resolution):** a literal dynamic-CNTP-rearm one-shot — it reworks the
   preemption timer for zero measurable benefit.
3. **Local file-op activity** — every devip file op that mutates socket state (ctl writes, data
   writes, connect/announce) ends with `drive_stack()` in the same syscall context (0.13's
   egress-exhaustion means one poll flushes the egress queue).

**The stack-debt flag rule (the idle corner):** when the timer leg fires with no registered
waiters, set the stack-debt flag; the next DEV_IP-dispatched syscall drives the stack first.
TCP timers are soft state (QoS, never correctness) — the phase gates never sit in the corner
(aan syncs at 8 s, ping at 1 s, tcpblast is continuous, ipconfig polls on alarm(1000)).

`poll()` never runs in IRQ context — r9's no-alloc-in-IRQ and mcslock-LEAF rules hold; the
BusyCell IRQ↔syscall handoff turns a violation into an accounted drop.  Bounded-drive
discipline: `poll_ingress_single` loops with an explicit per-drive cap + `poll_egress` +
`poll_maintenance` (docs.rs's own unbounded-work warning on a full `poll()`).

### 12.2 The waker bridge (P-C discipline)

smoltcp 0.14's waker methods live directly on each socket (`register_recv_waker` /
`register_send_waker`; there is no standalone public `WakerRegistration` type).  Semantics: one
waker per direction at a time, woken ONCE, spurious wakes allowed.  Therefore: the devip conv —
not the waker — owns the waiter set; the conv records the blocked reader's pid; the bridge
**re-registers after EVERY wake** and treats every wake as "re-check `can_recv`/`can_send`".
The IRQ-masked syscall invariant makes register → re-check → block wakeup-loss-free by
construction.  The `core::task::Waker` construction (RawWaker vtable) is the seam's only new
unsafe; it carries the SAFETY comment + an UNSAFE-AUDIT.md entry per the audit contract.

### 12.3 The accept pool (OQ-3) — with the memory math

smoltcp has NO listen backlog: excess SYNs to a listening socket are RSTed (docs.rs verbatim).
devip queues up to `Maxincall = 128` incoming calls per announced conv.  The bridge is an
**accept pool**: N smoltcp sockets all `listen(port)`; smoltcp's `accepts()` lets the first
still-`Listen` socket absorb each SYN (it then transitions SynReceived with the 4-tuple bound);
the `listen`-file open pops the oldest connected pool member and the pool refills.

**The math.** r9's kernel heap is a **4 MiB arena** (`port/src/globalalloc.rs:142`,
`GLOBAL_HEAP_SIZE = 4 * 1024 * 1024`, QuickFit).  Each pool member costs one rx socket buffer +
one tx socket buffer.  With the phase's TCP socket buffers of **rx = 2048 B, tx = 2048 B**:

```
pool depth × (rx + tx) = 128 × (2048 + 2048) B = 128 × 4 KiB = 512 KiB per announced conv
                                                    = 12.5% of the 4 MiB arena
```

Pools exist only while a conv is announced (allocated at `announce`, freed at close/unannounce)
— idle convs cost nothing.  The phase's gates run at most TWO concurrently announced TCP convs
(the aan listener and the tcp/7 echo listener) → worst-case pool footprint **1 MiB = 25% of the
arena**, against measured kernel heap use whose largest single allocation to date is devmnt's
~250 KB exec staging buffer.  D-08 pushes toward the devip-exact 128 and the math permits it.

**Decision: pool depth = 128 (devip-exact), socket buffers 2048/2048, allocated at announce.**
A future workload needing bigger windows or many concurrent announces revisits exactly this line
(the documented-deviation path: shrink buffers, or raise the arena) — the line is the contract.

### 12.4 One smoltcp Interface per bound ipifc (OQ-5)

One `Interface` owns one `Device`; devip allows multiple bound ipifcs (loopback + ether).  r9
creates one smoltcp Interface per bound ipifc at medium bind, with **address-class demux** at conv
connect/announce time: `127/8` and `::1` → the loopback iface; everything else → the ether iface.
Plans 08-04 (loopback leg) and 08-06 (ether leg) implement it; their gates validate the demux.

### 12.5 `/net/arp` as a snoop-fed view — and the static-add divergence

smoltcp 0.14 has NO public neighbor-cache accessor (the `iface` module's item list, verified).
So `/net/arp` is a **devip-owned table** with §2's exact surface (90-byte lines, NCACHE=256, LRU,
no timeout, flush/add/del).  Feed: the ether adapter parses ARP (ethertype 0x806) and NDP
(ICMPv6 NS/NA inside 0x86dd) frames passing through `receive()`/`transmit()` with smoltcp's own
`wire` parsers (reused, not hand-rolled), recording (ip→mac, OK) on replies/announcements and
(ip, WAIT) when we observe our own solicitations.  TX-side snooping lives at the ADAPTER level
(the Phase-7 netif TX self-delivery gap is moot for smoltcp but the snoop wants TX frames).

**The deliberate divergence, honestly recorded:** in 9k, `arp add` injects into the REAL
resolution path; in r9 it writes the view table only — smoltcp's internal cache self-manages
(and flushes on IP change).  A script that `arp add`s gets **file-level success and no resolution
steering**.  Every Phase-8 consumer (static/DHCP ipconfig, IPv6 RA via ipconfig, ping, aan,
tcpblast) works through smoltcp's own ARP/ND, so the divergence is observational, not functional;
the gates assert the file semantics.  (ipconfig's RA path's `add ether <ra-src> <lladdr>` write
succeeds and feeds the view — `ipv6.c:604-615`.)

### 12.6 Address-add guards (P-F/P-G)

* **`add 0.0.0.0 0.0.0.0` absorption:** ipconfig's DHCP preconfig writes exactly this
  (`main.c:926`).  smoltcp's `update_ip_addrs` PANICS on non-unicast addresses.  The bridge
  ABSORBS the unspecified-address add devip-side — recorded on the lifc as the DHCP-in-progress
  marker, never passed to `update_ip_addrs`.  Same guard for `::` and for overflow past
  `IFACE_MAX_ADDR_COUNT` (smoltcp default 8): overflow maps to a devip error string, never a
  panic.
* **DHCP src-0.0.0.0 TX (OQ-1):** smoltcp's udp socket source-selection with zero configured
  addresses is unproven for the DHCP shape; smoltcp's own dhcpv4 socket hand-crafts src-0.0.0.0
  at the wire layer (`socket/dhcpv4.rs:610` — proof the machinery exists).  Resolution venue:
  Plan 08-02 Task 2 pin (h) — the phase's FIRST host pin builds ipconfig's exact DHCP-shaped
  exchange against a host-side smoltcp instance (bind 68, broadcast TX, no iface address); the
  PASS-path/SHIM-path verdict is recorded here as a dated line when the pin lands.  The recorded
  fallback stays D-09's dhcpv4 socket.

  *(2026-11-05: pin pending — this line is replaced by the 08-02 verdict.)*

### 12.7 UDP err-file ICMP advice

smoltcp's icmp socket `Endpoint::Udp(port)` receives the associated ICMP errors, so the
`udpadvise` → qhangup wiring EXISTS in the seam.  The err-file advice path is listed in the D-08
surface; the phase gates leave it **ungated** (no milestone consumer reads UDP `err` this phase —
the gates' ICMP fidelity is asserted via ping, not via advice).  Recorded so the corner is a
decision, not an omission.

### 12.8 Defaults that must not leak through

* devip default ttl = `MAXTTL = 255` (§4 — this tree's `ip.h:44`); smoltcp's hop-limit default
  is 64.  The bridge sets hop limit 255 at conv creation; `ttl <n>` maps to `set_hop_limit`, and
  `ttl` with no argument resets to 255.  (The phase research's "MAXTTL=64" parenthetical is a
  transcription error; the source is the authority this doc exists to record.)
* `tos` default 0 (`DFLTTOS`).
* keepalive default 30000 ms (§5.1).
* smoltcp RTO floor 1 s (min == initial), max 60 s — sizes TCG-scaled gate timeouts; vs donor
  `tcpsettimer` 0.3–64 s.

### 12.9 Loopback/null/ether registration order

Locked sequencing: loopback and null medium NAMES register first (correct bind behavior — the
synchronous-drive loopback Device, the always-erroring null bind) before any ether binding; the
ether binding consumes Phase 7's devether (`#l`).  `Medium::Ip` for the loopback Device (no
ethernet header on the loopback path); `Medium::Ethernet` for the ether adapter (which is what
switches on smoltcp's internal ARP/NDISC).

---

*Sources: `sys/man/3/ip`, `sys/man/2/ip`, `sys/man/8/ipconfig`; Harvey `sys/src/9k/ip/` —
`devip.c`, `ip.h`, `tcp.c`, `udp.c`, `icmp.c`, `icmp6.c`, `ipifc.c`, `iproute.c`, `arp.c`,
`netlog.c`, `nullmedium.c`, `loopbackmedium.c`, `ethermedium.c`; smoltcp 0.14.0 (docs.rs + crate
source + the 2026-11-05 compile probe).  Every [man-vs-src] flag is a place the man and the
source disagree; source wins, the flag is the honesty.*
