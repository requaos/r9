// The DTB bootconf (ARM-10, Harvey 20-13): the host launcher writes a
// key=value boot string onto /chosen/bootargs of the DTB copy it hands
// qemu (`R9_BOOTARGS` in Harvey's nix/boot-r9.sh and nix/smoke-r9.sh,
// one `fdtput -t s`), and the exec prologue merges its pairs into the
// kernel-seeded ROOT env group after the compiled ones.  This is the
// aarch64 twin of x86_64's multiboot2 cmdline tag (GRUB `-append`).
//
// Carrier: measured in Harvey 20-13 Task 1 (20-EVIDENCE.md "Bootconf
// delivery").  Under qemu 11.1.2's ELF + `-dtb` boot a bootargs string
// fdtput wrote arrives intact, terminating NUL included, on hvf and
// tcg, and qemu adds none when none was written (the launchers never
// pass `-append`, the only thing qemu writes bootargs for).
//
// The pair parser is a DELIBERATE duplicate of
// x86_64/src/platform/q35/mb2.rs's parse_bootconf_pairs (behavior
// unchanged, error type local).  Hoisting it into port/src would
// change the x86_64 kernel bytes (and with them the boot-iso hash and
// the x64 gates) for no behavioral gain (research Pitfall 3); keep the
// two copies in step by hand.
//
// Gated `any(test, dev_exec)` in mod.rs: the only consumer is the
// dev_exec exec prologue, and the host tests (HOST-TEST-ROUTE A, the
// virtshell cfg) reach the pure halves.  The churn-driver profile
// (dev_exec + dev_churn, no boot chain) seeds the compiled pairs only
// and never reads the string, so the reader is dead code there too.
#![cfg_attr(not(all(dev_exec, any(not(dev_churn), dev_bootchain))), allow(dead_code))]

use port::fdt::DeviceTree;

/// The boot string's hard cap -- a string longer than 256 bytes before
/// its NUL is rejected LOUDLY, never silently truncated (a cut string
/// could change a value's meaning; the string is host-controlled
/// compose-time input, and first boot is the cheapest place to say so).
/// The DTB itself carries longer strings whole (a 300-byte string was
/// measured intact), so this constant is the only bound.
pub const MAX_CMDLINE: usize = 256;

/// The bootconf pair cap -- defensive headroom over the one- or
/// two-pair lines the images actually carry (init= alone); exceeded is
/// a loud named error like every other malformed-input class here.
pub const MAX_BOOTCONF_PAIRS: usize = 32;

/// The /chosen property the launcher writes and this reader takes
/// (owner-confirmed carrier, Harvey 20-13 Task 2: chosen-bootargs).
const BOOTARGS_PROP: &str = "bootargs";

/// Malformed boot-string classes, each a loud panic at first boot.
#[derive(Debug, PartialEq)]
pub enum BootconfError {
    /// A whitespace token with no '=' arrives before any pair it could
    /// continue, or carries an empty key.
    InvalidBootconfToken,
    /// More key=value pairs than MAX_BOOTCONF_PAIRS.
    TooManyBootconfPairs,
    /// The string is longer than MAX_CMDLINE before its first NUL.
    CmdlineTooLong,
}

/// The boot string copied out of the DTB: the bytes up to the first
/// NUL, in a fixed buffer (no allocation this early).  Empty when
/// /chosen or its bootargs property is absent.
pub struct BootArgs {
    buf: [u8; MAX_CMDLINE],
    len: usize,
}

impl BootArgs {
    /// The empty boot string (absent node or property).
    pub const fn empty() -> BootArgs {
        BootArgs { buf: [0; MAX_CMDLINE], len: 0 }
    }

    /// Copy a raw property value: cut at the first NUL (bytes after it
    /// are ignored), reject more than MAX_CMDLINE bytes before it.
    /// Pure, so the host tests pin it without a DTB.
    pub fn from_raw(raw: &[u8]) -> Result<BootArgs, BootconfError> {
        let string = match raw.iter().position(|&b| b == 0) {
            Some(nul) => &raw[..nul],
            None => raw,
        };
        let mut out = BootArgs::empty();
        let Some(buf) = out.buf.get_mut(..string.len()) else {
            return Err(BootconfError::CmdlineTooLong);
        };
        buf.copy_from_slice(string);
        out.len = string.len();
        Ok(out)
    }

    /// The string's bytes (no NUL).
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[..self.len]
    }
}

