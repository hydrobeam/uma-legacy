// Shorthands the page modules share. (No element lookup by id: a component reaches its DOM by refs.)
// odds as "1 in N" (N rounded from 100 up, one decimal below)
export const fmtIn = (p: number): string => {
  if (!(p > 0)) return 'never';
  const runs = 1 / p;
  return `1 in ${runs >= 100 ? Math.round(runs).toLocaleString() : runs.toFixed(1)}`;
};
