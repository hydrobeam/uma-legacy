// The Global era's page data, as the build writes '#page-data' (vite/pagedata.mjs) from pkg/loopopt/data/global/page.json,
// for node tests that read real records (umadump's export, the solver's reports): a test file hands it to the page's
// modules with `vi.mock('#page-data', () => import('./globalpage.ts'))`, then `await loadData(CAREER)` where it reads
// the career tables. No images: the catalog is empty, so every portrait is its initials.
import fs from 'node:fs';
import path from 'node:path';
import type { Assets, CareerTables, PageData } from '../../src/data.ts';

// (a path, not a URL: Vite turns `new URL(…, import.meta.url)` into an asset import)
const DATA = path.resolve(import.meta.dirname, '../../../pkg/loopopt/data');
const read = (relative: string) => JSON.parse(fs.readFileSync(path.join(DATA, relative), 'utf8'));
const page: PageData & { career: CareerTables } = read('global/page.json');

export const { cards, umas, skills } = page;
/** the career tables, for `loadData` */
export const CAREER = page.career;
export const assets: Assets = {
  wasm: 'solver.test.wasm',
  engine: { global: 'engine.global.test', gm: 'engine.gm.test' },
  dataGm: 'data-gm.test.json',
};
// the shipped images: the skill icons alone, from the art when this tree has it (vite/images.mjs `imageCatalog`)
const IMG = path.resolve(import.meta.dirname, '../../img');
const skillArt = fs.existsSync(path.join(IMG, 'skill')) ? fs.readdirSync(path.join(IMG, 'skill')).sort() : [];
export const img: { keys: string[]; base: string; ext: Record<string, string> } = {
  keys: skillArt.map((name) => `skill:${path.parse(name).name}`),
  base: 'img/',
  ext: { skill: path.parse(skillArt[0] ?? 'x.avif').ext.slice(1) },
};
// skill group -> icon id of its white and gold version, from the art's tables when this tree has them (as the build)
const icons = (name: string): Record<number, number> =>
  fs.existsSync(path.join(IMG, name)) ? JSON.parse(fs.readFileSync(path.join(IMG, name), 'utf8')) : {};
export const skillIcons = { white: icons('skill_icons.json'), gold: icons('skill_icons_gold.json') };
export const raceIds: Record<string, number> = read('page_raceids.json');
export const hintBase = { unity: 0.5, gc: 0.6, ura: 0.4, trackblazer: 0.45, grandmasters: 0.55 };
