# Model assumptions and the evidence behind each

The full mechanics (algorithms, job fields, deck model) are in `docs/SOLVER.md`. This file lists every
assumption the odds rest on, where it came from, and how sure we are. "Measured" means fitted to real careers;
"community" means a widely repeated figure without published data; "assumed" means ours. Rows whose detail would not
fit a table cell point to a section below the table.

## Inheritance and loops

| Assumption | Value in the model | Source | Status |
|---|---|---|---|
| Relation points | `pair`/`tri` = sum of `succession_relation.relation_point` over groups containing all members | master.mdb | exact (game data) |
| Race affinity | +3 per shared single-race G1 win, by race (relocated venues = same race: 1025→1012, 1027→1006, 1028→1005, and in the loop planner also 1029→1016; the career planner reads a veteran's wins by win-saddle group); multi-race titles count 0 | the game rule as community tools implement it; `docs/RESEARCH.md` §3 fits the weight per shared win on the runs' real lineages | assumed, tested (weight 3 per shared win kept: a refitted weight does not predict unseen runs better, §3) |
| Per-legacy affinity | parent P: pair(T,P) + pair(P1,P2) + Σ over P's own parents G [trio(T,P,G) + 3·shared(P,G)] + 3·shared(P,other parent); grandparent G under P: trio(T,P,G) + 3·shared(P,G). In a loop the other parent is also P's own parent, so the parents' shared G1s count twice | `docs/RESEARCH.md` §3, §5: the roll scales with this affinity on single-ancestor trials | assumed, tested (slope 1 kept: a refitted slope does not predict unseen runs better; known miss: parents at affinity 200 or more read about 3% below the model, §3) |
| Inheritance chance | per spark per legacy per roll: base × (1 + affinity/100), capped 1; two rolls per career (Classic and Senior April); grandparents full rate (no halving) | community figures; `docs/RESEARCH.md` §3 tests the skill-white rolls (grandparent weight, affinity slope, stars) and §13 on whole careers | fitted (grandparents full rate; skill whites calibrated apart from the known misses in §3 and §13: Tail Held High, and parents at affinity 200 or more) |
| Skill purchases | IT buys no skills during a career; the player buys them at the career's end, so a skill is never learned before an inspiration (an inspiration can always hint a target the trainee has not been given) | the owner (game behaviour, 2026-10-22) | game rule |
| Goal races | a lost goal race does not end an IT career: the career goes on | the owner (game behaviour, 2026-10-22) | game rule |
| Base by spark type and stars | skill/scenario white 3/6/9%, race white 1/2/3%, green 5/10/15%, pink 1/3/5%, blue 70/80/90% | community figures; skill whites tested in `docs/RESEARCH.md` §3 | skill whites fitted; the other types assumed (unused by the odds) |
| Star mix of legacy sparks | 50/45/5% (1/2/3★) for rank score 6,500–17,499 (default); 20/70/10% at 17,500+ (the page offers only these two bands), for skill (white) and unique (green) sparks; the page defaults the band by scenario: SS for Grand Concert and Grand Masters, below SS for URA Finale, Unity Cup and Trackblazer | community figures (mixes); the band per scenario from the runs' rank scores, `docs/RESEARCH.md` §2 | mixes assumed; majority-band defaults chosen from measured rank distributions; Grand Masters assumed |
| Spark generation | learned white 20%, gold 40% (double-circle 25%), ×1.1 per ancestor carrying the spark; loop uses n = 6: 35.43% / 70.86%. The spark is always the group's white (a bought gold sparks its white) | community figure | assumed; the runs cannot test it (they show inherited sparks, not produced ones) |
| End-of-career spark reroll (the game: once per career every spark drawn again, the better roll kept, 30 TP) | not modelled: the gain depends on the acquired white/gold configuration and carrier counts; a common scaling across careers is not established | the game; measured 2026-10-21 on 11 plan jobs (picks unchanged, estimate x0.50-0.56) | left out; picks unchanged only on the tested jobs, with no general ranking guarantee |
| Every target spark on all six legacies | yes (a running loop keeps veterans that landed everything) | assumed | simplification; a loop being built has fewer (the career planner uses real carriers) |
| Great-grandparents | no effect on affinity or carrier counts | the game's two-generation lineage | assumed (the runs show no great-grandparents) |
| Loop structure and ranking | four-uma loops (12 lineages: trainee, two of the other three as parents, each parent bred from the other parent and the fourth uma) and three-uma rotations (3 lineages: each parent bred from the other parent and the trainee's own earlier veteran), ranked together on the mean P(all) over their lineages, i.e. every member trains equally often | a modelling choice; the rotation occurs in uma-it runs (`docs/RESEARCH.md` §6) and in GameTora's legacy guide ("she can be her own sub-legacy") | the equal-training average is a choice, not a fit: it is the long-run rate of a player who trains every member in turn. A player who keeps re-training only the best trainee would score a structure by its best lineage instead (not offered) |
| Repeated character in a trio | tri(T, P, T) = 0: a relation group counts only for distinct characters (`rel_tables`), so a self-grandparent brings only her shared G1s with her child | fitted on uma-it inspiration trials (`docs/RESEARCH.md` §5); uma.moe's backend uses 0, uma-it-optimizer counts pair(T, P) | measured (a rate fit; the game's code path is not seen) |

The inheritance miss products assume independent firings across legacies and inspirations conditional on stars and
affinities. Averaging unknown stars across slots assumes independent stars for distinct veterans; repeated uses of
one veteran share her stars. Future loop affinity uses expected shared wins inside the nonlinear inheritance formula.

## Races

| Assumption | Value in the model | Source | Status |
|---|---|---|---|
| Win chance in IT | 110 − cost(distance) − cost(surface) − streak penalty, clamped; cost S/A 0, B 10, C 20, D 30, E 40, F 60, G 90; penalty by position in a streak of consecutive race turns (any race counts, a rest resets) 0,0,5,20,30,50; a goal race (one the career's goals require) takes the previous position's penalty, so goal races pay 0,0,0,5,20,30, and still counts toward the position of the races after it; every race of a goal over several turns (Fuji Kiseki, Fine Motion, Sakura Chiyono O, Matikanefukukitaru; master.mdb `determine_race` 3) is a goal race; in the career planner, a race whose aptitude basis (110 − both costs, on her grades with pinks) is 70 or less costs 3 more (the loop planner keeps the table there); repeated attempts share the pink aptitude history: P(at least one win) = E[p_C] + E[p_S] − E[p_C p_S], with independent race outcomes conditional on that history; certain aptitudes use 1 − (1−p_C)(1−p_S) | community table, with the goal shift fitted to the runs: `docs/RESEARCH.md` §4 tests it on every G1 attempt in the runs (the shift predicts held-out runs better than the community table, and counting goal races better than not counting them) | fitted: on aptitudes raised by pink sparks (as the career planner raises them) every streak position to the 6th is inside its interval or within 1% in wins, an allowance chosen after seeing the data; known misses (§4): goal races 4th in a row and chosen races 1st and 2nd lose more often than predicted, the lowest aptitude basis wins less (the career planner's extra cost is the one §4 fits there, held out against the table; on base grades, the loop planner's, the table errs the other way there), Trackblazer's goal races 4th to 6th win less; multi-turn goal groups are modelled with every option a goal, but the run records (each race's "Career-required" label, pooled) mark both of Fuji Kiseki's options required in 129-132 of 136 runs, Fine Motion's Mile Championship in 0 of 10 and Sakura Chiyono O's Takarazuka Kinen in 2 of 11 runs, so the grouping is not fully understood; the loop model keeps base aptitudes, which under-predict wins (a simplification); goal races 7th or later in a row win less than predicted even at the full penalty of 50, and the streak count stopping at 6 adds to it (exposure in optimised schedules unmeasured) |
| Goals besides races | not modelled: the export reads race goals only (master.mdb `single_mode_route_race` `condition_type` 1); fan-count goals (`condition_type` 3: 27 in Global, over 22 characters) and graded-placement goals (`condition_type` 2: 5) are left out | master.mdb | a missed goal does not end the career, so the schedule need not meet them; the non-G1 races a player runs for fans take turns and streak positions the schedule does not see (untested) |
| Turns a career forbids racing | `single_mode_race_restrict_turn` (chara, turn, gain_id = the story event that sets it, e.g. 107811811 = story 501078118): that turn keeps only her goal races. Global: Yamanin Zephyr turn 44 (no Tenno Sho (Autumn), Shuka Sho or Kikuka Sho in Classic), Sweep Tosho turn 36 (no Classic Takarazuka Kinen), both fixed story events with +20 energy; Matikanetannhauser turn 70 = her event-assigned Japan Cup goal | master.mdb; event titles from `text_data` 181 | exact (game data); that the Zephyr and Sweep events always fire is read from their being fixed story events, not tested |
| Loop schedules | chosen to maximise the total expected shared G1 wins (every pair alike), not the landing odds | `docs/RESEARCH.md` §6: on the validation results, with each loop's reported decks fixed and outfits reselected, any other schedule could raise a winner by at most the solver's pair-ceiling bound | derived for fixed decks; the pair ceiling bounds overlap without requiring a jointly attainable pair of schedules |

Repeated race outcomes are assumed independent conditional on the modelled race state. The schedule model assumes
its chosen race programme does not change the fitted training-hint and chain-event budgets; this is untested.

## Deck side

| Assumption | Value in the model | Source | Status |
|---|---|---|---|
| Training hints | h = base(scenario) × (1 + hint_frequency/100) × (1 + d(scenario) × specialty_priority/100) draws per card per run, each uniform over the card's skill options still below hint Lv 5 (no stat option in the pool: tested in §7, where draws that could also pick it, as often as each skill, fit unseen careers worse) (events still hint a skill at Lv 5); base / d: Unity Cup 2.428 / 0.299, Grand Concert 1.990 / 0.082, URA Finale 1.369 / 0.393, Trackblazer 1.579 / 0.162, Grand Masters 1.991 / 0.134 (placeholder: one base and one d fitted over every scenario's runs; it has none); the solver reads them from `research/results.json` when it is built, rounded to 3 decimals | `docs/RESEARCH.md` §7 | fitted; known misses and open questions in §7, among them that a card whose pool skills an inspiration hinted gives its other skills more hints than predicted (a removal share fits unseen careers better; not modelled) |
| Chain events | Binomial(21, p) per run, logit p = -1.09469 + 0.168072 × the deck's pals (pal cards, group cards not counted, and plain pals), each advancing the chain of one of the deck's still-running chain cards, each equally likely whatever its links left (the card rule); pals and R cards take no slot; IT's default choice ending a chain on some of its outcomes stops it there on 1/3 of its firings, one share for every such link (k/n before) | `docs/RESEARCH.md` §8 | fitted on 8,722 Independent Training runs from uma-it with readable chain markers (rate, card rule, stop share); held out it beats the old Binomial(21, 0.35) with links-left weighting and the same rate with links-left weighting; decks with Sasami Anshinzawa [This Might Sting!] need no term of their own; known misses: Inari One's careers reach fewer events, early links read short at 3 pals and long at 1, and with five chain cards chains that never start are slightly over-predicted |
| Event outcomes and choices | an event's outcomes exclude each other (equally likely unless measured); where an event offers targets on different choices a deck entry takes one choice, shown as the skill to prioritise; a card's gold and white of one skill are combined exactly | GameTora event structure (`alts` in card_events.json, `export/event_choices.py`) | follows from the event data; crediting each skill of an event separately would overstate the odds |
| Event outcome shares and per-career approximations | equally likely unless measured; four measured shares adopted (`pkg/loopopt/data/event_shares.json`, `manual.json`): Anshinzawa SSR's finale, Riko Kashimoto SSR's Ramp Up, Seiun Sky [Foolproof Plan] link 3 and Nakayama Festa [Trusting the Dice]'s Nothing Ventured; Smart Falcon [My Umadol Way! ☆] link 3 is back at the equal rule (its earlier 0.65 rested on readings biased by shared credits) | `docs/RESEARCH.md` §9 | measured with qualifications: Riko's adopted rate is the per-run chance on runs that prioritise Ramp Up and not Lone Wolf; Festa's denominator is partly imputed from exclusive-credit rates; rounding it into binomial trials gives an interval with unestablished coverage |
| Joint landing odds | the targets do not land independently: chain cards draw on one run's chain events (one binomial count per run), so their rewards land or miss together, and one event's outcomes exclude each other. Odds shown and the ranking are exact over the chain-progress distribution and those outcomes; one card's training draws are also joint over its targets. The search prunes with the product of per-target odds times an assumed cap per deck structure class (`docs/SOLVER.md`, *Exact odds*) | derived given the independence assumptions below; the caps assumed from measured exact/product ratios | chosen-deck evaluation exact under the independence assumptions; search optimality conditional on the stated caps and schedules found; the ratios seen, the cap checks and why no proven cap is used: `docs/SOLVER.md` |
| IT event choices | top choice, unless a prioritized skill (up to 10, chosen at career start) sits on another choice; the model assumes priorities reach the hints it counts | player practice; a uma-it run shows a "Prioritized skills" list | assumed; tested through its consequences (`docs/RESEARCH.md` §9, §13) |
| Random card events (a deck card's character's shared events) | not modelled | `docs/RESEARCH.md` §10 | not modelled; the hybrid acquisition reconstruction in §10 under-predicts Nimble Navigator and Ramp Up on decks with those characters; it mixes exact landing marginals and approximate gold chances on both comparison sides; the reported odds change is a product of positive marginals |
| Conditional character events (secret and race-result events) | not modelled, except Tokai Teio's Head-On (her Triple Crown event; all three races are career goals), counted as always acquired (job `extra`, as Yamanin Zephyr's Slipstream) | `docs/RESEARCH.md` §11; `uv run val/secret_events.py` lists them | not modelled except Head-On, whose certainty is assumed from observed wins; the sparse conditional-only test uses the same hybrid reconstruction as §10 and excludes zero-baseline rows from its score; it does not settle trainee-specific costs, and Vodka's own events give her Nimble Navigator in most runs, which the model misses |
| Pal first-year dates | don't fire in Unity Cup IT (Tazuna: Tail Held High; Anshinzawa: Uma Stan; SSR and R); always left out (the data keeps them disabled with a reason) | player observation | stated, not measured |
| Pal / group card events | hand-entered, each with a source, audited by `val/event_audit.py` against GameTora; pal finales assumed certain; finite all-success samples do not establish certainty | GameTora caches, player observation, umamusu.wiki; certainty checks in `docs/RESEARCH.md` §9 (Light Hello [From the Ground Up] credits See Ya Later! to her event in every run carrying her) | audited 2026-10-03, all target hints accounted for. uma-it-optimizer's "pal events only fire in the pal's own scenario" was not adopted: it rests on one card (SR Aoi Kiryuin) and contradicts player experience |
| Character rule | no trainee runs a card of her character or a group card she belongs to | the game's deck rule | game rule |
| Per-uma decks | each trainee uses her own best legal deck (default on) | design choice | exact per model; brute-force checked (`val/owncheck.py`, `val/permcheck.py`) |

