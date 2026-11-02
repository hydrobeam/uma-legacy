// @vitest-environment happy-dom
// The setup editors both planner pages share, on happy-dom's document over the real Global data (realpage.ts): the
// scenario picker (its tiles, their hint rates and golds, its keyboard and what closes it, a logo that fails), the
// lineage rank (it follows the scenario until set by hand; its ? note; arrows), the Support cards panel (sections, notes,
// Team Sirius's ban, None owned, a toggle that keeps its picture), the Umas panel (a tile per outfit, its sort menu, a
// toggle, umadump's files) and the target list's keyboard and suggestions; each with the job the settings make.
// npx vitest run (in webapp)
import fs from 'node:fs';
import path from 'node:path';
import { fireEvent, render, within } from '@testing-library/preact';
import { beforeEach, expect, test, vi } from 'vitest';
import { gameData, loadData } from '../../src/model/data.ts';
import * as S from '../../src/state/settings.ts';
import { setupViews } from '../../src/setup/Setup.tsx';
import { CardsPanel } from '../../src/setup/Cards.tsx';
import { RosterPanel } from '../../src/setup/Roster.tsx';
import { buildJob } from '../../src/loop/job.ts';
import type { DeckJob } from '../../src/loop/job.ts';
import { memory } from './fakes.ts';
import { GOLD_ICONS, HINT_BASE_KEYS, REPO, skills } from './realpage.ts';

vi.mock('#page-data', () => import('./realpage.ts'));

document.documentElement.className = 'game';
await loadData(null);

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
const text = (e: Element | null | undefined) => (e?.textContent ?? '').replace(/\s+/g, ' ').trim();
const job = (): DeckJob => {
  const built = S.settings.peek() && buildJob(S.settings.peek());
  if (!('job' in built)) throw new Error(`blocked: ${JSON.stringify(built)}`);
  return built.job;
};
const key = (el: Element, k: string) => fireEvent.keyDown(el, { key: k });
const focused = () => document.activeElement;

// the layout's boxes the editors draw into (loop/Layout.tsx), then the editors
beforeEach(async () => {
  S.initStore(memory);
  const views = setupViews({ page: 'loop' });
  render(
    <>
      <h1>page</h1>
      <div id="scen-dd">{views.scenario}</div>
      <div id="rank-ctl">{views.rank}</div>
      <div id="skills-panel">{views.targets}</div>
      <div id="umas">
        <RosterPanel fadeMore={false} />
      </div>
      <div id="set-cards">
        <CardsPanel fadeMore={false} />
      </div>
    </>,
  );
  await settle();
});

// ---------- the scenario picker
const button = () => $<HTMLButtonElement>('#x-scen');
const menu = () => $('#scen-menu');
const tile = (v: string) => $<HTMLButtonElement>(`#scen-menu [role="menuitemradio"][data-v="${v}"]`);
const openMenu = async () => {
  fireEvent.click(button());
  await settle();
};
// research/results.json's fit, rounded to 3 decimals as pkg/loopopt/build.rs reads it, as a multiple of URA Finale's
const values: Record<string, number> = JSON.parse(
  fs.readFileSync(path.join(REPO, 'research/results.json'), 'utf8'),
).values;
const base = (scen: string) => +(values[HINT_BASE_KEYS[scen] ?? ''] ?? Number.NaN).toFixed(3);
const rate = (scen: string) => `Hint rate ×${(base(scen) / base('ura')).toFixed(2)}`;
const goldName = (group: number) => skills.find(([g]) => g === group)?.[2];

