//! roaring — the hand-written Roaring bitmap (array, bitmap and run
//! containers) with its differential tests against a `BTreeSet` model and
//! the crates.io `roaring` test oracle (Phase 3 CONTEXT D-19).
//!
//! A bitmap is a sorted vector of chunks keyed by the high 16 bits of the
//! value; each chunk holds exactly one container over the low 16 bits:
//!
//! * `Array` — a sorted `Vec<u16>` holding at most [`ARRAY_MAX`] values;
//! * `Bitmap` — a boxed `[u64; 1024]` (65,536 bits) plus its cardinality;
//! * `Run` — sorted `(start, length)` pairs, where a pair covers
//!   `start ..= start + length` and `length` counts the additional values
//!   (Lemire et al., arXiv:1603.06549).
//!
//! The conversion sites are fixed so results are deterministic: an array
//! becomes a bitmap when an insert would take its cardinality past
//! [`ARRAY_MAX`]; a bitmap becomes an array when a removal brings it down to
//! [`ARRAY_MAX`]; [`RoaringBitmap::run_optimize`] rewrites each container in
//! the smallest of the three forms; and the crate-visible `insert_compact`
//! applies that same smallest-form rule to the one container it touched.
//! Set operations are not conversion sites: they build new containers (array
//! at or below [`ARRAY_MAX`], else bitmap) and an empty result container is
//! dropped with its chunk.
//!
//! The four hot loops live in the private [`crate::setops`] module and the
//! containers call only those functions, so a Phase 5 SIMD module can replace
//! the bodies with no caller change (D-20).  No SIMD code lands here in this
//! phase; the SIMD module is mounted at the crate root because a forbid
//! reaches submodules.

use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use core::ops::RangeInclusive;

use crate::setops;

/// The cardinality at which an array container becomes a bitmap container.
pub const ARRAY_MAX: usize = 4096;

const CHUNK_BITS: u32 = 16;
const CHUNK_LOW_MASK: u32 = 0xFFFF;
const BITMAP_BITS: usize = 65_536;
const BITMAP_BYTES: usize = 8_192;

#[derive(Clone)]
struct Chunk {
    key: u16,
    container: Container,
}

#[derive(Clone)]
enum Container {
    Array(Vec<u16>),
    Bitmap(Box<[u64; 1024]>, u32),
    Run(Vec<(u16, u16)>),
}

/// A hand-written Roaring bitmap over the full `u32` range.
#[derive(Clone)]
pub struct RoaringBitmap {
    chunks: Vec<Chunk>,
}

impl RoaringBitmap {
    /// An empty bitmap.
    pub const fn new() -> Self {
        Self { chunks: Vec::new() }
    }

    /// Inserts `v`; returns whether it was absent.
    pub fn insert(&mut self, v: u32) -> bool {
        self.insert_inner(v)
    }

    fn insert_inner(&mut self, v: u32) -> bool {
        let key = (v >> CHUNK_BITS) as u16;
        let low = (v & CHUNK_LOW_MASK) as u16;
        let at = self.chunks.partition_point(|chunk| chunk.key < key);
        if at < self.chunks.len() && self.chunks[at].key == key {
            let container = &mut self.chunks[at].container;
            let inserted = container.insert_low(low);
            if inserted {
                container.maybe_upgrade_array();
            }
            inserted
        } else {
            self.chunks.insert(
                at,
                Chunk {
                    key,
                    container: Container::Array(vec![low]),
                },
            );
            true
        }
    }

    /// Removes `v`; returns whether it was present.
    pub fn remove(&mut self, v: u32) -> bool {
        let key = (v >> CHUNK_BITS) as u16;
        let low = (v & CHUNK_LOW_MASK) as u16;
        let at = self.chunks.partition_point(|chunk| chunk.key < key);
        if at >= self.chunks.len() || self.chunks[at].key != key {
            return false;
        }
        if !self.chunks[at].container.remove_low(low) {
            return false;
        }
        if self.chunks[at].container.is_empty() {
            self.chunks.remove(at);
        } else {
            self.chunks[at].container.maybe_downgrade_bitmap();
        }
        true
    }

    /// Whether `v` is present.
    pub fn contains(&self, v: u32) -> bool {
        let key = (v >> CHUNK_BITS) as u16;
        let low = (v & CHUNK_LOW_MASK) as u16;
        let at = self.chunks.partition_point(|chunk| chunk.key < key);
        at < self.chunks.len() && self.chunks[at].key == key && self.chunks[at].container.contains_low(low)
    }

    /// The number of distinct values.
    pub fn len(&self) -> u64 {
        self.chunks.iter().map(|chunk| chunk.container.cardinality() as u64).sum()
    }

