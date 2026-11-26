//! Document checks for `docs/threat-model.md` (DSGN-01, Phase 2 plan 02-02;
//! rescope alignment DEBT-05, Phase 5 plan 05-02).
//!
//! The document is read at run time, so a missing document is a failing
//! test with a message, not a compile error.  The scope boundary is
//! compared against its source, section 1 of the v3.0 architecture
//! compendium, so the two cannot drift apart unnoticed.
//!
//! Run with `cargo test -p xtask threat_model`.

use std::path::PathBuf;

const DOC_REL: &str = "../docs/threat-model.md";
const AC_REL: &str = "../docs/plan_r9_architecture_code_compendium_v3_0.md";

const AC_HEADING: &str = "### Impact Summary: Dropping Software Cryptographic Memory";
const AC_MMU_PREFIX: &str = "* Uniform hardware MMU isolation";
const STATUS_SENTENCE: &str =
    "Except where a row says otherwise, no protection described in this document is implemented in r9's code.";
const NEW_MARK: &str = "**[new]**";
const SOURCE_TAGS: [&str; 4] = ["[AC ", "[RFC ", "[IG ", "[PT "];

const SECTIONS: [&str; 8] = [
    "## 1. Scope boundary",
    "## 2. Assets",
    "## 3. Attacker classes and capabilities",
    "## 4. Trust boundaries",
    "## 5. Protections",
    "## 6. Out of scope and the remedy",
    "## 7. Open points and source contradictions",
    "## Sources",
];

fn read(rel: &str, what: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) => panic!("{what} is missing or unreadable at {}: {e}", path.display()),
    }
}

fn doc() -> String {
    read(DOC_REL, "the threat model document docs/threat-model.md")
}

/// Blocks of text separated by blank lines.
fn paragraphs(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
        } else {
            if !cur.is_empty() {
                cur.push('\n');
            }
            cur.push_str(line);
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// The lines from the heading line `from` (exclusive) up to the heading
/// line `to` (exclusive).  Headings are matched as whole lines.
fn between<'a>(text: &'a str, from: &str, to: &str) -> Vec<&'a str> {
    let lines: Vec<&str> = text.lines().collect();
    let start = lines
        .iter()
        .position(|l| l.trim_end() == from)
        .unwrap_or_else(|| panic!("the heading `{from}` is missing from docs/threat-model.md"));
    let end = lines
        .iter()
        .position(|l| l.trim_end() == to)
        .unwrap_or_else(|| panic!("the heading `{to}` is missing from docs/threat-model.md"));
    assert!(start < end, "the heading `{from}` must come before `{to}` in docs/threat-model.md");
    lines[start + 1..end].to_vec()
}

fn is_table_line(line: &str) -> bool {
    line.trim_start().starts_with('|')
}

/// A `|---|:---|` separator row: every cell holds only dashes, colons and spaces.
fn is_separator(line: &str) -> bool {
    let t = line.trim();
    is_table_line(line) && t.contains('-') && t.chars().all(|c| matches!(c, '|' | '-' | ':' | ' '))
}

/// Table body rows: table lines that are neither a separator nor the
/// header row directly above a separator.
fn table_body_rows<'a>(lines: &[&'a str]) -> Vec<&'a str> {
    let mut out = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if !is_table_line(line) || is_separator(line) {
            continue;
        }
        let is_header = lines.get(i + 1).is_some_and(|next| is_separator(next));
        if !is_header {
            out.push(*line);
        }
    }
    out
}

fn is_bullet(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("- ") || t.starts_with("* ")
}

fn is_cited_or_marked(line: &str) -> bool {
    line.contains(NEW_MARK) || SOURCE_TAGS.iter().any(|tag| line.contains(tag))
}

#[test]
fn threat_model_has_status_and_citation_rule() {
    let doc = doc();
    let paras = paragraphs(&doc);

    let status = paras
        .iter()
        .find(|p| p.starts_with("**Status.**"))
        .expect("docs/threat-model.md has no `**Status.**` paragraph");
    assert!(
        status.contains("living document"),
        "the `**Status.**` paragraph does not say \"living document\""
    );
    let status_one_line = status.replace('\n', " ");
    assert!(
        status_one_line.contains(STATUS_SENTENCE),
        "the `**Status.**` paragraph does not contain the sentence: {STATUS_SENTENCE}"
    );

    let rule = paras
        .iter()
        .find(|p| p.starts_with("**Citation rule.**"))
        .expect("docs/threat-model.md has no `**Citation rule.**` paragraph");
    for tag in ["[AC", "[RFC", "[IG", "[PT"] {
        assert!(rule.contains(tag), "the `**Citation rule.**` paragraph does not define the tag `{tag}`");
    }
    assert!(
        rule.contains(NEW_MARK),
        "the `**Citation rule.**` paragraph does not define the mark `{NEW_MARK}`"
    );
}