test('scenario: closed at first, the button Unity Cup (its logo only, named); four tiles, each with its hint rate', async () => {
  expect(menu().hidden).toBe(true);
  expect(button().dataset.value).toBe('unity');
  expect(button().getAttribute('aria-label')).toBe('Scenario: Unity Cup');
  expect(text(button())).toBe('');
  expect($$('img', button()).length).toBe(1);
  await openMenu();
  expect(menu().hidden).toBe(false);
  const tiles = $$<HTMLButtonElement>('[role="menuitemradio"]', menu());
  expect(tiles.map((t) => t.dataset.v)).toEqual(['unity', 'gc', 'ura', 'trackblazer']);
  for (const t of tiles) {
    expect(text(t.lastElementChild)).toBe(rate(t.dataset.v ?? ''));
    expect(t.title).toBeTruthy();
    expect(t.title).toBe(t.getAttribute('aria-label'));
  }
  expect(tile('unity').title).toBe(button().title);
});

test("scenario: Unity Cup's tile shows its two guaranteed golds as icons, named by tooltip and alt text, no other's", async () => {
  await openMenu();
  const group = within(tile('unity')).getByRole('group');
  expect(group.getAttribute('aria-label')).toBe('Guaranteed in every run');
  expect(text(group)).toBe('');
  const icons = $$<HTMLImageElement>('img', group);
  expect(icons.map((i) => [i.alt, i.title, i.getAttribute('src')])).toEqual(
    [20049, 20046].map((g) => [goldName(g), goldName(g), `img/skill/${GOLD_ICONS[g]}.avif`]),
  );
  expect($$('[role="menuitemradio"]:not([data-v="unity"]) [role="group"]', menu()).length).toBe(0);
});

test('scenario: opened, the chosen tile is checked and focused; re-picking it closes the picker and changes nothing', async () => {
  await openMenu();
  expect(button().getAttribute('aria-expanded')).toBe('true');
  expect(focused()).toBe(tile('unity'));
  expect(tile('unity').getAttribute('aria-checked')).toBe('true');
  const before = S.settings.peek();
  fireEvent.click(tile('unity'));
  await settle();
  expect(menu().hidden).toBe(true);
  expect(S.settings.peek()).toBe(before);
  expect(focused()).toBe(button());
});

test('scenario keyboard: an arrow on the button opens it, arrows move in two columns, End then Enter picks; Escape returns to the button', async () => {
  button().focus();
  key(button(), 'ArrowDown');
  await settle();
  expect(menu().hidden).toBe(false);
  key(menu(), 'ArrowRight');
  key(menu(), 'ArrowDown');
  expect(focused()).toBe(tile('trackblazer'));
  key(menu(), 'Escape');
  await settle();
  expect(menu().hidden).toBe(true);
  expect(focused()).toBe(button());
  await openMenu();
  key(menu(), 'End');
  expect(focused()).toBe(tile('trackblazer'));
  // (Enter on a button is its click)
  fireEvent.click(tile('trackblazer'));
  await settle();
  expect(menu().hidden).toBe(true);
  expect(button().dataset.value).toBe('trackblazer');
  expect(button().getAttribute('aria-label')).toBe('Scenario: Trackblazer');
  expect(job().deck.scenario).toBe('trackblazer');
});

test('scenario: a click outside closes it: the page, its label, the rank control', async () => {
  for (const outside of [() => $('h1'), () => $('#x-scen-l'), () => $('#rank-ctl > *')]) {
    await openMenu();
    expect(menu().hidden).toBe(false);
    fireEvent.click(outside());
    await settle();
    expect(menu().hidden).toBe(true);
  }
});

test('scenario logos that fail to load fall back to name badges', async () => {
  await openMenu();
  for (const img of $$('#scen-dd [data-scen-logo] img')) fireEvent.error(img);
  await settle();
  expect($$('#scen-dd [data-scen-logo] img').length).toBe(0);
  expect($$('#scen-dd [data-scen-logo] b').map(text)).toEqual([
    'Unity Cup',
    'Unity Cup',
    'Grand Concert',
    'URA Finale',
    'Trackblazer',
  ]);
});

// ---------- the lineage rank
const rank = () => $('#rank');
const pickScen = async (v: string) => {
  await openMenu();
  fireEvent.click(tile(v));
  await settle();
};

