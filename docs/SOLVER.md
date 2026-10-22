# The solver (loopopt)

The Rust crate in `pkg/loopopt/`; crate paths below are relative to it, while `docs/`, `game/`, `val/`, `webapp/` and
`build.sh` paths are relative to the repository root. Its data is in `data/global/` and `data/jp/` (the Grand Masters
era). The CLI resolves a job's file names from the working directory; run it from the era's data folder.
Building and running it: `docs/BUILD.md`.

The affinity core and loop search's modules:

| File | Holds |
|---|---|
| `src/lib.rs` | Crate wiring, deterministic hashing, the native/wasm clock and the public CLI entry point |
| `src/data.rs` | Raw and loaded race data, schedule choices, relation tables, ceiling fingerprints, win probabilities and feasibility |
| `src/schedule.rs` | Exact single-uma best responses, scratch buffers and scoped memo |
| `src/optimize.rs` | Loop affinity scoring, ascent, group moves, local search and pairwise ceilings |
| `src/chart.rs` | Race/member labels and structured schedule charts |
| `src/cli.rs` | CLI jobs, event enrichment, kept-state persistence, dispatch and bound diagnostics |
| `src/tests.rs` | Affinity-core correctness tests against independent fixtures and model invariants |
| `src/skills.rs` | Skill targets, outfit choices and lineage inheritance probabilities |
| `src/deck/frontier.rs` | Deck-job setup and nondominated frontier enumeration |
| `src/deck/dominance.rs` | Dominance signatures, buckets, `DominanceIndex` and returned deck records |
| `src/deck/state_bound.rs` | State-conditioned values, bounds, configuration and ratio tables |
| `src/deck/loop_search.rs` | `Multi`, its inputs and keys, setup and `run_multi`; fixed schedules (`prebuilt`) |
| `src/deck/loop_search/tables.rs` | Per-uma exact values, `Grid`, pair ceilings, skyline classes and product ceilings |
| `src/deck/loop_search/shared.rs` | `SharedScan`, shared-deck scoring and cached scans |
| `src/deck/loop_search/bands.rs` | Loop bands, schedules, certification and solved passes |
| `src/deck/loop_search/report.rs` | Reported picks, JSON and diagnostic summaries |

The `kept` module is `career/plan/kept.rs`: cached career plans and searches between planner jobs.

Searches parent loops (four-uma loops and three-uma rotations) and support decks for landing target skills in
Independent Training, reports each uma's race schedule, and plans careers to a looped parent (`/career`). Its
inputs are data files exported from the game's `master.mdb` and GameTora's event data.

```
master.mdb ──(export/export_rs.py)──▶ data.json ──(loopopt)──▶ ranked loops + schedules (JSON)
```

---

## 1. Model

**Win chance** of one attempt (the model and its evidence: `docs/MODEL.md`):

```
p = clamp(110 − cost(distance grade) − cost(surface grade) − PEN[s], 0, 100) / 100
cost: S,A=0  B=10  C=20  D=30  E=40  F=60  G=90      PEN[s] = 0,0,5,20,30,50 for s = 1..6+
s = k for a chosen race; s = max(k − 1, 1) for a goal race (scored_position)
```

The algorithm tests run on frozen data with older win chances (`tests/fixtures/data_v1.json`: PEN =
0,0,10,25,35,50). `win_table_matches_community_table` checks 13 cost rows and all six streak positions using the
Global data's penalties; it does not check the exported aptitude costs.

- **Streak position k:** counts consecutive turns with *any* race, goal races included. A rest resets it. A goal race
  (an option of one of the career's goals) takes the penalty of position k − 1; it still counts toward the races after it.
- **Turn** = 24·year + 2·(month−1) + half, with year 0 = Junior, 1 = Classic, 2 = Senior: turns 1–24, 25–48, 49–72; half 1 = Early, 2 = Late.
- **Win probability per race:** P_a(r) = 1 − Π(1 − p) over uma a's attempts at race r. A race can be run in Classic and Senior; it still counts once.

**Objective (expected affinity per 4 runs):**

```
E = base + 3 · Σ_{i<j} C_ij · ⟨P_i, P_j⟩          ⟨P_i,P_j⟩ = Σ_r P_i(r)·P_j(r) = expected shared G1 wins
```

The optimiser uses C_ij = 2 for every pair and base = 2·Σ pair + 2·Σ trio (`C_PAIR`, `K_TRIO`), the normalisation of
the earlier per-legacy split with grandparents at half weight. The adopted model (grandparents at full weight) weights
every pair's overlap 20 times over a loop's 12 lineages (`docs/RESEARCH.md` §6): a different scale, but the same
weight for every pair, so both rank schedules identically; a three-uma rotation likewise weights its three pairs
equally. Test `score_matches_per_legacy_sum` checks the closed form against a direct sum over every lineage and slot.

- Aptitudes: each uma carries `apt` (ten codes 1..8 = G..S: short, mile, middle, long, turf, dirt, front runner, pace chaser, late surger, end closer; from her first outfit unless overridden at export); `race_cat` / `race_ground` give each race's distance class (0 short .. 3 long) and surface (1 turf, 2 dirt). The loop scheduler reads exported base win chances; the career schedule also reads these aptitude fields for the legacies' pink sparks (`Env::pink_career`).
- `pair(a,b)` / `tri(a,b,c)`: the sum of `succession_relation.relation_point` over the relation groups shared by all of them. `data.json` stores the groups (`relations`: `[points, [roster indices]]`, groups with at least two roster members) and the solver builds both tables at load.
- **Fixed rules** (race options from the exporter; affinity in the solver):
  - G1 only, +3 per shared G1.
  - Relocated-venue G1s count as the normal race: 1025→1012, 1027→1006, 1028→1005, 1029→1016.
  - `determine_race = 2` goal alternates are excluded.
  - Goals sharing a `sort_id` are alternatives; at least one must be run.
  - Turns in `single_mode_race_restrict_turn` keep only her goal races.

**Assumptions:**
- Schedules are fixed in advance.
- Stats and training turns aren't modelled.
- Two umas' runs are independent, so E[shared wins] = ⟨P_i, P_j⟩.

## 2. Algorithms

| Function | What it does | Exact? |
|---|---|---|
| `Uma::dp_exact` | Best response for one uma: maximize Σ_r W(r)·P(r) given partner weights W. Junior turns 1–24 run in sequence. Then Classic turn t and Senior turn t+24 are decided **together**, with state (Classic streak, Senior streak, goal bitmask), plus the Classic streak at turn 48 whenever turn 49 has a race option. Every race offered in both years is exactly 24 turns apart, so 1 − (1−p₁)(1−p₂) is computed exactly. `Uma::best_response` memoises calls within `optimize`. | **Yes** (tested against an exact MILP, §4) |
| `LoopProblem::ascent` | Best-response dynamics: each member re-solves against the other members until none improves by more than 1e-9. This is a Nash point up to that tolerance, not necessarily the global optimum. | Local |
| `LoopProblem::group_moves` | Coalition deviations: force 2+ members onto the same race (or rest) at one turn, then re-ascend. | Local |
| `optimize` (`LoopProblem::solve`) | Optimistic start (partners at best-case odds) → ascent → group moves → `iters` rounds of iterated local search (1–2 members re-solved against noisy weights, then ascent and group moves). | Heuristic |
| `pair_ceilings` | Exact pairwise ceilings PB(a,b) = min(BR_a(PM_b), BR_b(PM_a)), where PM is a partner's best-case odds per race: no pair of schedules shares more expected G1 wins. The deck search bounds every structure's lineage affinities with them before any schedule is optimised; `loopopt pbtable` prints the table for `export/add_pb.py` to merge into data.json. | **Guaranteed** bound |

**Why the ceiling is valid:**
- P_b ≤ PM_b elementwise and P_a ≥ 0, so ⟨P_a, P_b⟩ ≤ ⟨P_a, PM_b⟩ ≤ BR_a(PM_b).
- Take the min over both sides.

A structure whose bound is below a result already achieved cannot beat it.

## 3. Usage

```bash
# 1. export data (needs the Global master.mdb in game/global/)
(cd export && uv run build_base.py global && uv run export_rs.py global) # writes data/global/data.json (../../build.sh data: both eras)

# 2. build
cargo build --release
# optional, after any export: store the exact pairwise ceilings in data.json ("pb", with a fingerprint of the
# umas/goals/penalties; ignored when stale) so searches skip computing them (../../build.sh pb does both eras)
uv run export/add_pb.py target/release/loopopt data/global/data.json

# 3. run a job file from the era's data folder (the file paths in a job are relative to it)
(cd data/global && ../../target/release/loopopt data.json ../../../../val/jobs/pal_U.json) > /tmp/result.json
```

The CLI runs `"mode":"deck"` and `"mode":"career"` job files (below), and `loopopt pbtable data.json`, which prints
the pairwise-ceiling table `export/add_pb.py` merges in. The wasm interface (`src/web.rs`) accepts `cmd` requests:
`init`, `deck`, `career`, `state_out`, `state_in`.

### Schedule chart

A result's `schedule` (`chart::chart_rows`) lists races in calendar order, one cell per member (required non-G1 goal
races are not listed):
- **Shared G1s:** races (by identity, in either year) run by 2+ members. Their expected shared wins contribute affinity. A shared row with one runner means the other members run that race in the other year; it need not be her second attempt.
- **Unshared G1s:** run by one member only. They contribute no shared wins and can pass down race sparks. Skipping one never lowers affinity, since it can only shorten a streak, but a goal race may still be required.

