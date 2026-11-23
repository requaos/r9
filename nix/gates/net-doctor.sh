#!/usr/bin/env bash
# nix/gates/net-doctor.sh — THE read-only host-state doctor for the
# x86_64 net modes (16/HNET-05/D-14; moved from Harvey bd7a698's
# nix/net-doctor.sh by plan 01-24, D-07/D-09/HARN-08 — the body is
# byte-identical to that copy): diagnose + print, NEVER fix.  ONE
# implementation consumed two ways (D-03): this script is wrapped as
# the net-doctor flake app, and the moved tap-leg gates invoke the same
# wrapped app via R9_NET_DOCTOR so both front-ends print byte-identical
# check results.
#
# Output vocabulary: one line per check —
#   net-doctor: PASS|FAIL|WARN|INFO <check-name>: <detail>
# every FAIL prints an indented remediation block chosen by NixOS
# presence (/run/current-system — the immutable NixOS marker, not an
# env var, T-16-07): the declarative snippet first (D-10), the generic
# imperative commands otherwise.  WARN-class findings NEVER gate the
# exit code: the root-gated masquerade read degrades to a LOUD WARN
# carrying the exact sudo command, never a silent pass (OQ-3 adopted,
# Pitfall 8).  Exit semantics are deterministic for CI: exit 0 iff
# zero FAILs (the final line then reads "net-doctor: DOCTOR-OK"),
# else exit 1.
#
# READ-ONLY RULE (D-14's hard rule): no code path below mutates any
# host state — probes and printed remediation ONLY.  Every ip/nft/
# sysctl/chmod/echo line in a remediation block is TEXT for the human
# to apply; the doctor never execs it.  Probe output prints on detail
# lines only, never interpolated into the commands a human copies
# (T-16-05).
#
# The committed check list (D-14) — a host-state class NOT in this
# list is invisible to the doctor by design; extending it is a new
# decision, not a silent patch:
#   slirp: qemu-present, hostfwd-17010-free, hostfwd-5670-free
#   tap:   bridge-exists, bridge-up, bridge-addr, ip-forward,
#          masquerade, helper-wrapper, bridge-conf-allow, blastport-free
#   vmnet: socket-vmnet-info (the darwin manual leg; INFO n/a on linux)
#
# Env contract:
#   HARVEY          optional — the harvey checkout root (default $PWD).
#                   Only TAP mode reads the netaddr seam under it
#                   (tap_gwv4/blastport — the seam is the single
#                   address source host-side too, 16-02's keys);
#                   slirp/vmnet NEVER read the tap_ keys, so a slirp
#                   preflight is independent of 16-02's landing order.
#   R9_NET_DOCTOR   not consumed here — 16-04's boot app exports it
#                   naming this wrapped app (D-03).
#
# usage: net-doctor.sh [--mode slirp|tap|vmnet|all]   (default: all)

set -e

err() { echo "error: $*" >&2; exit 1; }

# --- host data: named constants + seam-populated values ----------------
#
# BRIDGE is a host OBJECT name, not a venue address — it matches the
# r9 TOML tap stanza's br=.  The tap subnet/ports come from the
# netaddr seam (16-02), parsed lazily below; NO address literal lives
# in this script.
BRIDGE=br0
# the NixOS setuid helper wrapper: the ONLY working helper path on
# NixOS — both qemu builds compile their default into the read-only
# nosuid store libexec (Pitfall 1), which is why the tap stanza
# carries helper= explicitly
HELPER=/run/wrappers/bin/qemu-bridge-helper
# the helper's ACL file: exact per-bridge allow lines, NEVER "allow
# all" (the ACL is the unprivileged-tap control point)
BRIDGE_CONF=/etc/qemu/bridge.conf
# the qemu binary every x64 gate boots
QEMU_BIN=qemu-system-x86_64
# the slirp hostfwd host ports (D-04: auth's guest 567 is host-side
# remapped to 5670 — Linux refuses unprivileged binds below 1024)
HOSTFWD_PORT_A=17010
HOSTFWD_PORT_B=5670

# seam-populated by seam_init (TAP mode only); pre-initialized so the
# seam-free modes never reference unset vars under set -u (the flake
# app wrapper runs `set -euo pipefail` ahead of this text)
tap_gwv4=
blastport=
TAP_SUBNET=
SEAM_PATH=
masq_text=

FAILS=0

pass() { echo "net-doctor: PASS $1: $2"; }
warn() { echo "net-doctor: WARN $1: $2"; }
info() { echo "net-doctor: INFO $1: $2"; }

fail() {
	# fail <name> <detail> [extra] — the FAIL line, the count, and the
	# exact remediation block (remedy dispatches on the check name)
	echo "net-doctor: FAIL $1: $2"
	FAILS=$((FAILS + 1))
	remedy "$1" "${3:-}"
}

