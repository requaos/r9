//! The cross-repo trap-frame drift gate, dual-arch (D-02b).  Parses
//! the frozen `aarch64/include/ureg.h` AND `amd64/include/ureg.h` in
//! the Harvey checkout named by R9_HARVEY_DIR and asserts the Ureg
//! field names/order/count match the pinned tables r9's TrapFrame and
//! x86_64 Ureg are built against (aarch64/ABI.md and amd64/ABI.md
//! kernel-side enforcement appendices).  The in-crate const asserts
//! (aarch64/src/trap.rs, x86_64/src/dat.rs) are always on; this gate
//! fails every boot-test profile on drift — an ABI mismatch is a
//! build failure, not a runtime mystery.
//!
//! The implicit `../harvey`
//! sibling-checkout default is REMOVED — every xtask path that touches the
//! Harvey checkout requires R9_HARVEY_DIR explicitly, and an unset variable
//! is a named error, never a silent relative-path guess.
//!
//! Plain text parsing — no crate.  The headers are frozen,
//! hand-maintained files, so the parser is deliberately narrow, per
//! arch:
//!   - aarch64: `u64int <name>;` declarations inside
//!     `typedef struct Ureg { ... }`, the r30/link union ONE slot
//!     (named by its first member).
//!   - amd64: `u64int`/`u16int` declarations inside the bare
//!     `struct Ureg { ... }` (no typedef wrapper), with the u16int
//!     ds/es/fs/gs quartet pinned EXPLICITLY as four fields of ONE
//!     qword (slots 15-18, bytes 120-126 of the 184-byte frame) —
//!     never skipped or wildcarded (Pitfall 9: a relaxed parser
//!     certifies nothing).

use std::path::PathBuf;
use std::{env, fs};

use crate::{Arch, DynError, Result};

/// The pinned aarch64 Ureg field order r9's TrapFrame matches: 35
/// slots, the r30/link union occupying one (aarch64/ABI.md's
/// kernel-side enforcement appendix).
const PINNED_FIELDS: [&str; 35] = [
    "r0", "r1", "r2", "r3", "r4", "r5", "r6", "r7", "r8", "r9", "r10", "r11", "r12", "r13",
    "r14", "r15", "r16", "r17", "r18", "r19", "r20", "r21", "r22", "r23", "r24", "r25", "r26",
    "r27", "r28", "r29", "link", "sp", "pc", "psr", "type",
];

/// The union slot's index in PINNED_FIELDS; the header's union may surface
/// under its first member's name instead of "link".
const UNION_SLOT: usize = 30;
const UNION_FIRST_MEMBER: &str = "r30";

/// The pinned amd64 Ureg field order r9's x86_64 Ureg matches
/// (amd64/ABI.md's kernel-side enforcement appendix): 26 declarations
/// — 22 u64int fields (ax..r15, type/error/ip/cs/flags/sp/ss) plus
/// the four u16int segment fields.  Slots 0-14 are the GP qwords,
/// slots 15-18 are the ds/es/fs/gs quartet (four u16 fields sharing
/// ONE qword — bytes 120-126 of the 184-byte frame; the packing rule
/// is the point, never a skip), slots 19-25 are
/// type/error/ip/cs/flags/sp/ss at frame bytes 128..184.
const PINNED_FIELDS_AMD64: [&str; 26] = [
    "ax", "bx", "cx", "dx", "si", "di", "bp", "r8", "r9", "r10", "r11", "r12", "r13", "r14",
    "r15", "ds", "es", "fs", "gs", "type", "error", "ip", "cs", "flags", "sp", "ss",
];

/// What a check run found.
pub enum CheckStatus {
    /// The live header matches the pinned table.
    Match,
    /// No Harvey checkout at the resolved path — the caller decides whether
    /// that is tolerable (bare builds) or fatal (boot-test).
    SkippedNoHarvey,
}

