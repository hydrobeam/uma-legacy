# Model check: Oguri, Digital, Tamamo, Inari (veterans in U_trained_chara_data.json)

Wins (chance >= 50% counts as won), single-race G1 saddles / title saddles:

- Oguri (100601): 23 G1 saddles [10, 11, 13, 14, 15, 17, 18, 19, 21, 22, 23, 25, 26, 27, 28, 29, 30, 34, 35, 36, 38, 165, 168]; titles [2, 5, 6, 9]
- Digital (101901): 22 G1 saddles [11, 14, 15, 17, 18, 19, 21, 22, 23, 25, 26, 27, 28, 29, 30, 32, 34, 35, 36, 38, 165, 168]; titles [9]
- Tamamo (102101): 18 G1 saddles [10, 11, 13, 14, 15, 18, 19, 21, 23, 25, 26, 27, 28, 32, 34, 35, 38, 165]; titles [5, 6, 7]
- Inari (103401): 23 G1 saddles [10, 11, 13, 14, 15, 17, 18, 19, 21, 22, 23, 25, 26, 27, 28, 29, 30, 32, 34, 35, 38, 165, 168]; titles [4, 5, 6, 7, 9]

Shared G1s by pair (our rule: single-race G1 saddles by group) and every shared saddle id:

- Oguri-Digital: pair 30, shared G1 21, all saddles 22
- Oguri-Tamamo: pair 28, shared G1 17, all saddles 19
- Oguri-Inari: pair 26, shared G1 22, all saddles 25
- Digital-Tamamo: pair 21, shared G1 16, all saddles 16
- Digital-Inari: pair 28, shared G1 21, all saddles 22
- Tamamo-Inari: pair 43, shared G1 18, all saddles 21

Per legacy: affinity = relation points + race points; spark chance per inspiration = base x (1 + aff/100);
q = chance a given target fires from at least one of the 6 legacies over 2 inspirations.

## Trainee Oguri; parents veteran #9 (Tamamo <- Inari + Digital) and #12 (Inari <- Tamamo + Digital)

Optimizer's own figure for this lineage (win chances instead of wins, star mix 45/50/5, base 0.048): q = 0.7612; parents' affinity 247.9 / 265.5. Same formula at base 0.048 on these veterans: q = 0.7759.
Displayed lineage total (game / calculators): 452 (double circle) under the G1 rule, 482 under the all-saddle rule.

| legacy | relation | race (G1 rule) | affinity | race (all-saddle rule) | affinity |
|---|---|---|---|---|---|
| Tamamo | 112 | 156 | 268 | 174 | 286 |
| gp Inari under Tamamo | 23 | 54 | 77 | 63 | 86 |
| gp Digital under Tamamo | 18 | 48 | 66 | 48 | 66 |
| Inari | 110 | 171 | 281 | 192 | 302 |
| gp Tamamo under Inari | 23 | 54 | 77 | 63 | 86 |
| gp Digital under Inari | 18 | 63 | 81 | 66 | 84 |

| spark | stars | per-inspiration chance, parents (G1 rule) | q (G1 rule) | q (all-saddle rule) |
|---|---|---|---|---|
| Uma Stan | 2 | 0.2208 / 0.2286 | 0.8515 | 0.8641 |
| No Stopping Me! | 2 | 0.2208 / 0.2286 | 0.8515 | 0.8641 |
| Playtime's Over! | 2 | 0.2208 / 0.2286 | 0.8515 | 0.8641 |
| It's On! | 2 | 0.2208 / 0.2286 | 0.8515 | 0.8641 |
| Tail Held High | 1 | 0.1104 / 0.1143 | 0.5970 | 0.6127 |
| Racing Spirit: Stamina | 3 | 0.3312 / 0.3429 | 0.9511 | 0.9580 |
| Slipstream | 2 | 0.2208 / 0.2286 | 0.8515 | 0.8641 |
| Risky Business | 2 | 0.2208 / 0.2286 | 0.8515 | 0.8641 |

## Trainee Oguri; parents veteran #6 (Digital <- Inari + Tamamo) and #12 (Inari <- Digital + Tamamo)

Optimizer's own figure for this lineage (win chances instead of wins, star mix 45/50/5, base 0.048): q = 0.7673; parents' affinity 260.5 / 265.8. Same formula at base 0.048 on these veterans: q = 0.7753.
Displayed lineage total (game / calculators): 452 (double circle) under the G1 rule, 470 under the all-saddle rule.

