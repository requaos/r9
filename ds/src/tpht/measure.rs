//! measure — the D-15 spill-threshold measurement (test only; plan 03-08).
//!
//! The method and the selection rule are fixed in
//! `.planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/SPILL-THRESHOLD.md`
//! before any run.  This module is the single implementation of the shared
//! arithmetic (`entries_at`, `exceeds_bar`, `banks_for`), the five key
//! distributions, the seeded generator and the counting model; the ignored
//! `measure_spill_threshold_table` is the generator run on demand and the pin
//! in `flattened.rs` re-runs the chosen cell through the production
//! constructor.
//!
//! A run is a pure function of (distribution, seed, blocks, occupancy, `T`):
//! the generator is seeded, uses wrapping integer arithmetic, and qualifies
//! only through the exact integer `exceeds_bar`.

use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;

use super::chained::BANK_NODES;
use super::flattened::{FlattenedTphtShard, IndexChoice};
use super::{HOME_SLOTS, index_bits};

/// The grid's block count (2^14), fixed before the measurement.
pub(crate) const GRID_BLOCKS: usize = 16_384;

/// The grid's three literal seeds, fixed before the measurement.
pub(crate) const GRID_SEEDS: [u64; 3] =
    [0x0000_0000_0000_0808, 0x1111_1111_1111_0808, 0x2222_2222_2222_0808];

/// The grid's occupancies as integer percents, fixed before the measurement.
pub(crate) const GRID_OCCUPANCIES: [usize; 7] = [50, 70, 80, 85, 90, 95, 100];

/// The grid's spill thresholds, fixed before the measurement.
pub(crate) const GRID_THRESHOLDS: [usize; 6] = [4, 8, 12, 14, 15, 16];

/// SplitMix64: the seeded PRNG of the uniform distribution (no `rand`,
/// no `proptest`).  Its output function is a bijection on `u64`, so successive
/// outputs are distinct keys.
struct SplitMix64(u64);

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

/// The grid's five key distributions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Distribution {
    /// Uniform random `u64` from the seeded generator.
    Uniform,
    /// Sequential `0..n`.
    Sequential,
    /// `PageKey`-style `(image_id << 32) | page_offset`, 64 images.
    PageKey,
    /// Keys that vary only in the upper 32 bits.
    Upper32,
    /// `(id << 32) | 7`.
    Upper32Or7,
}

impl Distribution {
    /// All five distributions, in the order the evidence reports them.
    pub(crate) const ALL: [Distribution; 5] =
        [Self::Uniform, Self::Sequential, Self::PageKey, Self::Upper32, Self::Upper32Or7];

    /// The name the evidence and the pin print.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Uniform => "uniform",
            Self::Sequential => "sequential",
            Self::PageKey => "pagekey",
            Self::Upper32 => "upper32",
            Self::Upper32Or7 => "upper32_or7",
        }
    }
}

/// The keys of `distribution` from index `start`, in order.  Distinct indices
/// give distinct keys for every distribution, so present keys (from 0) and
/// absent keys (from `n`) never collide.
pub(crate) fn keys(
    distribution: Distribution,
    seed: u64,
    start: usize,
) -> Box<dyn Iterator<Item = u64>> {
    match distribution {
        Distribution::Uniform => {
            let mut rng = SplitMix64::new(seed);
            for _ in 0..start {
                rng.next();
            }
            Box::new(core::iter::from_fn(move || Some(rng.next())))
        }
        Distribution::Sequential => Box::new((start..).map(|i| i as u64)),
        Distribution::PageKey => {
            Box::new((start..).map(|i| (((i % 64) as u64) << 32) | (i / 64) as u64))
        }
        Distribution::Upper32 => Box::new((start..).map(|i| (i as u64) << 32)),
        Distribution::Upper32Or7 => Box::new((start..).map(|i| ((i as u64) << 32) | 7)),
    }
}

/// The number of keys inserted for an occupancy: `blocks * 16 * percent / 100`
/// (integer division; the printed decimals are for reading only).
pub(crate) fn entries_at(blocks: usize, percent: usize) -> usize {
    blocks * HOME_SLOTS * percent / 100
}

/// The 0.90 bar as one exact integer test: `hits * 10 > lookups * 9`.
pub(crate) fn exceeds_bar(hits: usize, lookups: usize) -> bool {
    hits * 10 > lookups * 9
}

