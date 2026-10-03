# loopopt

Finds the best 4-uma parent loop for legacy affinity in Uma Musume (Global), with the exact race schedule for each uma. It reads only the game's plaintext `master.mdb` and does math; nothing in the game is modified.

```
master.mdb ──(export/export_rs.py)──▶ data.json ──(loopopt)──▶ ranked loops + schedules (JSON)
```

A whole pool (screen 814k foursomes, then search the survivors) takes **about 1 second** on a 2-core container.

---

## 1. Model

**Win chance** of one attempt (community table, Oct 2026):

```
p = clamp(110 − cost(distance grade) − cost(surface grade) − PEN[k], 0, 100) / 100
cost: S,A=0  B=10  C=20  D=30  E=40  F=60  G=90      PEN[k] = 0,0,5,20,30,50 for streak position k = 1..6+
```

(Earlier versions used 10 points per grade and PEN = 0,0,10,25,35,50; the algorithm tests run on that frozen data in `tests/fixtures/data_v1.json`, and `win_table_matches_community_table` checks the shipped data against every cell of the new table.)

- **Streak position k:** counts consecutive turns with *any* race, goal races included. A rest resets it.
- **Win probability per race:** P_a(r) = 1 − Π(1 − p) over uma a's attempts at race r. A race can be run in Classic and Senior; it still counts once.

**Objective (expected affinity per 4 runs):**

```
E = base + 3 · Σ_{i<j} C_ij · ⟨P_i, P_j⟩          ⟨P_i,P_j⟩ = Σ_r P_i(r)·P_j(r) = expected shared G1 wins
```

| Scheme | `scheme` | C_ij | base |
|---|---|---|---|
| Self-copy-free 12-lineage breeding | `"scf"` | 10/3 for every pair | 2·Σ pair + 4·Σ trio |
| Rotation (parents = previous two runs) | `"rot"` | 3 adjacent, 4 diagonal (3 orders tried) | 2·Σ pair + 3·Σ trio |

- `pair(a,b)` / `tri(a,b,c)`: the sum of `succession_relation.relation_point` over the relation groups shared by all of them.
- **Fixed rules** (applied by the exporter):
  - G1 only, +3 per shared G1.
  - Relocated-venue G1s count as the normal race: 1025→1012, 1027→1006, 1028→1005.
  - `determine_race = 2` goal alternates are excluded.
  - Goals sharing a `sort_id` are alternatives; at least one must be run.

**Assumptions:**
- Schedules are fixed in advance.
- Stats and training turns aren't modelled.
- Two umas' runs are independent, so E[shared wins] = ⟨P_i, P_j⟩.

## 2. Algorithms

| Function | What it does | Exact? |
|---|---|---|
| `dp_exact` | Best response for one uma: maximize Σ_r W(r)·P(r) given partner weights W. Junior turns 1–24 run in sequence. Then Classic turn t and Senior turn t+24 are decided **together**, with state (Classic streak, Senior streak, goal bitmask), plus the Classic streak at turn 48 for the 3 umas with goals on turn 49. Every race offered in both years is exactly 24 turns apart, so 1 − (1−p₁)(1−p₂) is computed exactly. About 0.1 ms. | **Yes** (validated, §4) |
| `ascent` | Best-response dynamics: each member re-solves against the other three until none improves. It converges to a Nash equilibrium of an identical-interest game, not necessarily the global optimum. | Local |
| `group_moves` | Coalition deviations: force 2+ members onto the same race (or rest) at one turn, then re-ascend. Escapes coordination traps, e.g. all four moving Hopeful → Zen-Nippon together, which no single member gains from alone. | Local |
| `optimize` | Optimistic start (partners at best-case odds) → ascent → group moves → `iters` rounds of iterated local search (1–2 members re-solved against noisy weights, then ascent and group moves). Rotation tries all 3 orders. | Heuristic |
| `run_best` | Computes exact pairwise ceilings PB(a,b) = min(BR_a(PM_b), BR_b(PM_a)), where PM is a partner's best-case odds per race, and from them a guaranteed ceiling UB(F) ≥ E for every foursome F. Then searches foursomes in descending-UB order until the next ceiling is below the current k-th best score. | Screen is **guaranteed**; ranking among the searched survivors is heuristic |