    /// Whether the bitmap holds no values.
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    /// Ascending iteration over the values.
    pub fn iter(&self) -> Iter<'_> {
        Iter {
            chunks: self.chunks.iter(),
            current: None,
        }
    }

    /// Inserts every value in `range`.
    pub fn insert_range(&mut self, range: RangeInclusive<u32>) {
        for value in range {
            self.insert(value);
        }
    }

    /// Rewrites every container in the smallest of the three forms.
    pub fn run_optimize(&mut self) {
        for chunk in &mut self.chunks {
            chunk.container.compact();
        }
    }

    /// Union with `other`.
    pub fn union(&self, other: &Self) -> Self {
        self.combine(other, SetOp::Union)
    }

    /// Intersection with `other`.
    pub fn intersection(&self, other: &Self) -> Self {
        self.combine(other, SetOp::Intersection)
    }

    /// Difference `self \ other`.
    pub fn difference(&self, other: &Self) -> Self {
        self.combine(other, SetOp::Difference)
    }

    /// In place: `self` becomes `self \ other`.
    pub fn andnot(&mut self, other: &Self) {
        *self = self.difference(other);
    }

    /// Chunk-wise merge of the two sorted chunk lists.
    fn combine(&self, other: &Self, op: SetOp) -> Self {
        let mut chunks = Vec::new();
        let mut left = 0usize;
        let mut right = 0usize;
        while left < self.chunks.len() || right < other.chunks.len() {
            let key = match (self.chunks.get(left), other.chunks.get(right)) {
                (Some(a), Some(b)) => a.key.min(b.key),
                (Some(a), None) => a.key,
                (None, Some(b)) => b.key,
                (None, None) => break,
            };
            let a = if self.chunks.get(left).is_some_and(|chunk| chunk.key == key) {
                let chunk = &self.chunks[left];
                left += 1;
                Some(chunk)
            } else {
                None
            };
            let b = if other.chunks.get(right).is_some_and(|chunk| chunk.key == key) {
                let chunk = &other.chunks[right];
                right += 1;
                Some(chunk)
            } else {
                None
            };
            let container = match (a, b) {
                (Some(a), Some(b)) => combine_containers(&a.container, &b.container, op),
                (Some(a), None) => match op {
                    SetOp::Union | SetOp::Difference => Some(a.container.clone()),
                    SetOp::Intersection => None,
                },
                (None, Some(b)) => match op {
                    SetOp::Union => Some(b.container.clone()),
                    SetOp::Intersection | SetOp::Difference => None,
                },
                (None, None) => None,
            };
            if let Some(container) = container {
                chunks.push(Chunk { key, container });
            }
        }
        Self { chunks }
    }
}

/// The three results a pair of containers can produce.
#[derive(Clone, Copy, PartialEq, Eq)]
enum SetOp {
    Union,
    Intersection,
    Difference,
}

fn combine_containers(a: &Container, b: &Container, op: SetOp) -> Option<Container> {
    match (a, b) {
        (Container::Bitmap(a_words, _), Container::Bitmap(b_words, _)) => {
            let mut out = zero_words();
            let card = match op {
                SetOp::Union => setops::bitmap_or(a_words, b_words, &mut out),
                SetOp::Intersection => setops::bitmap_and(a_words, b_words, &mut out),
                SetOp::Difference => setops::bitmap_andnot(a_words, b_words, &mut out),
            };
            Container::from_words(card, out)
        }
        (Container::Array(a_values), Container::Array(b_values)) => match op {
            SetOp::Union => array_union(a_values, b_values),
            SetOp::Intersection => {
                let mut out = Vec::new();
                setops::array_intersect(a_values, b_values, &mut out);
                if out.is_empty() {
                    None
                } else {
                    Some(Container::Array(out))
                }
            }
            SetOp::Difference => array_difference(a_values, b_values),
        },
        (Container::Array(values), other) => combine_array_with(values, other, true, op),
        (other, Container::Array(values)) => combine_array_with(values, other, false, op),
        _ => {
            let a_words = a.to_words();
            let b_words = b.to_words();
            let mut out = zero_words();
            let card = match op {
                SetOp::Union => setops::bitmap_or(&a_words, &b_words, &mut out),
                SetOp::Intersection => setops::bitmap_and(&a_words, &b_words, &mut out),
                SetOp::Difference => setops::bitmap_andnot(&a_words, &b_words, &mut out),
            };
            Container::from_words(card, out)
        }
    }
}

/// A mixed pairing: the sorted array probes or paints the other container,
/// so no word loop over two bitmap containers is written here.
fn combine_array_with(values: &[u16], other: &Container, array_is_left: bool, op: SetOp) -> Option<Container> {
    match op {
        SetOp::Union => {
            let mut words = other.to_words();
            let mut card = other.cardinality();
            for &value in values {
                let word = value as usize / 64;
                let mask = 1u64 << (value as usize % 64);
                if words[word] & mask == 0 {
                    words[word] |= mask;
                    card += 1;
                }
            }
            Container::from_words(card, words)
        }
        SetOp::Intersection => {
            let mut out = Vec::with_capacity(values.len());
            for &value in values {
                if other.contains_low(value) {
                    out.push(value);
                }
            }
            if out.is_empty() {
                None
            } else {
                Some(Container::Array(out))
            }
        }
        SetOp::Difference => {
            if array_is_left {
                let mut out = Vec::with_capacity(values.len());
                for &value in values {
                    if !other.contains_low(value) {
                        out.push(value);
                    }
                }
                if out.is_empty() {
                    None
                } else {
                    Some(Container::Array(out))
                }
            } else {
                let mut words = other.to_words();
                let card = other.cardinality();
                let mut removed = 0u32;
                for &value in values {
                    let word = value as usize / 64;
                    let mask = 1u64 << (value as usize % 64);
                    if words[word] & mask != 0 {
                        words[word] &= !mask;
                        removed += 1;
                    }
                }
                Container::from_words(card - removed, words)
            }
        }
    }
}