| legacy | relation | race (G1 rule) | affinity | race (all-saddle rule) | affinity |
|---|---|---|---|---|---|
| Digital | 94 | 174 | 268 | 180 | 274 |
| gp Inari under Digital | 18 | 63 | 81 | 66 | 84 |
| gp Tamamo under Digital | 18 | 48 | 66 | 48 | 66 |
| Inari | 95 | 180 | 275 | 195 | 290 |
| gp Digital under Inari | 18 | 63 | 81 | 66 | 84 |
| gp Tamamo under Inari | 23 | 54 | 77 | 63 | 86 |

| spark | stars | per-inspiration chance, parents (G1 rule) | q (G1 rule) | q (all-saddle rule) |
|---|---|---|---|---|
| Uma Stan | 2 | 0.2208 / 0.2250 | 0.8509 | 0.8586 |
| No Stopping Me! | 2 | 0.2208 / 0.2250 | 0.8509 | 0.8586 |
| Playtime's Over! | 2 | 0.2208 / 0.2250 | 0.8509 | 0.8586 |
| It's On! | 2 | 0.2208 / 0.2250 | 0.8509 | 0.8586 |
| Tail Held High | 1 | 0.1104 / 0.1125 | 0.5964 | 0.6058 |
| Racing Spirit: Stamina | 3 | 0.3312 / 0.3375 | 0.9507 | 0.9549 |
| Slipstream | 2 | 0.2208 / 0.2250 | 0.8509 | 0.8586 |
| Risky Business | 2 | 0.2208 / 0.2250 | 0.8509 | 0.8586 |

## Trainee Oguri; parents veteran #6 (Digital <- Tamamo + Inari) and #9 (Tamamo <- Digital + Inari)

Optimizer's own figure for this lineage (win chances instead of wins, star mix 45/50/5, base 0.048): q = 0.7424; parents' affinity 234.0 / 221.7. Same formula at base 0.048 on these veterans: q = 0.7561.
Displayed lineage total (game / calculators): 417 (double circle) under the G1 rule, 429 under the all-saddle rule.

| legacy | relation | race (G1 rule) | affinity | race (all-saddle rule) | affinity |
|---|---|---|---|---|---|
| Digital | 87 | 159 | 246 | 162 | 249 |
| gp Tamamo under Digital | 18 | 48 | 66 | 48 | 66 |
| gp Inari under Digital | 18 | 63 | 81 | 66 | 84 |
| Tamamo | 90 | 150 | 240 | 159 | 249 |
| gp Digital under Tamamo | 18 | 48 | 66 | 48 | 66 |
| gp Inari under Tamamo | 23 | 54 | 77 | 63 | 86 |

| spark | stars | per-inspiration chance, parents (G1 rule) | q (G1 rule) | q (all-saddle rule) |
|---|---|---|---|---|
| Uma Stan | 2 | 0.2076 / 0.2040 | 0.8340 | 0.8396 |
| No Stopping Me! | 2 | 0.2076 / 0.2040 | 0.8340 | 0.8396 |
| Playtime's Over! | 2 | 0.2076 / 0.2040 | 0.8340 | 0.8396 |
| It's On! | 2 | 0.2076 / 0.2040 | 0.8340 | 0.8396 |
| Tail Held High | 1 | 0.1038 / 0.1020 | 0.5766 | 0.5832 |
| Racing Spirit: Stamina | 3 | 0.3114 / 0.3060 | 0.9408 | 0.9441 |
| Slipstream | 2 | 0.2076 / 0.2040 | 0.8340 | 0.8396 |
| Risky Business | 2 | 0.2076 / 0.2040 | 0.8340 | 0.8396 |

## Trainee Digital; parents veteran #8 (Tamamo <- Inari + Oguri) and #11 (Inari <- Tamamo + Oguri)

Optimizer's own figure for this lineage (win chances instead of wins, star mix 45/50/5, base 0.048): q = 0.7575; parents' affinity 239.4 / 264.5. Same formula at base 0.048 on these veterans: q = 0.7726.
Displayed lineage total (game / calculators): 443 (double circle) under the G1 rule, 485 under the all-saddle rule.

| legacy | relation | race (G1 rule) | affinity | race (all-saddle rule) | affinity |
|---|---|---|---|---|---|
| Tamamo | 100 | 159 | 259 | 183 | 283 |
| gp Inari under Tamamo | 18 | 54 | 72 | 63 | 81 |
| gp Oguri under Tamamo | 18 | 51 | 69 | 57 | 75 |
| Inari | 107 | 174 | 281 | 201 | 308 |
| gp Tamamo under Inari | 18 | 54 | 72 | 63 | 81 |
| gp Oguri under Inari | 18 | 66 | 84 | 75 | 93 |