A cell carries the win % for that attempt (streak penalty included), the goal flag, the aptitude cost and the streak
position; `alts` names races that can replace the row's at no affinity cost when all its runners switch together (`ScheduleChart::interchangeable`).

**Optional fill pass:** after optimizing, each member re-solves against her partners with a tiny bonus (1e-7 per
expected G1 win), keeping the result if affinity drops by at most 1e-6. A second pass tries G1s on rest turns,
including 0% attempts, until no more fit: each addition must preserve every existing G1 attempt's win chance and
keep affinity within 1e-12 of the current score. Test `optional_g1s_cost_nothing` checks that stripping unshared
non-goal G1s leaves the score unchanged within tolerance.

### Fields shared by deck and career jobs

| Field | Meaning | Default |
|---|---|---|
| `skills_file` | outfit skill lists + skill catalog (`web_skills.json`, from `export/export_web.py`) | required |
| `targets[]` | `group` = skill group id; `guaranteed` 0 none / 1 white / 2 gold; `deck_white`, `deck_gold` = deck hint chance 0–1 | required |
| `star_dist` | spark star mix on the legacies `[p1★, p2★, p3★]`; supplied explicitly for another rank mix | `[0.50,0.45,0.05]` |
| `owned` | uma ids you own (empty = whole roster) | `[]` |
| `exclude` | uma ids to leave out | `[]` |
| `require` | uma ids every result must contain (at most the largest of `sizes`; added to the pool even if not in `owned`) | `[]` |
| `exclude_outfits` | outfit (card) ids you don't own | `[]` |
| `extra` | `[uma id, skill group, level]` skills an uma always gets (e.g. Zephyr Slipstream, Teio Head-On) | `[]` |
| `iters` | iterated-local-search rounds per structure's schedules | 30 |
| `seed` | RNG seed of the schedule search | 3 |
| `sizes` | structure sizes searched, ranked together: 3 (three-uma rotations, see *Three-uma rotations*) and/or 4 (four-uma loops); `require` may list at most the largest size, and four required umas leave only loops | `[3, 4]` |

A deck result's `loop`: `members` (3 or 4), `outfits` (each uma's most frequently chosen outfit option across her
lineages and the targets it covers), `p_all` (all targets land), `p_minus1` (at least all but one), `landed` (the chance
of exactly k targets, k = 0 ..= n, averaged over the lineages; exact over each lineage's landed sets, so `landed[n]`
= `p_all` and the top two sum to `p_minus1`), `per_trainee`, `mean_proc`, `rank_value`, `schedule` (*Schedule chart*)
and `detail` (per lineage: the six legacy slots' affinity parts, q, each target's odds).

### Deck + loop search (`"mode":"deck"`)

Finds the support deck and the loop together. Needs `skills_file`, `cards_file` (`web_cards.json`, from `export/export_cards.py`) and usually `events_file` (`card_events.json`, from `export/build_card_events.py` over the GameTora files that `./build.sh gametora` caches in `cache/gametora/` at the repository root).

```bash
(cd data/global && ../../target/release/loopopt data.json ../../../../val/jobs/deck_unity8.json)        # everyone owned
(cd data/global && ../../target/release/loopopt data.json ../../../../val/jobs/deck_unity8_noti.json)   # no Tamamo / Inari
```

Extra fields (on top of the shared fields above; scenario-guaranteed skills stay `guaranteed: 2`, everything the deck can give is left at 0):

| Field | Meaning | Default |
|---|---|---|
| `owned_cards` | card ids you own (empty = all) | `[]` |
| `borrow` | allow at most one card you don't own (friend card); does not require a borrowed card | `false` |
| `exclude_cards` / `lock_cards` | cards never / always used; more locked cards than `deck_size` is refused | `[]` |
| `deck_size` | cards per deck, 1 to 6 (the search counts each chain kind's cards as a base-7 digit) | 6 |
| `top_decks` | distinct decks returned, each with its best loop (with `per_uma`: loops returned) | 1 |
| `per_uma` | one deck per uma: each trainee runs her own best legal deck, the same in all the lineages she heads (three in a loop, one in a rotation) | `false` |
| `pals` | only decks with exactly this many pals | any |
| `schedule` | experiment: `"solo"` (each uma maximises her own race wins), `"goals"` (career objectives only) or `"none"` (shared wins ignored) instead of loop-optimised schedules | optimised |
| `scenario` | Independent Training scenario: `unity`, `gc` (Grand Concert), `ura`, `trackblazer`, `grandmasters` (JP); sets the training-hint base and specialty weight (Grand Masters uses the 1.86 / 0.26 placeholders) | `unity` |
| `chain_events` | `[n, p]`: chain events per run ~ Binomial(n, p) (experiments) | `[21, 0.35]` |
| `cap_audit` | audit: every exact cap at 2.0 instead of `EXACT_CAPS`, and ten times the band limit `EXACT_CAP` (deck and career jobs; `val/capaudit.py`) | `false` |
| `cap_free` | career jobs: every deck search cap-free (the search a cap breach reruns, *Cap-free rerun* below) instead of only those that saw a breach; no product-search proofs are kept (`ruled` bounds through the cap). For checks: slower | `false` |
| `min_rarity` | lowest rarity for cards with chain events (2 = SR); cards without chain events (R cards) are always considered, drop them with `exclude_cards` | 2 |
| `events`, `events_p`, `odds_events` | inline event rewards (merged with `events_file`) | `[]` |
| `alts` | events naming two or more skills, in full: `[card, link, [choice][outcome] -> [[group, 1 \| 2], ...]]` (from card_events.json `alts`); at these links they replace `events` / `events_p` | `[]` |

Model (the rates and their evidence: `docs/MODEL.md`, *Deck side*):
- **Training hints**: h = base(scenario) x (1 + hint_frequency/100) x (1 + d(scenario) x specialty_priority/100)
  draws per card per run (`hint_base`, `hint_spec`; hint
  frequency at max limit break, unique effect included; the floor or ceiling of h, weighted by the fraction), each
  uniform over the card's skill options still below Lv 5 (`nskill`; the stat option is never drawn; a skill leaves the
  pool after ceil(5 / `hinc`) draws, `hinc` = hint levels per draw at max limit break, so a +5 card never repeats a
  skill); one card's draws over several targets are joint (`miss_draws`). Non-pal cards without chain events take no
  chain slot; their training hints still count.