/// The Harvey checkout to check against: $R9_HARVEY_DIR — REQUIRED.
/// The implicit `../harvey` default was removed
/// here; an unset (or empty) variable is a named error, never a silent
/// relative-path guess.  pub(crate) for the initrd step, which
/// resolves the same checkout.
pub(crate) fn harvey_dir() -> Result<PathBuf> {
    match env::var("R9_HARVEY_DIR") {
        Ok(dir) if !dir.is_empty() => Ok(PathBuf::from(dir)),
        _ => Err("R9_HARVEY_DIR is not set — point it at the Harvey checkout (it must contain \
             aarch64/include/ureg.h and amd64/include/ureg.h, and for the initrd bake \
             build/aarch64/smoke-out/ plus .build/bin/mkpaqfs); r9's xtask no longer defaults \
             to a sibling ../harvey checkout (05-CONTEXT D-10)"
            .into()),
    }
}

/// Run the check against the resolved Harvey checkout, both arms.
/// Standalone (`cargo xtask ureg-check`) fails when R9_HARVEY_DIR is
/// unset and tolerates a named-but-headerless checkout with a skip
/// line per header (the const asserts still guard); drift is always a
/// hard error with a named-field diff.  The amd64 arm is followed by
/// the D-02b negative control (after the real check).
pub fn run_check() -> Result<CheckStatus> {
    let dir = harvey_dir()?;

    // Arm 1: aarch64 (frozen at the v2.0 pin — checked, not changed).
    let header = dir.join("aarch64/include/ureg.h");
    let mut aarch64 = CheckStatus::SkippedNoHarvey;
    if !header.exists() {
        println!(
            "ureg-check: SKIP — {} not found (no Harvey checkout; the layer-1 const asserts still guard the layout)",
            header.display()
        );
    } else {
        let text = fs::read_to_string(&header)
            .map_err(|e| format!("ureg-check: read {}: {e}", header.display()))?;
        compare_to_pinned(&text).map_err(|diff| -> DynError {
            format!("ureg-check: ureg.h drift detected in {}:\n{diff}", header.display()).into()
        })?;
        println!("ureg-check: 35 fields match aarch64/include/ureg.h");
        aarch64 = CheckStatus::Match;
    }

    // Arm 2: amd64 (the Phase 14 XEXEC-01 frozen header).
    let header = dir.join("amd64/include/ureg.h");
    let mut amd64 = CheckStatus::SkippedNoHarvey;
    if !header.exists() {
        println!(
            "ureg-check: SKIP — {} not found (no Harvey checkout; the layer-1 const asserts still guard the layout)",
            header.display()
        );
    } else {
        let text = fs::read_to_string(&header)
            .map_err(|e| format!("ureg-check: read {}: {e}", header.display()))?;
        compare_to_pinned_amd64(&text).map_err(|diff| -> DynError {
            format!("ureg-check: ureg.h drift detected in {}:\n{diff}", header.display()).into()
        })?;
        println!("ureg-check: 26 fields match amd64/include/ureg.h");
        amd64 = CheckStatus::Match;
        // D-02b: prove the gate can catch drift, not just parse the
        // pristine header — the mutated copy MUST be rejected.
        run_amd64_negative_control(&text)?;
    }

    match (aarch64, amd64) {
        (CheckStatus::Match, _) | (_, CheckStatus::Match) => Ok(CheckStatus::Match),
        (CheckStatus::SkippedNoHarvey, CheckStatus::SkippedNoHarvey) => {
            Ok(CheckStatus::SkippedNoHarvey)
        }
    }
}

