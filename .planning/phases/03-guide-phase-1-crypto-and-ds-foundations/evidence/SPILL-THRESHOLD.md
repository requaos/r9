# Spill-threshold measurement: method and selection rule (D-15)

This file is the evidence for DS-01's second half and D-15: how many of a
block's 16 home slots fill before a block's further keys spill into the
dereference banks.  This first section — the definitions, the grid, the
arithmetic and the selection rule — was committed **before any measurement
ran**.  It is not edited afterwards; the measured tables and the chosen values
are appended below it.

## Definitions

- **Occupancy** is entries inserted divided by `HOME_SLOTS * blocks`, where
  `HOME_SLOTS` is 16 and `blocks` is the grid's block count.  The grid stores
  occupancy as an integer percent; `entries_at(blocks, percent)` is
  `blocks * 16 * percent / 100` with integer division, and the printed
  decimals are for reading only.
- A lookup of a present key is a **home-block hit** when it resolves from the
  home header without touching a bank: `lookup` returns
  `Found { home: Some(_), .. }`.  A `Found { home: None, .. }` result is a
  spill, not a hit.  The fraction is over lookups of present keys only.
- The **spill threshold** `T` is the number of header slots a block may use
  before further keys of that block go to the banks, `1 <= T <= 16`.
- An **absent-key lookup ends at the header** when the key's home block has no
  spilled entries (its chain is empty), so a header scan alone proves absence.
  Absent keys are the same distribution's generator continued over the next
  `n` indices (`n..2n`), so they are distinct from the present keys; the
  absent-key fraction is computed from the counting model.

## Grid

- `T` in **4, 8, 12, 14, 15, 16**.
- Occupancy in **0.50, 0.70, 0.80, 0.85, 0.90, 0.95, 1.00** (integer percent).
- Five key distributions, with these exact generators (index `i` from 0):
  - `uniform`: the i-th output of SplitMix64 seeded with the literal seed
    (`state += 0x9E37_79B9_7F4A_7C15`; the MurmurHash3-style finalizer), so the
    sequence is distinct keys for distinct `i`;
  - `sequential`: `i`;
  - `pagekey`: `((i % 64) as u64) << 32 | (i / 64) as u64` (64 images,
    sequential page offsets);
  - `upper32`: `(i as u64) << 32`;
  - `upper32_or7`: `((i as u64) << 32) | 7`.
- Three literal seeds: `0x0000_0000_0000_0808`, `0x1111_1111_1111_0808`,
  `0x2222_2222_2222_0808`.
- **16,384 blocks** (2^14) in every cell; 15 runs per cell (5 distributions ×
  3 seeds).

## Arithmetic (one implementation, in `ds/src/tpht/measure.rs`)

- `entries_at(blocks, percent)`: `blocks * HOME_SLOTS * percent / 100`.
- `exceeds_bar(hits, lookups)`: the exact integer test `hits * 10 > lookups *
  9` — the 0.90 bar; qualification never uses a printed decimal.
- `banks_for(entries)`: twice the number of 255-node banks that would hold
  every entry, and at least 2 — `max(2, 2 * ceil(entries / 255))` — so no
  insert fails with `BanksFull` at any `T`.  The banks are sized for every
  entry, not only the expected spills.
- The harness is single-threaded, asserts every insert returned `Ok`, and uses
  a seeded generator with wrapping integer arithmetic, so a run is a pure
  function of (distribution, seed, blocks, occupancy, `T`).

## Reported per cell

- The **mixed-half** home-hit fraction for present keys, measured through the
  real table (`with_test_config(blocks, banks, T, IndexChoice::MixedHalf)`),
  reported as min, mean and max over the three seeds.
- The **unmixed low-bits** home-hit fraction — the compendium section 4.1
  reading, `feistel::split_unmixed` — computed from the counting model:
  `sum over blocks of min(count, T) / n`.
- The absent-key fraction that ends at the header, from the counting model.

## Selection rule (fixed before the measurement)

A pair (`T`, occupancy) **qualifies** when `exceeds_bar` holds with the
mixed-half index for **every distribution and every seed** — the home-hit
fraction is above 0.90 in all fifteen runs.

- `HIT_RATE_OCCUPANCY` is the **highest occupancy in the grid** for which some
  `T` qualifies.
- `SPILL_THRESHOLD` is the **smallest `T` that qualifies at that occupancy**.
- If no `T` qualifies at any occupancy of **0.85 or above**, nothing is
  chosen: the measurement stops and the table is returned to the owner.

The generator applies this rule in code and prints its result; the rule is
never applied by reading rounded figures.

## The pin and the rule

The pin `hit_rate_home_block_exceeds_90_percent` **re-runs the chosen cell**.
It uses the grid's 16,384 blocks, three seeds, five distributions and the same
key generators, `entries_at`, `banks_for` and `exceeds_bar`, on a shard built
by the production `with_banks`, so the threshold in force is the production
`SPILL_THRESHOLD` constant.  Because a run is a pure function of its inputs,
the pin's fifteen counts equal the chosen cell's fifteen counts, and a cell
that qualified passes the pin.  This is fixed here, before any measurement.

## No margin

The bar is not raised above 0.90 and no occupancy is skipped, so the choice
keeps D-15's meaning: the highest occupancy at which a threshold is over 90%.
What the choice rests on is recorded instead:

- a line `MARGIN hits=<h> lookups=<n>` for the weakest of the fifteen runs of
  the chosen cell;
- the same `T` at the next occupancy up the grid (when there is one), shown
  with its counts (`NEXT ...`) so the owner can see how far it fell short.

RESEARCH F9's model puts `T = 16` at 0.901 for occupancy 1.00 and 0.922 for
0.95; if the measurement chooses a cell that close to the bar, the `MARGIN`
line shows it, and the pin still passes because it is the same computation.

## Results

To be filled by the measurement run below; the section above is unchanged.
