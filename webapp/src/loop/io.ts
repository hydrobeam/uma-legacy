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
import type { SettingsView } from '../settings.ts';
import type { DeepReadonly } from '../store.ts';
import type { LoopResult, ResultLoop, ShownResult } from './types.ts';
// a skill group's name at a level (2: its gold's, when it has one)
const skillName = (group: number, level: number) => {
  const names = gameData().skillNames.get(group) || { w: String(group), gd: null };
  return level === 2 ? names.gd || names.w : names.w;
};
// the settings in words, for a reader of the export (an import reads `settings`)
function readableSettings(s: SettingsView) {
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
const loopSummary = (loop: DeepReadonly<ResultLoop>) => ({
  all_land: fmtIn(loop.p_all),
  at_least_n_minus_1: fmtIn(loop.p_minus1),
  umas: loop.members.map((m, k) => {
    // (a copy: the result's outfits are read-only, and pickOutfit may hand back the list it is given)
    const outfit = pickOutfit(m.id, [...loop.outfits[k].outfits]).list[0];
    return m.name + (outfit ? ` ${outfit.label}` : '');
  }),
});
// one result: its loop, its deck (and each member's when they differ), the deck's hint odds above half a percent
const resultSummary = ({ loop, deck, decks }: DeepReadonly<LoopResult>) => ({
  ...loopSummary(loop),
  deck: deck.cards,
  pals: deck.pals,
  borrowed: deck.borrowed,
  ...(decks ? { deck_per_uma: Object.fromEntries(loop.members.map((m, k) => [m.name, decks[k].cards])) } : {}),
  deck_odds: deck.odds
    .filter((o) => o.white > 0.005 || o.gold > 0.005)
    .map((o) => ({
      skill: skillName(o.group, 1),
      white: +deckHint(o.gold, o.white).toFixed(3),
      gold: +o.gold.toFixed(3),
    })),
});
// the export: the settings (as saved, and in words) and the result on show (`stale`: it is from other settings)
export function exportText(s: SettingsView, shown: DeepReadonly<ShownResult> | null, stale: boolean) {
  const saved = Object.fromEntries(SETTINGS.map((k) => [k, s[k]]));
  const readable = readableSettings(s);
  let results: Record<string, unknown> | null = null;
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
// setting keeps its value (saved, a wrong type would break every later load). Each takes the imported value as it
// comes (unknown) and the era the export names.
const obj = (v: unknown): v is Record<string, unknown> => !!v && typeof v === 'object' && !Array.isArray(v);
const VALID: Record<(typeof SETTINGS)[number], (v: unknown, era: string) => boolean> = {
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
  rank: (v) => Object.values(RANKS).includes(v as string), // (includes compares with ===: any value may be asked)
  auto: (v) => typeof v === 'boolean',
  perUma: (v) => typeof v === 'boolean',
  decksTop: (v) => v === '1' || v === '3',
  borrowPar: (v) => ['allow', 'later', 'never'].includes(v as string),
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
export function readImport(
  text: string,
  presets: SettingsView['presets'],
): { era: 'gm' | 'global'; loaded: Record<string, unknown>; bad: string[] } {
  const parsed = JSON.parse(text);
  if (!parsed?.settings) throw new Error('no settings');
  const given = parsed.settings,
    era = given.era === 'gm' ? 'gm' : 'global';
  const keys = SETTINGS.filter((k) => k in given),
    use = keys.filter((k) => VALID[k](given[k], era)),
    bad = keys.filter((k) => !use.includes(k));
  if (!use.length) throw new Error('nothing to load');
  const imported = (k: (typeof SETTINGS)[number]) =>
    k === 'presets' ? mergePresets(presets, given.presets) : given[k];
  return { era, loaded: Object.fromEntries(use.map((k) => [k, imported(k)])), bad };
}