The joint composition assumes independent training processes across cards and independent event-outcome groups
conditional on chain progress. Deck acquisition, own-event acquisition and inheritance are assumed independent given
the model inputs. After conditioning on the joint deck outcomes, remaining inheritance, own-event and generation
outcomes are assumed independent across targets. These assumptions are separate from exact summation within the model.

## Career planner (`/career`)

| Assumption | Value in the model | Source | Status |
|---|---|---|---|
| Veterans | the loop model with what real veterans carry: each legacy's stars per target (no spark: no chance), recorded single-race G1 wins for race affinity, the recorded parents as grandparents (unknown parent: no relation points, sparks still count); spark generation ×1.1 per legacy actually carrying the spark | umadump exports (trained_chara_data.json; rental_chara_data.json for the borrow list): factor_id = factor group × 100 + stars, saddles via `single_mode_wins_saddle.group_id`; borrowed when owner_viewer_id ≠ 0 | exact decoding (master.mdb); borrowed flag checked on one export of each kind (owner_viewer_id 0 on 242 own veterans, the lender's viewer_id on 69 borrowable; use_type 0 on all) |
| Career schedule | the trainee's schedule maximizes her expected wins of the G1s her parents won, each weighted by the chance each parent won it (affinity later: her parents' wins are the guide to what her later partners will have won); a G1 neither parent won is run where it costs those races almost nothing (a 1e-7 scalar tie-break, as in the loop planner: it can trade at most about 3.4e-6 expected weighted wins of them, none on base tables with certain parent wins); no training cost | the owner's rule (2026-10-22) | assumed |
| Next career (the planner's pick, `next`) | the career that brings a fully looped parent (a full veteran whose parents are full veterans) closest per run, with each candidate's continuation (`Cont`: how her own odds rise as builder careers improve her lineage) priced in; mechanics in `docs/SOLVER.md`, *Next career* | continuation form assumed, fitted per pool; shape fitted to simulated builder traces | section *Next career* below |
| Compare and Reroll (*Veteran analysis*) | Compare: the planner's expected careers to a fully looped parent (V) for the pool now against a plan of the pool with her added under today's plan's fit (`next.fit`: its stage candidates and continuations, so the two Vs differ by what she changes, not by a refit); Add to veterans then plans the pool with her afresh, refitted, so its V can differ from Compare's either way. Her own full veteran with full veteran parents is a fully looped parent. Reroll asks the plan for that one outcome's saving (`next.roll`, from the pick's kept scoring: her stars per target, her G1 wins at their expected chances), which prices her in the plan's routes (a candidate's parent beside a kept partner, so the route's last career's grandparent; her own full veteran as a side): shown at once as provisional and confirmed by a Compare once the roll has stood 0.8 s (the plan's routes are not every route a replan can take: another lineage of a candidate's whose last career she improves as a grandparent, without lifting the candidate's odds, can save careers). The plan, and so a roll's saving, takes at most 12 targets (10 not guaranteed by the scenario); over 12 the career has no joint outcome table, so Reroll draws each target on its own odds, independently | exact parts as the planner's | V can still rise with her under the same fit, less often: a fresh side may take any of a candidate's top lineages (`fresh_lineages`, 3: her best career now and her next two by her cached decks, each searched with both parents fixed), so the lineage her arrival displaces from a candidate's best career stays on offer while it stays among them; it can still drop out (a veteran can open three better pairs, and a pair search that runs out of budget can stop below the pair's best) (2026-10-24, before: 11 of 50 rolled veterans of the val pools' picks, +51 to +189 careers, on p10, p10_pink and p11; 2026-10-27, after, `val/mono.py -n 3` on the 13 val/plan pools: 1 of 39 rose under today's fit, p16_t10 by 35.48 of V 9,747.66, and that rise is gone with 8 fresh lineages, so it is the truncation; Compare, at 3, still shows it; refitted, as Add plans, 9 of the 39 rose, by 2 to 371 careers, up to 14% of V, on p10_pink; final tree 2026-10-27). The page shows a rise as it is and says it is the planner's, never clamps it |
| Aptitude (pink), blue and unique sparks | section *Sparks the odds don't use* | | |