/// Twice the number of 255-node banks that would hold every entry, at least 2,
/// so no insert fails with `BanksFull` at any `T`.
pub(crate) fn banks_for(entries: usize) -> usize {
    (2 * entries.div_ceil(BANK_NODES)).max(2)
}

/// Insert `entries_at(blocks, percent)` distinct keys and look every one of
/// them up; return `(home hits, lookups)` for the mixed-half index.  Every
/// insert must succeed, so a run is a pure function of its inputs.
pub(crate) fn home_hit_fraction(
    distribution: Distribution,
    seed: u64,
    blocks: usize,
    percent: usize,
    threshold: usize,
    banks: usize,
) -> (usize, usize) {
    let entries = entries_at(blocks, percent);
    let shard =
        FlattenedTphtShard::<2>::with_test_config(blocks, banks, threshold, IndexChoice::MixedHalf)
            .expect("the measurement's block and bank counts are valid");
    for (i, key) in keys(distribution, seed, 0).take(entries).enumerate() {
        let result = shard.insert(key, [key, !key], |v| v[0] == key);
        assert!(result.is_ok(), "insert {i} must succeed at T={threshold}: {result:?}");
    }
    let mut hits = 0;
    let mut lookups = 0;
    for key in keys(distribution, seed, 0).take(entries) {
        let found = shard.lookup(key, |v| v[0] == key).expect("a present key must resolve");
        assert_eq!(found.value, [key, !key], "lookup returns the inserted value");
        if found.home.is_some() {
            hits += 1;
        }
        lookups += 1;
    }
    (hits, lookups)
}

/// The counting model: `sum over blocks of min(count, threshold) / n` for the
/// same keys and index variant.
pub(crate) fn counting_model(
    distribution: Distribution,
    seed: u64,
    blocks: usize,
    percent: usize,
    threshold: usize,
    index_choice: IndexChoice,
) -> (usize, usize) {
    let entries = entries_at(blocks, percent);
    let bits = index_bits(blocks).expect("the grid's block count is a power of two");
    let mut counts = vec![0usize; blocks];
    for key in keys(distribution, seed, 0).take(entries) {
        let (block, _remainder) = index_choice.split(key, bits);
        counts[block] += 1;
    }
    let hits = counts.iter().map(|&count| count.min(threshold)).sum();
    (hits, entries)
}

/// The fraction of absent-key lookups that end at the header: an absent key
/// ends there when its home block has no spilled entries (chain empty, so
/// `count <= threshold`).  Absent keys are the same generator over `n..2n`.
pub(crate) fn absent_header_fraction(
    distribution: Distribution,
    seed: u64,
    blocks: usize,
    percent: usize,
    threshold: usize,
    index_choice: IndexChoice,
) -> (usize, usize) {
    let entries = entries_at(blocks, percent);
    let bits = index_bits(blocks).expect("the grid's block count is a power of two");
    let mut counts = vec![0usize; blocks];
    for key in keys(distribution, seed, 0).take(entries) {
        let (block, _remainder) = index_choice.split(key, bits);
        counts[block] += 1;
    }
    let mut ends_at_header = 0;
    let mut lookups = 0;
    for key in keys(distribution, seed, entries).take(entries) {
        let (block, _remainder) = index_choice.split(key, bits);
        if counts[block] <= threshold {
            ends_at_header += 1;
        }
        lookups += 1;
    }
    (ends_at_header, lookups)
}

/// Min, mean and max of a set of `(hits, lookups)` pairs, as fractions.
fn stats(runs: &[(usize, usize)]) -> (f64, f64, f64) {
    let fractions: Vec<f64> =
        runs.iter().map(|&(hits, lookups)| hits as f64 / lookups as f64).collect();
    let min = fractions.iter().copied().fold(f64::INFINITY, f64::min);
    let max = fractions.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let mean = fractions.iter().sum::<f64>() / fractions.len() as f64;
    (min, mean, max)
}

/// The chosen cell: its occupancy percent, threshold, fifteen `(hits,
/// lookups)` runs and, for the same threshold one occupancy up the grid, the
/// same fifteen runs.
struct Chosen {
    percent: usize,
    threshold: usize,
    runs: Vec<(usize, usize)>,
    next: Option<(usize, Vec<(usize, usize)>)>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tpht::miri_scaled;