/// Read /chosen/bootargs through port::fdt's property accessors (no
/// raw DTB slicing here).  Absent node or property: an empty BootArgs,
/// the default boot.  Over-long: a named panic, never a truncate.
pub fn read_bootargs(dt: &DeviceTree) -> BootArgs {
    let Some(chosen) = dt.find_by_path("/chosen") else {
        return BootArgs::empty();
    };
    let Some(prop) = dt.property(&chosen, BOOTARGS_PROP) else {
        return BootArgs::empty();
    };
    let Some(raw) = dt.property_value_bytes(&prop) else {
        return BootArgs::empty();
    };
    BootArgs::from_raw(raw)
        .unwrap_or_else(|err| panic!("bootconf: DTB bootargs parse failed: {err:?}"))
}

/// A parsed bootconf: the boot string's key=value pairs, key and value
/// slices BORROWED from the string (no allocation).  Consumed by the
/// exec prologue to merge into the seeded ROOT env group.
#[derive(Debug, PartialEq)]
pub struct Bootconf<'a> {
    pairs: [(&'a [u8], &'a [u8]); MAX_BOOTCONF_PAIRS],
    npairs: usize,
}

impl<'a> Bootconf<'a> {
    fn empty() -> Bootconf<'a> {
        Bootconf { pairs: [(&[], &[]); MAX_BOOTCONF_PAIRS], npairs: 0 }
    }

    /// The parsed pairs in string order.
    pub fn pairs(&self) -> impl Iterator<Item = (&'a [u8], &'a [u8])> {
        self.pairs[..self.npairs].iter().copied()
    }

    /// True when the string carried no pairs (the absent/empty case --
    /// zero seed changes).
    pub fn is_empty(&self) -> bool {
        self.npairs == 0
    }
}

/// Parse a boot string into key=value bootconf pairs -- the donor
/// plan9.ini shape, one line:
///
/// - tokens are whitespace-separated; a token containing '=' opens a
///   pair, split at the FIRST '=' -- further '=' inside the value are
///   literal and the value runs to the token's end (the next
///   whitespace);
/// - a token WITHOUT '=' continues the previous pair's value, the
///   value's span extending to the token's end: the one-line analog of
///   plan9.ini's value-runs-to-end-of-line (the donor carries values
///   with spaces -- `init=/arm64/init -c` is exactly this shape, and
///   boot(8)'s execinit tokenizes the multi-word value verbatim,
///   sys/src/9k/boot/boot.c:208-214);
/// - a continuation with no preceding pair, or an empty key, is a loud
///   named error -- malformed bootconf surfaces at first boot, never a
///   silent drop;
/// - the empty string parses to zero pairs (the no-op).
pub fn parse_bootconf_pairs(cmdline: &[u8]) -> Result<Bootconf<'_>, BootconfError> {
    // Pair spans as cmdline INDEX ranges; the borrowed slices materialize
    // only when the walk finishes (a continuation extends the previous
    // value's end index -- no slice surgery mid-walk).
    let mut keys = [(0usize, 0usize); MAX_BOOTCONF_PAIRS];
    let mut vals = [(0usize, 0usize); MAX_BOOTCONF_PAIRS];
    let mut npairs = 0usize;
    let mut i = 0usize;
    while i < cmdline.len() {
        while i < cmdline.len() && cmdline[i].is_ascii_whitespace() {
            i += 1;
        }
        let start = i;
        while i < cmdline.len() && !cmdline[i].is_ascii_whitespace() {
            i += 1;
        }
        if i == start {
            break; // trailing whitespace only
        }
        match cmdline[start..i].iter().position(|&b| b == b'=') {
            Some(0) => return Err(BootconfError::InvalidBootconfToken), // empty key ("=v")
            Some(eq) => {
                if npairs == MAX_BOOTCONF_PAIRS {
                    return Err(BootconfError::TooManyBootconfPairs);
                }
                keys[npairs] = (start, start + eq);
                vals[npairs] = (start + eq + 1, i);
                npairs += 1;
            }
            None => {
                // Continuation: extend the previous value's span to this
                // token's end (the whitespace between joins into the
                // value, exactly the donor's runs-to-end-of-line).
                if npairs == 0 {
                    return Err(BootconfError::InvalidBootconfToken);
                }
                vals[npairs - 1].1 = i;
            }
        }
    }
    let mut out = Bootconf::empty();
    out.npairs = npairs;
    for k in 0..npairs {
        out.pairs[k] = (&cmdline[keys[k].0..keys[k].1], &cmdline[vals[k].0..vals[k].1]);
    }
    Ok(out)
}

// The bootconf host pins (Harvey 20-17 Task 1): x86_64's mb2.rs
// cmdline/bootconf pins ported for the six behaviors 20-13 Task 3 kept.
// read_bootargs (the DTB walk) is covered live: every boot-r9-cpu run
// requires its `devenv: bootconf init=/arm64/init -c` line.
#[cfg(test)]
mod bootconf_tests {
    use super::{BootArgs, BootconfError, MAX_BOOTCONF_PAIRS, MAX_CMDLINE, parse_bootconf_pairs};
    use alloc::format;
    use alloc::string::String;
    use alloc::vec;
    use alloc::vec::Vec;

