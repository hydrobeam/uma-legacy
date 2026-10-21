// Her pink spark drawn (MODEL.md) from a career's `aptitudes` (the solver's report): her start grades, each legacy pink
// firing at each of the two April inspirations with its chance (+1 grade, at most S), then one aptitude uniform over those
// she ends at A or S, stars 20/70/10. The solver states the same model as each aptitude's chance of being the pink
// (`aptitudes.pink_gen`, `Env::pink_gen`); the browser suite's `career` group pins this draw against those chances on the
// reference outputs (val/ref__global__career_*.out). `rnd`: the random source (tests pass a seeded one).
export function drawPink(ap, rnd = Math.random) {
  if (!ap) return { pinks: {}, apt: null };
  const g = ap.start.slice();
  for (let e = 0; e < 2; e++) for (const f of ap.fire) if (rnd() < f.p) g[f.apt] = Math.min(8, g[f.apt] + 1);
  const as = g.map((x, k) => x >= 7 ? k : -1).filter(k => k >= 0); if (!as.length) return { pinks: {}, apt: g };
  const k = as[Math.floor(rnd() * as.length)], u = rnd();
  return { pinks: { [ap.pink_gen[k].group]: u < 0.2 ? 1 : u < 0.9 ? 2 : 3 }, apt: g };
}
