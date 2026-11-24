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

## Measurement

- **Venue:** `uname -sm` prints `Darwin arm64`.
- **Commit:** `970b7d99934a0ad103466770af9ef9d5dd896f09` (the commit that added
  the measurement harness; the rule above was committed earlier at `b479d28`).
- **Exact command:**

  ```sh
  nix develop -c cargo test -p ds --lib tpht::measure::tests::measure_spill_threshold_table -- --ignored --nocapture
  ```

- **Run:** exit code 0; the generator took 6.93 s in the `test` profile.  The
  block below is the generator's stdout, unedited.

```text
SPILL-THRESHOLD MEASUREMENT blocks=16384 seeds=0x0000000000000808,0x1111111111110808,0x2222222222220808 occupancies=0.50,0.70,0.80,0.85,0.90,0.95,1.00 thresholds=4,8,12,14,15,16
DIST uniform
MIXED home-hit fraction (min/mean/max over seeds)
                  0.50             0.70             0.80             0.85             0.90             0.95             1.00
T=4   0.4923/0.4928/0.4931 0.3566/0.3567/0.3568 0.3124/0.3124/0.3124 0.2941/0.2941/0.2941 0.2777/0.2778/0.2778 0.2631/0.2631/0.2632 0.2500/0.2500/0.2500
T=8   0.8594/0.8604/0.8614 0.6916/0.6918/0.6921 0.6166/0.6168/0.6171 0.5833/0.5834/0.5836 0.5526/0.5527/0.5528 0.5245/0.5247/0.5248 0.4990/0.4991/0.4991
T=12  0.9833/0.9837/0.9843 0.9123/0.9128/0.9135 0.8562/0.8566/0.8569 0.8258/0.8259/0.8261 0.7942/0.7945/0.7950 0.7625/0.7631/0.7637 0.7321/0.7324/0.7329
T=14  0.9959/0.9961/0.9963 0.9634/0.9637/0.9642 0.9283/0.9286/0.9289 0.9063/0.9065/0.9067 0.8815/0.8819/0.8824 0.8552/0.8557/0.8562 0.8280/0.8284/0.8289
T=15  0.9982/0.9983/0.9983 0.9779/0.9780/0.9782 0.9520/0.9522/0.9525 0.9345/0.9348/0.9351 0.9141/0.9146/0.9151 0.8919/0.8922/0.8928 0.8677/0.8681/0.8686
T=16  0.9992/0.9992/0.9993 0.9872/0.9873/0.9875 0.9690/0.9692/0.9695 0.9557/0.9561/0.9564 0.9399/0.9402/0.9408 0.9216/0.9220/0.9225 0.9010/0.9014/0.9021
ABSENT fraction ending at the header (counting model, min/mean/max over seeds)
                  0.50             0.70             0.80             0.85             0.90             0.95             1.00
T=4   0.0985/0.0991/0.0999 0.0114/0.0126/0.0132 0.0035/0.0041/0.0044 0.0021/0.0022/0.0024 0.0012/0.0014/0.0016 0.0006/0.0008/0.0011 0.0002/0.0004/0.0007
T=8   0.5898/0.5930/0.5971 0.2127/0.2161/0.2192 0.1052/0.1083/0.1116 0.0722/0.0741/0.0758 0.0483/0.0501/0.0513 0.0311/0.0334/0.0352 0.0204/0.0217/0.0234
T=12  0.9340/0.9355/0.9378 0.6681/0.6694/0.6705 0.4832/0.4853/0.4876 0.3961/0.3977/0.3998 0.3161/0.3186/0.3213 0.2485/0.2499/0.2523 0.1894/0.1911/0.1929
T=14  0.9814/0.9825/0.9837 0.8361/0.8397/0.8441 0.6980/0.6986/0.6996 0.6137/0.6156/0.6185 0.5296/0.5304/0.5310 0.4422/0.4447/0.4469 0.3649/0.3657/0.3667
T=15  0.9913/0.9920/0.9925 0.8945/0.8968/0.8996 0.7808/0.7819/0.7827 0.7077/0.7098/0.7119 0.6288/0.6307/0.6340 0.5472/0.5477/0.5480 0.4642/0.4664/0.4684
T=16  0.9961/0.9963/0.9964 0.9369/0.9376/0.9381 0.8492/0.8503/0.8510 0.7909/0.7916/0.7929 0.7208/0.7232/0.7254 0.6453/0.6464/0.6483 0.5638/0.5668/0.5685
UNMIXED low-bits-of-P home-hit fraction (counting model, min/mean/max over seeds)
                  0.50             0.70             0.80             0.85             0.90             0.95             1.00
T=4   0.4925/0.4926/0.4927 0.3566/0.3567/0.3567 0.3124/0.3124/0.3124 0.2940/0.2941/0.2941 0.2777/0.2777/0.2778 0.2631/0.2631/0.2632 0.2500/0.2500/0.2500
T=8   0.8599/0.8607/0.8613 0.6915/0.6917/0.6918 0.6167/0.6167/0.6167 0.5831/0.5832/0.5832 0.5524/0.5525/0.5526 0.5244/0.5246/0.5247 0.4989/0.4990/0.4991
T=12  0.9835/0.9838/0.9845 0.9122/0.9126/0.9129 0.8565/0.8566/0.8569 0.8257/0.8259/0.8260 0.7942/0.7944/0.7945 0.7630/0.7632/0.7633 0.7321/0.7322/0.7325
T=14  0.9958/0.9960/0.9964 0.9631/0.9635/0.9638 0.9281/0.9284/0.9289 0.9058/0.9062/0.9064 0.8810/0.8816/0.8820 0.8549/0.8555/0.8559 0.8277/0.8281/0.8284
T=15  0.9980/0.9982/0.9984 0.9775/0.9778/0.9781 0.9519/0.9522/0.9527 0.9341/0.9346/0.9349 0.9139/0.9144/0.9148 0.8914/0.8920/0.8924 0.8670/0.8677/0.8681
T=16  0.9990/0.9992/0.9993 0.9869/0.9871/0.9874 0.9691/0.9693/0.9698 0.9558/0.9562/0.9564 0.9397/0.9401/0.9405 0.9210/0.9218/0.9222 0.9003/0.9010/0.9014
DIST sequential
MIXED home-hit fraction (min/mean/max over seeds)
                  0.50             0.70             0.80             0.85             0.90             0.95             1.00
T=4   0.4924/0.4924/0.4924 0.3566/0.3566/0.3566 0.3124/0.3124/0.3124 0.2941/0.2941/0.2941 0.2777/0.2777/0.2777 0.2631/0.2631/0.2631 0.2500/0.2500/0.2500
T=8   0.8599/0.8599/0.8599 0.6911/0.6911/0.6911 0.6163/0.6163/0.6163 0.5831/0.5831/0.5831 0.5526/0.5526/0.5526 0.5247/0.5247/0.5247 0.4990/0.4990/0.4990
T=12  0.9838/0.9838/0.9838 0.9117/0.9117/0.9117 0.8562/0.8562/0.8562 0.8253/0.8253/0.8253 0.7940/0.7940/0.7940 0.7627/0.7627/0.7627 0.7317/0.7317/0.7317
T=14  0.9961/0.9961/0.9961 0.9631/0.9631/0.9631 0.9281/0.9281/0.9281 0.9058/0.9058/0.9058 0.8814/0.8814/0.8814 0.8552/0.8552/0.8552 0.8276/0.8276/0.8276
T=15  0.9982/0.9982/0.9982 0.9774/0.9774/0.9774 0.9520/0.9520/0.9520 0.9344/0.9344/0.9344 0.9141/0.9141/0.9141 0.8917/0.8917/0.8917 0.8670/0.8670/0.8670
T=16  0.9993/0.9993/0.9993 0.9868/0.9868/0.9868 0.9689/0.9689/0.9689 0.9558/0.9558/0.9558 0.9398/0.9398/0.9398 0.9216/0.9216/0.9216 0.9004/0.9004/0.9004
ABSENT fraction ending at the header (counting model, min/mean/max over seeds)
                  0.50             0.70             0.80             0.85             0.90             0.95             1.00
T=4   0.1018/0.1018/0.1018 0.0142/0.0142/0.0142 0.0047/0.0047/0.0047 0.0026/0.0026/0.0026 0.0013/0.0013/0.0013 0.0004/0.0004/0.0004 0.0003/0.0003/0.0003
T=8   0.5928/0.5928/0.5928 0.2164/0.2164/0.2164 0.1097/0.1097/0.1097 0.0762/0.0762/0.0762 0.0519/0.0519/0.0519 0.0346/0.0346/0.0346 0.0225/0.0225/0.0225
T=12  0.9350/0.9350/0.9350 0.6669/0.6669/0.6669 0.4868/0.4868/0.4868 0.3994/0.3994/0.3994 0.3214/0.3214/0.3214 0.2544/0.2544/0.2544 0.1952/0.1952/0.1952
T=14  0.9828/0.9828/0.9828 0.8420/0.8420/0.8420 0.6966/0.6966/0.6966 0.6133/0.6133/0.6133 0.5304/0.5304/0.5304 0.4476/0.4476/0.4476 0.3716/0.3716/0.3716
T=15  0.9913/0.9913/0.9913 0.8959/0.8959/0.8959 0.7854/0.7854/0.7854 0.7110/0.7110/0.7110 0.6317/0.6317/0.6317 0.5480/0.5480/0.5480 0.4695/0.4695/0.4695
T=16  0.9962/0.9962/0.9962 0.9373/0.9373/0.9373 0.8540/0.8540/0.8540 0.7930/0.7930/0.7930 0.7234/0.7234/0.7234 0.6463/0.6463/0.6463 0.5680/0.5680/0.5680
UNMIXED low-bits-of-P home-hit fraction (counting model, min/mean/max over seeds)
                  0.50             0.70             0.80             0.85             0.90             0.95             1.00
T=4   0.5000/0.5000/0.5000 0.3571/0.3571/0.3571 0.3125/0.3125/0.3125 0.2941/0.2941/0.2941 0.2778/0.2778/0.2778 0.2632/0.2632/0.2632 0.2500/0.2500/0.2500
T=8   1.0000/1.0000/1.0000 0.7143/0.7143/0.7143 0.6250/0.6250/0.6250 0.5882/0.5882/0.5882 0.5556/0.5556/0.5556 0.5263/0.5263/0.5263 0.5000/0.5000/0.5000
T=12  1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 0.9375/0.9375/0.9375 0.8824/0.8824/0.8824 0.8333/0.8333/0.8333 0.7895/0.7895/0.7895 0.7500/0.7500/0.7500
T=14  1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 0.9722/0.9722/0.9722 0.9211/0.9211/0.9211 0.8750/0.8750/0.8750
T=15  1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 0.9868/0.9868/0.9868 0.9375/0.9375/0.9375
T=16  1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000
DIST pagekey
MIXED home-hit fraction (min/mean/max over seeds)
                  0.50             0.70             0.80             0.85             0.90             0.95             1.00
T=4   0.4917/0.4917/0.4917 0.3564/0.3564/0.3564 0.3125/0.3125/0.3125 0.2941/0.2941/0.2941 0.2778/0.2778/0.2778 0.2632/0.2632/0.2632 0.2500/0.2500/0.2500
T=8   0.8545/0.8545/0.8545 0.6913/0.6913/0.6913 0.6162/0.6162/0.6162 0.5811/0.5811/0.5811 0.5509/0.5509/0.5509 0.5243/0.5243/0.5243 0.4990/0.4990/0.4990
T=12  0.9824/0.9824/0.9824 0.9141/0.9141/0.9141 0.8569/0.8569/0.8569 0.8240/0.8240/0.8240 0.7924/0.7924/0.7924 0.7643/0.7643/0.7643 0.7354/0.7354/0.7354
T=14  0.9976/0.9976/0.9976 0.9640/0.9640/0.9640 0.9317/0.9317/0.9317 0.9059/0.9059/0.9059 0.8811/0.8811/0.8811 0.8568/0.8568/0.8568 0.8313/0.8313/0.8313
T=15  0.9995/0.9995/0.9995 0.9773/0.9773/0.9773 0.9537/0.9537/0.9537 0.9341/0.9341/0.9341 0.9150/0.9150/0.9150 0.8933/0.8933/0.8933 0.8706/0.8706/0.8706
T=16  1.0000/1.0000/1.0000 0.9853/0.9853/0.9853 0.9689/0.9689/0.9689 0.9559/0.9559/0.9559 0.9405/0.9405/0.9405 0.9221/0.9221/0.9221 0.9036/0.9036/0.9036
ABSENT fraction ending at the header (counting model, min/mean/max over seeds)
                  0.50             0.70             0.80             0.85             0.90             0.95             1.00
T=4   0.1382/0.1382/0.1382 0.0091/0.0091/0.0091 0.0040/0.0040/0.0040 0.0043/0.0043/0.0043 0.0041/0.0041/0.0041 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000
T=8   0.5869/0.5869/0.5869 0.2522/0.2522/0.2522 0.1031/0.1031/0.1031 0.0750/0.0750/0.0750 0.0602/0.0602/0.0602 0.0409/0.0409/0.0409 0.0251/0.0251/0.0251
T=12  0.9170/0.9170/0.9170 0.6836/0.6836/0.6836 0.4703/0.4703/0.4703 0.4191/0.4191/0.4191 0.3334/0.3334/0.3334 0.2475/0.2475/0.2475 0.1780/0.1780/0.1780
T=14  0.9834/0.9834/0.9834 0.8528/0.8528/0.8528 0.7199/0.7199/0.7199 0.6276/0.6276/0.6276 0.5260/0.5260/0.5260 0.4569/0.4569/0.4569 0.3777/0.3777/0.3777
T=15  0.9971/0.9971/0.9971 0.9111/0.9111/0.9111 0.8071/0.8071/0.8071 0.7138/0.7138/0.7138 0.6453/0.6453/0.6453 0.5688/0.5688/0.5688 0.4778/0.4778/0.4778
T=16  1.0000/1.0000/1.0000 0.9429/0.9429/0.9429 0.8635/0.8635/0.8635 0.8224/0.8224/0.8224 0.7311/0.7311/0.7311 0.6544/0.6544/0.6544 0.5823/0.5823/0.5823
UNMIXED low-bits-of-P home-hit fraction (counting model, min/mean/max over seeds)
                  0.50             0.70             0.80             0.85             0.90             0.95             1.00
T=4   0.0625/0.0625/0.0625 0.0625/0.0625/0.0625 0.0625/0.0625/0.0625 0.0625/0.0625/0.0625 0.0625/0.0625/0.0625 0.0625/0.0625/0.0625 0.0625/0.0625/0.0625
T=8   0.1250/0.1250/0.1250 0.1250/0.1250/0.1250 0.1250/0.1250/0.1250 0.1250/0.1250/0.1250 0.1250/0.1250/0.1250 0.1250/0.1250/0.1250 0.1250/0.1250/0.1250
T=12  0.1875/0.1875/0.1875 0.1876/0.1876/0.1876 0.1875/0.1875/0.1875 0.1875/0.1875/0.1875 0.1875/0.1875/0.1875 0.1875/0.1875/0.1875 0.1875/0.1875/0.1875
T=14  0.2188/0.2188/0.2188 0.2188/0.2188/0.2188 0.2188/0.2188/0.2188 0.2188/0.2188/0.2188 0.2188/0.2188/0.2188 0.2188/0.2188/0.2188 0.2188/0.2188/0.2188
T=15  0.2344/0.2344/0.2344 0.2344/0.2344/0.2344 0.2344/0.2344/0.2344 0.2344/0.2344/0.2344 0.2344/0.2344/0.2344 0.2344/0.2344/0.2344 0.2344/0.2344/0.2344
T=16  0.2500/0.2500/0.2500 0.2500/0.2500/0.2500 0.2500/0.2500/0.2500 0.2500/0.2500/0.2500 0.2500/0.2500/0.2500 0.2500/0.2500/0.2500 0.2500/0.2500/0.2500
DIST upper32
MIXED home-hit fraction (min/mean/max over seeds)
                  0.50             0.70             0.80             0.85             0.90             0.95             1.00
T=4   0.5000/0.5000/0.5000 0.3571/0.3571/0.3571 0.3125/0.3125/0.3125 0.2941/0.2941/0.2941 0.2778/0.2778/0.2778 0.2632/0.2632/0.2632 0.2500/0.2500/0.2500
T=8   1.0000/1.0000/1.0000 0.7143/0.7143/0.7143 0.6250/0.6250/0.6250 0.5882/0.5882/0.5882 0.5556/0.5556/0.5556 0.5263/0.5263/0.5263 0.5000/0.5000/0.5000
T=12  1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 0.9375/0.9375/0.9375 0.8824/0.8824/0.8824 0.8333/0.8333/0.8333 0.7895/0.7895/0.7895 0.7500/0.7500/0.7500
T=14  1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 0.9722/0.9722/0.9722 0.9211/0.9211/0.9211 0.8750/0.8750/0.8750
T=15  1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 0.9868/0.9868/0.9868 0.9375/0.9375/0.9375
T=16  1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000
ABSENT fraction ending at the header (counting model, min/mean/max over seeds)
                  0.50             0.70             0.80             0.85             0.90             0.95             1.00
T=4   0.0000/0.0000/0.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000
T=8   1.0000/1.0000/1.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000
T=12  1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 0.2031/0.2031/0.2031 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000
T=14  1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 0.6111/0.6111/0.6111 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000
T=15  1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 0.8027/0.8027/0.8027 0.0000/0.0000/0.0000
T=16  1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000
UNMIXED low-bits-of-P home-hit fraction (counting model, min/mean/max over seeds)
                  0.50             0.70             0.80             0.85             0.90             0.95             1.00
T=4   0.0000/0.0000/0.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000
T=8   0.0001/0.0001/0.0001 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000
T=12  0.0001/0.0001/0.0001 0.0001/0.0001/0.0001 0.0001/0.0001/0.0001 0.0001/0.0001/0.0001 0.0001/0.0001/0.0001 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000
T=14  0.0001/0.0001/0.0001 0.0001/0.0001/0.0001 0.0001/0.0001/0.0001 0.0001/0.0001/0.0001 0.0001/0.0001/0.0001 0.0001/0.0001/0.0001 0.0001/0.0001/0.0001
T=15  0.0001/0.0001/0.0001 0.0001/0.0001/0.0001 0.0001/0.0001/0.0001 0.0001/0.0001/0.0001 0.0001/0.0001/0.0001 0.0001/0.0001/0.0001 0.0001/0.0001/0.0001
T=16  0.0001/0.0001/0.0001 0.0001/0.0001/0.0001 0.0001/0.0001/0.0001 0.0001/0.0001/0.0001 0.0001/0.0001/0.0001 0.0001/0.0001/0.0001 0.0001/0.0001/0.0001
DIST upper32_or7
MIXED home-hit fraction (min/mean/max over seeds)
                  0.50             0.70             0.80             0.85             0.90             0.95             1.00
T=4   0.5000/0.5000/0.5000 0.3571/0.3571/0.3571 0.3125/0.3125/0.3125 0.2941/0.2941/0.2941 0.2778/0.2778/0.2778 0.2632/0.2632/0.2632 0.2500/0.2500/0.2500
T=8   1.0000/1.0000/1.0000 0.7143/0.7143/0.7143 0.6250/0.6250/0.6250 0.5882/0.5882/0.5882 0.5556/0.5556/0.5556 0.5263/0.5263/0.5263 0.5000/0.5000/0.5000
T=12  1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 0.9375/0.9375/0.9375 0.8824/0.8824/0.8824 0.8333/0.8333/0.8333 0.7895/0.7895/0.7895 0.7500/0.7500/0.7500
T=14  1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 0.9722/0.9722/0.9722 0.9211/0.9211/0.9211 0.8750/0.8750/0.8750
T=15  1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 0.9868/0.9868/0.9868 0.9375/0.9375/0.9375
T=16  1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000
ABSENT fraction ending at the header (counting model, min/mean/max over seeds)
                  0.50             0.70             0.80             0.85             0.90             0.95             1.00
T=4   0.0000/0.0000/0.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000
T=8   1.0000/1.0000/1.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000
T=12  1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 0.2031/0.2031/0.2031 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000
T=14  1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 0.6111/0.6111/0.6111 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000
T=15  1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 0.8027/0.8027/0.8027 0.0000/0.0000/0.0000
T=16  1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000 1.0000/1.0000/1.0000
UNMIXED low-bits-of-P home-hit fraction (counting model, min/mean/max over seeds)
                  0.50             0.70             0.80             0.85             0.90             0.95             1.00
T=4   0.0000/0.0000/0.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000
T=8   0.0001/0.0001/0.0001 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000
T=12  0.0001/0.0001/0.0001 0.0001/0.0001/0.0001 0.0001/0.0001/0.0001 0.0001/0.0001/0.0001 0.0001/0.0001/0.0001 0.0000/0.0000/0.0000 0.0000/0.0000/0.0000
T=14  0.0001/0.0001/0.0001 0.0001/0.0001/0.0001 0.0001/0.0001/0.0001 0.0001/0.0001/0.0001 0.0001/0.0001/0.0001 0.0001/0.0001/0.0001 0.0001/0.0001/0.0001
T=15  0.0001/0.0001/0.0001 0.0001/0.0001/0.0001 0.0001/0.0001/0.0001 0.0001/0.0001/0.0001 0.0001/0.0001/0.0001 0.0001/0.0001/0.0001 0.0001/0.0001/0.0001
T=16  0.0001/0.0001/0.0001 0.0001/0.0001/0.0001 0.0001/0.0001/0.0001 0.0001/0.0001/0.0001 0.0001/0.0001/0.0001 0.0001/0.0001/0.0001 0.0001/0.0001/0.0001
QUALIFIES (mixed, all 15 runs over the 0.90 bar)
T=4 occupancy=0.50 qualifies=no
T=4 occupancy=0.70 qualifies=no
T=4 occupancy=0.80 qualifies=no
T=4 occupancy=0.85 qualifies=no
T=4 occupancy=0.90 qualifies=no
T=4 occupancy=0.95 qualifies=no
T=4 occupancy=1.00 qualifies=no
T=8 occupancy=0.50 qualifies=no
T=8 occupancy=0.70 qualifies=no
T=8 occupancy=0.80 qualifies=no
T=8 occupancy=0.85 qualifies=no
T=8 occupancy=0.90 qualifies=no
T=8 occupancy=0.95 qualifies=no
T=8 occupancy=1.00 qualifies=no
T=12 occupancy=0.50 qualifies=yes
T=12 occupancy=0.70 qualifies=yes
T=12 occupancy=0.80 qualifies=no
T=12 occupancy=0.85 qualifies=no
T=12 occupancy=0.90 qualifies=no
T=12 occupancy=0.95 qualifies=no
T=12 occupancy=1.00 qualifies=no
T=14 occupancy=0.50 qualifies=yes
T=14 occupancy=0.70 qualifies=yes
T=14 occupancy=0.80 qualifies=yes
T=14 occupancy=0.85 qualifies=yes
T=14 occupancy=0.90 qualifies=no
T=14 occupancy=0.95 qualifies=no
T=14 occupancy=1.00 qualifies=no
T=15 occupancy=0.50 qualifies=yes
T=15 occupancy=0.70 qualifies=yes
T=15 occupancy=0.80 qualifies=yes
T=15 occupancy=0.85 qualifies=yes
T=15 occupancy=0.90 qualifies=yes
T=15 occupancy=0.95 qualifies=no
T=15 occupancy=1.00 qualifies=no
T=16 occupancy=0.50 qualifies=yes
T=16 occupancy=0.70 qualifies=yes
T=16 occupancy=0.80 qualifies=yes
T=16 occupancy=0.85 qualifies=yes
T=16 occupancy=0.90 qualifies=yes
T=16 occupancy=0.95 qualifies=yes
T=16 occupancy=1.00 qualifies=yes
RULE SPILL_THRESHOLD=16
RULE HIT_RATE_OCCUPANCY=1.00
CELL dist=uniform seed=0x0000000000000808 hits=236470 lookups=262144
CELL dist=uniform seed=0x1111111111110808 hits=236196 lookups=262144
CELL dist=uniform seed=0x2222222222220808 hits=236262 lookups=262144
CELL dist=sequential seed=0x0000000000000808 hits=236035 lookups=262144
CELL dist=sequential seed=0x1111111111110808 hits=236035 lookups=262144
CELL dist=sequential seed=0x2222222222220808 hits=236035 lookups=262144
CELL dist=pagekey seed=0x0000000000000808 hits=236864 lookups=262144
CELL dist=pagekey seed=0x1111111111110808 hits=236864 lookups=262144
CELL dist=pagekey seed=0x2222222222220808 hits=236864 lookups=262144
CELL dist=upper32 seed=0x0000000000000808 hits=262144 lookups=262144
CELL dist=upper32 seed=0x1111111111110808 hits=262144 lookups=262144
CELL dist=upper32 seed=0x2222222222220808 hits=262144 lookups=262144
CELL dist=upper32_or7 seed=0x0000000000000808 hits=262144 lookups=262144
CELL dist=upper32_or7 seed=0x1111111111110808 hits=262144 lookups=262144
CELL dist=upper32_or7 seed=0x2222222222220808 hits=262144 lookups=262144
MARGIN hits=236035 lookups=262144 fraction=0.900402
NEXT NONE (the chosen occupancy is the top of the grid)
```

