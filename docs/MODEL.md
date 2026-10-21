# Model assumptions and the evidence behind each

The full mechanics (algorithms, job fields, deck model) are in `docs/SOLVER.md`. This file lists every
assumption the odds rest on, where it came from, and how sure we are. "Measured" means fitted to real careers;
"community" means a widely repeated figure without published data; "assumed" means ours. Rows whose detail would not
fit a table cell point to a section below the table.

## Inheritance and loops

| Assumption | Value in the model | Source | Status |
|---|---|---|---|
| Relation points | `pair`/`tri` = sum of `succession_relation.relation_point` over groups containing all members | master.mdb | exact (game data) |
| Race affinity | +3 per shared single-race G1 win, by win-saddle group (relocated venues = same race, 1025→1012, 1027→1006, 1028→1005); multi-race titles count 0 | the game rule as community tools implement it; `docs/RESEARCH.md` §3 fits the weight per shared win on the runs' real lineages | fitted (weight 3 per shared win within the 95% interval) |
| Per-legacy affinity | parent P: pair(T,P) + pair(P1,P2) + Σ over P's own parents G [trio(T,P,G) + 3·shared(P,G)] + 3·shared(P,other parent); grandparent G under P: trio(T,P,G) + 3·shared(P,G). In a loop the other parent is also P's own parent, so the parents' shared G1s count twice | `docs/RESEARCH.md` §3, §5: the roll scales with this affinity on single-ancestor trials | fitted (slope within the 95% interval of 1) |
| Inheritance chance | per spark per legacy per roll: base × (1 + affinity/100), capped 1; two rolls per career (Classic and Senior April); grandparents full rate (no halving) | community figures; `docs/RESEARCH.md` §3 tests the skill-white rolls (grandparent weight, affinity slope, stars) and §13 on whole careers | fitted (grandparents full rate; skill whites calibrated) |
| Base by spark type and stars | skill/scenario white 3/6/9%, race white 1/2/3%, green 5/10/15%, pink 1/3/5%, blue 70/80/90% | community figures; skill whites tested in `docs/RESEARCH.md` §3 | skill whites fitted; the other types assumed (unused by the odds) |
| Star mix of legacy sparks | 50/45/5% (1/2/3★) for rank score 6,500–17,499 (default); 20/70/10% at 17,500+ (90/10/0% below 6,500), for skill (white) and unique (green) sparks; the page defaults the band by scenario: SS for Grand Concert and Grand Masters, below SS for URA Finale, Unity Cup and Trackblazer | community figures (mixes); the band per scenario from the runs' rank scores, `docs/RESEARCH.md` §2 | mixes assumed; band per scenario derived |
| Spark generation | learned white 20%, gold 40% (double-circle 25%), ×1.1 per ancestor carrying the spark; loop uses n = 6: 35.43% / 70.86%. The spark is always the group's white (a bought gold sparks its white) | community figure | assumed; the runs cannot test it (they show inherited sparks, not produced ones) |
| End-of-career spark reroll (the game: once per career every spark drawn again, the better roll kept, 30 TP) | not modelled: rerolling when she learned every target roughly halves the careers to a fully looped parent (per career P(all land) becomes (2 - G) P, G under 0.001), but scales every career alike, so it doesn't change which career to run | the game; measured 2026-10-21 on 11 plan jobs (picks unchanged, estimate x0.50-0.56) | left out: it moves no decision |
| Every target spark on all six legacies | yes (a running loop keeps veterans that landed everything) | assumed | simplification; a loop being built has fewer (the career planner uses real carriers) |
| Great-grandparents | no effect on affinity or carrier counts | the game's two-generation lineage | assumed (the runs show no great-grandparents) |
| Loop structure and ranking | four-uma loops (12 lineages: trainee, two of the other three as parents, each parent bred from the other parent and the fourth uma) and three-uma rotations (3 lineages: each parent bred from the other parent and the trainee's own earlier veteran), ranked together on the mean P(all) over their lineages, i.e. every member trains equally often | a modelling choice (2026-10-07); the rotation occurs in uma-it runs (`docs/RESEARCH.md` §6) and in GameTora's legacy guide ("she can be her own sub-legacy") | the equal-training average is a choice, not a fit: it is the long-run rate of a player who trains every member in turn. A player who keeps re-training only the best trainee would score a structure by its best lineage instead (not offered) |
| Repeated character in a trio | tri(T, P, T) = 0: a relation group counts only for distinct characters (`rel_tables`), so a self-grandparent brings only her shared G1s with her child | fitted on uma-it inspiration trials (`docs/RESEARCH.md` §5, *Self-grandparent*); uma.moe's backend uses 0, uma-it-optimizer counts pair(T, P) | measured (a rate fit; the game's code path is not seen) |

