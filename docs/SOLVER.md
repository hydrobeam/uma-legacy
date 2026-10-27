# The solver (loopopt)

The Rust crate in `pkg/loopopt/`; crate paths below are relative to it, while `docs/`, `game/`, `val/`, `webapp/` and
`build.sh` paths are relative to the repository root. Its data is in `data/global/` and `data/jp/` (the Grand Masters
era). The CLI resolves a job's file names from the working directory; run it from the era's data folder.
Building and running it: `docs/BUILD.md`.

Searches parent loops (four-uma loops and three-uma rotations) and support decks for landing target skills in
Independent Training, reports each uma's race schedule, and plans careers to a looped parent (`/career`). Its
inputs are data files exported from the game's `master.mdb` and GameTora's event data.

```
master.mdb ──(export/export_rs.py)──▶ data.json ──(loopopt)──▶ ranked loops + schedules (JSON)
```

## Finding your way

A job comes in through the page's worker (`src/web.rs`) or the command line (`src/cli.rs`). A deck job (the loop
planner) enters at `loop_planner::job::run_deck`, which builds the deck frontier and hands it to
`loop_planner::search::run_multi`. A career job (`/career`) enters at `career::run_career` (`run_career_in` with the
worker's kept state), which builds the job's `Env` (`career::setup`), runs the main search, then the plan
(`career::plan::index_rule`) when the job has `next`, or the simulator (`career::sim`) when it has `simulate`. Both
planners search decks with the engine in `src/deck/` (`DeckSpace::search`; the exact odds in `deck::joint`). In a
folder, open `mod.rs` first: its header maps the children, and it holds the folder's main records and entry point.

| Module | Holds |
|---|---|
| `src/lib.rs` | Crate wiring, deterministic hashing (`Fx`), the native/wasm clock and the CLI entry point (`cli_main`) |
| `src/main.rs` | The `loopopt` binary: calls `cli_main` |
| `src/cli.rs` | CLI job loading, event enrichment, kept-state files, dispatch and bound diagnostics |
| `src/web.rs` | The wasm entry points and their `cmd` requests |
| `src/checks.rs` | The environment switches, read once into `Checks` (`checks::get`), and their thread-local counters |
| `src/data.rs` | Race data, schedule choices, relation and ceiling tables, win probabilities and feasibility |
| `src/uma.rs` | Roster indices and per-uma tables (`UmaIndex`, `UmaTable`) |
| `src/schedule.rs` | Exact single-uma best responses (`Uma::dp_exact`, `Data::best_response`), scratch buffers and scoped memo |
| `src/chart.rs` | Race/member labels and structured schedule charts (`chart_rows`) |
| `src/skills.rs` | The skill-target model (`Model`), outfit choices and lineage inheritance probabilities (`lins`) |
| `src/tests.rs` | Affinity-core tests against independent fixtures and model invariants |
| `src/loop_planner/mod.rs` | The loop planner: a deck job's path through its children |
| `src/loop_planner/dominance.rs` | Dominance signatures, `DominanceIndex`, `k_filter`, candidate decks (`DeckEval`) and odds clusters |
| `src/loop_planner/job.rs` | Deck-job setup and the nondominated deck frontier (`run_deck`) |
| `src/loop_planner/schedules.rs` | Loop affinity scoring, coordinated schedules (`LoopProblem`, `optimize`) and pairwise ceilings |
| `src/loop_planner/search/mod.rs` | Read-only `Multi`, mutable `LoopSearchState`, inputs and keys, setup and `run_multi`; fixed schedules (`prebuilt`) |
| `src/loop_planner/search/tables.rs` | Per-uma exact values, `Grid`, pair ceilings, skyline classes and product ceilings |
| `src/loop_planner/search/shared.rs` | `SharedScan`, shared-deck scoring and cached scans |
| `src/loop_planner/search/bands.rs` | Loop bands, schedule solving and the budgeted certification passes |
| `src/loop_planner/search/report.rs` | Reported picks, JSON and diagnostic summaries |
| `src/deck/mod.rs` | The deck-search engine: the job records (`Card`, `CardsRaw`, `DeckJob`) and the deck model |
| `src/deck/space.rs` | The space of legal decks (`DeckSpace`): entries by chain-kind key, legality tables and caps |
| `src/deck/enumerate/mod.rs` | The enumeration's entry (`DeckSpace::search`) and the records a search takes and returns (`BandBound`) |
| `src/deck/enumerate/prepare.rs` | Search legality, the pass setup (`configure_search`, `search_with_bounds`) and the shared bounds |
| `src/deck/enumerate/config.rs` | Legal card-count configurations and their bounds |
| `src/deck/enumerate/order.rs` | The visiting order of queued configurations |
| `src/deck/enumerate/traverse.rs` | Configuration traversal with its borrowed-card and state bounds |
| `src/deck/enumerate/walk.rs` | The depth-first walk of one configuration's decks (`walk`, `hopeful`) |
| `src/deck/enumerate/bounds.rs` | Outfit bounds and ratio prefixes |
| `src/deck/enumerate/scratch.rs` | Reusable buffers for configuration and node bounds |
| `src/deck/joint/mod.rs` | The joint landing odds' entry points (`joint_b`, `joint_fast`, `joint_checked`), `JointQuery` and the landing factor |
| `src/deck/joint/exact.rs` | The exact sum: `joint_prep`, `JointPrep`, `joint_eval_in`, `evaluate_prepared` |
| `src/deck/joint/reference.rs` | The full enumeration `JOINT_CHECK` compares with (`joint_trainee_ref`) |
| `src/deck/joint/upper.rs` | The negative-association upper bound (`upper_na`) |
| `src/deck/state_bound.rs` | State-conditioned bounds and values (`StateBound`, `state_value`), configuration and ratio tables, `StateBoundMemo` |
| `src/deck/sources.rs` | Each deck entry's target sources (`TargetSources`) and dependence checks, the event data merged into a job |
| `src/deck/chains.rs` | Chain events: count per run (`ChainEvents`), per-link marginals, final-state distributions (`key_level`) |
| `src/deck/hints.rs` | Training-hint draws per scenario (constants from `build.rs`) and their miss chances (`miss_draws`) |
| `src/deck/caps.rs` | The assumed caps per deck class (`EXACT_CAPS`, `cap_class`) and the exact search's budgets |
| `src/deck/pdeck.rs` | The decks a search returns (`PDeck`), their shared names (`Name`) and identity key (`DeckId`) |
| `src/deck/tests.rs` | The deck model's tests |
| `src/career/mod.rs` | The career planner: the job's records, `Env` and its four parts, `run_career` / `run_career_in` |
| `src/career/setup.rs` | The job read three ways (`CareerJobs`) and its `Env` built (`career_setup`), earlier jobs' proofs restored |
| `src/career/run.rs` | The answer once `Env` is built: the main search's output, the plan, rolls, the job's certificates |
| `src/career/search.rs` | Career candidates (`gen_cands`), ceilings and the best careers (`Env::solve`) |
| `src/career/decks.rs` | Exact career deck searches (`Env::best_deck`, `best_deck_run`, `best_deck_cf`) and their kept memos |
| `src/career/inherit.rs` | Legacy slots, affinity, inheritance chances, their folding (`set_ews`) and `rise` |
| `src/career/pool.rs` | Veteran pools and legal parent pairs |
| `src/career/outcomes.rs` | Career reports, outcome sets, corner tables and the veteran a career leaves |
| `src/career/schedule.rs` | Aptitude sparks and career race schedules |
| `src/career/cache.rs` | The deck cache (`Cache`) the simulator and the plan value careers through |
| `src/career/evaluator.rs` | The surrogate's valuation the simulator and the plan share (`Evaluator`, `deck_for`, `Cur`) |
| `src/career/state.rs` | The worker's kept work (`KeptState`) and its saved form (`state_out`, `state_in`) |
| `src/career/clock.rs` | Host clocks, progress and phase counters |
| `src/career/tests.rs` | Career-job tests: kept-state round trips, inheritance copies |
| `src/career/plan/mod.rs` | The page's next career (`index_rule`): its records and phases |
| `src/career/plan/candidates.rs` | Candidate setup, exact and surrogate careers, the proxies admitted |
| `src/career/plan/search.rs` | Candidate searches (`best_sur`, `certified`), rotation setup and proxy limits |
| `src/career/plan/rows.rs` | Plan rows (looped, no-borrow), their scoring and promotion |
| `src/career/plan/score/mod.rs` | A career's score over her possible veterans (`score_career`) |
| `src/career/plan/score/build.rs` | Building a scoring: her setups, `evaluate_career` and the collapsed targets (`prepare_scoring`) |
| `src/career/plan/score/kernel.rs` | The scoring kernels (`Scored::evaluate`, `evaluate_outcomes`, `sc_loop`) and their scratch |
| `src/career/plan/step.rs` | Step costs and the leaves the next career is estimated with |
| `src/career/plan/cont.rs` | The continuation (`Cont`) and its memo |
| `src/career/plan/combine.rs` | Continuation fits and candidate combinations |
| `src/career/plan/fold.rs` | Folding spark outcomes and their rises (`FoldContext`) |
| `src/career/plan/anchors.rs` | Anchor bounds (`outcome_hi`) and the best partners after an outcome |
| `src/career/plan/kept.rs` | Kept plans (`Bests`) and cached career searches |
| `src/career/plan/roll.rs` | The roll of the plan's pick and its kept state (`RollKeep`) |
| `src/career/plan/output.rs` | The assembled plan response |
| `src/career/sim/mod.rs` | The policy simulator's shared records, construction, and the bar and best pick of a path |
| `src/career/sim/play.rs` | `simulate` and `play`: path state, veteran admission and pruning, survival, summaries |
| `src/career/sim/policies.rs` | Greedy, loop, trace, exact and index career selection |
| `src/career/sim/draw.rs` | Keyed uniforms (`u01`) and failed-career veteran draws |
| `src/career/sim/audit.rs` | Surrogate checks against exact searches, audit rows |

---

## 1. Model

**Win chance** of one attempt (the model and its evidence: `docs/MODEL.md`):

```
p = clamp(110 − cost(distance grade) − cost(surface grade) − PEN[s], 0, 100) / 100
cost: S,A=0  B=10  C=20  D=30  E=40  F=60  G=90      PEN[s] = 0,0,5,20,30,50 for s = 1..6+
s = k for a chosen race; s = max(k − 1, 1) for a goal race (scored_position)
career mode: 110 − both costs is 3 points lower where it is 70 or less (LOW_BASIS_COST, LOW_BASIS_MAX)
```

The algorithm tests run on frozen data with older win chances (`tests/fixtures/data_v1.json`: PEN =
0,0,10,25,35,50). `win_table_matches_community_table` checks 13 cost rows and all six streak positions using the
Global data's penalties; it does not check the exported aptitude costs.

- **Streak position k:** counts consecutive turns with *any* race, goal races included. A rest resets it. A goal race
  (an option of one of the career's goals) takes the penalty of position k − 1; it still counts toward the races after it.
- **Low-basis cost (career mode):** the career planner's table (`career_uma`) charges `LOW_BASIS_COST` more where the
  aptitude basis (110 − both costs) is at most `LOW_BASIS_MAX`, both read from `research/results.json` when the solver
  is built (`build.rs`; `docs/RESEARCH.md` §4); the loop planner's base table is unchanged.
- **Turn** = 24·year + 2·(month−1) + half, with year 0 = Junior, 1 = Classic, 2 = Senior: turns 1–24, 25–48, 49–72; half 1 = Early, 2 = Late.
- **Win probability per race:** P_a(r) = E[1 − Π(1 − p)] over uma a's attempts at race r, with the expectation over her shared pink aptitude history. A race can be run in Classic and Senior; it still counts once. For two attempts this is E[p_C] + E[p_S] − E[p_C p_S]; race outcomes are independent conditional on the grades. With certain aptitudes it reduces to 1 − (1−p_C)(1−p_S).

**Objective (expected affinity per 4 runs):**

```
E = base + 3 · Σ_{i<j} C_ij · ⟨P_i, P_j⟩          ⟨P_i,P_j⟩ = Σ_r P_i(r)·P_j(r) = expected shared G1 wins
```

The optimiser uses C_ij = 2 for every pair and base = 2·Σ pair + 2·Σ trio (`C_PAIR`, `K_TRIO`), the normalisation of
the earlier per-legacy split with grandparents at half weight. The adopted model (grandparents at full weight) weights
every pair's overlap 20 times over a loop's 12 lineages (`docs/RESEARCH.md` §6): a different scale, but the same
weight for every pair, so both rank schedules identically; a three-uma rotation likewise weights its three pairs
equally. Test `score_matches_per_legacy_sum` checks the closed form against a direct sum over every lineage and slot.

- Aptitudes: each uma carries `apt` (ten codes 1..8 = G..S: short, mile, middle, long, turf, dirt, front runner, pace chaser, late surger, end closer; from her first outfit unless overridden at export) and `outfit_apt` ([card id, codes] of each outfit whose own differ); `race_cat` / `race_ground` give each race's distance class (0 short .. 3 long) and surface (1 turf, 2 dirt). The loop scheduler reads exported base win chances (the first outfit's); the career schedule reads the aptitudes of the outfit the career runs, with the legacies' pink sparks (`CareerSetup::pink_career`).
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
| `Uma::dp_exact` | Best response for one uma: maximize Σ_r W(r)·P(r) given partner weights W. Junior turns 1–24 run in sequence. Then Classic turn t and Senior turn t+24 are decided **together**, with state (Classic streak, Senior streak, goal bitmask), plus the Classic streak at turn 48 whenever turn 49 has a race option. Every race offered in both years is exactly 24 turns apart, so its reward is computed exactly: E[p_C] + E[p_S] − E[p_C p_S]. The adjusted uma precomputes E[p_C p_S] for each repeated race and streak-position pair by enumerating Classic grades, then Senior grades conditional on them, for distance and surface. The DP state is unchanged. Certain aptitudes retain 1 − (1−p_C)(1−p_S). `Data::best_response` memoises calls within `optimize`. | **Yes** (tested against an exact MILP, §4) |
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
position; `alts` names races that can replace the row's at no affinity cost when all its runners switch together (`chart::interchangeable`).

**Optional fill pass:** after optimizing, each member re-solves against her partners with a tiny bonus (1e-7 per
expected G1 win), keeping the result if affinity drops by at most 1e-6. A second pass tries G1s on rest turns,
including 0% attempts, until no more fit: each addition must preserve every existing G1 attempt's win chance and
keep affinity within 1e-12 of the current score. Test `optional_g1s_cost_nothing` checks that stripping unshared
non-goal G1s leaves the score unchanged within tolerance.

### Fields shared by deck and career jobs

| Field | Meaning | Default |
|---|---|---|
| `skills_file` | outfit skill lists, the skill catalog and the names an event reward can carry (the ○ beside a catalog ◎ included) (`web_skills.json`, from `export/export_web.py`) | required |
| `targets[]` | `group` = skill group id; `guaranteed` 0 none / 1 white / 2 gold; `deck_white`, `deck_gold` = deck hint chance 0–1 | required |
| `star_dist` | spark star mix on the legacies `[p1★, p2★, p3★]`; supplied explicitly for another rank mix; three non-negative shares summing to 1 (within 1e-9), else the job is refused | `[0.50,0.45,0.05]` |
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
| `scenario` | Independent Training scenario: `unity`, `gc` (Grand Concert), `ura`, `trackblazer`, `grandmasters` (JP); sets the training-hint base and specialty weight (Grand Masters, with no runs, uses one base and one weight fitted over the other scenarios) | `unity` |
| `chain_events` | `[n, p]`: chain events per run ~ Binomial(n, p) for every deck, in place of the pal-count rate (experiments) | the pal-count rate |
| `chain_budget` | `[a, b]`: the pal-count rate's logit p = a + b x pals in place of (`CHAIN_A`, `CHAIN_B`) (experiments); refused with `chain_events` | `[-1.09469, 0.168072]` |
| `chain_rule` | `"slots"`: a chain event picks a running card in proportion to its links left instead of every running card equally, and each card-event stop risk ends its chain on its own k of n outcomes instead of 1/3 (experiments; with `chain_events: [21, 0.35]`, the model before the pal rate and the card rule) | `"card"` |
| `stop_risk` | the job's own stops `[card, link, k, n]`: the chain ends at that link on exactly k of n firings (a pin of 1 of 1 is certain), in place of any card-event stop there | `[]` |
| `cap_audit` | audit: every exact cap at 2.0 instead of `EXACT_CAPS`, and ten times the band limit `EXACT_CAP` (deck and career jobs; `val/capaudit.py`) | `false` |
| `cap_free` | career jobs: every deck search cap-free (the search a cap breach reruns, *Cap-free rerun* below) instead of only those that saw a breach; no product-search proofs are kept (`ruled` bounds through the cap). For checks: slower | `false` |
| `min_rarity` | lowest rarity for cards with chain events (2 = SR); cards without chain events (R cards) are always considered, drop them with `exclude_cards` | 2 |
| `events`, `events_p`, `odds_events` | inline event rewards (merged with `events_file`) | `[]` |
| `alts` | events naming two or more skills, in full: `[card, link, [choice][outcome] -> [[group, 1 \| 2], ...]]` (from card_events.json `alts`); at these links they replace `events` / `events_p` | `[]` |

Model (the rates and their evidence: `docs/MODEL.md`, *Deck side*):
- **Training hints**: h = base(scenario) x (1 + hint_frequency/100) x (1 + d(scenario) x specialty_priority/100)
  draws per card per run (`hint_base`, `hint_spec`: `docs/RESEARCH.md` section 7's fit, which `build.rs` reads from
  `research/results.json` and rounds to 3 decimals; hint frequency at max limit break, unique effect included; the floor or ceiling of h, weighted by the fraction), each
  uniform over the card's skill options still below Lv 5 (`nskill`; the stat option is never drawn, docs/MODEL.md; a skill leaves the
  pool after ceil(5 / `hinc`) draws, `hinc` = hint levels per draw at max limit break, so a +5 card never repeats a
  skill); one card's draws over several targets are joint (`miss_draws`). Non-pal cards without chain events take no
  chain slot; their training hints still count.