fn array_union(a: &[u16], b: &[u16]) -> Option<Container> {
    let mut merged: Vec<u16> = Vec::with_capacity(a.len() + b.len());
    let mut i = 0usize;
    let mut j = 0usize;
    while i < a.len() || j < b.len() {
        let value = if j >= b.len() || (i < a.len() && a[i] < b[j]) {
            let value = a[i];
            i += 1;
            value
        } else if i >= a.len() || b[j] < a[i] {
            let value = b[j];
            j += 1;
            value
        } else {
            let value = a[i];
            i += 1;
            j += 1;
            value
        };
        merged.push(value);
    }
    if merged.is_empty() {
        return None;
    }
    if merged.len() <= ARRAY_MAX {
        Some(Container::Array(merged))
    } else {
        let card = merged.len() as u32;
        let mut words = zero_words();
        for value in merged {
            words[value as usize / 64] |= 1u64 << (value as usize % 64);
        }
        Some(Container::Bitmap(words, card))
    }
}

fn array_difference(a: &[u16], b: &[u16]) -> Option<Container> {
    let mut out: Vec<u16> = Vec::with_capacity(a.len());
    let mut j = 0usize;
    for &value in a {
        while j < b.len() && b[j] < value {
            j += 1;
        }
        if j < b.len() && b[j] == value {
            continue;
        }
        out.push(value);
    }
    if out.is_empty() {
        None
    } else {
        Some(Container::Array(out))
    }
}

impl Default for RoaringBitmap {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for RoaringBitmap {
    /// Equality is by contents: the same values in different container kinds
    /// are equal.
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len() && self.iter().eq(other.iter())
    }
}

impl Eq for RoaringBitmap {}

impl core::fmt::Debug for RoaringBitmap {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "RoaringBitmap {{ len: {} }}", self.len())
    }
}

/// Ascending iterator over a [`RoaringBitmap`].
pub struct Iter<'a> {
    chunks: core::slice::Iter<'a, Chunk>,
    current: Option<(u16, ContainerValues<'a>)>,
}

impl Iterator for Iter<'_> {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        loop {
            if let Some((key, values)) = self.current.as_mut() {
                if let Some(low) = values.next() {
                    return Some(((*key as u32) << CHUNK_BITS) | low as u32);
                }
                self.current = None;
            }
            let chunk = self.chunks.next()?;
            self.current = Some((chunk.key, ContainerValues::new(chunk)));
        }
    }
}

enum ContainerValues<'a> {
    Array(core::slice::Iter<'a, u16>),
    Bitmap {
        words: &'a [u64; 1024],
        index: usize,
        word: u64,
    },
    Run {
        runs: core::slice::Iter<'a, (u16, u16)>,
        next: u32,
        end: u32,
    },
}

impl<'a> ContainerValues<'a> {
    fn new(chunk: &'a Chunk) -> Self {
        match &chunk.container {
            Container::Array(values) => Self::Array(values.iter()),
            Container::Bitmap(words, _) => Self::Bitmap {
                words,
                index: 0,
                word: 0,
            },
            Container::Run(runs) => Self::Run {
                runs: runs.iter(),
                next: 1,
                end: 0,
            },
        }
    }
}

impl Iterator for ContainerValues<'_> {
    type Item = u16;

    fn next(&mut self) -> Option<u16> {
        match self {
            Self::Array(values) => values.next().copied(),
            Self::Bitmap { words, index, word } => loop {
                if *word != 0 {
                    let bit = word.trailing_zeros() as usize;
                    *word &= *word - 1;
                    return Some(((*index - 1) * 64 + bit) as u16);
                }
                if *index >= words.len() {
                    return None;
                }
                *word = words[*index];
                *index += 1;
            },
            Self::Run { runs, next, end } => loop {
                if *next <= *end {
                    let value = *next;
                    *next += 1;
                    return Some(value as u16);
                }
                let (start, length) = runs.next()?;
                *next = *start as u32;
                *end = *start as u32 + *length as u32;
            },
        }
    }
}

impl Container {
    fn cardinality(&self) -> u32 {
        match self {
            Self::Array(values) => values.len() as u32,
            Self::Bitmap(_, card) => *card,
            Self::Run(runs) => runs.iter().map(|&(_, length)| length as u32 + 1).sum(),
        }
    }

    fn is_empty(&self) -> bool {
        self.cardinality() == 0
    }

    fn contains_low(&self, low: u16) -> bool {
        match self {
            Self::Array(values) => values.binary_search(&low).is_ok(),
            Self::Bitmap(words, _) => words[low as usize / 64] & (1u64 << (low as usize % 64)) != 0,
            Self::Run(runs) => {
                let at = runs.partition_point(|&(start, _)| start <= low);
                if at == 0 {
                    return false;
                }
                let (start, length) = runs[at - 1];
                (low as u32) <= start as u32 + length as u32
            }
        }
    }

    fn insert_low(&mut self, low: u16) -> bool {
        match self {
            Self::Array(values) => match values.binary_search(&low) {
                Ok(_) => false,
                Err(at) => {
                    values.insert(at, low);
                    true
                }
            },
            Self::Bitmap(words, card) => {
                let word = low as usize / 64;
                let mask = 1u64 << (low as usize % 64);
                if words[word] & mask != 0 {
                    false
                } else {
                    words[word] |= mask;
                    *card += 1;
                    true
                }
            }
            Self::Run(runs) => run_insert(runs, low),
        }
    }

    fn remove_low(&mut self, low: u16) -> bool {
        match self {
            Self::Array(values) => match values.binary_search(&low) {
                Ok(at) => {
                    values.remove(at);
                    true
                }
                Err(_) => false,
            },
            Self::Bitmap(words, card) => {
                let word = low as usize / 64;
                let mask = 1u64 << (low as usize % 64);
                if words[word] & mask == 0 {
                    false
                } else {
                    words[word] &= !mask;
                    *card -= 1;
                    true
                }
            }
            Self::Run(runs) => run_remove(runs, low),
        }
    }

