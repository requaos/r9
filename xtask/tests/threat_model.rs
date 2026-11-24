//! Document checks for `docs/threat-model.md` (DSGN-01, Phase 2 plan 02-02).
//!
//! The document is read at run time, so a missing document is a failing
//! test with a message, not a compile error.  The scope boundary is
//! compared against its source, section 2.2 of the architecture
//! compendium, so the two cannot drift apart unnoticed.
//!
//! Run with `cargo test -p xtask threat_model`.

use std::path::PathBuf;

const DOC_REL: &str = "../docs/threat-model.md";
const AC_REL: &str = "../.planning/r9-architecture-and-code-v4.md";

const AC_HEADING: &str = "### 2.2 Architectural Threat Model Boundaries";
const STATUS_SENTENCE: &str = "No protection described in this document is implemented in r9's code.";
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
    let ac = read(AC_REL, "the architecture compendium");

    let ac_lines: Vec<&str> = ac.lines().collect();
    let heading = ac_lines
        .iter()
        .position(|l| l.trim_end() == AC_HEADING)
        .unwrap_or_else(|| panic!("the compendium has no heading `{AC_HEADING}`"));
    let after = &ac_lines[heading + 1..];

    for prefix in ["* **In Scope:**", "* **Out of Scope:**"] {
        let line = after
            .iter()
            .find(|l| l.starts_with(prefix))
            .unwrap_or_else(|| panic!("the compendium has no line starting `{prefix}` after `{AC_HEADING}`"));
        let sentence = line.strip_prefix("* ").expect("prefix checked above");
        assert!(
            doc.contains(sentence),
            "docs/threat-model.md does not reproduce the compendium's section 2.2 line word for word: {sentence}"
        );
    }

    for name in ["SEV-SNP", "TDX"] {
        assert!(doc.contains(name), "docs/threat-model.md does not name the remedy `{name}`");
    }
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
    // Sections 2 to 7.  Section 1 holds the two quoted boundary lines and is exempt.
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

#[test]
fn threat_model_protections_are_not_implemented() {
    let doc = doc();
    let section = between(&doc, "## 5. Protections", "## 6. Out of scope and the remedy");
    let rows = table_body_rows(&section);
    assert!(
        rows.len() >= 10,
        "section 5 of docs/threat-model.md has {} protection row(s); at least 10 are required",
        rows.len()
    );
    let wrong: Vec<&str> = rows.iter().copied().filter(|r| !r.contains("not implemented")).collect();
    assert!(
        wrong.is_empty(),
        "{} protection row(s) in section 5 do not carry the status `not implemented`:\n{}",
        wrong.len(),
        wrong.join("\n")
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