- **Chain events**: Binomial(21, p) per run, logit p = -1.09469 + 0.168072 x pals, pals the deck's pal cards (group cards
  not counted) and plain pals (`CHAIN_N`, `CHAIN_A`, `CHAIN_B`, `ChainModel`; `chain_budget` replaces a and b,
  `chain_events` sets one Binomial for every deck), each advancing the chain of one of the deck's still-running chain
  cards, each equally likely whatever its links left (the card rule, `PickRule`, `pick_weight`; `chain_rule` replaces it). Pal
  finales are certain. Every chance the solver computes rises with p (a run with one more event is the same run with
  one more pick, whatever the pick rule), so a bound over decks whose pal count is still
  open takes the largest p among them (`ChainModel::most_favourable`: the key bound and card dominance's weakest odds
  at the smallest); a search configuration's decks share one pal count (rate-raising chainless entries are a group of
  their own, apart from group and R cards), so everything inside it is at its own p, and every deck is scored exactly
  at its own.
- **Outcome shares**: an outcome-dependent reward counts at its share of the outcomes, k of n at k/n before measured weights are applied. A measured share replaces k/n where one exists: `chance` in
  manual.json for hand-entered entries (Anshinzawa SSR's finale, Riko Kashimoto SSR's Ramp Up), and
  `event_shares.json` for generated chain events, which the builders turn into the chain shares and `alt_w`, the job
  field the solver uses as outcome weights in place of equal outcomes.
- **Early chain ends**: a choice that ends a card's chain (GameTora's 'ee' reward) is offered as a separate deck
  entry: the card's earlier links as usual, the ending choice's rewards at that link, and its remaining chain slots
  freed for the other cards. Choices that end the chain on only some outcomes are not offered, but when such a choice
  is IT's default (first) choice the chain stops there on 1/3 of its firings, one share for every such link whatever its
  k of n outcomes (`stop_risk` in card_events.json, the job's `event_stop_risk`; `CHAIN_STOP_SHARE`, k/n under
  `chain_rule: "slots"`); a job's own `stop_risk` [card, link, k, n] stops on exactly k of n and replaces the card-event
  stop at that link. The chain odds are the exact mixture over where it stops, freed slots going to the other cards.
- **Links and choices**: multiple links can contribute to acquiring a target, but repeated hints do not generate extra sparks.
  An event's outcomes exclude each other, and where an event offers targets on different choices the card is offered
  once per choice worth taking (a choice no other beats on every target), like an early chain end; the deck output
  names it (`choices`: "link 1: Uma Stan", the skill to prioritise). A hand-entered pal or group entry may name the
  event and choice it models (`event`: [story id, choice]); the event builders then derive `prioritise` from GameTora's
  choice layout and IT's rule (the skills on that choice and on no choice above it; a choice nothing steers to is an
  error), and the page shows it on the deck card (Anshinzawa SSR: Risky Business).
- **Gold and white**: a card's gold and white of one skill are combined exactly (deck `odds`: `gold` = P(gold hint),
  `white` = P(white hint | no gold hint)); `TargetSources::src_miss` accumulates the corresponding miss factors.

The environment switches named here (`JOINT_CHECK`, `CF_CHECK`, `SBOUND` and the others) are read once per process into
`Checks` (`checks.rs`, each field saying what it does), which the code reads where it needs it (`checks::get`). What a search
runs under is passed to it: a band's extra bound (`BandBound`: the class caps alone, the state-conditioned bound, or
cap-free with its node limit) and each deck's chain events (`ChainEvents`). Only the checks' counters (`SBCHK`, `CFCHK`
and the like) are thread-local, totals the CLI prints on exit.