### Comparison with the RESEARCH F9 model

RESEARCH F9's Python model predicted `T = 16` (the only threshold over 0.90 at
0.95 occupancy) and about 0.901 for random keys at occupancy 1.00.  The
measurement agrees: the rule chooses `T = 16` at occupancy 1.00, and the
uniform distribution's chosen cell is 0.9010..0.9021.  The printed model and
the measured tables agree to within a few thousandths across the grid (for
example, uniform at `T = 14`, 0.85: model 0.906, measured 0.9063..0.9067), so
the choice is unchanged.  `T = 12` falls below 0.90 already at 0.85 occupancy,
as the model predicted.

## Choice

Checked by hand against the `qualifies=` flags above: at occupancy 1.00 only
`T = 16` qualifies (`T = 15` reaches 0.914 for uniform at 0.90 but 0.868 at
1.00, and every lower threshold fails at 1.00; the flags print `qualifies=no`
for each of them).  `T = 16` qualifies at every occupancy in the grid, so the
highest occupancy with a qualifying threshold is 1.00, and the smallest
qualifying threshold there is 16.

CHOSEN SPILL_THRESHOLD=16
CHOSEN HIT_RATE_OCCUPANCY=1.00
  justified by: T=16 qualifies at occupancy 1.00 (the only qualifying cell at
  that occupancy), and T=16 qualifies at 0.50, 0.70, 0.80, 0.85, 0.90 and 0.95
  as well; no threshold reaches the bar at 1.00 except T=16.

