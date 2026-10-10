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
| Per-legacy affinity | parent P: pair(T,P) + pair(P1,P2) + Σ over P's own parents G [trio(T,P,G) + 3·shared(P,G)] + 3·shared(P,other parent); grandparent G under P: trio(T,P,G) + 3·shared(P,G). In a loop the other parent is also P's own parent, so the parents' shared G1s count twice | uma-it-optimizer fit to 8,155 IT receipts (fired vs predicted within 0.1% parents, 2% grandparents); uma.moe gives identical numbers (the best Unity loop's 12 veterans, written by `val/umadump_lineage.py` and loaded into uma.moe: lineage total 452 = 184 + 97 + 171) | measured; uma.moe agreement shows our implementation matches theirs, not that the formula matches the game. umaishow's split (grandparents halved) puts every spark chance at about half the measured level |
| Inheritance chance | per spark per legacy per roll: base × (1 + affinity/100), capped 1; two rolls per career (Classic and Senior April); grandparents full rate (no halving) | herohero note (1,000+ trials), umareference/crazyfellow, uma-it-optimizer fit; uma.moe identical | measured / consistent |
| Base by spark type and stars | skill/scenario white 3/6/9%, race white 1/2/3%, green 5/10/15%, pink 1/3/5%, blue 70/80/90% | same; the model uses only skill whites | consistent across all sources |
| Star mix of legacy sparks | 50/45/5% (1/2/3★) for rank score 6,500–17,499 (default); 20/70/10% at 17,500+ (90/10/0% below 6,500). Applies to skill (white) and unique (green) sparks; pink stars are fixed at 20/70/10 and blue stars follow the stat's value (hakuraku note, its addendum on ~22M team-trial umas for green). The page defaults the band by scenario: SS for Grand Concert and Grand Masters, below SS for URA Finale, Unity Cup and Trackblazer | hakuraku.moe CM10–12 room data: 528,637 and 196,646 white sparks, fits p = 0.13 and 0.77. Scenario defaults from uma-it summaries (score if every skill point is spent): URA 0% reach 17,500, Unity 47% (median 17,428), Grand Concert 79% (median 18,094), Trackblazer 0% (median 12,386, 282 runs); Grand Masters has no IT data, so SS is assumed | measured |
| Spark generation | learned white 20%, gold 40% (double-circle 25%), ×1.1 per ancestor carrying the spark; loop uses n = 6: 35.43% / 70.86%. The spark is always the group's white (a bought gold sparks its white) | aoneko_pochi 2024; hakuraku fit on ~1.2M observations (p ≥ 0.84 per tier); uma.moe uses the same curve; master.mdb has spark factors for whites only | measured (the hakuraku data can't separate 0.35 from 0.354 at six carriers). Additive "+2.5 points per carrier" rejected (p ≈ 1e-14) |
| Every target spark on all six legacies | yes (a running loop keeps veterans that landed everything) | assumed | simplification; a loop being built has fewer (the career planner uses real carriers) |
| Great-grandparents | no effect on affinity or carrier counts | uma.moe on `val/umadump_carriers.py`'s second file (`OUT_B`: great-grandparents carrying Nimble Navigator), 2026-10-02 | uma.moe behaviour; matches the game's two-generation lineage |
| Year-1 factor gains in receipts | start-of-career application (all blues + unique sparks), not a third skill inheritance | one uma-it run (Special Week, 2026-10-02); uma-it-optimizer counts years 2 and 3 only | observed once |
| Hint level from inheritance | unknown rule (one 2★ proc of Ramp Up showed Lv 5 in a uma-it run) | one run | open; matters only if the model ever uses hint levels |
| Loop structure and ranking | four-uma loops (12 lineages: trainee, two of the other three as parents, each parent bred from the other parent and the fourth uma) and three-uma rotations (3 lineages: each parent bred from the other parent and the trainee's own earlier veteran), ranked together on the mean P(all) over their lineages, i.e. every member trains equally often | a modelling choice (2026-10-07); the rotation occurs in uma-it runs (`docs/RESEARCH.md` §5) and in GameTora's legacy guide ("she can be her own sub-legacy") | the equal-training average is a choice, not a fit: it is the long-run rate of a player who trains every member in turn. A player who keeps re-training only the best trainee would score a structure by its best lineage instead (not offered) |
| Repeated character in a trio | tri(T, P, T) = 0: a relation group counts only for distinct characters (`rel_tables`), so a self-grandparent brings only her shared G1s with her child | fitted on uma-it inspiration trials (`docs/RESEARCH.md` §4, *Self-grandparent*); uma.moe's backend uses 0, uma-it-optimizer counts pair(T, P) | measured (a rate fit; the game's code path is not seen) |