indent4() { sed 's/^/    /'; }

# --- remediation blocks -------------------------------------------------
# ALL text below is PRINTED for the human to apply — the read-only
# rule at the top of this file.  Nothing here is executed by the
# doctor; the mutation-shaped lines are heredoc/echo payload only.

remedy() {
	# remedy <check-name> [extra]: pick the venue-appropriate block
	case "$1" in
	hostfwd-17010-free | hostfwd-5670-free | blastport-free)
		# freeing a port is venue-neutral: name the holder, the
		# human stops it — nothing declarative to add
		{
			echo "remediation:"
			echo "  name the holder: ss -tlnp | grep \":${2:-}\" (root sees other users' sockets)"
			echo "  stop that listener or wait it out — the port is the net"
			echo "  contract this mode needs; the doctor never stops or"
			echo "  re-binds anything"
		} | indent4
		return
		;;
	esac
	if [ -d /run/current-system ]; then
		{
			echo "remediation (NixOS — declarative; add to the host/colmena config, then rebuild):"
			remedy_nixos "$1"
		} | indent4
	else
		{
			echo "remediation (generic linux — as root):"
			remedy_generic "$1"
		} | indent4
	fi
}

remedy_nixos() {
	case "$1" in
	qemu-present)
		cat <<EOF
environment.systemPackages = [ pkgs.qemu ];
EOF
		;;
	bridge-exists | bridge-up | bridge-addr)
		cat <<EOF
networking.bridges.$BRIDGE.interfaces = [ ];  # empty member list: the pure NAT bridge (D-08)
networking.interfaces.$BRIDGE.ipv4.addresses = [
  { address = "$tap_gwv4"; prefixLength = 24; }  # the seam's tap_gwv4
];
EOF
		;;
	ip-forward)
		cat <<EOF
boot.kernel.sysctl."net.ipv4.ip_forward" = 1;
EOF
		;;
	masquerade)
		cat <<EOF
networking.nat.enable = true;
networking.nat.externalInterface = "<default-route iface>";  # ip route show default names it
networking.nat.internalInterfaces = [ "$BRIDGE" ];
networking.firewall.trustedInterfaces = [ "$BRIDGE" ];  # br0<->host forwarding only: NO LAN-to-br0 inbound stays the rule (D-08)
EOF
		;;
	helper-wrapper)
		cat <<EOF
security.wrappers.qemu-bridge-helper = {
  source = "\${lib.getBin pkgs.qemu}/libexec/qemu-bridge-helper";
  owner = "root";
  setuid = true;
};
EOF
		;;
	bridge-conf-allow)
		cat <<EOF
environment.etc."qemu/bridge.conf".text = ''
  allow $BRIDGE
  allow virbr0  # keep ONLY while libvirt's virbr0 must stay allowed (append semantics)
'';
# the EXACT per-bridge line only — "allow all" is forbidden (the ACL is the control point)
EOF
		;;
	esac
}

remedy_generic() {
	case "$1" in
	qemu-present)
		cat <<EOF
# install qemu from the distro, e.g.:
#   apt install qemu-system-x86   (debian/ubuntu)
#   dnf install qemu-system-x86   (fedora)
EOF
		;;
	bridge-exists)
		cat <<EOF
ip link add $BRIDGE type bridge
ip link set $BRIDGE up
EOF
		;;
	bridge-up)
		cat <<EOF
ip link set $BRIDGE up
EOF
		;;
	bridge-addr)
		cat <<EOF
ip addr add $tap_gwv4/24 dev $BRIDGE
EOF
		;;
	ip-forward)
		cat <<EOF
sysctl -w net.ipv4.ip_forward=1
# persist across reboots via the distro's /etc/sysctl.d/
EOF
		;;
	masquerade)
		cat <<EOF
nft add table nat
nft add chain nat postrouting '{ type nat hook postrouting priority 100 ; }'
nft add rule nat postrouting oifname "<default-iface>" ip saddr $TAP_SUBNET masquerade
EOF
		;;
	helper-wrapper)
		cat <<EOF
# locate the distro helper first (path varies): find /usr -name qemu-bridge-helper
chmod u+s /usr/libexec/qemu-bridge-helper
EOF
		;;
	bridge-conf-allow)
		cat <<EOF
echo "allow $BRIDGE" >> $BRIDGE_CONF
# the EXACT per-bridge line only — never "allow all"
EOF
		;;
	esac
}

# --- check helpers ------------------------------------------------------

port_busy() {
	# port_busy <port>: true when ss -tln shows a state-LISTEN socket
	# whose local address ends in the port digits — the exact match,
	# word-bounded so 17010 never matches 170100
	ss -tln 2>/dev/null | grep -Eq "[:.]$1\b"
}