    fn pairs(s: &[u8]) -> Vec<(&[u8], &[u8])> {
        parse_bootconf_pairs(s).expect("parses").pairs().collect()
    }

    #[test]
    fn bootconf_empty_and_absent_strings_parse_to_zero_pairs() {
        // An absent property reads as the empty BootArgs.
        assert!(parse_bootconf_pairs(BootArgs::empty().as_bytes()).unwrap().is_empty());
        assert!(parse_bootconf_pairs(b"").unwrap().is_empty());
        assert!(parse_bootconf_pairs(b"   ").unwrap().is_empty());
        // A property holding only its NUL.
        let raw = BootArgs::from_raw(b"\0").unwrap();
        assert_eq!(raw.as_bytes(), b"");
        assert!(parse_bootconf_pairs(raw.as_bytes()).unwrap().is_empty());
    }

    #[test]
    fn bootconf_multi_word_init_is_one_pair() {
        assert_eq!(pairs(b"init=/arm64/init -c"), vec![(&b"init"[..], &b"/arm64/init -c"[..])]);
        // A continuation after a fresh pair extends only the LAST value.
        assert_eq!(
            pairs(b"a=1 b=2 tail"),
            vec![(&b"a"[..], &b"1"[..]), (&b"b"[..], &b"2 tail"[..])]
        );
    }

    #[test]
    fn bootconf_pairs_keep_order_and_literal_equals() {
        assert_eq!(pairs(b"a=1 b=2"), vec![(&b"a"[..], &b"1"[..]), (&b"b"[..], &b"2"[..])]);
        assert_eq!(
            pairs(b"key=a=b c=d=e"),
            vec![(&b"key"[..], &b"a=b"[..]), (&b"c"[..], &b"d=e"[..])]
        );
        assert_eq!(pairs(b"  a=1  "), vec![(&b"a"[..], &b"1"[..])]);
    }

    #[test]
    fn bootconf_leading_continuation_and_empty_key_are_invalid() {
        assert_eq!(parse_bootconf_pairs(b"-c"), Err(BootconfError::InvalidBootconfToken));
        assert_eq!(parse_bootconf_pairs(b"=value"), Err(BootconfError::InvalidBootconfToken));
        assert_eq!(parse_bootconf_pairs(b"a=1 =2"), Err(BootconfError::InvalidBootconfToken));
    }

    #[test]
    fn bootconf_pair_and_length_caps_are_loud() {
        // Exactly MAX_BOOTCONF_PAIRS parses; one more is refused.
        let mut many = String::new();
        for k in 0..MAX_BOOTCONF_PAIRS {
            many.push_str(&format!("k{k}=v "));
        }
        assert_eq!(
            parse_bootconf_pairs(many.as_bytes()).unwrap().pairs().count(),
            MAX_BOOTCONF_PAIRS
        );
        many.push_str("one=more");
        assert_eq!(parse_bootconf_pairs(many.as_bytes()), Err(BootconfError::TooManyBootconfPairs));
        // 256 bytes before the NUL is the cap itself; 257 is refused.
        let mut cap = vec![b'a'; MAX_CMDLINE];
        cap.push(0);
        assert_eq!(BootArgs::from_raw(&cap).unwrap().as_bytes().len(), MAX_CMDLINE);
        let mut over = vec![b'a'; MAX_CMDLINE + 1];
        assert!(matches!(BootArgs::from_raw(&over), Err(BootconfError::CmdlineTooLong)));
        over.push(0);
        assert!(matches!(BootArgs::from_raw(&over), Err(BootconfError::CmdlineTooLong)));
    }

    #[test]
    fn bootconf_bytes_after_the_first_nul_are_ignored() {
        let raw = BootArgs::from_raw(b"init=/arm64/init -c\0junk=1 more").unwrap();
        assert_eq!(raw.as_bytes(), b"init=/arm64/init -c");
        assert_eq!(
            parse_bootconf_pairs(raw.as_bytes()).unwrap().pairs().collect::<Vec<_>>(),
            vec![(&b"init"[..], &b"/arm64/init -c"[..])]
        );
        // A long tail after the NUL does not count toward the cap.
        let mut long_tail = b"a=1\0".to_vec();
        long_tail.extend(vec![b'z'; 2 * MAX_CMDLINE]);
        assert_eq!(BootArgs::from_raw(&long_tail).unwrap().as_bytes(), b"a=1");
    }
}
