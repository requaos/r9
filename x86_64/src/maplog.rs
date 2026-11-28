//! x86_64/src/maplog.rs — a proc's user mappings, indexed by virtual
//! address.
//!
//! Every user mapping a proc owns is one record (proc.rs's
//! `MappingRecord`).  Until 2026-11-27 the records sat in a `Vec` inside
//! the fixed kernel heap, and every page fault asked "is this page
//! mapped?" by scanning it.  A proc that mapped a gibibyte needed an
//! 8 MiB heap vector — then a 16 MiB one, where the 32 MiB heap ran out
//! (measured on the KVM host: `oom: Layout { size: 16777216 }`) — and a
//! quarter of a million linear scans.  The log is a radix tree keyed by
//! VA instead:
//!
//! - four levels of 512-entry nodes, each ONE page from the frame
//!   allocator and never from the kernel heap: the root, one node per
//!   512 GiB of VA in use, one per gibibyte, one leaf per 2 MiB;
//! - a leaf slot IS the record: the frame's physical address in its low
//!   48 bits, the segment kind above them, a present bit;
//! - insert, lookup and removal take a few table reads; the walk yields
//!   the records in address order.
//!
//! A proc that maps 3 GiB of contiguous VA costs about 6 MiB of index
//! pages — two per mille — and every index page goes back as soon as it
//! holds no record.
//!
//! The nodes are reached through `table_entry`/`set_table_entry`
//! (proc.rs's audited page-table accessors), so nothing here is unsafe.

use alloc::vec::Vec;
use core::marker::PhantomData;

use port::mem::PAGE_SIZE_4K;

use crate::proc::{MappingRecord, SegKind, alloc_phys_page, free_phys_page, set_table_entry, table_entry};

/// The levels: the root, then one per nine address bits down to the page.
const LEVELS: usize = 4;
const ENTRIES: usize = 512;
/// The present bit of an entry (an index node's and a leaf's alike).
const PRESENT: u64 = 1 << 63;
/// A node's or a frame's address inside an entry.
const PA_MASK: u64 = 0x0000_ffff_ffff_ffff;
/// A leaf entry keeps the segment kind above the address.
const KIND_SHIFT: u64 = 48;
/// The user half: 128 TiB of virtual address, 256 entries at the root.
const USER_VA_LIMIT: usize = 1 << 47;
const NO_INDEX_PAGE: &str = "mapping log: no page for an index node";

/// A proc's user mappings, by page.
#[derive(Debug)]
pub struct MappingLog {
    /// The root node's page address — 0 until the first record.
    root: u64,
    len: usize,
}

/// The entry index of `va` at `level` (0 is the root).
fn index(va: usize, level: usize) -> usize {
    (va >> (39 - 9 * level)) & 0x1ff
}

/// The page VA a fully taken path names.
fn va_of(taken: &[usize; LEVELS]) -> usize {
    (taken[0] << 39) | (taken[1] << 30) | (taken[2] << 21) | (taken[3] << 12)
}

/// The segment kinds' codes inside a record.
fn kind_code(kind: SegKind) -> u64 {
    match kind {
        SegKind::Text => 0,
        SegKind::Data => 1,
        SegKind::Bss => 2,
        SegKind::Stack => 3,
        SegKind::Tos => 4,
    }
}

fn kind_of(code: u64) -> SegKind {
    match code {
        0 => SegKind::Text,
        1 => SegKind::Data,
        2 => SegKind::Bss,
        3 => SegKind::Stack,
        _ => SegKind::Tos,
    }
}

fn record(entry: u64, va: usize) -> MappingRecord {
    MappingRecord { va, phys: (entry & PA_MASK) as usize, size: PAGE_SIZE_4K, kind: kind_of((entry >> KIND_SHIFT) & 7) }
}

/// Whether every entry of the node is zero.
fn node_is_empty(node: u64) -> bool {
    (0..ENTRIES).all(|i| table_entry(node, i) == 0)
}

/// Free a node and everything under it (at most `LEVELS` frames deep).
fn free_tree(node: u64, level: usize) {
    if level < LEVELS - 1 {
        for i in 0..ENTRIES {
            let entry = table_entry(node, i);
            if entry & PRESENT != 0 {
                free_tree(entry & PA_MASK, level + 1);
            }
        }
    }
    free_phys_page(node);
}

impl MappingLog {
    pub const fn new() -> MappingLog {
        MappingLog { root: 0, len: 0 }
    }