check_port_free() {
	# check_port_free <check-name> <port>
	if port_busy "$2"; then
		holder=$(ss -tln 2>/dev/null | grep -E "[:.]$2\b" || true)
		fail "$1" "port $2 held by a LISTEN socket: $holder" "$2"
	else
		pass "$1" "port $2 free (no LISTEN holder)"
	fi
}

seam_init() {
	# the tap checks are meaningless without the seam (16-02's tap_
	# keys are the doctor's address source) — err loudly, never
	# silently skip.  Only TAP mode calls this: slirp/vmnet preflights
	# stay independent of the seam (and of 16-02's landing order).
	HARVEY="${HARVEY:-$PWD}"
	SEAM_PATH="$HARVEY/build/x86_64/root-tree/lib/netaddr.rc"
	[ -f "$SEAM_PATH" ] || err "tap mode needs the netaddr seam: $SEAM_PATH missing — run from the harvey repo root or set HARVEY (the tap checks key on its tap_gwv4/blastport)"
	# eval the grep-extracted key assignments (committed repo data —
	# the trusted seam, not probe output)
	eval "$(grep -E '^(tap_gwv4|blastport)=' "$SEAM_PATH" || true)"
	[ -n "$tap_gwv4" ] || err "seam $SEAM_PATH: no tap_gwv4 key (16-02's tap leg expected)"
	[ -n "$blastport" ] || err "seam $SEAM_PATH: no blastport key"
	# the /24 the bridge-addr match and the masquerade greps key on
	# (the seam's tap_maskv4 is 255.255.255.0 — a /24 by its own data)
	TAP_SUBNET="${tap_gwv4%.*}.0/24"
}

masq_verdict() {
	# a SUCCESSFUL ruleset read: the rule match decides PASS vs FAIL
	# (iptables-save prints "-j MASQUERADE", nft prints "masquerade")
	if printf '%s\n' "$masq_text" | grep -F "$TAP_SUBNET" | grep -qi masquerade; then
		pass masquerade "a $TAP_SUBNET MASQUERADE rule is present in the ruleset"
	else
		fail masquerade "no MASQUERADE rule for $TAP_SUBNET in the readable ruleset — guest traffic past the host would blackhole"
	fi
}

check_masquerade() {
	# the masquerade read is root-gated (Pitfall 8: nft is absent from
	# PATH here, iptables-save needs root) — OQ-3's semantics: a
	# denied/unavailable read is a LOUD WARN with the exact sudo
	# command, never a silent pass and never a FAIL (the doctor cannot
	# know); a successful read with no matching rule IS a FAIL
	if command -v iptables-save >/dev/null 2>&1; then
		# shellcheck disable=SC2312  # the masked return value is the point: denial => WARN below
		if masq_text=$(iptables-save 2>&1); then
			masq_verdict
			return
		fi
	fi
	if command -v nft >/dev/null 2>&1; then
		# shellcheck disable=SC2312  # the masked return value is the point: denial => WARN below
		if masq_text=$(nft list ruleset 2>&1); then
			masq_verdict
			return
		fi
	fi
	warn masquerade "cannot read the netfilter ruleset unprivileged — verify with: sudo iptables-save | grep '$TAP_SUBNET' (or: sudo nft list ruleset | grep '$TAP_SUBNET'); want a MASQUERADE rule for $TAP_SUBNET out the default route"
}

# --- the per-mode check blocks ------------------------------------------

check_slirp() {
	# slirp is the hermetic zero-privilege default (HNET-01): qemu
	# present + the D-04 hostfwd ports free on loopback
	if command -v "$QEMU_BIN" >/dev/null 2>&1; then
		pass qemu-present "$QEMU_BIN on PATH"
	else
		fail qemu-present "$QEMU_BIN not found on PATH"
	fi
	check_port_free hostfwd-17010-free "$HOSTFWD_PORT_A"
	check_port_free hostfwd-5670-free "$HOSTFWD_PORT_B"
}

