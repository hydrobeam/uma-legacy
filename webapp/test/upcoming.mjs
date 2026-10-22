// a build holds the next banner ahead of its start (pkg/loopopt/data/global/upcoming.json): hidden and left out of every search
// until its start time, then there without a rebuild (the page's clock moved past it)
import fs from 'node:fs';
import { check, open, settle, repoUrl } from './helpers.mjs';

export default async function upcoming(b) {
  const up = JSON.parse(fs.readFileSync(repoUrl('pkg/loopopt/data/global/upcoming.json'), 'utf8'));
  const later = Object.fromEntries(Object.entries(up).map(([k, l]) => [k, l.filter(([, t]) => t > Date.now() / 1000)]));
  const all = Object.values(later).flat();
  if (!all.length) { check('upcoming: nothing in this build starts later (nothing to check)', true); return; }
  const at = Math.max(...all.map(([, t]) => t)) * 1000 + 60000;
  const look = async (init) => {
    const pg = await open(b, { init });
    const tiles = await pg.locator('#roster .ut').count(), cards = await pg.locator('#cards .ct').count();
    await pg.click('#rank button[data-v="0.20,0.70,0.10"]'); await settle(pg);
    const job = await pg.evaluate(() => window.__runs.filter(j => j.cmd === 'deck').at(-1));
    const errs = pg.errors; await pg.context().close();
    return { tiles, cards, job, errs };
  };
  const ids = (k) => later[k].map(([id]) => id), has = (l, xs) => xs.every(x => (l || []).includes(x)), none = (l, xs) => !xs.some(x => (l || []).includes(x));
  const now = await look(null);
  check('upcoming: what starts later is left out of the search', now.job && has(now.job.exclude, ids('umas')) && has(now.job.exclude_outfits, ids('outfits')) && has(now.job.exclude_cards, ids('cards')),
    JSON.stringify(later));
  const then = await look(`{ const off = ${at} - Date.now(), n = Date.now; Date.now = () => n() + off; }`);
  // R cards stay out by default whatever their date (the page's R-cards switch)
  const rare = new Set(JSON.parse(fs.readFileSync(repoUrl('pkg/loopopt/data/global/web_cards.json'), 'utf8')).cards.filter(c => c.rarity > 1 || c.pal).map(c => c.id));
  check('upcoming: once started it is in the search', then.job && none(then.job.exclude, ids('umas')) && none(then.job.exclude_outfits, ids('outfits'))
    // biome-ignore lint/complexity/useOptionalChain: `?.` would turn null into undefined, which the stringified check compares
    && none(then.job.exclude_cards, ids('cards').filter(id => rare.has(id))), JSON.stringify(then.job && then.job.exclude_cards.filter(id => ids('cards').includes(id))));
  check('upcoming: once started its outfits and cards are on the page', then.tiles - now.tiles === ids('outfits').length && then.cards > now.cards,
    `tiles ${now.tiles} -> ${then.tiles}, cards ${now.cards} -> ${then.cards}`);
  check('no page errors', !now.errs.length && !then.errs.length, [...now.errs, ...then.errs].join(' | '));
}