/// The boot-test precondition: drift fails the run, and so does an
/// unset or headerless Harvey checkout — the cross-repo gate is the
/// point of the harness.  Each arch requires ITS OWN frozen header:
/// aarch64 profiles pin aarch64/include/ureg.h, x86_64 profiles pin
/// amd64/include/ureg.h.
pub fn check_for_boot_test(arch: Arch) -> Result<()> {
    let dir = harvey_dir()?;
    let (header, arm) = match arch {
        Arch::Aarch64 => (dir.join("aarch64/include/ureg.h"), "aarch64/include/ureg.h"),
        Arch::X86_64 => (dir.join("amd64/include/ureg.h"), "amd64/include/ureg.h"),
        Arch::Riscv64 => {
            return Err("ureg-check: boot-test currently supports aarch64 and x86_64 only".into())
        }
    };
    if !header.exists() {
        return Err(format!(
            "ureg-check: boot-test requires the Harvey checkout (R9_HARVEY_DIR must name it — \
             it must contain {arm}; 05-CONTEXT D-10 removed the ../harvey default) — the \
             cross-repo gate is the point of the harness"
        )
        .into());
    }
    let text = fs::read_to_string(&header)
        .map_err(|e| format!("ureg-check: read {}: {e}", header.display()))?;
    match arch {
        Arch::Aarch64 => compare_to_pinned(&text),
        Arch::X86_64 => compare_to_pinned_amd64(&text),
        Arch::Riscv64 => unreachable!("rejected above"),
    }
    .map_err(|diff| -> DynError {
        format!("ureg-check: ureg.h drift detected in {}:\n{diff}", header.display()).into()
    })?;
    Ok(())
}

/// The D-02b negative control: a deliberately-mutated copy of the
/// real amd64 header MUST be rejected with a named-field diff.  If
/// the mutated copy ever PASSES the check, the control prints
/// NEGATIVE CONTROL FAILED and fails the run — a gate that cannot
/// catch drift certifies nothing (the mutation is in-memory; the
/// committed header is never touched).
pub(crate) fn run_amd64_negative_control(header_text: &str) -> Result<()> {
    let mutated = header_text.replacen("u64int\tcs;", "u64int\tcseg;", 1);
    if mutated == header_text {
        return Err("NEGATIVE CONTROL FAILED: the mutation did not apply — the amd64 header no \
             longer declares `u64int\tcs;` where the control expects it; update the control's \
             mutation target to a field the frozen header still carries"
            .into());
    }
    match compare_to_pinned_amd64(&mutated) {
        Err(diff) if diff.contains("expected \"cs\"") && diff.contains("found \"cseg\"") => {
            println!(
                "ureg-check: negative control fired — the mutated amd64 header copy (cs -> cseg) was rejected with a named-field diff"
            );
            Ok(())
        }
        Err(diff) => Err(format!(
            "NEGATIVE CONTROL FAILED: the mutated copy was rejected but the diff does not name \
             the mutated field (cs -> cseg):\n{diff}"
        )
        .into()),
        Ok(()) => Err("NEGATIVE CONTROL FAILED: the mutated amd64 header copy (cs -> cseg) \
             PASSED the check — the gate cannot catch drift; do not relax the parser, fix it"
            .into()),
    }
}

/// Parse the aarch64 Ureg field names in order and compare against
/// the pinned table.  Err text is a per-field expected-vs-found diff.
fn compare_to_pinned(header_text: &str) -> std::result::Result<(), String> {
    let fields = extract_ureg_fields(header_text)
        .ok_or_else(|| "no `typedef struct Ureg { ... }` found".to_string())?;
    compare_fields(&fields, &PINNED_FIELDS, Some((UNION_SLOT, UNION_FIRST_MEMBER)))
}

/// Parse the amd64 Ureg field names in order (u64int AND u16int
/// declarations, bare struct form) and compare against the pinned
/// table.  Err text is a per-field expected-vs-found diff.
fn compare_to_pinned_amd64(header_text: &str) -> std::result::Result<(), String> {
    let fields = extract_ureg_fields_amd64(header_text)
        .ok_or_else(|| "no bare `struct Ureg { ... }` found (amd64)".to_string())?;
    compare_fields(&fields, &PINNED_FIELDS_AMD64, None)
}