- **Chain events**: Binomial(21, 0.35) per run (`CHAIN_N`, `CHAIN_P`; `chain_events` overrides), each advancing a
  random remaining chain slot of a non-pal card. Pal finales are certain.
- **Outcome shares**: an outcome-dependent reward counts at its share of the outcomes, k of n at k/n before measured weights are applied. A measured share replaces k/n where one exists: `chance` in
  manual.json for hand-entered entries (Anshinzawa SSR's finale, Riko Kashimoto SSR's Ramp Up), and
  `event_shares.json` for generated chain events, which the builders turn into the chain shares and `alt_w`, the job
  field the solver uses as outcome weights in place of equal outcomes.
- **Early chain ends**: a choice that ends a card's chain (GameTora's 'ee' reward) is offered as a separate deck
  entry: the card's earlier links as usual, the ending choice's rewards at that link, and its remaining chain slots
  freed for the other cards. Choices that end the chain on only some outcomes are not offered, but when such a choice
  is IT's default (first) choice the chain stops there with chance k/n (`stop_risk` in card_events.json; the chain
  odds are the exact mixture over where it stops, freed slots going to the other cards).
- **Links and choices**: multiple links can contribute to acquiring a target, but repeated hints do not generate extra sparks.
  An event's outcomes exclude each other, and where an event offers targets on different choices the card is offered
  once per choice worth taking (a choice no other beats on every target), like an early chain end; the deck output
  names it (`choices`: "link 1: Uma Stan", the skill to prioritise). A hand-entered pal or group entry may name the
  event and choice it models (`event`: [story id, choice]); the event builders then derive `prioritise` from GameTora's
  choice layout and IT's rule (the skills on that choice and on no choice above it; a choice nothing steers to is an
  error), and the page shows it on the deck card (Anshinzawa SSR: Risky Business).
- **Gold and white**: a card's gold and white of one skill are combined exactly (deck `odds`: `gold` = P(gold hint),
  `white` = P(white hint | no gold hint)).

Exact odds. Targets do not land independently: chain cards share one run's chain events (one binomial count), so their chain progress is coupled, and an event's outcomes can give one target or another. A deck's exact value for a trainee at inheritance chance q, V(u, q, D), is computed over the joint distribution of chain progress, given which every source is independent except outcome groups touching two or more targets, which are enumerated per cluster of targets they join (`joint_fast`; `joint_trainee_ref` is the full enumeration, and `JOINT_CHECK=1` compares both in the `joint_checked` wrapper; direct calls to `joint_fast` bypass it). The search ranks loops and decks by exact values. The product of per-target odds, V̂ (`rank_value` in the output), is only used to bound and order the work:
- **Assumed caps, per deck class.** The capped search assumes V ≤ ρ·V̂, with ρ set by the deck's class
  (`EXACT_CAPS`, `cap_class`): 1.05 when at most two targets are fed by chain links (`TargetSources::chain_misses`) and every outcome
  group passes `group_assoc`, otherwise 1.25 (`EXACT_RHO`). `TargetSources::uncertified_association` marks a group that fails that check; merely
  touching two targets does not fail it. These caps are assumptions, not proven bounds. A search node takes the
  class of its cards plus every candidate still to add, so its cap is at least any completion's; bounds that know no
  deck (`Env::ceiling`, the loop tables' product fallback, the reroll's `y_hi`) use 1.25. Within the work budget, decks
  and loops whose capped product ceilings could beat the incumbent are refined or scored unless another bound
  excludes them. Guards: the output's `class_caps` and per class `class_max_ratio` / `class_max_deck`; `val/cmp2.py` and
  `val/planval.py` fail when a scored deck's ratio passes 1 + 0.75 (cap − 1); job field `cap_audit: true` (every cap
  at 2.0, ten times the band limit) with `val/capaudit.py` checks the decks the caps pruned.
