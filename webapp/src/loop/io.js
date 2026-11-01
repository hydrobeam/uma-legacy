// ---------- export / import: the text Export shows, and what an import's text may set ----------
// Values in, values out (the settings, the result on show): Export.tsx shows them and applies an import.
import { fmtIn } from '../dom.js';
import { SETTINGS, ERA } from '../saved.ts';
import { SCENARIOS, RANKS, gameData } from '../data.js';
import { goldGiven, isOwned } from '../job.js';
import { pickOutfit } from '../roster.js';
import { mergePresets } from '../presets.js';
import { deckHint } from '../odds.js';
import { scenarioOf } from '../scenario.js';
/** @import { SettingsView } from '../settings.ts' */
/** @import { ShownResult } from './types.ts' */
// a skill group's name at a level (2: its gold's, when it has one)
const skillName = (group, level) => {
  const names = gameData().skillNames.get(group) || { w: String(group), gd: null };
  return level === 2 ? names.gd || names.w : names.w;
};
// the settings in words, for a reader of the export (an import reads `settings`)
/** @param {SettingsView} s */
function readableSettings(s) {
  return {
    targets: s.targets.map((t) => ({
      skill: skillName(t.group, 1),
      gold: gameData().skillNames.get(t.group)?.gd || null,
      guaranteed: goldGiven(s.scen, t.group) ? 'gold' : 'no',
    })),
    umas_not_owned: gameData()
      .umas.filter((u) => !isOwned(u, s))
      .map((u) => u.name),
    cards_not_owned: gameData()
      .cards.filter((c) => s.cardsOff[c.id] === false)
      .map((c) => c.name),
    deck: s.perUma !== false ? 'find best deck per uma' : 'find best deck (one for the whole loop)',
    game_data: ERA === 'gm' ? 'JP, Grand Masters' : 'Global',
    scenario: scenarioOf(s.scen).name,
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
    .map((o) => ({
      skill: skillName(o.group, 1),
      white: +deckHint(o.gold, o.white).toFixed(3),
      gold: +o.gold.toFixed(3),
    })),
});
// the export: the settings (as saved, and in words) and the result on show (`stale`: it is from other settings)
/** @param {SettingsView} s @param {import('../store.ts').DeepReadonly<ShownResult> | null} shown @param {boolean} stale */
export function exportText(s, shown, stale) {
  const saved = Object.fromEntries(SETTINGS.map((k) => [k, s[k]]));
  const readable = readableSettings(s);
  /** @type {Record<string, unknown> | null} */
  let results = null;
  if (shown) {
    const { r, meta } = shown;
    results = {
      example: !!meta.example,
      decks: (r.top || []).map(resultSummary),
      decks_searched: r.decks_built,
      proven: r.proven,
      ...(stale && { stale: true }), // from other settings than these
    };
  }
  return JSON.stringify(
    { app: 'legacy-loop-planner', version: 1, readable, results, settings: { era: ERA, ...saved } },
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
// An import: the settings an export's `text` carries, those of a known shape (`bad`: the rest, named and left as they
// were), for the era it names; presets join `presets` (yours), never replace them. Throws when the text is no export.
/** @param {string} text @param {SettingsView['presets']} presets @returns {{era: 'gm' | 'global', loaded: Record<string, unknown>, bad: string[]}} */
export function readImport(text, presets) {
  const parsed = JSON.parse(text);
  if (!parsed?.settings) throw new Error('no settings');
  const given = parsed.settings,
    era = given.era === 'gm' ? 'gm' : 'global';
  const keys = SETTINGS.filter((k) => k in given),
    use = keys.filter((k) => VALID[k](given[k], era)),
    bad = keys.filter((k) => !use.includes(k));
  if (!use.length) throw new Error('nothing to load');
  const imported = (k) => (k === 'presets' ? mergePresets(presets, given.presets) : given[k]);
  return { era, loaded: Object.fromEntries(use.map((k) => [k, imported(k)])), bad };
}