    /// Conversion site (1): an array past [`ARRAY_MAX`] becomes a bitmap.
    fn maybe_upgrade_array(&mut self) {
        let card = match self {
            Self::Array(values) => values.len(),
            _ => return,
        };
        if card <= ARRAY_MAX {
            return;
        }
        let mut words = zero_words();
        if let Self::Array(values) = self {
            for &value in values.iter() {
                words[value as usize / 64] |= 1u64 << (value as usize % 64);
            }
        }
        *self = Self::Bitmap(words, card as u32);
    }

    /// Conversion site (2): a bitmap at [`ARRAY_MAX`] becomes an array.
    fn maybe_downgrade_bitmap(&mut self) {
        let card = match self {
            Self::Bitmap(_, card) => *card,
            _ => return,
        };
        if card as usize != ARRAY_MAX {
            return;
        }
        let values = if let Self::Bitmap(words, _) = self {
            array_of_words(words)
        } else {
            Vec::new()
        };
        *self = Self::Array(values);
    }

    /// Conversion site (3): rewrite this container in its smallest form.
    fn compact(&mut self) {
        let previous = core::mem::replace(self, Self::Array(Vec::new()));
        *self = previous.smallest_form();
    }

    /// The smallest of the three forms, by payload bytes.  Ties prefer the
    /// array, then the run, then the bitmap.
    fn smallest_form(self) -> Self {
        let card = self.cardinality() as usize;
        if card == 0 {
            return self;
        }
        let runs = self.runs();
        let run_bytes = runs.len() * 4;
        let array_bytes = card * 2;
        if card <= ARRAY_MAX && array_bytes <= run_bytes && array_bytes <= BITMAP_BYTES {
            Self::Array(self.into_array())
        } else if run_bytes <= BITMAP_BYTES {
            Self::Run(runs)
        } else {
            match self {
                Self::Bitmap(words, card) => Self::Bitmap(words, card),
                other => Self::Bitmap(other.to_words(), card as u32),
            }
        }
    }

    /// The run-compressed form, ascending.
    fn runs(&self) -> Vec<(u16, u16)> {
        match self {
            Self::Run(runs) => runs.clone(),
            Self::Array(values) => runs_of_values(values.iter().copied()),
            Self::Bitmap(words, _) => runs_of_values(BitmapValues::new(words)),
        }
    }

    fn into_array(self) -> Vec<u16> {
        match self {
            Self::Array(values) => values,
            Self::Bitmap(words, _) => array_of_words(&words),
            Self::Run(runs) => {
                let capacity = runs.iter().map(|&(_, length)| length as usize + 1).sum();
                let mut values = Vec::with_capacity(capacity);
                for (start, length) in runs {
                    for value in start as u32..=start as u32 + length as u32 {
                        values.push(value as u16);
                    }
                }
                values
            }
        }
    }

    /// This container as a bitmap container's words (the mixed-pairing path).
    fn to_words(&self) -> Box<[u64; 1024]> {
        match self {
            Self::Bitmap(words, _) => words.clone(),
            Self::Array(values) => {
                let mut words = zero_words();
                for &value in values.iter() {
                    words[value as usize / 64] |= 1u64 << (value as usize % 64);
                }
                words
            }
            Self::Run(runs) => {
                let mut words = zero_words();
                for &(start, length) in runs.iter() {
                    for value in start as u32..=start as u32 + length as u32 {
                        words[value as usize / 64] |= 1u64 << (value as usize % 64);
                    }
                }
                words
            }
        }
    }

    /// A result container from words: array at or below [`ARRAY_MAX`], else
    /// bitmap; an empty result yields `None` so no empty container survives.
    fn from_words(card: u32, words: Box<[u64; 1024]>) -> Option<Self> {
        if card == 0 {
            return None;
        }
        if card as usize <= ARRAY_MAX {
            Some(Self::Array(array_of_words(&words)))
        } else {
            Some(Self::Bitmap(words, card))
        }
    }
}

/// A boxed all-zero bitmap container, built without an 8 KiB stack copy.
fn zero_words() -> Box<[u64; 1024]> {
    let words: Box<[u64]> = vec![0u64; 1024].into_boxed_slice();
    match words.try_into() {
        Ok(words) => words,
        Err(_) => unreachable!("a 1024-word slice converts to a boxed 1024-word array"),
    }
}

fn array_of_words(words: &[u64; 1024]) -> Vec<u16> {
    let mut values = Vec::new();
    for (index, &word) in words.iter().enumerate() {
        let mut remaining = word;
        while remaining != 0 {
            let bit = remaining.trailing_zeros() as usize;
            values.push((index * 64 + bit) as u16);
            remaining &= remaining - 1;
        }
    }
    values
}

/// Ascending iteration over a bitmap container's set bits.
struct BitmapValues<'a> {
    words: &'a [u64; 1024],
    index: usize,
    word: u64,
}

impl<'a> BitmapValues<'a> {
    fn new(words: &'a [u64; 1024]) -> Self {
        Self {
            words,
            index: 0,
            word: 0,
        }
    }
}

impl Iterator for BitmapValues<'_> {
    type Item = u16;

    fn next(&mut self) -> Option<u16> {
        loop {
            if self.word != 0 {
                let bit = self.word.trailing_zeros() as usize;
                self.word &= self.word - 1;
                return Some(((self.index - 1) * 64 + bit) as u16);
            }
            if self.index >= BITMAP_BITS / 64 {
                return None;
            }
            self.word = self.words[self.index];
            self.index += 1;
        }
    }
}

