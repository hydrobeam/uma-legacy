# The solver (loopopt)

The Rust crate in `pkg/loopopt/`; paths below are relative to it. Its data is in `data/global/` and `data/jp/` (the
Grand Masters era); jobs run from the era's folder, which the file names in a job are relative to. Building and running
it: `docs/BUILD.md`.

Finds the best parent loop (a four-uma loop or a three-uma rotation) and support deck for landing target skills in Uma Musume (Global), with the exact race schedule for each uma, and plans careers to a looped parent (`/career`). Its inputs are data files exported from the game's `master.mdb` and GameTora's event data.

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

The algorithm tests run on frozen data with an older table (`tests/fixtures/data_v1.json`: 10 points per grade, PEN = 0,0,10,25,35,50); `win_table_matches_community_table` checks the shipped data against every cell of the current one.

- **Streak position k:** counts consecutive turns with *any* race, goal races included. A rest resets it.
- **Turn** = 24·year + 2·(month−1) + half: Junior 1–24, Classic 25–48, Senior 49–72; half 1 = Early, 2 = Late.
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

- Aptitudes: each uma carries `apt` (ten codes 1..8 = G..S: short, mile, middle, long, turf, dirt, front runner, pace chaser, late surger, end closer; from her first outfit); `race_cat` / `race_ground` give each race's distance class (0 short .. 3 long) and surface (1 turf, 2 dirt). Only the career schedule uses them, for the legacies' pink sparks (`Env::pink_career`).
- `pair(a,b)` / `tri(a,b,c)`: the sum of `succession_relation.relation_point` over the relation groups shared by all of them. `data.json` stores the groups (`relations`: `[points, [roster indices]]`, groups with at least two roster members) and the solver builds both tables at load.
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
| `dp_exact` | Best response for one uma: maximize Σ_r W(r)·P(r) given partner weights W. Junior turns 1–24 run in sequence. Then Classic turn t and Senior turn t+24 are decided **together**, with state (Classic streak, Senior streak, goal bitmask), plus the Classic streak at turn 48 for the 3 umas with goals on turn 49. Every race offered in both years is exactly 24 turns apart, so 1 − (1−p₁)(1−p₂) is computed exactly. Microseconds per call, memoised within a search. | **Yes** (tested against an exact MILP, §4) |
| `ascent` | Best-response dynamics: each member re-solves against the other three until none improves. It converges to a Nash equilibrium of an identical-interest game, not necessarily the global optimum. | Local |
| `group_moves` | Coalition deviations: force 2+ members onto the same race (or rest) at one turn, then re-ascend. Escapes coordination traps, e.g. all four moving Hopeful → Zen-Nippon together, which no single member gains from alone. | Local |
| `optimize` | Optimistic start (partners at best-case odds) → ascent → group moves → `iters` rounds of iterated local search (1–2 members re-solved against noisy weights, then ascent and group moves). | Heuristic |
| `pair_ceilings` | Exact pairwise ceilings PB(a,b) = min(BR_a(PM_b), BR_b(PM_a)), where PM is a partner's best-case odds per race: no pair of schedules shares more expected G1 wins. The deck search bounds every structure's lineage affinities with them before any schedule is optimised; `loopopt pbtable` stores the table in data.json. | **Guaranteed** bound |

**Why the ceiling is valid:**
- P_b ≤ PM_b elementwise and P_a ≥ 0, so ⟨P_a, P_b⟩ ≤ ⟨P_a, PM_b⟩ ≤ BR_a(PM_b).
- Take the min over both sides.

A structure whose bound is below a result already achieved cannot beat it.

## 3. Usage

```bash
# 1. export data (needs the Global master.mdb in game/global/)
cd export && uv run build_base.py global && uv run export_rs.py global   # writes data/global/data.json (../../build.sh data: both eras)

# 2. build
cargo build --release
# optional, after any export: store the exact pairwise ceilings in data.json ("pb", with a fingerprint of the
# umas/goals/penalties; ignored when stale) so searches skip computing them (../../build.sh pb does both eras)
uv run export/add_pb.py target/release/loopopt data/global/data.json

# 3. run a job file from the era's data folder (the file paths in a job are relative to it)
cd data/global && ../../target/release/loopopt data.json ../../../../val/jobs/pal_U.json > /tmp/result.json
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

Finds the support deck and the loop together. Needs `skills_file`, `cards_file` (`web_cards.json`, from `export/export_cards.py`) and usually `events_file` (`card_events.json`, from `export/build_card_events.py` over the GameTora files that `./build.sh gametora` caches in `cache/gametora/` at the repository root).

```bash
cd data/global && ../../target/release/loopopt data.json ../../../../val/jobs/deck_unity8.json        # everyone owned
cd data/global && ../../target/release/loopopt data.json ../../../../val/jobs/deck_unity8_noti.json   # no Tamamo / Inari
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
| `scenario` | Independent Training scenario: `unity`, `gc` (Grand Concert), `ura`, `trackblazer`, `grandmasters` (JP); sets the training-hint base and specialty weight (Grand Masters, with no runs to fit, uses the pooled 1.86 / 0.26 placeholders) | `unity` |
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
  skill); one card's draws over several targets are joint (`miss_draws`). Cards without chain events (R cards) take no
  chain slot, like pals, and give training hints only.