test('lineage rank: its ? opens the note, expanded; it follows the scenario (Grand Concert SS, Unity Cup below) until set by hand', async () => {
  fireEvent.click($('#rank-q'));
  await settle();
  expect($('#rank-note').hidden).toBe(false);
  expect($('#rank-q').getAttribute('aria-expanded')).toBe('true');
  fireEvent.click($('#rank-q'));
  await pickScen('gc');
  expect(rank().dataset.value).toBe('ss_and_up');
  expect(job().skills.rank_class).toBe('ss_and_up');
  await pickScen('unity');
  expect(rank().dataset.value).toBe('below_ss');
  fireEvent.click($('#rank button[data-v="ss_and_up"]'));
  await settle();
  await pickScen('unity');
  expect(rank().dataset.value).toBe('ss_and_up');
  expect(job().skills.rank_class).toBe('ss_and_up');
});

test('lineage rank: a radio group with the game badges; the arrows switch it and aria-checked follows', async () => {
  const checked = $<HTMLButtonElement>('#rank button[aria-checked="true"]');
  checked.focus();
  key(checked, 'ArrowRight');
  await settle();
  expect(rank().dataset.value).toBe('ss_and_up');
  key($('#rank button[data-v="ss_and_up"]'), 'ArrowLeft');
  await settle();
  expect(rank().dataset.value).toBe('below_ss');
  expect($('#rank button[data-v="below_ss"]').getAttribute('aria-checked')).toBe('true');
  expect($$('#rank [data-r] img').length).toBe(3);
});

// ---------- the Support cards panel
const cardTile = (name: RegExp) => {
  const found = $$<HTMLButtonElement>('#cards button[aria-pressed]').filter((t) => name.test(text(t)));
  expect(found.length).toBe(1);
  const [only] = found;
  if (!only) throw new Error(`no card ${name}`);
  return only;
};

test('cards: sections SSR, SR, R; hint notes in plain words with tooltips; names without rarity; type icons; R holds only R pals', () => {
  expect($$('#cards [role="group"] h3').map(text)).toEqual(['SSR', 'SR', 'R']);
  const gifts = $$('#cards [data-gift]');
  expect(gifts.length).toBeGreaterThan(0);
  for (const g of gifts) {
    expect(text($('small', g))).toMatch(/^(hint|event|\d(st|nd|rd|th) event)( · \d+%)?$/);
    expect(g.title.length).toBeGreaterThan(0);
  }
  for (const n of $$('#cards button[aria-pressed] [data-card] + span')) expect(text(n)).not.toMatch(/ (SSR|SR|R)$/);
  expect($$('#x-rcards').length).toBe(0);
  const rSection = $$('#cards [role="group"]').find((g) => text($('h3', g)) === 'R');
  const rPals = gameData().cards.filter((c) => c.rar === 1 && c.pal).length;
  expect(rSection ? $$('button[aria-pressed]', rSection).length : -1).toBe(rPals);
  expect($$('#cards button[aria-pressed] [data-card="art"]').length).toBe(
    $$('#cards button[aria-pressed] [data-ctype]').length,
  );
  expect(text($('#cs'))).toBe('Release ↓');
});

test('cards: Team Sirius is not allowed in Unity Cup or Grand Concert, says so, and no job holds it; URA allows it', async () => {
  const sirius = () => cardTile(/Team Sirius/);
  expect(text($('[data-ban]', sirius()))).toBe('Not allowed in Unity Cup');
  await pickScen('ura');
  expect(sirius().querySelector('[data-ban]')).toBeNull();
  await pickScen('gc');
  expect(text($('[data-ban]', sirius()))).toBe('Not allowed in Grand Concert');
});

test('cards: a toggle marks the card not owned and keeps its picture element (no flicker)', async () => {
  const t = $$<HTMLButtonElement>('#cards button[aria-pressed]')[0];
  if (!t) throw new Error('a card');
  const img = t.querySelector('[data-card]');
  fireEvent.click(t);
  await settle();
  expect(t.getAttribute('aria-pressed')).toBe('false');
  expect(t.querySelector('[data-card]')).toBe(img);
});