/// The ascending run-compressed form of an ascending value stream.
fn runs_of_values<I: Iterator<Item = u16>>(values: I) -> Vec<(u16, u16)> {
    let mut runs: Vec<(u16, u16)> = Vec::new();
    let mut current: Option<(u16, u16)> = None;
    for value in values {
        match current {
            None => current = Some((value, value)),
            Some((start, last)) => {
                if last as u32 + 1 == value as u32 {
                    current = Some((start, value));
                } else {
                    runs.push((start, last - start));
                    current = Some((value, value));
                }
            }
        }
    }
    if let Some((start, last)) = current {
        runs.push((start, last - start));
    }
    runs
}

/// Inserts `low` into an ascending, non-adjacent run list, extending or
/// merging runs as needed.
fn run_insert(runs: &mut Vec<(u16, u16)>, low: u16) -> bool {
    let value = low as u32;
    let at = runs.partition_point(|&(start, _)| (start as u32) <= value);
    if at > 0 {
        let (start, length) = runs[at - 1];
        let end = start as u32 + length as u32;
        if value <= end {
            return false;
        }
        if value == end + 1 {
            let next_starts_after_value = runs.get(at).is_some_and(|&(next, _)| next as u32 == value + 1);
            if next_starts_after_value {
                let (next_start, next_length) = runs[at];
                let merged_end = next_start as u32 + next_length as u32;
                runs[at - 1] = (start, (merged_end - start as u32) as u16);
                runs.remove(at);
            } else {
                runs[at - 1] = (start, (value - start as u32) as u16);
            }
            return true;
        }
    }
    if let Some(&(start, length)) = runs.get(at)
        && start as u32 == value + 1
    {
        runs[at] = (low, (start as u32 + length as u32 - value) as u16);
        return true;
    }
    runs.insert(at, (low, 0));
    true
}

/// Removes `low` from an ascending, non-adjacent run list, shrinking or
/// splitting a run as needed.
fn run_remove(runs: &mut Vec<(u16, u16)>, low: u16) -> bool {
    let value = low as u32;
    let at = runs.partition_point(|&(start, _)| (start as u32) <= value);
    if at == 0 {
        return false;
    }
    let (start, length) = runs[at - 1];
    let start = start as u32;
    let end = start + length as u32;
    if value > end {
        return false;
    }
    if length == 0 {
        runs.remove(at - 1);
    } else if value == start {
        runs[at - 1] = ((start + 1) as u16, length - 1);
    } else if value == end {
        runs[at - 1] = (start as u16, length - 1);
    } else {
        runs[at - 1] = (start as u16, (value - start - 1) as u16);
        runs.insert(at, ((value + 1) as u16, (end - value - 1) as u16));
    }
    true
}

/// The container kind a chunk currently holds (test inspection only).
#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContainerKind {
    Array,
    Bitmap,
    Run,
}

#[cfg(test)]
impl RoaringBitmap {
    /// The kind of each chunk's container, in chunk order.
    pub(crate) fn container_kinds(&self) -> Vec<ContainerKind> {
        self.chunks
            .iter()
            .map(|chunk| match chunk.container {
                Container::Array(_) => ContainerKind::Array,
                Container::Bitmap(_, _) => ContainerKind::Bitmap,
                Container::Run(_) => ContainerKind::Run,
            })
            .collect()
    }