#[test]
fn threat_model_quotes_the_ac_boundary() {
    let doc = doc();
    let ac = read(AC_REL, "the v3.0 architecture compendium");

    let ac_lines: Vec<&str> = ac.lines().collect();
    let heading = ac_lines
        .iter()
        .position(|l| l.trim_end() == AC_HEADING)
        .unwrap_or_else(|| panic!("the v3.0 compendium has no heading `{AC_HEADING}`"));
    let after = &ac_lines[heading + 1..];

    // The memory boundary sentence is the first Retained bullet of the
    // rescope summary; the threat model must reproduce it word for word.
    let line = after
        .iter()
        .map(|l| l.trim_start())
        .find(|l| l.starts_with(AC_MMU_PREFIX))
        .unwrap_or_else(|| {
            panic!("the v3.0 compendium has no line starting `{AC_MMU_PREFIX}` after `{AC_HEADING}`")
        });
    let sentence = line.strip_prefix("* ").expect("bullet prefix checked above");
    assert!(
        doc.contains(sentence),
        "docs/threat-model.md does not reproduce the v3.0 compendium's memory-boundary line word for word: {sentence}"
    );

    for name in ["SEV-SNP", "TDX"] {
        assert!(doc.contains(name), "docs/threat-model.md does not name the remedy `{name}`");
    }
}

/// DEBT-05: the AC tag is defined against the v3.0 compendium in docs/, not
/// the superseded v4 compendium in .planning/.
#[test]
fn threat_model_defines_ac_as_the_v3_0_compendium() {
    let doc = doc();
    let ac_def = doc
        .lines()
        .find(|l| l.trim_start().starts_with("- AC:"))
        .expect("docs/threat-model.md Sources does not define the AC tag");
    assert!(
        ac_def.contains("docs/plan_r9_architecture_code_compendium_v3_0.md"),
        "the AC tag is not defined against the v3.0 compendium: {ac_def}"
    );
    assert!(
        !ac_def.contains("r9-architecture-and-code-v4"),
        "the AC tag still points at the superseded v4 compendium: {ac_def}"
    );
}

/// DEBT-05: the memory boundary is stated exactly, in the scope section, and
/// the dropped mechanisms are named as removed there — not as protections.
#[test]
fn threat_model_states_the_hardware_mmu_boundary() {
    let doc = doc();
    let section = between(&doc, "## 1. Scope boundary", "## 2. Assets").join("\n");
    for token in ["hardware MMU isolation", "`CR3`", "`TTBR0`", "`satp`"] {
        assert!(
            section.contains(token),
            "section 1 of docs/threat-model.md does not state {token} as part of the memory boundary"
        );
    }
    assert!(
        section.contains("2026-11-25"),
        "section 1 of docs/threat-model.md does not date the v3.0 rescope"
    );
    for dropped in ["pages at rest", "shadow-frame", "ymm0", "`PKEY`/`WRPKRU`", "sibling-key"] {
        assert!(
            section.contains(dropped),
            "section 1 of docs/threat-model.md does not name {dropped} among the removed-from-scope items"
        );
    }
}

/// DEBT-05: the exact bit-18 RFNOLEGACY wording, with no ambiguous fallback.
#[test]
fn threat_model_states_bit_18_rfnolegacy() {
    let doc = doc();
    for wording in [
        "bits 18-20 were retired with software cryptographic memory",
        "`RFNOLEGACY` is bit 18 (`1 << 18`)",
        "bits 19-21 are free",
    ] {
        assert!(doc.contains(wording), "docs/threat-model.md does not state: {wording}");
    }

    // Stale framings of the retired bits must not survive anywhere.
    for stale in [
        "bits 18-20 are free",
        "bits 18 to 20 are free",
        "bits 18-20 are unreserved",
        "bits 18 to 20 are unreserved",
        "bits 18-20 are reserved",
        "bits 18 to 20 are reserved",
        "bits 18-21 are reserved",
        "bits 18 to 21 are reserved",
    ] {
        assert!(
            !doc.contains(stale),
            "docs/threat-model.md carries the stale rfork framing: {stale}"
        );
    }
}

/// DEBT-05: no live protection may rest on a mechanism the 2026-11-25 rescope
/// dropped (software pages-at-rest encryption, stale-frame or shadow-frame
/// encryption, ymm sealing, PKEY/WRPKRU hooks, sibling-key isolation).
#[test]
fn threat_model_has_no_stale_crypto_memory_protections() {
    let doc = doc();
    let section = between(&doc, "## 5. Protections", "## 6. Out of scope and the remedy");
    let rows = table_body_rows(&section);
    let dropped = [
        "RFCKEY",
        "RFROBASE",
        "RFRANDMEM",
        "shadow frame",
        "shadow-frame",
        "ShadowFrame",
        "PKEY",
        "WRPKRU",
        "ymm",
        "stale-frame",
        "stale frame",
        "pages at rest",
        "page ciphertext",
    ];
    let stale: Vec<&str> = rows
        .iter()
        .copied()
        .filter(|row| dropped.iter().any(|token| row.contains(token)))
        .collect();
    assert!(
        stale.is_empty(),
        "{} protection row(s) in section 5 rest on mechanisms dropped by the v3.0 rescope:\n{}",
        stale.len(),
        stale.join("\n")
    );
}