| spark | stars | per-inspiration chance, parents (G1 rule) | q (G1 rule) | q (all-saddle rule) |
|---|---|---|---|---|
| Uma Stan | 2 | 0.2154 / 0.2286 | 0.8486 | 0.8662 |
| No Stopping Me! | 2 | 0.2154 / 0.2286 | 0.8486 | 0.8662 |
| Playtime's Over! | 2 | 0.2154 / 0.2286 | 0.8486 | 0.8662 |
| It's On! | 2 | 0.2154 / 0.2286 | 0.8486 | 0.8662 |
| Tail Held High | 1 | 0.1077 / 0.1143 | 0.5935 | 0.6155 |
| Racing Spirit: Stamina | 3 | 0.3231 / 0.3429 | 0.9495 | 0.9591 |
| Slipstream | 2 | 0.2154 / 0.2286 | 0.8486 | 0.8662 |
| Risky Business | 2 | 0.2154 / 0.2286 | 0.8486 | 0.8662 |

## Trainee Digital; parents veteran #3 (Oguri <- Inari + Tamamo) and #11 (Inari <- Oguri + Tamamo)

Optimizer's own figure for this lineage (win chances instead of wins, star mix 45/50/5, base 0.048): q = 0.7692; parents' affinity 266.1 / 264.9. Same formula at base 0.048 on these veterans: q = 0.7783.
Displayed lineage total (game / calculators): 459 (double circle) under the G1 rule, 501 under the all-saddle rule.

| legacy | relation | race (G1 rule) | affinity | race (all-saddle rule) | affinity |
|---|---|---|---|---|---|
| Oguri | 92 | 183 | 275 | 207 | 299 |
| gp Inari under Oguri | 18 | 66 | 84 | 75 | 93 |
| gp Tamamo under Oguri | 18 | 51 | 69 | 57 | 75 |
| Inari | 90 | 186 | 276 | 213 | 303 |
| gp Oguri under Inari | 18 | 66 | 84 | 75 | 93 |
| gp Tamamo under Inari | 18 | 54 | 72 | 63 | 81 |

| spark | stars | per-inspiration chance, parents (G1 rule) | q (G1 rule) | q (all-saddle rule) |
|---|---|---|---|---|
| Uma Stan | 2 | 0.2250 / 0.2256 | 0.8535 | 0.8706 |
| No Stopping Me! | 2 | 0.2250 / 0.2256 | 0.8535 | 0.8706 |
| Playtime's Over! | 2 | 0.2250 / 0.2256 | 0.8535 | 0.8706 |
| It's On! | 2 | 0.2250 / 0.2256 | 0.8535 | 0.8706 |
| Tail Held High | 1 | 0.1125 / 0.1128 | 0.5996 | 0.6212 |
| Racing Spirit: Stamina | 3 | 0.3375 / 0.3384 | 0.9522 | 0.9613 |
| Slipstream | 2 | 0.2250 / 0.2256 | 0.8535 | 0.8706 |
| Risky Business | 2 | 0.2250 / 0.2256 | 0.8535 | 0.8706 |

## Trainee Digital; parents veteran #3 (Oguri <- Tamamo + Inari) and #8 (Tamamo <- Oguri + Inari)

Optimizer's own figure for this lineage (win chances instead of wins, star mix 45/50/5, base 0.048): q = 0.7487; parents' affinity 250.0 / 223.7. Same formula at base 0.048 on these veterans: q = 0.7619.
Displayed lineage total (game / calculators): 424 (double circle) under the G1 rule, 460 under the all-saddle rule.

| legacy | relation | race (G1 rule) | affinity | race (all-saddle rule) | affinity |
|---|---|---|---|---|---|
| Oguri | 94 | 168 | 262 | 189 | 283 |
| gp Tamamo under Oguri | 18 | 51 | 69 | 57 | 75 |
| gp Inari under Oguri | 18 | 66 | 84 | 75 | 93 |
| Tamamo | 85 | 156 | 241 | 177 | 262 |
| gp Oguri under Tamamo | 18 | 51 | 69 | 57 | 75 |
| gp Inari under Tamamo | 18 | 54 | 72 | 63 | 81 |