**Why the ceiling is valid:**
- P_b ≤ PM_b elementwise and P_a ≥ 0, so ⟨P_a, P_b⟩ ≤ ⟨P_a, PM_b⟩ ≤ BR_a(PM_b).
- Take the min over both sides, then sum over pairs with weights C.

A foursome whose ceiling is below a score already achieved cannot beat it.

## 3. Usage

```bash
# 1. export data (needs Python 3 + the global master.mdb next to the export scripts)
cd export && uv run build_base.py && uv run export_rs.py            # writes ../data.json
# alternate outfits: e.g. [Fiery Aqua Vitae] (Christmas) Vodka
OUTFIT=1008:100802 uv run build_base.py && OUT=../data_xmas_vodka.json uv run export_rs.py

# 2. build
cargo build --release
# optional, after any export: store the exact pairwise ceilings in data.json ("pb", with a fingerprint of the
# umas/goals/penalties; ignored when stale) so searches skip computing them (../../build.sh pb does both eras)
uv run export/add_pb.py target/release/loopopt data.json

# 3. run a job file
target/release/loopopt data.json jobs/best_no_oguri_yukino_inari.json > result.json
```

### Job file (JSON)

| Field | Meaning | Default |
|---|---|---|
| `mode` | `"best"` (screen + search), `""` (optimize listed foursomes), `"score"` (score given schedules), `"legacy"` (per-slot affinity of every lineage for given schedules), `"br"` (best-response test), `"pb"` (ceiling table) | `""` |
| `scheme` | `"scf"` or `"rot"` | required |
| `owned` | uma ids you own (empty = whole roster) | `[]` |
| `exclude` | uma ids to leave out | `[]` |
| `gp_weight` | grandparent spark base rate as a fraction of parents' (community estimate 0.5). The objective becomes parent-slot affinity + w × grandparent-slot affinity. 1 = plain total affinity | `1` |
| `require` | uma ids every returned loop must contain (at most 4; added to the pool even if not in `owned`) | `[]` |
| `top` | how many loops to return in `best` mode | 5 |
| `iters` | iterated-local-search rounds per foursome | required (30 is plenty) |
| `seed` | RNG seed | required |
| `margin` | group-move pre-screen: skip re-ascent when the forced state is this far below the current best | 60 |
| `threads` | worker threads | 2 |
| `markdown` | if set, also write a readable sheet to this path (see *Schedule sheet* below) | `""` |
| `jobs` | for `""` / `"score"` modes: `[{"members":[id,id,id,id], "init":[[[turn,program_id],...] x4]}]` | `[]` |

Example (`best`, owned list):

```json
{"mode":"best","scheme":"scf","iters":30,"seed":3,"threads":8,"top":5,
 "owned":[1008,1009,1014,1015,1017,1019,1023,1030,1069]}
```

`best` output has:
- `top[]`: `members`, `order`, `score`, `base`, `g1_bonus`, `ceiling`, and each uma's `races` as `[turn, program_id]`.
- Screen counts: `foursomes`, `optimized`, `ruled_out_by_ceiling`, `next_unoptimized_ceiling`.
- `timing_s`.

Every schedule also has a `detail` list: `turn`, `when` (e.g. `Cl L-May`), `race`, `program`, `g1`, `goal`, `streak`, `win`.

Example sheet: `jobs/sheet_vodka.json` writes `results/vodka_daiwa_pasa_digital.md`.

Turn = 24·year + 2·(month−1) + half: Junior 1–24, Classic 25–48, Senior 49–72; half 1 = Early, 2 = Late.

### Schedule sheet (`markdown`)