### Next career

The mechanics (objective V, `Cont`, a career's score, builders through their deck class) are in `docs/SOLVER.md`,
*Next career*. The evidence behind them:

- **Cont's shape.** `CEIL_SHARE` 0.6 and `LAM_DECAY` 1.5 are a calibration to builder traces measured with the
  policy simulator's `trace` policy (`val/sim/README.md`), first fitted on 2026-10-08 on four player pools that are not
  in the repository. Refitted on 2026-10-26, after the lift chance came to follow her odds, lam and phi to come from the
  builder whose continuation costs her least, and p_inf's grandparents to carry the rank's star mix and expected wins
  (`val/sim/fit_cont.py`: the 9 pools of `val/sim/trace_pools.json`, each pool's two stage candidates, 6 runs of 400
  careers each). Fitted to the traces' odds the shape runs to the grid's edge (1.2 / 3.0) and misses the step's cost
  held out by 18% on average. Fitted to that cost, which is what the plan reads, it is 0.55 / 1.0 (leaving one pool out:
  0.5-0.65 / 0.6-1.3), with a held-out mean cost error of 10.8% against the current shape's 10.5%, so the current shape
  is kept. Per trace the current shape's cost is off the achievable cost (split in half) by -13% to +34%: too
  pessimistic on most pools where building pays (p10's Agnes Digital +34.5%, pool14's two +22% and +29.5%), too
  optimistic on p11 (-13.3%, -9.2%), exact on p16 and gm16, where building does not pay. Taking lam as the larger of now and one
  lift later changes no trace's cost (not built). These are the model's own simulated careers, not real ones. Not yet
  measured against other policies: the simulator's `index` policy runs the page's
  planner on the same draws as `greedy` and the plan's own trio, under goal `looped` (careers to a fully looped
  parent, the plan's goal; the default goal counts careers to the first full veteran, which the plan does not pursue:
  `docs/SOLVER.md`, *Policy simulator*), but no comparison of useful size has been run.