| spark | stars | per-inspiration chance, parents (G1 rule) | q (G1 rule) | q (all-saddle rule) |
|---|---|---|---|---|
| Uma Stan | 2 | 0.2172 / 0.2046 | 0.8391 | 0.8551 |
| No Stopping Me! | 2 | 0.2172 / 0.2046 | 0.8391 | 0.8551 |
| Playtime's Over! | 2 | 0.2172 / 0.2046 | 0.8391 | 0.8551 |
| It's On! | 2 | 0.2172 / 0.2046 | 0.8391 | 0.8551 |
| Tail Held High | 1 | 0.1086 / 0.1023 | 0.5825 | 0.6018 |
| Racing Spirit: Stamina | 3 | 0.3258 / 0.3069 | 0.9439 | 0.9529 |
| Slipstream | 2 | 0.2172 / 0.2046 | 0.8391 | 0.8551 |
| Risky Business | 2 | 0.2172 / 0.2046 | 0.8391 | 0.8551 |

## Trainee Tamamo; parents veteran #5 (Digital <- Inari + Oguri) and #10 (Inari <- Digital + Oguri)

Optimizer's own figure for this lineage (win chances instead of wins, star mix 45/50/5, base 0.048): q = 0.7882; parents' affinity 272.3 / 300.1. Same formula at base 0.048 on these veterans: q = 0.7904.
Displayed lineage total (game / calculators): 487 (double circle) under the G1 rule, 508 under the all-saddle rule.

| legacy | relation | race (G1 rule) | affinity | race (all-saddle rule) | affinity |
|---|---|---|---|---|---|
| Digital | 85 | 189 | 274 | 198 | 283 |
| gp Inari under Digital | 18 | 63 | 81 | 66 | 84 |
| gp Oguri under Digital | 18 | 63 | 81 | 66 | 84 |
| Inari | 112 | 192 | 304 | 207 | 319 |
| gp Digital under Inari | 18 | 63 | 81 | 66 | 84 |
| gp Oguri under Inari | 23 | 66 | 89 | 75 | 98 |

| spark | stars | per-inspiration chance, parents (G1 rule) | q (G1 rule) | q (all-saddle rule) |
|---|---|---|---|---|
| Uma Stan | 2 | 0.2244 / 0.2424 | 0.8638 | 0.8721 |
| No Stopping Me! | 2 | 0.2244 / 0.2424 | 0.8638 | 0.8721 |
| Playtime's Over! | 2 | 0.2244 / 0.2424 | 0.8638 | 0.8721 |
| It's On! | 2 | 0.2244 / 0.2424 | 0.8638 | 0.8721 |
| Tail Held High | 1 | 0.1122 / 0.1212 | 0.6125 | 0.6232 |
| Racing Spirit: Stamina | 3 | 0.3366 / 0.3636 | 0.9578 | 0.9620 |
| Slipstream | 2 | 0.2244 / 0.2424 | 0.8638 | 0.8721 |
| Risky Business | 2 | 0.2244 / 0.2424 | 0.8638 | 0.8721 |

## Trainee Tamamo; parents veteran #2 (Oguri <- Inari + Digital) and #10 (Inari <- Oguri + Digital)

Optimizer's own figure for this lineage (win chances instead of wins, star mix 45/50/5, base 0.048): q = 0.7932; parents' affinity 286.4 / 300.2. Same formula at base 0.048 on these veterans: q = 0.7962.
Displayed lineage total (game / calculators): 503 (double circle) under the G1 rule, 536 under the all-saddle rule.

| legacy | relation | race (G1 rule) | affinity | race (all-saddle rule) | affinity |
|---|---|---|---|---|---|
| Oguri | 95 | 195 | 290 | 216 | 311 |
| gp Inari under Oguri | 23 | 66 | 89 | 75 | 98 |
| gp Digital under Oguri | 18 | 63 | 81 | 66 | 84 |
| Inari | 110 | 195 | 305 | 216 | 326 |
| gp Oguri under Inari | 23 | 66 | 89 | 75 | 98 |
| gp Digital under Inari | 18 | 63 | 81 | 66 | 84 |

