# Model assumptions and the evidence behind each

The full mechanics (algorithms, job fields, deck model) are in `docs/SOLVER.md`. This file lists every
assumption the odds rest on, where it came from, and how sure we are. "Measured" means fitted to real careers;
"community" means a widely repeated figure without published data; "assumed" means ours. Rows whose detail would not
fit a table cell point to a section below the table.

## Inheritance and loops

| Assumption | Value in the model | Source | Status |
|---|---|---|---|
| Relation points | `pair`/`tri` = sum of `succession_relation.relation_point` over groups containing all members | master.mdb | exact (game data) |
| Race affinity | +3 per shared single-race G1 win, by win-saddle group (relocated venues = same race, 1025→1012, 1027→1006, 1028→1005); multi-race titles count 0 | uma-it-optimizer `affinity.py`; uma.moe matched exactly (umadump test, 2026-10-02) | confirmed by two tools; titles-count-0 shown by uma.moe |
| Per-legacy affinity | parent P: pair(T,P) + pair(P1,P2) + Σ over P's own parents G [trio(T,P,G) + 3·shared(P,G)] + 3·shared(P,other parent); grandparent G under P: trio(T,P,G) + 3·shared(P,G). In a loop the other parent is also P's own parent, so the parents' shared G1s count twice | uma-it-optimizer fit to 8,155 IT receipts (fired vs predicted within 0.1% parents, 2% grandparents); uma.moe gives identical numbers (the owner loaded the best Unity loop's 12 veterans, written by `val/umadump_lineage.py`: lineage total 452 = 184 + 97 + 171) | measured; uma.moe agreement shows our implementation matches theirs, not that the formula matches the game. Adopted in v19: under umaishow's earlier split (grandparents halved) every spark chance was about half the measured level |
| Inheritance chance | per spark per legacy per roll: base × (1 + affinity/100), capped 1; two rolls per career (Classic and Senior April); grandparents full rate (no halving) | herohero note (1,000+ trials), umareference/crazyfellow, uma-it-optimizer fit; uma.moe identical | measured / consistent |
| Base by spark type and stars | skill/scenario white 3/6/9%, race white 1/2/3%, green 5/10/15%, pink 1/3/5%, blue 70/80/90% | same; the model uses only skill whites | consistent across all sources |
| Star mix of legacy sparks | 50/45/5% (1/2/3★) for rank score 6,500–17,499 (default); 20/70/10% at 17,500+ (90/10/0% below 6,500). Applies to skill (white) and unique (green) sparks; pink stars are fixed at 20/70/10 and blue stars follow the stat's value (hakuraku note, its addendum on ~22M team-trial umas for green). The page defaults the band by scenario: SS for Grand Concert and Grand Masters, below SS for URA Finale, Unity Cup and Trackblazer | hakuraku.moe CM10–12 room data: 528,637 and 196,646 white sparks, fits p = 0.13 and 0.77. Scenario defaults from uma-it summaries (score if every skill point is spent): URA 0% reach 17,500, Unity 47% (median 17,428), Grand Concert 79% (median 18,094), Trackblazer 0% (median 12,386, 282 runs); Grand Masters has no IT data, SS is the owner's assumption | measured (adopted v21; the earlier 45/50/5 matched neither band) |
| Spark generation | learned white 20%, gold 40% (double-circle 25%), ×1.1 per ancestor carrying the spark; loop uses n = 6: 35.43% / 70.86%. The spark is always the group's white (a bought gold sparks its white) | aoneko_pochi 2024; hakuraku fit on ~1.2M observations (p ≥ 0.84 per tier); uma.moe uses the same curve; master.mdb has spark factors for whites only | measured (adopted v20 at the owner's request; the hakuraku data can't separate 0.35 from 0.354 at six carriers). Additive "+2.5 points per carrier" rejected (p ≈ 1e-14) |
| Every target spark on all six legacies | yes (a running loop keeps veterans that landed everything) | assumed | simplification; a loop being built has fewer (the career planner uses real carriers) |
| Great-grandparents | no effect on affinity or carrier counts | uma.moe test file B (2026-10-02) | uma.moe behaviour; matches the game's two-generation lineage |
| Year-1 factor gains in receipts | start-of-career application (all blues + unique sparks), not a third skill inheritance | one uma-it run (Special Week, 2026-10-02); uma-it-optimizer counts years 2 and 3 only | observed once |
| Hint level from inheritance | unknown rule (one 2★ proc of Ramp Up showed Lv 5 in a uma-it run) | one run | open; matters only if the model ever uses hint levels |
| Loop structure and ranking | four-uma loops (12 lineages: trainee, two of the other three as parents, each parent bred from the other parent and the fourth uma) and three-uma rotations (3 lineages: each parent bred from the other parent and the trainee's own earlier veteran), ranked together on the mean P(all) over their lineages, i.e. every member trains equally often | owner's decision 2026-10-07; the rotation's existence: 907 of 3,179 uma-it runs have a grandparent of the trainee's own character (none a parent), GameTora's legacy guide ("she can be her own sub-legacy") | the equal-training average is a choice, not a fit: it is the long-run rate of a player who trains every member in turn. A player who keeps re-training only the best trainee would score a structure by its best lineage instead (not offered) |
| Repeated character in a trio | tri(T, P, T) = 0: a relation group counts only for distinct characters (`rel_tables`), so a self-grandparent brings only her shared G1s with her child | fit on 101,536 inspiration trials attributable to one ancestor in uma-it runs: self-grandparents 523 passes seen, 521.8 predicted with 0, 605.4 with tri = pair(T, P) (about 13 nats worse; k·pair(T, P) fits k = 0, 95% upper limit about 0.35); uma.moe's backend uses 0, uma-it-optimizer the other | measured (a rate fit; the game's code path is not seen) |