    /// The number of records.
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The record for `va`'s page, if the proc maps it.
    pub fn get(&self, va: usize) -> Option<MappingRecord> {
        let page = va & !(PAGE_SIZE_4K - 1);
        let (leaf, at) = self.path(page)?[LEVELS - 1];
        let entry = table_entry(leaf, at);
        (entry & PRESENT != 0).then(|| record(entry, page))
    }

    /// The node on the way to `va` and the entry it was reached
    /// through, level by level.  None when the path stops short of a
    /// leaf.
    fn path(&self, va: usize) -> Option<[(u64, usize); LEVELS]> {
        let mut path = [(0u64, 0usize); LEVELS];
        let mut node = self.root;
        if node == 0 {
            return None;
        }
        for (level, reached) in path.iter_mut().enumerate() {
            let at = index(va, level);
            *reached = (node, at);
            if level < LEVELS - 1 {
                let entry = table_entry(node, at);
                if entry & PRESENT == 0 {
                    return None;
                }
                node = entry & PA_MASK;
            }
        }
        Some(path)
    }

    /// Record that `va`'s page maps `phys` as `kind`, replacing any
    /// record the page already has.  Fails only when an index page
    /// cannot be had: the records are unchanged then (index pages the
    /// attempt already linked stay until the log is cleared).
    pub fn insert(&mut self, va: usize, phys: usize, kind: SegKind) -> Result<(), &'static str> {
        debug_assert!(va < USER_VA_LIMIT && va & (PAGE_SIZE_4K - 1) == 0, "mapping log: {va:#x} is not a user page");
        debug_assert!(phys < 1usize << 48, "mapping log: {phys:#x} does not fit a record");
        if self.root == 0 {
            self.root = alloc_phys_page().map_err(|_| NO_INDEX_PAGE)?;
        }
        let mut node = self.root;
        for level in 0..LEVELS - 1 {
            let at = index(va, level);
            let entry = table_entry(node, at);
            node = if entry & PRESENT != 0 {
                entry & PA_MASK
            } else {
                let child = alloc_phys_page().map_err(|_| NO_INDEX_PAGE)?;
                set_table_entry(node, at, child | PRESENT);
                child
            };
        }
        let at = index(va, LEVELS - 1);
        if table_entry(node, at) & PRESENT == 0 {
            self.len += 1;
        }
        let frame_pa = (phys as u64) & PA_MASK;
        set_table_entry(node, at, frame_pa | (kind_code(kind) << KIND_SHIFT) | PRESENT);
        Ok(())
    }

    /// Forget the record for `va`'s page; the record is returned.  An
    /// index page left with no records goes back to the allocator, up
    /// to the root.
    pub fn remove(&mut self, va: usize) -> Option<MappingRecord> {
        let page = va & !(PAGE_SIZE_4K - 1);
        let path = self.path(page)?;
        let (leaf, at) = path[LEVELS - 1];
        let entry = table_entry(leaf, at);
        if entry & PRESENT == 0 {
            return None;
        }
        set_table_entry(leaf, at, 0);
        self.len -= 1;
        let mut level = LEVELS - 1;
        let mut node = leaf;
        while node_is_empty(node) {
            if level == 0 {
                self.root = 0;
            } else {
                let (parent, from) = path[level - 1];
                set_table_entry(parent, from, 0);
            }
            free_phys_page(node);
            if level == 0 {
                break;
            }
            level -= 1;
            node = path[level].0;
        }
        Some(record(entry, page))
    }

    /// Drop every record for which `keep` returns false.  `keep` runs
    /// exactly once per record.
    pub fn retain(&mut self, mut keep: impl FnMut(&MappingRecord) -> bool) {
        let mut gone: Vec<usize> = Vec::new();
        for rec in self.iter() {
            if !keep(&rec) {
                gone.push(rec.va);
            }
        }
        for va in gone {
            self.remove(va);
        }
    }

    /// Forget every record; the index pages go back to the allocator.
    pub fn clear(&mut self) {
        if self.root != 0 {
            free_tree(self.root, 0);
            self.root = 0;
        }
        self.len = 0;
    }

    /// The records in address order.
    pub fn iter(&self) -> Iter<'_> {
        Iter { log: PhantomData, nodes: [(self.root, 0), (0, 0), (0, 0), (0, 0)], taken: [0; LEVELS] }
    }

}

/// The index pages held (tests).
#[cfg(test)]
fn count_nodes(node: u64, level: usize) -> usize {
    if node == 0 {
        return 0;
    }
    let mut n = 1;
    if level < LEVELS - 1 {
        for i in 0..ENTRIES {
            let entry = table_entry(node, i);
            if entry & PRESENT != 0 {
                n += count_nodes(entry & PA_MASK, level + 1);
            }
        }
    }
    n
}