test('cards: None owned says "0 of N owned"; the job allows no owned card, only one borrowed', async () => {
  fireEvent.click($('#c-none'));
  await settle();
  expect(text($('#c-count'))).toMatch(/^0 of \d+ owned$/);
  const deck = job().deck;
  expect(deck.owned_cards).toEqual([0]);
  expect(deck.borrow).toBe(true);
});

// ---------- the Umas panel
const tiles = () => $$('#roster > [data-uma]');
const firstTile = () => text(tiles()[0]);
const all = () =>
  gameData().umas.flatMap((u) =>
    (u.outfits.length ? u.outfits : [null]).map((o) => ({
      name: u.name,
      label: o?.label ?? '',
      rel: o?.rel ?? 0,
      id: o?.id ?? 0,
    })),
  );

test('umas: one tile per outfit; the sort menu: Release newest first, re-picked oldest first, Name A to Z; it stays open; Escape closes', async () => {
  expect(tiles().length).toBe(all().length);
  const sort = $('#rs');
  const newest = [...all()].sort((a, c) => c.rel - a.rel || c.id - a.id)[0];
  const oldest = [...all()].sort((a, c) => a.rel - c.rel || a.id - c.id)[0];
  const byName = [...all()].sort((a, c) => `${a.name} ${a.label}`.localeCompare(`${c.name} ${c.label}`))[0];
  expect(text(sort)).toBe('Release ↓');
  expect(firstTile()).toContain(newest?.label);
  fireEvent.click(sort);
  await settle();
  fireEvent.click($('#rs-menu [data-k="rel"]'));
  await settle();
  expect(text(sort)).toBe('Release ↑');
  expect(firstTile()).toContain(oldest?.label);
  expect($('#rs-menu').hidden).toBe(false);
  fireEvent.click($('#rs-menu [data-k="name"]'));
  await settle();
  expect(text(sort)).toBe('Name ↑');
  expect(firstTile()).toContain(byName?.name);
  key($('#rs-menu'), 'Escape');
  await settle();
  expect($('#rs-menu').hidden).toBe(true);
});

test('umas: a tap marks that outfit not owned, a second tap owned again', async () => {
  const multi = gameData().umas.find((u) => u.outfits.length > 1);
  if (!multi) throw new Error('an uma with several outfits');
  const states = () => $$(`#roster [data-uma="${multi.id}"]`).map((t) => t.dataset.s);
  const toggle = $(`#roster [data-uma="${multi.id}"] button`);
  fireEvent.click(toggle);
  await settle();
  expect(states().filter((s) => s === 'out').length).toBe(1);
  fireEvent.click(toggle);
  await settle();
  expect(states().every((s) => s === 'own')).toBe(true);
});

// umadump's owned outfits and cards (made-up: val/umadump/U_card_data.json, U_support_card_data.json), both files at
// once through the Umas panel's Import: what they list owned, everything else not owned, ids the data lacks skipped
const pick = async (names: string[]) => {
  const input = $<HTMLInputElement>('#r-file');
  const files = names.map((n) => new File([fs.readFileSync(path.join(REPO, 'val/umadump', n))], n));
  Object.defineProperty(input, 'files', { value: files, configurable: true });
  fireEvent.change(input);
  for (let i = 0; i < 10 && !text($('#r-msg')); i++) await settle();
  await settle();
  return text($('#r-msg'));
};

