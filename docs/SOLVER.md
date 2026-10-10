# The solver (loopopt)

The Rust crate in `pkg/loopopt/`; paths below are relative to it. Building and running it: `docs/BUILD.md`.

Finds the best parent loop (a four-uma loop or a three-uma rotation) and support deck for landing target skills in Uma Musume (Global), with the exact race schedule for each uma, and plans careers to a looped parent (`/career`). It reads only the game's plaintext `master.mdb` and does math; nothing in the game is modified.

```
master.mdb ──(export/export_rs.py)──▶ data.json ──(loopopt)──▶ ranked loops + schedules (JSON)
```

---

## 1. Model

**Win chance** of one attempt (community table, Oct 2026):

```
p = clamp(110 − cost(distance grade) − cost(surface grade) − PEN[k], 0, 100) / 100
cost: S,A=0  B=10  C=20  D=30  E=40  F=60  G=90      PEN[k] = 0,0,5,20,30,50 for streak position k = 1..6+
```

(Earlier versions used 10 points per grade and PEN = 0,0,10,25,35,50; the algorithm tests run on that frozen data in `tests/fixtures/data_v1.json`, and `win_table_matches_community_table` checks the shipped data against every cell of the new table.)

- **Streak position k:** counts consecutive turns with *any* race, goal races included. A rest resets it.
- **Turn** = 24·year + 2·(month−1) + half: Junior 1–24, Classic 25–48, Senior 49–72; half 1 = Early, 2 = Late.
- **Win probability per race:** P_a(r) = 1 − Π(1 − p) over uma a's attempts at race r. A race can be run in Classic and Senior; it still counts once.

**Objective (expected affinity per 4 runs):**

```
E = base + 3 · Σ_{i<j} C_ij · ⟨P_i, P_j⟩          ⟨P_i,P_j⟩ = Σ_r P_i(r)·P_j(r) = expected shared G1 wins
```

| Scheme | `scheme` | C_ij | base |
|---|---|---|---|
| Self-copy-free 12-lineage breeding | `"scf"` | 10/3 for every pair | 2·Σ pair + 4·Σ trio |
| Rotation (parents = previous two runs) | `"rot"` | 3 adjacent, 4 diagonal (3 orders tried) | 2·Σ pair + 3·Σ trio |

The deck search optimises loop schedules under `"scf@0.5"` (*Grandparent weighting*, §3); `rot` is kept in the
affinity core and its tests.

- Aptitudes (career page): each uma carries `apt` (ten codes 1..8 = G..S: short, mile, middle, long, turf, dirt, front runner, pace chaser, late surger, end closer; her first outfit), and `race_cat` / `race_ground` give each race's distance class (0 short .. 3 long) and surface (1 turf, 2 dirt); the loop planner reads neither (its `bp` per option is unchanged), the career schedule uses them for the legacies' pink sparks (`Env::pink_career`, MODEL.md).
- `pair(a,b)` / `tri(a,b,c)`: the sum of `succession_relation.relation_point` over the relation groups shared by all of them. `data.json` stores the groups (`relations`: `[points, [roster indices]]`, groups with at least two roster members) and the solver rebuilds both tables at load; older files with stored `pair` / `tri` tables still load.
- **Fixed rules** (applied by the exporter):
  - G1 only, +3 per shared G1.
  - Relocated-venue G1s count as the normal race: 1025→1012, 1027→1006, 1028→1005.
  - `determine_race = 2` goal alternates are excluded.
  - Goals sharing a `sort_id` are alternatives; at least one must be run.
  - Turns in `single_mode_race_restrict_turn` (set by a story event of her career) keep only her goal races: Yamanin Zephyr
    races nothing at turn 44 (Classic late October), Sweep Tosho nothing at turn 36 (Classic late June), Matikanetannhauser
    only her Japan Cup goal at turn 70. JP adds rows for more umas (Daitaku Helios, Tanino Gimlet, Katsuragi Ace among them).

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
| `pair_ceilings` | Exact pairwise ceilings PB(a,b) = min(BR_a(PM_b), BR_b(PM_a)), where PM is a partner's best-case odds per race: no pair of schedules shares more expected G1 wins. The deck search bounds every structure's lineage affinities with them before any schedule is optimised; `loopopt pbtable` stores the table in data.json. | **Guaranteed** bound |

**Why the ceiling is valid:**
- P_b ≤ PM_b elementwise and P_a ≥ 0, so ⟨P_a, P_b⟩ ≤ ⟨P_a, PM_b⟩ ≤ BR_a(PM_b).
- Take the min over both sides.

A structure whose bound is below a result already achieved cannot beat it.

## 3. Usage

```bash
# 1. export data (needs Python 3 + the global master.mdb next to the export scripts)
cd export && uv run build_base.py && uv run export_rs.py            # writes ../data.json
# alternate outfits: e.g. [Fiery Aqua Vitae] (Christmas) Vodka, into a data file of its own
OUTFIT=1008:100802 uv run build_base.py && OUT=../data_xmas_vodka.json uv run export_rs.py

# 2. build
cargo build --release
# optional, after any export: store the exact pairwise ceilings in data.json ("pb", with a fingerprint of the
# umas/goals/penalties; ignored when stale) so searches skip computing them (../../build.sh pb does both eras)
uv run export/add_pb.py target/release/loopopt data.json

# 3. run a job file (from this folder: the file paths in a job are relative to it)
target/release/loopopt data.json jobs/pal_U.json > result.json
```

The CLI runs `"mode":"deck"` and `"mode":"career"` job files (below), the jobs the page's worker sends to the wasm
build (`src/web.rs`: `init`, `deck`, `career`, `state_out`, `state_in`), and `loopopt pbtable data.json`, which prints
the pairwise-ceiling table `export/add_pb.py` merges in.

### Schedule chart

A result's `schedule` (`skills::chart_rows`) lists races in calendar order, one cell per member (required non-G1 goal
races are not listed):
- **Shared G1s:** races (by identity, in either year) run by 2+ members. These are the affinity. A row with one runner is a second attempt at a race the others run in the other year; it raises that uma's P(win at least once).
- **Optional G1s:** run by one member only. They have no affinity effect and can pass down race sparks. Skipping one never lowers affinity, since it can only shorten a streak.

A cell carries the win % for that attempt (streak penalty included), the goal flag, the aptitude cost and the streak
position; `alts` names races that can replace the row's at no cost (`interchangeable`).

**Optional fill pass:** after optimizing, each member re-solves against her partners with a tiny bonus (1e-7 per expected G1 win), and the result is kept only if the affinity score doesn't drop by more than 1e-6. That adds every G1 that costs nothing. Test `optional_g1s_cost_nothing` checks that stripping them leaves the score unchanged.

### Grandparent weighting

Spark proc chance per legacy = base × (1 + affinity/100), and the old community estimate put grandparent base rates at half of parents' (receipt fits since say full rate: `docs/MODEL.md`). Using umaishow's per-legacy split, the objective with grandparent weight w (scheme string `scf@w` / `rot@w`) is:

| Scheme | base | C (race weight) |
|---|---|---|
| scf | 2·Σ pair + 4w·Σ trio | (2 + 8w)/3 for every pair |
| rot | 2·Σ pair + 3w·Σ trio | 1 + 2w adjacent, 4w diagonal |

- w = 1 reproduces the plain model (10/3; 3 and 4).
- For scf, C stays uniform across pairs, so the optimal schedules for a given foursome don't change; only the ranking across foursomes does. The deck search uses `scf@0.5`.
- For rotation with w = 0.5, adjacent and diagonal pairs weigh the same (2 and 2).
- Test `gp_weight_matches_per_legacy_sum` checks these closed forms against a direct sum over every lineage and slot.

### Fields shared by deck and career jobs

| Field | Meaning | Default |
|---|---|---|
| `skills_file` | outfit skill lists + skill catalog (`web_skills.json`, from `export/export_web.py`) | required |
| `targets[]` | `group` = skill group id; `guaranteed` 0 none / 1 white / 2 gold; `deck_white`, `deck_gold` = deck hint chance 0–1 | required |
| `star_dist` | spark star mix on the legacies `[p1★, p2★, p3★]` | `[0.50,0.45,0.05]` (rank score below 17,500; `[0.20,0.70,0.10]` from 17,500 up) |
| `owned` | uma ids you own (empty = whole roster) | `[]` |
| `exclude` | uma ids to leave out | `[]` |
| `require` | uma ids every result must contain (at most the largest of `sizes`; added to the pool even if not in `owned`) | `[]` |
| `exclude_outfits` | outfit (card) ids you don't own | `[]` |
| `extra` | `[uma id, skill group, level]` skills an uma always gets (e.g. Zephyr Slipstream, Teio Head-On) | `[]` |
| `iters` | iterated-local-search rounds per structure's schedules | 30 |
| `seed` | RNG seed of the schedule search | 3 |
| `sizes` | structure sizes searched, ranked together: 3 (three-uma rotations, see *Three-uma rotations*) and/or 4 (four-uma loops); `require` may list at most the largest size, and four required umas leave only loops | `[3, 4]` |

A deck result's `loop`: `members` (3 or 4), `outfits` (the outfit each uma uses and which targets it covers), `p_all` (all targets land), `p_minus1` (all but one), `landed` (the chance of exactly k targets, k = 0 ..= n, averaged over the lineages; the deck search's exact over each lineage's landed sets, so `landed[n]` = `p_all` and the top two sum to `p_minus1`), `per_trainee`, `mean_proc`, `rank_value`, `schedule` (*Schedule chart*) and `detail` (per lineage: the six legacy slots' affinity parts, q, each target's odds).

### Deck + loop search (`"mode":"deck"`)

Finds the support deck and the loop together. Needs `skills_file`, `cards_file` (`web_cards.json`, from `export/export_cards.py`) and usually `events_file` (`card_events.json`, from `export/build_card_events.py` over the cached GameTora files in `gametora/`).

```bash
target/release/loopopt data.json jobs/deck_unity8.json        # everyone owned
target/release/loopopt data.json jobs/deck_unity8_noti.json   # no Tamamo / Inari
```

Extra fields (on top of the shared fields above; scenario-guaranteed skills stay `guaranteed: 2`, everything the deck can give is left at 0):

| Field | Meaning | Default |
|---|---|---|
| `owned_cards` | card ids you own (empty = all) | `[]` |
| `borrow` | allow one card you don't own (friend card); the search uses it only when it makes the deck better (the pages always send `true`) | `false` |
| `exclude_cards` / `lock_cards` | cards never / always used | `[]` |
| `top_decks` | distinct decks returned, each with its best loop (with `per_uma`: loops returned) | 1 |
| `per_uma` | one deck per uma: each trainee runs her own best legal deck, the same in all the lineages she heads (three in a loop, one in a rotation) | `false` |
| `pals` | only decks with exactly this many pals | any |
| `schedule` | experiment: `"solo"` (each uma maximises her own race wins), `"goals"` (career objectives only) or `"none"` (shared wins ignored) instead of loop-optimised schedules | optimised |
| `scenario` | Independent Training scenario: `unity`, `gc` (Grand Concert), `ura`, `trackblazer`, `grandmasters` (JP); sets the training-hint base (Grand Masters, with no runs to fit, uses a 2.30 placeholder) | `unity` |
| `chain_events` | `[n, p]`: chain events per run ~ Binomial(n, p) (experiments) | `[21, 0.35]` |
| `cap_audit` | audit: every exact cap at 2.0 instead of `EXACT_CAPS`, and ten times the band limit `EXACT_CAP` (deck and career jobs; `val/capaudit.py`) | `false` |
| `min_rarity` | lowest rarity for cards with chain events (2 = SR); cards without chain events (R cards) are always considered, drop them with `exclude_cards` | 2 |
| `events`, `events_p`, `odds_events` | inline event rewards (merged with `events_file`) | `[]` |
| `alts` | events naming two or more skills, in full: `[card, link, [choice][outcome] -> [[group, 1 \| 2], ...]]` (from card_events.json `alts`); at these links they replace `events` / `events_p` | `[]` |

Model (the rates and their evidence: `docs/MODEL.md`, *Deck side*):
- **Training hints**: h = base(scenario) x (1 + hint_frequency/100) draws per card per run (`hint_base`; hint
  frequency at max limit break, unique effect included; the floor or ceiling of h, weighted by the fraction), each
  uniform over the card's skill options still below Lv 5 (`nskill`; the stat option is never drawn; a skill leaves the
  pool after ceil(5 / `hinc`) draws, `hinc` = hint levels per draw at max limit break, so a +5 card never repeats a
  skill); one card's draws over several targets are joint (`miss_draws`). Cards without chain events (R cards) take no
  chain slot, like pals, and give training hints only.
- **Chain events**: Binomial(21, 0.35) per run (`CHAIN_N`, `CHAIN_P`; `chain_events` overrides), each advancing a
  random remaining chain slot of a non-pal card. Pal finales are certain.
- **Outcome shares**: an outcome-dependent reward counts at its share of the outcomes, k of n at k/n (Nakayama Festa
  [Trusting the Dice]: each skill in 2 of 4 outcomes, 50%). A measured share replaces k/n where one exists: `chance` in
  card_events.json for hand-entered entries (Anshinzawa SSR's finale, Riko Kashimoto SSR's Ramp Up), and
  `event_shares.json` for generated chain events, which the builders turn into the chain shares and `alt_w`, the job
  field the solver uses as outcome weights in place of equal outcomes.
- **Early chain ends**: a choice that ends a card's chain (GameTora's 'ee' reward) is offered as a separate deck
  entry: the card's earlier links as usual, the ending choice's rewards at that link, and its remaining chain slots
  freed for the other cards. Choices that end the chain on only some outcomes are not offered, but when such a choice
  is IT's default (first) choice the chain stops there with chance k/n (`stop_risk` in card_events.json; the chain
  odds are the exact mixture over where it stops, freed slots going to the other cards).
