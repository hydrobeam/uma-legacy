// ---------- the odds as the planner pages print them (the odds popup itself is loop/OddsDialog.tsx) ----------
// under 1%: two significant figures (0.0061%, 0.013%), so long odds that differ don't print the same
export const pct = (p: number): string => {
  const percent = p * 100;
  if (p === 0) return '0%';
  if (percent < 1e-6) return '<0.000001%';
  const digits = percent >= 10 ? 0 : percent >= 1 ? 1 : Math.min(6, 1 - Math.floor(Math.log10(percent)));
  return `${percent.toFixed(digits)}%`;
};
// odds as "1 in N" (N rounded from 100 up, one decimal below)
export const fmtIn = (p: number): string => {
  if (!(p > 0)) return 'never';
  const runs = 1 / p;
  return `1 in ${runs >= 100 ? Math.round(runs).toLocaleString() : runs.toFixed(1)}`;
};
// a deck's chance of hinting a target (its gold, which counts for the white too, or else its white): the solver's
// white chance is given no gold hint, so it is gold + (1 - gold) x white
export const deckHint = (gold: number, white: number): number => 1 - (1 - gold) * (1 - white);
