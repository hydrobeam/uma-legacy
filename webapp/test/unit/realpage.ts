// The page data for node tests that check what the page draws from the real game (the Global era's exported lists,
// pkg/loopopt/data/global/page.json, as vite/pagedata.mjs '#page-data' hands them out; the scenarios' hint bases read
// from research/results.json as vite/research.mjs reads them). A test file hands it to the page's modules with
// `vi.mock('#page-data', () => import('./realpage.ts'))`. The images it ships are few: the two Unity Cup golds' icons
// (made-up icon ids), the four Global scenarios' logos and the lineage rank badges, so a test sees a picture drawn where
// the page has one (and its fallback when one fails to load); no portrait or card art.
import fs from 'node:fs';
import path from 'node:path';
import type { Card, Uma } from '../../src/data.ts';

/** the repository's root (a path, not a URL: Vite turns `new URL(…, import.meta.url)` into an asset import) */
export const REPO = path.resolve(import.meta.dirname, '../../..');
const read = (relative: string) => JSON.parse(fs.readFileSync(path.join(REPO, relative), 'utf8'));
const page: { cards: Card[]; umas: Uma[]; skills: [number, string, string | null][] } = read(
  'pkg/loopopt/data/global/page.json',
);
export const { cards, umas, skills } = page;

// research/results.json's fits by scenario, rounded to 3 decimals (vite/research.mjs HINT_BASE_KEYS, pkg/loopopt/build.rs)
export const HINT_BASE_KEYS: Record<string, string> = {
  unity: '7.specialty_base_unity_cup',
  gc: '7.specialty_base_grand_concert',
  ura: '7.specialty_base_ura_finale',
  trackblazer: '7.specialty_base_trackblazer',
  grandmasters: '7.one_base_and_one_d_base',
};
const values: Record<string, number> = read('research/results.json').values;
export const hintBase: Record<string, unknown> = Object.fromEntries(
  Object.entries(HINT_BASE_KEYS).map(([scenario, key]) => [scenario, +(values[key] ?? Number.NaN).toFixed(3)]),
);
/** the made-up icon ids of Unity Cup's two golds (No Stopping Me!, It's On!) */
export const GOLD_ICONS: Record<number, number> = { 20049: 9001, 20046: 9002 };
export const assets = { wasm: 'solver.test.wasm', engine: { global: 'engine.global.test', gm: 'engine.gm.test' } };
export const img: { keys: string[]; base: string; ext: Record<string, string> } = {
  keys: [
    ...Object.values(GOLD_ICONS).map((id) => `skill:${id}`),
    ...[1, 2, 3, 4].map((id) => `scen:${id}`),
    ...['b', 'splus', 'ss'].map((rank) => `ui:rank_${rank}`),
  ],
  base: 'img/',
  ext: { skill: 'avif', scen: 'avif', ui: 'webp' },
};
export const skillIcons: { white: Record<number, number>; gold: Record<number, number> } = {
  white: {},
  gold: GOLD_ICONS,
};
export const raceIds: Record<string, number> = read('pkg/loopopt/data/page_raceids.json');