For the owner: the weakest of the chosen cell's fifteen runs is sequential at
236,035 / 262,144 = **0.900402**, only 0.000402 above the 0.90 bar, and the
chosen occupancy is the top of the grid, so there is no occupancy one step
higher to compare (`NEXT NONE` above).  The pin still passes because it is the
same computation on the same inputs; the `MARGIN` line records how thin the
margin is, as the no-margin rule requires.

## Home index: compendium 4.1 as written versus the mixed half

At the chosen threshold (`T = 16`) and occupancy (1.00), for each distribution,
home-hit fraction over the three seeds (min/mean/max; the mixed column is
measured through the real table, the unmixed column from the counting model):

| Distribution | Mixed half `L'` (this design) | Unmixed low bits of `P` (compendium 4.1 as written) |
|---|---|---|
| `uniform` | 0.9010 / 0.9014 / 0.9021 | 0.9003 / 0.9010 / 0.9014 |
| `sequential` | 0.9004 / 0.9004 / 0.9004 | 1.0000 / 1.0000 / 1.0000 |
| `pagekey` | 0.9036 / 0.9036 / 0.9036 | **0.2500 / 0.2500 / 0.2500** |
| `upper32` | 1.0000 / 1.0000 / 1.0000 | **0.0001 / 0.0001 / 0.0001** |
| `upper32_or7` | 1.0000 / 1.0000 / 1.0000 | **0.0001 / 0.0001 / 0.0001** |

The unmixed reading puts `pagekey` at 25% and both upper-half-only families at
about 0.01%, because with one Feistel round the low bits of `P` are the key's
own low bits and structured keys collapse into one block.  The mixed-half index
keeps every distribution over the bar.  Plan 03-06 took the index from the low
bits of `L'`; plan 03-09 records the correction to compendium section 4.1 in
PROJECT.md beside the other Phase 3 planning-level corrections (RESEARCH C4,
Open Question 1), for the owner's review at phase verification.  This choice is
outside the IG-D09 lock (D-17).