## Races

| Assumption | Value in the model | Source | Status |
|---|---|---|---|
| Win chance in IT | 110 − cost(distance) − cost(surface) − streak penalty, clamped; cost S/A 0, B 10, C 20, D 30, E 40, F 60, G 90; penalty by races in a row 0,0,5,20,30,50 | community table | community |
| Turns a career forbids racing | `single_mode_race_restrict_turn` (chara, turn, gain_id = the story event that sets it, e.g. 107811811 = story 501078118): that turn keeps only her goal races. Global: Yamanin Zephyr turn 44 (no Tenno Sho (Autumn), Shuka Sho or Kikuka Sho in Classic), Sweep Tosho turn 36 (no Classic Takarazuka Kinen), both fixed story events with +20 energy; Matikanetannhauser turn 70 = her event-assigned Japan Cup goal | master.mdb; event titles from `text_data` 181 | exact (game data); that the Zephyr and Sweep events always fire is read from their being fixed story events, not tested |
| Loop schedules | chosen to maximise the total expected shared G1 wins (every pair alike), not the landing odds | tested 2026-10-03: re-optimising every solved loop for the landing odds directly (gradient-weighted best responses, ascent, group moves, 30 ILS rounds) gained at most +0.12% on any loop, +0.065% on a winner, and changed no top-1 or top-3 loop over 46 validation runs; the search's ceiling (each pair at its best overlap) is only 0.56-2.83% above the found odds | measured; kept |

## Deck side

