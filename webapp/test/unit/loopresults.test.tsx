// @vitest-environment happy-dom
// The loop page's result cards (loop/Results.tsx), its status line (loop/RunNote.tsx), the odds popup and the export, on
// happy-dom's document with the Global era's lists (globalpage.ts) and a recorded result of the default settings'
// search (loopdefault.json: the solver's reply, its loops and decks). What these draw from a result and from the
// settings, without a browser: the layout and the keyboard are the browser flows' (test/browser/loop.spec.ts).
// npx vitest run (in webapp)
import fs from 'node:fs';
import path from 'node:path';
import { fireEvent, render } from '@testing-library/preact';
import { createRef } from 'preact';
import { expect, test, vi } from 'vitest';
import { loadData, SCEN_GOLDS } from '../../src/model/data.ts';
import { deckHint } from '../../src/model/odds.ts';
import { goldGiven } from '../../src/model/job.ts';
import { exportText, readImport } from '../../src/loop/io.ts';
import { blockedStatus, buildJob } from '../../src/loop/job.ts';
import { drawResult, Results } from '../../src/loop/Results.tsx';
import { RunAct, RunNote } from '../../src/loop/RunNote.tsx';
import { beginSearch, initLoopState, interim, loopDrawn, settle, tick } from '../../src/loop/state.ts';
import type { DrawnReport, LoopResult, ShownResult } from '../../src/loop/types.ts';
import { loadExample, saveCurrent } from '../../src/state/presets.ts';
import { PresetDialogs, openPresets } from '../../src/setup/Presets.tsx';
import {
  DEFAULTS,
  importSettings,
  initStore,
  jobSetup,
  settings,
  setOwned,
  setPresets,
  setTargets,
} from '../../src/state/settings.ts';
import { on } from '../../src/state/store.ts';
import { OddsDialog } from '../../src/ui/OddsDialog.tsx';
import { RenderGuard, sayRenderErrors } from '../../src/ui/RenderGuard.tsx';
import { memory } from './fakes.ts';
import { SKILL_ART, skills, umas } from './globalpage.ts';

vi.mock('#page-data', () => import('./globalpage.ts'));

document.documentElement.className = 'game'; // (the planner pages' class)
await loadData(null);

const REPORT: DrawnReport = JSON.parse(fs.readFileSync(path.join(import.meta.dirname, 'loopdefault.json'), 'utf8'));
const settled = async () => {
  for (let i = 0; i < 5; i++) await new Promise((r) => setTimeout(r, 0));
};
const fresh = () => {
  initStore(() => memory());
  initLoopState();
};
/** the job of the settings now (blocked settings make none) */
const job = () => {
  const built = buildJob(settings.peek());
  if ('blocked' in built) throw new Error(`no job: ${built.blocked}`);
  return built;
};
const targets = () => job().targets;
/** the results column with `r` drawn in it, as the page draws a finished search */
function drawn(r: DrawnReport = REPORT) {
  const column = createRef<HTMLElement>();
  const view = render(
    <main>
      <section id="rcol" tabIndex={-1} ref={column}>
        <div id="results">
          <Results column={column} />
        </div>
      </section>
      <OddsDialog />
    </main>,
  );
  drawResult(r, targets());
  settle({ kind: 'done', text: 'Checked' }, { kind: 'deck', r, meta: {} });
  return view;
}
const all = (root: ParentNode, selector: string) => [...root.querySelectorAll<HTMLElement>(selector)];
const one = (root: ParentNode, selector: string): HTMLElement => {
  const found = root.querySelector<HTMLElement>(selector);
  if (!found) throw new Error(`no ${selector}`);
  return found;
};
const text = (e: Element | null | undefined) => (e?.textContent ?? '').replace(/\s+/g, ' ').trim();

test('the default search is a Unity Cup job, with no loop size (the solver picks rotations or loops)', () => {
  fresh();
  const built = job().job;
  expect(built.deck.scenario).toBe('unity');
  expect(built.skills.sizes).toBeUndefined();
});