impl Default for MappingLog {
    fn default() -> MappingLog {
        MappingLog::new()
    }
}

impl Drop for MappingLog {
    fn drop(&mut self) {
        self.clear();
    }
}

#[cfg(test)]
impl MappingLog {
    fn node_pages(&self) -> usize {
        count_nodes(self.root, 0)
    }
}

impl<'a> IntoIterator for &'a MappingLog {
    type Item = MappingRecord;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Iter<'a> {
        self.iter()
    }
}

/// A walk of the records in address order: at most four node pages are
/// on the walk at a time, and nothing is copied.
pub struct Iter<'a> {
    /// The nodes read belong to the log, which the walk borrows.
    log: PhantomData<&'a MappingLog>,
    /// Per level: the node's page (0 once popped) and the next entry to
    /// read.
    nodes: [(u64, usize); LEVELS],
    /// The entry taken at each level, for the record's VA.
    taken: [usize; LEVELS],
}

impl<'a> Iterator for Iter<'a> {
    type Item = MappingRecord;

    fn next(&mut self) -> Option<MappingRecord> {
        // The deepest node still on the walk.
        let mut level = (0..LEVELS).rev().find(|&l| self.nodes[l].0 != 0)?;
        loop {
            let (node, at) = self.nodes[level];
            if at == ENTRIES {
                // Exhausted: pop it.  Its parent's cursor is already
                // past the entry that led here.
                self.nodes[level] = (0, 0);
                if level == 0 {
                    return None;
                }
                level -= 1;
                continue;
            }
            self.nodes[level].1 += 1;
            let entry = table_entry(node, at);
            if entry & PRESENT == 0 {
                continue;
            }
            self.taken[level] = at;
            if level == LEVELS - 1 {
                return Some(record(entry, va_of(&self.taken)));
            }
            level += 1;
            self.nodes[level] = (entry & PA_MASK, 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const K: usize = PAGE_SIZE_4K;
    const TOP: usize = (1 << 47) - K;

    fn frame(n: usize) -> usize {
        0x1000 * n + 0x10_0000
    }

    #[test]
    fn an_empty_log_holds_no_page_and_answers_nothing() {
        let mut log = MappingLog::new();
        assert_eq!(log.len(), 0);
        assert!(log.is_empty());
        assert!(log.get(0x2000).is_none());
        assert!(log.remove(0x2000).is_none());
        assert_eq!(log.iter().count(), 0);
        assert_eq!(log.node_pages(), 0);
    }

    #[test]
    fn a_record_round_trips_with_every_segment_kind() {
        let kinds = [SegKind::Text, SegKind::Data, SegKind::Bss, SegKind::Stack, SegKind::Tos];
        for (i, kind) in kinds.iter().enumerate() {
            let mut log = MappingLog::new();
            let va = 0x20_0000 + i * K;
            log.insert(va, frame(i + 1), *kind).expect("insert");
            assert_eq!(log.len(), 1);
            assert_eq!(log.get(va), Some(MappingRecord { va, phys: frame(i + 1), size: K, kind: *kind }));
            assert_eq!(log.node_pages(), 4, "the root, two nodes and a leaf");
        }
    }

    #[test]
    fn lookup_rounds_down_to_the_page() {
        let mut log = MappingLog::new();
        log.insert(0x7fff_ffff_f000, 0x1234_5000, SegKind::Stack).expect("insert");
        assert_eq!(log.get(0x7fff_ffff_f123).map(|r| r.va), Some(0x7fff_ffff_f000));
        assert_eq!(log.get(0x7fff_ffff_f000).map(|r| r.phys), Some(0x1234_5000));
        assert!(log.get(0x7fff_ffff_e000).is_none(), "the page below it is not mapped");
        assert!(log.get(0).is_none());
    }

    #[test]
    fn records_come_back_in_address_order_across_leaves_and_nodes() {
        let mut log = MappingLog::new();
        // The root's first and last entries, two gibibytes apart, and
        // two leaves of one gibibyte.
        let scattered = [
            TOP,
            0x2000_0000 + 700 * K,
            0x2000_0000 + K,
            0x20_0000,
            0x2000_0000,
            0x1_0000_0000,
            0x7ffe_0000_0000 + 5 * K,
        ];
        for (i, &va) in scattered.iter().enumerate() {
            log.insert(va, frame(i + 1), SegKind::Bss).expect("insert");
        }
        let mut want = scattered;
        want.sort_unstable();
        let got: Vec<usize> = log.iter().map(|r| r.va).collect();
        assert_eq!(got, want);
        assert_eq!(log.iter().count(), scattered.len(), "a second walk starts over");
        assert_eq!(log.len(), scattered.len());
    }

    #[test]
    fn recording_the_same_page_again_replaces_its_record() {
        let mut log = MappingLog::new();
        log.insert(0x400_000, frame(1), SegKind::Data).expect("insert");
        log.insert(0x400_000, frame(2), SegKind::Bss).expect("insert");
        assert_eq!(log.len(), 1);
        assert_eq!(log.get(0x400_000).map(|r| (r.phys, r.kind)), Some((frame(2), SegKind::Bss)));
    }

    #[test]
    fn an_emptied_index_page_goes_back_while_its_siblings_stay() {
        let mut log = MappingLog::new();
        let a = 0x20_0000;
        let b = 0x20_0000 + 600 * K;
        let c = 0x1_0000_0000 + 9 * K;
        for (i, &va) in [a, b, c].iter().enumerate() {
            log.insert(va, frame(i + 1), SegKind::Bss).expect("insert");
        }
        // The root, the 512 GiB node, two gibibyte nodes, three leaves.
        assert_eq!(log.node_pages(), 7);
        assert_eq!(log.remove(b).map(|r| r.va), Some(b));
        assert_eq!(log.node_pages(), 6, "b's leaf is empty and goes back");
        assert!(log.get(b).is_none());
        assert_eq!(log.remove(c).map(|r| r.va), Some(c));
        assert_eq!(log.node_pages(), 4, "c's leaf and its gibibyte node go back");
        assert_eq!(log.remove(a).map(|r| r.va), Some(a));
        assert_eq!(log.node_pages(), 0, "nothing is left, not even the root");
        assert!(log.is_empty());
    }

    #[test]
    fn removing_an_absent_page_changes_nothing() {
        let mut log = MappingLog::new();
        log.insert(0x20_0000, frame(1), SegKind::Bss).expect("insert");
        assert!(log.remove(0x20_0000 + K).is_none());
        assert!(log.remove(0x1_0000_0000).is_none());
        assert_eq!(log.len(), 1);
        assert_eq!(log.node_pages(), 4);
    }

    #[test]
    fn retain_drops_exactly_the_records_that_fail() {
        let mut log = MappingLog::new();
        for i in 0..40 {
            log.insert(0x20_0000 + i * K, frame(i + 1), SegKind::Data).expect("insert");
        }
        let mut seen = 0;
        log.retain(|rec| {
            seen += 1;
            rec.phys % 3 == 0
        });
        assert_eq!(seen, 40, "the predicate runs once per record");
        let kept: Vec<usize> = log.iter().map(|r| r.phys).collect();
        assert_eq!(kept.len(), (1..=40).filter(|n| frame(*n) % 3 == 0).count());
        assert!(kept.iter().all(|&phys| phys % 3 == 0));
    }

    #[test]
    fn clear_and_take_return_every_index_page() {
        let mut log = MappingLog::new();
        log.insert(0x20_0000, frame(1), SegKind::Bss).expect("insert");
        log.insert(TOP, frame(2), SegKind::Stack).expect("insert");
        assert!(log.node_pages() > 4);
        let taken = core::mem::take(&mut log);
        assert!(log.is_empty());
        assert_eq!(log.node_pages(), 0);
        assert_eq!(taken.len(), 2);
        drop(taken);
        log.insert(0x40_0000, frame(3), SegKind::Data).expect("a taken log takes records again");
        log.clear();
        assert_eq!(log.node_pages(), 0);
    }

    #[test]
    fn contiguous_pages_cost_one_leaf_per_512() {
        let pages = 3000;
        let mut log = MappingLog::new();
        for i in 0..pages {
            log.insert(0x20_0000 + i * K, frame(i + 1), SegKind::Bss).expect("insert");
        }
        // The root, two nodes, and six leaves (3000 pages span six).
        assert_eq!(log.node_pages(), 9);
        assert_eq!(log.len(), pages);
        let all: Vec<MappingRecord> = log.iter().collect();
        assert_eq!(all.len(), pages);
        for (i, rec) in all.iter().enumerate() {
            assert_eq!(rec.va, 0x20_0000 + i * K);
            assert_eq!(rec.phys, frame(i + 1));
        }
    }
}