| Assumption | Value in the model | Source | Status |
|---|---|---|---|
| Training hints | h = base(scenario) × (1 + hint_frequency/100) draws per run (hint frequency at max limit break, the unique effect's included), each uniform over the card's skill options still below hint Lv 5 (the stat option is never drawn; a skill leaves after ceil(5 / levels per draw) draws; the cap applies to training hints only: events still hint a skill already at Lv 5, a player observation); base Unity Cup 2.70, Grand Concert 2.03, URA Finale 1.60, Trackblazer 1.81, Grand Masters 2.30 (placeholder, no runs); h read as floor or ceiling, weighted by its fraction; one card's draws over several targets are joint, so it hints two of them together less often than independent odds | `docs/RESEARCH.md` §6: Independent Training runs from uma-it, bases refit 2026-10-05 on 2,402 runs; skills-only, the near-fixed draw count, uniform draws with the Lv-5 cap and the joint formula all checked there | measured. Caveats: a given setup can be off by about half a draw (IT's count looks rounded per setup); Unity's hf slope rests on three cards (hf-0, specialty-0 cards over-rated about 10% in Unity); no R cards and hint frequency only 0–60 in the data; a specialty-priority term (×exp(0.025 per 10 points)) is a candidate, not adopted |
| Chain events | Binomial(21, 0.35) per run (mean 7.35, variance 4.8), each advancing a random remaining chain slot of a non-pal card, weighted by links left; pals and R cards take no slot; IT's default choice ending a chain on k of n outcomes stops it there with chance k/n (exact mixture) | `docs/RESEARCH.md` §7: fitted 2026-10-05 on uma-it IT runs whose chain progress is readable from event hints; distribution-free fit mean 7.30, variance 5.1; Binomial(21, 0.35) 6.6 log-likelihood units behind it with 14 fewer parameters, Poisson(7.1) 27 behind; no difference between scenarios (Unity 7.56, Grand Concert 7.16, URA 6.53; per-scenario fit gains 15.7 for 32 parameters); refit on 1,792 runs keeps it | measured; the binomial is kept (14 fewer parameters than the free fit) until more data justify more. Known limits: two-chain decks finish both chains in 82% of runs against 75% predicted; decks with 4–5 chain cards have about a third of chains never start (model ~16%) while final-link completion matches; the slot rule (over the card rule) is an empirical preference |
| Event outcomes and choices | an event's outcomes exclude each other (one random outcome per firing; equally likely unless measured, below); where an event offers targets on different choices, a deck entry takes one choice (the solver offers one entry per choice worth taking, like an early chain end), shown as the skill to prioritise; a card's gold and white of one skill are combined exactly | GameTora event structure (`alts` in card_events.json, `export/event_choices.py`) | follows from the event data. Crediting each skill of an event on its own would overstate the odds: Nakayama Festa [Trusting the Dice]'s Nothing Ventured / Risky Business would give any hint in 75% of the link's firings instead of 100%, and King Halo [Tonight, We Waltz]'s Stop Right There! and Uma Stan, on separate choices, would both count |
| Event outcome shares | equally likely outcomes unless measured; four measured shares adopted at 0.75: Anshinzawa SSR finale (830080001; it always fires, and with Risky Business prioritised IT takes choice 2: Nothing Ventured +1 on success, Risky Business +1 on failure), Riko Kashimoto SSR's "Unexpected Side" Ramp Up, Seiun Sky [Foolproof Plan] link 3 Fast-Paced, Smart Falcon [My Umadol Way! ☆] link 3 Center Stage (`chance` for the pal, `event_shares.json` for generated chain events, turned by the builders into chain shares and `alt_w` outcome weights) | `docs/RESEARCH.md` §8 (uma-it runs, 2026-10-06; measurements and the shares left at 0.5) | measured |
| Joint landing odds | the targets do not land independently: chain cards draw on one run's chain events (one binomial count per run), so their rewards land or miss together, and one event's outcomes exclude each other. Odds shown and the ranking are exact over the chain-progress distribution and those outcomes; training hints stay independent per target. The search prunes with the product of per-target odds times an assumed cap per deck structure class (`docs/SOLVER.md`, *Exact odds*) | derived from the chain and event model above; the caps assumed from measured exact/product ratios | exact for the model under the stated caps, relative to the schedules found; the ratios seen, the cap checks and why no proven cap is used: `docs/SOLVER.md` |
| IT event choices | top choice, unless a prioritized skill (up to 10, chosen at career start) sits on another choice; the model assumes priorities reach the hints it counts | player practice; a uma-it run shows a "Prioritized skills" list | stated; incomplete in the data (with neither skill prioritised, 20 of 72 Anshinzawa runs still took her choice 2) |
| Random card events (a deck card's character's shared events) | not modelled | `docs/RESEARCH.md` §9: fire in about 15% of careers (21% with 4 random events on offer, 13% with 14) | tested: crediting them at 0.17 raised odds by at most 1.7% and changed no best loop; not added (within the model's other uncertainties, and Seeking the Pearl's Uma Stan sits on a bottom choice with no readings) |
| Conditional character events (secret and race-result events) | not modelled, except Tokai Teio's Head-On (her Triple Crown event; all three races are career goals), counted as always acquired (job `extra`, as Yamanin Zephyr's Slipstream) | `uv run val/secret_events.py` (GameTora `training_events_char`; writes the full list to `cache/secret_events.md`): 81 umas, about 170 events with hints; 14 touch the default targets (Nimble Navigator, Ramp Up, Tail Held High), all as white hints | checked 2026-10-03: forcing every target-relevant event to fire with certainty, even as a gold, leaves the best loop unchanged on the default, GM-era and a player's own setups, with or without the Unity golds, so they are not modelled. Recheck with that forced-certain test (an `extra` entry per source) if a new target is mainly supplied by such events (Head-On is the likely case) |
| Pal first-year dates | don't fire in Unity Cup IT (Tazuna: Tail Held High; Anshinzawa: Uma Stan; SSR and R); always left out (the data keeps them disabled with a reason) | player observation | stated, not measured |
| Pal / group card events | hand-entered, each with a source, audited by `val/event_audit.py` against GameTora; pal finales certain | GameTora caches, player observation, umamusu.wiki; certainty checks in `docs/RESEARCH.md` §8 (Light Hello [From the Ground Up] credits See Ya Later! to her event in every run carrying her) | audited 2026-10-03, all target hints accounted for. uma-it-optimizer's "pal events only fire in the pal's own scenario" was not adopted: it rests on one card (SR Aoi Kiryuin) and contradicts player experience |
| Character rule | no trainee runs a card of her character or a group card she belongs to | the game's deck rule | game rule |
| Per-uma decks | each trainee uses her own best legal deck (default on) | design choice | exact per model; brute-force checked (`val/owncheck.py`, `val/permcheck.py`) |