test('headline: "1 in 103 runs to land all 8" alone, no rank with one result; three members, each "as trainee: 1 in …"', async () => {
  fresh();
  const { container } = drawn();
  await settled();
  const card = one(container, '#results article');
  expect(text(card.querySelector('[data-odds]'))).toBe('1 in 103 runs to land all 8');
  expect(card.querySelector('[data-rank]')).toBeNull();
  const own = all(card, '[data-own]').map(text);
  expect(own).toHaveLength(3);
  for (const o of own) expect(o).toMatch(/^as trainee: 1 in [\d,.]+$/);
});

test('members: each names one outfit, never "any outfit" (her popup carries any note)', async () => {
  fresh();
  const { container } = drawn();
  await settled();
  const members = all(container, '#results [data-member]');
  expect(members.length).toBeGreaterThan(0);
  for (const m of members) {
    expect(m.dataset.outfitLabel).toBeTruthy();
    expect(m.dataset.outfitLabel).not.toMatch(/any outfit/i);
  }
  expect(text(one(container, '#results'))).not.toMatch(/any outfit/i);
});

test('deck: Sasami Anshinzawa SSR says "prioritise Risky Business"; no part of a chip carries a title of its own', async () => {
  fresh();
  const { container } = drawn();
  await settled();
  const notes = all(container, '#results button[data-cid="30080"] i').map(text);
  expect(notes).toContain('prioritise Risky Business');
  const chips = all(container, '#results button[data-cid]');
  expect(chips.length).toBeGreaterThan(0);
  expect(chips.flatMap((c) => all(c, '[title]'))).toEqual([]);
});

test('deck odds: closed at first, the (i) opens them (names, percentages, a skill icon each) and closes them', async () => {
  fresh();
  const { container, getAllByRole } = drawn();
  await settled();
  const panel = one(container, '#results [data-hint-odds]');
  expect(panel.hidden).toBe(true);
  const [info] = getAllByRole('button', { name: 'Hint odds from this deck' });
  if (!info) throw new Error('an (i) button');
  fireEvent.click(info);
  await settled();
  expect(info.getAttribute('aria-expanded')).toBe('true');
  expect(panel.hidden).toBe(false);
  const hints = all(panel, '[data-hint]');
  expect(hints.length).toBeGreaterThan(0);
  // (the icons only where this tree has the game's art)
  if (SKILL_ART) expect(all(panel, '[data-hint] img[data-icon="skill"]')).toHaveLength(hints.length);
  expect(text(panel)).toMatch(/\d+%/);
  fireEvent.click(info);
  await settled();
  expect(panel.hidden).toBe(true);
});

test("the deck's hint chance (gold, else white) is one figure: the (i) panel's white tiles and Export's, from the result's fields", async () => {
  fresh();
  const { container } = drawn();
  await settled();
  const [top] = REPORT.top;
  const odds = top.deck.odds.filter((o) => o.white > 0.005 || o.gold > 0.005);
  expect(odds.some((o) => o.gold > 0.005 && o.white > 0.005)).toBe(true);
  const tiles = all(one(container, '#results [data-hint-odds]'), '[data-hint="white"] b').map(text);
  expect(tiles).toEqual(
    odds
      .filter((o) => deckHint(o.gold, o.white) - o.gold > 0.005)
      .map((o) => `${Math.round(deckHint(o.gold, o.white) * 100)}%`),
  );
  const exported = JSON.parse(exportText(settings.peek(), { kind: 'deck', r: REPORT, meta: {} }, false));
  const deckOdds: { white: number; gold: number }[] = exported.results.decks[0].deck_odds;
  expect(deckOdds).toEqual(
    odds.map((o) =>
      expect.objectContaining({ white: +deckHint(o.gold, o.white).toFixed(3), gold: +o.gold.toFixed(3) }),
    ),
  );
});

