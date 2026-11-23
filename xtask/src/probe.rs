//! The throwaway swap probe (plan 01-20, D-17/D-27): build one `no_std`
//! Rust program as a Plan 9 amd64 `a.out` under `target/probe/x86_64/`.
//!
//! D-18: this module and `xtask/probe/` are temporary — the plan's
//! flip-back commit deletes both once the swap has proven HARN-04.  The
//! recipe is S6's (`spikes/s6/build-probe.sh` + `aout.py`), ported to
//! plain rustc + Rust so the wrap/check rules run in-tree: a flat
//! rust-lld payload at r9's `INITTEXT`, prefixed with the 40-byte
//! big-endian Plan 9 header r9's `x86_64/src/exec.rs` `parse_aout`
//! accepts.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use xtask::{Arch, DynError, Result};

/// The amd64 Plan 9 a.out magic (`S_MAGIC_AMD64`, `x86_64/src/exec.rs`).
pub const MAGIC: u32 = 0x8A97;
/// The amd64 a.out header size (`HDR_SIZE`, `x86_64/src/exec.rs`).
pub const HDR_SIZE: usize = 40;
/// The amd64 load/entry base (`INITTEXT`, `x86_64/src/exec.rs`).
pub const INITTEXT: u64 = 0x200028;

/// The amd64 probe source (S6's `probe-amd64.rs`, byte-verbatim).
const PROBE_SRC: &str = "xtask/probe/probe-amd64.rs";
/// The amd64 probe linker script (S6's `probe-amd64.ld`, flat at INITTEXT).
const PROBE_LD: &str = "xtask/probe/probe-amd64.ld";
/// The bare-metal target the S6 recipe links against.
const PROBE_TARGET: &str = "x86_64-unknown-none";

/// Build the amd64 probe and return the wrapped `ps.aout` path.
///
/// x86-64 only: the probe's source, linker script, header magic and
/// entry address are the amd64 ones (D-17 settles `ps`, which the probe
/// replaces in the cpu image).  Every other arch is a named error.
pub fn build_probe(arch: Arch) -> Result<PathBuf> {
    if arch != Arch::X86_64 {
        return Err(format!(
            "probe: --arch {arch:?} is not supported — the swap probe is x86-64 only \
             (its source, linker script, a.out magic and entry are the amd64 ones; \
             D-17/D-27)"
        )
        .into());
    }
    let root = super::workspace();
    let out_dir = root.join("target/probe/x86_64");
    fs::create_dir_all(&out_dir)?;
    let flat = out_dir.join("flat.bin");
    let ld = root.join(PROBE_LD);
    let src = root.join(PROBE_SRC);
    let status = Command::new(xtask::env_or("RUSTC", "rustc"))
        .current_dir(&root)
        .args([
            "--edition",
            "2021",
            "--crate-type",
            "bin",
            "--target",
            PROBE_TARGET,
            "-C",
            "panic=abort",
            "-C",
            "opt-level=s",
            "-C",
            "relocation-model=static",
        ])
        .arg("-C")
        .arg(format!("link-arg=-T{}", ld.display()))
        .arg("-C")
        .arg("link-arg=--oformat=binary")
        .arg("-o")
        .arg(&flat)
        .arg(&src)
        .status()
        .map_err(|e| -> DynError { format!("probe: spawn rustc: {e}").into() })?;
    if !status.success() {
        return Err(format!(
            "probe: rustc {} --target {PROBE_TARGET} failed ({status}) — the probe builds \
             inside r9's devshell (plain rustc, never a workspace member; D-18)",
            src.display()
        )
        .into());
    }
    let payload = fs::read(&flat)
        .map_err(|e| -> DynError { format!("probe: read {}: {e}", flat.display()).into() })?;
    let aout = wrap(&payload, INITTEXT);
    check(&aout).map_err(|e| -> DynError {
        format!("probe: the wrapped a.out failed its own parse_aout check: {e}").into()
    })?;
    let out = out_dir.join("ps.aout");
    fs::write(&out, &aout)
        .map_err(|e| -> DynError { format!("probe: write {}: {e}", out.display()).into() })?;
    println!(
        "probe: {} flat bytes -> {} a.out bytes at {}",
        payload.len(),
        aout.len(),
        out.display()
    );
    Ok(out)
}

/// Prepend the 40-byte Plan 9 a.out header to a flat payload.
///
/// Layout (S6 `aout.py wrap`): eight big-endian u32 fields — magic,
/// text length, data, bss, syms, the vestigial entry32, spsz, pcsz —
/// then the big-endian u64 entry at bytes 32..39, then the payload.
pub fn wrap(flat: &[u8], entry: u64) -> Vec<u8> {
    let text = u32::try_from(flat.len()).expect("the flat payload fits the a.out u32 text field");
    let mut out = Vec::with_capacity(HDR_SIZE + flat.len());
    for field in [MAGIC, text, 0, 0, 0, entry as u32, 0, 0] {
        out.extend_from_slice(&field.to_be_bytes());
    }
    out.extend_from_slice(&entry.to_be_bytes());
    out.extend_from_slice(flat);
    out
}

