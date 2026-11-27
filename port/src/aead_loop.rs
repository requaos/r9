//! aead_loop — the boot-switch-gated in-kernel `9P2000.aead` loopback server
//! (SMP-13, D-28).
//!
//! This module is **RED stage scaffolding**.  The behaviour it documents and
//! the tests below are the plan's contract; the implementation follows in the
//! GREEN commit.  Nothing here is called on a default boot.

#![allow(dead_code)]

use alloc::sync::Arc;

use crypto::key::Key;

use crate::dat::Chan;
use crate::fid::FidTable;
use crate::Result;

/// Whether the kernel command line carries the exact `aeadloop=1` token.
pub fn parse_boot_switch(_cmdline: &str) -> bool {
    false
}

/// A live in-kernel loopback mount (only ever built behind `aeadloop=1`).
pub struct Loopback {
    root: Chan,
    table: Arc<FidTable>,
}

impl Loopback {
    /// A fresh fid-cloned `DEV_MNT` root chan for the `#M` registry arm.
    pub fn attach_root(&self, _spec: &[u8]) -> Result<Chan> {
        Err("namec: unknown # device")
    }

    #[allow(dead_code)]
    pub(crate) fn table(&self) -> &Arc<FidTable> {
        &self.table
    }
}

/// Build the loopback when (and only when) the boot switch is active.
pub fn start(_cmdline: &str) -> Option<Loopback> {
    None
}

/// Start and publish the loopback behind the boot switch; `true` when active.
pub fn maybe_start_from_cmdline(_cmdline: &str) -> bool {
    false
}

/// The `#M` registry arm: a fresh cloned root chan when active, `Err` when not.
pub fn attach_root(_spec: &[u8]) -> Result<Chan> {
    Err("namec: unknown # device")
}

