// ---------- export / import ----------
import { $, $dialog, $input, $textarea, fmtIn } from './dom.js';
import { paintTop } from './cards.js';
import { PER_ERA, SETTINGS, SAVED, ERA, persist } from './settings.js';
import { CARDS, UMAS, SK, SCENARIOS, RANKS } from './data.js';
import { state, ui, goldGiven, dropRequired } from './state.js';
import { save } from './save.js';
import { noOutfit, renderRoster, pickOutfit } from './roster.js';
import { renderTargets } from './targets.js';
import { mergePresets } from './presets.js';
import { paintScen, paintRank, scenarioOf } from './scenario.js';
// a skill group's name at a level (2: its gold's, when it has one)
const skillName = (group, level) => {
  const names = SK.get(group) || { w: String(group), gd: null };
  return level === 2 ? names.gd || names.w : names.w;
};
// the settings in words, for a reader of the export (an import reads `settings`)
function readableSettings() {
  return {
    targets: state.targets.map((t) => ({
      skill: skillName(t.group, 1),
      gold: SK.get(t.group)?.gd || null,
      guaranteed: goldGiven(t.group) ? 'gold' : 'no',
    })),
    umas_not_owned: UMAS.filter((u) => state.roster[u.id] === 'out' || noOutfit(u)).map((u) => u.name),
    cards_not_owned: CARDS.filter((c) => state.cardsOff[c.id] === false).map((c) => c.name),
    deck: state.perUma !== false ? 'find best deck per uma' : 'find best deck (one for the whole loop)',
    game_data: ERA === 'gm' ? 'JP, Grand Masters' : 'Global',
    scenario: scenarioOf(state.scen).name,
  };
}
// a loop's odds and members, each with the outfit the result names
const loopSummary = (loop) => ({
  all_land: fmtIn(loop.p_all),
  at_least_n_minus_1: fmtIn(loop.p_minus1),
  umas: loop.members.map((m, k) => {
    const outfit = pickOutfit(m.id, loop.outfits[k].outfits).list[0];
    return m.name + (outfit ? ` ${outfit.label}` : '');
  }),
});
// one result: its loop, its deck (and each member's when they differ), the deck's hint odds above half a percent
const resultSummary = (top) => ({
  ...loopSummary(top.loop),
  deck: top.deck.cards,
  pals: top.deck.pals,
  borrowed: top.deck.borrowed,
  ...(top.decks
    ? { deck_per_uma: Object.fromEntries(top.loop.members.map((m, k) => [m.name, top.decks[k].cards])) }
    : {}),
  deck_odds: top.deck.odds
    .filter((o) => o.white > 0.005 || o.gold > 0.005)
    .map((o) => ({ skill: skillName(o.group, 1), white: +o.white.toFixed(3), gold: +o.gold.toFixed(3) })),
});
function exportText() {
  const settings = Object.fromEntries(SETTINGS.map((k) => [k, state[k]]));
  const readable = readableSettings();
  let results = null;
  if (ui.lastResult) {
    const { r, meta } = ui.lastResult;
    results = {
      example: !!meta.example,
      decks: (r.top || []).map(resultSummary),
      decks_searched: r.decks_built,
      proven: r.proven,
    };
  }
  return JSON.stringify(
    { app: 'legacy-loop-planner', version: 1, readable, results, settings: { era: ERA, ...settings } },
    null,
    1,
  );
}
// What an import may set, per setting: a value of another shape (a hand edit, an old format) is left out, and the
// setting keeps its value (saved, a wrong type would break every later load).
const obj = (v) => !!v && typeof v === 'object' && !Array.isArray(v);
const VALID = {
  targets: (v) => Array.isArray(v) && v.every((t) => obj(t) && Number.isInteger(t.group)),
  presets: (v) =>
    Array.isArray(v) &&
    v.every(
      (p) =>
        obj(p) &&
        typeof p.name === 'string' &&
        p.name.trim() !== '' &&
        Array.isArray(p.groups) &&
        p.groups.every(Number.isInteger),
    ),
  rank: (v) => Object.values(RANKS).includes(v),
  auto: (v) => typeof v === 'boolean',
  perUma: (v) => typeof v === 'boolean',
  decksTop: (v) => v === '1' || v === '3',
  borrowPar: (v) => ['allow', 'later', 'never'].includes(v),
  scen: (v, era) => SCENARIOS.some((s) => s.v === v && (!s.era || s.era === era)),
  rsort: (v) => v === 'rel' || v === 'name',
  csort: (v) => v === 'rel' || v === 'name',
  rdir: (v) => v === 'asc' || v === 'desc',
  cdir: (v) => v === 'asc' || v === 'desc',
  roster: (v) => obj(v) && Object.values(v).every((x) => x === 'out' || x === 'req'), // 'req': an old export's "must include",
  outfits: (v) => obj(v) && Object.values(v).every((x) => x === false || x === 'req'), // read as owned (dropRequired)
  cardsOff: (v) => obj(v) && Object.values(v).every((x) => x === false),
};
// Load: the settings an export carries (those of a known shape; the rest are named and left as they were). Settings for
// the other data set are stored under its keys and the page reloads into it.
function loadSettings() {
  const message = $('ex-msg');
  try {
    const parsed = JSON.parse($textarea('ex-text').value);
    if (!parsed?.settings) throw new Error('no settings');
    const settings = parsed.settings,
      era = settings.era === 'gm' ? 'gm' : 'global';
    const keys = SETTINGS.filter((k) => k in settings),
      use = keys.filter((k) => VALID[k](settings[k], era)),
      bad = keys.filter((k) => !use.includes(k));
    if (!use.length) throw new Error('nothing to load');
    const imported = (k) => (k === 'presets' ? mergePresets(state.presets, settings.presets) : settings[k]); // presets join yours, never replace them
    if (era !== ERA) {
      for (const k of use) (PER_ERA.includes(k) ? SAVED[era] : SAVED.common)[k] = imported(k);
      SAVED.era = era;
      persist();
      location.reload();
      return;
    }
    for (const k of use) state[k] = imported(k);
    dropRequired(state);
    save();
    paintRank();
    paintTop();
    $input('c-peruma').checked = state.perUma !== false;
    $input('x-auto').checked = state.auto !== false;
    paintScen();
    renderRoster();
    renderTargets();
    message.textContent = `Settings loaded.${bad.length ? ` Not understood, so left as they were: ${bad.join(', ')}.` : ''}`;
  } catch {
    message.textContent = 'That text is not an export from this page.';
  }
}
export function initIO() {
  $('ex-open').onclick = () => {
    $textarea('ex-text').value = exportText();
    $('ex-msg').textContent = '';
    $dialog('ex-dlg').showModal();
  };
  $('ex-close').onclick = () => $dialog('ex-dlg').close();
  $dialog('ex-dlg').addEventListener('click', (e) => {
    if (e.target === $dialog('ex-dlg')) $dialog('ex-dlg').close();
  });
  $('ex-copy').onclick = () => {
    const text = $textarea('ex-text');
    const done = () => {
      $('ex-msg').textContent = 'Copied.';
    };
    const fallback = () => {
      text.focus();
      text.select();
      $('ex-msg').textContent = 'Selected. Press Ctrl+C (or ⌘C) to copy.';
    };
    try {
      navigator.clipboard.writeText(text.value).then(done, fallback);
    } catch {
      fallback();
    }
  };
  $('ex-load').onclick = loadSettings;
}