- **Links and choices**: a skill offered at several links of one card counts once, from the earliest link that fires.
  An event's outcomes exclude each other, and where an event offers targets on different choices the card is offered
  once per choice worth taking (a choice no other beats on every target), like an early chain end; the deck output
  names it (`choices`: "link 1: Uma Stan", the skill to prioritise). A hand-entered pal or group entry may name the
  event and choice it models (`event`: [story id, choice]); the event builders then derive `prioritise` from GameTora's
  choice layout and IT's rule (the skills on that choice and on no choice above it; a choice nothing steers to is an
  error), and the page shows it on the deck card (Anshinzawa SSR: Risky Business).
- **Gold and white**: a card's gold and white of one skill are combined exactly (deck `odds`: `gold` = P(gold hint),
  `white` = P(white hint | no gold hint)).

Exact odds. Targets do not land independently: chain cards share one run's chain events (one binomial count), so their rewards land or miss together, and an event's outcomes can give one target or another. A deck's exact value for a trainee at inheritance chance q, V(u, q, D), is computed over the joint distribution of chain progress, given which every source is independent except outcome groups touching two or more targets, which are enumerated per cluster of targets they join (`joint_fast`; `joint_trainee_ref` is the full enumeration, and `JOINT_CHECK=1` runs both on every call). The search ranks loops and decks by exact values. The product of per-target odds, V̂ (the old ranking value, `rank_value` in the output), is only used to bound and order the work:
- **Assumed caps, per structure class.** No deck's exact value exceeds ρ times its product value, V ≤ ρ·V̂, with ρ by the deck's class (`EXACT_CAPS`, `cap_class`): its chain-varying targets are those a chain link of one of its cards feeds (`Src::cm`); at most two and no multi-target group (`Src::multi`) ρ = 1.05; three or more, or a multi-target group, 1.25 = `EXACT_RHO`. All assumed from measurement: chain cards share the run's chain events and compete for them, which can correlate even one target's feeders (no deck with at most one such target has been scored yet). Largest ratios seen on any deck scored: two 1.008, three 1.127 (King Halo, `val/plan/boot_mob`), more 1.18 (`pal_UF`; 1.17 on Grand Masters). A search node takes the class of its cards plus every candidate left (a superset of any completion's), so its cap is at least any completion's; a deck passes at its own. Ceilings that know no deck (`Env::ceiling`, the loop tables' product fallback, the reroll's `y_hi`) take 1.25. Every deck and loop whose product ceiling times its cap could beat the best exact value is examined. Output: `class_caps` and per class `class_max_ratio` / `class_max_deck` (the deck that set it); `val/cmp2.py` and `val/planval.py` fail when a scored deck's ratio passes 1 + 0.75 (cap - 1). Job field `cap_audit: true` sets every class cap to 2.0 and ten times the band limit (slow; `val/capaudit.py` compares such runs with normal ones: about 37 s for its three default jobs).
- **State-conditioned bound** (career searches' band pass, at each configuration's root): over the configuration's
  chain progress states x, P(x) times the product over targets of each target's factor at its lowest possible misses
  given x (per open slot the best entry of its group at that slot's level); proven >= the exact value of every deck of
  the configuration (given x, sources are independent and one group's per-target hints negatively associated), so a
  configuration below the bar is dropped without the cap. An extra prune beside the cap's (outputs unchanged);
  `SBOUND` 0 turns it off, `SB_CHECK=1` checks the value against every exact scoring and the node bounds against the
  leaves below them.
- **Every legal deck.** `DeckSpace` enumerates every deck legal for the trainee (for all four members with a shared deck: the character rule is a legality filter, no splitting), by chain-kind key, before card dominance drops any card; cards interchangeable for the trainee (same rewards on the targets her outfits leave open, or none) are merged; a partial deck is dropped when the best its completions could reach is below the threshold: per (chance, outfit) the lower of the per-target ideal point (the r lowest miss chances per target in each kind group) and a submodular bound (each card's ratio P({card})/P({}) for the product of one outfit at one chance, which gains less the more the deck holds; the best ratios of the cards still to add); branches that cannot place a locked card are dropped.
- **Proven filter.** Before a deck is scored exactly, `upper_na` bounds it: given N = n chain events, the progress vector is negatively upper-orthant dependent (checked exactly per chain-kind key over its state distribution, early ends and stop risk included; with two cards feeding one target, the same check over every up-set of their pair), so with every card grouped with the targets its chain feeds, E[Π groups | n] ≤ Π E[group | n]; a target that would join a larger group (three or more feeding cards, or two other than the one pair) is taken at every feeder's full progress instead, a constant at least its factor on every chain state (a target's factor only rises as its misses fall), so such decks get a looser bound rather than none (Grand Masters decks often have them); event outcomes enter at their marginal chances (exclusive outcomes only lower the product). U = Σ_n P(n) Π E[· | n] ≥ V; on the reference setups U is within 0.2% of V. `UPPER_CHECK=1` asserts U ≥ V on every deck scored.
- **Per-uma tables.** X(u, i) = the best exact value of trainee u over every legal deck at grid chance i (64 points); V rises with q, so a lineage at chance q is bounded by X at the next grid point up. Cells are computed lazily: a loop's ceiling uses each cell's exact value when known, else the lowest known value at a higher grid point, else ρ times the product ceiling; while the ceiling beats the k-th best, the loosest term is refined (first at the uma's highest grid point, then halfway down).
- **Loops.** Structures of both sizes are generated in bands of a separable bound, the mean of the members' best product values M_u at their highest chances (a member heads 3 of 12 lineages in a loop, 1 of 3 in a rotation, so the product ceiling is at most the golds' factor times that mean; bands are kept in four-member sum units, a rotation's sum s counting as 4s/3); a structure is solved (schedules, then exact values) only when its exact ceiling at the pairwise-ceiling chances, then at its own schedule's chances, can still beat the k-th best exact value; the search stops when no ceiling (times ρ for the ungenerated ones) can.
- **Shared deck** (`per_uma` false): a loop's value is the best exact sum over its lineages (12 or 3) among decks legal for all its members, divided by their number; ceilings are the lower of the per-uma tables and ρ times the best single skyline deck's product value over its lineages. `top_decks` counts distinct decks, each with its best loop; per loop, every deck at or above the k-th best distinct deck so far is kept.
- **Certificate and budget.** The pass at ρ = 1.25 (each class at its cap) runs first; if it exceeds the work budget (`EXACT_BUDGET`, counted in work units so results do not depend on machine load: about 3 s natively), passes with every cap at most 1.12, then 1.05 follow (likewise when a deck search would return over `EXACT_CAP` = 50,000 decks, `cap_hit`), each reusing every exact value, table cell and schedule already computed; if none finishes, a pass at ρ = 1 (product-odds pruning, no deck searches) answers. Output `exact`: `rho` the cap the answer is certified under, `max_ratio` the largest V/V̂ on any deck scored (a value above `rho` would contradict the assumption), `complete`, counts. Job field `exact: false` runs the ρ = 1 pass alone: the page shows it first, then the exact one.
- `p_all`, `p_minus1`, `per_trainee` and the detail's per-lineage `p_all` are exact.

"Proven" (`proven`, `exact.complete`) means the search ended by its stop rule under the certified cap. It is relative to the schedules the optimiser finds (iterated local search, not proven optimal; `exact.schedule_rejects` counts loops rejected only after their schedule was built) and to the cap.

Typical time natively: about 0.3–4 s per search with one result (the Global default 0.25 s, Grand Masters all owned 0.9 s on an i5-1340P; with four-uma loops only, `sizes: [4]`, 0.97 s and 1.15 s: a strong rotation found early prunes most loops), most of it the exact deck searches; three-result shared searches 10–35 s. The browser's exact pass is about 1–2× native; its first pass (ρ = 1) takes under 0.5 s. Dominance tests in the skyline enumeration use a 64-bit signature (16 coordinates × 4 bits) as a fast necessary check before the exact comparison.

Skill model: a target is acquired if guaranteed, on the trainee's own outfit skill list, hinted by the deck, or inherited (proc = 1 − Π(1 − base·(1 + affinity/100))² over the six legacies, base 4.65% below rank score 17,500 (stars 50/45/5%), 5.7% from 17,500 up (20/70/10%)). Each legacy's affinity follows uma-it-optimizer's per-legacy fit (`docs/MODEL.md`, Per-legacy affinity; github.com/Acrith/uma-it-optimizer, `tools/loop/affinity.py`, `inspiration_rates.py`): a parent P counts pair(T,P) + pair(P1,P2) + trio(T,P,G) for each of its own parents G + 3 per G1 shared with the other parent and with each G; a grandparent G under P counts trio(T,P,G) + 3 per G1 shared with P; grandparents get no extra factor. In a loop P's own parents are the other parent and the fourth member, so the parents' shared G1s count for both terms. An acquired skill becomes a spark 35.4% of the time as a white, 70.9% as a gold (20% / 40% ×1.1 per ancestor already carrying it, all six in a running loop). Odds are averaged over the structure's lineages (12 in a loop, 3 in a rotation).

### Three-uma rotations (`sizes`)

A rotation {A, B, C}: each member trains once per round from the other two, whose own parents are the other two of the
three. Trainee A has parents B and C; B was bred from (C, A), C from (B, A). Over member positions its lineages are
(T, P1, P2, X) = (0, 1, 2, 0), (1, 0, 2, 1), (2, 0, 1, 2): the loop's form with X = T (`skills::lins`). The value is the
mean P(all) over the 3 lineages, the loop's over its 12: both are the chance per career when every member trains in turn,
and the search ranks both sizes together.

- **Repeated character.** A grandparent may be the trainee's own character (907 of 3,179 uma-it runs have one;
  none has a parent of her character). `rel_tables` counts a relation group only for distinct members, so
  tri(T, P, T) = 0 and pair(a, a) = 0; fitted on 101,536 inspiration trials, that (H0) predicts 521.8 passes from
  self-grandparents against 523 seen, while counting the trio as pair(T, P) predicts 605.4 (about 13 nats worse).
- **Her races.** In a steady rotation the grandparent of character T is T's own earlier veteran with T's schedule, so its
  shared wins with parent P are o(P, T), the same overlap as between those two members anywhere in the rotation; the
  ×1.1⁶ spark generation is unchanged (all six legacies carry every spark).
- **Race weights (derived).** With o(a, b) a pair's expected shared G1 wins, a lineage (T, P1, P2, X) counts 3·o in its
  slot affinities: o(P1, P2) six times (each parent: once for the other parent and once for that parent as her own parent;
  each grandparent slot under them once), o(P1, X) and o(P2, X) twice each (the parent and the grandparent slot), the
  trainee's own pairs never. Summed over the 12 loop lineages every pair gets 6·2 + 2·4 = 20; over the 3 rotation
  lineages (X = T) every pair gets 6·1 + 2·2 = 10. Both are uniform over the pairs, so a rotation's schedules maximise the
  plain sum of its three pairs' shared wins, as a loop's do over six; `optimize` is generic over the member count (test
  `rotation_affinities_by_hand` checks the 30 / 60 affinity per unit of a pair's overlap).
- **Bounds.** `q_ub_trainee` lets the X slot also be the trainee (trio 0 + 3·PB(P, T)); pairwise ceilings are unchanged
  (structure-free). Exact per-uma tables, the certificate and the stop rule are shared by both sizes.

Why rotations usually win (measured when they were added, 2026-10-07: every four-uma result stayed bit-identical
and 43 of 46 deck jobs became rotation-led; `pal_H` and `pal_I` shared stayed four-uma loops): a rotation's careers
are each a little worse than the loop's best (the fourth uma's relation points as grandparent, a median ×1.05 per
career over 660 trainee-parent combinations), but a loop must also train its fourth member, usually a weak trainee
(Oguri Cap at 1 in 216 in `pal_U`).

### Career planner (`"mode":"career"`, page `/career`)

The best next career from the veterans a player has: trainee, two veterans as parents (at most one borrowed), deck.
Same files as deck mode, plus `legacies`; the shared and deck fields apply (`owned`/`exclude`/`exclude_outfits`
choose the trainees, `owned_cards`, `borrow`, `scenario`, ...; `star_dist` sets the stars of the new veteran's sparks in the look ahead; other loop fields are unused).
`val/career_job.py TRAINED_CHARA_JSON BASE_JOB OUT_JOB` builds a job from an umadump export, as the page does.
Several career jobs run in one process with `target/release/loopopt data.json a.json b.json ...` (one output line each),
as the page's solver worker runs them: a job's fully checked decks are kept per setup (everything but the pool and the
places) and bound the later jobs' searches, never the searches that feed the rollout's cache, so every output equals
the job run alone.

| Field | Meaning | Default |
|---|---|---|
| `legacies` | veterans: `{chara, borrowed, sparks: [[skill group, stars], ...], pinks: [[aptitude group, stars], ...], wins: [race index], parents: [{chara, sparks, pinks, wins}, ...]}`; race index = position in data.json `races`; aptitude groups 11 turf, 12 dirt, 21-24 front runner to end closer, 31-34 sprint to long; a parent with `chara` 0 is unknown (no relation points, its sparks still count) | required |
| `trainees` | chara ids to consider (empty: every owned uma) | `[]` |
| `fix` | legacy indices that must be among the two parents (0, 1 or 2) | `[]` |
| `per_trainee` | one result per trainee (each with her best parents and deck); false: the best careers whoever trains (with one trainee: her best parent pairs) | `true` |
| `top` | careers returned | 5 |
| `next` | the page's next-career plan, section *Next career (`next`)* below | none |
| `lookahead`, `p_now`, `p_minus1_now`, `policy`, `rollout`, `plan` | solver-only, section *Solver-only career fields* | |
| `compare` | marks the page's veteran-analysis jobs (the best career with her fixed as a parent, and the plan of the pool with her added); echoed as `compare: true` in the output, changes nothing else | `false` |
| `no_reports` | leaves out `results` (the main search's career reports and the first one's schedule search): the page's plan-only jobs read `next` alone; out of the setup fingerprint and the kept main search's key, so it changes nothing else | `false` |
| `cmd: "state_out"` / `cmd: "state_in"` | the worker's kept work across page loads: `state_out` returns `{ok, state}`, the kept plans (each uma's best career per pool, as above) and the kept proofs (fully checked decks per setup and deck class), every float as its bit pattern in hex; `state_in` with that `state` puts them back (all or nothing: a state that does not parse, or of another version, is refused with `error`). The career page stores it in IndexedDB keyed by the solver's and data's fingerprints, after each plan, and restores it before a new worker's first job. Natively: `LOOPOPT_STATE_IN` / `LOOPOPT_STATE_OUT` (files) around a career run | none |

#### Model per career

Model per career (T, L1, L2): the six legacies are L1, L2 and their recorded parents. Affinity as in the loop model
with the recorded G1 wins (shared = same race index; the page maps win saddles by saddle group, so a race at a
relocated venue is the same race). Per target k: q_k = 1 − Π_legacies (1 − min(1, 0.03·s·(1 + A/100)))², s = that
legacy's stars of k (0 without a spark); spark generation 0.2 / 0.4 × 1.1^n_k, n_k = legacies carrying k. Derived:
q_k and the trainee's own event hint chance e enter every landing factor only as (1 − e)(1 − q_k), so the trainee's e is
replaced by 1 − (1 − e)(1 − q_k) and the deck search runs unchanged at q = 0; the factor 1.1^(n_k − 6) per target is the
same for every deck and multiplies P(all) (`JointExt` applies it per target for P(all but one) and the per-target
chances, which are exact marginals of the joint model). Tested against the loop model (`career_fold_and_scale`; and
end to end: with 2★ sparks everywhere and no race overlap every lineage of `pal_U` gives the same P(all) and q to
9 digits).

#### Search and output

Search: each candidate gets a ceiling ρ·C·max_outfit Π_k f_k at each target's best possible deck odds (the deck_size
lowest miss chances of the trainee's legal entries, every link firing); candidates are taken best ceiling first. A
candidate is solved like a per-uma trainee (best product deck, then every deck whose product value times ρ reaches
the best exact value, through `upper_na`, scored exactly). Between candidates of one trainee a proven bound skips
most of them: from chances q0 to q a target's factor grows at most by (1 − (1 − e)(1 − q_k)) / (1 − (1 − e)(1 − q0_k))
(1 when q_k ≤ q0_k or the outfit has the skill), for every deck and chain state, so E(q) ≤ E(q0) · max_outfit Π_k of
those ratios. The same bound holds between trainees of one deck class (equal outfit coverage and own-skill chances of
the targets, the same relevant cards barred, as many plain pals: the best deck is then one function of the chances), so
the fully checked decks and product-search proofs kept between searches and jobs (`seen`, `ruled`) are kept per class:
a classmate's proof bounds her candidates too (on the owner's 16-veteran pool 47 of 68 umas share one class). Identical
deck problems are searched once: a candidate whose trainee, chances and spark scale are bit-identical to an earlier one's
in the same search takes its outcome (`Twin`: interchangeable veterans, as a sample pool of mob veterans gives, make
every parent pair one problem), and a deck search without start decks is memoised per job by what it reads (deck class,
folded chances, spark scale, bar, work spent; `deck_memo`), so classmates at the same chances search once. The
search stops when the next ceiling is below the k-th best (one per trainee). Output per career:
`trainee` (outfits, covers), `parents` (legacy indices), `slots` (six legacies: affinity parts, stars), `targets` (per
target `q`, `carriers`, landing chance `p`, and `buy`: the chances she buys it as gold / white and lands it from each
(`gold`, `white`, `land_gold`, `land_white`; land_gold + land_white = p) with its sources `deck_gold`, `deck_white`,
`own_event`, `inherit`, `outfit` (0, 1 her white, 2 her gold), or `scenario`), `pinks` (per aptitude among the six legacies: `q` = chance at least one of
the two inheritance rolls passes it on, 1/3/5% per star × (1 + affinity/100), and `carriers`; reported only: aptitudes
do not enter the skill odds), `aptitudes` (her grades and pink spark, MODEL.md: `base` and `start` codes in `apt` order,
`fire` = each legacy pink with its chance to fire at one April inspiration, `pink_gen` = per aptitude the chance her
veteran's pink is that one; her schedule's win chances count them; absent without aptitude data), `p_all`, `p_minus1`, `expected` (Σ p), `deck`; and `candidates`, `solved`,
`bounded`, `complete`, `proven_top`, `work` (the main search's deck-search nodes and exact scorings, deterministic; 0
when the worker kept the search from the job before). The best career also carries `schedule`, `g1_wins` and `outcome_sets`:

- `schedule`: every G1 she could enter, in turn order, as the loop planner's chart rows for one member (`race` the
  chart name, `when`, `year`, `cells` = [win %, goal, aptitude and streak terms] or [null], `shared` = a parent won it)
  plus `name` (full race name), `parents` (which won it), `run`/`skipped`, `instead` (the race she runs that turn, or
  "rest") and `p` (her win chance, null when skipped). The schedule maximizes expected distinct G1 wins, each race weighted
  1 plus 1 per parent who won it (`best_response`; a G1 she shares with a parent adds 3 affinity to each of them when
  she is a parent herself), under goal races, same-turn clashes and streak win penalties. `g1_wins`: expected wins.
- `outcome_sets` (up to 12 targets, else null): the exact chance of each set of targets landing, indexed by bitmask
  (bit i = job target i), guaranteed golds' spark generation included.

#### Next career (`next`)

The page's plan (`sim::plan::index_rule`): exact where it can be, not simulated. The page sends it as a second job,
`next: {}` with `no_reports: true`, after the search job (it reuses the search the worker keeps). The model and its
evidence: `docs/MODEL.md`, *Next career*. Constants are in `deck/career/sim/plan.rs`.

| Field of `next` (all optional) | Meaning | Default |
|---|---|---|
| `sur`, `sur_margin` | 1: each uma's best career from the simulator's surrogate (`best_sur`: every legal pair by ceiling, her cached decks, an exact deck search where the bound beats the table by more than `sur_margin`) in place of the career search; 2: both, the differences listed in `sur_diff` (on the owner's 16-veteran pool: none over 68 umas, the same time) | 0, 0 |
| `value_only` | V alone, no score per candidate | `false` |
| `exact_all` | every uma's best career searched exactly (no proxies: the rule before builders went through their deck class) | `false` |
| `proxy_check` | each builder's proxy reported beside her exact career (`proxy_check` in the output); changes nothing else | `false` |
| `proxy_eff` | test knob replacing `PROXY_EFF`; `tests/plan_memo.rs` sets -1 (pass 1's builders left to their proxies, so the pass-2 searches refit Cont) | 0.06 |
| `fit` | `[[chara, lam, phi, pinf], ...]` (a plan's own `fit` output): the stage candidates and their continuations taken as given instead of fitted to this pool. Unused by the page since Compare plans exactly as Add will (b2ecc73) | none |
| `loop_members` | loop members to add as stage candidates; absent: none (the plan runs no loop search) | absent |
| `search_work`, the other `plan` fields | read from the same structure as `plan` (section *Solver-only career fields*) | as `plan` |

**Stage candidates** (at most 8): the best career's trainee, the loop members when named, the umas of the best
career's lineage and of your veterans, the search's other trainees. Below 4 (`HINT_FLOOR`: a loop being started, mob or
other-uma veterans) they are filled from the owned umas with the highest odds under a full loop lineage (parents
hypothetical veterans of the first two candidates' umas, each bred from the other and from her, valued by
`Env::ceiling`, no deck search; reported in `hint`). Each gets her best career. On the val/plan pools the loop members
always entered by the other routes, which is why the loop planner's search was dropped from the plan.

**Pass 1**: every allowed uma's best career, exact over her outcomes, giving per candidate the chance her veteran
raises that candidate's odds and the mean odds raised to.
- A builder's career goes through her deck class: her class's winning decks (the main search's and the candidates'
  best careers), valued at her own chances by the surrogate, no career search. She is searched exactly when her lift
  efficiency is within `PROXY_EFF` (6%) of a candidate's second best, or her saving within `PROXY_SAVE` (6%) of the
  lowest that the shown rows (`OPT_SHOW` = 8), the pick's near-tie band or two deep take. Recalibrated 2026-10-09 with
  `proxy_check` on val/plan: the proxy reads at most 2.79% / 2.67% low. A row left on its proxy carries `exact: false`.
- Before a near-top builder is searched, her proxy is certified where her deck class's proofs allow (`certified`: over
  every legal pair the ceiling, the class's fully checked decks through `rise` and its product proofs stay at or below
  the proxy's value, so it is her best career's value); with mob or affinity-free parents every classmate has the same
  chances and one search proves her class. Counted in `umas_certified`.
- Cont's two builders come from the exactly searched careers.

**Continuation and V.** Per candidate a continuation (`Cont`, assumed form): builder careers lift her odds with chance
lam_j = lam ((1 - phi)^j)^`LAM_DECAY` (1.5) at the j-th success, each closing phi of the gap in log odds to
p_inf = `CEIL_SHARE` (0.6) x her odds with a full loop lineage (parents hypothetical 8/8s of her two most efficient
builders, bred from each other and from her), at least her odds now. Her step (a fresh 8/8) costs
min over n of sum_{j<n} 1/lam_j + 1/p_n (n up to `CONT_N` = 500; `cost_lo` stops once the building alone reaches the
best cost, so the cap decides nothing). `Cont::plan` and `Cont::cost_lo` are memoised per Cont by the bits of p
(`PLAN_MEMO=1` prints lookups and hits, `PLAN_MEMO_CHECK` recomputes every hit and panics unless bit-identical,
`PLAN_MEMO_OFF`, `PLAN_MEMO_CAP` replaces the 16k cap). V = min over trios of side(A) + side(B) + 1/p_C: A and B
candidates, each side either a real 8/8 veteran of hers (cost 0) or a fresh 8/8 (her step; the veteran her best career
now would leave, `Env::veteran`: its parents, every target at the rank's star mix, her G1 wins at their chances), C
every allowed uma at her odds with those two as parents. The cheapest choice of sides wins: a real 8/8 with poor parents
can lose to a fresh one.

**Pass 2 and the pick.** A career's score is E[V - V with her veteran] - 1 over its outcomes: her veteran as a parent
raises any candidate's odds, her own 8/8 makes her fresh side free, and with two 8/8 parents her own 8/8 is the goal
(V after 0). Two deep, for the 3 highest (`TWO_TOP`) and the best odds: her rolls one target short of 8/8 (`TWO_MISS`;
stars at the rank's mix), as a parent of the 3 highest builders (`TWO_BUILDERS`), scored the same way, over today's
best score. Pick: the highest; within 0.25 careers per run (`NEAR`) or 5%, the better own odds. The goal is always the
next fully looped parent, any uma: a fully looped veteran you have is a real 8/8 (free side) with 8/8 parents.

**Output `next`:**
- `options`: per career `role` (plan / finish / loop / candidate / builder), `index`, `two_step`, `score`, `lift` (the
  chance her roll lowers V), `near`, `stage_cost` for a candidate, `chosen`; `choice`, `mode` (trio).
- `members` (A and B): `{trainee, p, have8, real (the trio uses a real 8/8 of hers), p_inf, lam, phi, builds,
  p_switch, careers}`; `v0` (V); `v0_plan` (`{a, b, c, careers}`); `fit` (the stage candidates in order with their
  continuations, `[chara, lam, phi, pinf]`).
- `outcomes`: the pick's careers saved per outcome, V - V with her veteran: `{trainee, parents, uncv, fixed, y}`. `y` is
  indexed by leaf << len(fixed) | fm, leaf = her uncertain targets' stars in base 4 in `uncv` order (0 absent), fm =
  the scenario's guaranteed golds present; the page's Reroll reads a roll's saving from it. `y_hi` at the same index: at
  least `y`, the most her veteran's direct lifts could save for that outcome (each candidate's odds with her as a
  parent bounded over every partner y by the least over anchors A of c' E_A rise(q', q_A) (`Env::rise`: no assumed
  cap), E_A at least the best odds at q_A: today's pairs (y, z) at p_now / c_yz, the main search's found careers of the
  candidate's deck class at their `Ref`s, and, for the partners behind the band's likeliest outcomes, her best deck
  searched at her 2 and 3 stars on every uncertain target (`HI_STARS`; at most `HI_SEARCHES` searches, 0 by default = off, 10 when on; outcomes whose
  partners' bounds pass `HI_RISE` = 1.5 x p_now are left as they are), the latter two under the caps; and by the ceiling
  (C4's, `outfit_fast` at q'); the step at `Cont::cost_lo`); `provisional`: the chance a roll reads
  y < 0.5 <= y_hi, which the page checks exactly. The replan's other careers with her as a parent are not bounded (its
  candidates and Cont are the plan's).
- `her_trios`: her own 8/8 as a free trio side when her uma is not a stage candidate (at the rank's star mix, as the
  trios' fresh sides, with every candidate side and the best C).
- `umas_scored`, `umas_searched` (careers searched exactly), `umas_certified`, `hint`, `sur`, `sur_diff`, `work` (this
  plan's deck-search nodes and exact scorings, deterministic: a speed change can be measured at any machine load;
  `work.max_ratio` the largest exact/product ratio scored, `class_max_ratio` / `class_max_deck` per class, `class_caps`,
  `cap` the cap the plan's deck searches hold under), timings. A career deck search whose band would list over
  50,000 decks is rebuilt at 1.12, 1.05, then 1 (`Env::cert`); its answer is not a proof (no `Ref`), and the output's
  top-level `cap` is lowered to that cap, `complete` set false and `band_overflows` counts them.

**Kept plans.** Each trainee's best careers are kept per plan (the last few, in the worker). A plan of the same setup,
places and pool takes them as they are (no search: a reload, an edit undone); a plan of the same setup and places whose
pool is a kept one plus one veteran at the end (Compare, then Add) takes the better of the kept best and a search with
her fixed, whose bar starts at the kept value (`Env::floor`). Exact (on the owner's 16-veteran pool identical to the
plan run cold), about 3 ms instead of 9 s. The kept plans and proofs survive a page reload through `cmd: "state_out"` /
`"state_in"` (fields table above).

#### Solver-only career fields (not sent by the page)

Earlier page features, kept in the solver for the simulator, the tests and reverting. The page sends only `next`.

| Field | Meaning | Default | Used by |
|---|---|---|---|
| `lookahead` | `true`: the reroll-or-move-on look ahead for the best career, facts only. Per landed set S (exact) of her veteran, f(S) = the best P(all) of a next career with her in the pool: the four best setups (`SHORTLIST`) with her landing everything, each rescored at S (a floor, at least `p_now`); for up to 8 targets also exact over her stars (P(all) is multilinear in each target's miss factor, so 2^nu corner values give every outcome). Output `lookahead`: `p_now`, `p_minus1_now`, `improve`, `partial_improve`, `by_count`, `miss_one` (per target: `p`, `f`, `no_gain`, ...), `improve_stars`, `f_if_improve`, `improve_by_count`, `best_case_with_her`, `multilinear_check`, `stars_exact` | `false` | `jobs/career_U4.json`, `career_U5.json` (val references), `tests/career_progress.rs` |
| `p_now`, `p_minus1_now` | today's best P(all) (and P(at least n−1) on that career) over every career: the floor of the look ahead and of the rollout; computed when absent | none | look ahead, rollout |
| `policy` | `{stay_trainee, stay_parents: [a, b], p_floor}`: for one veteran fixed as a parent (`fix`), "stay" (that career again) against "advance" (the best career with her), each scored C_a = 1 + Σ_{S≠all} P_a(S) / f_a(S) (one more career, then the best setup it leaves, repeated; f floored at `p_floor` and both careers' P(all); null when a failure has f = 0). Output `policy`: `stay`, `advance` (`expected_careers_estimate`, `p_all`, `p_within_two`, ...), `continuation_floor`; with `plan` also `policy.plan` | none | `tests/career_progress.rs` |
| `rollout` | K > 0: of the K best careers whoever trains (`trainees`, `fix` respected), the one minimising Q(a) = 1 + Σ_{S≠all} P_a(S) / f_a(S): a rollout of "repeat the best career it leaves", so never worse than repeating today's best (derived); a ranking rule, never shown as careers (it counts every improvement as permanent: Q at the U4 start 568 against the policy's measured 230). Also p* (today's best: 1/p* careers by repeating it) and the ceiling p_bar (every career's P(all) at most p_bar, so E[N] ≥ 1 + (1 − p*)/p_bar). Output `rollout`: `pick`, `choice`, `candidates`, `differs`, `p_star`, `p_bar`, `p_bar_proven`, `lower_bound`, `repeat_careers`, `candidates_proven`, `shortlist_proven`, `table_check`. Measured under the model: 230 careers to the first hit on `career_U4` against 270 for greedy and 587 for repeating; K = 6 chosen, 12 not better | 0 | the simulator's `planner` policy (`planner_k`), `val/policy/` (`policies.mjs`, pools), `jobs/career_U5.json` |
| `plan` | `{budget_ms (20000), samples_min (4), samples_max (16), eps (1e-3), horizon_max (200), seed (1), top (6), recent (3), trainees (12), loop_members, margin (1e9), search_work (5,000,000), refresh_work (0)}`: the page's pick before `next`. Per candidate (the K best careers, the best with each of the R newest veterans, the N best trainees' best, each loop member's, today's best), the expected careers to an all-target veteran over two base policies (surrogate greedy, the loop planner's loop as a rotation) on common random numbers; samples doubled until the 95% paired intervals separate or the budget ends; pick the highest P(all) among those not separated from the leader. Output `plan` | none | nothing in val/ or tests; `next` reads the same structure (`search_work`, `loop_members`) |

#### Work budget

Work budget: a career search has its own (`CAREER_BUDGET`, 300M units; the loop planner's 30M left synthetic pools of
12, 100 and 200 veterans unproven with the same picks; measured need 10M-85M up to 400 veterans; `EXACT_BUDGET` in the
environment overrides it). Candidates are kept as (ceiling, trainee, parents) and their chances recomputed when solved
(peak memory at 400 veterans 914 MB -> 265 MB, same results).

### Policy simulator (`simulate` on a career job)

Measures a decision policy (which career to run next) the way `val/policy_sim.mjs` does, inside one solver process.
With `simulate` present the career job runs the simulator instead of the search; jobs without it are unchanged. Per
run, careers t = 0, 1, ... with p_t the exact P(all) of career t (derived, as in policy_sim):

    E = sum over t of S_t,  S_0 = 1,  S_{t+1} = S_t (1 - p_t);  stop when S_t < eps or at max_careers, plus the tail S_t / p_t

Each failed career leaves a veteran drawn exactly as policy_sim draws it (landed set from `outcome_sets` without the
all-landed set, inverse CDF over sets ordered fewest-landed first; stars from `star_dist`; per race a win with the
schedule's chance of at least one win; her parents the career's), with policy_sim's random numbers bit for bit (`u01`
keyed by seed, run, career, stream, item), so a run index faces the same uniforms under every policy and a greedy path
equals policy_sim's greedy path while the picks agree. Dominated new veterans are pruned as in policy_sim.

The surrogate (policy `greedy`): each candidate (trainee, parents) is valued through its trainee's cached decks' corner
tables (`Cache`, exact P(all) of that deck at any inheritance chances), bounded first by the search's ceiling, by
`rise` from her fully checked decks and by her product-search proofs (`ruled`). Where the bound rb clears the best so far
B and exceeds the table value tv by more than the margin m, her deck is searched there (`best_deck` at bar B) and
cached. Derived: the career picked has an exact P(all), with the deck it runs, of at least E*/(1 + m), E* the exact best
career's (unless a deck search ran out of work); m = 0 gives the exact greedy pick up to ties. A candidate is valued once,
when her later parent arrives; a trainee whose static ceiling (q = 1, every spark on six legacies) is below B is skipped.

| Field of `simulate` | Meaning | Default |
|---|---|---|
| `policy` | `greedy` (surrogate), `greedy_exact` / `planner` (the solver in process per decision: best career by P(all), the rollout pick), `loop`, `rollout_greedy`, `rollout_loop`; with `loop_members` as the candidate list: `stale` (the page's rule: a candidate trains once after her best career changes, else the best career), `pick_index` (the careers-saved index per candidate, `samples` veterans each, decided again when a best value changes), `cycle` (`cycle`: chara ids trained in turn, each on her best career), `trace` (building for `loop_members[0]` with the rest as builders: each career the most efficient builder runs; per career her odds before and after, the builder, the exact lift chance and mean raised odds; val/sim/README.md E5), `next` / `next_noretry` / `next_retry1` (the former loop rule and its retry variants); measurement modes `index` (per candidate: one career, then the best forever, `samples` veterans) and `index2` (the grandparent route through `builder`, `samples` × `inner`) | `greedy` |
| `runs`, `first`, `seed` | run indices `first` .. `first + runs - 1`; policy_sim's seed | 16, 0, 1 |
| `eps`, `max_careers` | stop at S < eps or this many careers (tail S/p added) | 1e-3, 100000 |
| `margin` | refresh margin m; -1: every valuation a bound does not rule out gets a fresh exact deck search (exact-backed: the same proofs as the career search, no trust in cached decks) | 0 |
| `prune`, `share_cache` | prune dominated veterans; keep the deck cache across runs | true, true |
| `check`, `check_lineages`, `check_frac` | every `check` new veterans compare with exact searches: the exact greedy pick (career search), and up to `check_lineages` of the newest veteran's candidates with ceiling at least `check_frac` x B (exact best deck, table value, landed-set distance) | 0, 0, 0.3 |
| `samples`, `cand_top`, `cand_per_trainee`, `cand_recent`, `redecide`, `cont_eps` | rollout policies: paired continuations per candidate; candidates = the base policy's pick, the K best careers, the best career of the N best trainees, the best career with each of the R newest veterans; decide at the first career and every `redecide` careers (0: first only); continuations stop at `cont_eps` (default `eps`) | 8, 6, 4, 2, 0, none |
| `audit`, `audit_sample`, `audit_every` | fidelity audit on the main path: at each greedy pick change the exact career search's pick and, for the pick and `audit_sample` random candidates with the newest veteran, the table value against a fresh exact deck search (relative error, landed-set total variation); the same for each rollout decision's choice and `audit_sample` random candidates; every `audit_every` loop careers her deck and the keep-or-replace decision under exact values | false, 3, 50 |
| `loop_members` | `loop`: rotation members (chara ids) in order | none |
| `planner_k`, `progress` | the planner's rollout width; log every N careers | 6, 0 |

`loop`: member i mod n trains; her parents are the rotation veterans of the previous two members. A member keeps the
better of her rotation veteran and the new one, by the summed surrogate P(all) of the two loop careers she is a parent
of (always replacing her latest veteran collapses: a veteran missing a target breeds lineages with fewer carriers until
a target has none and P(all) is 0 for good). Bootstrap: a member without both parent veterans gets the best pair by
exact P(all) from the pool with the existing ones fixed. No pruning. Each career's schedule is her own (the career
model's); the loop planner coordinates schedules across members, this does not.

`rollout_*`: at a decision career each candidate is run (sample j's uniforms the same for every candidate) and the base
policy follows until S < `cont_eps`; the candidate with the lowest mean E over the samples runs, the other careers
follow the base policy. Output: per run `E`, `careers`, `tail`, `final_p`, `pool`, `pruned`, `wall_s`, `refreshes`,
`segs` (each pick change: career, p, pool size, S), `decisions`, `checks`; and `E` (mean, sd, 95% CI) over the runs.
Results and commands: `val/sim/README.md`.

## 4. Validation

`cargo test --release` (in `pkg/loopopt`) covers, among others:
- **Reference fixtures** (`tests/fixtures/`):
  - 200 best responses from an exact MILP, including forced races;
  - 71 rest-forced best responses from the MILP, including turns 24/25/48/49/50 for the turn-49 goal umas;
  - the full 2,278-pair ceiling table;
  - 23 schedules scored by the Python reference.
- **Properties checked:**
  - `feasible()` rejects bad schedules;
  - a force that conflicts with a goal is infeasible;
  - known optima are reproduced;
  - the grandparent-weight closed forms equal a direct per-legacy sum; optional G1s cost nothing.

An independent adversarial review (separate agent, its own MILP and SQL, 576 best-response cases, 6,400 ceiling checks, a full data export cross-check) passed every core claim. It found 3 interface bugs in the affinity-only CLI modes (since removed: a force could override a goal, `top = 0` panicked, duplicate turns were accepted); the force check is still tested.

The load step now asserts the structural fact the exact DP relies on: every race offered twice is a Classic/Senior pair exactly 24 turns apart.

| Check | Result |
|---|---|
| Scoring vs the Python reference: 32 known schedules (scf + rot, both sessions' outputs); 23 of them ship in `tests/fixtures/known_schedules.json` | max \|diff\| 4.5e-13 |
| `dp_exact` vs exact MILP (scipy/HiGHS), random weights, 40 umas × 4 weight types | 160/160 equal |
| `dp_exact` vs MILP, all 68 umas, incl. forced group moves, turn-49 goal umas (Meisho Doto, Mejiro Palmer, Haru Urara) and multi-turn goal alternatives (Fuji Kiseki, Fine Motion, Fukukitaru, Chiyono O) | 192/192 equal, max \|gap\| 3.4e-13 |
| Ceiling table `pb` vs the Python MILP table (2,278 pairs) | max \|diff\| 1.1e-14 |
| Cold-start search reproduces every known best loop: 11 scf + 3 rot | all reproduced exactly (2058.92, 2011.30, 2008.51, 1961.02, 1958.00, 1956.38, 1948.98, 1945.95, 1942.37, 1941.28, 1936.89; rot 1944.49, 1915.98, 1833.03) |
| Deep search (200 rounds, no group-move pre-screen) on 6 contenders | no value exceeded the known optima |
| Independent checker (written from the spec without seeing this code): 12 final loops | all feasible; scores match to 4.5e-13 |

## 5. Benchmarks (2-core container)

| Operation | Python (MILP) | Rust v1 | Rust v2 (current) |
|---|---|---|---|
| Exact best response, one uma | 110 ms | 35–40 µs | ~5.5 µs, plus a memo: ~99% of calls inside a search are repeats |
| Ceiling table, all 2,278 pairs | 387 s | 0.18–0.25 s | 0.04 s |
| Full search, one foursome (scf, 30 rounds) | ~6–7 min | 1.6–2.3 s | ~0.03–0.05 s |

v1 → v2 is byte-identical on every job tested (only timing fields differ). The changes:
- `dp_exact`: precomputed per-option win tables, no per-call allocation, and loops inverted so choice pairs are outer and states inner, which keeps the tie-breaking order.
- `ascent` skips re-solving members whose partners haven't changed.
- A best-response memo per `optimize` call, keyed on exact weight bits.
- `group_moves` reuses probability vectors.

## 6. Limits

- **Schedules are heuristic.** `optimize` reproduces every known optimum and the deep search found nothing better, but no proof is given that a structure's true schedule optimum isn't higher.
- **Model assumptions (§1) are user-supplied or community-derived:** the win formula and the affinity-only objective. The skill model's per-legacy affinity is uma-it-optimizer's receipt-fitted model; its exact parent terms are fitted on totals only, so the split between them is less certain than the overall level. The code doesn't validate them.
- **Junior G1s need 1,000 fans and a debut win gives 700.** Unless a fan bonus applies, add one extra Junior race the uma wins, before Early Nov. This doesn't change affinity.
- **Base-outfit aptitudes by default.** Use `OUTFIT=chara:card` for alternate outfits (Vodka 100802, Daiwa 100902, Urara 105202); this needs a re-export.