#[test]
fn threat_model_has_required_sections() {
    let doc = doc();
    let lines: Vec<&str> = doc.lines().collect();
    let mut last: Option<usize> = None;
    for heading in SECTIONS {
        let at = lines
            .iter()
            .position(|l| l.trim_end() == heading)
            .unwrap_or_else(|| panic!("docs/threat-model.md has no heading `{heading}`"));
        if let Some(prev) = last {
            assert!(at > prev, "the heading `{heading}` is out of order in docs/threat-model.md");
        }
        last = Some(at);
    }
}

#[test]
fn threat_model_rows_and_bullets_are_cited_or_marked() {
    let doc = doc();
    // Sections 2 to 7.  Section 1 holds the quoted boundary line and is exempt.
    let body = between(&doc, "## 2. Assets", "## Sources");

    let mut checked = 0usize;
    let mut bare = Vec::new();
    for line in table_body_rows(&body) {
        checked += 1;
        if !is_cited_or_marked(line) {
            bare.push(line);
        }
    }
    for line in body.iter().filter(|l| is_bullet(l)) {
        checked += 1;
        if !is_cited_or_marked(line) {
            bare.push(line);
        }
    }

    assert!(checked > 0, "sections 2 to 7 of docs/threat-model.md hold no table row or bullet to check");
    assert!(
        bare.is_empty(),
        "{} row(s) or bullet(s) in sections 2 to 7 carry neither a source tag nor {NEW_MARK}:\n{}",
        bare.len(),
        bare.join("\n")
    );
}

/// The one status a section 5 row may carry instead of `not implemented`: the
/// Phase 4 wire protections, which are built and host-tested but reached by no
/// kernel (D-23).
const PHASE4_STATUS: &str = "implemented and host-tested in Phase 4; not used by a booted kernel";
const NOT_IMPLEMENTED: &str = "not implemented";
const USED_CLAIM: &str = "used by a booted kernel";
const NEGATION: &str = "not ";

/// True when `row` claims something is `used by a booted kernel` anywhere the
/// claim is not negated.  Section 5 may say the Phase 4 code exists; it may not
/// say a booted kernel runs it.
fn has_unnegated_use_claim(row: &str) -> bool {
    let mut rest = row;
    while let Some(at) = rest.find(USED_CLAIM) {
        if !rest[..at].ends_with(NEGATION) {
            return true;
        }
        rest = &rest[at + USED_CLAIM.len()..];
    }
    false
}

#[test]
fn threat_model_protections_are_not_overclaimed() {
    let doc = doc();
    let section = between(&doc, "## 5. Protections", "## 6. Out of scope and the remedy");
    let rows = table_body_rows(&section);
    assert!(
        rows.len() >= 10,
        "section 5 of docs/threat-model.md has {} protection row(s); at least 10 are required",
        rows.len()
    );
    let wrong: Vec<&str> = rows
        .iter()
        .copied()
        .filter(|r| !r.contains(NOT_IMPLEMENTED) && !r.contains(PHASE4_STATUS))
        .collect();
    assert!(
        wrong.is_empty(),
        "{} protection row(s) in section 5 carry neither the status `not implemented` nor the \
         Phase 4 status `{PHASE4_STATUS}`:\n{}",
        wrong.len(),
        wrong.join("\n")
    );

    // D-23: the six Phase 4 rows say the wire protections are host-tested and
    // reached by no kernel.  A row that says otherwise overstates what ships.
    let overclaimed: Vec<&str> = rows
        .iter()
        .copied()
        .filter(|r| r.contains(PHASE4_STATUS) && has_unnegated_use_claim(r))
        .collect();
    assert!(
        overclaimed.is_empty(),
        "{} protection row(s) carry the Phase 4 status but claim the code is used by a booted \
         kernel; D-23 says the six Phase 4 wire protections are not linked into a booted kernel:\n{}",
        overclaimed.len(),
        overclaimed.join("\n")
    );
}

#[test]
fn threat_model_makes_no_lock_claim() {
    let doc = doc();
    let hits: Vec<&str> = doc
        .lines()
        .filter(|line| {
            line.split(|c: char| !c.is_ascii_alphabetic())
                .any(|word| word.eq_ignore_ascii_case("locked"))
        })
        .collect();
    assert!(
        hits.is_empty(),
        "docs/threat-model.md uses the word `locked`; the document is a living document and fixes nothing:\n{}",
        hits.join("\n")
    );
}