The schedule chart has two sections, rows in calendar order, one column per uma (required non-G1 goal races are not listed):
- **Shared G1s:** races (by identity, in either year) run by 2+ members. These are the affinity. A row with one runner is a second attempt at a race the others run in the other year; it raises that uma's P(win at least once).
- **Optional G1s:** run by one member only. They have no affinity effect and can pass down race sparks. Skipping one never lowers affinity, since it can only shorten a streak.

A number is the win % for that attempt, streak penalty included; `*` marks a goal; `·` means not running it. Below the chart: P(win at least once) per G1 per uma, and expected shared G1 wins per pair.

**Optional fill pass:** after optimizing, each member re-solves against her partners with a tiny bonus (1e-7 per expected G1 win), and the result is kept only if the affinity score doesn't drop by more than 1e-6. That adds every G1 that costs nothing. Test `optional_g1s_cost_nothing` checks that stripping them leaves the score unchanged.

### Grandparent weighting (`gp_weight`)

Spark proc chance per legacy = base × (1 + affinity/100), and grandparent base rates are about half of parents' (community estimate). Using umaishow's per-legacy split, the objective with grandparent weight w is:

| Scheme | base | C (race weight) |
|---|---|---|
| scf | 2·Σ pair + 4w·Σ trio | (2 + 8w)/3 for every pair |
| rot | 2·Σ pair + 3w·Σ trio | 1 + 2w adjacent, 4w diagonal |

- w = 1 reproduces the plain model (10/3; 3 and 4).
- For scf, C stays uniform across pairs, so the optimal schedules for a given foursome don't change; only the ranking across foursomes does.
- For rotation with w = 0.5, adjacent and diagonal pairs weigh the same (2 and 2).
- Test `gp_weight_matches_per_legacy_sum` checks these closed forms against a direct sum over every lineage and slot.

`"mode":"legacy"` prints each lineage's six slot affinities (expected values, no rounding), for computing per-spark proc chances.

### Skill-target search (`"mode":"skills"`)

Ranks loops by the chance that every target skill lands as a spark on the new veteran. This is the solver behind the web page.

```bash
target/release/loopopt data.json jobs/skills_unity8.json > result.json     # run from this folder (skills_file is relative)
```

```json
{"mode":"skills","scheme":"scf","iters":30,"seed":3,"skills_file":"web_skills.json","top":5,
 "exclude":[1021,1034],
 "targets":[{"group":20159},{"group":20049,"guaranteed":2},{"group":20166,"guaranteed":2},{"group":20046,"guaranteed":2},
            {"group":20161,"deck_white":0.54},{"group":21010},{"group":20165,"deck_gold":0.54},{"group":20203,"guaranteed":2}],
 "extra":[[1078,20165,1],[1003,20190,1]]}
```

| Field | Meaning | Default |
|---|---|---|
| `skills_file` | outfit skill lists + skill catalog (`web_skills.json`, from `export/export_web.py`) | required |
| `targets[]` | `group` = skill group id; `guaranteed` 0 none / 1 white / 2 gold; `deck_white`, `deck_gold` = deck hint chance 0–1 | required |
| `star_dist` | spark star mix on the legacies `[p1★, p2★, p3★]` | `[0.50,0.45,0.05]` (rank score below 17,500; `[0.20,0.70,0.10]` from 17,500 up) |
| `owned`, `exclude`, `require` | as in `best` | `[]` |
| `exclude_outfits` | outfit (card) ids you don't own | `[]` |
| `extra` | `[uma id, skill group, level]` skills an uma always gets (e.g. Zephyr Slipstream, Teio Head-On) | `[]` |
| `top` | loops returned | 5 |

