# Outside sources, tools and repos

## Game data

| What | Where | Notes |
|---|---|---|
| Global `master.mdb` | the owner's game install (local data folder) | shipped in the handoff zip at `pkg/loopopt/export/master.mdb` (copy of 2026-10-01); not tracked by git |
| JP `master.mdb` | https://github.com/SimpleSandman/UmaMusumeMetaMasterMDB (`master/master.mdb`; commit b5f50b5 of 2026-09-30, but that commit only retired the repo: the data is from its last update, 2025-10-09. Its `meta` is older still: it stopped when JP encrypted `meta` in Sept 2025) | shipped in the handoff zip at `jp/export/master.mdb`; not tracked by git. The GM-era export cuts by release date (L'Arc launch, 2023-08-24). Older mirrors: SimpleSandman/UmaMusumeMasterMDB (2022), ChihyuenWang/Umamusume-MasterMDB (2024) |
| GameTora training-event files | `pkg/loopopt/gametora/` (cached; `dict_evrew`, `dict_te_pairs_en`, `skills`, `training_events_{sr,ssr,friend,group,scenario,shared,char_card}`, `support-cards` for each card's page `url_name`; current hashes in https://gametora.com/data/manifests/umamusume.json) | the only source of event rewards. Reward code k → `dict_evrew[k-36]`. Choice = `[label, rewards]`. Titles: master.mdb `text_data` category 181 by story id |

## Models we adopted or checked against

| Source | URL | What it gave us |
|---|---|---|
| uma-it-optimizer (Acrith) | https://github.com/Acrith/uma-it-optimizer (`tools/loop/affinity.py`, `inspiration_rates.py`, `loopdata.py`, `loop_README.md`, `companion-plan.md`) | per-legacy affinity model (adopted v19), grandparents at full rate, spark-generation curve, measured hint and chain rates (not adopted). Fits on IT receipts pulled from the uma-it site |
| umadump (Werseter) | https://github.com/Werseter/umadump ; site https://werseter.github.io/umadump/ | game-memory reader writing `trained_chara_data.json` (veterans; positions 10/20 parents, 11/12/21/22 grandparents) used by uma.moe and others. `val/umadump_lineage.py` writes this format |
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
