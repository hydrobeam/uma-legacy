// ---------- the scenarios: those this era offers, a pick (with its golds and default lineage rank), their labels ----------
// The scenario picker and the lineage rank control that draw them are ui/Scenario.tsx.
import { SCENARIOS, SCEN_GOLDS, RANKS, gameData } from './data.js';
import { ERA } from './saved.ts';
import { settings, setScenario } from './settings.ts';
import { srcFor } from './images.js';
// the scenarios this era's data offers (Grand Masters only with the JP Grand Masters-era data)
export const LIST = SCENARIOS.filter((s) => !s.era || s.era === ERA);
export const scenarioOf = (v) => LIST.find((s) => s.v === v) || LIST[0];
// the lineage rank a scenario starts from (its `ss`)
const rankOf = (sc) => (sc.ss ? RANKS.ss : RANKS.below);
// switch to scenario v (its guaranteed golds follow settings.scen: job.js goldGiven) and its default lineage rank, which
// can be changed by hand afterwards; saved
/** @param {string} v */
export function chooseScenario(v) {
  const sc = scenarioOf(v);
  setScenario({ scen: sc.v, rank: rankOf(sc) });
}
// Switching to the JP Grand Masters-era data opens on its newest scenario, Grand Masters (with its golds and rank
// defaults); switching back to Global turns it into Unity Cup on load (initScenario). The settings `era` opens with
// that differ from these (settings.ts switchEra).
/** @param {string} era @returns {{scen?: string, rank?: string}} */
export function eraScenario(era) {
  const sc = SCENARIOS.find((s) => s.v === 'grandmasters');
  if (era !== 'gm' || !sc || settings.value.scen === sc.v) return {};
  return { scen: sc.v, rank: rankOf(sc) };
}
// the golds a scenario hands out: [skill group, gold's name] (those with a name)
/** @type {(sc: import('./data.js').Scenario) => [number, string][]} */
export const goldNames = (sc) =>
  (SCEN_GOLDS[sc.v] || []).map((group) => [group, gameData().skillNames.get(group)?.gd]).filter(([, name]) => name);
// the tile's (and button's) accessible name: the logo is the only visible name
/** @param {import('./data.js').Scenario} sc */
export const label = (sc) => {
  const golds = goldNames(sc).map(([, name]) => name);
  return `${sc.name}, ${sc.rate}${golds.length ? `, ${golds.join(' and ')} guaranteed` : ''}`;
};
// the scenario select art (styled on the career page) as a CSS url(); absolute: a url() in a custom property resolves
// against the stylesheet that uses it, not the page. null without the art.
/** @type {(sc: import('./data.js').Scenario) => string | null} */
export const artUrl = (sc) => {
  const art = srcFor('scenart', sc.id);
  return art ? `url(${new URL(art, document.baseURI).href})` : null;
};
// A scenario this era does not offer (Grand Masters, saved while the GM-era data was active; the choice is shared by
// both eras) becomes the first one offered, Unity Cup, as if picked. Each page's entry calls it first: the first
// search starts from its setup.
export function initScenario() {
  if (scenarioOf(settings.value.scen).v !== settings.value.scen) chooseScenario(scenarioOf(settings.value.scen).v);
}