Output `loops[]`: `members`, `outfits` (the outfit each uma uses and which targets it covers), `p_all` (all targets land), `p_minus1` (all but one), `per_trainee`, `mean_proc`, `bound`, and `schedule` (chart rows: `race`, `when`, `shared`, `cells` with win % and goal flag, `runners`). Top-level: `proven_top` (no unsearched loop can beat #1), `foursomes`, `searched`.

### Deck + loop search (`"mode":"deck"`)

Finds the support deck and the loop together. Needs `skills_file`, `cards_file` (`web_cards.json`, from `export/export_cards.py`) and usually `events_file` (`card_events.json`, from `export/build_card_events.py` over the cached GameTora files in `gametora/`).

```bash
target/release/loopopt data.json jobs/deck_unity8.json        # everyone owned
target/release/loopopt data.json jobs/deck_unity8_noti.json   # no Tamamo / Inari
```

Extra fields (on top of the skills-mode fields; scenario-guaranteed skills stay `guaranteed: 2`, everything the deck can give is left at 0):

| Field | Meaning | Default |
|---|---|---|
| `owned_cards` | card ids you own (empty = all) | `[]` |
| `borrow` | allow one card you don't own (friend card); the search picks it | `false` |
| `exclude_cards` / `lock_cards` | cards never / always used | `[]` |
| `top_decks` | distinct decks returned, each with its best loop (with `per_uma`: loops returned) | 1 |
| `per_uma` | one deck per uma: each trainee runs her own best legal deck, the same in all three lineages she heads | `false` |
| `pals` | only decks with exactly this many pals | any |
| `schedule` | experiment: `"solo"` (each uma maximises her own race wins), `"goals"` (career objectives only) or `"none"` (shared wins ignored) instead of loop-optimised schedules | optimised |
| `lambda` | mean chain events per run | 7.1 |
| `min_rarity` | lowest rarity for cards with chain events (2 = SR); cards without chain events (R cards) are always considered, drop them with `exclude_cards` | 2 |
| `events`, `events_p`, `odds_events` | inline event rewards (merged with `events_file`) | `[]` |

Model: training hints 1 + hint_frequency/25 per run (hint frequency at max limit break, unique effect included), uniform over the card's hint options; chain events Poisson(lambda), each advancing a random remaining chain slot of a non-pal card; pal finales certain; outcome-dependent rewards at their share of the outcomes, k of n at k/n (e.g. Anshinzawa SSR's finale: Nothing Ventured on success, Risky Business on failure with choice 2, 50% each; Nakayama Festa [Trusting the Dice]: each in 2 of 4 outcomes, 50%). Cards without chain events (R cards) take no chain slot, like pals, and give training hints only. A choice that ends a card's chain (GameTora's 'ee' reward) is offered as a separate deck entry: the card's earlier links as usual, the ending choice's rewards at that link, and its remaining chain slots freed for the other cards; choices that end the chain on only one outcome are left out. A skill offered at several links of one card counts once, from the earliest link that fires. The search is exact for the model:
- decks are compared on (P(gold hint), P(any hint)) per target, which the landing odds rise with; one group for every pal count, so a deck that `top_decks` others match or beat on every coordinate is dropped (the per-pal-count `best` list is indicative; rerun with `pals` for an exact answer at one pal count);
- decks are enumerated one chain-kind key at a time (the key fixes every card's chain odds). Under a key, a card that another card beats on every target and can legally replace (same kind, character rules, borrow limit) gives a swapped deck at least as good; decks with `top_decks` such swaps are skipped, branches whose swaps survive every later addition are cut, and cards with enough such dominators never enter that key's decks;
- with `top_decks` > 1, a first pass finds the decks no other deck beats, and the full pass drops any deck that `top_decks` of them beat;
- foursomes are generated in bands of a separable bound (each uma heads 3 of the 12 lineages, so a loop's ceiling is at most a quarter of the sum of its members' best per-trainee values at their highest possible inheritance chance); only those that can still beat the `top_decks`-th best get a real ceiling (each trainee with its own best deck at a rounded-up inheritance chance), and the best-first search solves schedules until no ceiling can win.

Character rule: no trainee runs a card of her own character, or a group card she belongs to (`charas` in web_cards.json: the card's character plus the group's members). With one shared deck the search first ignores the rule; when a returned deck blocks one of its loop's members u, the space is split into loops without u and loops with u whose decks hold no card blocking u, and both halves are searched again until every answer obeys the rule (a half whose unconstrained best cannot beat the current k-th legal result is dropped). Most runs need one search; checked against brute force over every deck on small owned sets (`val/owncheck.py`).

Per-uma decks (`per_uma`): a loop's value is fixed/12 · Σ over the four umas of the best legal deck's summed value over her three lineages. Every uma's best deck lies on the skyline of decks no deck beats on every coordinate, thinned per class of umas whose outfits leave the same targets open (a target her outfits always give as gold doesn't matter to her). Where a thinned list holds a deck that blocks the uma, her legal decks come from a second deck search without the blocking cards, run the first time a foursome with her is solved. Ceilings use the whole skyline, which no legal deck beats, so the answer is exact; checked against brute force (every foursome × every deck) in `val/permcheck.py`.