    /// The number of runs in each chunk's container, in chunk order.
    pub(crate) fn run_counts(&self) -> Vec<usize> {
        self.chunks
            .iter()
            .map(|chunk| match &chunk.container {
                Container::Array(_) | Container::Bitmap(_, _) => chunk.container.runs().len(),
                Container::Run(runs) => runs.len(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// Eight literal seeds: every randomized case reproduces from its seed.
    const SEEDS: [u64; 8] = [
        0x0000_0000_0000_0001,
        0x5DEE_CE66_D1CE_4B1D,
        0x1234_5678_9ABC_DEF0,
        0xDEAD_BEEF_CAFE_F00D,
        0x0F1E_2D3C_4B5A_6978,
        0xA5A5_5A5A_1234_4321,
        0xFFFF_FFFF_0000_0000,
        0x9E37_79B9_7F4A_7C15,
    ];

    /// SplitMix64: a seeded generator with no dependency (RESEARCH
    /// "Don't Hand-Roll": no `rand`, no `proptest`).
    struct SplitMix64 {
        state: u64,
    }

    impl SplitMix64 {
        fn new(seed: u64) -> Self {
            Self { state: seed }
        }

        fn next_u64(&mut self) -> u64 {
            self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = self.state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        }
    }

    /// The five value distributions of Task 1.
    fn value_for(rng: &mut SplitMix64, which: usize) -> u32 {
        match which {
            0 => rng.next_u64() as u32,
            1 => (7u32 << 16) | (rng.next_u64() as u32 & CHUNK_LOW_MASK),
            2 => rng.next_u64() as u32,
            3 => {
                const BOUNDARY: [u32; 10] =
                    [0, 1, 2, 65_534, 65_535, 65_536, 65_537, 131_071, u32::MAX - 1, u32::MAX];
                BOUNDARY[(rng.next_u64() % BOUNDARY.len() as u64) as usize]
            }
            _ => {
                if rng.next_u64() & 1 == 0 {
                    rng.next_u64() as u32
                } else {
                    (3u32 << 16) | (rng.next_u64() as u32 & CHUNK_LOW_MASK)
                }
            }
        }
    }

    #[test]
    fn membership_matches_models() {
        for seed in SEEDS {
            for which in 0..5usize {
                let mut rng = SplitMix64::new(seed ^ (0x9E37_79B9_7F4A_7C15u64.wrapping_mul(which as u64 + 1)));
                let mut bitmap = RoaringBitmap::new();
                let mut model: BTreeSet<u32> = BTreeSet::new();
                let mut oracle = ::roaring::RoaringBitmap::new();
                let steps = 400 + which * 25;
                for step in 0..steps {
                    let value = value_for(&mut rng, which);
                    if !rng.next_u64().is_multiple_of(5) {
                        assert_eq!(
                            bitmap.insert(value),
                            model.insert(value),
                            "insert disagrees with the BTreeSet model (seed {seed:#018x}, distribution {which}, step {step})"
                        );
                        oracle.insert(value);
                    } else {
                        assert_eq!(
                            bitmap.remove(value),
                            model.remove(&value),
                            "remove disagrees with the BTreeSet model (seed {seed:#018x}, distribution {which}, step {step})"
                        );
                        oracle.remove(value);
                    }
                    assert_eq!(
                        bitmap.contains(value),
                        model.contains(&value),
                        "contains disagrees with the BTreeSet model (seed {seed:#018x}, distribution {which}, step {step})"
                    );
                    assert_eq!(
                        bitmap.len(),
                        model.len() as u64,
                        "len disagrees with the BTreeSet model (seed {seed:#018x}, distribution {which}, step {step})"
                    );
                    assert_eq!(
                        bitmap.len(),
                        oracle.len(),
                        "len disagrees with the roaring oracle (seed {seed:#018x}, distribution {which}, step {step})"
                    );
                    if step % 16 == 0 {
                        let ours: Vec<u32> = bitmap.iter().collect();
                        let expected: Vec<u32> = model.iter().copied().collect();
                        let oracle_values: Vec<u32> = oracle.iter().collect();
                        assert_eq!(
                            ours, expected,
                            "ascending iteration disagrees with the BTreeSet model (seed {seed:#018x}, distribution {which}, step {step})"
                        );
                        assert_eq!(
                            ours, oracle_values,
                            "ascending iteration disagrees with the roaring oracle (seed {seed:#018x}, distribution {which}, step {step})"
                        );
                    }
                }
                assert_eq!(bitmap.is_empty(), model.is_empty(), "is_empty (seed {seed:#018x}, distribution {which})");
            }
        }
    }

    /// A base set from one of the five distributions, reused across pairings
    /// so each seed's sets are built once.
    fn distribution(seed: u64, which: usize) -> RoaringBitmap {
        let mut rng = SplitMix64::new(seed ^ (which as u64).wrapping_mul(0xA24B_AED4_963E_E407));
        let mut bitmap = RoaringBitmap::new();
        match which {
            0 => {
                for _ in 0..1500 {
                    bitmap.insert(rng.next_u64() as u32);
                }
            }
            1 => {
                let base = 7u32 << 16;
                for _ in 0..5000 {
                    bitmap.insert(base | (rng.next_u64() as u32 & CHUNK_LOW_MASK));
                }
                bitmap.insert_range(base + 1000..=base + 2000);
            }
            2 => {
                for _ in 0..40 {
                    let start = rng.next_u64() as u32;
                    let length = (rng.next_u64() % 300) as u32;
                    bitmap.insert_range(start..=start.saturating_add(length));
                }
            }
            3 => {
                for value in [
                    0u32,
                    1,
                    2,
                    65_534,
                    65_535,
                    65_536,
                    65_537,
                    131_071,
                    131_072,
                    u32::MAX - 1,
                    u32::MAX,
                ] {
                    bitmap.insert(value);
                }
            }
            _ => {
                for _ in 0..400 {
                    bitmap.insert(rng.next_u64() as u32);
                }
                bitmap.insert_range((5u32 << 16)..=((5u32 << 16) + 9000));
                let base = 9u32 << 16;
                for _ in 0..3000 {
                    bitmap.insert(base | (rng.next_u64() as u32 & CHUNK_LOW_MASK));
                }
                bitmap.insert(0);
                bitmap.insert(u32::MAX);
            }
        }
        bitmap
    }

    fn prepared(base: &RoaringBitmap, optimize: bool) -> RoaringBitmap {
        let mut copy = base.clone();
        if optimize {
            copy.run_optimize();
        }
        copy
    }

    fn collected(bitmap: &RoaringBitmap) -> Vec<u32> {
        bitmap.iter().collect()
    }

    fn check_operation(
        label: &str,
        name: &str,
        ours: &RoaringBitmap,
        model: &BTreeSet<u32>,
        oracle: &::roaring::RoaringBitmap,
    ) {
        let our_values = collected(ours);
        let model_values: Vec<u32> = model.iter().copied().collect();
        let oracle_values: Vec<u32> = oracle.iter().collect();
        assert_same_contents(&our_values, &model_values, label, name, "the BTreeSet model");
        assert_eq!(
            ours.len(),
            model_values.len() as u64,
            "{name} len vs the BTreeSet model ({label})"
        );
        assert_same_contents(&our_values, &oracle_values, label, name, "the roaring oracle");
        assert_eq!(ours.len(), oracle.len(), "{name} len vs the roaring oracle ({label})");
    }

    /// Reports the first divergence instead of printing whole sets, so a
    /// failing differential run stays readable and the seed identifies it.
    fn assert_same_contents(ours: &[u32], reference: &[u32], label: &str, name: &str, against: &str) {
        if ours == reference {
            return;
        }
        let at = ours
            .iter()
            .zip(reference.iter())
            .position(|(left, right)| left != right)
            .unwrap_or(ours.len().min(reference.len()));
        panic!(
            "{name} contents differ from {against} at index {at}: ours {:?} vs {:?} (lengths {} and {}) ({label})",
            ours.get(at),
            reference.get(at),
            ours.len(),
            reference.len()
        );
    }

    #[test]
    fn set_algebra_matches_models() {
        for seed in SEEDS {
            let bases: Vec<RoaringBitmap> = (0..5).map(|which| distribution(seed, which)).collect();
            let variants: Vec<RoaringBitmap> = (0..5)
                .flat_map(|which| [prepared(&bases[which], false), prepared(&bases[which], true)])
                .collect();
            let models: Vec<BTreeSet<u32>> = variants.iter().map(|bitmap| bitmap.iter().collect()).collect();
            let oracles: Vec<::roaring::RoaringBitmap> =
                variants.iter().map(|bitmap| bitmap.iter().collect()).collect();
            for left in 0..5usize {
                for right in 0..5usize {
                    for variant in 0..4usize {
                        let left_index = left * 2 + (variant & 1);
                        let right_index = right * 2 + ((variant >> 1) & 1);
                        let a = &variants[left_index];
                        let b = &variants[right_index];
                        let model_a = &models[left_index];
                        let model_b = &models[right_index];
                        let oracle_a = &oracles[left_index];
                        let oracle_b = &oracles[right_index];
                        let label = format!(
                            "seed {seed:#018x}, A=distribution {left}, B=distribution {right}, run_optimize variant {variant}"
                        );

                        let union = a.union(b);
                        let model_union: BTreeSet<u32> = model_a.union(model_b).copied().collect();
                        let oracle_union = oracle_a | oracle_b;
                        check_operation(&label, "union", &union, &model_union, &oracle_union);

                        let intersection = a.intersection(b);
                        let model_intersection: BTreeSet<u32> =
                            model_a.intersection(model_b).copied().collect();
                        let oracle_intersection = oracle_a & oracle_b;
                        check_operation(&label, "intersection", &intersection, &model_intersection, &oracle_intersection);

                        let difference = a.difference(b);
                        let model_difference: BTreeSet<u32> =
                            model_a.difference(model_b).copied().collect();
                        let oracle_difference = oracle_a - oracle_b;
                        check_operation(&label, "difference", &difference, &model_difference, &oracle_difference);

                        let mut andnot = a.clone();
                        andnot.andnot(b);
                        let mut oracle_andnot = oracle_a.clone();
                        oracle_andnot -= oracle_b;
                        check_operation(&label, "andnot", &andnot, &model_difference, &oracle_andnot);
                        assert_eq!(andnot, difference, "andnot and difference agree ({label})");
                    }
                }
            }
        }
    }

    #[test]
    fn set_algebra_with_empty() {
        let empty = RoaringBitmap::new();
        let mut nonempty = RoaringBitmap::new();
        nonempty.insert_range(10..=20);
        nonempty.insert(0);
        nonempty.insert(u32::MAX);

        assert_eq!(nonempty.union(&empty), nonempty, "A union empty = A");
        assert_eq!(empty.union(&nonempty), nonempty, "empty union A = A");
        assert!(nonempty.intersection(&empty).is_empty(), "A intersection empty = empty");
        assert!(empty.intersection(&nonempty).is_empty(), "empty intersection A = empty");
        assert_eq!(nonempty.difference(&empty), nonempty, "A difference empty = A");
        assert!(empty.difference(&nonempty).is_empty(), "empty difference A = empty");

        let mut andnot = nonempty.clone();
        andnot.andnot(&empty);
        assert_eq!(andnot, nonempty, "A andnot empty = A");
        let mut empty_andnot = empty.clone();
        empty_andnot.andnot(&nonempty);
        assert!(empty_andnot.is_empty(), "empty andnot A = empty");

        for result in [empty.union(&empty), empty.intersection(&empty), empty.difference(&empty)] {
            assert!(result.is_empty(), "an empty result has no values");
            assert_eq!(result.len(), 0, "an empty result has len zero");
            assert_eq!(result.iter().count(), 0, "iteration agrees with len on an empty result");
            assert!(
                result.container_kinds().is_empty(),
                "an empty result leaves no empty container behind"
            );
        }
        assert!(
            nonempty.intersection(&empty).container_kinds().is_empty(),
            "an empty intersection leaves no empty container behind"
        );
    }

    #[test]
    fn set_algebra_container_pairings() {
        let mut array = RoaringBitmap::new();
        for value in 0..10u32 {
            array.insert(value * 3);
        }
        let mut bitmap = RoaringBitmap::new();
        bitmap.insert_range(0..=4096);
        assert_eq!(
            bitmap.container_kinds(),
            vec![ContainerKind::Bitmap],
            "a contiguous 4097-value chunk is a bitmap before run_optimize"
        );
        let mut run = RoaringBitmap::new();
        run.insert_range(0..=8191);
        run.run_optimize();
        assert_eq!(run.container_kinds(), vec![ContainerKind::Run], "run_optimize makes a run container");

        fn expected_kind(name: &str) -> ContainerKind {
            match name {
                "array" => ContainerKind::Array,
                "bitmap" => ContainerKind::Bitmap,
                _ => ContainerKind::Run,
            }
        }

        let operands: [(&str, &RoaringBitmap); 3] = [("array", &array), ("bitmap", &bitmap), ("run", &run)];
        let mut pairings = 0usize;
        for (name_a, a) in operands {
            for (name_b, b) in operands {
                pairings += 1;
                assert_eq!(
                    a.container_kinds(),
                    vec![expected_kind(name_a)],
                    "the left operand is a {name_a} container"
                );
                assert_eq!(
                    b.container_kinds(),
                    vec![expected_kind(name_b)],
                    "the right operand is a {name_b} container"
                );
                let model_a: BTreeSet<u32> = a.iter().collect();
                let model_b: BTreeSet<u32> = b.iter().collect();

                let union = a.union(b);
                assert_same_contents(
                    &collected(&union),
                    &model_a.union(&model_b).copied().collect::<Vec<u32>>(),
                    &format!("pairing {name_a}/{name_b}"),
                    "union",
                    "the BTreeSet model",
                );
                let intersection = a.intersection(b);
                assert_same_contents(
                    &collected(&intersection),
                    &model_a.intersection(&model_b).copied().collect::<Vec<u32>>(),
                    &format!("pairing {name_a}/{name_b}"),
                    "intersection",
                    "the BTreeSet model",
                );
                let difference = a.difference(b);
                assert_same_contents(
                    &collected(&difference),
                    &model_a.difference(&model_b).copied().collect::<Vec<u32>>(),
                    &format!("pairing {name_a}/{name_b}"),
                    "difference",
                    "the BTreeSet model",
                );
                let mut andnot = a.clone();
                andnot.andnot(b);
                assert_eq!(andnot, difference, "andnot pairing {name_a}/{name_b}");
            }
        }
        assert_eq!(pairings, 9, "every one of the nine operand pairings is exercised");
    }

    #[test]
    fn chunk_boundary_values_survive() {
        let mut a = RoaringBitmap::new();
        for value in [65_535u32, 65_536, 70_000, 0] {
            a.insert(value);
        }
        let mut b = RoaringBitmap::new();
        for value in [65_535u32, 65_536, 1, 2] {
            b.insert(value);
        }
        assert!(a.contains(65_535) && a.contains(65_536), "A holds both sides of the chunk boundary");
        assert!(b.contains(65_535) && b.contains(65_536), "B holds both sides of the chunk boundary");

        let union = a.union(&b);
        assert!(
            union.contains(65_535) && union.contains(65_536),
            "both boundary values survive union"
        );
        let intersection = a.intersection(&b);
        assert!(
            intersection.contains(65_535) && intersection.contains(65_536),
            "both boundary values survive intersection"
        );

        let mut disjoint = RoaringBitmap::new();
        disjoint.insert(1);
        disjoint.insert(2);
        let difference = a.difference(&disjoint);
        assert!(
            difference.contains(65_535) && difference.contains(65_536),
            "both boundary values survive difference"
        );
        let mut andnot = a.clone();
        andnot.andnot(&disjoint);
        assert!(
            andnot.contains(65_535) && andnot.contains(65_536),
            "both boundary values survive andnot"
        );
        assert_eq!(andnot, difference, "andnot and difference agree across the chunk boundary");

        assert!(
            union.container_kinds().len() >= 2,
            "65_535 and 65_536 land in different chunks"
        );
        let model_a: BTreeSet<u32> = a.iter().collect();
        let model_b: BTreeSet<u32> = b.iter().collect();
        assert_same_contents(
            &collected(&union),
            &model_a.union(&model_b).copied().collect::<Vec<u32>>(),
            "chunk boundary",
            "union",
            "the BTreeSet model",
        );
    }

    #[test]
    fn container_kinds_and_conversions() {
        let mut kinds_seen: Vec<ContainerKind> = Vec::new();

        let mut array = RoaringBitmap::new();
        for value in 0..4096u32 {
            assert!(array.insert(value), "first insert of {value}");
        }
        assert_eq!(
            array.container_kinds(),
            vec![ContainerKind::Array],
            "4096 values are an array container at ARRAY_MAX"
        );
        kinds_seen.push(ContainerKind::Array);

        assert!(array.insert(4096), "the 4097th value inserts");
        assert_eq!(
            array.container_kinds(),
            vec![ContainerKind::Bitmap],
            "an insert past ARRAY_MAX converts the array container to a bitmap container"
        );
        kinds_seen.push(ContainerKind::Bitmap);

        array.run_optimize();
        assert_eq!(
            array.container_kinds(),
            vec![ContainerKind::Run],
            "run_optimize turns a contiguous chunk into one run container"
        );
        assert_eq!(array.len(), 4097, "run_optimize preserves the values");
        assert_eq!(array.run_counts(), vec![1], "the contiguous chunk holds one run");
        kinds_seen.push(ContainerKind::Run);

        let mut touching = RoaringBitmap::new();
        touching.insert_range(1..=5);
        touching.insert_range(6..=9);
        assert_eq!(touching.len(), 9, "touching ranges hold nine values");
        touching.run_optimize();
        assert_eq!(touching.container_kinds(), vec![ContainerKind::Run]);
        assert_eq!(
            touching.run_counts(),
            vec![1],
            "two runs that touch ([1..=5] and [6..=9]) are stored as one run after run_optimize"
        );

        assert!(
            kinds_seen.contains(&ContainerKind::Array)
                && kinds_seen.contains(&ContainerKind::Bitmap)
                && kinds_seen.contains(&ContainerKind::Run),
            "all three container kinds must be observed through the helper"
        );
    }
}