| spark | stars | per-inspiration chance, parents (G1 rule) | q (G1 rule) | q (all-saddle rule) |
|---|---|---|---|---|
| Uma Stan | 2 | 0.2340 / 0.2430 | 0.8688 | 0.8812 |
| No Stopping Me! | 2 | 0.2340 / 0.2430 | 0.8688 | 0.8812 |
| Playtime's Over! | 2 | 0.2340 / 0.2430 | 0.8688 | 0.8812 |
| It's On! | 2 | 0.2340 / 0.2430 | 0.8688 | 0.8812 |
| Tail Held High | 1 | 0.1170 / 0.1215 | 0.6189 | 0.6353 |
| Racing Spirit: Stamina | 3 | 0.3510 / 0.3645 | 0.9604 | 0.9666 |
| Slipstream | 2 | 0.2340 / 0.2430 | 0.8688 | 0.8812 |
| Risky Business | 2 | 0.2340 / 0.2430 | 0.8688 | 0.8812 |

## Trainee Tamamo; parents veteran #2 (Oguri <- Digital + Inari) and #5 (Digital <- Oguri + Inari)

Optimizer's own figure for this lineage (win chances instead of wins, star mix 45/50/5, base 0.048): q = 0.7866; parents' affinity 289.6 / 275.5. Same formula at base 0.048 on these veterans: q = 0.7876.
Displayed lineage total (game / calculators): 474 (double circle) under the G1 rule, 495 under the all-saddle rule.

| legacy | relation | race (G1 rule) | affinity | race (all-saddle rule) | affinity |
|---|---|---|---|---|---|
| Oguri | 99 | 192 | 291 | 207 | 306 |
| gp Digital under Oguri | 18 | 63 | 81 | 66 | 84 |
| gp Inari under Oguri | 23 | 66 | 89 | 75 | 98 |
| Digital | 87 | 189 | 276 | 198 | 285 |
| gp Oguri under Digital | 18 | 63 | 81 | 66 | 84 |
| gp Inari under Digital | 18 | 63 | 81 | 66 | 84 |

| spark | stars | per-inspiration chance, parents (G1 rule) | q (G1 rule) | q (all-saddle rule) |
|---|---|---|---|---|
| Uma Stan | 2 | 0.2346 / 0.2256 | 0.8615 | 0.8698 |
| No Stopping Me! | 2 | 0.2346 / 0.2256 | 0.8615 | 0.8698 |
| Playtime's Over! | 2 | 0.2346 / 0.2256 | 0.8615 | 0.8698 |
| It's On! | 2 | 0.2346 / 0.2256 | 0.8615 | 0.8698 |
| Tail Held High | 1 | 0.1173 / 0.1128 | 0.6096 | 0.6203 |
| Racing Spirit: Stamina | 3 | 0.3519 / 0.3384 | 0.9564 | 0.9608 |
| Slipstream | 2 | 0.2346 / 0.2256 | 0.8615 | 0.8698 |
| Risky Business | 2 | 0.2346 / 0.2256 | 0.8615 | 0.8698 |

## Trainee Inari; parents veteran #4 (Digital <- Tamamo + Oguri) and #7 (Tamamo <- Digital + Oguri)

Optimizer's own figure for this lineage (win chances instead of wins, star mix 45/50/5, base 0.048): q = 0.7464; parents' affinity 233.2 / 236.0. Same formula at base 0.048 on these veterans: q = 0.7581.
Displayed lineage total (game / calculators): 427 (double circle) under the G1 rule, 436 under the all-saddle rule.

| legacy | relation | race (G1 rule) | affinity | race (all-saddle rule) | affinity |
|---|---|---|---|---|---|
| Digital | 85 | 159 | 244 | 162 | 247 |
| gp Tamamo under Digital | 18 | 48 | 66 | 48 | 66 |
| gp Oguri under Digital | 18 | 63 | 81 | 66 | 84 |
| Tamamo | 105 | 147 | 252 | 153 | 258 |
| gp Digital under Tamamo | 18 | 48 | 66 | 48 | 66 |
| gp Oguri under Tamamo | 23 | 51 | 74 | 57 | 80 |

| spark | stars | per-inspiration chance, parents (G1 rule) | q (G1 rule) | q (all-saddle rule) |
|---|---|---|---|---|
| Uma Stan | 2 | 0.2064 / 0.2112 | 0.8358 | 0.8400 |
| No Stopping Me! | 2 | 0.2064 / 0.2112 | 0.8358 | 0.8400 |
| Playtime's Over! | 2 | 0.2064 / 0.2112 | 0.8358 | 0.8400 |
| It's On! | 2 | 0.2064 / 0.2112 | 0.8358 | 0.8400 |
| Tail Held High | 1 | 0.1032 / 0.1056 | 0.5786 | 0.5836 |
| Racing Spirit: Stamina | 3 | 0.3096 / 0.3168 | 0.9419 | 0.9444 |
| Slipstream | 2 | 0.2064 / 0.2112 | 0.8358 | 0.8400 |
| Risky Business | 2 | 0.2064 / 0.2112 | 0.8358 | 0.8400 |