- **Cont above p0 and a lifted route's C** (design choices, 2026-10-27; neither validated against runs or traces).
  Priced on the rate at her odds, a candidate's step rose and fell with lifted odds above p0 (37 of the 94 reference
  fits, by up to 9 careers, about 3%), so a typical lift could raise her step. Above p0 the step is now interpolated
  between the costs to go from p0's own path's points (`docs/SOLVER.md`, *Continuation and V*), which never rise with
  the odds; at p0 it is unchanged, and so is the calibration above, which prices the step at p0. A route through a
  lifted side priced C on the lineage the lift made even where that side's step still builds, though its later
  builders replace those parents, which V's own route never pays for: on a pool of veterans carrying 7 of 8 targets
  with blank grandparents every row scored about -1, and on boot_mob roughly 500 of the pick's 547 was inferred to be
  this channel. Now such a route's C is V's (the trio's own C), and C is priced on the new lineage only where the
  side reruns at once.
- **The pick under the shape's and the calibration's uncertainty** (the robustness check, `docs/SOLVER.md`, *Next
  career*; the 13 val/plan pools, 2026-10-26): among the checked rows (the 8 shown on each pool), the planner's rule
  selects the same career under all six tested variants on 12. On p11 it selects different careers under the tested
  variants: Yamanin Zephyr [Fluttertail Spirit], the pick (taken within `NEAR` by her odds; Gold City scores 0.08
  higher), under the plan's model, `CEIL_SHARE` 0.65 with either decay and P(all) x 0.961; Agnes Digital [Full-Color
  Fangirling] (2.3x the odds, 0.33 below the top, just outside `NEAR`) at `CEIL_SHARE` 0.5 with either decay and P(all)
  x 0.766. Across the variants V moves by -14% to +31% and the pick's score by up to 1.8x (p10_pink: 3.85 to 7.04); a
  finishing row (p16_88's pick, both parents full veterans) stays at 0 under the P(all) variants, her own all-target
  chance scaled with every other. The check covers those rows and variants, with the plan's searches held fixed; it says
  nothing of errors outside them. Not validated against runs.
- **Shared builders** (the joint-lift oracle, a diagnostic; the same pools): pricing a builder whose veteran lifts both
  trio sides once, on a joint lattice of the sides' own continuations, lowers V by 0 to 8.3% (most on p16_t10, then
  p10_pink, p11_noagnes, p10, p16_ss: 3.1-4.3%; 0 on boot_mob, and on p16_88 and p16_borrow, whose cheapest trios
  have a real full veteran side) and changes
  the cheapest trio on 7 of 13. The lattice with each builder lifting only her own side reproduces today's V to 1e-15.
  No row is scored under it, so whether the pick would move is not measured.