    /// A fraction measured through the real table equals the counting model
    /// for the same keys: `hits = sum over blocks of min(count, T)`.
    #[test]
    fn table_agrees_with_counting_model() {
        let blocks = miri_scaled(1_024, 16);
        let seed_count = miri_scaled(GRID_SEEDS.len(), 1);
        let dist_count = miri_scaled(Distribution::ALL.len(), 2);
        let percent = 100;
        let entries = entries_at(blocks, percent);
        let banks = banks_for(entries);
        for distribution in Distribution::ALL.iter().take(dist_count) {
            for &seed in GRID_SEEDS.iter().take(seed_count) {
                let measured = home_hit_fraction(
                    *distribution,
                    seed,
                    blocks,
                    percent,
                    crate::tpht::SPILL_THRESHOLD,
                    banks,
                );
                let modelled = counting_model(
                    *distribution,
                    seed,
                    blocks,
                    percent,
                    crate::tpht::SPILL_THRESHOLD,
                    IndexChoice::MixedHalf,
                );
                assert_eq!(
                    measured, modelled,
                    "the table and the counting model must agree for {:?} seed {seed:#018x}",
                    distribution
                );
            }
        }
    }

    /// The D-15 generator: run on demand, never under Miri, never a pin.
    #[test]
    #[ignore = "generator run on demand for the D-15 evidence; not part of the pin suite"]
    fn measure_spill_threshold_table() {
        let blocks = GRID_BLOCKS;
        println!(
            "SPILL-THRESHOLD MEASUREMENT blocks={blocks} seeds={:#018x},{:#018x},{:#018x} \
             occupancies=0.50,0.70,0.80,0.85,0.90,0.95,1.00 thresholds=4,8,12,14,15,16",
            GRID_SEEDS[0], GRID_SEEDS[1], GRID_SEEDS[2]
        );
        // mixed[dist][T][occupancy][seed] = (hits, lookups)
        let mut mixed: Vec<Vec<Vec<Vec<(usize, usize)>>>> = Vec::new();
        for &distribution in Distribution::ALL.iter() {
            let mut per_threshold = Vec::new();
            for &threshold in GRID_THRESHOLDS.iter() {
                let mut per_occupancy = Vec::new();
                for &percent in GRID_OCCUPANCIES.iter() {
                    let entries = entries_at(blocks, percent);
                    let banks = banks_for(entries);
                    let runs = GRID_SEEDS
                        .iter()
                        .map(|&seed| {
                            home_hit_fraction(distribution, seed, blocks, percent, threshold, banks)
                        })
                        .collect();
                    per_occupancy.push(runs);
                }
                per_threshold.push(per_occupancy);
            }
            mixed.push(per_threshold);
        }

        for (dist_index, &distribution) in Distribution::ALL.iter().enumerate() {
            println!("DIST {}", distribution.name());
            println!("MIXED home-hit fraction (min/mean/max over seeds)");
            print_header();
            for (t_index, &threshold) in GRID_THRESHOLDS.iter().enumerate() {
                print!("T={threshold:<3}");
                for occ_index in 0..GRID_OCCUPANCIES.len() {
                    let (min, mean, max) = stats(&mixed[dist_index][t_index][occ_index]);
                    print!(" {min:.4}/{mean:.4}/{max:.4}");
                }
                println!();
            }
            println!(
                "ABSENT fraction ending at the header (counting model, min/mean/max over seeds)"
            );
            print_header();
            for &threshold in GRID_THRESHOLDS.iter() {
                print!("T={threshold:<3}");
                for &percent in GRID_OCCUPANCIES.iter() {
                    let runs: Vec<(usize, usize)> = GRID_SEEDS
                        .iter()
                        .map(|&seed| {
                            absent_header_fraction(
                                distribution,
                                seed,
                                blocks,
                                percent,
                                threshold,
                                IndexChoice::MixedHalf,
                            )
                        })
                        .collect();
                    let (min, mean, max) = stats(&runs);
                    print!(" {min:.4}/{mean:.4}/{max:.4}");
                }
                println!();
            }
            println!(
                "UNMIXED low-bits-of-P home-hit fraction (counting model, min/mean/max over seeds)"
            );
            print_header();
            for &threshold in GRID_THRESHOLDS.iter() {
                print!("T={threshold:<3}");
                for &percent in GRID_OCCUPANCIES.iter() {
                    let runs: Vec<(usize, usize)> = GRID_SEEDS
                        .iter()
                        .map(|&seed| {
                            counting_model(
                                distribution,
                                seed,
                                blocks,
                                percent,
                                threshold,
                                IndexChoice::UnmixedLowBits,
                            )
                        })
                        .collect();
                    let (min, mean, max) = stats(&runs);
                    print!(" {min:.4}/{mean:.4}/{max:.4}");
                }
                println!();
            }
        }

        // The rule, applied in code to the measured mixed fractions.
        let qualifies = |t_index: usize, occ_index: usize| {
            (0..Distribution::ALL.len()).all(|dist_index| {
                mixed[dist_index][t_index][occ_index]
                    .iter()
                    .all(|&(hits, lookups)| exceeds_bar(hits, lookups))
            })
        };
        println!("QUALIFIES (mixed, all 15 runs over the 0.90 bar)");
        for (t_index, &threshold) in GRID_THRESHOLDS.iter().enumerate() {
            for (occ_index, &percent) in GRID_OCCUPANCIES.iter().enumerate() {
                let answer = if qualifies(t_index, occ_index) { "yes" } else { "no" };
                println!("T={threshold} occupancy={} qualifies={answer}", occupancy(percent));
            }
        }

        let mut chosen: Option<Chosen> = None;
        'occupancy: for (occ_index, &percent) in GRID_OCCUPANCIES.iter().enumerate().rev() {
            if percent < 85 {
                break;
            }
            for (t_index, &threshold) in GRID_THRESHOLDS.iter().enumerate() {
                if qualifies(t_index, occ_index) {
                    let runs = (0..Distribution::ALL.len())
                        .flat_map(|dist_index| mixed[dist_index][t_index][occ_index].clone())
                        .collect();
                    let next = GRID_OCCUPANCIES.get(occ_index + 1).map(|&next_percent| {
                        let next_runs = (0..Distribution::ALL.len())
                            .flat_map(|dist_index| {
                                mixed[dist_index][t_index][occ_index + 1].clone()
                            })
                            .collect();
                        (next_percent, next_runs)
                    });
                    chosen = Some(Chosen { percent, threshold, runs, next });
                    break 'occupancy;
                }
            }
        }