test('umas: umadump import owns what its files list, nothing else; says what it set and skipped; a veterans file is refused', async () => {
  const msg = await pick(['U_card_data.json', 'U_support_card_data.json']);
  const after = S.settings.peek();
  const { umas, cards } = gameData();
  const fileOutfits = [100101, 100102, 199901],
    fileCards = [30001, 30002, 10001, 39999];
  const outfitIds = umas.flatMap((u) => u.outfits.map((o) => o.id));
  for (const u of umas) expect(after.roster[u.id] === 'out').toBe(!u.outfits.length && u.id !== 1001);
  for (const id of outfitIds) expect(after.outfits[id] === false).toBe(!fileOutfits.includes(id));
  for (const c of cards) expect(after.cardsOff[c.id] === false).toBe(!fileCards.includes(c.id));
  const knownOutfits = fileOutfits.filter((id) => outfitIds.includes(id)).length,
    knownCards = fileCards.filter((id) => cards.some((c) => c.id === id)).length;
  expect(msg).toBe(
    `${knownOutfits} outfits owned (${fileOutfits.length - knownOutfits} not in the data). ${knownCards} cards owned (${fileCards.length - knownCards} not in the data).`,
  );
  const refused = await pick(['U_trained_chara_data.json']);
  expect(refused).toMatch(/veterans file/);
  expect(S.settings.peek()).toBe(after);
});

// ---------- the target list
const targetTiles = () => $$('#targets > [data-g]');
const removes = () => within($('#targets')).queryAllByRole('button', { name: /^Remove / });
const addBox = () => $<HTMLInputElement>('#t-add');
const type = async (value: string) => {
  addBox().focus();
  fireEvent.input(addBox(), { target: { value } });
  await settle();
};

test('targets: a gold the scenario guarantees shows as the gold itself; removing by keyboard moves to the next ×, then the add box', async () => {
  expect(text($('#g-20049 > span'))).toBe(goldName(20049));
  expect($('#g-20049').dataset.g).toBe('2');
  expect(targetTiles().length).toBe(8);
  removes()[2]?.focus();
  fireEvent.click(removes()[2] ?? document.body);
  await settle();
  expect(targetTiles().length).toBe(7);
  expect(focused()).toBe(removes()[2]);
  for (let i = 0; i < 7; i++) {
    const at = focused();
    if (!(at instanceof HTMLElement)) throw new Error('the focus is on a remove button');
    fireEvent.click(at);
    await settle();
  }
  expect(targetTiles().length).toBe(0);
  expect(focused()).toBe(addBox());
});

test('targets: arrows walk the suggestions, Up from the first returns to the box; Escape closes; Enter adds the first match', async () => {
  S.setTargets([]);
  await settle();
  await type('corner');
  const suggestions = () => $$<HTMLButtonElement>('#t-sugg button');
  const first = text(suggestions()[0]);
  key(addBox(), 'ArrowDown');
  key(focused() ?? addBox(), 'ArrowDown');
  key(focused() ?? addBox(), 'ArrowUp');
  expect(focused()).toBe(suggestions()[0]);
  key(focused() ?? addBox(), 'ArrowUp');
  expect(focused()).toBe(addBox());
  key(addBox(), 'ArrowDown');
  key(focused() ?? addBox(), 'Escape');
  await settle();
  expect($('#t-sugg').hidden).toBe(true);
  expect(focused()).toBe(addBox());
  await type('corner');
  key(addBox(), 'Enter');
  await settle();
  const white = first.split(' → ')[0] ?? '';
  expect(targetTiles().length).toBe(1);
  expect(text(targetTiles()[0])).toContain(white);
  expect(focused()).toBe(addBox());
  await type(white.slice(0, 8));
  expect($$('#t-sugg p').map(text)).toContain(`${white} is already a target`);
  await type('zzqx');
  expect(text($('#t-sugg'))).toMatch(/^No skill matches/);
  // a gold matched by its own name shows with its white
  const gold = skills.find(
    ([, w, g]) =>
      g &&
      !w.toLowerCase().includes(g.toLowerCase().slice(0, 6)) &&
      !skills.some(
        (y) =>
          y[0] !== skills.find(([, , gg]) => gg === g)?.[0] &&
          `${y[1]}${y[2] ?? ''}`.toLowerCase().includes(g.toLowerCase()),
      ),
  );
  if (!gold?.[2]) throw new Error('a gold named apart from its white');
  await type(gold[2]);
  expect(text(suggestions()[0])).toBe(`${gold[1]} → ${gold[2]}`);
});