/// Shared per-field comparison; `union_slot` carries the aarch64
/// r30/link first-member alias (the amd64 table has no union).
fn compare_fields(
    fields: &[String],
    pinned: &[&str],
    union_slot: Option<(usize, &str)>,
) -> std::result::Result<(), String> {
    let mut problems: Vec<String> = Vec::new();
    if fields.len() != pinned.len() {
        problems.push(format!(
            "  field count: expected {}, found {}",
            pinned.len(),
            fields.len()
        ));
    }
    for (i, found) in fields.iter().enumerate() {
        let Some(&expected) = pinned.get(i) else {
            problems.push(format!("  field {i}: unexpected extra field \"{found}\""));
            continue;
        };
        let union_here = union_slot.is_some_and(|(slot, _)| slot == i);
        if found == expected || (union_here && found == union_slot.unwrap().1) {
            continue;
        }
        let expected = if union_here {
            let (_, first) = union_slot.unwrap();
            format!("\"{expected}\" (or union first member \"{first}\")")
        } else {
            format!("\"{expected}\"")
        };
        problems.push(format!("  field {i}: expected {expected}, found \"{found}\""));
    }
    if fields.len() < pinned.len() {
        for (i, expected) in pinned.iter().enumerate().skip(fields.len()) {
            problems.push(format!("  field {i}: missing \"{expected}\""));
        }
    }
    if problems.is_empty() { Ok(()) } else { Err(problems.join("\n")) }
}

/// Extract the Ureg field names in declaration order.  The r30/link union
/// contributes ONE slot, named by its first member.
fn extract_ureg_fields(text: &str) -> Option<Vec<String>> {
    let cleaned = strip_block_comments(text);
    let start = cleaned.find("typedef struct Ureg")?;
    let rest = &cleaned[start..];
    let body = brace_body(rest)?;

    let mut fields: Vec<String> = Vec::new();
    let mut in_union = false;
    let mut union_named = false;
    for line in body.lines() {
        let line = line.trim();
        if line.starts_with("union") {
            in_union = true;
            union_named = false;
            continue;
        }
        if in_union && line.starts_with("}") {
            in_union = false;
            continue;
        }
        let Some(rest) = line.strip_prefix("u64int") else { continue };
        let name = rest.trim().trim_end_matches(';');
        if name.is_empty() {
            continue;
        }
        // Union members share one slot: only the first names it.
        if in_union && union_named {
            continue;
        }
        if in_union {
            union_named = true;
        }
        fields.push(name.to_string());
    }
    Some(fields)
}

/// Extract the amd64 Ureg field names in declaration order: `u64int`
/// AND `u16int` declarations inside the bare `struct Ureg { ... }`
/// (no typedef wrapper).  The u16int quartet lands as four ordinary
/// slots — the packing rule is enforced by the pinned table, never by
/// skipping the fields (Pitfall 9).
fn extract_ureg_fields_amd64(text: &str) -> Option<Vec<String>> {
    let cleaned = strip_block_comments(text);
    let start = find_bare_struct_ureg(&cleaned)?;
    let rest = &cleaned[start..];
    let body = brace_body(rest)?;

    let mut fields: Vec<String> = Vec::new();
    for line in body.lines() {
        let line = line.trim();
        let name = if let Some(rest) = line.strip_prefix("u64int") {
            rest
        } else {
            let Some(rest) = line.strip_prefix("u16int") else { continue };
            rest
        };
        let name = name.trim().trim_end_matches(';');
        if name.is_empty() {
            continue;
        }
        fields.push(name.to_string());
    }
    Some(fields)
}

/// The text between the first `{` in `s` and its matching `}`.
fn brace_body(s: &str) -> Option<&str> {
    let open = s.find('{')?;
    let mut depth = 0usize;
    let mut close = None;
    for (i, c) in s.char_indices().skip(open) {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }
    Some(&s[open + 1..close?])
}

/// Find the bare `struct Ureg` (NOT preceded by `typedef`) — the
/// amd64 header's form.  Returns the byte index of `struct`.
fn find_bare_struct_ureg(cleaned: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(rel) = cleaned[from..].find("struct Ureg") {
        let at = from + rel;
        let before = cleaned[..at].trim_end();
        if !before.ends_with("typedef") {
            return Some(at);
        }
        from = at + "struct Ureg".len();
    }
    None
}