        match chosen {
            Some(chosen) => {
                println!("RULE SPILL_THRESHOLD={}", chosen.threshold);
                println!("RULE HIT_RATE_OCCUPANCY={}", occupancy(chosen.percent));
                let mut run_index = 0;
                for &distribution in Distribution::ALL.iter() {
                    for &seed in GRID_SEEDS.iter() {
                        let (hits, lookups) = chosen.runs[run_index];
                        run_index += 1;
                        println!(
                            "CELL dist={} seed={seed:#018x} hits={hits} lookups={lookups}",
                            distribution.name()
                        );
                    }
                }
                let mut weakest = chosen.runs[0];
                for &run in &chosen.runs {
                    if run.0 * weakest.1 < weakest.0 * run.1 {
                        weakest = run;
                    }
                }
                println!(
                    "MARGIN hits={} lookups={} fraction={:.6}",
                    weakest.0,
                    weakest.1,
                    weakest.0 as f64 / weakest.1 as f64
                );
                match chosen.next {
                    Some((next_percent, next_runs)) => {
                        let mut run_index = 0;
                        for &distribution in Distribution::ALL.iter() {
                            for &seed in GRID_SEEDS.iter() {
                                let (hits, lookups) = next_runs[run_index];
                                run_index += 1;
                                println!(
                                    "NEXT T={} occupancy={} dist={} seed={seed:#018x} \
                                     hits={hits} lookups={lookups}",
                                    chosen.threshold,
                                    occupancy(next_percent),
                                    distribution.name()
                                );
                            }
                        }
                    }
                    None => println!("NEXT NONE (the chosen occupancy is the top of the grid)"),
                }
            }
            None => println!("RULE NONE"),
        }
    }

    fn occupancy(percent: usize) -> alloc::string::String {
        alloc::format!("{:.2}", percent as f64 / 100.0)
    }

    fn print_header() {
        print!("     ");
        for &percent in GRID_OCCUPANCIES.iter() {
            print!(" {:>16}", occupancy(percent));
        }
        println!();
    }
}