## Races

| Assumption | Value in the model | Source | Status |
|---|---|---|---|
| Win chance in IT | 110 − cost(distance) − cost(surface) − streak penalty, clamped; cost S/A 0, B 10, C 20, D 30, E 40, F 60, G 90; penalty by races in a row 0,0,5,20,30,50 | community table | community |
| Turns a career forbids racing | `single_mode_race_restrict_turn` (chara, turn, gain_id = the story event that sets it, e.g. 107811811 = story 501078118): that turn keeps only her goal races. Global: Yamanin Zephyr turn 44 (no Tenno Sho (Autumn), Shuka Sho or Kikuka Sho in Classic), Sweep Tosho turn 36 (no Classic Takarazuka Kinen), both fixed story events with +20 energy; Matikanetannhauser turn 70 = her event-assigned Japan Cup goal | master.mdb; event titles from `text_data` 181 | exact (game data); that the Zephyr and Sweep events always fire is read from their being fixed story events, not tested |
| Loop schedules | chosen to maximise the total expected shared G1 wins (every pair alike), not the landing odds | tested 2026-10-03 (branch `sched-exp`, scratch): re-optimising every solved loop for the landing odds directly (gradient-weighted best responses, ascent, group moves, 30 ILS rounds) gained at most +0.12% on any loop, +0.065% on a winner, and changed no top-1 or top-3 loop over 46 validation runs; the search's ceiling (each pair at its best overlap) is only 0.56-2.83% above the found odds | measured; kept |

## Deck side

