//! D-06 layer 2: the cross-repo trap-frame drift gate.  Parses the frozen
//! `aarch64/include/ureg.h` in the sibling Harvey checkout and asserts the
//! Ureg field names/order/count match the pinned table r9's TrapFrame is
//! built against (aarch64/ABI.md, "Kernel-side enforcement (Phase 4
//! appendix)").  Layer 1 (the in-crate const asserts in
//! aarch64/src/trap.rs) is always on and guards bare builds; this layer
//! fails every boot-test profile on drift — SC#1: an ABI mismatch is a
//! build failure, not a runtime mystery.
//!
//! Plain text parsing — no crate.  The header is a frozen, hand-maintained
//! file, so the parser is deliberately narrow: it extracts `u64int <name>;`
//! declarations inside `typedef struct Ureg { ... }`, treating the r30/link
//! union as ONE slot (named by its first member).

use std::path::PathBuf;
use std::{env, fs};

use crate::{DynError, Result, workspace};

/// The pinned Ureg field order r9's TrapFrame matches: 35 slots, the
/// r30/link union occupying one (aarch64/ABI.md Phase-4 appendix).
const PINNED_FIELDS: [&str; 35] = [
    "r0", "r1", "r2", "r3", "r4", "r5", "r6", "r7", "r8", "r9", "r10", "r11", "r12", "r13",
    "r14", "r15", "r16", "r17", "r18", "r19", "r20", "r21", "r22", "r23", "r24", "r25", "r26",
    "r27", "r28", "r29", "link", "sp", "pc", "psr", "type",
];

/// The union slot's index in PINNED_FIELDS; the header's union may surface
/// under its first member's name instead of "link".
const UNION_SLOT: usize = 30;
const UNION_FIRST_MEMBER: &str = "r30";

/// What a check run found.
pub enum CheckStatus {
    /// The live header matches the pinned table.
    Match,
    /// No Harvey checkout at the resolved path — the caller decides whether
    /// that is tolerable (bare builds) or fatal (boot-test).
    SkippedNoHarvey,
}

/// The Harvey checkout to check against: $R9_HARVEY_DIR when set, else
/// `../harvey` relative to the r9 workspace root (D-03/D-06 cross-repo glue;
/// INT-04 replaces it with a flake input in Phase 5).  pub(crate) for the
/// 04-05 initrd step, which resolves the same checkout.
pub(crate) fn harvey_dir() -> PathBuf {
    match env::var("R9_HARVEY_DIR") {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => workspace().join("../harvey"),
    }
}

/// Run the check against the resolved Harvey checkout.  Standalone
/// (`cargo xtask ureg-check`) tolerates a missing checkout with a skip line
/// (layer 1 still guards); drift is always a hard error with a named-field
/// diff.
pub fn run_check() -> Result<CheckStatus> {
    let header = harvey_dir().join("aarch64/include/ureg.h");
    if !header.exists() {
        println!(
            "ureg-check: SKIP — {} not found (no Harvey checkout; the layer-1 const asserts still guard the layout)",
            header.display()
        );
        return Ok(CheckStatus::SkippedNoHarvey);
    }
    let text = fs::read_to_string(&header)
        .map_err(|e| format!("ureg-check: read {}: {e}", header.display()))?;
    compare_to_pinned(&text).map_err(|diff| -> DynError {
        format!("ureg-check: ureg.h drift detected in {}:\n{diff}", header.display()).into()
    })?;
    println!("ureg-check: 35 fields match aarch64/include/ureg.h");
    Ok(CheckStatus::Match)
}

/// The boot-test precondition (SC#1): drift fails the run, and so does a
/// missing Harvey checkout — the cross-repo gate is the point of the harness.
pub fn check_for_boot_test() -> Result<()> {
    match run_check()? {
        CheckStatus::Match => Ok(()),
        CheckStatus::SkippedNoHarvey => Err("ureg-check: boot-test requires the Harvey checkout \
             (set R9_HARVEY_DIR; default ../harvey) — the cross-repo gate is the point of the \
             harness"
            .into()),
    }
}

/// Parse the Ureg field names in order and compare against the pinned
/// table.  Err text is a per-field expected-vs-found diff.
fn compare_to_pinned(header_text: &str) -> std::result::Result<(), String> {
    let fields = extract_ureg_fields(header_text)
        .ok_or_else(|| "no `typedef struct Ureg { ... }` found".to_string())?;
    let mut problems: Vec<String> = Vec::new();
    if fields.len() != PINNED_FIELDS.len() {
        problems.push(format!(
            "  field count: expected {}, found {}",
            PINNED_FIELDS.len(),
            fields.len()
        ));
    }
    for (i, found) in fields.iter().enumerate() {
        let Some(&expected) = PINNED_FIELDS.get(i) else {
            problems.push(format!("  field {i}: unexpected extra field \"{found}\""));
            continue;
        };
        if found == expected || (i == UNION_SLOT && found == UNION_FIRST_MEMBER) {
            continue;
        }
        let expected = if i == UNION_SLOT {
            format!("\"{expected}\" (or union first member \"{UNION_FIRST_MEMBER}\")")
        } else {
            format!("\"{expected}\"")
        };
        problems.push(format!("  field {i}: expected {expected}, found \"{found}\""));
    }
    if fields.len() < PINNED_FIELDS.len() {
        for (i, expected) in PINNED_FIELDS.iter().enumerate().skip(fields.len()) {
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
    let open = rest.find('{')?;
    let mut depth = 0usize;
    let mut close = None;
    for (i, c) in rest.char_indices().skip(open) {
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
    let body = &rest[open + 1..close?];

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
}