#[cfg(test)]
pub(crate) fn build_with(_secret: Key, _table: Arc<FidTable>, _client_id: u32) -> Result<Loopback> {
    Err("aeadloop: not implemented")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dat::{Qid, OREAD, QTDIR};

    /// The plan's synthetic payload: 4 KiB per file, `data0`..`data7`.
    const FILE_LEN: usize = 4096;
    const DATA_FILES: usize = 8;
    const ROOT_PATH: u64 = 0x1000;

    fn expected_byte(file: usize, off: usize) -> u8 {
        (file as u8)
            .wrapping_mul(37)
            .wrapping_add((off as u8).wrapping_mul(3))
            .wrapping_add(1)
    }

    fn expected_file(file: usize) -> alloc::vec::Vec<u8> {
        (0..FILE_LEN).map(|off| expected_byte(file, off)).collect()
    }

    fn domain_key() -> Key {
        let mut bytes = [0u8; 32];
        for (i, b) in bytes.iter_mut().enumerate() {
            *b = 0xA0u8.wrapping_add(i as u8);
        }
        Key::from_bytes(bytes)
    }

    fn test_secret() -> Key {
        let mut bytes = [0u8; 32];
        for (i, b) in bytes.iter_mut().enumerate() {
            *b = 0x10u8.wrapping_add(i as u8);
        }
        Key::from_bytes(bytes)
    }

    fn entry(fid_key: u32, qid_path: u64, qid_type: u8) -> crate::fid::FidEntry {
        crate::fid::FidEntry {
            fid_key,
            client_id: 0,
            qid_path,
            offset: 0,
            qid_vers: 0,
            iounit: 0,
            lease_gen: 0,
            ring_slot: 0,
            dev_handler: 0,
            epoch: 0,
            qid_type,
            open_mode: 0,
            flags: 0,
            volatility: 0,
            _reserved: [0u8; 7],
        }
    }

    fn active_fids(table: &FidTable) -> u64 {
        (0..table.shard_count() as u8)
            .filter_map(|shard| table.bitmap_lens(shard))
            .map(|(active, _reclaimed)| active)
            .sum()
    }

    fn read_file(root: &Chan, idx: usize) -> alloc::vec::Vec<u8> {
        let name = alloc::format!("data{idx}");
        let walked = crate::namec::walk_one(root, name.as_bytes()).expect("walk data file");
        assert_eq!(
            walked.qid.path,
            ROOT_PATH + 1 + idx as u64,
            "the walked file carries its deterministic qid path"
        );
        let mut opened = crate::namec::open_chan(&walked, OREAD).expect("open data file");
        let mut buf = alloc::vec![0u8; FILE_LEN];
        let n = crate::devmnt::read(&mut opened, &mut buf, 0).expect("read data file") as usize;
        assert_eq!(n, FILE_LEN, "the whole deterministic file is returned");
        crate::namec::close_chan(&opened);
        drop(opened);
        drop(walked);
        buf.truncate(n);
        buf
    }

    fn global_guard() -> std::sync::MutexGuard<'static, ()> {
        static G: std::sync::Mutex<()> = std::sync::Mutex::new(());
        G.lock().unwrap_or_else(|poison| poison.into_inner())
    }

    #[test]
    fn aead_loop_parse_boot_switch_is_exact_token() {
        assert!(!parse_boot_switch("console=ttyS0"));
        assert!(!parse_boot_switch(""));
        assert!(!parse_boot_switch("aeadloop=0"));
        assert!(!parse_boot_switch("aeadloop=11"));
        assert!(!parse_boot_switch("xaeadloop=1"));
        assert!(!parse_boot_switch("aeadloop=1x"));
        assert!(parse_boot_switch("aeadloop=1"));
        assert!(parse_boot_switch("console=ttyS0 aeadloop=1 root=/dev/vda"));
    }

    #[test]
    fn aead_loop_dormant_default_boot_exposes_nothing() {
        let _g = global_guard();
        assert!(
            !maybe_start_from_cmdline("console=ttyS0 root=/dev/vda"),
            "a default boot must not start the loopback server"
        );
        assert_eq!(
            attach_root(b"").err(),
            Some("namec: unknown # device"),
            "the dormant #M attach is the donor Enodev shape"
        );
        assert!(start("console=ttyS0").is_none());
    }

    #[test]
    fn aead_loop_enabled_serves_deterministic_files() {
        let _g = global_guard();
        assert!(maybe_start_from_cmdline("aeadloop=1"), "the switch activates the loopback");
        let root = attach_root(b"").expect("the active #M attach returns the root chan");
        assert_eq!(root.qid.typ & QTDIR, QTDIR, "the root is a directory");
        for idx in 0..DATA_FILES {
            let got = read_file(&root, idx);
            assert_eq!(got, expected_file(idx), "file {idx} reads back its exact pattern");
        }
        drop(root);
    }

    #[test]
    fn aead_loop_two_threads_share_one_table_without_leaks() {
        let table = Arc::new(FidTable::synthetic(domain_key(), 1));
        let mut threads = alloc::vec::Vec::new();
        for client_id in [1u32, 2u32] {
            let shared = Arc::clone(&table);
            threads.push(std::thread::spawn(move || {
                let lb = build_with(test_secret(), shared, client_id).expect("build loopback");
                let baseline = active_fids(lb.table());
                for round in 0..25 {
                    for idx in 0..DATA_FILES {
                        let got = read_file(&lb.root, idx);
                        assert_eq!(
                            got,
                            expected_file(idx),
                            "client {client_id} round {round} file {idx} payload"
                        );
                    }
                    assert_eq!(
                        active_fids(lb.table()),
                        baseline,
                        "client {client_id} round {round} leaks no fids"
                    );
                }
            }));
        }
        for t in threads {
            t.join().expect("concurrent loopback thread");
        }

        // The table keeps the owning-client check: a handle issued to client 1
        // never resolves for client 2 (D-08).
        let handle = table.alloc(1, entry(1, ROOT_PATH, QTDIR)).expect("alloc for client 1");
        assert!(
            table.resolve(2, &handle).is_err(),
            "a client-1 handle is refused to client 2"
        );
        assert!(table.resolve(1, &handle).is_ok());
        table.clunk(1, &handle).expect("clunk the probe handle");
        let _ = Qid { path: 0, vers: 0, typ: 0 };
    }
}
