// ---------- the scenarios: those this era offers, a pick (with its golds and default lineage rank), their labels ----------
// The scenario picker and the lineage rank control that draw them are ui/Scenario.tsx.
import { SCENARIOS, SCEN_GOLDS, RANKS, gameData } from './data.ts';
import { ERA } from './saved.ts';
import { settings, setScenario } from './settings.ts';
import { srcFor } from './images.ts';
import type { Scenario } from './data.ts';
// the scenarios this era's data offers (Grand Masters only with the JP Grand Masters-era data)
export const LIST = SCENARIOS.filter((s) => !s.era || s.era === ERA);
export const scenarioOf = (v: string): Scenario => LIST.find((s) => s.v === v) || LIST[0];
// the lineage rank a scenario starts from (its `ss`)
const rankOf = (sc: Scenario) => (sc.ss ? RANKS.ss : RANKS.below);
// switch to scenario v (its guaranteed golds follow settings.scen: job.ts goldGiven) and its default lineage rank, which
// can be changed by hand afterwards; saved
export function chooseScenario(v: string) {
  const sc = scenarioOf(v);
  setScenario({ scen: sc.v, rank: rankOf(sc) });
}
// Switching to the JP Grand Masters-era data opens on its newest scenario, Grand Masters (with its golds and rank
// defaults); switching back to Global turns it into Unity Cup on load (initScenario). The settings `era` opens with
// that differ from these (settings.ts switchEra).
export function eraScenario(era: string): { scen?: string; rank?: string } {
  const sc = SCENARIOS.find((s) => s.v === 'grandmasters');
  if (era !== 'gm' || !sc || settings.value.scen === sc.v) return {};
  return { scen: sc.v, rank: rankOf(sc) };
}
// the golds a scenario hands out: [skill group, gold's name] (those with a name: the filter keeps only those)
type GoldName = [group: number, name: string];
export const goldNames = (sc: Scenario): GoldName[] =>
  (SCEN_GOLDS[sc.v] || [])
    .map((group) => [group, gameData().skillNames.get(group)?.gd])
    .filter(([, name]) => name) as GoldName[];
// the tile's (and button's) accessible name: the logo is the only visible name
export const label = (sc: Scenario) => {
  const golds = goldNames(sc).map(([, name]) => name);
  return `${sc.name}, ${sc.rate}${golds.length ? `, ${golds.join(' and ')} guaranteed` : ''}`;
};
// the scenario select art (styled on the career page) as a CSS url(); absolute: a url() in a custom property resolves
// against the stylesheet that uses it, not the page. null without the art.
export const artUrl = (sc: Scenario): string | null => {
  const art = srcFor('scenart', sc.id);
  return art ? `url(${new URL(art, document.baseURI).href})` : null;
};
// A scenario this era does not offer (Grand Masters, saved while the GM-era data was active; the choice is shared by
// both eras) becomes the first one offered, Unity Cup, as if picked. Each page's entry calls it first: the first
// search starts from its setup.
export function initScenario() {
  if (scenarioOf(settings.value.scen).v !== settings.value.scen) chooseScenario(scenarioOf(settings.value.scen).v);
}