Exact odds. Targets do not land independently: chain cards share one run's chain events (one binomial count), so their chain progress is coupled, and an event's outcomes can give one target or another. A deck's exact value for a trainee at inheritance chance q, V(u, q, D), is computed over the joint distribution of chain progress, given which every source is independent except outcome groups touching two or more targets, which are enumerated per cluster of targets they join (`joint_fast`; `joint_trainee_ref` is the full enumeration, and `JOINT_CHECK=1` compares both in the `joint_checked` wrapper; direct calls to `joint_fast` bypass it). `joint_prep` retains the deck's collapsed states and coupled groups in `JointPrep`; `JointPrep::evaluate` uses fresh scratch, while `joint_eval_in` reuses explicit caller-owned buffers. The search ranks loops and decks by exact values. The product of per-target odds, V̂ (`rank_value` in the output), is only used to bound and order the work:
- **Assumed caps, per deck class.** The capped search assumes V ≤ ρ·V̂, with ρ set by the deck's class
  (`EXACT_CAPS`, `cap_class`, with entry masks from `TargetSources::cv_mask`): 1.05 when at most two targets are fed by chain links (`TargetSources::chain_misses`) and every outcome
  group passes `group_assoc`, otherwise 1.25 (`EXACT_RHO`). `TargetSources::uncertified_association` marks a group that fails that check; merely
  touching two targets does not fail it. These caps are assumptions, not proven bounds. A search node takes the
  class of its cards plus every candidate still to add, so its cap is at least any completion's; bounds that know no
  deck (the loop tables' product fallback, the reroll's `y_hi`, `CareerSetup::ceiling` where an entry legal for the
  trainee is not `certified_cap_free`) use 1.25; the career ceiling of a trainee whose every legal entry is takes 1,
  a proven bound (`ceiling_rhos`: given the chain state no deck's odds pass the ideal point's, and the groups'
  dependence gives P(all) at most the product there). The dependence checks (`group_assoc`, `group_nuod`,
  `group_nuod_lvl`) certify joint ≤ product of marginals up to a relative 1e-12 only (`ORTHANT_RELATIVE`), no
  absolute slack (an absolute term would certify a rare outcome hinting two targets together, joint 1e-14 against a
  product 1e-28). The relative allowance composes over a deck's g certified groups to a factor (1 + 1e-12)^g, about
  1 + g·1e-12: inside the 1e-9 slack of the tests that prune (`Bar::Below`), while a test without slack (`Bar::AtMost`,
  the proxy certificate's tie test) certifies only up to it, so a career above the certified one by less than that
  allowance (and the float rounding, about 1e-15 per operation) is not excluded. Within the work
  budget, decks and loops whose capped product ceilings could beat the incumbent are refined or scored unless another
  bound excludes them. Guards: the output's `class_caps` and per class `class_max_ratio` / `class_max_deck`;
  `val/cmp2.py` and `val/planval.py` fail when a scored deck's ratio passes 1 + 0.75 (cap − 1); job field `cap_audit:
  true` (every cap at 2.0, ten times the band limit) with `val/capaudit.py` checks the decks the caps pruned.
- **Nested chains** (career searches, `deck::nested`). A card whose own chain feeds two or more targets (`NESTED_CHAIN`
  in its `cv_mask`) breaks the class caps however few its decks are: exact / product reaches about 1 / P(its lower
  link fires) for two targets when nothing else gives the later one (4.00 on a Seiun Sky [Paint the Sky Red] deck
  without pals), 1 / (ab) for three targets at links reached with chances a, b, c. For a deck whose nested card j is
  the only chain feeder of its targets K, a bound (`NestedTrainee::deck_bound`): given the chain state the targets'
  factors multiply out and those of K depend only on j's links done d, so V ≤ B = max_o V̂_o R_o,
  R_o = E_d[∏_K F_k] / ∏_K p_k over j's final progress (`chain_states`, at most four states). It is proven, exact in
  the chain law, when K is every chain-varying target of the deck and every group is certified, in any class (a card
  feeding three targets included). Per configuration and trainee a cap η ≥ R_o for every such deck
  (`NestedTrainee::pair_caps`): R_o is linear-fractional in each target's other-source factors, so its largest value
  is at a corner of their range (3^k corner combinations); η is near 1 where the card's links nearly always fire. The
  capped band tests a node, configuration or key that can reach a nested chain at max(class cap, η) (a key at the
  largest η over its configurations' chain events), and such a leaf deck passes when its own bound reaches the bar;
  the capped product search (`SearchMode::BestCovered`) ranks decks by max(V̂, B / 1.25) where η passes 1.25 (node and
  configuration bounds scaled by η / 1.25 there, the dominance skip off), so 1.25 times its best bounds every deck's
  exact value, which the seed, its `Bounded` test and the `ruled` proof read; the deck search's band loop takes each
  deck's bound beside its class cap. A deck within its bound is no breach: its ratio goes to the output's
  `nested_max_ratio`, not `class_max_ratio`; one past it is a breach. Beside other chain cards feeding targets outside
  K, or an uncertified group (chainless entries included), B is the same expression with every other entry at its
  marginal misses: it leaves out the cross-card dependence (the shared chain events), so it holds by assumption, named
  `multi_feeder_cap` (`caps::multi_feeder`). Not bounded, measured too costly (the same corner caps over two-card sets
  cost 1.9x (p10), 2.9x (p16) and 15x (boot_mob) plan time): two cards each feeding one target, held at their class
  cap; a nested card with another feeder of one of its targets, or two nested cards, held at the larger of the class
  cap and their configuration's nested cap η (the leaf as its nodes, and valued max(V̂, η V̂ / 1.25) in the product
  search: no cost on p10, p16 or boot_mob), by assumption. A scored deck past its class's cap but within its nested
  bound (B, or η V̂ where B does not apply) is no breach, but the capped search's prunes around it rested on caps that
  shape strains: the call is answered by the cap-free rerun, the class not flagged
  (`DeckSearchAudit::nested_past_cap`; no val pool has such a deck, `nested_max_ratio`). With Mejiro Palmer [Go Ahead
  and Laugh] allowed in the nested_nopal fixture, beside Seiun Sky [Paint the Sky Red], the two-card deck's exact /
  product is 2.71: before both changes the capped search answered Seiun Sky alone (P(all) 0.005472) and reported it
  proven, against the cap-free 0.005884 (`tests/cap_shapes.rs`).
- **What "proven" rests on** (career searches: `complete`, `proven_top`, a plan row's `proven`). Derived and exact,
  given the dependence checks (relative 1e-12): the per-career odds, the rise bound, the ceilings of trainees whose
  every legal entry is certified (ρ = 1), the nested-chain bound for a nested card alone feeding every chain-varying
  target (in any class, up to `NESTED_TARGET_LIMIT` = 8 targets for its configuration cap: a card feeding more gets no
  cap, its decks are class 3 and their caps assumed under `multi_feeder_cap`), the state-conditioned and cap-free
  bounds, `upper_na`. The `Ref`, `ruled`, Twin and dominance proofs transfer exactly what a deck search proved, so
  they hold as far as the capped search behind them did: under the class caps and named assumptions below wherever it
  pruned by them. Assumed, each with the worst ratio known:
  - the class caps, from measurement: class 0 and 1 at 1.05, classes 2 and 3 at 1.25 (largest scored ratios 1.127 and
    1.18 on validation decks), and the ceilings of trainees with an uncertified legal entry at 1.25;
  - `two_feeder_cap` (`assumes`): class-1 decks of two chain-varying targets fed by two or more chain cards (two cards
    each feeding one target: about 1.17 at worst in constructed decks, five SSRs and one pal, each target given only by
    one card's last link, held at 1.05; a nested pair with another feeder: 2.71 constructed (Mejiro Palmer [Go Ahead
    and Laugh] beside Seiun Sky [Paint the Sky Red] without pals), held at the larger of 1.05 and η, and up to the
    nested card's own 4.00 as the second feeder weakens), listed wherever a search pruned a configuration or key that
    can hold one;
  - `multi_feeder_cap` (`assumes`): class 2 and 3 decks with a nested card beside other chain feeders or an
    uncertified group, held to the larger of the class cap and the nested card's bound B with the cross-card
    dependence left out: in the unit test's constructed nested pair beside a third target's feeder exact / B reaches
    1.025 while exact / max(1.25 V̂, B) stays at most 0.91, and no deck scored on p10, p16 or boot_mob passes it (no
    `cap_exceeded`).
  Every deck a search scores exactly is checked against its cap or bound: band decks, the product deck, and start
  decks, which are scored before any answer the caps give where they may clear the bar (`upper_na`; one that clears
  it there does so only past its cap); a breach is caught, answered by a cap-free rerun, reported (`cap_exceeded`,
  `cap_breaches`, `cap_breached`) and leaves the search and its class unproven; a deck a cap pruned before scoring is
  not seen, which is what the assumptions cover.
- **Class flags** (`ClassFlags`, per deck class; career jobs). What a class's certificates (its fully checked decks,
  product proofs, a kept plan's proofs) rest on: `breached` (a capped deck search of it breached its cap) and the
  named assumptions its searches pruned under (`Assumes`: `two_feeder_cap`, `multi_feeder_cap`). A flag is only ever
  set: it is kept by class content (`CareerSetup::class_key`: the setup and the class's deck problem, whatever the
  trainees, pool or class index; `KeptState::flags`, never evicted, across jobs and in the stored state), every job
  starts from the kept flags and keeps its own, and a restored main search, a kept plan (which carries the flags it
  rested on, `Bests::flags`, stored with it) or a stored state unions its flags in, never replacing them. A breached class's certificates are dropped and bound nothing (`solve`,
  the proxy certificate, the evaluator); an assuming class's are kept and used, proxy certificates included. Every
  answer reports the flags of the classes it rests on, as they stand when it is given: a career search its trainees'
  (`unproven: ["cap_breach"]`, then not `complete` nor `proven_top`; `assumes`), a kept main search reused after a
  later search flagged one of them included; a plan row its trainee's (`proven: false`, or `assumes`), at the end of
  the plan, so a flag a later plan search set reaches the rows and the main search before it; the job's
  `job_unproven` / `job_assumes` every class it answers for. A stored search that says it is proven with a class of
  its trainees breached is refused (`state_in`).
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
  their `work`; `cap_exceeded` still lists the classes past their caps. A breach refutes the cap for that deck space,
  so its class's product proofs (`ruled`, which bound through 1.25) and fully checked decks are dropped and no more are
  kept (`ClassFlags::breached`, kept by class content), and every career search of the class is not `complete`
  (`unproven: ["cap_breach"]`): the rerun proves its own deck search only, while the search's other prunes of that
  class rested on the cap. A search that never scores a breaching deck has nothing to trigger it, so "proven" stays
  relative to the caps where no breach was seen. Checks: `CF_CHECK=s` runs the cap-free search
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
  at least its factor on every chain state. Event outcomes enter at their marginal chances after `group_orthant_certified`
  checks their dependence at every level (a target's factor rises with gold as well as with any hint): `group_nuod`
  for a white-only group, `group_nuod_lvl` for one giving gold. U = Σ_n P(n) Π E[· | n] ≥ V. The bound returns `None` when its dependence checks fail;
  callers then use another bound or score exactly. `UPPER_CHECK=1` asserts U ≥ V for covered decks scored through
  the loop search's exact-value cache.
- **Loop-search state.** Setup builds `Multi` and `LoopSearchState` once; `run_multi` owns both across certification
  passes. `Multi` holds the job inputs, skyline classes and clusters, fixed ceilings and ranked pool. Cache-filling
  phases take `&Multi` and `&mut LoopSearchState`: the latter holds the two chance-grid tables, per-uma and shared
  exact results, schedules, two cluster scratch buffers and `Tally`. Scoring takes `&mut Tally`, cluster scans take
  their buffer by `&mut`, and reporting reads `&Tally`. `LoopSearchState` also owns the job's `StateBoundMemo`, passed
  mutably through deck enumeration. `DeckSpace` retains its other interior caches and node counter.
- **Per-uma tables.** X(u, i) = the best exact value of trainee u over every legal deck at grid chance i (64 points); V rises with q, so a lineage at chance q is bounded by X at the next grid point up. Cells are computed lazily: a loop's ceiling uses each cell's exact value when known, else the lowest known value at a higher grid point, else ρ times the product ceiling; while the ceiling beats the k-th best, the loosest term is refined (first at the uma's highest grid point, then halfway down).
- **Loops.** Structures of both sizes are generated in bands of a separable bound, the mean of the members' best product values M_u at their highest chances (a member heads 3 of 12 lineages in a loop, 1 of 3 in a rotation, so the product ceiling is at most the golds' factor times that mean; bands are kept in four-member sum units, a rotation's sum s counting as 4s/3); a structure is solved (schedules, then exact values) only when its exact ceiling at the pairwise-ceiling chances, then at its own schedule's chances, can still beat the k-th best exact value; the search stops when no ceiling (times ρ for the ungenerated ones) can.
- **Shared deck** (`per_uma` false): a loop's value is the best exact sum over its lineages (12 or 3) among decks legal for all its members, divided by their number; ceilings are the lower of the per-uma tables and ρ times the best single skyline deck's product value over its lineages. `top_decks` counts distinct decks, each with its best loop; per loop, every deck at or above the k-th best distinct deck so far is kept.
  With fixed `pals` and several requested decks, a complete small deck catalogue also bounds each deck against the
  greater of the k-th value and that deck's own incumbent. Per-member exact values at maximum inheritance chances
  bound the structures worth visiting; upward-rounded chance-grid values tighten that bound before scheduling. These
  values and the product prefilters take the deck at the most favourable chain events any loop's copy of it can have
  (`DeckSpace::deck_events_most`: every plain pal counted as raising the rate, since which plain pals a loop's members
  leave legal decides it); a deck filled in for a loop is scored, and its odds reported, at that loop's own events.
  The ordinary queue retains its order and ties. Exhausting these candidates certifies the distinct results
  (`next_bound` is then the k-th value, a bound on new or improved results). A complete catalogue also caps the
  requested count when fewer usable identities exist; missing skyline seeds are filled from that catalogue before
  the per-loop band applies its k-th cutoff. Catalogue or structure-list overflow falls back to the ordinary
  search; an incomplete list never certifies pruning.
- **Certificate and budget.** The pass at ρ = 1.25 (each class at its cap) runs first; if it exceeds the work budget (`EXACT_BUDGET`, default 30M work units, independent of machine load), passes with every cap at most 1.12, then 1.05 follow (likewise when a deck search would return over `EXACT_CAP` = 50,000 decks, `cap_hit`), each reusing every exact value, table cell and schedule already computed; if none finishes, a pass at ρ = 1 (product-odds pruning, no deck searches) answers. Output `exact`: `rho` the cap the answer is certified under, `max_ratio` the largest V/V̂ on any deck scored; `class_max_ratio` must be compared with each class's cap at the returned `rho`, `complete`, counts. Job field `exact: false` runs the ρ = 1 pass alone; its returned decks are still scored exactly.
- `p_all`, `p_minus1`, `per_trainee` and the detail's per-lineage `p_all` are exact.

"Proven" (`proven`, `exact.complete`) means the search ended by its stop rule under the certified cap. It is relative to the schedules the optimiser finds (iterated local search, not proven optimal; `exact.schedule_rejects` counts loops rejected only after their schedule was built) and to the cap, which is assumed: a scored deck whose ratio passes its class's cap shows in `class_max_ratio` but leaves `complete` as it is here (the career search reruns the affected deck search cap-free, *Search and output* below).

Dominance tests in the skyline enumeration use a 64-bit signature (16 coordinates × 4 bits) as a fast necessary
check before the exact comparison.

Skill model: a target is acquired if guaranteed, on the trainee's own outfit skill list, hinted by the deck, or
inherited; an acquired skill becomes a spark at the generation rate. The equations and fitted values (inheritance,
per-legacy affinity, spark generation, star mix) are in `docs/RESEARCH.md` §1-5. Odds are averaged over the
structure's lineages (12 in a loop, 3 in a rotation).
A loop lineage (T, P1, P2, X) is a trainee T, two of the other three members as her parents P1 and P2, and the fourth
member X, each parent bred from the other parent and X.

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
Same files as deck mode, plus `legacies` (each with at most two recorded `parents`, else the job is refused); the shared and deck fields apply (`owned`/`exclude`/`exclude_outfits`
choose the trainees, `owned_cards`, `borrow`, `scenario`, ...; `star_dist` sets the stars of a veteran before her result is known: the plan's fresh full veterans, the simulator's draws; other loop fields are unused).
`uv run val/career_job.py TRAINED_CHARA_JSON BASE_JOB OUT_JOB` builds a job from an umadump export, as the page does.
Several career jobs run in one process with `target/release/loopopt data.json a.json b.json ...` (one output line each),
sharing the first job's skill and card files. A job's fully checked decks are kept per setup, trainees and pool
(`CareerBuild::seen_fingerprint`: `setup_fp`, which excludes the pool, trainee and parent filters, result counts and
page request flags, with the trainees, whose order sets the deck space's plain pals, and the veterans and borrowing
rule) and bound later jobs' searches of the same pool. The simulator's seed search disables these kept proofs. Timings
and work counts depend on reuse.
The page runs a career as two jobs: the search, then the same job with `next`, which reuses the
search the worker keeps (`KeptState::searches`, the last three, keyed by the whole job without `next` and `no_reports`) together with what its
product searches proved (`SearchState::ruled` as the search left it). `tests/plan_state.rs` compares separate
search/plan jobs with a combined job, and a job after a job on another pool with the job alone. The product-search
proofs go no further than their own search, and the fully checked decks no further than their pool: `certified` takes a
proxy career a proof certifies in place of an exact search, which may name another pair or deck of equal value, so
proofs from other pools could change an output. (A fully checked deck holds in any pool of the setup; the key keeps
the career named at a tie from depending on what the worker ran before on another pool.)

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
| `career` | a career id (`career` in a report: `kept::career_id`, the uma, her parents in order (legacy indices) and her deck by card identity: each named entry's card id, choice variant and early-end link, the plain pals' card ids, the filler count, so an id names the same deck in every job on the same cards and veterans) to report as it is, with no deck search (`career/given.rs`): needs one trainee and both parents fixed, the id's; its hint chances are worked out from its cards (`DeckSpace::hint_chances_at`), so its odds can differ from the report that named it in the last bits; the deck must be one of this job's for her (every entry in this job's deck space, none of her own card, one per character, every locked card held, at most the borrowed cards allowed, the plain pals the first she may take, the deck size); the answer has one result, `given: true`, and neither `complete` nor `proven_top` (no search compared it with another career). The page's Use this sends it for a builder's other career (`builder_careers`), which a search on its parents would replace with her best deck | none |
| `cmd: "state_out"` / `cmd: "state_in"` | the worker's kept work across page loads: `state_out` returns `{ok, state}`, the kept plans (each uma's best career per pool, as above), the kept proofs (fully checked decks per setup and deck class), the kept main searches (`KeptState::searches` with their product proofs, so a reload's first run searches nothing again) and the plans' roll states (`KeptState::rolls`, *Next career*: a reload's first roll plans nothing again), every float as its bit pattern in hex; `state_in` with that `state` puts them back (all or nothing: a state that does not parse, or of another version, is refused with `error`; so is one with a stored count above 1024 (`STATE_MAX_N`; a count is at most the umas in the data), an index past its list, a NaN, a chance outside [0, 1], a deck of more than six cards or a roll state whose vectors do not fit together or exceed the plan's 12 targets, 10 uncertain. Stored decks and careers are checked again where they meet a setup (`CareerSetup::seen_fits`, `solved_fits`): one naming a deck entry, trainee or veteran the setup does not have is not used there). The career page stores it in IndexedDB keyed by the solver's and data's fingerprints, after each plan, and restores it before a new worker's first job. Natively: `LOOPOPT_STATE_IN` / `LOOPOPT_STATE_OUT` (files) around a career run | none |

#### Model per career

Model per career (T, L1, L2): the six legacies are L1, L2 and their recorded parents. Affinity as in the loop model
with the recorded G1 wins (shared = same race index; the page maps win saddles by saddle group, so a race at a
relocated venue is the same race). Per target k: q_k = 1 − Π_legacies (1 − min(1, 0.03·s·(1 + A/100)))², s = that
legacy's stars of k (0 without a spark); spark generation 0.2 / 0.4 × 1.1^n_k, n_k = legacies carrying k. Derived:
q_k and the trainee's own event hint chance e enter every landing factor only as (1 − e)(1 − q_k), so the trainee's e is
replaced by 1 − (1 − e)(1 − q_k) and the deck search runs unchanged at q = 0; the factor 1.1^(n_k − 6) per target is the
same for every deck and multiplies P(all) (`JointExt` applies it per target for P(all but one) and the per-target
chances, which are exact marginals of the joint model). Tested against the loop model by `career_fold_and_scale`.

`run_career_in` builds `Model` once; `CareerSetup` borrows it read-only. `Model::event_chances` always holds the own-event chances.
A job's `Env` owns four records, split by who changes what. `CareerSetup` (data, model, targets, deck space, odds
ceilings, outfit coverage, deck classes, borrowing rule) is built once by `career_setup`, which also restores the proofs
earlier jobs kept, and is only read after it (its methods take `&self`). `SearchState` is what a search changes: the
folded buffers, work and budget, exact scorings, seen decks, product proofs, the pruning floor. `DeckSearchAudit` holds
what the deck searches certified (certificates, band overflows, cap ratios and breaches, cap-free counters, the decks
that set the ratios); `DeckSearchMemo` owns the deck and corner memos kept between jobs, plus `StateBoundMemo` for
this job alone. State-bound configurations are keyed by the fixed deck space's entry set and configuration; enumeration
borrows that memo mutably beside `&DeckSpace`. `Env::solve` and `Env::best_deck` take the whole `Env` (`&mut self`); the
setup's methods take what they change as `&mut` beside `&self`, so the cap-free check beside a search saves and
restores the audit as one value.
`SearchState::folded_events` owns one reusable outfit/target buffer per trainee; `set_ews` refills it in place for each
lineage or corner. Exact scoring and its bounds read the chosen chances through `JointQuery::event_chances`;
enumeration borrows `DeckChances` (the model plus the selected event-chance table). Loop searches select the
unfolded table. The career deck memo keys the folded buffer; class and rise calculations read `Model::event_chances`.

#### Search and output

Search: each candidate gets a ceiling ρ·C·max_outfit Π_k f_k at each target's best possible deck odds (the deck_size
lowest miss chances of the trainee's legal entries, every link firing), ρ = 1 where every entry legal for the trainee
is `certified_cap_free` (a proven bound, `CareerSetup::ceiling_rho`) and 1.25 elsewhere; candidates are taken best
ceiling first. Rankings across trainees that keep only a few (the plan's hint candidates) take 1.25 for every trainee
(`ceiling_uniform`), so the certificate does not reorder them; within one trainee the factor is one number, so her
pairs' order (`ranked_pairs`: `best_sur`, the looped pairs) is as before and only their stop comes sooner. A
candidate is solved like a per-uma trainee (best product deck, then every deck whose product value times ρ reaches
the best exact value, through `upper_na`, scored exactly). Between candidates of one trainee a proven bound skips
most of them: from chances q0 to q a target's factor grows at most by (1 − (1 − e)(1 − q_k)) / (1 − (1 − e)(1 − q0_k))
(1 when q_k ≤ q0_k or the outfit has the skill), for every deck and chain state, so E(q) ≤ E(q0) · max_outfit Π_k of
those ratios. The same bound holds between trainees of one deck class (equal outfit coverage and own-skill chances of
the targets, the same relevant cards barred, as many plain pals: the best deck is then one function of the chances), so
the fully checked decks and product-search proofs kept between searches and jobs (`seen`, `ruled`) are kept per class:
a classmate's proof bounds her candidates too. `rise(t, q', q)` and every proof (`Ref`, `ruled`) depend on the trainee only
through her class, so the sharing is exact; within one search a classmate's fully checked decks bound a candidate as
her own do (her deck searches still start from her own, each distinct deck once). The planner's deck cache holds exact winners only: pruning, the proofs kept
between jobs and class sharing change which decks a search happens to check fully, and a cache of those would make the
plan depend on search history. Identical
deck problems are searched once: a candidate whose trainee, chances and spark scale are bit-identical to an earlier one's
in the same search takes its outcome (`Twin`: interchangeable veterans, as a sample pool of mob veterans gives, make
every parent pair one problem; a repeat adds no second Ref or proof), and a deck search without start decks is
memoised per job by what it reads (deck class, folded chances, spark scale, bar, work spent; `DeckSearchMemo::decks`):
a call repeats only with the same work spent before it (between jobs, and where a plan's search starts afresh).
The search stops when the next ceiling is below the k-th best (one per trainee when `per_trainee` is true). Output per
career: `trainee` (`outfits`: those of her deck's outfit option at the grades her schedule runs or at grades whose
schedule scores as well (within 1e-12, relative), the schedule's own first, empty for "any outfit" only when none
covers a target and every outfit of her character has the same grades; `covers`; `schedule_outfit`: the card
id whose grades her schedule and aptitudes use, the option's best for the schedule, 0 for her character's), `parents`
(legacy indices), `slots` (six legacies: affinity parts, stars),
`targets` (per target `q`, `carriers`, landing chance `p`, and `buy`: the chances she buys it as gold / white and lands
it from each (`gold`, `white`, `land_gold`, `land_white`; land_gold + land_white = p to rounding: `gold` is exact over
the deck's chain states, from a second evaluation at q = 1, where the landing marginal is affine in it, and the white
part is the rest of p) with its sources `deck_gold`, `deck_white` (the deck's per-target hint chances, its sources'
product at their marginal chances), `own_event`, `inherit`, `outfit` (0, 1 her white, 2 her gold), or `scenario`), `pinks` (per aptitude among the six legacies: `q` = chance at least one of
the two inheritance rolls passes it on, 1/3/5% for 1/2/3 stars × (1 + affinity/100), capped at 1 per roll, and `carriers`; reported only: aptitudes
do not enter the skill odds), `aptitudes` (her grades and pink spark, MODEL.md: `base` and `start` codes in `apt` order,
`fire` = each legacy pink with its chance to fire at one April inspiration, `pink_gen` = per aptitude the chance her
veteran's pink is that one; with a veteran before her result is known among the six, `outcomes` ([{p, start, fire}],
one per combination of the pinks such veterans may carry) in place of `start` and `fire`; her schedule's win chances
count them, combining repeated races over the shared grade history as above; a hypothetical veteran
(`CareerSetup::veteran`) carries `pink_gen` split by the 20/70/10 star draw as her possible pinks, which her children's
`pink_career` expands into those outcomes (exact given her pink is independent of her win chances, which share her
aptitude history: unmeasured, second order; a later slot of her character with the same possible pinks is taken as
her, sharing the outcome); absent without aptitude data), `p_all`, `p_minus1`, `expected` (Σ p), `deck`; and `candidates`, `solved`,
`bounded`, `complete`, `proven_top`, `unproven` (why the main search is not complete, when it is not: `work_cap`, the
work budget or a hard cap ran out; `band_overflow`, a band was certified at a lower cap only; `cap_breach`, a class of
its trainees is breached, *Class flags* above, as the flags stand at the end of the job), `assumes` (the named
assumptions what it proves rests on: `two_feeder_cap`, `multi_feeder_cap`, *What "proven" rests on* above),
`work` (the main search's deck-search nodes and exact scorings, deterministic; 0
when the worker kept the search from the job before). A scored deck whose exact / product ratio passes its class's
cap (`EXACT_CAPS`, an assumption) has its deck search rerun cap-free (*Cap-free rerun* above), and the search it was in
is not complete. The output lists the classes in `cap_exceeded` ([class, highest ratio, cap]), the capped deck
searches that breached in `cap_breaches`, the reruns in `cap_rerun`, the trainees whose class is breached in
`cap_breached` (chara ids), the reasons any search of the job (the plan's included) or the flags of a class it answers
for leave an answer incomplete in `job_unproven` (band overflows, breaches), `job_assumes` likewise for the named
assumptions, and the largest ratio of a deck past its
class's cap that the nested-chain bound covered in `nested_max_ratio`. The best career also carries `schedule`, `g1_wins` and `outcome_sets`:

- `schedule`: every G1 she could enter, in turn order, as the loop planner's chart rows for one member (`race` the
  chart name, `when`, `year`, `cells` = [{p, goal, apt, low, row, pen}] or [null] (`apt`: her expected grade costs
  with her pinks; `low`: her expected low-basis cost; rounded so `apt` + `low` = 110 − her expected career basis), `shared` = a parent won it)
  plus `name` (full race name), `parents` (which won it), `run`/`skipped`, `instead` (the race she runs that turn, or
  "rest") and `p` (her win chance, null when skipped). The schedule maximizes expected distinct G1 wins, each race weighted
  by the chance each parent won it (`CareerSetup::plan_schedule`, per distinct grades among her outfit option's outfits, the best kept; a G1 she shares with a parent adds 3 affinity to each of them
  when she is a parent herself, and her parents' wins guide what later partners will have won), under goal races,
  same-turn clashes and streak win penalties. Every G1 also weighs `FILL_EPS` (1e-7, the loop planner's tie-break), so a
  G1 neither parent won is run where it costs the parents' races almost nothing: a bounded scalar tie-break (at most
  `FILL_EPS` x the G1s added, about 3.4e-6 expected weighted wins; exactly lexicographic on base tables with certain
  parent wins, whose values lie on a 1e-4 lattice). `g1_wins`: expected wins.
- `outcome_sets` (up to 12 targets, else null): the exact chance of each set of targets landing, indexed by bitmask
  (bit i = job target i), guaranteed golds' spark generation included.

#### Next career (`next`)

**Vocabulary:** a **full veteran** carries every target spark, for any number of player-selected targets.
A **fully looped parent** is a full veteran whose parents are both full veterans.

The page's plan (`career::plan::index_rule`): exact where it can be, not simulated. The page sends it as a second job,
`next: {}` with `no_reports: true`, after the search job (it reuses the search the worker keeps). The model and its
evidence: `docs/MODEL.md`, *Next career*. The planner is in `src/career/plan/` (`mod.rs` and its children).

| Field of `next` (all optional) | Meaning | Default |
|---|---|---|
| `sur`, `sur_margin` | 1: each uma's best career from the simulator's surrogate (`best_sur`: every legal pair by ceiling, her cached decks, an exact deck search where the bound beats the table by more than `sur_margin`) in place of the career search; 2: both, the differences listed in `sur_diff` | 0, 0 |
| `exact_all` | every uma's best career searched exactly (no proxies) | `false` |
| `proxy_check` | disable proxies for the main plan and report each builder's proxy beside her exact career (`proxy_check` in the output, each row with its `origin`, base / looped / nb / alternative, and `looped`: a looped row's proxy is her unconstrained career, so only base rows compare like for like); also evaluate looped rows' lift statistics | `false` |
| `proxy_eff` | test knob replacing `PROXY_EFF`; `tests/plan_memo.rs` sets -1 (pass 1's builders left to their proxies, so the pass-2 searches refit Cont) | 0.06 |
| `fit` | `[[chara, lam, phi, pinf, p0], ...]` (a plan's own `fit` output): the stage candidates and their continuations taken as given instead of fitted to this pool, each anchored at the odds p0 it was fitted at (a 4-element entry, written before p0 was, is anchored at the candidate's odds now, so it re-anchors). The page's Compare prices the pool with her under the plan's own `fit`, passed on unchanged, where today's plan has a V and a fit (else it refits; Add to veterans always refits) | none |
| `partners` | partners kept per candidate where a scored career's veteran is a parent (1 to 8) | 2 |
| `fresh_lineages` | fresh lineages a trio side may take per candidate: her best career now and her next `fresh_lineages` - 1 by her cached decks (1 to 64) | 3 |
| `builder_careers` | careers per builder scored as rows: her best and her next `builder_careers` - 1 distinct (parent pair, deck) careers through her cached decks, other decks on her best career's own parents among them (rows with `alternative: true`, on the search pool, scored for their lifts too, so Cont's builder choice reads them, and a plan under `borrowed_parent: "later"` fits what one under "allow" fits; not added to the cache). The pick, where it is one, is searched exactly on its pair (`exact_pair_career`) before it is shown, and scored again where that finds another deck (1 to 8). The default is 1: measured 2026-10-26 on the 13 val/plan pools (`robust: "all"`, `pair_search`), the pair search's recommendation for the pick's trainee is the same at 1 and 3 on 11 (gm16_nu8: Agnes Digital [10,15] 2.682 at 1, Yukino Bijin [14,15] 2.21 at 3; p16_t10: Inari One [10,15] 1.87 at 1, Oguri Cap [10,15] 1.937 at 3; neither is gm16_nu8's exhaustive best, Oguri Cap [10,15]), and 3 about doubles the plan's time (p16 0.7 -> 1.4 s, gm16_nu8 2.6 -> 6.0 s, p16_t10 10.3 -> 22.7 s). The extra careers enter Cont's builder evidence, so V and every score move with the setting: V lower at 3 by 0 to 7.3% (p16_t10 9,885.1 -> 9,165.5; p10 -5.8%, p10_pink -6.1%; p16_borrow 214.9 -> 211.4, against the greedy policy's 198.1) and the same career's score moves by a median of 0 to 0.36 careers per pool, at most 1.14 (p16_t10), over the base rows both settings list (p16's pair-search best Agnes Digital [10,15]: 2.631 at 1, 2.444 at 3). That spread measures how far the scores depend on Cont's builder evidence | 1 |
| `audit_lifts` | measurement: for the pick and the top 3 builder rows, at each one's 5 likeliest outcomes that land a target, each liftable candidate's odds as the row's own kept scoring read them (`table`: her cached decks, the kept partners; null where the plan kept none), as the row scored again reads them (`table_rescored`), with every partner (`all_partners`) and her best career with that veteran as a parent searched exactly (`exact`, `complete`), in `lift_audit`; every row from the plan's cache and proofs as its numbers were taken, put back before each row and after | `false` |
| `loop_members` | loop members to add as stage candidates; absent: none (the plan runs no loop search) | absent |
| `search_work` | work budget (career-search units) of each candidate search beyond the job's own; 0 = the job's budget | 5,000,000 |
| `margin` | the surrogate's refresh margin (section *Policy simulator*) | 1e9 |
| `roll` | a roll of the plan's pick: her stars per job target (0 where it did not land); the output is `next.roll` (below). Answered from the plan's kept roll state with no search when the worker has it for this job, else after the plan, which keeps it | absent |

**Stage candidates** (at most 8, `IDX_CANDS`, unless supplied by `fit`): the best career's trainee, the loop members when named, the umas of the best
career's lineage (its two parents' umas, then its four grandparents', each generation in uma order), the main search's trainees (best first), then
the allowed umas with the highest odds under a full loop lineage (parents hypothetical veterans of the first two candidates' umas, each bred from
the other and from her, valued by `CareerSetup::ceiling`, no deck search; equal ceilings in uma order; reported in `hint`, the ranked umas left
out in `cut`). The set and its order read which veterans the best career and the main search's careers take, never the veterans' positions:
p10 with its legacies reversed gives the same candidates and V (2653.99; before the lineage was ordered, parent-position order seeded the
ceiling lineage and the reversed pool took 1008 and 1011 for 1005 and 1040, V 2668.33). What is not covered: two of the main search's careers
at exactly equal odds keep the search's own order, which can depend on the pool's. Each gets a career search (or a surrogate under `sur: 1`).
The plan runs no loop search.
The main search's trainees come before the ceiling because the ceilings can be nearly flat: on p11_noagnes they span
2.5%, and ranking by them alone cut V's B (Special Week, 6.6e-4 now) and raised V by 2.2%.
**Side-only umas** (`side_only` in the output): every uma with a real full veteran a career may take as a parent
(borrowed ones where the borrowing rule lets one alone be a parent), whatever the trainee filter, that is not a stage
candidate. Her real full veterans are trio sides (free); she has no career search, odds, step, continuation, fresh side
or lift slot, and does not count toward `IDX_CANDS`. Measured 2026-10-26: p16_88 with its legacies reordered (El Condor
Pasa's full veteran first) gives V 288.61, the same as in order (before: 385.56); with Tamamo Cross excluded from
training her full veteran stays a side (V 288.61; before 432.28); p16_borrow with Oguri Cap untrainable gives V 214.85,
the same as with her trainable (before: 357.93).

**Pass 1**: evaluate each allowed uma's searched or proxy career over its outcomes, giving per candidate the chance her veteran
raises that candidate's odds and the mean odds raised to.
- A builder's career goes through her deck class: her class's winning decks (the main search's and the candidates'
  best careers), valued at her own chances by the surrogate, no career search. She is searched exactly when her lift
  efficiency is within `PROXY_EFF` (6%) of a candidate's second best, or her saving within `PROXY_SAVE` (6%) of the
  lowest that the shown rows (`OPT_SHOW` = 8) or the pick's near-tie band take. A row left on its proxy carries `exact: false`.
  Every builder is searched exactly (no proxies, as `exact_all` searches them) in a job with 10 targets or more
  (`EXACT_BUILDERS_TARGETS`) or with a borrowed veteran in the pool (`exact_builders`): there `proxy_check` measured
  proxies further below the exact careers than the band (efficiency 46% low on p16_t10, 10 targets of which 8 uncertain;
  6.4% low on p16_borrow), and the exact searches cost little or save time (p16_t10 13.8 s exact against 19.3 s with
  proxies; p16_borrow 1.3 against 1.2 s; 2026-10-27). Elsewhere exact costs more (p10 3.37 against 2.21 s, p16 1.38
  against 0.83, gm16_nu8 5.59 against 3.75) and proxies stay.
  A row whose career search is not `complete` (it ran out of its work cap, `search_work`; a band overflowed; or a capped deck search breached its cap, the rerun proving that deck search alone: the best career it found, not a proven best; the reasons are the search's `CareerSearchResult::unproven`), or whose trainee's class is flagged breached when the plan ends, carries `proven: false`; a row of an assuming class carries `assumes` (the names) instead. A fallback row (no trio: the pick's main search's careers, reported `exact`) takes its completeness and assumptions from that search as well as her class's flags, and counts in `umas_unproven` when not proven; and the page
  notes it under the table.
- Before a near-top builder is searched, her proxy is certified where her deck class's proofs allow (`certified`: over
  every legal pair one of the bounds `solve` prunes with (the ceiling, the class's fully checked decks through `rise`,
  its product proofs) is at most the proxy's value up to rounding (`Bar::AtMost`, 1e-12), so it is her best career's
  value; a tie is the usual case, a classmate at the same chances whose proof is the proxy's deck valued exactly); with
  mob or affinity-free parents every classmate has the same chances and one search proves her class. Counted in
  `umas_certified`.
- Cont's two builders come from the exactly searched careers.
- **Looped careers.** Each uma whose career above is not on two full veteran parents also gets her best career on two real full
  veterans (a legal pair, all of `fix` among them; pairs by ceiling, each searched exactly with both fixed and the best
  looped value so far as its bar, `SearchState::floor`, stopping when the next ceiling is below it; the surrogate's
  under `sur` 1), as a row of its own: only there is her own full veteran a fully looped
  parent. These rows are not among Cont's builders and are not added to the deck cache. A looped row that repeats
  another row's career is left out.
- **Without a borrowed parent now** (`borrowed_parent` "later"). The rows are the careers the player may run now, while
  everything V and V' read is the job with borrowing: the main search with borrowing (the "allow" job's, kept under its
  key, so a plan after an "allow" run takes it as it is), each candidate's best career and odds, Cont's builders, the
  trios and every scoring's partners and anchors (`RollKeep::outcome_hi`). A base row whose career has a borrowed parent stays a
  builder (its lifts) and is neither scored against V, shown nor picked; her best career without one becomes a row of
  its own after the looped rows (an *nb row*: her cached decks through the surrogate while proxies are on, searched or
  certified exactly on the pool without borrowed parents once a shown row or the pick's band can turn on it; exact from
  the start whenever proxies are off, as under `exact_all`). A proxy row whose exact search turns out a borrowed career
  leaves its proxy career as her nb row. The looped rows are searched on that pool too, also for an uma whose best
  career is looped on a borrowed full veteran. Nb rows
  are not among Cont's builders and not added to the deck cache. The pass-2 band and the outcome tables' pruning read
  the rows that can be picked only, so which proxies are searched exactly can differ from the "allow" plan's, and V with
  it.

**Continuation and V.** Per candidate a continuation (`Cont`, assumed form), anchored at the odds p0 it is fitted at
(her odds now; a given `fit` carries its own): at odds x a builder career lifts her with chance lam(x) = lam
min(1, g(x))^`LAM_DECAY` (1.5), g(x) = ln(p_inf / x) / ln(p_inf / p0) the gap left as a share of p0's, each success
closing phi of the gap in log probability to p_inf. From p0 the j-th success's chance is lam ((1 - phi)^j)^1.5; from a
lifted candidate's odds the path goes on where the lift left her, so her step after a lift is not priced with the
decay started again. p_inf is the greater of the builder's mean lifted odds and `CEIL_SHARE` (0.6) x her odds with a
full loop lineage (parents hypothetical full veterans of her two most efficient builders, bred from each other and from
her; parents and grandparents alike every target at the rank's star mix with the G1 wins of their uma's row's career at
their chances, as `CareerSetup::veteran` makes a fresh one); with only one builder's uma, the builder's mean lifted
odds. lam (at most 1: a lift chance summed from outcome chances can pass 1 by rounding) and phi come from the exactly
searched base builder whose continuation costs her the least at her odds now (`anchored_cost`; ties: the more
efficient), so a builder added beside the others, the pair behind p_inf unchanged, can only lower her step. Her step (a
fresh full veteran) at p0 and below costs min_n sum_{j<n} 1/lam(p_j) + 1/p_n: at p0 the table's own f64s (V at the
fit's odds is unchanged by the anchor), below p0 each success's rate capped at lam; `Cont::plan` starts at n = 0 and
stops at the first non-improving n (each success's waiting time rises with n and the grind's saving falls), with at
most `CONT_N` = 500 successes. Above p0 (a design choice, not derived: priced on the state-local rate the step was
sawtoothed in the odds, up to 9 careers on the reference fits) it is read off p0's lattice, p_j = p_inf (p0 /
p_inf)^((1 - phi)^j): K_j, the cost to go from p_j, the least of grinding there and success j + 1 (1/lam_j) plus
K_{j+1}, falls with j (K_0 the table's own value); at odds x between p_j and p_{j+1}, with ln(p_inf / x) = (1 - phi)^t
ln(p_inf / p0), the step is the least of 1/x (rerunning at once: no building, her odds x) and K_j + (t - j)(K_{j+1} -
K_j), held between the two; both fall as the odds rise, so the step does, and it is never above rerunning at once
(with phi = 1 every odds below p_inf is at index 0, so there min(K_0, 1/x); past a small phi's last lattice point,
1/x). The careers built are interpolated alike (`members`'
`building_careers`), and a step builds where they are above 0. Rust leaves the precision of ln, exp and powf
unspecified, so no bound in the plan relies on it: `ScoreBase::step_floor`, at most her step at every odds up to p,
takes IEEE arithmetic and the stored tables alone (the least of 1/p, 1/lam, the table's first sum and the lattice's
last cost: loose, and read only by y_hi and by the routes' prune). `Cont::plan` is memoised per Cont by the bits of p
(`PLAN_MEMO=1` prints lookups and hits, `PLAN_MEMO_CHECK` recomputes every hit and panics unless bit-identical,
`PLAN_MEMO_OFF`, `PLAN_MEMO_CAP` replaces the 16k cap); `score_career` costs every lifted step (through a per-slot
cache of the odds costed). V = min over trios of side(A) + side(B) +
1/p_C: A and B candidates, each side a real full veteran of hers (cost 0), the fresh full veteran her best career now
would leave (her step), or one of her next `fresh_lineages` - 1 (default 2) careers would leave (other parent pairs
ranked by her cached decks, `alternative_careers`, each then searched with both parents fixed under the job's own
budget, not `search_work`, `exact_pair_career`, memoised per plan, `proven` in `members` where the search completed:
a complete search's best is taken even where it is below the cached decks' interpolated value; an incomplete one's
where it is not below it, else the cached decks' career, not proven; her step at that career's odds, and
admitted only where she grinds at once there, `building_careers` 0: no scored career's veteran lifts such a side, so one
that builds would be priced on a build no route advances, and her building would replace the lineage it wins by) (`CareerSetup::veteran`: the career's parents,
every target at the rank's star mix, her G1 wins and pinks from its schedule), C every allowed uma at her odds with
those two as parents (C's own step is not lifted by builders). A fresh side's stars are random, so C's odds are their
expectation and the trio prices C at 1/E[p] where her expected careers are E[1/p]: a fresh side is priced up to about
2.2% cheap (derived: eight inheritance-only targets, both sides fresh, the 50/45/5 band; under 1% with one fresh side). The cheapest choice of sides wins: a real full veteran
with poor parents can lose to a fresh one, and a lineage that a new veteran displaces from a candidate's best career
(one worse for C) is still offered while it stays among her `fresh_lineages` best by her cached decks, at the value
the earlier plan found where both searches of the pair completed (a complete search's best is the pair's, whatever
proofs the worker kept). Neither condition is guaranteed (a veteran can open that many better pairs; a search that
runs out of budget gives its incumbent), and an old lineage that now builds is no Alternative side at all, so V can
still rise when a veteran is added under a fixed fit. Measured 2026-10-26 (`val/mono.py -n 3`, every val/plan pool): 8
rises under the fixed fit, none gone at 8 fresh lineages: p10 1 of 3 rolls (+126.4), p10_pink 3 of 3 (+12.9 to
+117.8), p11 3 of 3 (+34.4) and p16_t10's last veteran (+37.1). Each is a candidate whose best career moves onto the
added veteran, her old lineage left only as an Alternative that builds. With building Alternative sides admitted (the
rule before), the same rolls give no rise (p10 roll 2 and p11 roll 0 rechecked: 2737.32 and 1959.95, the pool's V).

**Step caps** (`caps.rs`): a pool that holds every veteran of a pool planned before can still follow that plan, so a
stage candidate's step there costs at most what that plan's fit prices it at, at her odds now: a refit never makes a
side dearer than the old fit says it is, and a lift since then lowers the cap as the old fit would have. Each plan
without a given `fit` keeps, per stage candidate, her continuation's curve (`CapCurve`: lam, phi, pinf, p0 and the
decay, what `Cont::with_decay` takes; her cap's curve instead where that bound at her odds now, so a cap only falls
along a chain of added veterans) and her odds now, with its pool (the veterans by their records' text, a multiset) and
scope (the setup, deck space and borrowing rule, `kept::plan_scope`; the fixed parents named by their veterans' records,
not their positions, since the pool's order is not part of the key; and the trainee filter): the last 8 plans, in
`state_out` (`STATE_VERSION` 9). A later plan in the same scope whose veterans include a kept plan's reads, per
candidate, the least such curve at her odds now; her step at any odds p is then the least of her continuation's cost
and the cap's curve's at p (`ScoreBase::step_plan`, `step_floor` with the curve's own floor, the trios' steps),
Alternative sides excepted (a cap is not of their lineage). A curve's Cont is built once per thread (`CapCurve::with_cont`).
A removed veteran voids the caps of every pool she was in. The robustness check's variants refit without caps; a given
`fit` neither reads nor keeps them. Output `step_caps` (`{chara: {careers, p}}`: the curve's careers at her odds now and
her odds in the kept plan, the candidates whose cap binds: a cap that does not is left out, so a plan of the same pool
again answers as before) and `capped` per member. Measured 2026-10-26 on the cont lane's traces (p10 and p10_pink, three
rolled veterans each, El Condor Pasa and Agnes Digital; impl_V3): the cap binds on 6 of 12 refits, nearer the trace's
cost than the uncapped refit on 3 of them and farther on 3; read off the old curve at today's odds it is 0 to 4 careers
below the old scalar cap (p10_pink roll 2, El Condor Pasa, 1/p 3425 to 2401: refit 1292, scalar cap 1017, curve cap
1013, trace 1070). An earlier build (impl_V1, before `builder_careers` 3, scalar caps) bound on 10 of 12 refits, nearer on all 10,
and on `val/mono.py -n 3` with each refit on its base's kept state cut refit rises from 10 to 7; neither is rerun here.

**Pass 2 and the pick.** A career's score is E[V - V with her veteran] - 1 over its outcomes: her veteran as a parent
raises any candidate's odds (with her best `partners` partners), her own full veteran makes her fresh side free, and with two full veteran parents her own full veteran is the goal
(V after 0). V after her is the least of V (every trio as it was) and the routes through her veteran (`v_after`,
`score::routes`), each a trio taken with her in it and priced on its own lineage at each outcome: a trio side on a
candidate's best career now, lifted through one of her lift slots (one per kept partner: her best career's deck on
the lifted lineage, her veteran and that partner the side's parents, in the one outfit option the side's record runs
where her outfits' grades differ, so the step's odds, the record's G1 wins and pinks and the route's C describe one
career; the candidate's step at that slot's odds), the trio's other side at the base; both sides lifted (each pair of
their lift slots); and, where she lands every target, her own full veteran (this career's own lineage) beside every side
of every other uma's candidate at the base and beside each lifted side. A route's C is held fixed: a trio's routes take
the trio's own C (left out where she is no legal C on the new parents), her own full veteran's the allowed uma best on
its two parents with her carrying every target at 3 stars (the most any outcome gives, so its ceiling bounds every
outcome's); her slots in C's lineage (C's parent, or the lifted side's parent and
so C's grandparent; `CareerSetup::her_slots`) are a finisher setup of the scoring's fold, so C's odds are valued at
each outcome's stars and landed set exactly as a candidate's lifted odds are. That lineage holds only where the lifted
side grinds at once at its lift's odds: where her step there still builds (`building_careers` > 0), its later builders
replace the parents that put her in C's lineage, so the route's C is priced as V prices it (`Route::while_building`,
V's own convention: a trio's route the trio's own C, her own full veteran's route with a lifted side the route with
that side as it is today, where that route is kept, else none); the bound (y_hi), not knowing, takes the cheaper of
the two. A route is left out where even its steps' floors (`ScoreBase::step_floor` at the slot's most odds), its fixed
sides and the cheaper of its C's at their most odds cannot go below V. Held fixed: each route keeps one C, and a lifted side one deck (her best career's; another of her decks with
her as a parent has no route). The lift statistics (Cont's fit, `proxy_check`) still read the partner slots: the best
of every cached deck. Memory: the fold's per-leaf rows and levels take at most `OUTCOME_BUDGET` (2^25 f64, 256 MiB,
natively and in wasm alike, so the page answers as the native validation does; a page's linear memory never shrinks:
10 uncertain targets with the default 2 partners' leaf rows take all of it, leaving no route); a job whose partner and
lift slots' leaf rows alone would pass it is refused before it runs (fewer `partners`); where with their corner tables
they pass it, the partner setups ranked last are left out (the last candidate's first, then rank by rank:
`fit_partner_setups`, counted in `sc_partner_setups_dropped`), so an admitted plan never fails for memory (fewer
partners only lower what a scoring credits); and the routes past it are left out, the highest lower bounds first (`routes_dropped` in `next`, `sc_routes_dropped` in
`work.prof`; the saving is then at most the model's). A target whose stars can't move any table value (per scoring and for every setup, either her miss factor
for it is the same f64 at 1, 2 and 3 stars, or every kept table reads the same with the target landed or not) is folded
at 0 and 1 star only: an outcome with 2 or 3 stars there reads the 1-star outcome's y and lifted odds, and every
outcome is still summed at its own chance, in order. The fold is exact in real arithmetic; floating-point rounding can differ between star levels. A roll
(`next.roll`) reads the same representative. Pick: the highest; within 0.25 careers per run (`NEAR`) or 5%, the better own odds. The goal is always the
next fully looped parent, any uma: a fully looped veteran you have is a real full veteran (free side) with full veteran parents.

**Robustness check** (`robust.rs`, `next.robust`): a plan whose pick can be checked (no given `fit`, a finite V)
keeps what the check reads (`CheckKeep`, the last 2, `CHECK_KEEP`, by the plan job without its roll, check and oracle
requests, `check_key`; not in `state_out`): the rows the page shows, in the plan's row order (the first `OPT_SHOW` by
score and every other within `NEAR` of the top, less repeats), their careers and scores, the plan's candidates, its fit
(each continuation's evidence, the sides, the trios) and its deck cache, moved out of the plan's simulator (the output
does not read it). The plan job again with `next.robust: "next"` (the page, once the pick is shown) runs the next
variant on that state, no search and no plan (an error, `unavailable`, where the worker keeps none); `"all"` runs every
variant left, the plan run first where none is kept (the CLI). With `next.oracle` too, `"next"` is refused and `"all"`
runs the plan (the kept state has no oracle). After the last variant the state keeps its report alone (the rows, fit,
candidates and cache go). So the pick's wait does not grow. The page sends these as background jobs (`engine.js`): an
edit lets the variant under way finish, its job waiting behind it, so the worker and what it keeps stay; a check cut
short is resumed when its answer is shown again, and one whose state is gone (a stopped comparison, a solver error, a
reload) is shown as not finished. Each variant scores the kept rows again, exactly, and applies the pick rule (`PickRows`, in the plan's
row order with its strict comparisons) to their scores. The first is the nominal control: the plan's own model with its
step caps dropped, which every variant drops (where the plan read no cap its scores are the plan's: equal on all 13
val/plan pools, 2026-10-26), so a pick that moves under a variant is not the caps' removal. Four are Cont's shape at the corners of the range the
leave-one-pool-out refits on cost span (`CEIL_SHARE` 0.5 and 0.65, `LAM_DECAY` 0.6 and 1.3; `val/sim/fit_cont.py`,
2026-10-26), each candidate's continuation refitted from the plan's own evidence (her builders' lifts and her
full-loop ceiling, `ContEvidence`) as the plan fits it. Two scale every career's P(all) by each end of research §13's
all-target obs/pred ratio's 95% interval (`13.p_all_eight_ratio_all_95_pct_low` / `_high`, read from
`research/results.json` when the solver and the page are built, to 3 decimals): the candidates' odds now, their lifted
odds, C's (`ScoreBase::odds_scales`, per uma: `OddsScales`) and the scored career's own landed sets (`build::scaled_landed_sets`: her full set's
chance f times the scale, the mass it loses spread over her other sets in proportion to their chances, each times
(1 - scale f) / (1 - f)), the continuations refitted on the scaled odds. Lam, a chance, is the builders' as the plan
measured it: their own landed sets are not re-weighted, so the lifts the moved mass would remove are kept (a known
approximation). A variant keeps the plan's trios and sides and prices them under its continuations; a differential
variant (below) chooses each trio's C again at its priced odds, the others keep the plan's C's (a uniform scale keeps
every C's order); its scorings read the kept cache only (`Evaluator::frozen`). Output `robust` (the report so far): `plan` (the pick checked, a
`kept::career_id`), `rows` (the careers rescored), per variant done (the plan's model first) `kind` ("plan", "nominal",
"shape", "ratio", "differential"), `ceil_share`, `lam_decay`, `odds_scale` (the common scale), `scaled` (the umas at their
own, `[uma index, scale]`), `v0`, the `pick` (the variant's strict argmax over the
checked rows, no `NEAR` band), its `score` and `margin` (its score less the best other checked row's), `scores` (every
checked row's), `routes_dropped` and `partner_setups_dropped` (the memory budget's truncations over its scorings,
`sc_routes_dropped` / `sc_partner_setups_dropped`: a pick that moves with them can be the budget's; null for the plan's
own scores, made during the plan; 0 under every variant on the 13 pools), `pending` (variants left), `stable` (every
variant done picks the plan's pick), `contenders` (each variant's pick, in the order first picked, the plan's model's
first), where they disagree `pick_varies: true` and `tied` (the contenders, best own odds first), where they agree on
another row than the plan's pick `near_override: true` (the variants do not disagree: the plan's rule took its pick
within `NEAR` for its better odds), `score_winner` (the row every variant done picks, null where they disagree),
`check_ms`, `pairs` (below). A uniform P(all) scale leaves a finishing career's score p V - 1 unchanged, so the ratio
variants do not test one career's calibration against another's; the differential ones do: after the fixed variants,
one per competitor, the 3 best-scoring other umas among the checked rows (by their best row under the plan's model), the
plan's model with the strict argmax's uma's P(all) times 0.76 and the competitor's times 1.32: her
odds now and lifted (`ScoreBase::step_plan`, `step_floor`), as C (each trio's C chosen again at the priced odds,
`rechoose_finishers`; each route's finisher slot, `AfterHer::finisher_umas`, C chosen at her priced odds,
`best_finisher`), her own landed sets, her continuation refitted at her scaled odds; the builders' lift evidence (their
lift chances and mean lifted odds, `ContEvidence`) is the plan's, not re-weighted for the scaled umas' full and partial
outcomes (as in the ratio variants, a known approximation). The
spread of the all-target obs/pred ratio between umas was measured from the run data (tau, the SD of its log between
groups, about 0.2, weakly identified: 0.14-0.17 between umas, not detected, 0.21-0.25 between outfits, permutation
p about 0.02; 13 umas and 15 outfits measured; a scratch measurement, not a research finding); the adversarial split
for a pair is inferred from it: m = 1.386 tau each way, exp(-/+ m) = 0.76 / 1.32, covering the relative error of 95% of
random pairs of umas. C's pruning at any positive scales (`score::routes`, `price_drafts`): a route is priced unless
C's priced odds p_C s_C, bounded by her scaled ceiling, are at most 1 / (V0 (1 + 1e-9) - its others) and its C while a
lifted side builds (the trio's, or the paired route's at its scaled most odds) cannot bring it under V0 either
(`finisher_floor`); a paired route (her own full veteran with that side as it is today) is priced at the floor its
dependents need too (the least others among it and them), and its C stays their fallback where its own route cannot
pass, that route then kept, never V after her's least itself, wherever a priced dependent's fallback can bring the
dependent under V0. What this proves: an omitted route's every value at every outcome is at least V0 (its others plus
the least its own C or its fallback can cost), provided C's odds at any outcome are at most her odds with her at 3
stars on every target, the bound `best_finisher` ranks C by (the model's premise that C's odds only rise with her
stars); these omissions are `sc_routes_floor_pruned`, and only the memory budget's (`routes_dropped`) are truncation.
Before 2026-10-26 the fallback condition was missing (astra-review-V4 #2) and then a paired route pruned on its own lost
its C as its dependents' fallback (astra-review-V5 #1); on the 13 val/plan pools neither changed a score, V or pick
(the second fix prices up to 1,625 more routes per plan, p16_88). C's own floor is 1 / (her most odds x her scale).
Measured 2026-10-26 (13 val/plan pools, each trio's C chosen again): the differential picks keep the plan's pick on
p16, p16_ss, p16_ura, gm16, gm16_nu8 and p16_t10, and move it on p10 (to Gold City [6,7] in all three), p10_pink (two of
three), p11, p11_noagnes, boot_mob (to Gold Ship, its scores all about 1.01), p16_88 (to Inari One) and p16_borrow;
check_ms 0.5-2.4 s but boot_mob 14.6 s and p16_t10 12.6 s. With `next.pair_search` the check then runs the pair search (`pairs`), every career in it scored under the nominal
model (the plan's fit uncapped: one V and one fit, the shown careers rescored under it, `PairSearchFits`). Phase 1, per
trainee: the pick's, each trainee a differential scenario picks ("tied"), then the 4 best trainees by their best checked
row's score ("top", equal scores by uma index). The work limit (`PAIR_SEARCH_OUTCOMES`, 30 million, `admitted_trainees`)
works on estimates fixed before any search: a trainee's estimate is the pairs she is scored on (below) times the
outcomes (`sc_out`) one scoring of her shown career evaluates. The pick's and the tied trainees always run, their estimates counted; a "top" one
runs where the estimates of those admitted before her plus hers stay within the limit (one skipped does not stop a
cheaper one after her; the others listed in `skipped_for_work`; on p16_t10 its pick alone). So it bounds the sum of
those estimates and nothing else: not the work actually done (a trainee's finalists, their exact searches and the
spread of her scorings' outcomes are outside it), not the pick's and the tied trainees' own work, and not phase 2.
The pair bound (`PAIR_SEARCH_PAIRS`, 128, plan/pair_bound.rs) caps every trainee the pair search scores, in both
phases, the pick's and the tied ones included: a trainee with at most 128 runnable pairs is scored on all of them, in
the order the search always took; past 128, on the 128 with the highest P(all) ceilings (`ranked_pairs`' ceiling, taken
again with the parents in identity order), equal ceilings by the parents' identities (`Legacy::identity`: the veteran's
record as canonical text, ordered by its hash and then by the text, so a hash collision cannot tie two different
records; the same order puts the parents in identity order), so which pairs are scored depends on the veterans and not on their order in the pool. What it
covers: the scorings per trainee (at most 128 plus her finalists' decks), and with them the pick's and the tied
trainees' own work, which the work limit does not bound. What it can miss: a skipped pair is never scored, and nothing
bounds what it would have saved; the ceiling bounds a pair's P(all), not its score, and the best pair by score has sat
at ceiling rank 2 to 11 of the first 40 (the pair sweep on p10, p16 and gm16_nu8, 2026-10-26; no rank past 40 was
measured). The 13 val/plan pools have at most 108 runnable pairs per trainee, so the bound changes nothing there.
Measured 2026-10-27 on p16_borrow grown by the greedy policy to 116 legacies (native, one plan job with `robust` "all" and `pair_search`): the check 18.3 -> 3.9 s, the pair search 18.0 -> 3.6 s; the pick's (Agnes Digital, 551 pairs) and the tied (Tamamo Cross, 3,791) trainees each scored on 128, which lets the work limit admit the two "top" trainees it had skipped (Inari One, 591; Gold Ship, 3,906); `best` and the recommendation unchanged (the pick, Agnes Digital [16,110]; both trainees' best pairs, unbounded, were among their 128). Her pairs so chosen
are each scored with her best cached deck (by P(all)); the 4 best pairs so ranked are finalists: every
cached deck of hers on the pair is scored (one with no chance of the full set too: its partial veterans can lift
sides), the pair is searched exactly (`exact_pair_career`), and the
better by score of her best-scoring cached deck and the search's career is kept (a P(all) search that proposes a deck
scoring lower does not replace the old one); the best finalist by score is hers, and it and her shown career are scored
under each shape corner and at her own P(all) times 0.76 and 1.32 (both careers alike, each trio's C chosen again at
those odds as in the differential variants). Holding the successor values fixed (F and L below; the check itself also
refits her continuation and re-chooses C, which move them), one scale s on her P(all) gives a career S(s) + 1 = s F +
(1 - s f) / (1 - f) L, f her full set's chance, F and L the expected saving's parts from her full set and her other
sets; this differs between pairs with different (f, F, L), so a per-uma error does not cancel within a trainee: it
enters as one common s, not a ratio between two umas. Phase 2: the best pairs phase 1 found, each tried for every
allowed trainee without a phase-1 report (those the work limit skipped included) that can run it, with her best
cached deck there; the 4 best by score are finalists as above (gm16_nu8: Oguri Cap [10,15], which no trainee's own top
pairs reach, 2026-10-26). The best (`best`) is the best (trainee, pair) by score over both phases. The pick's side is
the pick's trainee's best pair where it beats her shown career on another pair (`better_parents`, the page's "better
parents"), else the plan's pick. Where `best` is another trainee's, its margin over the pick's side is taken under the
nominal model and in the two per-uma scenarios (the pick's side's uma's P(all) x 0.76 and hers x 1.32, `pick_down`, and
the reverse, `pick_up`, each trio's C chosen again); it is recommended where all three are positive (`holds`) and is
otherwise tied with the pick's side. The recommendation (`recommendation`, what the page tells the player to run, and
what the simulator's `index` runs under `act_on` "pair_search"): that career, else the pick's side. Measured
2026-10-26 (the four pools where `best` is another trainee's; margins nominal / pick_down / pick_up): p16_ss Oguri Cap
[10,15] against Agnes Digital [10,15] +0.086 / -0.624 / +0.285, gm16_nu8 Oguri Cap [10,15] against Agnes Digital
[10,15] +0.131 / -0.041 / -0.177, p11 Gold City [7,10] against the pick (Sakura Chiyono O [7,10]) +0.165 / +0.218 /
-0.045, all three tied; p11_noagnes Gold City [7,10] against the pick (Yamanin Zephyr [7,10]) +0.208 / +0.486 /
+1.039, recommended. A scenario margin is not monotone in the scales (p16_ss: lowering Agnes Digital's odds raises her
own career's score more than Oguri Cap's), since a scale moves V, C and the sides with the career. Output `pairs`:
`trainees` (phase 1) [{`trainee`, `reason` ("pick",
"tied", "top"), `shown`, `shown_score`, `best_parents`, `career`, `score`, `search` (how its exact search ended:
"proven", "proven_below_cached" (complete, its best one or more ulps under the cached deck's interpolated value, the
search's career taken), "incomplete" (its work ran out or a class is breached), "none_found"), `proven` (the search
complete and its career kept), `deck_choice` (`p_all_deck_score`, `best_cached_score`, `score_deck_differs`,
`searched_score`, `kept`: "searched" or "cached"), `margin_over_shown` (`score` less her shown career's, nominal),
`better_parents` (that margin positive and the pair not her shown career's),
`margin_over_other_pairs` (`score` less the best other pair evaluated, `best_other_pair`: a finalist's kept career or,
past the finalists, its P(all)-best cached deck's; null with none; neither margin certifies the best pair),
`corner_margins`, `scale_margins` (both over her shown career), `alternatives` (the other 3 finalists: `career`,
`score`, `search`, `deck_choice`), `pairs_scored` (pairs scored), `pairs_runnable` (pairs she can run),
`pairs_truncated` (the bound left some unscored), `trainee_ms`, `work` (`outcomes`, `nodes`)}], `shared` (phase 2's
finalists, best first: `trainee`, `reason` "shared_pair", `best_parents`, `career`, `score`, `search`, `deck_choice`,
`pairs_tried`, `phase_ms`, `work`), `shared_coverage` (`trainees` phase 2 tried, `careers` it scored,
`pairs_runnable` the shared pairs they could run, `trainees_truncated` those the bound left pairs unscored for),
`pair_bound` (128), `best`
(`trainee`, `career`, `best_parents`, `score`, `reason`, `pick_trainee_shown`, `margin_over_pick_trainee_shown`, and
where it is another trainee's `versus_pick_side`: `career`, `source`, `margins` {`nominal`, `pick_down`, `pick_up`},
`holds`), `recommendation` (`career`, `trainee`, `trainee_id`, `parents`, `source`: "cross_trainee",
"pick_trainee_pair" or "plan_pick", `score` its pair-search score, null for the plan's pick, and `report` the career's
report as a row's, deck included), `skipped_for_work`, `pairs_ms`. It changes no pick. It needs
the check's working state, which goes after the last variant where no job had asked for the pair search: asked later,
`robust: "all"` runs the plan and its check again (`check_kept`), and `"next"` reports `pairs_error`, saying why
`pairs` is null. The page sends `pair_search` on the check's follow-up jobs only (`check_key` and `roll_key` leave it
out, so they find the plan's kept state), keeps the plan's pick first in the table, and adds under it, for the pick's
and each tied trainee, her better parents and their margins, or that none beat her shown career, and where `best` is
another trainee's, a line naming it against the pick's side with its three margins: a better career where it holds,
tied with the pick's side where it does not, with the coverage of both phases (umas searched on their pairs, those the
bound cut to their 128 by ceiling named as such, and umas tried on the best pairs found); each trainee's line says when
the bound left some of her pairs unchecked. Where the recommendation is not the plan's pick, the table gets a row for
it ("pair search", its pair-search score) whose Use this sends its `career` id, so the career job reports it with its
own deck (no deck search), and its deck is shown under the lines. The plan's own pick
keeps `NEAR` (the highest score among the rows that can be the pick, then within `NEAR` the best odds). It covers the checked rows and the tested variants only, with every search choice of the plan held
fixed, and reports: the pick stays the rule's.

**Joint-lift oracle** (`oracle.rs`, `next.oracle: "joint"`, a diagnostic, off by default): V adds A's and B's steps as
if every lift were a career of its own, so a builder whose veteran lifts both sides is charged twice. The oracle prices
the 8 cheapest trios on a joint two-side lattice instead: a state is the lifts each side has taken along her
continuation's own path (`Cont::building_path`, to `CONT_N`) and which sides are done; an action is a builder career
(A's and B's builders, the rows their continuations took lam and phi from, one action where they are the same career),
charged once, whose joint lift sets over every outcome are gathered by a `Want::Joint` scoring (against the placeholder
base the lift statistics read, the cached decks only), or a side's grind (1 / her odds, a commitment), and then C's
step, the trio's. Solved exactly from the caps down by the self-loop formula J = (1 + sum P J') / P(move). Assumed
beyond Cont: a lift moves its side one step whichever builder made it; after n lifts a side's lift chances from any
builder are their chances at the start times min(1, rate_n / her own builder's chance at the start), the two sides'
factors independent: her own builder then lifts her at min(own chance, rate_n), Cont's rate where the rescored chance
at the start is at least it, else slower (that side then costs more than Cont's step; on the 13 val/plan pools every
lattice trio's `v_separate` reproduces today's V to 1e-15, so no side was clipped there). Output `oracle`: `v0`, `v0_joint` (the
least of the priced trios' joint values), `best_trio_changes`, and per trio its umas, today's `v`, `v_joint`,
`v_separate` (each builder lifting only her own side: today's V up to the rescoring's lift chances), `same_builder`,
the builders' `lifts` ([both, A alone, B alone]) and the sides' `lam`; a trio with a real full veteran side has no
lattice. A side's path is her continuation's state-local one (`Cont::building_path`), which the plan's step follows at
and below her anchor only (above it the plan reads her lattice): a trio with a side above its anchor is marked
`above_anchor`, and there the joint lifts' effect is `v_joint` against `v_separate` (the same paths), not against `v`;
`v0_separate` (the least of the priced trios' `v_separate`) stands beside `v0_joint`, and `above_anchor` at the top says
whether any trio had such a side. It prices V0 only: no row is scored under it.

**Sequenced oracle** (`sequenced.rs`, `next.oracle: "sequenced"`, a diagnostic, off by default): V adds A's and B's
steps as if each were bred from today's pool. The oracle prices the 8 cheapest trios with their sides made in turn:
the first side as today, her fresh full veteran (the side's own `TrioSideVeteran::legacy`, never a hypothetical
full-loop one) then held in the pool; the second side the cheaper of her side today and her step at her best career
with that veteran as a parent (a career search with it fixed, under `search_work`; its veteran from that career); C
the best allowed uma on the two; the cheaper order counts (a real full veteran side first changes nothing). Its
scorings and C's values read the cached decks only (`aside`); its career searches can keep proofs and set class flags
as any search does (as the lift audit's), and the output reads the flags. Output `oracle`: `v0`, `v0_sequenced`, `best_trio_changes`, per trio its umas, `sides`, `c`,
`v`, `first_a` and `first_b` (`{second, second_moved, c, total}`) and `v_sequenced`; and `rows`, the 3 best rows
rescored against the trios at their sequenced values (each trio's change put in its C term, so the routes' other sides
and their C while a side builds are today's), with `score`, `score_sequenced` and the gaps between them: the scores'
sensitivity to putting the sequencing discount in C, not evidence about the pick. It changes no pick or V. Measured
2026-10-26 (bounds: greedy rollouts, an upper bound on the optimum): p16_borrow 211.39 sequenced = today, above
greedy's 207.8; boot_build 334.29 = today, above greedy's 330.4. On both the cheapest trio has a real full veteran side
(which sequencing cannot move: she is in the pool already); of the 8 trios, 3 have two fresh sides, and sequencing
lowers those by 13% (p16_borrow, 360.1 -> 312.6 at best) and 19-20% (boot_build, 480.7 -> 387.5), still above V. On 11
of the other 12 val/plan pools sequencing lowers V by 8.7-11.9% (boot_mob 40.9%, p16_88 0).

**Output `next`:**
- `options`: per career `role` (plan / finish / loop / candidate / builder; finish is C's career on A's and B's real
  full veterans, the trios' last step, only when both sides are real and her parents are both full veterans), `looped` (both parents
  full veterans), `best_odds` (the main search's best career, one row), `index`, `score`, `lift` (the chance her roll
  lowers V), `near`, `stage_cost` for a candidate, `chosen`; `choice`, `mode` (trio).
- `members` (A and B): `{trainee, p, parents, alternative (the side is from one of her next best careers, at its odds
  `p` and `parents`), proven (`p` is the career's proven best), has_full_veteran, real (the trio uses a real full veteran of hers), p_inf, lam, phi, p0,
  building_careers (the expected careers spent building before she grinds), p_switch, builder (the career id of the
  row, base or alternative, her lam and phi are from, `fit_continuation`; null where she grinds at `p`, her Cont was
  given by `fit` or her step is her cap's), capped (her step is her kept cap's),
  careers}`; `v0` (V); `v0_plan` (`{a, b, c, careers}`); `fit` (the stage candidates in order with their continuations,
  `[chara, lam, phi, pinf, p0]`; the solver parses floats with serde_json's `float_roundtrip`, so a `fit` sent back
  re-parses to the same bits); `pinf_builders` (`{chara: [first, second]}`: per candidate fitted from this pool, the two
  builders, uma ids, behind her full-loop ceiling); `routes_dropped` (routes the scorings' memory budget left out).
  An option from `builder_careers` carries `alternative: true`, and so does the career of each fresh side of the 3
  cheapest trios that is no row otherwise (whatever `builder_careers` is); such a row is `proven: false` unless its pair
  was searched to the end (the pick's, or a side's, `exact_pair_career`), and `searched: false` where no pair search
  ran (her cached deck there; not counted in `umas_unproven`, which counts searches that did not complete). `pair_sweep` (with `next.pair_sweep: true`, a
  measurement): for the 4 best-scoring trainees, her best cached deck on each of her 40 best pairs by ceiling, scored.
  `pair_ascent` (with `next.pair_ascent: true`, a measurement): coordinate ascent over (trainee, pair) by score, each
  pair her best cached deck there scored exactly (memoised): her best pair over every legal pair, then the best trainee
  on that pair, until neither changes; from the plan's pick and the 3 best-scoring trainees' best rows; with the pool's
  legal pairs per uma and the cost per scoring and per P(all) evaluation.
  `trios`: the 3 cheapest trios, cheapest first (`{a, b, c, careers, value, near}`; `near` within 2% of V's, an
  assumed band: the joint-lift oracle reorders p16's trios by 1.6%). `step_caps`, `capped`: *Step caps*. `side_only`,
  `hint`, `cut`: *Stage candidates*.
- `outcomes`: the career a roll is of, `{trainee, parents, career, uncv, fixed}` (`career`, as every career report
  carries it: her uma, her parents in order and her deck by card identity, `kept::career_id`; two decks on the same parents are two
  careers, and the page matches careers by it): the pick, her uncertain targets and the
  scenario's guaranteed ones (job-target indices). The page checks a roll's career against it.
- `roll` (a job with `next.roll`; a carried `roll` returns it as the job's `roll`): `{y, y_hi}` at her outcome (her
  stars per uncertain target, at most 3, and the guaranteed targets landed or not), with `ms` and `prof`, and
  `planned: true` when the plan had to run for it. The plan keeps its pick's roll state (`RollKeep`, the last 4 plans,
  `ROLL_KEEP`, keyed by the plan job without `next.roll`, top-level `roll`, `no_reports` and `compare`; in `state_out`): the pick's last
  scoring as its fold and outcome loop read it (`Scored`: the kept tables, her miss factors and spark scales per
  setup (partner and finisher slots), the collapsed targets, her landed sets' chances, the routes), the base it was
  scored against (steps, V, odds now, which candidates' odds are proven, the anchored Conts and their step caps' curves; `STATE_VERSION` 9) and the
  main search's found-career anchors (`found_anchors`). `y`, the pick's careers saved, V - V with her veteran (to 0.01): the plan's own fold and
  outcome loop (`Scored::evaluate`, generic over the leaves it produces: `All` in the plan, `OneLeaf` in a roll, which folds
  only the prefixes of the outcome's representative and runs the outcome loop at it and the outcome), so the f64 the
  plan's full table held (checked bit for bit against the plan's full table in one process,
  `score::build::tests::a_roll_is_the_full_tables_value_at_its_outcome`, and pinned against frozen fixtures by
  `tests/plan_roll.rs`). `y_hi` (rounded up to 0.01) where y < 0.5 (null elsewhere): a diagnostic, which the page
  never reads (it is loose by construction, `ScoreBase::step_floor`); at least `y`, the most her veteran's routes in the scoring could save for that outcome. Neither is a verdict: a replan
  can take routes the scoring has not (a partner or deck it did not keep, another C, or a candidate's other lineage
  whose C her veteran improves as a grandparent through relation points or shared wins without lifting the candidate's
  odds), so the page shows a roll's figure as provisional and confirms every roll with a Compare (each candidate's odds with her as a
  parent bounded over every partner y by the least over anchors A of c' E_A rise(q', q_A) (`CareerSetup::rise`: no
  assumed cap), E_A at least the best odds at q_A: today's pairs (y, z) at p_now / c_yz where the candidate's search proved
  p_now her best (complete; else no pair anchor), the main search's found careers
  of the candidate's deck class at their `Ref`s, and, for the partners behind the band's likeliest outcomes, her best
  deck searched at her 2 and 3 stars on every uncertain target (`HI_STARS`; at most `HI_SEARCHES` searches, currently 0, so these extra searches are off; outcomes whose
  partners' bounds pass `HI_RISE` = 1.5 x p_now are left as they are), the latter two under the caps; and by the ceiling
  (`outfit_fast` at q'); every lift slot of hers lifted at the bound, each step at `ScoreBase::step_floor` there; the
  routes' C at their odds at the outcome. A lift through a partner or deck the scoring did not keep has no route, so
  its C is not bounded). The least over anchors is found per (candidate, partner)
  by an exact branch and bound over the leaves' star-digit prefixes (an anchor's factors are positive, so its prefix's
  product times each remaining digit's least factor bounds its leaves from below; a subtree it can't lower is
  skipped; the leaves of no wanted outcome count as nothing to lower, so their subtrees are skipped too: a roll wants
  one, so each anchor's walk is one path), and each wanted outcome's bound is folded in as each partner finishes. A
  roll's first bound keeps, per (candidate, partner), the anchors whose root test passes against the most the ceiling
  holds at any leaf (`RollKeep::partner_anchors`, not stored by `state_out`): every other anchor fails the test at every outcome,
  so later rolls fold in the same anchors without computing their chances again. The
  page checks every roll exactly. The replan's other careers with her as a parent are not bounded (its
  candidates and Cont are the plan's).
- `umas_scored`, `umas_searched` (careers searched exactly), `umas_certified`, `umas_unproven` (umas whose best, looped or
  no-borrow career search is not `complete`, as for `proven` above), `hint`, `sur`, `sur_diff`, `work` (this
  plan's deck-search nodes and exact scorings, deterministic: a speed change can be measured at any machine load;
  `work.max_ratio` the largest exact/product ratio scored, `class_max_ratio` / `class_max_deck` per class, `class_caps`,
  `cap` the cap the plan's deck searches hold under), timings. A career deck search whose band would list over
  50,000 decks is rebuilt at 1.12, 1.05, then 1 (`DeckSearchAudit::certified_cap`, the lowest completed cap; 0 if every band overflows); its answer is not a proof (no `Ref`), and the output's
  top-level `cap` is lowered to that cap, `complete` set false and `band_overflows` counts overflows. A proven
  cap-free rerun supersedes that call's reduced certificate and overflow count.

**Kept plans.** Each trainee's best careers are kept per plan (the last few, in the worker). A plan of the same setup,
places and pool takes them as they are (no search: a reload, an edit undone); a plan of the same setup and places whose
pool is a kept one plus one veteran at the end (Compare, then Add) takes the better of the kept best and a search with
her fixed, whose bar starts at the kept value (`SearchState::floor`). Both need the same deck space too
(`bests_key`'s `CareerSetup::plan_space_key`: every uma's deck class content and the plain pals' order, which a
trainee filter can change through the plain pals' blocking order): a plan under another trainee filter that changes
a class's plain-pal rate prefix reuses none of the kept careers, and a reused plan's class flags come with it
(`Bests::flags`). `best_deck`'s results (with their certificates, band overflows, work counts
and per-class ratios) and the corner tables are also kept between jobs of one setup and trainee order (`KeptState::decks`), subject to memo keys and capacity limits. The kept plans, proofs, main searches and roll states survive a page reload through `cmd: "state_out"` /
`"state_in"` (fields table above).

The worker owns one `KeptState` (plans, proofs, main searches, rolls and deck memos). Native and wasm entry points
borrow it from one thread-local; `run_career_in`, planning and nested simulator jobs pass `&mut KeptState` downward.
`state_out` reads `&KeptState`; `state_in` validates everything before replacing its serialized fields through `&mut`,
leaving process-local deck memos intact. The JSON format is unchanged. `run_career_in` explicitly returns the deck
and corner memos to kept state after every completed job, including early answers; `Env` has no publishing destructor.
State-bound configurations expire with the job, never enter kept state, and are not serialized.

#### Work budget

A career search has its own work budget (`CAREER_BUDGET`, 300M units; `EXACT_BUDGET` in the environment overrides it). It does not stop the search: past it each deck search scores
only its product deck and start decks, so the search is not `complete`, and `solve` goes on to its stop rule (only the
plan's `search_work`, a hard cap, stops it taking candidates). Candidates are kept
as (ceiling, trainee, parents) and their chances recomputed when solved, which keeps memory low on large pools.

### Policy simulator (`simulate` on a career job)

Measures a decision policy (which career to run next) inside one solver process. With `simulate` present the career
job runs the simulator instead of the search; jobs without it are unchanged. `E` is the expected number of careers to
the run's goal, under the model, from the job's pool:

- `goal` "first" (the default): to the first veteran that lands every target. Every career *finishes*: its success
  ends the run.
- `goal` "looped": to the first fully looped parent, the page's plan's goal (*Next career*): a veteran that lands
  every target on two full-veteran parents (a spark of every target, yours or borrowed). A career finishes only on two
  full veterans; any other career's all-target veteran is a full veteran who joins the pool, and the run goes on.

Per run, careers t = 0, 1, ... with p_t the exact P(all) of career t (derived):

    E = sum over t of S_t,  S_0 = 1,  S_{t+1} = S_t (1 - p_t) if career t finishes, else S_t

A finishing career's success stays analytic in S: the path goes on through its failure, her veteran drawn from
`outcome_sets` without the all-landed set. A career that does not finish has her veteran drawn from every set, the
all-landed one last. Both are inverse CDFs over the sets ordered fewest-landed first. This is unbiased: the path is drawn
from the process with the finishing careers forced to fail, and S_t is the chance the real process is still running at
t given the rest of the path. The stars come from `star_dist`, each race is won with the schedule's chance of at least
one win, and her pink is drawn as the page's roll draws it (`webapp/src/pink.mjs`; the April inspirations fire from
her start grades, then one aptitude among those at A or S, stars 20/70/10). Her parents are the career's.

A run stops when S_t < eps or at `max_careers`, and a tail is added there. The tail is S_t / p_t when career t
finishes (that career repeated forever). Under goal "looped", when career t does not finish, it is S_t / p of the last
finishing career the run ran: repeating that one stays possible, since her parents stay in the pool. A run that ran
none (S_t = 1) is `censored`: tail 0, its E a lower bound. That tail is a stated rule, not the policy's own continuation,
so under goal "looped" `max_careers` should be at least 3 times the pool's V (the plan's `v0`); otherwise E is mostly
tail.

The uniforms (`u01`) are keyed by seed, run, career, stream and item, so a run index faces the same uniforms under every
policy. These common random numbers are unbiased, and differences between policies are paired. The coupling is not
monotone: a landed set is matched by its quantile along the (landed count, mask) order, not by set inclusion, and once
two policies' careers differ the same uniforms drive different careers. A new veteran is pruned when an owned veteran
of the same character with the same two parents has at least her stars on every target, every G1 she won, and the
same pinks (pinks set her children's aptitudes, so a veteran whose pinks differ is kept). The dominator gives at least
the P(all) of every career the pruned one could parent, and is a full veteran when she is (a family fixes both
parents). It is not the same veteran for the rest of the run, since her wins steer her children's schedules: pruning
is exact for P(all), not for a whole run.

The surrogate (policy `greedy`): each candidate (trainee, parents) is valued through its trainee's cached decks' corner
tables (`Cache`, exact P(all) of that deck at any inheritance chances), bounded first by the search's ceiling, by
`rise` from her fully checked decks and by her product-search proofs (`ruled`). Where the bound rb clears the best so far
B and exceeds tv × (1 + m), her deck is searched there (`best_deck` at bar B) and
cached. Derived: the career picked has an exact P(all), with the deck it runs, of at least E*/(1 + m), E* the exact best
career's, subject to the assumed search caps and complete deck searches; m = 0 gives the exact greedy pick up to ties. Under
goal "looped" the same holds among the looped careers, B the best looped one: a looped candidate is valued against that
bar. A candidate is valued once, when her later parent arrives; a trainee whose static ceiling (q = 1, every spark on
six legacies) is below B is skipped. Its candidate filter (`Evaluator::fast_ub`) counts a slot of random stars (a veteran
before her result is known) at 2 stars where the ceiling averages over her mix: an upper bound only while the mix's mean
is at most 2 stars, as both page bands are (`star_dist` [0, 0, 1] passes the job check); the simulator's pools hold
recorded and drawn veterans only, so no such slot reaches it.

| Field of `simulate` | Meaning | Default |
|---|---|---|
| `goal` | `first` or `looped` (above). `looped` is refused for `trace` and for greedy's `check` and `audit`, which measure the unrestricted pick | `first` |
| `policy` | `greedy` (surrogate; under goal `looped` the best looped career, both parents full veterans, once one exists), `greedy_exact` (the career search in process per decision: the best career by P(all); under goal `looped` the best among the full veterans' careers when they make one), `loop` (a fixed rotation of `loop_members`), `trace` (building for `loop_members[0]` with the others as builders: each career the builder whose best career lifts her odds most efficiently runs, her veteran drawn from every landed set, since a builder's full veteran does not end the building; records her odds before and after, the builder, lam and the odds raised to), `index` (the page's plan, *Next career*, above) | `greedy` |
| `runs`, `first`, `seed` | run indices `first` .. `first + runs - 1`; the seed | 16, 0, 1 |
| `eps`, `max_careers` | stop at S < eps or this many careers (the tail above added) | 1e-3, 100000 |
| `margin` | refresh margin m; -1: every valuation a bound does not rule out gets a fresh exact deck search (exact-backed: the same proofs as the career search, no trust in cached decks) | 0 |
| `prune`, `share_cache` | prune dominated veterans; keep the deck cache across runs | true, true |
| `check`, `check_lineages`, `check_frac` | every `check` new veterans compare with exact searches: the exact greedy pick (career search), and up to `check_lineages` of the newest veteran's candidates with ceiling at least `check_frac` x B (exact best deck, table value, landed-set distance) | 0, 0, 0.3 |
| `audit`, `audit_sample`, `audit_every` | fidelity audit: at each greedy pick change the exact career search's pick and, for the pick and `audit_sample` random candidates with the newest veteran, the table value against a fresh exact deck search (relative error, landed-set total variation); every `audit_every` careers of `loop`, its career and its keep-or-replace decision under exact values | false, 3, 50 |
| `loop_members` | `loop`: rotation members (chara ids) in order; `trace`: the trainee built for, then her builders | none |
| `progress` | log every N careers | 0 |
| `next` | `index`: the plan job's `next` (fields in *Next career*) | `{}` (the page's) |
| `act_on` | `index`: what each step runs: `choice`, the plan's pick (`next.choice`); `pair_search`, the page's final recommendation once its check is in (below) | `choice` |
| `force` | `{"trainee": chara id, "parents": [a, b]}`: every run's first career is that trainee on those two legacies with her best deck (the career search with both fixed), the policy taking over from the second; `greedy`, `greedy_exact` and `index` only (`val/sim/rollouts.py`) | none |
| `keep_pool` | per run also `survival` (S at the stop, before the tail) and `final_legacies` (the pool there, as a job's `legacies`) | false |

`loop`: member i mod n trains; her parents are the rotation veterans of the previous two members. A member keeps the
better of her rotation veteran and the new one, by the summed surrogate P(all) of the two loop careers she is a parent
of. Bootstrap: a member without both parent veterans gets the best pair by
exact P(all) from the pool with the existing ones fixed. No pruning. Each career's schedule is her own (the career
model's); the loop planner coordinates schedules across members, this does not.

`index`: each career is the page's pick for the pool as it stands. The simulator runs the plan job the page sends
(`webapp/src/career.js`: the job with the pool as `legacies`, no places, `top` 6, `per_trainee` false, `next`,
`no_reports`; the job's `trainees` kept) through `run_career_in` in process, and runs `next.choice` (its exact P(all) and
landed-set chances from the report) until the pool changes; a pruned veteran (as for `greedy`) leaves the pool, and so
the pick, as it was. Each step's pool is the last one plus one veteran at the end, so each plan takes the kept plan of
the step before (*Kept plans*: the better of each uma's kept best and a search with her fixed) instead of searching
every uma afresh; the main search runs again each step. Kept state and determinism: the plan jobs read and leave the
kept plans, proofs, main searches, roll states and `best_deck` memos in `KeptState`; each run swaps the whole record
for an empty one and restores the state from before the simulation afterward, so a run's careers depend on the job
and the run index alone
(the same run alone or among others, in any order, gives the same decisions and sampled outcomes). No checks or audits. Per run it adds
`index`: `plans` (plan jobs run), `plan_s` (each one's seconds), `roles` (the picks' `role` counts) and `best_odds` (picks
that were the main search's best career, the `greedy_exact` pick up to ties). Under goal "first" the plan is measured
against a goal it does not pursue: it can pick a career with lower odds to build towards a fully looped parent, and E
counts careers to the first full veteran. Goal "looped" measures it on its own goal. The baselines there are `greedy`
(restricted to looped careers once two full veterans of different characters make one) and the plan's own trio
(`v0_plan` a, b, c) run as `loop`, with `loop_members` [a, b, c].

`act_on` "pair_search": each step's plan job also asks `robust` "all" and `pair_search`, so the page's whole check
(every variant, then the pair search) runs on the plan's kept state in the same job, as the page's follow-up jobs run
it one at a time (the same report: `tests/sim_pair_search.rs`). The per-uma scenarios must run: the trainees they pick
("tied") are searched in phase 1, so the recommendation depends on them; the other variants do not feed the pair search
but run too, so the check is the page's to the variant. The step runs `pairs.recommendation`: the plan's pick where it
names it (or where the plan kept no check), else the career it names, as a `career` job (one trainee, both parents
fixed, the named deck, no deck search) whose report gives its exact P(all) and landed-set chances. The pick's own deck
is never searched again. Per run `index` also has `acted_on` (plans by the recommendation's `source`, "no_check" where
the plan kept no check), `careers` (the career asked for after each plan, recorded before it runs), `executed` (per
plan, what ran: the `career` id of the report its career was built from, the plan's `choice` or the `career` job's
result, and the `p_all` and `outcome_sets` it ran with), `check_s` (each plan's check seconds, inside its `plan_s`) and
`career_job_s`. `tests/sim_pair_search.rs` checks the first career's executed lineage (`segs`), that report's id and
its landed sets against the page's career (the `career` job its Use this sends, or the plan's pick), to the bit. The check runs in the run's kept state, as the page's check jobs run in its worker;
whether what its exact pair searches leave there changes a later plan's pick is not measured.

Output: per run `E`, `careers`, `tail`, `final_p`, `pool`, `pruned`, `wall_s`, `refreshes`, `segs` (each pick change:
career, p, pool size, S), `checks`, `audits`; under goal "looped" also `finishing` (careers run on two full veterans),
`full_veterans` (in the pool at the end) and `censored`; and `E` (mean, sd, 95% CI) over the runs, with `goal` and the
count of `censored` runs under goal "looped". `trace` returns its rows per run instead.

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

The career planner's checks, by what each can detect. A pin detects change, not error; an invariance holds the same
engine to itself; an independent recomputation can catch a wrong result.

| Check | Kind | Detects |
|---|---|---|
| `val/planval.py` (val/plan pools), `val/simval.py` (val/sim/jobs.json), test `plan_roll` (a self-pin: its fixture is re-taken from the solver's own rolls) | pin | a moved output |
| `val/planval.py`'s proxy gate (`next.proxy_check`: every builder searched exactly, base rows) | the plan against the all-exact plan; each proxy against its exact row | the pick, V or the set of the top 8 rows moving when proxies are on; a proxy efficiency more than 3% low (half the 6% band, where the exact one is at least 1% of the row's best); a proxy saving more than 0.25 careers low. `proxy_check` prices proxies on the deck cache from before the builders' searches (rows.rs `check_plan_proxies`) while the exact rows were scored on later caches, so until the two share a cache the saving criterion can fail on a proxy that is the exact career |
| tests `plan_state`, `plan_memo`, `plan_value`, `plan_borrowed`, `career_fold_and_scale` | invariance | a broken equality (kept state, memo, reload, borrowing rule, fold) |
| test `plan_looped` | one semantic property | two real full veterans make the finish the pick, saving at least one career |
| test `sim_determinism` | invariance | an `index` run that depends on the runs before it |
| test `sim_looped` | constructed pool, goal "looped" | a broken full-veteran draw, switch to finishing careers, last-finisher tail or greedy's looped restriction |
| unit test `sim_pink_draw_matches_pink_gen` (`src/career/sim/draw.rs`) | the simulator's pink draw against the model's exact pink chances | a draw that disagrees with `pink_gen` |
| `val/mono.py` | necessary condition (fixed fit) | V rising, or lost, when the pool's last veteran or a rolled veteran of the pick (drawn as the page's Reroll draws her) is added under today's fit (what Compare shows); a plan or roll that fails also fails it. Exempt: a fixed-fit rise that is gone when both plans are re-run with 8 fresh lineages (`fresh_lineages`) is reported as the documented lineage truncation, not failed; the page's Compare runs at 3, so it can still show such a rise. The refit column (what Add then shows) is reported, not failed on |
| `val/career_check.py` | independent recomputation (Python, from the job and data.json) | a wrong slot affinity, q, carrier count, spark scale, per-target buying or landing factor, landed-set total or marginal, or joint P(all) (deck model restated by `val/joint_check.py`) in a career report or plan row |
| `val/joint_check.py` | independent recomputation, same constants | a wrong P(all) of a deck job's loops |
| simulator `index` under goal "looped" against `greedy` and the plan's trio as `loop` (`val/sim/README.md`) | measurement | the plan's policy losing to a baseline on its own goal (not run at a size that resolves 10% of V) |
| `val/planval.py`'s first-step tripwire | internal consistency (policy improvement) | V's own first step (the row that runs V's route first) scoring below -0.1 careers, where V's route promises it saves 1 (fails p10, p10_pink and p16_t10 at c89918a: Alternative sides whose lift no route credits, and on p16_t10 951 dropped routes); a score above 0.1 is reported as V not its own fixed point (which way V errs it does not say), not failed; the route's row is matched exactly (a builder by career id; her own career by parents and odds; the finish by its parents' umas), and a plan whose first step has no row, more than one, or no finite score fails; not applicable only to a plan with no V and no route, a Cont given by the job's `next.fit`, or a builder the borrowing rule keeps from running now. The -0.1 is assumed |
| `val/sim/rollouts.py` (paired truncated rollouts) | Bellman consistency of V under greedy, on common draws | a top row's score disagreeing with what forcing it first and continuing with greedy (goal "looped") for 50 careers gains, valued by the plan's V of the final pool. On every measured pool (p10, p11, boot_mob, p16) no run finished within its 50 careers, so each run's T is exactly 50 + V(final pool): both terms are V's, and the check cannot give V's level |
| `val/sim/v_level.py` | measurement against a feasible policy (complete runs, goal "looped") | V reading high: V above the mean cost of 1,000 complete greedy runs (on p16_borrow, 2026-10-26: V 212.52 against greedy's 198.1 [192.2, 204.0], V 7.3% high, z 4.8). It tests neither Cont (p16_borrow's one building side builds 1.8 of its 121.7 careers) nor the pick |
| `val/sim/fit_cont.py` (`val/sim/trace_pools.json`) | measurement against the simulator's `trace` building, each pool held out of the fit | Cont's shape (`CEIL_SHARE`, `LAM_DECAY`) pricing a step's cost worse than a refit on pools it was not fitted on (2026-10-26: held-out mean cost error 10.8% for the cost refit, 10.5% for the current shape) |

What none of them checks: V's level on a pool where Cont's building matters (v_level.py's one pool has a real full
veteran and hardly builds), the pick between near options (on val/plan the score gaps between them were 0.06-0.2% of V
before the 2026-10-26 plan changes, below any simulation's resolution), Cont's shape against real building (`fit_cont.py`
fits it to the model's own simulated careers), and anything on real outcomes beyond docs/RESEARCH.md section 13's
all-target ratio.

## 5. Limits

- **Schedules are heuristic.** `optimize` is an iterated local search with a provably exact best response per uma; nothing proves a structure's schedules optimal.
- **Model assumptions (§1) are assumed or community-derived:** the win formula and the affinity objective. The code doesn't validate them.
- **First-outfit aptitudes in the loop planner.** data.json's `apt` is each uma's first outfit's; the career planner grades a career by the outfit it runs (`outfit_apt`), the loop planner does not. `build_base.py global CHARA:CARD` writes another outfit's aptitudes as `apt` to `base.json`; rerun `export_rs.py global` to put them in `data.json`.