test('swap rows: after a "Swaps" label, each the cards out ➜ the cards in, its changes "Skill a% → b%", its umas as icons named for screen readers', async () => {
  fresh();
  const { container } = drawn();
  await settled();
  const rows = all(container, '#results [data-deck="swap"]');
  expect(rows.length).toBeGreaterThan(0);
  const label = all(container, '#results article p').find((p) => text(p) === 'Swaps');
  if (!label || !rows[0]) throw new Error('a Swaps label and a swap row');
  expect(label.compareDocumentPosition(rows[0]) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  for (const row of rows) {
    const out = all(row, '[data-side="out"] button[data-cid]').length;
    expect(out).toBeGreaterThan(0);
    expect(all(row, '[data-side="in"] button[data-cid]')).toHaveLength(out);
    const changes = all(row, '[data-delta]').map(text);
    expect(changes.length).toBeGreaterThan(0);
    for (const c of changes) expect(c).toMatch(/\d+% → \d+%$/);
    for (const who of all(row, '[data-who]')) {
      const faces = all(who, 'button[data-uma]');
      expect(faces.length).toBeGreaterThan(0);
      expect(text(who.querySelector(':scope > span'))).toMatch(/’s deck:$/);
      for (const f of faces) expect(f.title).toMatch(/^(?!.* and )(?!.*’s deck).+/);
      const shown = [...who.childNodes].filter((n) => n.nodeType === Node.TEXT_NODE).map((n) => n.textContent);
      expect(shown.join('').trim()).toBe('');
    }
  }
});

test("swap rows leave out the targets their umas have on their own skill list; a row's changes unfold and fold", async () => {
  fresh();
  const { container } = drawn();
  await settled();
  const whiteOf = new Map<string, string>();
  for (const [, white, gold] of skills) if (gold) whiteOf.set(gold, white);
  const rows = all(container, '#results [data-deck="swap"]').map((d) => {
    const ids = all(d, '[data-who] button[data-uma]').map((f) => f.dataset.uma);
    const per = ids.map((id) =>
      all(container, `#results [data-member][data-uma="${id}"] [data-spark]`).flatMap((s) =>
        s.dataset.spark === 'gold' ? [text(s), whiteOf.get(text(s)) ?? ''] : [text(s)],
      ),
    );
    const [first = []] = per;
    return { mine: first.filter((n) => per.every((p) => p.includes(n))), changes: all(d, '[data-delta]').map(text) };
  });
  expect(rows.some((r) => r.mine.length)).toBe(true);
  for (const r of rows) for (const c of r.changes) expect(r.mine.some((n) => c.startsWith(`${n} `))).toBe(false);
  // (a row with more than two changes folds the rest behind its button)
  const more = container.querySelector<HTMLElement>(
    '#results [data-deck="swap"] button[aria-expanded]:has([data-delta])',
  );
  if (!more) return;
  fireEvent.click(more);
  await settled();
  expect(more.getAttribute('aria-expanded')).toBe('true');
  fireEvent.click(more);
  await settled();
  expect(more.getAttribute('aria-expanded')).toBe('false');
});

test('three results: ranked #1, #2, #3; one with #1\'s loop says "Same loop as #1" in place of its umas', async () => {
  fresh();
  const [top] = REPORT.top;
  const other: LoopResult = { ...top, deck: { ...top.deck, card_ids: [...top.deck.card_ids].reverse() } };
  const { container } = drawn({ ...REPORT, top: [top, other, other] });
  await settled();
  const cards = all(container, '#results article');
  expect(cards.map((c) => text(c.querySelector('[data-rank]')))).toEqual(['#1', '#2', '#3']);
  const [, second] = cards;
  expect(second?.querySelector('[data-same]')).not.toBeNull();
  expect(second?.querySelectorAll('[data-member]')).toHaveLength(0);
  expect(cards[0]?.querySelectorAll('[data-member]')).toHaveLength(3);
});

test('a fresh identical result is not drawn again: the same elements stay', async () => {
  fresh();
  const { container } = drawn();
  await settled();
  const odds = container.querySelector('#results [data-odds]');
  drawResult(structuredClone(REPORT), targets());
  await settled();
  expect(container.querySelector('#results [data-odds]')).toBe(odds);
});

test('P5: a card un-owned in the picker greys it in the result on show at once (the same element); owned again, not', async () => {
  fresh();
  const { container } = drawn();
  await settled();
  const chip = one(container, '#results button[data-cid]:not([data-tag="borrowed"])');
  const id = Number(chip.dataset.cid);
  setOwned({ cardsOff: { [id]: false } });
  await settled();
  expect(chip.isConnected).toBe(true);
  expect(chip.hasAttribute('data-off')).toBe(true);
  setOwned({ cardsOff: {} });
  await settled();
  expect(chip.hasAttribute('data-off')).toBe(false);
});

test('a result drawn while the focus is in it: the focus goes to the results column', async () => {
  fresh();
  const { container } = drawn();
  await settled();
  one(container, '#results button[data-cid]').focus();
  const [top] = REPORT.top;
  drawResult(
    { ...REPORT, top: [{ ...top, loop: { ...top.loop, members: [...top.loop.members].reverse() } }] },
    targets(),
  );
  await settled();
  expect(document.activeElement).toBe(one(container, '#rcol'));
});

test('loop odds: the Odds button opens every count 8/8 .. 0/8, all 8 at the headline odds, 7 or better at 1 in 12.1', async () => {
  fresh();
  const { getAllByRole, container } = drawn();
  await settled();
  const [odds] = getAllByRole('button', { name: 'Odds' });
  if (!odds) throw new Error('an Odds button');
  fireEvent.click(odds);
  await settled();
  const rows = all(container, '#odds-dlg table tbody tr').map((r) => [...r.children].map((c) => text(c)));
  const p = (row: string[] | undefined) => Number.parseFloat(row?.[1] ?? '') / 100;
  expect(rows).toHaveLength(9);
  expect(rows[0]?.[0]).toBe('8/8');
  expect(rows[8]?.[0]).toBe('0/8');
  expect(Math.abs(1 / p(rows[0]) - 103)).toBeLessThan(2);
  expect(Math.abs(1 / (p(rows[0]) + p(rows[1])) - 12.1)).toBeLessThan(0.3);
});

test('a result that throws while drawn: the status line says so, the column says why it is empty; the next result draws', async () => {
  fresh();
  const said: string[] = [];
  sayRenderErrors((t) => said.push(t));
  const reported: unknown[] = [];
  const report = globalThis.reportError;
  globalThis.reportError = (e) => reported.push(e);
  const { container } = render(
    <div id="results">
      <RenderGuard input={loopDrawn} what="The result" fallback={<p>The result could not be drawn.</p>}>
        <Results column={createRef()} />
      </RenderGuard>
    </div>,
  );
  const [top] = REPORT.top;
  drawResult({ ...REPORT, top: [{ ...top, loop: { ...top.loop, outfits: [] } }] }, targets());
  await settled();
  expect(said.join(' | ')).toMatch(/^The result could not be drawn: /);
  expect(reported.length).toBeGreaterThan(0);
  expect(container.querySelectorAll('article')).toHaveLength(0);
  expect(text(one(container, '#results'))).toBe('The result could not be drawn.');
  drawResult(REPORT, targets());
  await settled();
  expect(container.querySelectorAll('article').length).toBeGreaterThan(0);
  globalThis.reportError = report;
});

test('the status line: "Searching…", its clock, the exact check; a change mid-search clears the clock until the new search runs', async () => {
  fresh();
  const { container } = render(
    <>
      <p id="run-note">
        <RunNote />
      </p>
      <p id="r-act">
        <RunAct stop={() => undefined} />
      </p>
    </>,
  );
  beginSearch(1000);
  await settled();
  expect(text(one(container, '#run-note'))).toBe('Searching…');
  expect(text(one(container, '#r-act'))).toBe('Stop');
  tick(4200);
  await settled();
  expect(text(one(container, '#run-note'))).toMatch(/^Searching… 3 s/);
  interim({ kind: 'deck', r: REPORT, meta: {} }, 4300);
  await settled();
  expect(text(one(container, '#run-note'))).toMatch(/^Checking the exact odds…/);
  // a change mid-search: a new search's pause starts its clock afresh
  beginSearch(5000);
  await settled();
  expect(text(one(container, '#run-note'))).toBe('Searching…');
});

test('blocked settings: no targets says to add one, its button loads the 8/8 example; no umas offers to mark them all owned', async () => {
  fresh();
  const { container } = render(
    <>
      <p id="run-note">
        <RunNote />
      </p>
      <p id="r-act">
        <RunAct stop={() => undefined} />
      </p>
    </>,
  );
  const blocked = () => {
    const built = buildJob(settings.peek());
    if (!('blocked' in built)) throw new Error('blocked settings');
    return blockedStatus(built, loadExample);
  };
  setTargets([]);
  settle(blocked());
  await settled();
  expect(text(one(container, '#run-note'))).toMatch(/Add a target skill/);
  const load = one(container, '#r-act button');
  expect(text(load)).toMatch(/8\/8 example/);
  fireEvent.click(load);
  expect(settings.peek().targets).toHaveLength(8);
  setOwned({ roster: Object.fromEntries(umas.map((u) => [u.id, 'out'])) });
  settle(blocked());
  await settled();
  expect(text(one(container, '#run-note'))).toMatch(/at least 3 umas/);
  const mark = one(container, '#r-act button');
  expect(text(mark)).toMatch(/Mark all umas owned/);
  fireEvent.click(mark);
  expect(Object.values(settings.peek().roster)).not.toContain('out');
});

test("scenario golds: Unity Cup guarantees No Stopping Me! and It's On!, URA none; the 8/8 example under URA has none", () => {
  fresh();
  expect(SCEN_GOLDS.unity).toEqual(expect.arrayContaining([20049, 20046]));
  expect(goldGiven('unity', 20049) && goldGiven('unity', 20046)).toBe(true);
  expect(goldGiven('ura', 20049) || goldGiven('ura', 20046)).toBe(false);
  importSettings({ scen: 'ura' });
  loadExample();
  const { targets: sent } = job();
  expect(sent).toHaveLength(8);
  expect(sent.every((t) => !t.guaranteed)).toBe(true);
});

test('a preset saved changes no job setting (no search starts); scenario and presets survive a reload; an import merges presets', () => {
  const stored = memory();
  initStore(() => stored);
  let woke = 0;
  const off = on([jobSetup], () => {
    woke++;
  });
  const before = woke;
  setPresets([...settings.peek().presets, { name: 'Seven', groups: [20159, 20049] }]);
  expect(woke).toBe(before);
  off();
  importSettings({ scen: 'ura' });
  initStore(() => stored); // (a reload: the record read back from storage)
  expect(settings.peek().scen).toBe('ura');
  expect(settings.peek().presets.map((p) => p.name)).toEqual(['8/8 example', 'Seven']);
  const theirs = JSON.parse(exportText(settings.peek(), null, false));
  theirs.settings.presets = [
    { name: 'Seven', groups: [20159] },
    { name: 'Other', groups: [20049] },
  ];
  const { loaded } = readImport(JSON.stringify(theirs), settings.peek().presets);
  importSettings(loaded);
  expect(settings.peek().presets.map((p) => p.name)).toEqual(['8/8 example', 'Seven', 'Seven (imported)', 'Other']);
});

test("an old export's must-include mark ('req') loads as owned", async () => {
  fresh();
  const u = umas.find((x) => x.outfits.length > 1);
  if (!u) throw new Error('an uma with two outfits');
  const [outfit] = u.outfits;
  if (!outfit) throw new Error('her outfit');
  const theirs = JSON.parse(exportText(DEFAULTS, null, false));
  theirs.settings.outfits = { [outfit.id]: 'req' };
  theirs.settings.roster = { [u.id]: 'req' };
  importSettings(readImport(JSON.stringify(theirs), settings.peek().presets).loaded);
  expect(settings.peek().outfits[outfit.id]).not.toBe(false);
  expect(settings.peek().roster[u.id]).toBeUndefined();
});

test('Export marks a result on show from other settings as stale', () => {
  fresh();
  const shown: ShownResult = { kind: 'deck', r: REPORT, meta: {} };
  expect(JSON.parse(exportText(settings.peek(), shown, true)).results.stale).toBe(true);
  expect(JSON.parse(exportText(settings.peek(), shown, false)).results.stale).toBeUndefined();
});

test('presets: one saved is marked current; with no targets, Overwrite is off on every preset (a preset is never emptied)', async () => {
  fresh();
  const { container } = render(<PresetDialogs />);
  setTargets(settings.peek().targets.slice(1)); // (one removed: seven of the example's eight)
  expect(saveCurrent('Seven').saved).toBe(true);
  openPresets();
  await settled();
  const row = (name: string) => one(container, `#pr-list [role="group"][aria-label="${name}"]`);
  expect(text(row('Seven'))).toMatch(/current/);
  expect(text(row('8/8 example'))).not.toMatch(/current/);
  setTargets([]);
  await settled();
  const overwrite = all(container, '#pr-list [role="group"] button').filter((b) => text(b) === 'Overwrite');
  expect(overwrite.length).toBeGreaterThan(0);
  for (const b of overwrite) expect(b.hasAttribute('disabled')).toBe(true);
});