"Proven" is relative to the schedules the optimiser finds for the loops it solves.

Typical time natively, R cards included: under 1 s for one deck with the Unity Cup golds guaranteed, 3–11 s with none guaranteed (Global; GM era about 3 s), up to about 100 s for three decks; the browser is about 1.5× slower. Dominance tests use a 64-bit signature (16 coordinates × 4 bits) as a fast necessary check before the exact comparison.

Skills-mode model: a target is acquired if guaranteed, on the trainee's own outfit skill list, hinted by the deck, or inherited (proc = 1 − Π(1 − base·(1 + affinity/100))² over the six legacies, base 4.65% below rank score 17,500 (stars 50/45/5%), 5.7% from 17,500 up (20/70/10%), from hakuraku.moe's room-match data; it was 4.8% from a 45/50/5 split that matched neither band). Each legacy's affinity follows uma-it-optimizer's fit to 8,155 Independent Training receipts (github.com/Acrith/uma-it-optimizer, `tools/loop/affinity.py`, `inspiration_rates.py`): a parent P counts pair(T,P) + pair(P1,P2) + trio(T,P,G) for each of its own parents G + 3 per G1 shared with the other parent and with each G; a grandparent G under P counts trio(T,P,G) + 3 per G1 shared with P; grandparents get no extra factor (halving them predicted half of what fired). In a loop P's own parents are the other parent and the fourth member, so the parents' shared G1s count for both terms. Before this, the model used umaishow's split (a parent took half the parent–parent relation and shared G1s) with grandparents halved, which put every spark chance at about half the measured level. An acquired skill becomes a spark 35.4% of the time as a white, 70.9% as a gold (20% / 40% ×1.1 per ancestor already carrying it, all six in a running loop; fit to CM room data at hakuraku.moe/notes/spark_generation). Odds are averaged over the 12 lineages.

## 4. Validation

`cargo test --release` runs 16 tests in under 1 s:
- **Reference fixtures** (`tests/fixtures/`):
  - 200 best responses from an exact MILP, including forced races;
  - 71 rest-forced best responses from the MILP, including turns 24/25/48/49/50 for the turn-49 goal umas;
  - the full 2,278-pair ceiling table;
  - 23 schedules scored by the Python reference.
- **Properties checked:**
  - ceilings bound both optimized and truly random feasible schedules (both schemes, all 3 rotation orders);
  - the best-first stop rule;
  - `feasible()` rejects bad schedules;
  - a force that conflicts with a goal is infeasible;
  - schedule parsing rejects duplicate or out-of-range turns;
  - `top = 0` is rejected;
  - known optima are reproduced.

An independent adversarial review (separate agent, its own MILP and SQL, 576 best-response cases, 6,400 ceiling checks, a full data export cross-check) passed every core claim. It found 3 interface bugs, now fixed and tested:
1. A force could override a goal in `br` mode.
2. `top = 0` panicked.
3. Score mode accepted duplicate turns.