- **Builders through their deck class**: the proxy does not hold its band everywhere. At integration (2026-10-27,
  `proxy_check` on the 13 val/plan pools) a builder's proxy efficiency read 46% low on p16_t10 (10 targets) and 6.4%
  low on p16_borrow, against the 6% band (`PROXY_EFF`), so there a better builder can stay unsearched; on those pools
  the plan with proxies still equalled the all-exact plan in pick, V and top 8 rows. Searching every builder exactly
  (`exact_all`, the same binary, the same picks and V everywhere) costs p16_t10 13.8 s against 19.3 s with proxies and
  p16_borrow 1.3 against 1.2 s, and more elsewhere (p10 3.37 against 2.21 s, p16 1.38 against 0.83, gm16_nu8 5.59
  against 3.75, p11 2.92 against 2.14, boot_mob 0.65 against 0.64, p11_noagnes 3.47 against 3.04, p10_pink 3.33 against
  2.23). So builders are searched exactly in a job with 10 targets or more or a borrowed veteran in the pool, and keep
  their proxies elsewhere (`docs/SOLVER.md`, *Pass 1*); the measured case is 10 targets of which 8 uncertain, while
  gm16_nu8 (8 uncertain of 8) keeps its proxies within the band. The saving errors measured then
  (up to 3.7 careers on p10's 7th shown row) come from `proxy_check` pricing the proxy on an earlier deck cache than
  the exact row, not from the proxy (on p10 and p11 the proxy is the same career). `val/planval.py` fails a pool when
  proxies move the pick, V or the top 8 rows, when a proxy efficiency reads more than 3% low, or when a proxy saving
  reads more than 0.25 careers low.
- **A lift's lineage**: a career's veteran lifts a candidate by being her parent, which makes her C's grandparent on
  that side; each such route is priced on its own lineage at each outcome (`docs/SOLVER.md`, *Pass 2*), with the
  route's C held fixed (a lifted side's route keeps its trio's C) and the lifted side's G1 wins from her best career's
  deck. On val/plan p10 (2026-10-26) the pick (Agnes Digital [Full-Color Fangirling]) is unchanged and its score is 5.32, against
  0.62 when C's odds were read on the trio's lineage as it was, without her in it. Not validated against runs.
- **Forms rejected**: crediting each improvement as the last (a frozen tail) overrated builders 10-40x; a single-step
  trainee score gave a strong uma's career no credit as a parent for her partners; pricing C from hypothetical parents
  bred from each other asked for a full loop one generation deeper than the goal and ignored real full veterans.

### Sparks the odds don't use

**Aptitude (pink).** Her grades: her outfit's ten aptitude codes (`card_rarity_data`). data.json's `apt` is each uma's first outfit's and `outfit_apt` lists the outfits whose own differ: Vodka [Fiery Aqua Vitae], Daiwa Scarlet [Nuit Étoilée de Scarlet] and Haru Urara [New Year ♪ New Urara!] (the JP era adds Tosen Jordan [Aurore☆Vacances], late surger only). The career planner grades a career by the outfit it runs: of her deck's outfits (those of equal coverage), the one whose schedule scores best; the loop planner keeps the first outfit's grades. At the start, per
aptitude the six legacies' pink stars summed raise her base grade by 1/2/3/4 at 1/4/7/10★, at most to A. At each April
inspiration (Classic and Senior, 0-based turns 30 and 54; races from that turn on) each legacy's pink fires with base
1/3/5% by stars × (1 + that legacy's affinity / 100), +1 grade per firing, at most S. Her schedule's win chances are
the expectation over those grades (the schedule is fixed: not re-planned after a firing). Her veteran's pink: one,
uniform over the aptitudes she ends at A or S, stars 20/70/10 (exact over the firings; the page's roll draws it). The plan's veteran before her result is known carries it as a mixture, each pink at its
chance (the plan's routes schedule her children). Her children's grades and schedules average over it exactly given
her pink is independent of her G1 wins, which she carries as separate chances; both follow one aptitude history (a
firing that raises her mile makes both a mile pink and her mile wins likelier), so this is an assumption, unmeasured
and second order. A slot of her character with the same possible pinks is taken as her again, sharing her pink.
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
completion, and event-choice behaviour; the uma-it runs measure the first two, and four outcome shares,
part by part. `docs/RESEARCH.md` §13 checks acquisition using products of card marginals and checks spark-weighted
acquisition against exact landing odds. It does not observe produced sparks or validate candidate rankings. On the planner side, the continuation (`Cont`) is a calibration on the model's simulated building on nine pools,
whose cost it misses by -13% to +34% per trace.