check_tap() {
	seam_init
	# bridge-exists + bridge-up (ip link show; the flags set carries
	# UP — the <NO-CARRIER,...,UP> shape included)
	if ip link show dev "$BRIDGE" >/dev/null 2>&1; then
		pass bridge-exists "$BRIDGE present"
		if ip link show dev "$BRIDGE" 2>/dev/null | grep -Eq '(<|,)UP(,|>)'; then
			pass bridge-up "$BRIDGE flags carry UP"
		else
			fail bridge-up "$BRIDGE exists but is DOWN"
		fi
	else
		fail bridge-exists "$BRIDGE absent — the tap netdev's br= names it"
	fi
	# bridge-addr: the seam's tap_gwv4 with its /24 on the bridge
	if ip addr show dev "$BRIDGE" 2>/dev/null | grep -qF "$tap_gwv4/24"; then
		pass bridge-addr "$tap_gwv4/24 present on $BRIDGE (the seam's tap_gwv4)"
	else
		fail bridge-addr "no $tap_gwv4/24 on $BRIDGE — the seam's tap_gwv4 is the committed bridge address"
	fi
	# ip-forward
	# shellcheck disable=SC2312  # the || true inside is deliberate: any read failure => not "1" => FAIL
	if [ "$(sysctl -n net.ipv4.ip_forward 2>/dev/null || true)" = "1" ]; then
		pass ip-forward "net.ipv4.ip_forward=1"
	else
		fail ip-forward "net.ipv4.ip_forward is not 1"
	fi
	# masquerade (the root-gated read — WARN class, never gates exit)
	check_masquerade
	# helper-wrapper: present AND setuid (stat's mode digits: a leading
	# 4/6 carries setuid; present-but-nosuid is the Pitfall-1 failure)
	if [ -x "$HELPER" ]; then
		helper_mode=$(stat -c %a "$HELPER" 2>/dev/null || true)
		case "$helper_mode" in
		[46]*)
			pass helper-wrapper "$HELPER present + setuid (mode $helper_mode)"
			;;
		*)
			fail helper-wrapper "$HELPER present but NOT setuid (mode $helper_mode) — the compiled-in default is the nosuid store libexec (Pitfall 1)"
			;;
		esac
	else
		fail helper-wrapper "$HELPER absent — the security.wrappers path is the only working helper on NixOS (Pitfall 1)"
	fi
	# bridge-conf-allow: grep -x 'allow br0' — the EXACT-line match on
	# the ACL file, never a substring match (allow br0x must not pass)
	if [ -f "$BRIDGE_CONF" ] && grep -xq "allow $BRIDGE" "$BRIDGE_CONF" 2>/dev/null; then
		pass bridge-conf-allow "$BRIDGE_CONF carries the exact 'allow $BRIDGE' line"
	else
		conf_allow=$(grep '^allow' "$BRIDGE_CONF" 2>/dev/null | tr '\n' ' ' || true)
		fail bridge-conf-allow "$BRIDGE_CONF has no exact 'allow $BRIDGE' line (allow lines now:${conf_allow:- none})"
	fi
	# blastport-free (the seam's blastport — the soak peer's port)
	check_port_free blastport-free "$blastport"
}

check_vmnet() {
	# vmnet is the darwin leg (HNET-04, documented-manual: nixpkgs qemu
	# is unsigned — the builtin vmnet-* backends need an
	# Apple-restricted entitlement; socket_vmnet is the rootless
	# route).  INFO class everywhere: this leg never gates the exit.
	case "$(uname -s)" in
	Darwin)
		if [ -S /var/run/socket_vmnet ]; then
			info socket-vmnet-info "/var/run/socket_vmnet socket present (the manual leg — build/x86_64/README.md)"
		else
			info socket-vmnet-info "no /var/run/socket_vmnet socket — install/start socket_vmnet (brew install socket_vmnet; sudo brew services start socket_vmnet); build/x86_64/README.md"
		fi
		;;
	*)
		info socket-vmnet-info "n/a on $(uname -s) — vmnet is the darwin manual leg (HNET-04); see build/x86_64/README.md"
		;;
	esac
}

# --- argv + dispatch ----------------------------------------------------

MODE=all
while [ $# -gt 0 ]; do
	case "$1" in
	--mode)
		[ $# -ge 2 ] || err "--mode needs a value — want --mode slirp|tap|vmnet|all (the valid modes)"
		MODE="$2"
		shift 2
		;;
	--mode=*)
		MODE="${1#--mode=}"
		shift
		;;
	--help | -h)
		echo "usage: net-doctor.sh [--mode slirp|tap|vmnet|all]   (default: all)"
		exit 0
		;;
	*)
		err "unknown argument '$1' — usage: net-doctor.sh [--mode slirp|tap|vmnet|all] (the valid modes: slirp tap vmnet all)"
		;;
	esac
done

case "$MODE" in
slirp)
	check_slirp
	;;
tap)
	check_tap
	;;
vmnet)
	check_vmnet
	;;
all)
	check_slirp
	check_tap
	check_vmnet
	;;
*)
	err "--mode $MODE: want slirp|tap|vmnet|all (the valid modes) — usage: nix run .#net-doctor -- --mode <mode>"
	;;
esac

if [ "$FAILS" -eq 0 ]; then
	echo "net-doctor: DOCTOR-OK"
	exit 0
fi
echo "net-doctor: $FAILS FAIL(s) — apply the printed remediations (the doctor is read-only: it diagnoses, the human applies)"
exit 1