| Assumption | Value in the model | Source | Status |
|---|---|---|---|
| Training hints | h = base(scenario) × (1 + hint_frequency/100) draws per run (hint frequency at max limit break, the unique effect's included), each uniform over the card's skill options still below hint Lv 5 (the stat option is never drawn; a skill leaves after ceil(5 / levels per draw) draws; the cap applies to training hints only: events still hint a skill already at Lv 5, owner); base Unity Cup 2.70, Grand Concert 2.03, URA Finale 1.60, Trackblazer 1.81, Grand Masters 2.30 (placeholder, no runs); h read as floor or ceiling, weighted by its fraction; one card's draws over several targets are joint, so it hints two of them together less often than independent odds | `docs/RESEARCH.md` §6: Independent Training runs from uma-it, bases refit 2026-10-05 on 2,402 runs; skills-only, the near-fixed draw count, uniform draws with the Lv-5 cap and the joint formula all checked there | measured. Caveats: a given setup can be off by about half a draw (IT's count looks rounded per setup); Unity's hf slope rests on three cards (hf-0, specialty-0 cards over-rated about 10% in Unity); no R cards and hint frequency only 0–60 in the data; a specialty-priority term (×exp(0.025 per 10 points)) is a candidate, not adopted |
| Chain events | Binomial(21, 0.35) per run (mean 7.35, variance 4.8), each advancing a random remaining chain slot of a non-pal card, weighted by links left; pals and R cards take no slot; IT's default choice ending a chain on k of n outcomes stops it there with chance k/n (exact mixture) | `docs/RESEARCH.md` §7: fitted 2026-10-05 on uma-it IT runs whose chain progress is readable from event hints; distribution-free fit mean 7.30, variance 5.1; Binomial(21, 0.35) 6.6 log-likelihood units behind it with 14 fewer parameters, Poisson(7.1) 27 behind; no difference between scenarios (Unity 7.56, Grand Concert 7.16, URA 6.53; per-scenario fit gains 15.7 for 32 parameters); refit on 1,792 runs keeps it | measured, owner's call: keep the binomial and revisit with more data. Known limits: two-chain decks finish both chains in 82% of runs against 75% predicted; decks with 4–5 chain cards have about a third of chains never start (model ~16%) while final-link completion matches; the slot rule (over the card rule) is an empirical preference |
| Event outcomes and choices | an event's outcomes exclude each other (one random outcome per firing; equally likely unless measured, below); where an event offers targets on different choices, a deck entry takes one choice (the solver offers one entry per choice worth taking, like an early chain end), shown as the skill to prioritise; a card's gold and white of one skill are combined exactly | GameTora event structure (`alts` in card_events.json, `export/event_choices.py`) | follows from the event data. Fixed 2026-10-04: before, every skill of an event was credited on its own (Nakayama Festa [Trusting the Dice]'s Nothing Ventured / Risky Business gave any hint 75% of the link's firings instead of 100%; King Halo [Tonight, We Waltz]'s Stop Right There! and Uma Stan, on separate choices, were both credited) |
| Anshinzawa SSR finale (830080001) | always fires; with Risky Business prioritised IT takes choice 2: Nothing Ventured +1 on success (0.75), Risky Business +1 on failure | uma-it runs, 2026-10-06 (section *Event outcome shares*) | measured; value set by the owner |
| Measured event outcome shares | 0.75 for Riko Kashimoto SSR's "Unexpected Side" Ramp Up, Seiun Sky [Foolproof Plan] link 3 Fast-Paced, Smart Falcon [My Umadol Way! ☆] link 3 Center Stage (`chance` for the pal, `event_shares.json` for generated chain events, turned by the builders into chain shares and `alt_w` outcome weights) | uma-it runs, 2026-10-06 (section *Event outcome shares*) | measured; values set by the owner |
| Joint landing odds | the targets do not land independently: chain cards draw on one run's chain events (one binomial count per run), so their rewards land or miss together, and one event's outcomes exclude each other. Odds shown and the ranking are exact over the chain-progress distribution and those outcomes; training hints stay independent per target. The search prunes with the product of per-target odds times a cap per deck structure class (`EXACT_CAPS`, by the deck's chain-varying targets, those a chain link of one of its cards feeds): at most two such targets and no multi-target group 1.05, three or more (or a multi-target group) 1.25, the top cap `EXACT_RHO`. A search node takes the class of every target it can still reach. It filters candidates with a proven bound (negative orthant dependence of chain progress given the event count, checked exactly per chain-kind key), and reports the cap each answer is certified under (lower when the work budget runs out or a deck band would hold over 50,000 decks: 1.12, 1.05, or 1 = product ranking; the career planner then also reports `complete` false) | derived from the chain and event model above; every cap assumed from measurement: chain cards share one run's chain events and compete for them, which can correlate even one target's feeders, so no class has a proven cap | exact for the model under the stated caps, relative to the schedules found. Exact/product ratios seen: up to 1.18 on Global deck jobs (`pal_UF`; `pal_I` 1.165) and 1.17 on Grand Masters (0-1 pals), both class 4+; two targets at most 1.008, three 1.127 (King Halo in `boot_mob`; the 1.10 proposed for three would have been below it, so three takes 1.25). Checked on every deck scored, per class (`exact.class_max_ratio`, `next.work.class_max_ratio`; `val/cmp2.py` and `val/planval.py` fail when one passes three quarters of its cap's headroom over 1), and on pruned decks by `val/capaudit.py` (every cap at 2.0). No data-free proven cap exists (a target whose only source is one card's last link makes the ratio 1/P(link)). The cap-free state-conditioned bound was built and is used only as an extra prune at configuration roots: as a replacement it is 13-45x slower, so the owner kept the cap (`docs/PERF_PLANNER.md`); the per-class caps are the owner's call of 2026-10-09. Before v38 the search ranked by the product and rescored only what it kept: Grand Masters all owned reported 1 in 202 where 1 in 194 exists |
| IT event choices | top choice, unless a prioritized skill (up to 10, chosen at career start) sits on another choice; the model assumes priorities reach the hints it counts | player practice; a uma-it run shows a "Prioritized skills" list | stated; incomplete in the data (with neither skill prioritised, 20 of 72 Anshinzawa runs still took her choice 2) |
| Random card events (a deck card's character's shared events) | not modelled | `docs/RESEARCH.md` §9: fire in about 15% of careers (21% with 4 random events on offer, 13% with 14) | tested: crediting them at 0.17 raised odds by at most 1.7% and changed no best loop; not added (within the model's other uncertainties, and Seeking the Pearl's Uma Stan sits on a bottom choice with no readings) |
| Conditional character events (secret and race-result events) | not modelled, except Tokai Teio's Head-On (her Triple Crown event; all three races are career goals), counted as always acquired (job `extra`, as Yamanin Zephyr's Slipstream) | `docs/SECRET_EVENTS.md` (GameTora `training_events_char`, generated by `val/secret_events.py`): 81 umas, about 170 events with hints; 14 touch the default targets (Nimble Navigator, Ramp Up, Tail Held High), all as white hints | checked 2026-10-03: forcing every target-relevant event to fire with certainty, even as a gold, leaves the best loop unchanged on the default, GM-era and owner setups, with or without the Unity golds; owner: not worth modelling. Recheck with that forced-certain test (an `extra` entry per source) if a new target is mainly supplied by such events (Head-On is the likely case) |
| Pal first-year dates | don't fire in Unity Cup IT (Tazuna: Tail Held High; Anshinzawa: Uma Stan; SSR and R); always left out (the page option was removed 2026-10-03; the data keeps them disabled with a reason) | owner | owner-stated |
| Pal / group card events | hand-entered, each with a source, audited by `val/event_audit.py` against GameTora; pal finales certain | GameTora caches + owner + umamusu.wiki; Light Hello [From the Ground Up] credits See Ya Later! to her event in 812 of 812 runs | audited 2026-10-03, all target hints accounted for. uma-it-optimizer's "pal events only fire in the pal's own scenario" was not adopted: it rests on one card (SR Aoi Kiryuin) and contradicts the owner's experience |
| Character rule | no trainee runs a card of her character or a group card she belongs to | owner | game rule |
| Per-uma decks | each trainee uses her own best legal deck (default on) | owner request | exact per model; brute-force checked (`val/owncheck.py`, `val/permcheck.py`) |

### Event outcome shares

An event's outcomes are equally likely unless measured; outcome odds are server-side (master.mdb holds none). Counted
on uma-it runs (2026-10-06) only where every outcome could show (her hint credited, the skill's level from other
sources below 5). Intervals are 90%.

- **Anshinzawa SSR finale.** 254 runs with the card and without Nakayama Festa [Trusting the Dice] (whose "Upping the
  Ante" also gives Nothing Ventured +1/+3, credited jointly). Hint levels identify the choice: her Nothing Ventured +3
  = choice 1, +1 = choice 2. With Risky Business shown as prioritised she gave Nothing Ventured in 58 of 77 runs
  (0.75 [0.67, 0.82]), either choice, no loss classification needed: the model value. Choice 2 alone: 37/45 = 0.82
  [0.71, 0.90] in those runs without inherited Risky Business (a loss shows as her Risky Business at level 2: storyline
  choice 3 +1, finale +1); with inherited levels mixed in, 0.77–0.87. With Risky Business prioritised IT took choice 2
  in about 95% of runs; the exceptions mostly had Risky Business at hint level 5. With neither skill prioritised, 20 of
  72 runs still took choice 2. Choice 1 (Nothing Ventured +3 or nothing): 69/127 = 0.54 if the finale always fires;
  Grand Concert lower (unexplained). With Risky Business prioritised IT also takes the bottom choice of her date-start
  event "A Pleasant Practice Partner?" (Risky Business +1; GameTora marks it 'ee', which outside IT declines her
  dates), yet the finale still fires (at least 75 of 77 such runs show its outcome; her extra Risky Business +1 appears
  in 53 of 54 choice-2 wins with the priority, 1 of 20 without): IT apparently keeps her dates either way. Was 0.5 by
  the equal-outcome rule.
- **Riko Kashimoto SSR, "Unexpected Side" Ramp Up** (top choice; Maverick ○ otherwise): 527/585 = 0.90 [0.88, 0.92]
  when the top choice is taken (IT takes her bottom choice, Lone Wolf, when Lone Wolf is prioritised: 55 of 62 runs).
- **Seiun Sky [Foolproof Plan] link 3** Fast-Paced (Escape Artist otherwise): 33/40 = 0.82 [0.71, 0.90].
- **Smart Falcon [My Umadol Way! ☆] link 3** Center Stage (Prudent Positioning otherwise): 37/51 = 0.73 [0.61, 0.81].
- Off 50/50 but not changed: Tazuna Hayakawa SSR finale Concentration 0.70, Agnes Tachyon [Q≠0] link 1 choice 2 Murmur
  0.70, Marvelous Sunday [Dazzling Day in the Snow] link 3 Position Pilfer 0.58, Light Hello SSR's finale +3 See Ya
  Later! in 1,497 of 2,103 runs (0.71). Consistent with 0.5: Gold Ship [That Time I Became the Strongest] 0.61, King
  Halo [Tonight, We Waltz] 0.57, Daitaku Helios [Sunlit Angel ♪] Tail Held High 0.47 (given link 2 fired), Nakayama
  Festa [Trusting the Dice] Nothing Ventured 0.48–0.65 (37–50 readings).

## Career planner (`/career`)

| Assumption | Value in the model | Source | Status |
|---|---|---|---|
| Veterans | the loop model with what real veterans carry: each legacy's stars per target (no spark: no chance), recorded single-race G1 wins for race affinity, the recorded parents as grandparents (unknown parent: no relation points, sparks still count); spark generation ×1.1 per legacy actually carrying the spark | umadump export (trained_chara_data.json): factor_id = factor group × 100 + stars, saddles via `single_mode_wins_saddle.group_id` | exact decoding (master.mdb); borrowed flag (owner_viewer_id) unverified on a real export |
| Career schedule | the trainee's schedule maximizes expected distinct G1 wins, a race a parent won counting double (affinity later); no training cost, as in the loop planner | derived | |
| Next career (the planner's pick, `next`) | the career that brings a fully looped parent (an 8/8 whose parents are 8/8s) closest per run; section *Next career* | continuation form assumed, fitted per pool; shape fitted to simulated builder traces | see the section |
| Compare and Reroll (*Veteran analysis*) | Compare: the planner's expected careers to a fully looped parent (V) for the pool now against a full plan of the pool with her added, fitted to that pool (the plan Add to veterans then shows); her own 8/8 with 8/8 parents is a fully looped parent. Reroll reads the plan's per-outcome table (`next.outcomes`), proven "no saving" where its bound `y_hi` is under half a career, else checked by a Compare | exact parts as the planner's | the two Vs come from two fits (Cont is refitted per pool), so a veteran can raise V; the page shows a rise as no change. Monotonicity is checked by `val/mono.py`, not assumed |
| Aptitude (pink), blue and unique sparks | section *Sparks the odds don't use* | | |
| Look ahead, rollout, policy comparison, `plan` simulator | solver-only (`lookahead`, `rollout`, `policy`, `plan` job fields; the page sends only `next`): described in `docs/SOLVER.md`. `plan`'s surrogate measured: its greedy pick equalled the exact search's at 29 of 29 pick changes (career_U5, owner4, 4 × 25 careers) | derived | kept for reverting and for the simulator |

### Next career

- **Objective.** V = the expected careers to the next fully looped parent, any uma: the cheapest trio (A, B, C). A
  and B are stage candidates, each side either a real 8/8 veteran of hers (cost 0) or a fresh 8/8 (her step); C is any
  allowed uma at her odds with those two as parents (a fresh 8/8 being what her best career now would leave). A
  career with two 8/8 parents landing all targets reaches the goal (V after 0). A fully looped veteran you already
  have is a real 8/8 (free side) with 8/8 parents.
- **A step builds while it pays** (`Cont`, assumed form): builder careers lift her odds with chance lam_j = lam ×
  ((1 − phi)^j)^1.5 at the j-th success, each closing phi of the remaining log-odds gap to p_inf = 0.6 × her odds with
  a full loop lineage (parents hypothetical 8/8s of her two most efficient builders, bred from each other and from
  her); her step costs min over n of Σ_{j<n} 1/lam_j + 1/p_n. lam and phi are fitted to the pool now; the shape
  (`CEIL_SHARE` 0.6, `LAM_DECAY` 1.5) to the measured builder traces of `val/sim/README.md` E5. C's own step is not
  lifted by builders.
- **A career's score**: E[V − V with her veteran] − 1 over its outcomes, exact over every result she can roll, her
  veteran counting for every step it would parent better and her own 8/8 for her own step; two deep for the top 3
  and the best odds; pick the highest, near-ties (0.25 careers per run or 5%) to the better own odds.
- **Builders through their deck class** (owner decision, 2026-10-08): a builder's career is first taken from her
  class's winning decks (no career search) and searched exactly only where the plan can turn on her: within 6%
  (`PROXY_EFF`, `PROXY_SAVE`) of a candidate's two most efficient builders, or of the savings the shown rows, the
  pick's near-tie band and two deep take. Recalibrated 2026-10-09 with `proxy_check`: the proxy reads at most 2.79%
  low (efficiency) and 2.67% (saving), so 6% is over twice the worst error; picks, the top-8 rows, V and the shown
  scores unchanged on val/plan.
- **Evidence and errors.** E5 (owner's 2-, 5-, 11- and 19-veteran pools): Cont tracks achievable building within
  0-5% on 2, 11 and 19 veterans and is 70% too pessimistic on 5, where lam rises after the first lift (builders
  improve) while Cont only lets it decay; leave-one-pool-out fits of the shape ranged 0.6-1.0 and 1-3, so it is a
  calibration on these pools, not a law. The continuation's ceiling for Agnes Digital is about 1 in 165-180 (another
  agent's 1 in 166 is the same calculation, not independent). Not yet measured against other policies.
- **Replaced** (2026-10-08): the frozen-tail score (each improvement credited as the last: builders overrated
  10-40x); the single-step trainee score (a strong uma's career got no credit as a parent for her partners); C priced
  from hypothetical parents bred from each other (a full loop one generation deeper than the goal, real 8/8s ignored,
  so the page forced goal steps 2 and 3).

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
`docs/RESEARCH.md` §12. On the planner side, the continuation (`Cont`) is a calibration on four of
the owner's pools.