## Trainee Inari; parents veteran #1 (Oguri <- Tamamo + Digital) and #7 (Tamamo <- Oguri + Digital)

Optimizer's own figure for this lineage (win chances instead of wins, star mix 45/50/5, base 0.048): q = 0.7565; parents' affinity 250.2 / 246.4. Same formula at base 0.048 on these veterans: q = 0.7673.
Displayed lineage total (game / calculators): 443 (double circle) under the G1 rule, 464 under the all-saddle rule.

| legacy | relation | race (G1 rule) | affinity | race (all-saddle rule) | affinity |
|---|---|---|---|---|---|
| Oguri | 95 | 165 | 260 | 180 | 275 |
| gp Tamamo under Oguri | 23 | 51 | 74 | 57 | 80 |
| gp Digital under Oguri | 18 | 63 | 81 | 66 | 84 |
| Tamamo | 112 | 150 | 262 | 162 | 274 |
| gp Oguri under Tamamo | 23 | 51 | 74 | 57 | 80 |
| gp Digital under Tamamo | 18 | 48 | 66 | 48 | 66 |

| spark | stars | per-inspiration chance, parents (G1 rule) | q (G1 rule) | q (all-saddle rule) |
|---|---|---|---|---|
| Uma Stan | 2 | 0.2160 / 0.2172 | 0.8439 | 0.8532 |
| No Stopping Me! | 2 | 0.2160 / 0.2172 | 0.8439 | 0.8532 |
| Playtime's Over! | 2 | 0.2160 / 0.2172 | 0.8439 | 0.8532 |
| It's On! | 2 | 0.2160 / 0.2172 | 0.8439 | 0.8532 |
| Tail Held High | 1 | 0.1080 / 0.1086 | 0.5880 | 0.5993 |
| Racing Spirit: Stamina | 3 | 0.3240 / 0.3258 | 0.9467 | 0.9520 |
| Slipstream | 2 | 0.2160 / 0.2172 | 0.8439 | 0.8532 |
| Risky Business | 2 | 0.2160 / 0.2172 | 0.8439 | 0.8532 |

## Trainee Inari; parents veteran #1 (Oguri <- Digital + Tamamo) and #4 (Digital <- Oguri + Tamamo)

Optimizer's own figure for this lineage (win chances instead of wins, star mix 45/50/5, base 0.048): q = 0.7694; parents' affinity 269.5 / 263.0. Same formula at base 0.048 on these veterans: q = 0.7743.
Displayed lineage total (game / calculators): 449 (double circle) under the G1 rule, 464 under the all-saddle rule.

| legacy | relation | race (G1 rule) | affinity | race (all-saddle rule) | affinity |
|---|---|---|---|---|---|
| Oguri | 97 | 177 | 274 | 189 | 286 |
| gp Digital under Oguri | 18 | 63 | 81 | 66 | 84 |
| gp Tamamo under Oguri | 23 | 51 | 74 | 57 | 80 |
| Digital | 94 | 174 | 268 | 180 | 274 |
| gp Oguri under Digital | 18 | 63 | 81 | 66 | 84 |
| gp Tamamo under Digital | 18 | 48 | 66 | 48 | 66 |

| spark | stars | per-inspiration chance, parents (G1 rule) | q (G1 rule) | q (all-saddle rule) |
|---|---|---|---|---|
| Uma Stan | 2 | 0.2244 / 0.2208 | 0.8500 | 0.8565 |
| No Stopping Me! | 2 | 0.2244 / 0.2208 | 0.8500 | 0.8565 |
| Playtime's Over! | 2 | 0.2244 / 0.2208 | 0.8500 | 0.8565 |
| It's On! | 2 | 0.2244 / 0.2208 | 0.8500 | 0.8565 |
| Tail Held High | 1 | 0.1122 / 0.1104 | 0.5953 | 0.6033 |
| Racing Spirit: Stamina | 3 | 0.3366 / 0.3312 | 0.9503 | 0.9538 |
| Slipstream | 2 | 0.2244 / 0.2208 | 0.8500 | 0.8565 |
| Risky Business | 2 | 0.2244 / 0.2208 | 0.8500 | 0.8565 |