- **Chain events**: Binomial(21, 0.35) per run (`CHAIN_N`, `CHAIN_P`; `chain_events` overrides), each advancing a
  random remaining chain slot of a non-pal card. Pal finales are certain.
- **Outcome shares**: an outcome-dependent reward counts at its share of the outcomes, k of n at k/n (Nakayama Festa
  [Trusting the Dice]: each skill in 2 of 4 outcomes, 50%). A measured share replaces k/n where one exists: `chance` in
  manual.json for hand-entered entries (Anshinzawa SSR's finale, Riko Kashimoto SSR's Ramp Up), and
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
- **Assumed caps, per deck class.** No deck's exact value exceeds ρ times its product value (V ≤ ρ·V̂), with ρ set by
  the deck's class (`EXACT_CAPS`, `cap_class`): 1.05 when at most two targets are fed by chain links (`Src::cm`) and no
  outcome group touches two targets (`Src::multi`), otherwise 1.25 (`EXACT_RHO`). The caps are assumed from
  measurement, because chain cards compete for one run's chain events and so correlate their targets: the largest
  ratios scored are 1.037 with two chain-fed targets (`pal_Fk3`), 1.127 with three (King Halo, `val/plan/boot_mob`) and
  1.18 with more (`pal_UF`; 1.17 on Grand Masters); the planner's own jobs reach 1.03-1.04. One cap for both eras keeps
  the headroom where a deck above an era's measured maximum would otherwise pass unseen. No data-free cap exists: a
  target whose only source is one card's last link makes the ratio 1/P(link) (about 6.7 with two such targets), and
  range-only and Hölder bounds come out at 1.2-1.7; only a deck with no target-bearing chain card provably has ratio
  at most 1. Ranking by the product alone misses better answers (Grand Masters, all owned: 1 in 202 where 1 in 194
  exists), hence exact ranking. A search node takes the class of its cards plus every candidate still to add, so its
  cap is at least any completion's; bounds that know no deck (`Env::ceiling`, the loop tables' product fallback, the
  reroll's `y_hi`) use 1.25. Every deck and loop whose product ceiling times its cap could beat the best exact value is
  examined. Guards: the output's `class_caps` and per class `class_max_ratio` / `class_max_deck`; `val/cmp2.py` and
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
  (2) where (1) passes, per chain state x (`SbCfg`'s projected states) the lower of the per-target bound A_x and the
  ratio bound B_x (log-submodularity of each target's factor, card by card, given x), summed over states, heaviest
  first with an early stop once the sum decides the test; the open slots of one group share its lowest misses and best
  ratios, so states that differ only by the order of that group's open-slot levels have one term, computed once. Both
  rest on `state_value`'s argument, which needs every group's per-target levels negatively upper-orthant dependent at
  every white/gold threshold (`group_nuod_lvl`, `Src::cf_ok`); a deck space with an entry not covered proves nothing.
  Proven when the band finishes within its own budget (`CF_BUDGET`, default 60M work units, 15 per cap-free node,
  about 3 s; apart from the career's budget): then a best below th / c is `Bounded`, else the deck is a fully checked
  one (`Ref`, `full`); unproven, the better of its best and the capped answer, not full (the search is not
  `complete`). The output's `cap_rerun` counts the reruns (`searches`), those that proved nothing (`unproven`) and
  their `work`; `cap_exceeded` still lists the classes past their caps. Not covered: the career search's own ceilings
  and `ruled` proofs still use EXACT_RHO, and a search that never scores a breaching deck has nothing to trigger it,
  so "proven" stays relative to the caps where no breach was seen. Checks: `CF_CHECK=s` runs the cap-free search
  beside every capped one (state put back) and compares answers, passes its nodes at s times their bound and scores
  every leaf reached exactly against every bound above it, and checks the state value against each exact scoring
  (`CFCHK` on stderr); `CAPS_TEST=a,b,c,d` lowers the caps to force breaches; `CAP_RERUN=0` leaves a breach unanswered
  (the search not `complete`, as before the rerun). Measured (2026-10-20): on the 15 tiny Grand Masters slices
  (`.perf/ana2`, 8-10 targets) with every cap at 1.0, each job's breaching search reruns and every answer equals
  `cap_audit`'s, proven (job field `cap_free` the same); on export 2 at 10 targets (24 veterans, main search) with
  caps 1.15 (1.0) 7 (15) searches rerun, all proven, the `cap_audit` answer, 13 s (16 s) against 0.7 s capped, about 7
  s (9 s) since the bounds' speedups of the same day (`docs/PERF.md`); without the rerun the same answer, not
  `complete`. A cap-free node costs about 0.8 us (0.2 us capped).
- **State-conditioned bound** (career searches' band pass, at each configuration's root): over the configuration's
  chain progress states x, P(x) times the product over targets of each target's factor at its lowest possible misses
  given x (per open slot the best entry of its group at that slot's level); proven >= the exact value of every deck of
  the configuration (given x, sources are independent and one group's per-target hints negatively associated), so a
  configuration below the bar is dropped. It prunes beside the caps (outputs unchanged); as a replacement for them it
  is 10-45x slower. `SBOUND` 0 turns it off, `SB_CHECK=1` checks the value against every exact scoring and the node bounds against the
  leaves below them.
- **Every legal deck.** `DeckSpace` enumerates every deck legal for the trainee (for all four members with a shared deck: the character rule is a legality filter, no splitting), by chain-kind key, before card dominance drops any card; cards interchangeable for the trainee (same rewards on the targets her outfits leave open, or none) are merged; a partial deck is dropped when the best its completions could reach is below the threshold: per (chance, outfit) the lower of the per-target ideal point (the r lowest miss chances per target in each kind group) and a submodular bound (each card's ratio P({card})/P({}) for the product of one outfit at one chance, which gains less the more the deck holds; the best ratios of the cards still to add); branches that cannot place a locked card are dropped. The submodular bound may split the targets: the ideal point's factors for some, the ratio bound over the rest, the lowest of the splits taken. An outfit with a target at zero chance without a hint (no inheritance, event or outfit chance) has infinite ratios unsplit, so it is split by its zero targets; one with targets at low base chance (at most half their certain-white factor) also by those (10-target job slice `j_t10_1`: nodes 14.1M -> 0.54M). The configurations keep the unsplit bound's visiting order, which decides which of two tied decks is found first; `RB_CHECK=s` checks every leaf against its ancestors' bounds (nodes pass at s times their bound, results unchanged). In the product search (the best deck by product value) a card is skipped where a free card of its group, legal wherever it is (borrow, lock and pal rules), is at least as good on every target's white and gold miss: the deck with that card comes first and is at least as good in floats, so the deck found is the same; its configurations' ratio tables and roots are built only for configurations the search may visit. The band pass keeps every deck (removing one would change what it scores exactly).
- **Proven filter.** Before a deck is scored exactly, `upper_na` bounds it: given N = n chain events, the progress vector is negatively upper-orthant dependent (checked exactly per chain-kind key over its state distribution, early ends and stop risk included; with two cards feeding one target, the same check over every up-set of their pair), so with every card grouped with the targets its chain feeds, E[Π groups | n] ≤ Π E[group | n]; a target that would join a larger group (three or more feeding cards, or two other than the one pair) is taken at every feeder's full progress instead, a constant at least its factor on every chain state (a target's factor only rises as its misses fall), so such decks get a looser bound rather than none (Grand Masters decks often have them); event outcomes enter at their marginal chances (exclusive outcomes only lower the product). U = Σ_n P(n) Π E[· | n] ≥ V; on the reference setups U is within 0.2% of V. `UPPER_CHECK=1` asserts U ≥ V on every deck scored.
- **Per-uma tables.** X(u, i) = the best exact value of trainee u over every legal deck at grid chance i (64 points); V rises with q, so a lineage at chance q is bounded by X at the next grid point up. Cells are computed lazily: a loop's ceiling uses each cell's exact value when known, else the lowest known value at a higher grid point, else ρ times the product ceiling; while the ceiling beats the k-th best, the loosest term is refined (first at the uma's highest grid point, then halfway down).
- **Loops.** Structures of both sizes are generated in bands of a separable bound, the mean of the members' best product values M_u at their highest chances (a member heads 3 of 12 lineages in a loop, 1 of 3 in a rotation, so the product ceiling is at most the golds' factor times that mean; bands are kept in four-member sum units, a rotation's sum s counting as 4s/3); a structure is solved (schedules, then exact values) only when its exact ceiling at the pairwise-ceiling chances, then at its own schedule's chances, can still beat the k-th best exact value; the search stops when no ceiling (times ρ for the ungenerated ones) can.
- **Shared deck** (`per_uma` false): a loop's value is the best exact sum over its lineages (12 or 3) among decks legal for all its members, divided by their number; ceilings are the lower of the per-uma tables and ρ times the best single skyline deck's product value over its lineages. `top_decks` counts distinct decks, each with its best loop; per loop, every deck at or above the k-th best distinct deck so far is kept.
- **Certificate and budget.** The pass at ρ = 1.25 (each class at its cap) runs first; if it exceeds the work budget (`EXACT_BUDGET`, counted in work units so results do not depend on machine load: about 3 s natively), passes with every cap at most 1.12, then 1.05 follow (likewise when a deck search would return over `EXACT_CAP` = 50,000 decks, `cap_hit`), each reusing every exact value, table cell and schedule already computed; if none finishes, a pass at ρ = 1 (product-odds pruning, no deck searches) answers. Output `exact`: `rho` the cap the answer is certified under, `max_ratio` the largest V/V̂ on any deck scored (a value above `rho` would contradict the assumption), `complete`, counts. Job field `exact: false` runs the ρ = 1 pass alone: the page shows it first, then the exact one.
- `p_all`, `p_minus1`, `per_trainee` and the detail's per-lineage `p_all` are exact.

"Proven" (`proven`, `exact.complete`) means the search ended by its stop rule under the certified cap. It is relative to the schedules the optimiser finds (iterated local search, not proven optimal; `exact.schedule_rejects` counts loops rejected only after their schedule was built) and to the cap, which is assumed: a scored deck whose ratio passes its class's cap shows in `class_max_ratio` but leaves `complete` as it is here (the career search instead marks itself not complete, *Search and output* below).

Typical time natively (measured 2026-10-07): about 0.3–4 s per search with one result (the Global default 0.25 s, Grand Masters all owned 0.9 s on an i5-1340P; with four-uma loops only, `sizes: [4]`, 0.97 s and 1.15 s: a strong rotation found early prunes most loops), most of it the exact deck searches; three-result shared searches 10–35 s. The browser's exact pass is about 1–2× native; its first pass (ρ = 1) takes under 0.5 s. Dominance tests in the skyline enumeration use a 64-bit signature (16 coordinates × 4 bits) as a fast necessary check before the exact comparison.

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

- **Repeated character.** A grandparent may be the trainee's own character (common in uma-it runs: `docs/RESEARCH.md` §6;
  none has a parent of her character). `rel_tables` counts a relation group only for distinct members, so
  tri(T, P, T) = 0 and pair(a, a) = 0, as fitted on inspiration trials (`docs/RESEARCH.md` §5, *Self-grandparent*).
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

Why rotations usually win: a rotation's careers are each a little worse than a loop's best (the fourth uma's relation
points as a grandparent are missing; a median ×1.05 per career over 660 trainee-parent combinations), but a loop must
also train its fourth member, usually a weak trainee (Oguri Cap at 1 in 216 in `pal_U`).

### Career planner (`"mode":"career"`, page `/career`)

The best next career from the veterans a player has: trainee, two veterans as parents (at most one borrowed, or none:
`borrowed_parent`), deck.
Same files as deck mode, plus `legacies`; the shared and deck fields apply (`owned`/`exclude`/`exclude_outfits`
choose the trainees, `owned_cards`, `borrow`, `scenario`, ...; `star_dist` sets the stars of a veteran before her result is known: the plan's fresh 8/8s, the simulator's draws; other loop fields are unused).
`val/career_job.py TRAINED_CHARA_JSON BASE_JOB OUT_JOB` builds a job from an umadump export, as the page does.
Several career jobs run in one process with `target/release/loopopt data.json a.json b.json ...` (one output line each),
as the page's solver worker runs them: a job's fully checked decks are kept per setup (everything but the pool and the
places) and bound the later jobs' searches, never the search that seeds the simulator's cache, so every output equals
the job run alone. The page runs a career as two jobs: the search, then the same job with `next`, which reuses the
search the worker keeps (`LAST_SOLVE`, the last three, keyed by the whole job without `next` and `no_reports`; the
second job's search costs about 3 ms in wasm) together with what its product searches proved (`Env::ruled` as the
search left it), so the plan starts from the state a combined job's plan starts from: on the nine val/plan jobs its
output and its deck-search nodes equal the combined job's (`tests/plan_state.rs`). Without the proofs the plan job did
up to 0.45% more nodes (gm16_nu8), outputs equal. The proofs go no further than their own search: `certified` takes a
proxy career a proof certifies in place of an exact search, which may name another pair or deck of equal value, so
proofs from other pools could change an output.

| Field | Meaning | Default |
|---|---|---|
| `legacies` | veterans: `{chara, borrowed, sparks: [[skill group, stars], ...], pinks: [[aptitude group, stars], ...], wins: [race index], parents: [{chara, sparks, pinks, wins}, ...]}`; race index = position in data.json `races`; aptitude groups 11 turf, 12 dirt, 21-24 front runner to end closer, 31-34 sprint to long; a parent with `chara` 0 is unknown (no relation points, its sparks still count) | required |
| `trainees` | chara ids to consider (empty: every owned uma) | `[]` |
| `fix` | legacy indices that must be among the two parents (0, 1 or 2) | `[]` |
| `borrowed_parent` | `"allow"`: at most one borrowed parent per career, now and in the plan's look-ahead. `"later"`: the career picked has none (the search's `results` and the plan's rows, *Next career* below), while the plan's V and V' still assume later careers borrow (borrows refill daily); a `next` job then runs the main search twice, without and with borrowing. `"never"`: no pair holds a borrowed veteran anywhere (`Pool::max_bor` 0 in every pool the job builds), which answers as the job without its borrowed veterans (`tests/plan_borrowed.rs`). A rule on the pairs as `fix` is: out of `setup_fp`, so the kept proofs and decks are shared across settings; in the kept main search's key and the kept plans' (`bests_key`). The simulator (`simulate`) takes "later" as "allow" | `"allow"` |
| `per_trainee` | one result per trainee (each with her best parents and deck); false: the best careers whoever trains (with one trainee: her best parent pairs) | `true` |
| `top` | careers returned | 5 |
| `next` | the page's next-career plan, section *Next career (`next`)* below | none |
| `simulate` | runs the policy simulator instead of the search (section *Policy simulator*) | none |
| `compare` | marks the page's veteran-analysis jobs (the best career with her fixed as a parent, and the plan of the pool with her added); echoed as `compare: true` in the output, changes nothing else | `false` |
| `no_reports` | leaves out `results` (the main search's career reports and the first one's schedule search): the page's plan-only jobs read `next` alone; out of the setup fingerprint and the kept main search's key, so it changes nothing else | `false` |
| `roll` | `{job, stars}`: a roll of a plan, answered beside this job's own answer as `roll` (the plan job `job` run after this one with `next.roll` = `stars`, its `next.roll`; below). (The page's Reroll now sends the plan job with `next.roll` alone first, so the saving shows at once, then its search) | none |
| `cmd: "state_out"` / `cmd: "state_in"` | the worker's kept work across page loads: `state_out` returns `{ok, state}`, the kept plans (each uma's best career per pool, as above), the kept proofs (fully checked decks per setup and deck class), the kept main searches (`LAST_SOLVE` with their product proofs, so a reload's first run searches nothing again) and the plans' roll states (`ROLLS`, *Next career*: a reload's first roll plans nothing again), every float as its bit pattern in hex; `state_in` with that `state` puts them back (all or nothing: a state that does not parse, or of another version, is refused with `error`). The career page stores it in IndexedDB keyed by the solver's and data's fingerprints, after each plan, and restores it before a new worker's first job. Natively: `LOOPOPT_STATE_IN` / `LOOPOPT_STATE_OUT` (files) around a career run | none |

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
a classmate's proof bounds her candidates too (on val/plan p16, 47 of the 68 umas of 2026-10-08 share one class, in 21
classes; p10/p11 38 of 51, 13 classes). `rise(t, q', q)` and every proof (`Ref`, `ruled`) depend on the trainee only
through her class, so the sharing is exact. The planner's deck cache holds exact winners only: pruning, the proofs kept
between jobs and class sharing change which decks a search happens to check fully, and a cache of those would make the
plan depend on search history. Identical
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
when the worker kept the search from the job before). A scored deck whose exact / product ratio passes its class's cap (`EXACT_CAPS`, an assumption) has its deck search rerun cap-free (*Cap-free rerun* above; `complete` stays true when every rerun proves its answer), the output lists the classes in `cap_exceeded` ([class, highest ratio, cap]) and the reruns in `cap_rerun`; none passes on val/plan, a 10-target export reached 1.297 in class 3 (four or more chain-fed targets or a multi-target group; cap 1.25). The best career also carries `schedule`, `g1_wins` and `outcome_sets`:

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
evidence: `docs/MODEL.md`, *Next career*. Constants are in `src/deck/career/sim/plan.rs`.

| Field of `next` (all optional) | Meaning | Default |
|---|---|---|
| `sur`, `sur_margin` | 1: each uma's best career from the simulator's surrogate (`best_sur`: every legal pair by ceiling, her cached decks, an exact deck search where the bound beats the table by more than `sur_margin`) in place of the career search; 2: both, the differences listed in `sur_diff` (on val/plan p16: none over its 68 umas, the same time) | 0, 0 |
| `exact_all` | every uma's best career searched exactly (no proxies: the rule before builders went through their deck class) | `false` |
| `proxy_check` | each builder's proxy reported beside her exact career (`proxy_check` in the output); changes no result (the looped careers' scorings then evaluate their statistics, so `sc_out` runs higher) | `false` |
| `proxy_eff` | test knob replacing `PROXY_EFF`; `tests/plan_memo.rs` sets -1 (pass 1's builders left to their proxies, so the pass-2 searches refit Cont) | 0.06 |
| `fit` | `[[chara, lam, phi, pinf], ...]` (a plan's own `fit` output): the stage candidates and their continuations taken as given instead of fitted to this pool. Unused by the page, since Compare plans exactly as Add will | none |
| `loop_members` | loop members to add as stage candidates; absent: none (the plan runs no loop search) | absent |
| `search_work` | work budget (career-search units) of each candidate search beyond the job's own; 0 = the job's budget | 5,000,000 |
| `margin` | the surrogate's refresh margin (section *Policy simulator*) | 1e9 |
| `roll` | a roll of the plan's pick: her stars per job target (0 where it did not land); the output is `next.roll` (below). Answered from the plan's kept roll state with no search when the worker has it for this job, else after the plan, which keeps it | absent |

**Stage candidates** (at most 8): the best career's trainee, the loop members when named, the umas of the best
career's lineage and of your veterans, the search's other trainees. Below 4 (`HINT_FLOOR`: a loop being started, mob or
other-uma veterans) they are filled from the owned umas with the highest odds under a full loop lineage (parents
hypothetical veterans of the first two candidates' umas, each bred from the other and from her, valued by
`Env::ceiling`, no deck search; reported in `hint`). Each gets her best career. On the val/plan pools the loop members
always entered by the other routes, so the loop planner's search is not part of the plan.

**Pass 1**: every allowed uma's best career, exact over her outcomes, giving per candidate the chance her veteran
raises that candidate's odds and the mean odds raised to.
- A builder's career goes through her deck class: her class's winning decks (the main search's and the candidates'
  best careers), valued at her own chances by the surrogate, no career search. She is searched exactly when her lift
  efficiency is within `PROXY_EFF` (6%) of a candidate's second best, or her saving within `PROXY_SAVE` (6%) of the
  lowest that the shown rows (`OPT_SHOW` = 8) or the pick's near-tie band take. Recalibrated 2026-10-09 with
  `proxy_check` on val/plan: the proxy reads at most 2.79% / 2.67% low. A row left on its proxy carries `exact: false`.
  A row whose career search is not `complete` (it ran out of its work cap, `search_work`; a band overflowed; or a deck
  it scored passed its class's cap: the best career it found, not a proven best) carries `proven: false`, and the page
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
  parent. The trios' last step is such a career (C on A's and B's 8/8s), so without these rows the career V itself
  prices was never offered, and an uma whose best-odds pair happened to be two 8/8s took the goal's credit alone. They
  are not among Cont's builders and not added to the deck cache, so V and every other row are unchanged; a looped row
  that repeats another row's career is left out. The bar changes no looped value, but a barred search keeps fewer proofs,
  which pass 2's `certified` reads, so on an exact tie another pair of equal value could be named (not seen on
  val/plan or the large exports). On a 315-veteran pool with six 8/8s (one owned) they added 37 rows and, when added
  (2026-10-17, before the bar), took the plan from 11 s to 16 s natively; the bar cut their deck-search nodes there
  from 21.7M to 7.5M. On pools without two 8/8s they cost nothing.
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
  it in principle; on p16 with #14 borrowed V, the fit and the trios are the "allow" job's to the bit
  (`tests/plan_borrowed.rs`).

**Continuation and V.** Per candidate a continuation (`Cont`, assumed form; its shape constants 0.6 and 1.5 are tuning
choices, set against simulated builder traces on four veteran pools, a comparison not reproduced in the repository): builder careers lift her odds with chance
lam_j = lam ((1 - phi)^j)^`LAM_DECAY` (1.5) at the j-th success, each closing phi of the gap in log odds to
p_inf = `CEIL_SHARE` (0.6) x her odds with a full loop lineage (parents hypothetical 8/8s of her two most efficient
builders, bred from each other and from her), at least her odds now. Her step (a fresh 8/8) costs
min over n of sum_{j<n} 1/lam_j + 1/p_n (n up to `CONT_N` = 500; `cost_lo` stops once the building alone reaches the
best cost, so the cap decides nothing). `Cont::plan` and `Cont::cost_lo` are memoised per Cont by the bits of p
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
outcome is still summed at its own chance, in order. Exact in reals; a flat table's fold rounds differently per star
level, so scores move by at most about 1e-13 relative (decisions identical on val/plan); a roll (`next.roll`) reads
the same representative. Pick: the highest; within 0.25 careers per run (`NEAR`) or 5%, the better own odds. The goal is always the
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
  `ROLL_KEEP`, keyed by the plan job without `next.roll`, `no_reports` and `compare`; in `state_out`): the pick's last
  scoring as its fold and outcome loop read it (`Scored`: the kept tables, her miss factors and spark scales per
  setup, the collapsed targets, her landed sets' chances, her own 8/8's trio sides), the base it was scored against,
  her own 8/8's trio sides as the bound reads them at the plan's end (`pick_her`) and the main search's found-career
  anchors (`found_anchors`). `y`, the pick's careers saved, V - V with her veteran (to 0.01): the plan's own fold and
  outcome loop (`sc_eval`, generic over the leaves it produces: `All` in the plan, `OneLeaf` in a roll, which folds
  only the prefixes of the outcome's representative and runs the outcome loop at it and the outcome), so the f64 the
  plan's full table held, which the plan no longer computes (`tests/plan_roll.rs` checks rolls against the tables the
  plan exported before 2026-10-21, bit for bit). `y_hi` (rounded up to 0.01) where y < 0.5 (null elsewhere: the page
  reads it only below 0.5, and computes nothing else from it): at
  least `y`, the most her veteran's direct lifts could save for that outcome (each candidate's odds with her as a
  parent bounded over every partner y by the least over anchors A of c' E_A rise(q', q_A) (`Env::rise`: no assumed
  cap), E_A at least the best odds at q_A: today's pairs (y, z) at p_now / c_yz, the main search's found careers of the
  candidate's deck class at their `Ref`s, and, for the partners behind the band's likeliest outcomes, her best deck
  searched at her 2 and 3 stars on every uncertain target (`HI_STARS`; at most `HI_SEARCHES` searches, 0 by default = off, 10 when on; outcomes whose
  partners' bounds pass `HI_RISE` = 1.5 x p_now are left as they are), the latter two under the caps; and by the ceiling
  (C4's, `outfit_fast` at q'); the step at `Cont::cost_lo`). The least over anchors is found per (candidate, partner)
  by an exact branch and bound over the leaves' star-digit prefixes (an anchor's factors are positive, so its prefix's
  product times each remaining digit's least factor bounds its leaves from below; a subtree it can't lower is
  skipped; the leaves of no wanted outcome count as nothing to lower, so their subtrees are skipped too: a roll wants
  one, so each anchor's walk is one path), and each wanted outcome's bound is folded in as each partner finishes. A
  roll's first bound keeps, per (candidate, partner), the anchors whose root test passes against the most the ceiling
  holds at any leaf (`RollKeep::hi_anc`, not stored by `state_out`): every other anchor fails the test at every outcome,
  so later rolls fold in the same anchors without computing their chances again (e2_full_t10: 18,895 of 531,166). The
  page checks a roll with y < 0.5 <= y_hi exactly. The replan's other careers with her as a parent are not bounded (its
  candidates and Cont are the plan's).
- `her_trios`: her own 8/8 as a free trio side when her uma is not a stage candidate (at the rank's star mix, as the
  trios' fresh sides, with every candidate side and the best C).
- `umas_scored`, `umas_searched` (careers searched exactly), `umas_certified`, `umas_unproven` (umas whose best or
  looped career search is not `complete`, as for `proven` above), `hint`, `sur`, `sur_diff`, `work` (this
  plan's deck-search nodes and exact scorings, deterministic: a speed change can be measured at any machine load;
  `work.max_ratio` the largest exact/product ratio scored, `class_max_ratio` / `class_max_deck` per class, `class_caps`,
  `cap` the cap the plan's deck searches hold under), timings. A career deck search whose band would list over
  50,000 decks is rebuilt at 1.12, 1.05, then 1 (`Env::cert`); its answer is not a proof (no `Ref`), and the output's
  top-level `cap` is lowered to that cap, `complete` set false and `band_overflows` counts them.

**Kept plans.** Each trainee's best careers are kept per plan (the last few, in the worker). A plan of the same setup,
places and pool takes them as they are (no search: a reload, an edit undone); a plan of the same setup and places whose
pool is a kept one plus one veteran at the end (Compare, then Add) takes the better of the kept best and a search with
her fixed, whose bar starts at the kept value (`Env::floor`). Exact (on val/plan p16 identical to the
plan run cold), about 3 ms (2026-10-10). `best_deck`'s results (with their certificates, band overflows, work counts
and per-class ratios) and the corner tables are also kept between jobs of one setup (`DECK_KEEP`), so a reroll's plan
repeats none of the base plan's proxy searches. The kept plans, proofs, main searches and roll states survive a page reload through `cmd: "state_out"` /
`"state_in"` (fields table above).

#### Work budget

A career search has its own work budget (`CAREER_BUDGET`, 300M units: synthetic pools of 12-400 veterans at the default
8 targets needed 10M-85M for a proven result, measured 2026-10-07; the loop planner's 30M left pools of 100 and more
unproven; `EXACT_BUDGET` in the environment overrides it). It does not stop the search: past it each deck search scores
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
B and exceeds the table value tv by more than the margin m, her deck is searched there (`best_deck` at bar B) and
cached. Derived: the career picked has an exact P(all), with the deck it runs, of at least E*/(1 + m), E* the exact best
career's (unless a deck search ran out of work); m = 0 gives the exact greedy pick up to ties. A candidate is valued once,
when her later parent arrives; a trainee whose static ceiling (q = 1, every spark on six legacies) is below B is skipped.

| Field of `simulate` | Meaning | Default |
|---|---|---|
| `policy` | `greedy` (surrogate), `greedy_exact` (the career search in process per decision: the best career by P(all)), `loop` (a fixed rotation of `loop_members`), `trace` (building for `loop_members[0]` with the others as builders: each career the builder whose best career lifts her odds most efficiently runs; records her odds before and after, the builder, lam and the odds raised to, the traces `Cont`'s shape constants were set against, *Continuation and V*), `index` (the page's plan, *Next career*, below) | `greedy` |
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
of (always replacing her latest veteran collapses: a veteran missing a target breeds lineages with fewer carriers until
a target has none and P(all) is 0 for good). Bootstrap: a member without both parent veterans gets the best pair by
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
(the same run alone or among others, in any order, gives the same output). No checks or audits. Per run it adds
`index`: `plans` (plan jobs run), `plan_s` (each one's seconds), `roles` (the picks' `role` counts) and `best_odds` (picks
that were the main search's best career, the `greedy_exact` pick up to ties). On val/plan p10 (Agnes Digital
[Full-Color Fangirling] at P(all) 9.3e-4) the first plan of a run takes about 1.7 s and each later one 0.45-1.2 s; on
p16, 0.6-0.75 s and 0.23 s (native, one core, pools of 10-20). A plan's cost grows with the pool, so a run of a few
hundred careers takes minutes, and one to S < eps (about 7,000 careers at p near 1e-3) is out of reach: compare with a
`max_careers` cap, by the survival at the cap (`tail` x `final_p`, paired by run), since E's tail repeats the last pick
forever and the plan's last pick may be a builder's career with low odds.

Output: per run `E`, `careers`, `tail`, `final_p`, `pool`, `pruned`, `wall_s`, `refreshes`, `segs` (each pick change:
career, p, pool size, S), `checks`, `audits`; and `E` (mean, sd, 95% CI) over the runs. `trace` returns its rows per run
instead.

Adding a policy: `simulate` (src/deck/career/sim.rs) matches the policy name to a base (`Base`); `play` asks the base
for the next career each step (`greedy_cur`, `loop_cur`: a `Cur`, the career with its exact P(all), landed-set chances
and win chances), runs it, draws the veteran and adds her to the state (`St`). A new policy is a new arm there and a
function returning the `Cur` to run; `play` does the rest (E, the tail, pruning, checks, audits). A policy that runs a
different loop (as `trace` does) returns its own JSON from `simulate`. `index` (`index_cur`) runs a whole career job
per decision, as `greedy_exact` (`exact_cur`) does.

Results and commands: `val/sim/README.md`.

## 4. Validation

`cargo test --release` (in `pkg/loopopt`) covers, among others:
- **Reference fixtures** (`tests/fixtures/`):
  - 200 best responses from an exact MILP, including forced races;
  - 71 rest-forced best responses from the MILP, including turns 24/25/48/49/50 for the turn-49 goal umas;
  - the full 2,278-pair ceiling table.
- **Properties checked:**
  - `feasible()` rejects bad schedules;
  - a force that conflicts with a goal is infeasible;
  - the objective equals a direct per-legacy sum; optional G1s cost nothing; interchangeable races preserve the score;
  - a three-uma rotation's schedules are a Nash point of its objective.

The loader asserts the structural fact the exact DP relies on: every race offered twice is a Classic/Senior pair
exactly 24 turns apart.

## 5. Limits

- **Schedules are heuristic.** `optimize` is an iterated local search with a provably exact best response per uma; nothing proves a structure's schedules optimal.
- **Model assumptions (§1) are assumed or community-derived:** the win formula and the affinity objective. The skill model's per-legacy affinity is uma-it-optimizer's receipt-fitted model; its exact parent terms are fitted on totals only, so the split between them is less certain than the overall level. The code doesn't validate them.
- **Base-outfit aptitudes by default.** `build_base.py global CHARA:CARD` exports with an alternate outfit (Vodka 1008:100802, Daiwa 1009:100902, Urara 1052:105202).