/// Remove C block comments (no nesting in the frozen header).
fn strip_block_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            let mut prev = '\0';
            for c2 in chars.by_ref() {
                if prev == '*' && c2 == '/' {
                    break;
                }
                prev = c2;
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The frozen header's exact shape (aarch64/include/ureg.h), comments
    /// and tabs included.
    const FROZEN_UREG: &str = "\
typedef struct Ureg {
	u64int	r0;
	u64int	r1;
	u64int	r26;
	u64int	r27;
	u64int	r28;	/* sb */
	u64int	r29;
	union {
		u64int	r30;
		u64int	link;
	};
	u64int	sp;
	u64int	pc;	/* interrupted addr */
	u64int	psr;
	u64int	type;	/* of exception */
} Ureg;
";

    /// The full 35-field frozen header (the loop-written middle registers
    /// spelled out, as the real file spells them).
    fn frozen_header() -> String {
        let mut s = String::from("typedef struct Ureg {\n");
        for i in 0..=28 {
            if i == 28 {
                s.push_str("\tu64int\tr28;\t/* sb */\n");
            } else {
                s.push_str(&format!("\tu64int\tr{i};\n"));
            }
        }
        s.push_str("\tu64int\tr29;\n\tunion {\n\t\tu64int\tr30;\n\t\tu64int\tlink;\n\t};\n");
        s.push_str("\tu64int\tsp;\n\tu64int\tpc;\t/* interrupted addr */\n\tu64int\tpsr;\n");
        s.push_str("\tu64int\ttype;\t/* of exception */\n} Ureg;\n");
        s
    }

    /// The frozen amd64 header (amd64/include/ureg.h), comments and
    /// tabs included: the bare struct form, the u16int quartet, the
    /// seven-qword tail.
    fn frozen_header_amd64() -> String {
        let mut s = String::from("struct Ureg {\n");
        for name in ["ax", "bx", "cx", "dx", "si", "di", "bp", "r8", "r9", "r10", "r11", "r12",
            "r13", "r14", "r15"]
        {
            s.push_str(&format!("\tu64int\t{name};\n"));
        }
        s.push_str("\n\tu16int\tds;\n\tu16int\tes;\n\tu16int\tfs;\n\tu16int\tgs;\n\n");
        s.push_str("\tu64int\ttype;\n");
        s.push_str("\tu64int\terror;\t\t\t\t/* error code (or zero) */\n");
        s.push_str("\tu64int\tip;\t\t\t\t/* pc */\n");
        s.push_str("\tu64int\tcs;\t\t\t\t/* old context */\n");
        s.push_str("\tu64int\tflags;\t\t\t\t/* old flags */\n");
        s.push_str("\tu64int\tsp;\t\t\t\t/* sp */\n");
        s.push_str("\tu64int\tss;\t\t\t\t/* old stack segment */\n");
        s.push_str("};\n");
        s
    }

    #[test]
    fn frozen_header_matches_the_pinned_table() {
        assert_eq!(compare_to_pinned(&frozen_header()), Ok(()));
    }

    #[test]
    fn union_counts_as_one_slot_under_either_name() {
        let fields = extract_ureg_fields(&frozen_header()).unwrap();
        assert_eq!(fields.len(), 35);
        assert_eq!(fields[UNION_SLOT], "r30"); // first member names the slot
        // A header that names the union slot "link" first also passes.
        let link_first = frozen_header().replace("\tu64int\tr30;\n\t\tu64int\tlink;", "\tu64int\tlink;\n\t\tu64int\tr30;");
        assert_eq!(compare_to_pinned(&link_first), Ok(()));
    }

    #[test]
    fn drift_names_the_divergent_field() {
        let drifted = frozen_header().replace("u64int\tpsr;", "u64int\tpsrX;");
        let err = compare_to_pinned(&drifted).unwrap_err();
        assert!(err.contains("field 33"), "{err}");
        assert!(err.contains("\"psr\""), "{err}");
        assert!(err.contains("\"psrX\""), "{err}");
    }

    #[test]
    fn field_count_drift_is_reported() {
        let dropped = frozen_header().replace("\tu64int\ttype;\t/* of exception */\n", "");
        let err = compare_to_pinned(&dropped).unwrap_err();
        assert!(err.contains("expected 35, found 34"), "{err}");
        assert!(err.contains("missing \"type\""), "{err}");
    }

    #[test]
    fn block_comments_are_stripped_before_parsing() {
        let fields = extract_ureg_fields(FROZEN_UREG).unwrap();
        // The truncated fixture still parses cleanly (comments on r28/pc/type
        // must not leak into names).
        assert!(fields.contains(&"r28".to_string()));
        assert!(fields.contains(&"pc".to_string()));
        assert!(!fields.iter().any(|f| f.contains("sb") || f.contains('/')));
    }

    #[test]
    fn amd64_frozen_header_matches_the_pinned_table() {
        assert_eq!(compare_to_pinned_amd64(&frozen_header_amd64()), Ok(()));
        let fields = extract_ureg_fields_amd64(&frozen_header_amd64()).unwrap();
        assert_eq!(fields.len(), 26);
        // The u16 quartet is pinned at slots 15-18 (four fields of one
        // qword, bytes 120-126) — never skipped (Pitfall 9).
        assert_eq!(&fields[15..19], &["ds", "es", "fs", "gs"]);
        assert_eq!(fields[19], "type");
        assert_eq!(fields[25], "ss");
    }

    #[test]
    fn amd64_drift_names_the_divergent_field() {
        let drifted = frozen_header_amd64().replace("u64int\tcs;", "u64int\tcseg;");
        let err = compare_to_pinned_amd64(&drifted).unwrap_err();
        assert!(err.contains("field 22"), "{err}");
        assert!(err.contains("\"cs\""), "{err}");
        assert!(err.contains("\"cseg\""), "{err}");
    }

    #[test]
    fn amd64_field_count_drift_is_reported() {
        let dropped = frozen_header_amd64()
            .replace("\tu64int\tss;\t\t\t\t/* old stack segment */\n", "");
        let err = compare_to_pinned_amd64(&dropped).unwrap_err();
        assert!(err.contains("expected 26, found 25"), "{err}");
        assert!(err.contains("missing \"ss\""), "{err}");
    }

    #[test]
    fn amd64_quartet_removal_is_drift_not_a_pass() {
        // Pitfall 9: a relaxed parser that skips the u16int quartet
        // would certify this header — the pinned table demands the
        // four fields, so their absence is a loud count drift.
        let mut relaxed = frozen_header_amd64();
        for name in ["ds", "es", "fs", "gs"] {
            relaxed = relaxed.replace(&format!("\tu16int\t{name};\n"), "");
        }
        let err = compare_to_pinned_amd64(&relaxed).unwrap_err();
        assert!(err.contains("expected 26, found 22"), "{err}");
        // The quartet's first field surfaces as a named per-field diff
        // (the slots shifted up), and the tail as missing fields.
        assert!(err.contains("field 15: expected \"ds\""), "{err}");
        assert!(err.contains("missing \"ss\""), "{err}");
    }

    #[test]
    fn amd64_typedef_form_is_not_the_bare_struct() {
        // The bare-struct finder must not latch onto a typedef'd
        // `struct Ureg` (the aarch64 form) when parsing amd64.
        assert!(extract_ureg_fields_amd64(&frozen_header()).is_none());
    }

    #[test]
    fn amd64_negative_control_fires_and_loudly_fails_when_broken() {
        // The control leg: the mutated copy is rejected with the
        // named-field diff (Ok is the pass).
        run_amd64_negative_control(&frozen_header_amd64()).unwrap();
        // A control whose mutation cannot apply is the loud
        // NEGATIVE CONTROL FAILED failure, never a pass.
        let nomut = frozen_header_amd64().replace("u64int\tcs;", "u64int\tcx;");
        let err = run_amd64_negative_control(&nomut).unwrap_err().to_string();
        assert!(err.contains("NEGATIVE CONTROL FAILED"), "{err}");
    }
}
