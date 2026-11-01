// ---------- target skills: the adder's suggestions, adding and removing a target ----------
// The Target skills panel that draws them is ui/Targets.tsx.
import { gameData } from './data.js';
import { settings, setTargets } from './settings.ts';

// a skill group's names: white, and gold where it has one
/** @type {(group: number) => import('./data.js').SkillNames} */
export const namesOf = (group) => gameData().skillNames.get(group) || { w: `Skill ${group}`, gd: null };
const SHOWN_MAX = 30; // suggestions listed at most
/**
 * What the adder offers for the typed text (two characters at least): the catalog's skills whose white or gold name holds
 * it, those not yet targets first.
 * @typedef {object} Suggestions
 * @property {{group: number, white: string, gold: string | null, viaGold: boolean}[]} hits up to SHOWN_MAX skills to
 *   add; `viaGold`: matched on the gold's name only (the button shows the gold with its white: the target is the white,
 *   its gold comes with it)
 * @property {string[]} have up to 3 matching skills that are targets already (their white names)
 * @property {boolean} none no skill matches
 * @param {string} typed @returns {Suggestions}
 */
export function suggest(typed) {
  const filter = typed.trim().toLowerCase();
  if (filter.length < 2) return { hits: [], have: [], none: false };
  const have = new Set(settings.value.targets.map((t) => t.group));
  const matches = (/** @type {string | null} */ name) => !!name && name.toLowerCase().includes(filter);
  const match = gameData().catalog.filter(([, white, gold]) => matches(white) || matches(gold));
  return {
    hits: match
      .filter(([group]) => !have.has(group))
      .slice(0, SHOWN_MAX)
      .map(([group, white, gold]) => ({ group, white, gold, viaGold: matches(gold) && !matches(white) })),
    have: match
      .filter(([group]) => have.has(group))
      .slice(0, 3)
      .map(([, white]) => white),
    none: !match.length,
  };
}
/** @param {number} group */
export const addTarget = (group) => setTargets([...settings.value.targets, { group }]);
/** @param {number} at the target's place */
export const removeTargetAt = (at) => setTargets(settings.value.targets.filter((_, j) => j !== at));