## Career planner (`/career`)

| Assumption | Value in the model | Source | Status |
|---|---|---|---|
| Veterans | the loop model with what real veterans carry: each legacy's stars per target (no spark: no chance), recorded single-race G1 wins for race affinity, the recorded parents as grandparents (unknown parent: no relation points, sparks still count); spark generation ×1.1 per legacy actually carrying the spark | umadump export (trained_chara_data.json): factor_id = factor group × 100 + stars, saddles via `single_mode_wins_saddle.group_id` | exact decoding (master.mdb); borrowed flag (owner_viewer_id) unverified on a real export |
| Career schedule | the trainee's schedule maximizes expected distinct G1 wins, a race a parent won counting double (affinity later); no training cost, as in the loop planner | derived | |
| Next career (the planner's pick, `next`) | the career that brings a fully looped parent (an 8/8 whose parents are 8/8s) closest per run, with each candidate's continuation (`Cont`: how her own odds rise as builder careers improve her lineage) priced in; mechanics in `docs/SOLVER.md`, *Next career* | continuation form assumed, fitted per pool; shape fitted to simulated builder traces | section *Next career* below |
| Compare and Reroll (*Veteran analysis*) | Compare: the planner's expected careers to a fully looped parent (V) for the pool now against a full plan of the pool with her added, fitted to that pool (the plan Add to veterans then shows); her own 8/8 with 8/8 parents is a fully looped parent. Reroll reads the plan's per-outcome table (`next.outcomes`), proven "no saving" where its bound `y_hi` is under half a career, else checked by a Compare | exact parts as the planner's | the two Vs come from two fits (Cont is refitted per pool), so a veteran can raise V; the page shows a rise as no change. Monotonicity is checked by `val/mono.py`, not assumed |
| Aptitude (pink), blue and unique sparks | section *Sparks the odds don't use* | | |
| Look ahead, rollout, policy comparison, `plan` simulator | solver-only (`lookahead`, `rollout`, `policy`, `plan` job fields; the page sends only `next`): described in `docs/SOLVER.md`. `plan`'s surrogate measured: its greedy pick equalled the exact search's at 29 of 29 pick changes (career_U5 and `val/policy/pools/pool4.json`, 4 × 25 careers) | derived | kept for reverting and for the simulator |

### Next career

The mechanics (objective V, `Cont`, a career's score, builders through their deck class) are in `docs/SOLVER.md`,
*Next career*. The evidence behind them:

- **Cont's shape.** `CEIL_SHARE` 0.6 and `LAM_DECAY` 1.5 are fitted to the measured builder traces of
  `val/sim/README.md` E5 (four player pools of 2, 5, 11 and 19 veterans): Cont tracks achievable building within 0-5% on
  2, 11 and 19 veterans and is 70% too pessimistic on 5, where lam rises after the first lift (builders improve) while
  Cont only lets it decay; leave-one-pool-out fits of the shape ranged 0.6-1.0 and 1-3, so it is a calibration on
  these pools, not a law. The continuation's ceiling for Agnes Digital is about 1 in 165-180 (a second computation's
  1 in 166 is the same calculation, not independent). Not yet measured against other policies.
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
`docs/RESEARCH.md` §12. On the planner side, the continuation (`Cont`) is a calibration on four player pools.
