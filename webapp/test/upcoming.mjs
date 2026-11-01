// a build holds the next banner ahead of its start (pkg/loopopt/data/global/upcoming.json): hidden and left out of every search
// until its start time, then there without a rebuild (the page's clock moved past it). The newest outfits and cards in
// the page's data stand in for a banner on every build: the page's clock is put a minute before their start (or, while
// the data holds something truly upcoming, a minute after it)
import { check, open, settle, json } from './helpers.mjs';

export default async function upcoming(b) {
  // the page at a clock `off` ms from now: its roster and cards, the job of a search, the data's release times
  const look = async (off) => {
    const pg = await open(b, {
      init: off ? `{ const off = ${off}, n = Date.now; Date.now = () => n() + off; }` : undefined,
    });
    const tiles = await pg.locator('#roster > [data-uma]').count(),
      cards = await pg.locator('#cards button[aria-pressed]').count();
    await pg.click('#rank button[data-v="ss_and_up"]');
    await settle(pg);
    const job = await pg.evaluate(() => window.__runs.filter((j) => j.cmd === 'deck').at(-1));
    const data = { cards: await json(pg, 'data-cards'), umas: await json(pg, 'data-umas') };
    const errs = pg.errors;
    await pg.context().close();
    return { tiles, cards, job, data, errs };
  };
  const first = await look(0),
    { cards: dataCards, umas: dataUmas } = first.data,
    outfits = dataUmas.flatMap((u) => u.outfits || []);
  const newest = Math.max(...dataCards.map((c) => c.rel), ...outfits.map((o) => o.rel)) * 1000;
  // `first` is the "then" look unless the data's newest start is still ahead of the real clock
  const ahead = newest > Date.now(),
    other = await look(newest + (ahead ? 60000 : -60000) - Date.now()),
    [now, then] = ahead ? [first, other] : [other, first],
    from = (ahead ? Date.now() : newest - 60000) / 1000; // the "now" look's clock, s
  // what starts after the "now" look's clock: an uma counts from her first outfit's start (data.ts `gameData().later`)
  const later = {
    outfits: outfits.filter((o) => o.rel > from).map((o) => o.id),
    cards: dataCards.filter((c) => c.rel > from).map((c) => c.id),
    umas: dataUmas.filter((u) => u.outfits?.length && u.outfits.every((o) => o.rel > from)).map((u) => u.id),
  };
  const has = (l, xs) => xs.every((x) => (l || []).includes(x)),
    none = (l, xs) => !xs.some((x) => (l || []).includes(x));
  check(
    'upcoming: what starts later is left out of the search',
    later.outfits.length + later.cards.length > 0 &&
      now.job &&
      has(now.job.skills.exclude, later.umas) &&
      has(now.job.skills.exclude_outfits, later.outfits) &&
      has(now.job.deck.exclude_cards, later.cards),
    JSON.stringify(later),
  );
  // R cards stay out whatever their date (no setting turns them on: settings.ts `rcards`)
  const rare = new Set(dataCards.filter((c) => c.rar > 1 || c.pal).map((c) => c.id));
  check(
    'upcoming: once started it is in the search',
    then.job &&
      none(then.job.skills.exclude, later.umas) &&
      none(then.job.skills.exclude_outfits, later.outfits) &&
      none(
        then.job.deck.exclude_cards,
        later.cards.filter((id) => rare.has(id)),
      ),
    // biome-ignore lint/complexity/useOptionalChain: `?.` would turn null into undefined, which the stringified check compares
    JSON.stringify(then.job && then.job.deck.exclude_cards.filter((id) => later.cards.includes(id))),
  );
  check(
    'upcoming: once started its outfits and cards are on the page',
    then.tiles - now.tiles === later.outfits.length && then.cards > now.cards,
    `tiles ${now.tiles} -> ${then.tiles}, cards ${now.cards} -> ${then.cards}`,
  );
  check('no page errors', !now.errs.length && !then.errs.length, [...now.errs, ...then.errs].join(' | '));
}
