# Model assumptions and the evidence behind each

The full mechanics (algorithms, job fields, deck model) are in `pkg/loopopt/README.md`. This file lists every
assumption the odds rest on, where it came from, and how sure we are. "Measured" means fitted to real careers;
"community" means a widely repeated figure without published data; "assumed" means ours.

| Assumption | Value in the model | Source | Status |
|---|---|---|---|
| Relation points | `pair`/`tri` = sum of `succession_relation.relation_point` over groups containing all members | master.mdb | exact (game data) |
| Race affinity | +3 per shared single-race G1 win, by win-saddle group (relocated venues = same race, 1025→1012, 1027→1006, 1028→1005); multi-race titles count 0 | uma-it-optimizer `affinity.py`; uma.moe matched exactly (umadump test, 2026-10-02) | confirmed by two tools; titles-count-0 shown by uma.moe |
| Per-legacy affinity | parent P: pair(T,P) + pair(P1,P2) + Σ over P's own parents G [trio(T,P,G) + 3·shared(P,G)] + 3·shared(P,other parent); grandparent G under P: trio(T,P,G) + 3·shared(P,G). In a loop the other parent is also P's own parent, so the parents' shared G1s count twice | uma-it-optimizer fit to 8,155 IT receipts (fired vs predicted within 0.1% parents, 2% grandparents); uma.moe gives identical numbers | measured; uma.moe agreement is not independent (shared lineage of formula) |
| Inheritance chance | per spark per legacy per roll: base × (1 + affinity/100), capped 1; two rolls per career (Classic and Senior April); grandparents full rate (no halving) | herohero note (1,000+ trials), umareference/crazyfellow, uma-it-optimizer fit; uma.moe identical | measured / consistent |
| Base by spark type and stars | skill/scenario white 3/6/9%, race white 1/2/3%, green 5/10/15%, pink 1/3/5%, blue 70/80/90% | same; the model uses only skill whites | consistent across all sources |
| Star mix of legacy sparks | 50/45/5% (1/2/3★) for rank score 6,500–17,499 (default); 20/70/10% at 17,500+ | hakuraku.moe CM10–12 room data: 528,637 and 196,646 white sparks, fits p = 0.13 and 0.77 | measured. The old 45/50/5 matched neither band (fixed in v21) |
| Every target spark on all six legacies | yes (a running loop keeps veterans that landed everything) | assumed | simplification; a loop being built has fewer |
| Spark generation | learned white 20%, gold 40% (double-circle 25%), ×1.1 per ancestor carrying the spark; loop uses n = 6: 35.43% / 70.86%. The spark is always the group's white (a bought gold sparks its white) | aoneko_pochi 2024; hakuraku fit on ~1.2M observations (p ≥ 0.84 per tier); uma.moe uses the same curve; master.mdb has spark factors for whites only | measured. Additive "+2.5 points per carrier" rejected (p ≈ 1e-14) |
| Great-grandparents | no effect on affinity or carrier counts | uma.moe test file B (2026-10-02) | uma.moe behaviour; matches the game's two-generation lineage |
| Year-1 factor gains in receipts | start-of-career application (all blues + unique sparks), not a third skill inheritance | one uma-it run (Special Week, 2026-10-02); uma-it-optimizer counts years 2 and 3 only | observed once |
| Win chance in IT | 110 − cost(distance) − cost(surface) − streak penalty, clamped; cost S/A 0, B 10, C 20, D 30, E 40, F 60, G 90; penalty by races in a row 0,0,5,20,30,50 | community table (`inputs/user/reference_tables/`) | community |
| Training hints | 1 + hint_frequency/25 per run, uniform over the card's hint options | assumed | uma-it-optimizer measured ≈ 3.04 + 0.038·hf hints in Unity Cup; not adopted |
| Chain events | Poisson(λ = 7.1) per run, each advancing a random remaining chain slot of a non-pal card; split outcomes 50% | assumed | uma-it-optimizer measured Unity SSR final-link completion ≈ 24.5% and pal multipliers ×0.22/0.69/1.06/1.69; not adopted |
| Pal first-year dates | don't fire in Unity Cup IT (Tazuna: Tail Held High; Anshinzawa: Uma Stan; SSR and R); a page option re-enables them | owner | owner-stated |
| Pal / group card events | hand-entered, each with a source, audited by `val/event_audit.py` against GameTora | GameTora caches + owner + umamusu.wiki | audited 2026-10-03, all target hints accounted for |
| Character rule | no trainee runs a card of her character or a group card she belongs to | owner | game rule |
| Per-uma decks | each trainee uses her own best legal deck (default on) | owner request | exact per model; brute-force checked |
| Hint level from inheritance | unknown rule (one 2★ proc of Ramp Up showed Lv 5 on the uma-it page) | one run | open |

## Where the model is weakest

The pieces that decide rankings, and have the least evidence, are the deck-side ones: training-hint counts, chain
completion, and event-choice behaviour. Everything on the inheritance side is now backed by large datasets.
uma-it receipts (per run: hints with sources and levels, sparks fired per year, all six ancestors) are the data
that would test the deck side; see `docs/HANDOFF.md`.
