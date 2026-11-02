// @vitest-environment happy-dom
// Saved settings as the loop page's setup draws them, over the real Global data (realpage.ts): owned umas and cards per
// data set and the shared rest; Grand Masters saved while the Global data is active; Export and its import (a malformed
// setting named and left out, the rest loaded, the stored settings still loading after it); and a banner held ahead of
// its start (data.ts `later`): left out of the page and of what its jobs leave out until the page's clock passes it,
// then there.
// npx vitest run (in webapp)
import { fireEvent, render } from '@testing-library/preact';
import { expect, test, vi } from 'vitest';
import { memory } from './fakes.ts';

vi.mock('#page-data', () => import('./realpage.ts'));

document.documentElement.className = 'game';
const KEY = 'llp:settings';
const settle = async () => {
  for (let i = 0; i < 6; i++) await new Promise((r) => setTimeout(r, 0));
};
const $ = <E extends Element = HTMLElement>(sel: string, root: ParentNode = document): E => {
  const found = root.querySelector<E>(sel);
  if (!found) throw new Error(`no ${sel}`);
  return found;
};
const $$ = <E extends Element = HTMLElement>(sel: string, root: ParentNode = document) => [
  ...root.querySelectorAll<E>(sel),
];

/** The page's modules loaded afresh (the data read at the clock now), the settings from `stored`, the setup drawn. */
async function page(stored: object | null) {
  vi.resetModules();
  document.body.replaceChildren();
  const data = await import('../../src/data.ts');
  const S = await import('../../src/settings.ts');
  const { initScenario } = await import('../../src/scenario.ts');
  const { setupViews } = await import('../../src/ui/Setup.tsx');
  const { RosterPanel } = await import('../../src/ui/Roster.tsx');
  const { CardsPanel } = await import('../../src/ui/Cards.tsx');
  const { ExportDialog, openExport } = await import('../../src/loop/Export.tsx');
  const { initLoopState } = await import('../../src/loop/state.ts');
  const { buildJob } = await import('../../src/loop/job.ts');
  await data.loadData(null);
  const storage = memory();
  if (stored) storage.setItem(KEY, JSON.stringify(stored));
  S.initStore(() => storage);
  initLoopState();
  initScenario();
  const views = setupViews({ page: 'loop' });
  render(
    <>
      <div id="scen-dd">{views.scenario}</div>
      <div id="rank-ctl">{views.rank}</div>
      <div id="skills-panel">{views.targets}</div>
      <RosterPanel fadeMore={false} />
      <CardsPanel fadeMore={false} />
      <ExportDialog />
    </>,
  );
  await settle();
  const job = () => {
    const built = buildJob(S.settings.peek());
    if (!('job' in built)) throw new Error('blocked');
    return built.job;
  };
  return { data, S, storage, openExport, job };
}

test('saved settings load: an uma not owned, a card not owned and the rank; each data set its own owned marks', async () => {
  await page({
    v: 2,
    era: 'global',
    common: { rank: 'ss_and_up' },
    global: { roster: { 1001: 'out' }, cardsOff: { 30052: false } },
    gm: { roster: { 1002: 'out' } },
  });
  const tiles = $$('#roster [data-uma="1001"]');
  expect(tiles.length).toBeGreaterThan(0);
  expect(tiles.every((t) => t.dataset.s === 'out')).toBe(true);
  expect($$('#roster [data-uma="1002"]').every((t) => t.dataset.s === 'own')).toBe(true);
  expect($$('#cards button[aria-pressed="false"]').length).toBe(1);
  expect($('#rank').dataset.value).toBe('ss_and_up');
});

test('Grand Masters saved, the Global data active: Unity Cup is used, with its rank and golds, in the search too', async () => {
  const { job } = await page({
    v: 2,
    era: 'global',
    common: {
      scen: 'grandmasters',
      rank: 'ss_and_up',
      targets: [{ group: 20049 }, { group: 20046 }, { group: 20159 }],
    },
    global: {},
    gm: {},
  });
  expect($('#x-scen').dataset.value).toBe('unity');
  expect(job().deck.scenario).toBe('unity');
  expect($('#rank').dataset.value).toBe('below_ss');
  expect($('#g-20049').dataset.g).toBe('2');
});

test('Export then import: the rank as set and the scenario named; an import sets them; a malformed setting is named and left, the rest loads, and the settings load after it', async () => {
  const { S, storage, openExport } = await page({ v: 2, era: 'global', common: { rank: 'ss_and_up' }, global: {} });
  openExport();
  await settle();
  const box = $<HTMLTextAreaElement>('#ex-text');
  const text = box.value;
  const exported: { settings: { rank: string; targets: unknown }; readable: { scenario: string } } = JSON.parse(text);
  expect(exported.settings.rank).toBe('ss_and_up');
  expect(exported.readable.scenario).toBe('Unity Cup');
  const load = async (value: string) => {
    fireEvent.input(box, { target: { value } });
    fireEvent.click($('#ex-load'));
    await settle();
    return $('#ex-msg').textContent;
  };
  expect(await load(JSON.stringify({ ...exported, settings: { ...exported.settings, rank: 'below_ss' } }))).toBe(
    'Settings loaded.',
  );
  expect($('#rank').dataset.value).toBe('below_ss');
  expect(await load(text.replace('"scen": "unity"', '"scen": "gc"'))).toBe('Settings loaded.');
  expect($('#x-scen').dataset.value).toBe('gc');
  const said = await load(JSON.stringify({ ...exported, settings: { ...exported.settings, targets: 5 } }));
  expect(said).toMatch(/Not understood, so left as they were: targets\.$/);
  expect($$('#targets > [data-g]').length).toBe(8);
  expect($('#rank').dataset.value).toBe('ss_and_up');
  // what is stored loads: the page after a reload has its targets
  const saved = storage.getItem(KEY);
  const again = memory();
  if (saved !== null) again.setItem(KEY, saved);
  S.initStore(() => again);
  expect(S.settings.peek().targets.length).toBe(8);
});

test('a banner ahead of its start is left out of the page and its search; once the clock passes it, it is there', async () => {
  const { cards, umas } = await import('./realpage.ts');
  const outfits = umas.flatMap((u) => u.outfits);
  const newest = Math.max(...cards.map((c) => c.rel), ...outfits.map((o) => o.rel));
  const look = async (at: number) => {
    vi.useFakeTimers({ toFake: ['Date'] });
    vi.setSystemTime(at * 1000);
    const p = await page(null);
    vi.useRealTimers();
    return {
      tiles: $$('#roster > [data-uma]').length,
      cards: $$('#cards button[aria-pressed]').length,
      later: p.data.gameData().later,
    };
  };
  const before = await look(newest - 60),
    after = await look(newest + 60);
  const later = {
    outfits: outfits.filter((o) => o.rel > newest - 60).map((o) => o.id),
    cards: cards.filter((c) => c.rel > newest - 60).map((c) => c.id),
    umas: umas.filter((u) => u.outfits.length && u.outfits.every((o) => o.rel > newest - 60)).map((u) => u.id),
  };
  expect(later.outfits.length + later.cards.length).toBeGreaterThan(0);
  // what every job leaves out (engine.ts `gate` adds it to each job sent: the browser flow loopui.spec.ts checks a job)
  expect(before.later).toEqual(later);
  expect(after.later).toEqual({ umas: [], outfits: [], cards: [] });
  expect(after.tiles - before.tiles).toBe(later.outfits.length);
  expect(after.cards).toBeGreaterThan(before.cards);
});
