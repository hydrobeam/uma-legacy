# Outside sources, tools and repos

## Game data

| What | Where | Notes |
|---|---|---|
| Global `master.mdb` | the game client (Global) | not tracked by git |
| JP-only art | the JP client's bundles (found by name in the JP asset index of 2025-10-09, the `meta` file of https://github.com/SimpleSandman/UmaMusumeMetaMasterMDB), read by `webapp/images.py` from `cache/jp_bundles/` when present (not committed) | what the Global client lacks: the Grand Masters scenario's logo and character art (`single/banner/bnr_005_text`, `bnr_chara_big_005`; its logo in Japanese) and 14 JP-only skill icons |
| JP `master.mdb` | https://github.com/SimpleSandman/UmaMusumeMetaMasterMDB (`master/master.mdb`; commit b5f50b5 of 2026-09-30, but that commit only retired the repo: the data is from its last update, 2025-10-09) | placed at `jp/export/master.mdb`; not tracked by git. The GM-era export cuts by release date (L'Arc launch, 2023-08-24). Older mirrors: SimpleSandman/UmaMusumeMasterMDB (2022), ChihyuenWang/Umamusume-MasterMDB (2024) |
| GameTora training-event files | downloaded by `./build.sh gametora` into `cache/gametora/` (not committed; `dict_evrew`, `dict_te_pairs_en`, `skills`, `training_events_{sr,ssr,friend,group,scenario,shared,char_card,char}`, `common_char_events`, `support-cards` for each card's page `url_name`; current names from https://gametora.com/data/manifests/umamusume.json) | the only source of event rewards. Reward code k → `dict_evrew[k-36]`. Choice = `[label, rewards]`. Titles: master.mdb `text_data` category 181 by story id. In `training_events_char` a character row is [id, meta, story events, date events, conditional events (secret and race-result events; each has a [0, conditions] entry and older versions under [9, ...]), localisation]; `val/secret_events.py` decodes the conditional ones into `docs/SECRET_EVENTS.md` |

## Models we adopted or checked against

| Source | URL | What it gave us |
|---|---|---|
| uma-it-optimizer (Acrith) | https://github.com/Acrith/uma-it-optimizer (`tools/loop/affinity.py`, `inspiration_rates.py`, `loopdata.py`, `loop_README.md`, `companion-plan.md`) | per-legacy affinity model (adopted v19), grandparents at full rate, spark-generation curve, measured hint and chain rates (not adopted). Fits on IT receipts |
| hakuraku.moe research note | https://hakuraku.moe/notes/spark_generation | spark generation ×1.1ⁿ fit, white-spark star mix by rank score, blue/pink checks; data: ~107k CM10–12 room-match characters |
| umamusustation red factor analysis | https://umamusustation.com/factor/red_factor_analysis.html | pink (red) spark generation on 22,411 team-trial horses: a pink needs aptitude A or better; A 90.36% / S 9.64% of selections, S not favoured; stars 20/70/10 at A and S alike |
| GameTora legacies | https://gametora.com/umamusume/legacies | every veteran: exactly one blue spark (stat random) and one pink; a unique (green) spark guaranteed when the outfit is at 3★ or more at the career's start |
| umamusu.wiki Inheritance | https://umamusu.wiki/Game:Inheritance | pink sparks at the career start: the six legacies' stars summed per aptitude, 1/4/7/10★ raise 1/2/3/4 grades, at most A; at the Classic and Senior April inspirations a firing can take A to S (community) |
| uma.guide sparks | https://uma.guide/guides/sparks | pink: an April inspiration firing raises the aptitude one grade regardless of stars; the start rule as above (community) |
| uma.moe lineage planner | https://uma.moe | identical affinity, per-roll and per-career odds, generation curve, GGPs ignored, "Optimal Races" = union of parents' G1 wins |
| umadump (Werseter) | https://github.com/Werseter/umadump ; site https://werseter.github.io/umadump/ | game-memory reader writing `trained_chara_data.json` (veterans; positions 10/20 parents, 11/12/21/22 grandparents) used by uma.moe and others. `val/umadump_lineage.py` writes this format |
| uma-it / UmaLadder IT dashboard | https://training.umaladder.moe | Independent Training runs from uma-it (19,536 summarised, 3,179 in detail): the measured training-hint rate, chain-event count, event-outcome shares, random-event rates and the end-to-end calibration. Statistics are pooled aggregates, never per run and never by who ran the career |
| umaishow | https://github.com/mee1080/umaishow | the earlier affinity split (replaced in v19) |
| JP research notes | silvervine0822.hatenablog.com (白因子確率), owadablog.com/white-factor/, note.com/toijuichi3 (いぬかもめ), note.com/herohero_3 (へろへろ, 1,000+ trial inheritance rates) | community figures for generation and inheritance; superseded by hakuraku's data where they conflict |
| GameTora legacies guide | https://gametora.com/umamusume/legacies | base generation rates, star mix by rank, blue stars by stat |
| umareference | https://www.umareference.com/guide/legacies/chance-of-inheriting-sparks | inheritance base table (crazyfellow's guide); its "grandparents halved" is contradicted by receipt fits |
| crazyfellow's parenting & gene guide | https://docs.google.com/document/d/1Q3IJKbtkplmuY-PAJMNjYiLtasv0eU0aIBEqp8_C3tg/ | the community baseline hakuraku tested |
| umamusu.wiki | (wiki) | some hand-entered pal facts (`src` fields name it) |

## Tools in this repo that talk to those sources

- `val/umadump_lineage.py LOOP_OUT OUT.json OUT.md`: a loop's 12 veterans in umadump format plus our numbers for them.
- `val/umadump_carriers.py`: test veterans with 1–6 carriers per spark (reads a lineage file from the above).
- `val/spark_gen_fit.py trained_chara_data.json`: measures spark generation and star mix from a real veteran dump.
- `val/event_audit.py`: hand-entered pal/group entries vs GameTora.