/// Assert the amd64 `parse_aout` rules on a wrapped probe.
///
/// Accepts only what r9's loader accepts: magic 0x8A97, non-zero text,
/// entry inside `[INITTEXT, INITTEXT + text)`, file length exactly
/// `HDR_SIZE + text + data`, and zero data/bss/syms (the probe is one
/// flat text segment).
pub fn check(aout: &[u8]) -> std::result::Result<(), String> {
    if aout.len() < HDR_SIZE {
        return Err(format!("short-header: {} bytes, need {HDR_SIZE}", aout.len()));
    }
    let u32_at = |offset: usize| {
        u32::from_be_bytes([
            aout[offset],
            aout[offset + 1],
            aout[offset + 2],
            aout[offset + 3],
        ])
    };
    let magic = u32_at(0);
    let text = u32_at(4);
    let data = u32_at(8);
    let bss = u32_at(12);
    let syms = u32_at(16);
    let entry = u64::from_be_bytes([
        aout[32], aout[33], aout[34], aout[35], aout[36], aout[37], aout[38], aout[39],
    ]);

    if magic != MAGIC {
        return Err(format!("magic: 0x{magic:08x}, want 0x{MAGIC:08x}"));
    }
    if text == 0 {
        return Err("text: the header's text field is 0 (parse_aout rejects it)".to_string());
    }
    if !(INITTEXT..INITTEXT + u64::from(text)).contains(&entry) {
        return Err(format!(
            "entry: 0x{entry:x} is outside text [0x{INITTEXT:x}, 0x{:x})",
            INITTEXT + u64::from(text)
        ));
    }
    let want_len = HDR_SIZE + text as usize + data as usize;
    if aout.len() != want_len {
        return Err(format!(
            "length: {} bytes, want {HDR_SIZE} + text({text}) + data({data}) = {want_len}",
            aout.len()
        ));
    }
    if data != 0 {
        return Err(format!("data: {data} (the probe is flat text only)"));
    }
    if bss != 0 {
        return Err(format!("bss: {bss} (the probe is flat text only)"));
    }
    if syms != 0 {
        return Err(format!("syms: {syms} (the probe carries no symbol table)"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The marker is the S6 one: the gate's `--serial-check` witness
    /// string is `R9-RUST-AOUT-PROBE-OK`.
    #[test]
    fn wrap_prepends_the_40_byte_plan9_header() {
        let flat = b"\x90\x90\xc3";
        let aout = wrap(flat, INITTEXT);
        assert_eq!(aout.len(), HDR_SIZE + flat.len());
        assert_eq!(&aout[0..4], &0x8A97u32.to_be_bytes());
        assert_eq!(&aout[4..8], &(flat.len() as u32).to_be_bytes());
        assert_eq!(&aout[8..12], &0u32.to_be_bytes());
        assert_eq!(&aout[12..16], &0u32.to_be_bytes());
        assert_eq!(&aout[16..20], &0u32.to_be_bytes());
        assert_eq!(&aout[20..24], &(INITTEXT as u32).to_be_bytes());
        assert_eq!(&aout[24..28], &0u32.to_be_bytes());
        assert_eq!(&aout[28..32], &0u32.to_be_bytes());
        assert_eq!(&aout[32..40], &INITTEXT.to_be_bytes());
        assert_eq!(&aout[HDR_SIZE..], flat);
    }

    #[test]
    fn check_accepts_the_wrapped_payload() {
        let aout = wrap(b"\x90\x90\xc3", INITTEXT);
        assert_eq!(check(&aout), Ok(()));
    }

    /// Every parse_aout refusal is a named error, one broken field per
    /// fixture: wrong magic, empty text, entry outside text, a length
    /// that disagrees with the header, and non-zero data/bss/syms.
    #[test]
    fn check_rejects_each_parse_aout_violation() {
        let good = wrap(b"\x90\x90\xc3", INITTEXT);

        let mut wrong_magic = good.clone();
        wrong_magic[0..4].copy_from_slice(&0x8C47u32.to_be_bytes());
        assert!(check(&wrong_magic).unwrap_err().contains("magic"));

        let empty_text = wrap(&[], INITTEXT);
        assert!(check(&empty_text).unwrap_err().contains("text"));

        let mut outside = good.clone();
        outside[32..40].copy_from_slice(&(INITTEXT + 3).to_be_bytes());
        assert!(check(&outside).unwrap_err().contains("entry"));

        let mut short = good.clone();
        short.pop();
        assert!(check(&short).unwrap_err().contains("length"));

        for offset in [8usize, 12, 16] {
            let mut nonzero = good.clone();
            nonzero[offset..offset + 4].copy_from_slice(&1u32.to_be_bytes());
            assert!(check(&nonzero).is_err(), "offset {offset} must be refused");
        }
    }

    /// The probe is amd64-only (D-17/D-27): the other arches are a named
    /// error, never a silent build with the wrong magic/entry.
    #[test]
    fn build_probe_names_the_arch_for_non_x86_64() {
        for arch in [Arch::Aarch64, Arch::Riscv64] {
            let err = build_probe(arch).unwrap_err().to_string();
            assert!(
                err.contains("x86-64") && err.contains("probe"),
                "the {arch:?} refusal must name the probe and the supported arch: {err}"
            );
        }
    }
}