The load step now asserts the structural fact the exact DP relies on: every race offered twice is a Classic/Senior pair exactly 24 turns apart.

**Reproduce:**
- `cd validation && uv run check_scores.py` checks score parity; it needs only the built binary.
- `dp_vs_milp.py` (best response vs MILP) needs scipy plus the `export/` modules and master.mdb on the path.
- `independent_check.py` is the spec-only checker used on `results/final_loops.json`.


| Check | Result |
|---|---|
| Scoring vs the Python reference: 32 known schedules (scf + rot, both sessions' outputs); 23 of them ship in `validation/known_schedules.json` | max \|diff\| 4.5e-13 |
| `dp_exact` vs exact MILP (scipy/HiGHS), random weights, 40 umas × 4 weight types | 160/160 equal |
| `dp_exact` vs MILP, all 68 umas, incl. forced group moves, turn-49 goal umas (Meisho Doto, Mejiro Palmer, Haru Urara) and multi-turn goal alternatives (Fuji Kiseki, Fine Motion, Fukukitaru, Chiyono O) | 192/192 equal, max \|gap\| 3.4e-13 |
| Ceiling table `pb` vs the Python MILP table (2,278 pairs) | max \|diff\| 1.1e-14 |
| Cold-start search reproduces every known best loop: 11 scf + 3 rot | all reproduced exactly (2058.92, 2011.30, 2008.51, 1961.02, 1958.00, 1956.38, 1948.98, 1945.95, 1942.37, 1941.28, 1936.89; rot 1944.49, 1915.98, 1833.03) |
| Deep search (200 rounds, no group-move pre-screen) on 6 contenders | no value exceeded the known optima |
| Independent checker (written from the spec without seeing this code): 12 final loops | all feasible; scores match to 4.5e-13 |
| Note | The "random schedule" tests in the first suite used DP-optimal schedules for random weights. Genuinely random feasible schedules are covered by `ceilings_bound_truly_random_schedules`. |

## 5. Benchmarks (2-core container)

| Operation | Python (MILP) | Rust v1 | Rust v2 (current) |
|---|---|---|---|
| Exact best response, one uma | 110 ms | 35–40 µs | ~5.5 µs, plus a memo: ~99% of calls inside a search are repeats |
| Ceiling table, all 2,278 pairs | 387 s | 0.18–0.25 s | 0.04 s |
| Full search, one foursome (scf, 30 rounds) | ~6–7 min | 1.6–2.3 s | ~0.03–0.05 s |
| Whole pool (screen + search) | hours | 14–44 s | **0.4–1.2 s** |

v1 → v2 is byte-identical on every job tested (only timing fields differ). The changes:
- `dp_exact`: precomputed per-option win tables, no per-call allocation, and loops inverted so choice pairs are outer and states inner, which keeps the tie-breaking order.
- `ascent` skips re-solving members whose partners haven't changed.
- A best-response memo per `optimize` call, keyed on exact weight bits.
- `group_moves` reuses probability vectors.

Rejected: a speculative work queue in `best`, which was slower at the current speed.

## 6. Limits

- **The ranking among searched survivors is heuristic.** The search reproduces every known optimum and the deep search found nothing better, but no proof is given that a survivor's true optimum isn't higher. The ceilings are 30–110 points loose, so certifying would need a much tighter relaxation.
- **Model assumptions (§1) are user-supplied or community-derived:** the win formula and the affinity-only objective. The skill modes' per-legacy affinity is uma-it-optimizer's receipt-fitted model; its exact parent terms are fitted on totals only, so the split between them is less certain than the overall level. The code doesn't validate them.
- **Junior G1s need 1,000 fans and a debut win gives 700.** Unless a fan bonus applies, add one extra Junior race the uma wins, before Early Nov. This doesn't change affinity.
- **Base-outfit aptitudes by default.** Use `OUTFIT=chara:card` for alternate outfits (Vodka 100802, Daiwa 100902, Urara 105202); this needs a re-export.
