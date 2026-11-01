// The research results the page states, read from research/results.json's `values` as pkg/loopopt/build.rs (the
// solver's source) reads them: the same keys, numbers rounded to 3 decimals (toFixed rounds the exact value, as Rust's
// `{:.3}`). A missing or malformed value stops the build, naming its key.
import fs from 'node:fs';

const RESULTS = new URL('../../research/results.json', import.meta.url);
const values = () => JSON.parse(fs.readFileSync(RESULTS, 'utf8')).values;

/** `values."{key}"` to 3 decimals, as text */
function decimals(all, key) {
  const value = all[key];
  if (typeof value !== 'number')
    throw new Error(
      `research/results.json: \`values."${key}"\` is missing or not a number; \`./build.sh research\` computes it`,
    );
  return value.toFixed(3);
}

// each scenario's training-hint base (hints per card per run at hint frequency 0 and specialty 0), by the page's
// scenario key: section 7's fit. Grand Masters has no runs: it takes the fit of one base over every other scenario's.
const HINT_BASE_KEYS = {
  unity: '7.specialty_base_unity_cup',
  gc: '7.specialty_base_grand_concert',
  ura: '7.specialty_base_ura_finale',
  trackblazer: '7.specialty_base_trackblazer',
  grandmasters: '7.one_base_and_one_d_base',
};

/** the scenarios' training-hint bases by scenario key (the page's scenario tiles; the solver's hint_base) */
export function hintBases() {
  const all = values();
  return Object.fromEntries(Object.entries(HINT_BASE_KEYS).map(([scenario, key]) => [scenario, +decimals(all, key)]));
}

/** /about's research figures: the career planner's low-basis cost (whole points, and the bucket's bound) and the ends
 * of the all-target obs/pred ratio's 95% interval (build.rs `race_constants`, `plan_constants`) */
export function aboutFigures() {
  const all = values();
  const costKey = '4.low_basis_cost_fitted_on_every_run',
    bucketKey = '4.low_aptitude_basis_bucket';
  const cost = all[costKey],
    bucket = all[bucketKey];
  if (!Number.isInteger(cost) || cost < 0)
    throw new Error(`research/results.json: \`values."${costKey}"\` is ${JSON.stringify(cost)}, not whole points`);
  if (typeof bucket !== 'string' || !/^<=-?\d+$/.test(bucket))
    throw new Error(
      `research/results.json: \`values."${bucketKey}"\` is ${JSON.stringify(bucket)}, not "<=" and a whole basis`,
    );
  return {
    lowBasisCost: String(cost),
    lowBasisMax: bucket.slice(2),
    pAllLow: decimals(all, '13.p_all_eight_ratio_all_95_pct_low'),
    pAllHigh: decimals(all, '13.p_all_eight_ratio_all_95_pct_high'),
  };
}