## Races

| Assumption | Value in the model | Source | Status |
|---|---|---|---|
| Win chance in IT | 110 − cost(distance) − cost(surface) − streak penalty, clamped; cost S/A 0, B 10, C 20, D 30, E 40, F 60, G 90; penalty by races in a row 0,0,5,20,30,50 | community table; `docs/RESEARCH.md` §4 tests it on every G1 attempt in the runs | fitted: calibrated on aptitudes raised by pink sparks (as the career planner raises them); the loop model keeps base aptitudes, which under-predict wins (a simplification); goal races from the 4th in a row win more than the penalty allows |
| Turns a career forbids racing | `single_mode_race_restrict_turn` (chara, turn, gain_id = the story event that sets it, e.g. 107811811 = story 501078118): that turn keeps only her goal races. Global: Yamanin Zephyr turn 44 (no Tenno Sho (Autumn), Shuka Sho or Kikuka Sho in Classic), Sweep Tosho turn 36 (no Classic Takarazuka Kinen), both fixed story events with +20 energy; Matikanetannhauser turn 70 = her event-assigned Japan Cup goal | master.mdb; event titles from `text_data` 181 | exact (game data); that the Zephyr and Sweep events always fire is read from their being fixed story events, not tested |
| Loop schedules | chosen to maximise the total expected shared G1 wins (every pair alike), not the landing odds | `docs/RESEARCH.md` §6: on the validation results, any other schedule could raise a winner by at most the solver's pair-ceiling bound | derived (bound computed) |

## Deck side

| Assumption | Value in the model | Source | Status |
|---|---|---|---|
| Training hints | h = base(scenario) × (1 + hint_frequency/100) × (1 + d(scenario) × specialty_priority/100) draws per card per run, each uniform over the card's skill options still below hint Lv 5 (events still hint a skill at Lv 5); base / d: Unity Cup 2.36 / 0.36, Grand Concert 1.97 / 0.08, URA Finale 1.34 / 0.39, Trackblazer 1.70 / 0.11, Grand Masters 1.86 / 0.26 (placeholder: the pooled fit, no runs) | `docs/RESEARCH.md` §7 | fitted; known misses and open questions in §7 |
| Chain events | Binomial(21, 0.35) per run, each advancing a random remaining chain slot of a non-pal card, weighted by links left; pals and R cards take no slot; IT's default choice ending a chain on k of n outcomes stops it there with chance k/n | `docs/RESEARCH.md` §8 | fitted; known misses in §8 (a binomial with another n fits better) |
| Event outcomes and choices | an event's outcomes exclude each other (equally likely unless measured); where an event offers targets on different choices a deck entry takes one choice, shown as the skill to prioritise; a card's gold and white of one skill are combined exactly | GameTora event structure (`alts` in card_events.json, `export/event_choices.py`) | follows from the event data; crediting each skill of an event separately would overstate the odds |
| Event outcome shares | equally likely unless measured; five measured shares adopted (`pkg/loopopt/data/event_shares.json`, `manual.json`): Anshinzawa SSR's finale, Riko Kashimoto SSR's Ramp Up, Seiun Sky [Foolproof Plan] link 3, Smart Falcon [My Umadol Way! ☆] link 3 and Nakayama Festa [Trusting the Dice]'s Nothing Ventured | `docs/RESEARCH.md` §9 | measured |
| Joint landing odds | the targets do not land independently: chain cards draw on one run's chain events (one binomial count per run), so their rewards land or miss together, and one event's outcomes exclude each other. Odds shown and the ranking are exact over the chain-progress distribution and those outcomes; one card's training draws are also joint over its targets. The search prunes with the product of per-target odds times an assumed cap per deck structure class (`docs/SOLVER.md`, *Exact odds*) | derived from the chain and event model above; the caps assumed from measured exact/product ratios | exact for the model under the stated caps, relative to the schedules found; the ratios seen, the cap checks and why no proven cap is used: `docs/SOLVER.md` |
| IT event choices | top choice, unless a prioritized skill (up to 10, chosen at career start) sits on another choice; the model assumes priorities reach the hints it counts | player practice; a uma-it run shows a "Prioritized skills" list | assumed; tested through its consequences (`docs/RESEARCH.md` §9, §13) |
| Random card events (a deck card's character's shared events) | not modelled | `docs/RESEARCH.md` §10 | not modelled; measured: leaving them out under-predicts the two default targets they can hint (Nimble Navigator, Ramp Up) on decks with those characters |
| Conditional character events (secret and race-result events) | not modelled, except Tokai Teio's Head-On (her Triple Crown event; all three races are career goals), counted as always acquired (job `extra`, as Yamanin Zephyr's Slipstream) | `docs/RESEARCH.md` §11; `uv run val/secret_events.py` lists them | not modelled except Head-On; measured: calibrated where only they can hint a target, but Vodka's own events give her Nimble Navigator in most runs, which the model misses |
| Pal first-year dates | don't fire in Unity Cup IT (Tazuna: Tail Held High; Anshinzawa: Uma Stan; SSR and R); always left out (the data keeps them disabled with a reason) | player observation | stated, not measured |
| Pal / group card events | hand-entered, each with a source, audited by `val/event_audit.py` against GameTora; pal finales certain | GameTora caches, player observation, umamusu.wiki; certainty checks in `docs/RESEARCH.md` §9 (Light Hello [From the Ground Up] credits See Ya Later! to her event in every run carrying her) | audited 2026-10-03, all target hints accounted for. uma-it-optimizer's "pal events only fire in the pal's own scenario" was not adopted: it rests on one card (SR Aoi Kiryuin) and contradicts player experience |
| Character rule | no trainee runs a card of her character or a group card she belongs to | the game's deck rule | game rule |
| Per-uma decks | each trainee uses her own best legal deck (default on) | design choice | exact per model; brute-force checked (`val/owncheck.py`, `val/permcheck.py`) |