- **Cap-free rerun** (career searches, `best_deck_run`). A deck search that scores a deck whose ratio passes its
  class's cap (a breach: the cap is wrong for that deck space, so a deck it pruned may beat its answer) is answered by
  a cap-free search instead (`best_deck_cf`), started from the capped search's best product deck and its start decks;
  without a breach the capped answer stands, so outputs are unchanged wherever no cap is breached. Its band holds
  every deck whose bound reaches the bar (the incumbent, or the career's bar th / c if higher), built with two proven
  bounds and nothing else (no cap in any test), then scored through `upper_na`. Per partial deck: (1) given N = n only,
  per chain slot its card's ratio (its value with the card at a given links done over the filled level-free slots'
  value; an open slot: the best of its group from no card), a rising function of that slot's level, so where the
  configuration's progress vector given n is negatively upper-orthant dependent (`key_level`, exact) the expected
  product is at most the product of the slots' expected ratios; the zero-base targets ride on the slot that feeds them
  most, the others at their lowest misses; each n's term at most A with every slot at most min(n, stop) links done;
  (2) where (1) passes, per chain state x (`StateBoundConfig`'s projected states) the lower of the per-target bound A_x and the
  ratio bound B_x (log-submodularity of each target's factor, card by card, given x), summed over states, heaviest
  first with an early stop once the sum decides the test; the open slots of one group share its lowest misses and best
  ratios, so states that differ only by the order of that group's open-slot levels have one term, computed once. Both
  rest on `state_value`'s argument, which needs every group's per-target levels negatively upper-orthant dependent at
  every white/gold threshold (`group_nuod_lvl`, `TargetSources::certified_cap_free`); a deck space with an entry not covered proves nothing.
  Proven when every legal entry is covered and the band finishes without overflow within its own budget
  (`CF_BUDGET`, default 60M work units, 15 per cap-free node; apart from the career's budget): then a best below
  th / c is `Bounded`, else the deck is a fully checked
  one (`Ref`, `full`); unproven, the better of its best and the capped answer, not full (the search is not
  `complete`). The output's `cap_rerun` counts the reruns (`searches`), those that proved nothing (`unproven`) and
  their `work`; `cap_exceeded` still lists the classes past their caps. Not covered: the career search's own ceilings
  and `ruled` proofs still use EXACT_RHO, and a search that never scores a breaching deck has nothing to trigger it,
  so "proven" stays relative to the caps where no breach was seen. Checks: `CF_CHECK=s` runs the cap-free search
  beside every capped one (state put back) and compares answers, passes its nodes at s times their bound and scores
  every leaf reached exactly against every bound above it, and checks the state value against each exact scoring
  (`CFCHK` on stderr); `CAPS_TEST=a,b,c,d` replaces the caps for breach tests; `CAP_RERUN=0` leaves a breach
  unanswered and the search not `complete`.
- **State-conditioned bound** (career searches' band pass, at each configuration's root): over the configuration's
  chain progress states x, P(x) times the product over targets of each target's factor at its lowest possible misses
  given x (per open slot the best entry of its group at that slot's level); proven >= the exact value of every deck of
  the configuration (given x, sources are independent and one group's per-target hints negatively associated), so a
  configuration below the bar is dropped when the dependence checks cover its sources. It prunes beside the caps.
  `SBOUND=0` turns it off; `SB_CHECK=1` checks the value against exact scorings and checks root and depth-1 node
  bounds against their leaves.
- **Every legal deck.** `DeckSpace` enumerates every deck legal for the trainee (for every member with a shared deck: the character rule is a legality filter), by chain-kind key, before card dominance drops any card; cards interchangeable for the trainee (same rewards on the targets her outfits leave open, or none) are merged; a partial deck is dropped when the best its completions could reach is below the threshold: per (chance, outfit) the lower of the per-target ideal point (the r lowest miss chances per target in each kind group) and a submodular bound (each card's ratio P({card})/P({}) for the product of one outfit at one chance, which gains less the more the deck holds; the best ratios of the cards still to add); branches that cannot place a locked card are dropped. The submodular bound may split the targets: the ideal point's factors for some, the ratio bound over the rest, the lowest of the splits taken. An outfit with a target at zero chance without a hint (no inheritance, event or outfit chance) has infinite ratios unsplit, so it is split by its zero targets; one with targets at low base chance (at most half their certain-white factor) also by those. The configurations keep the unsplit bound's visiting order, which decides which of two tied decks is found first; `RB_CHECK=s` checks every leaf against its ancestors' bounds (nodes pass at s times their bound, results unchanged). In the product search (the best deck by product value) a card is skipped where a free card of its group, legal wherever it is (borrow, lock and pal rules), is at least as good on every target's white and gold miss: the deck with that card comes first and is at least as good in floats, so the deck found is the same; its configurations' ratio tables and roots are built only for configurations the search may visit. The band pass keeps every deck (removing one would change what it scores exactly).
- **Proven filter.** Before a deck is scored exactly, `upper_na` tries to bound it: given N = n chain events, it
  checks negative upper-orthant dependence per chain-kind key over its state distribution, early ends and stop risk
  included; with two cards feeding one target, it checks every up-set of their pair. With every card grouped with
  the targets its chain feeds, E[Π groups | n] ≤ Π E[group | n]. A target that would join a larger group (three or
  more feeding cards, or two other than the one pair) is taken at every feeder's full progress instead, a constant
  at least its factor on every chain state. Event outcomes enter at their marginal chances after `group_nuod`
  checks their dependence. U = Σ_n P(n) Π E[· | n] ≥ V. The bound returns `None` when its dependence checks fail;
  callers then use another bound or score exactly. `UPPER_CHECK=1` asserts U ≥ V for covered decks scored through
  the loop search's exact-value cache.
- **Per-uma tables.** X(u, i) = the best exact value of trainee u over every legal deck at grid chance i (64 points); V rises with q, so a lineage at chance q is bounded by X at the next grid point up. Cells are computed lazily: a loop's ceiling uses each cell's exact value when known, else the lowest known value at a higher grid point, else ρ times the product ceiling; while the ceiling beats the k-th best, the loosest term is refined (first at the uma's highest grid point, then halfway down).
- **Loops.** Structures of both sizes are generated in bands of a separable bound, the mean of the members' best product values M_u at their highest chances (a member heads 3 of 12 lineages in a loop, 1 of 3 in a rotation, so the product ceiling is at most the golds' factor times that mean; bands are kept in four-member sum units, a rotation's sum s counting as 4s/3); a structure is solved (schedules, then exact values) only when its exact ceiling at the pairwise-ceiling chances, then at its own schedule's chances, can still beat the k-th best exact value; the search stops when no ceiling (times ρ for the ungenerated ones) can.
- **Shared deck** (`per_uma` false): a loop's value is the best exact sum over its lineages (12 or 3) among decks legal for all its members, divided by their number; ceilings are the lower of the per-uma tables and ρ times the best single skyline deck's product value over its lineages. `top_decks` counts distinct decks, each with its best loop; per loop, every deck at or above the k-th best distinct deck so far is kept.
- **Certificate and budget.** The pass at ρ = 1.25 (each class at its cap) runs first; if it exceeds the work budget (`EXACT_BUDGET`, default 30M work units, independent of machine load), passes with every cap at most 1.12, then 1.05 follow (likewise when a deck search would return over `EXACT_CAP` = 50,000 decks, `cap_hit`), each reusing every exact value, table cell and schedule already computed; if none finishes, a pass at ρ = 1 (product-odds pruning, no deck searches) answers. Output `exact`: `rho` the cap the answer is certified under, `max_ratio` the largest V/V̂ on any deck scored; `class_max_ratio` must be compared with each class's cap at the returned `rho`, `complete`, counts. Job field `exact: false` runs the ρ = 1 pass alone; its returned decks are still scored exactly.
- `p_all`, `p_minus1`, `per_trainee` and the detail's per-lineage `p_all` are exact.

"Proven" (`proven`, `exact.complete`) means the search ended by its stop rule under the certified cap. It is relative to the schedules the optimiser finds (iterated local search, not proven optimal; `exact.schedule_rejects` counts loops rejected only after their schedule was built) and to the cap, which is assumed: a scored deck whose ratio passes its class's cap shows in `class_max_ratio` but leaves `complete` as it is here (the career search reruns the affected deck search cap-free, *Search and output* below).

Dominance tests in the skyline enumeration use a 64-bit signature (16 coordinates × 4 bits) as a fast necessary
check before the exact comparison.

Skill model: a target is acquired if guaranteed, on the trainee's own outfit skill list, hinted by the deck, or
inherited; an acquired skill becomes a spark at the generation rate. The equations and fitted values (inheritance,
per-legacy affinity, spark generation, star mix) are in `docs/RESEARCH.md` §1-5. Odds are averaged over the
structure's lineages (12 in a loop, 3 in a rotation).

### Three-uma rotations (`sizes`)

A rotation {A, B, C}: each member trains once per round from the other two, whose own parents are the other two of the
three. Trainee A has parents B and C; B was bred from (C, A), C from (B, A). Over member positions its lineages are
(T, P1, P2, X) = (0, 1, 2, 0), (1, 0, 2, 1), (2, 0, 1, 2): the loop's form with X = T (`skills::lins`). The value is the
mean P(all) over the 3 lineages, the loop's over its 12: both are the chance per career when every member trains in turn,
and the search ranks both sizes together.

- **Repeated character.** A grandparent may be the trainee's own character. `rel_tables` counts a relation group
  only for distinct members, so tri(T, P, T) = 0 and pair(a, a) = 0. The evidence for this model is in
  `docs/RESEARCH.md` §5.
- **Her races.** In a steady rotation the grandparent of character T is T's own earlier veteran with T's schedule, so its
  shared wins with parent P are o(P, T), the same overlap as between those two members anywhere in the rotation; the
  ×1.1⁶ spark generation is unchanged (all six legacies carry every spark).
- **Race weights (derived).** With o(a, b) a pair's expected shared G1 wins, a lineage (T, P1, P2, X) counts 3·o in its
  slot affinities: o(P1, P2) six times (each parent: once for the other parent and once for that parent as her own parent;
  each grandparent slot under them once), o(P1, X) and o(P2, X) twice each (the parent and the grandparent slot), the
  trainee's own pairs have no additional overlap term. Summed over the 12 loop lineages every pair gets 6·2 + 2·4 = 20; over the 3 rotation
  lineages (X = T) every pair gets 6·1 + 2·2 = 10. Both are uniform over the pairs, so a rotation's schedules maximise the
  plain sum of its three pairs' shared wins, as a loop's do over six; `optimize` is generic over the member count (test
  `rotation_affinities_by_hand` checks the 30 / 60 affinity per unit of a pair's overlap).
- **Bounds.** `q_ub_trainee` lets the X slot also be the trainee (trio 0 + 3·PB(P, T)); pairwise ceilings are unchanged
  (structure-free). Exact per-uma tables, the certificate and the stop rule are shared by both sizes.

### Career planner (`"mode":"career"`, page `/career`)

The best next career from the veterans a player has: trainee, two veterans as parents (at most one borrowed, or none:
`borrowed_parent`), deck.
Same files as deck mode, plus `legacies`; the shared and deck fields apply (`owned`/`exclude`/`exclude_outfits`
choose the trainees, `owned_cards`, `borrow`, `scenario`, ...; `star_dist` sets the stars of a veteran before her result is known: the plan's fresh 8/8s, the simulator's draws; other loop fields are unused).
`uv run val/career_job.py TRAINED_CHARA_JSON BASE_JOB OUT_JOB` builds a job from an umadump export, as the page does.
Several career jobs run in one process with `target/release/loopopt data.json a.json b.json ...` (one output line each),
sharing the first job's skill and card files. A job's fully checked decks are kept per setup and bound later jobs'
searches; `setup_fp` excludes the pool, trainee and parent filters, result counts and page request flags. The
simulator's seed search disables these kept proofs. Timings and work counts depend on reuse.
The page runs a career as two jobs: the search, then the same job with `next`, which reuses the
search the worker keeps (`LAST_SOLVE`, the last three, keyed by the whole job without `next` and `no_reports`) together with what its
product searches proved (`Env::ruled` as the search left it). `tests/plan_state.rs` compares separate search/plan
jobs with a combined job. The product-search proofs go no further than their own search: `certified` takes a
proxy career a proof certifies in place of an exact search, which may name another pair or deck of equal value, so
proofs from other pools could change an output.

| Field | Meaning | Default |
|---|---|---|
| `legacies` | veterans: `{chara, borrowed, sparks: [[skill group, stars], ...], pinks: [[aptitude group, stars], ...], wins: [race index], parents: [{chara, sparks, pinks, wins}, ...]}`; race index = position in data.json `races`; aptitude groups 11 turf, 12 dirt, 21-24 front runner to end closer, 31-34 sprint to long; a parent with `chara` 0 is unknown (no relation points, its sparks still count) | required |
| `trainees` | chara ids to consider (empty: every owned uma) | `[]` |
| `fix` | legacy indices that must be among the two parents (0, 1 or 2) | `[]` |
| `borrowed_parent` | `"allow"`: at most one borrowed parent per career, now and in the plan's look-ahead. `"later"`: the career picked has none (the search's `results` and the plan's rows, *Next career* below), while the plan's V and V' allow later careers to borrow; a `next` job searches without and with borrowing, reusing kept searches when available. `"never"`: `Pool::max_borrowed` is 0 in every pool the job builds (`tests/plan_borrowed.rs`). This pair rule is excluded from `setup_fp`, so kept proofs and decks are shared across settings; it is included in the kept main search's key and the kept plans' (`bests_key`). Simulator policies `greedy`, `loop` and `trace` take "later" as "allow"; `greedy_exact` and `index` pass it through to their career jobs | `"allow"` |
| `per_trainee` | one result per trainee (each with her best parents and deck); false: the best careers whoever trains (with one trainee: her best parent pairs) | `true` |
| `top` | careers returned | 5 |
| `next` | the page's next-career plan, section *Next career (`next`)* below | none |
| `simulate` | runs the policy simulator instead of the search (section *Policy simulator*) | none |
| `compare` | marks the page's veteran-analysis jobs (the best career with her fixed as a parent, and the plan of the pool with her added); echoed as `compare: true` in the output, changes nothing else | `false` |
| `no_reports` | leaves out `results` (the main search's career reports and the first one's schedule search): the page's plan-only jobs read `next` alone; out of the setup fingerprint and the kept main search's key, so it changes nothing else | `false` |
| `roll` | `{job, stars}`: a roll of a plan, answered beside this job's own answer as `roll` (the plan job `job` run after this one with `next.roll` = `stars`, its `next.roll`; below). (The page's Reroll now sends the plan job with `next.roll` alone first, so the saving shows at once, then its search) | none |
| `cmd: "state_out"` / `cmd: "state_in"` | the worker's kept work across page loads: `state_out` returns `{ok, state}`, the kept plans (each uma's best career per pool, as above), the kept proofs (fully checked decks per setup and deck class), the kept main searches (`LAST_SOLVE` with their product proofs, so a reload's first run searches nothing again) and the plans' roll states (`ROLLS`, *Next career*: a reload's first roll plans nothing again), every float as its bit pattern in hex; `state_in` with that `state` puts them back (all or nothing: a state that does not parse, or of another version, is refused with `error`; so is one with a stored count above 1024 (`STATE_MAX_N`; a count is at most the umas in the data), an index past its list, a NaN, a chance outside [0, 1], a deck of more than six cards or a roll state whose vectors do not fit together or exceed the plan's 12 targets, 10 uncertain. Stored decks and careers are checked again where they meet a setup (`Env::seen_fits`, `solved_fits`): one naming a deck entry, trainee or veteran the setup does not have is not used there). The career page stores it in IndexedDB keyed by the solver's and data's fingerprints, after each plan, and restores it before a new worker's first job. Natively: `LOOPOPT_STATE_IN` / `LOOPOPT_STATE_OUT` (files) around a career run | none |

#### Model per career

Model per career (T, L1, L2): the six legacies are L1, L2 and their recorded parents. Affinity as in the loop model
with the recorded G1 wins (shared = same race index; the page maps win saddles by saddle group, so a race at a
relocated venue is the same race). Per target k: q_k = 1 − Π_legacies (1 − min(1, 0.03·s·(1 + A/100)))², s = that
legacy's stars of k (0 without a spark); spark generation 0.2 / 0.4 × 1.1^n_k, n_k = legacies carrying k. Derived:
q_k and the trainee's own event hint chance e enter every landing factor only as (1 − e)(1 − q_k), so the trainee's e is
replaced by 1 − (1 − e)(1 − q_k) and the deck search runs unchanged at q = 0; the factor 1.1^(n_k − 6) per target is the
same for every deck and multiplies P(all) (`JointExt` applies it per target for P(all but one) and the per-target
chances, which are exact marginals of the joint model). Tested against the loop model by `career_fold_and_scale`.

`run_career` builds `Model` once; `Env` borrows it read-only. `Model::ews` always holds the own-event chances.
`Env::folded_events` owns one reusable outfit/target buffer per trainee; `set_ews` refills it in place for each
lineage or corner. Exact scoring and its bounds read the chosen chances through `JointQuery::event_chances`;
enumeration borrows `DeckChances` (the model plus the selected event-chance table). Loop searches select the
unfolded table. The career deck memo keys the folded buffer; class and rise calculations read `Model::ews`.

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
a classmate's proof bounds her candidates too. `rise(t, q', q)` and every proof (`Ref`, `ruled`) depend on the trainee only
through her class, so the sharing is exact. The planner's deck cache holds exact winners only: pruning, the proofs kept
between jobs and class sharing change which decks a search happens to check fully, and a cache of those would make the
plan depend on search history. Identical
deck problems are searched once: a candidate whose trainee, chances and spark scale are bit-identical to an earlier one's
in the same search takes its outcome (`Twin`: interchangeable veterans, as a sample pool of mob veterans gives, make
every parent pair one problem), and a deck search without start decks is memoised per job by what it reads (deck class,
folded chances, spark scale, bar, work spent; `deck_memo`), so classmates at the same chances search once. The
search stops when the next ceiling is below the k-th best (one per trainee when `per_trainee` is true). Output per career:
`trainee` (outfits, covers), `parents` (legacy indices), `slots` (six legacies: affinity parts, stars), `targets` (per
target `q`, `carriers`, landing chance `p`, and `buy`: the chances she buys it as gold / white and lands it from each
(`gold`, `white`, `land_gold`, `land_white`; land_gold + land_white = p) with its sources `deck_gold`, `deck_white`,
`own_event`, `inherit`, `outfit` (0, 1 her white, 2 her gold), or `scenario`), `pinks` (per aptitude among the six legacies: `q` = chance at least one of
the two inheritance rolls passes it on, 1/3/5% for 1/2/3 stars × (1 + affinity/100), capped at 1 per roll, and `carriers`; reported only: aptitudes
do not enter the skill odds), `aptitudes` (her grades and pink spark, MODEL.md: `base` and `start` codes in `apt` order,
`fire` = each legacy pink with its chance to fire at one April inspiration, `pink_gen` = per aptitude the chance her
veteran's pink is that one; her schedule's win chances count them; absent without aptitude data), `p_all`, `p_minus1`, `expected` (Σ p), `deck`; and `candidates`, `solved`,
`bounded`, `complete`, `proven_top`, `work` (the main search's deck-search nodes and exact scorings, deterministic; 0
when the worker kept the search from the job before). A scored deck whose exact / product ratio passes its class's
cap (`EXACT_CAPS`, an assumption) has its deck search rerun cap-free (*Cap-free rerun* above); a proven rerun does
not itself make the search incomplete. The output lists the classes in `cap_exceeded` ([class, highest ratio, cap])
and the reruns in `cap_rerun`. The best career also carries `schedule`, `g1_wins` and `outcome_sets`:

- `schedule`: every G1 she could enter, in turn order, as the loop planner's chart rows for one member (`race` the
  chart name, `when`, `year`, `cells` = [{p, goal, apt, row, pen}] or [null], `shared` = a parent won it)
  plus `name` (full race name), `parents` (which won it), `run`/`skipped`, `instead` (the race she runs that turn, or
  "rest") and `p` (her win chance, null when skipped). The schedule maximizes expected distinct G1 wins, each race weighted
  1 plus 1 per parent who won it (`Uma::best_response`; a G1 she shares with a parent adds 3 affinity to each of them when
  she is a parent herself), under goal races, same-turn clashes and streak win penalties. `g1_wins`: expected wins.
- `outcome_sets` (up to 12 targets, else null): the exact chance of each set of targets landing, indexed by bitmask
  (bit i = job target i), guaranteed golds' spark generation included.

#### Next career (`next`)

The page's plan (`career::plan::index_rule`): exact where it can be, not simulated. The page sends it as a second job,
`next: {}` with `no_reports: true`, after the search job (it reuses the search the worker keeps). The model and its
evidence: `docs/MODEL.md`, *Next career*. The planner is split across `src/career/plan.rs` and `src/career/plan/`.

| Field of `next` (all optional) | Meaning | Default |
|---|---|---|
| `sur`, `sur_margin` | 1: each uma's best career from the simulator's surrogate (`best_sur`: every legal pair by ceiling, her cached decks, an exact deck search where the bound beats the table by more than `sur_margin`) in place of the career search; 2: both, the differences listed in `sur_diff` | 0, 0 |
| `exact_all` | every uma's best career searched exactly (no proxies) | `false` |
| `proxy_check` | disable proxies for the main plan and report each builder's proxy beside her exact career (`proxy_check` in the output); also evaluate looped rows' lift statistics | `false` |
| `proxy_eff` | test knob replacing `PROXY_EFF`; `tests/plan_memo.rs` sets -1 (pass 1's builders left to their proxies, so the pass-2 searches refit Cont) | 0.06 |
| `fit` | `[[chara, lam, phi, pinf], ...]` (a plan's own `fit` output): the stage candidates and their continuations taken as given instead of fitted to this pool. Unused by the page, since Compare plans exactly as Add will | none |
| `loop_members` | loop members to add as stage candidates; absent: none (the plan runs no loop search) | absent |
| `search_work` | work budget (career-search units) of each candidate search beyond the job's own; 0 = the job's budget | 5,000,000 |
| `margin` | the surrogate's refresh margin (section *Policy simulator*) | 1e9 |
| `roll` | a roll of the plan's pick: her stars per job target (0 where it did not land); the output is `next.roll` (below). Answered from the plan's kept roll state with no search when the worker has it for this job, else after the plan, which keeps it | absent |

**Stage candidates** (at most 8 unless supplied by `fit`): the best career's trainee, the loop members when named, the umas of the best
career's lineage and of your owned veterans, the search's other trainees. With neither `fit` nor `loop_members`, below 4 (`HINT_FLOOR`: a loop being started, mob or
other-uma veterans) they are filled from the owned umas with the highest odds under a full loop lineage (parents
hypothetical veterans of the first two candidates' umas, each bred from the other and from her, valued by
`Env::ceiling`, no deck search; reported in `hint`). Each gets a career search (or a surrogate under `sur: 1`). The plan runs no loop search.

**Pass 1**: evaluate each allowed uma's searched or proxy career over its outcomes, giving per candidate the chance her veteran
raises that candidate's odds and the mean odds raised to.
- A builder's career goes through her deck class: her class's winning decks (the main search's and the candidates'
  best careers), valued at her own chances by the surrogate, no career search. She is searched exactly when her lift
  efficiency is within `PROXY_EFF` (6%) of a candidate's second best, or her saving within `PROXY_SAVE` (6%) of the
  lowest that the shown rows (`OPT_SHOW` = 8) or the pick's near-tie band take. A row left on its proxy carries `exact: false`.
  A row whose career search is not `complete` (it ran out of its work cap, `search_work`; a band overflowed; or a cap breach was not resolved by a proven cap-free rerun: the best career it found, not a proven best) carries `proven: false`, and the page
  notes it under the table.
- Before a near-top builder is searched, her proxy is certified where her deck class's proofs allow (`certified`: over
  every legal pair one of the bounds `solve` prunes with (the ceiling, the class's fully checked decks through `rise`,
  its product proofs) is at most the proxy's value up to rounding (`Bar::AtMost`, 1e-12), so it is her best career's
  value; a tie is the usual case, a classmate at the same chances whose proof is the proxy's deck valued exactly); with
  mob or affinity-free parents every classmate has the same chances and one search proves her class. Counted in
  `umas_certified`.
- Cont's two builders come from the exactly searched careers.
- **Looped careers.** Each uma whose career above is not on two 8/8 parents also gets her best career on two real 8/8
  veterans (a legal pair, all of `fix` among them; pairs by ceiling, each searched exactly with both fixed and the best
  looped value so far as its bar, `Env::floor`, stopping when the next ceiling is below it; the surrogate's under `sur`
  1), as a row of its own: only there is her own 8/8 a fully looped
  parent. These rows are not among Cont's builders and are not added to the deck cache. A looped row that repeats
  another row's career is left out.
- **Without a borrowed parent now** (`borrowed_parent` "later"). The rows are the careers the player may run now, while
  everything V and V' read is the job with borrowing: the main search with borrowing (the "allow" job's, kept under its
  key, so a plan after an "allow" run takes it as it is), each candidate's best career and odds, Cont's builders, the
  trios and every scoring's partners and anchors (`outcome_hi`). A base row whose career has a borrowed parent stays a
  builder (its lifts) and is neither scored against V, shown nor picked; her best career without one becomes a row of
  its own after the looped rows (an *nb row*: her cached decks through the surrogate while proxies are on, searched or
  certified exactly on the pool without borrowed parents once a shown row or the pick's band can turn on it; exact from
  the start whenever proxies are off, as under `exact_all`). A proxy row whose exact search turns out a borrowed career
  leaves its proxy career as her nb row. The looped rows are searched on that pool too, also for an uma whose best
  career is looped on a borrowed 8/8. Nb rows
  are not among Cont's builders and not added to the deck cache. The pass-2 band and the outcome tables' pruning read
  the rows that can be picked only, so which proxies are searched exactly can differ from the "allow" plan's, and V with
  it.

**Continuation and V.** Per candidate a continuation (`Cont`, assumed form): builder careers lift her odds with chance
lam_j = lam ((1 - phi)^j)^`LAM_DECAY` (1.5) at the j-th success, each closing phi of the gap in log probability to
p_inf, the greater of the best builder's mean lifted odds and `CEIL_SHARE` (0.6) x her odds with a full loop lineage
(parents hypothetical 8/8s of her two most efficient builders, bred from each other and from her). With only one
builder, p_inf is her mean lifted odds. Her step (a fresh 8/8) costs
sum_{j<n} 1/lam_j + 1/p_n: `Cont::plan` starts at n = 0 and stops at the first non-improving n, with at most
`CONT_N` = 500 successes. `Cont::cost_lo`, used for bounds, takes the minimum over that range, stopping early
only when building alone reaches the best cost. `Cont::plan` and `Cont::cost_lo` are memoised per Cont by the bits of p
(`PLAN_MEMO=1` prints lookups and hits, `PLAN_MEMO_CHECK` recomputes every hit and panics unless bit-identical,
`PLAN_MEMO_OFF`, `PLAN_MEMO_CAP` replaces the 16k cap); `score_career` costs a lifted candidate's step only where it
could still set V's minimum against a lower bound per cell of her odds (`Cont::cost_lb`), the same V to the bit. V = min over trios of side(A) + side(B) + 1/p_C: A and B
candidates, each side either a real 8/8 veteran of hers (cost 0) or a fresh 8/8 (her step; the veteran her best career
now would leave, `Env::veteran`: its parents, every target at the rank's star mix, her G1 wins at their chances), C
every allowed uma at her odds with those two as parents (C's own step is not lifted by builders). The cheapest choice of sides wins: a real 8/8 with poor parents
can lose to a fresh one.

**Pass 2 and the pick.** A career's score is E[V - V with her veteran] - 1 over its outcomes: her veteran as a parent
raises any candidate's odds, her own 8/8 makes her fresh side free, and with two 8/8 parents her own 8/8 is the goal
(V after 0). A target whose stars can't move any table value (per scoring and for every setup, either her miss factor
for it is the same f64 at 1, 2 and 3 stars, or every kept table reads the same with the target landed or not) is folded
at 0 and 1 star only: an outcome with 2 or 3 stars there reads the 1-star outcome's y and lifted odds, and every
outcome is still summed at its own chance, in order. The fold is exact in real arithmetic; floating-point rounding can differ between star levels. A roll
(`next.roll`) reads the same representative. Pick: the highest; within 0.25 careers per run (`NEAR`) or 5%, the better own odds. The goal is always the
next fully looped parent, any uma: a fully looped veteran you have is a real 8/8 (free side) with 8/8 parents.

**Output `next`:**
- `options`: per career `role` (plan / finish / loop / candidate / builder; finish is C's career on A's and B's real
  8/8s, the trios' last step, only when both sides are real and her parents are both 8/8s), `looped` (both parents
  8/8s), `best_odds` (the main search's best career, one row), `index`, `score`, `lift` (the chance her roll
  lowers V), `near`, `stage_cost` for a candidate, `chosen`; `choice`, `mode` (trio).
- `members` (A and B): `{trainee, p, have8, real (the trio uses a real 8/8 of hers), p_inf, lam, phi, builds,
  p_switch, careers}`; `v0` (V); `v0_plan` (`{a, b, c, careers}`); `fit` (the stage candidates in order with their
  continuations, `[chara, lam, phi, pinf]`).
- `outcomes`: the career a roll is of, `{trainee, parents, uncv, fixed}`: the pick, her uncertain targets and the
  scenario's guaranteed ones (job-target indices). The page checks a roll's career against it.
- `roll` (a job with `next.roll`; a carried `roll` returns it as the job's `roll`): `{y, y_hi}` at her outcome (her
  stars per uncertain target, at most 3, and the guaranteed targets landed or not), with `ms` and `prof`, and
  `planned: true` when the plan had to run for it. The plan keeps its pick's roll state (`RollKeep`, the last 4 plans,
  `ROLL_KEEP`, keyed by the plan job without `next.roll`, top-level `roll`, `no_reports` and `compare`; in `state_out`): the pick's last
  scoring as its fold and outcome loop read it (`Scored`: the kept tables, her miss factors and spark scales per
  setup, the collapsed targets, her landed sets' chances, her own 8/8's trio sides), the base it was scored against,
  her own 8/8's trio sides as the bound reads them at the plan's end (`pick_her`) and the main search's found-career
  anchors (`found_anchors`). `y`, the pick's careers saved, V - V with her veteran (to 0.01): the plan's own fold and
  outcome loop (`Scored::evaluate`, generic over the leaves it produces: `All` in the plan, `OneLeaf` in a roll, which folds
  only the prefixes of the outcome's representative and runs the outcome loop at it and the outcome), so the f64 the
  plan's full table held, checked bit for bit against frozen fixtures by `tests/plan_roll.rs`. `y_hi` (rounded up to 0.01) where y < 0.5 (null elsewhere: the page
  reads it only below 0.5, and computes nothing else from it): at
  least `y`, the most her veteran's direct lifts could save for that outcome (each candidate's odds with her as a
  parent bounded over every partner y by the least over anchors A of c' E_A rise(q', q_A) (`Env::rise`: no assumed
  cap), E_A at least the best odds at q_A: today's pairs (y, z) at p_now / c_yz, the main search's found careers of the
  candidate's deck class at their `Ref`s, and, for the partners behind the band's likeliest outcomes, her best deck
  searched at her 2 and 3 stars on every uncertain target (`HI_STARS`; at most `HI_SEARCHES` searches, currently 0, so these extra searches are off; outcomes whose
  partners' bounds pass `HI_RISE` = 1.5 x p_now are left as they are), the latter two under the caps; and by the ceiling
  (`outfit_fast` at q'); the step at `Cont::cost_lo`). The least over anchors is found per (candidate, partner)
  by an exact branch and bound over the leaves' star-digit prefixes (an anchor's factors are positive, so its prefix's
  product times each remaining digit's least factor bounds its leaves from below; a subtree it can't lower is
  skipped; the leaves of no wanted outcome count as nothing to lower, so their subtrees are skipped too: a roll wants
  one, so each anchor's walk is one path), and each wanted outcome's bound is folded in as each partner finishes. A
  roll's first bound keeps, per (candidate, partner), the anchors whose root test passes against the most the ceiling
  holds at any leaf (`RollKeep::partner_anchors`, not stored by `state_out`): every other anchor fails the test at every outcome,
  so later rolls fold in the same anchors without computing their chances again. The
  page checks a roll with y < 0.5 <= y_hi exactly. The replan's other careers with her as a parent are not bounded (its
  candidates and Cont are the plan's).
- `umas_scored`, `umas_searched` (careers searched exactly), `umas_certified`, `umas_unproven` (umas whose best, looped or
  no-borrow career search is not `complete`, as for `proven` above), `hint`, `sur`, `sur_diff`, `work` (this
  plan's deck-search nodes and exact scorings, deterministic: a speed change can be measured at any machine load;
  `work.max_ratio` the largest exact/product ratio scored, `class_max_ratio` / `class_max_deck` per class, `class_caps`,
  `cap` the cap the plan's deck searches hold under), timings. A career deck search whose band would list over
  50,000 decks is rebuilt at 1.12, 1.05, then 1 (`Env::cert`, the lowest completed cap; 0 if every band overflows); its answer is not a proof (no `Ref`), and the output's
  top-level `cap` is lowered to that cap, `complete` set false and `band_overflows` counts overflows. A proven
  cap-free rerun supersedes that call's reduced certificate and overflow count.

**Kept plans.** Each trainee's best careers are kept per plan (the last few, in the worker). A plan of the same setup,
places and pool takes them as they are (no search: a reload, an edit undone); a plan of the same setup and places whose
pool is a kept one plus one veteran at the end (Compare, then Add) takes the better of the kept best and a search with
her fixed, whose bar starts at the kept value (`Env::floor`). `best_deck`'s results (with their certificates, band overflows, work counts
and per-class ratios) and the corner tables are also kept between jobs of one setup and trainee order (`DECK_KEEP`), subject to memo keys and capacity limits. The kept plans, proofs, main searches and roll states survive a page reload through `cmd: "state_out"` /
`"state_in"` (fields table above).

#### Work budget

A career search has its own work budget (`CAREER_BUDGET`, 300M units; `EXACT_BUDGET` in the environment overrides it). It does not stop the search: past it each deck search scores
only its product deck and start decks, so the search is not `complete`, and `solve` goes on to its stop rule (only the
plan's `search_work`, a hard cap, stops it taking candidates). Candidates are kept
as (ceiling, trainee, parents) and their chances recomputed when solved, which keeps memory low on large pools.

### Policy simulator (`simulate` on a career job)

Measures a decision policy (which career to run next) inside one solver process. With `simulate` present the career
job runs the simulator instead of the search; jobs without it are unchanged. Per run, careers t = 0, 1, ... with p_t the
exact P(all) of career t (derived):

    E = sum over t of S_t,  S_0 = 1,  S_{t+1} = S_t (1 - p_t);  stop when S_t < eps or at max_careers, plus the tail S_t / p_t

Each failed career leaves a veteran drawn from the model: landed set from `outcome_sets` without the all-landed set
(inverse CDF over sets ordered fewest-landed first), stars from `star_dist`, per race a win with the schedule's chance
of at least one win, her parents the career's. The uniforms (`u01`) are keyed by seed, run, career, stream and item, so
a run index faces the same uniforms under every policy (common random numbers: differences between policies are
paired). A new veteran is pruned when an owned veteran of the same character with the same two parents has at least
her stars on every target and every G1 she won (exact for P(all)).

The surrogate (policy `greedy`): each candidate (trainee, parents) is valued through its trainee's cached decks' corner
tables (`Cache`, exact P(all) of that deck at any inheritance chances), bounded first by the search's ceiling, by
`rise` from her fully checked decks and by her product-search proofs (`ruled`). Where the bound rb clears the best so far
B and exceeds tv × (1 + m), her deck is searched there (`best_deck` at bar B) and
cached. Derived: the career picked has an exact P(all), with the deck it runs, of at least E*/(1 + m), E* the exact best
career's, subject to the assumed search caps and complete deck searches; m = 0 gives the exact greedy pick up to ties. A candidate is valued once,
when her later parent arrives; a trainee whose static ceiling (q = 1, every spark on six legacies) is below B is skipped.

| Field of `simulate` | Meaning | Default |
|---|---|---|
| `policy` | `greedy` (surrogate), `greedy_exact` (the career search in process per decision: the best career by P(all)), `loop` (a fixed rotation of `loop_members`), `trace` (building for `loop_members[0]` with the others as builders: each career the builder whose best career lifts her odds most efficiently runs; records her odds before and after, the builder, lam and the odds raised to), `index` (the page's plan, *Next career*, above) | `greedy` |
| `runs`, `first`, `seed` | run indices `first` .. `first + runs - 1`; the seed | 16, 0, 1 |
| `eps`, `max_careers` | stop at S < eps or this many careers (tail S/p added) | 1e-3, 100000 |
| `margin` | refresh margin m; -1: every valuation a bound does not rule out gets a fresh exact deck search (exact-backed: the same proofs as the career search, no trust in cached decks) | 0 |
| `prune`, `share_cache` | prune dominated veterans; keep the deck cache across runs | true, true |
| `check`, `check_lineages`, `check_frac` | every `check` new veterans compare with exact searches: the exact greedy pick (career search), and up to `check_lineages` of the newest veteran's candidates with ceiling at least `check_frac` x B (exact best deck, table value, landed-set distance) | 0, 0, 0.3 |
| `audit`, `audit_sample`, `audit_every` | fidelity audit: at each greedy pick change the exact career search's pick and, for the pick and `audit_sample` random candidates with the newest veteran, the table value against a fresh exact deck search (relative error, landed-set total variation); every `audit_every` careers of `loop`, its career and its keep-or-replace decision under exact values | false, 3, 50 |
| `loop_members` | `loop`: rotation members (chara ids) in order; `trace`: the trainee built for, then her builders | none |
| `progress` | log every N careers | 0 |
| `next` | `index`: the plan job's `next` (fields in *Next career*) | `{}` (the page's) |

`loop`: member i mod n trains; her parents are the rotation veterans of the previous two members. A member keeps the
better of her rotation veteran and the new one, by the summed surrogate P(all) of the two loop careers she is a parent
of. Bootstrap: a member without both parent veterans gets the best pair by
exact P(all) from the pool with the existing ones fixed. No pruning. Each career's schedule is her own (the career
model's); the loop planner coordinates schedules across members, this does not.

`index`: each career is the page's pick for the pool as it stands. The simulator runs the plan job the page sends
(`webapp/src/career.js`: the job with the pool as `legacies`, no places, `top` 6, `per_trainee` false, `next`,
`no_reports`; the job's `trainees` kept) through `run_career` in process, and runs `next.choice` (its exact P(all) and
landed-set chances from the report) until the pool changes; a pruned veteran (as for `greedy`) leaves the pool, and so
the pick, as it was. Each step's pool is the last one plus one veteran at the end, so each plan takes the kept plan of
the step before (*Kept plans*: the better of each uma's kept best and a search with her fixed) instead of searching
every uma afresh; the main search runs again each step. Kept state and determinism: the plan jobs read and leave the
kept plans, proofs (`SEEN`), main searches and `best_deck` memos (`DECK_KEEP`); each run starts with all four empty and
the state from before the simulation is put back after it, so a run's careers depend on the job and the run index alone
(the same run alone or among others, in any order, gives the same decisions and sampled outcomes). No checks or audits. Per run it adds
`index`: `plans` (plan jobs run), `plan_s` (each one's seconds), `roles` (the picks' `role` counts) and `best_odds` (picks
that were the main search's best career, the `greedy_exact` pick up to ties). The tail assumes the final pick repeats forever; a plan can finish at a builder
with low odds, so a capped comparison should also report survival at the cap (`tail` × `final_p`, paired by run).

Output: per run `E`, `careers`, `tail`, `final_p`, `pool`, `pruned`, `wall_s`, `refreshes`, `segs` (each pick change:
career, p, pool size, S), `checks`, `audits`; and `E` (mean, sd, 95% CI) over the runs. `trace` returns its rows per run
instead.

The simulator's modules (under `src/career/`):

| File | What it holds |
|---|---|
| `sim.rs` | Shared simulator records, construction, deck evaluation and legacy identity/JSON helpers. |
| `sim/draw.rs` | Keyed uniforms and failed-career veteran draws; pinned uniform test. |
| `sim/policies.rs` | Greedy, loop, trace, exact and index career selection; rotation and index-run records. |
| `sim/audit.rs` | Surrogate checks against exact searches, audit rows and ranked careers. |
| `sim/play.rs` | Path state, veteran admission/pruning, survival accumulation, simulation entry and summaries. |

Adding a policy: `simulate` (src/career/sim/play.rs) matches the policy name to a base (`Base`); `play` asks the base
for the next career each step (`greedy_cur`, `loop_cur`: a `Cur`, the career with its exact P(all), landed-set chances
and win chances), runs it, draws the veteran and adds her to the state (`PathState`). A new policy is a new arm there and a
function returning the `Cur` to run; `play` does the rest (E, the tail, pruning, checks, audits). A policy that runs a
different loop (as `trace` does) returns its own JSON from `simulate`. `index` (`index_cur`) runs a whole career job
per decision, as `greedy_exact` (`exact_cur`) does. These policy methods live in `src/career/sim/policies.rs`.

Results and commands: `val/sim/README.md`.

## 4. Validation

`cargo test --release` (in `pkg/loopopt`) covers, among others:
- **Reference fixtures** (`tests/fixtures/`):
  - 200 best responses from an exact MILP, including forced races;
  - 71 rest-forced best responses from the MILP, including boundary turns 24, 25 and 48;
  - the full 2,278-pair ceiling table.
- **Properties checked:**
  - `Uma::feasible` rejects bad schedules;
  - a force that conflicts with a goal is infeasible;
  - the objective equals a direct per-legacy sum; optional G1s cost nothing; interchangeable races preserve the score;
  - a three-uma rotation's schedules are a Nash point of its objective.

The loader asserts the structural fact the exact DP relies on: every race offered twice is a Classic/Senior pair
exactly 24 turns apart.

## 5. Limits

- **Schedules are heuristic.** `optimize` is an iterated local search with a provably exact best response per uma; nothing proves a structure's schedules optimal.
- **Model assumptions (§1) are assumed or community-derived:** the win formula and the affinity objective. The code doesn't validate them.
- **Base-outfit aptitudes by default.** `build_base.py global CHARA:CARD` writes the alternate outfit's aptitudes to `base.json`; rerun `export_rs.py global` to put them in `data.json`.