## Career planner (`/career`)

| Assumption | Value in the model | Source | Status |
|---|---|---|---|
| Veterans | the loop model with what real veterans carry: each legacy's stars per target (no spark: no chance), recorded single-race G1 wins for race affinity, the recorded parents as grandparents (unknown parent: no relation points, sparks still count); spark generation ×1.1 per legacy actually carrying the spark | umadump exports (trained_chara_data.json; rental_chara_data.json for the borrow list): factor_id = factor group × 100 + stars, saddles via `single_mode_wins_saddle.group_id`; borrowed when owner_viewer_id ≠ 0 | exact decoding (master.mdb); borrowed flag checked on one export of each kind (owner_viewer_id 0 on 242 own veterans, the lender's viewer_id on 69 borrowable; use_type 0 on all) |
| Career schedule | the trainee's schedule maximizes expected distinct G1 wins, a race a parent won counting double (affinity later); no training cost, as in the loop planner | derived | |
| Next career (the planner's pick, `next`) | the career that brings a fully looped parent (an 8/8 whose parents are 8/8s) closest per run, with each candidate's continuation (`Cont`: how her own odds rise as builder careers improve her lineage) priced in; mechanics in `docs/SOLVER.md`, *Next career* | continuation form assumed, fitted per pool; shape fitted to simulated builder traces | section *Next career* below |
| Compare and Reroll (*Veteran analysis*) | Compare: the planner's expected careers to a fully looped parent (V) for the pool now against a full plan of the pool with her added, fitted to that pool (the plan Add to veterans then shows); her own 8/8 with 8/8 parents is a fully looped parent. Reroll asks the plan for that one outcome's saving (`next.roll`, from the pick's kept scoring), proven "no saving" where its bound `y_hi` is under half a career, else checked by a Compare | exact parts as the planner's | the two Vs come from two fits (Cont is refitted per pool), so a veteran can raise V; the page shows a rise as no change. Monotonicity is checked by `val/mono.py`, not assumed |
| Aptitude (pink), blue and unique sparks | section *Sparks the odds don't use* | | |

### Next career

The mechanics (objective V, `Cont`, a career's score, builders through their deck class) are in `docs/SOLVER.md`,
*Next career*. The evidence behind them:

- **Cont's shape.** `CEIL_SHARE` 0.6 and `LAM_DECAY` 1.5 are fitted to the measured builder traces of
  `val/sim/README.md` E5 (four player pools of 2, 5, 11 and 19 veterans): Cont tracks achievable building within 0-5% on
  2, 11 and 19 veterans and is 70% too pessimistic on 5, where lam rises after the first lift (builders improve) while
  Cont only lets it decay (taking lam as the larger of now and one lift later fixes most of it: 995 -> 734
  careers on 5 veterans, the others unchanged; not built); leave-one-pool-out fits of the shape ranged 0.6-1.0 and 1-3, so it is a calibration on
  these pools, not a law. Not yet measured against other policies: the simulator's `index` policy runs the page's
  planner on the same draws as `greedy` (`docs/SOLVER.md`, *Policy simulator*), but no comparison of useful size has
  been run.
- **Builders through their deck class** (2026-10-08): the proxy, checked 2026-10-09 with `proxy_check` on val/plan,
  reads at most 2.79% low (efficiency) and 2.67% (saving), so the 6% band (`PROXY_EFF`, `PROXY_SAVE`) is over twice
  the worst error; picks, the top-8 rows, V and the shown scores are unchanged on val/plan.
- **Forms rejected**: crediting each improvement as the last (a frozen tail) overrated builders 10-40x; a single-step
  trainee score gave a strong uma's career no credit as a parent for her partners; pricing C from hypothetical parents
  bred from each other asked for a full loop one generation deeper than the goal and ignored real 8/8s.

### Sparks the odds don't use

**Aptitude (pink).** Her grades: each uma's ten aptitude codes (first outfit, `card_rarity_data`). At the start, per
aptitude the six legacies' pink stars summed raise her base grade by 1/2/3/4 at 1/4/7/10★, at most to A. At each April
inspiration (Classic and Senior, 0-based turns 30 and 54; races from that turn on) each legacy's pink fires with base
1/3/5% by stars × (1 + that legacy's affinity / 100), +1 grade per firing, at most S. Her schedule's win chances are
the expectation over those grades (the schedule is fixed: not re-planned after a firing). Her veteran's pink: one,
uniform over the aptitudes she ends at A or S, stars 20/70/10 (exact over the firings; the page's roll draws it).
Pinks don't move skill odds. Sources: generation from hakuraku (107,159 characters: uniform over A/S, p = 0.39; stars
20/70/10, p = 0.73) and umamusustation (22,411 team-trial horses: A or better required, S not favoured, stars 20/70/10
at A and S alike), measured; the start thresholds, the A cap, +1 per firing and S being reachable from umamusu.wiki and
uma.guide (community; master.mdb's `succession_factor_effect` holds only generic values 1 and 2 per spark type and
star level, not decodable; the uma-it runs record which pinks fired, not her grades). That the April turns count for
that turn's race is assumed.

**Blue (stat) and unique (green)**, rolls only. Her blue: one of the five stats, each 20%; its stars by that stat's
value (under 600: 90/10/0, 600-1099: 50/45/5, 1100+: 20/70/10); stats aren't modelled, so the rank band's star mix
stands in. Her unique spark (her outfit's): always (guaranteed for an outfit at 3★ or more; the page assumes yours are),
stars by the rank band, as white sparks, not by the skill's level. Neither enters the odds; shown, rolled and kept on
her veteran. Sources: hakuraku (blue types over 107,159 characters, near-uniform, p = 0.0002: a slight lean to the
stronger stats; blue stars by stat band; unique stars by rank band over 22M team-trial umas), GameTora (a unique spark
is guaranteed at 3★ or more). The blue stars' stat band is an assumption.

## Where the model is weakest

The pieces that decide rankings, and have the least evidence, are the deck-side ones: training-hint counts, chain
completion, and event-choice behaviour; the uma-it runs have now measured the first two and four outcome shares
part by part. The composed model (a whole career's predicted P(target) against what landed) is tested end to end in
`docs/RESEARCH.md` §13. On the planner side, the continuation (`Cont`) is a calibration on four player pools.
