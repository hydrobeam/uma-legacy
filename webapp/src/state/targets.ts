// ---------- target skills: the adder's suggestions, adding and removing a target ----------
// The Target skills panel that draws them is ui/Targets.tsx.
import { gameData } from '../model/data.ts';
import { settings, setTargets } from './settings.ts';
import type { SkillNames } from '../model/data.ts';
import type { Target } from './settings.ts';

// a skill group's names: white, and gold where it has one
export const namesOf = (group: number): SkillNames =>
  gameData().skillNames.get(group) || { w: `Skill ${group}`, gd: null };
const SHOWN_MAX = 30; // suggestions listed at most
/** What the adder offers for the typed text (two characters at least): the catalog's skills whose white or gold name
 * holds it, those not yet among `targets` first. */
export interface Suggestions {
  /** up to SHOWN_MAX skills to add; `viaGold`: matched on the gold's name only (the button shows the gold with its
   * white: the target is the white, its gold comes with it) */
  hits: { group: number; white: string; gold: string | null; viaGold: boolean }[];
  /** up to 3 matching skills that are targets already */
  have: { group: number; white: string }[];
  /** no skill matches */
  none: boolean;
}
export function suggest(typed: string, targets: readonly Target[]): Suggestions {
  const filter = typed.trim().toLowerCase();
  if (filter.length < 2) return { hits: [], have: [], none: false };
  const have = new Set(targets.map((t) => t.group));
  const matches = (name: string | null) => !!name && name.toLowerCase().includes(filter);
  const match = gameData().catalog.filter(([, white, gold]) => matches(white) || matches(gold));
  return {
    hits: match
      .filter(([group]) => !have.has(group))
      .slice(0, SHOWN_MAX)
      .map(([group, white, gold]) => ({ group, white, gold, viaGold: matches(gold) && !matches(white) })),
    have: match
      .filter(([group]) => have.has(group))
      .slice(0, 3)
      .map(([group, white]) => ({ group, white })),
    none: !match.length,
  };
}
export const addTarget = (group: number) => setTargets([...settings.value.targets, { group }]);
// at: the target's place
export const removeTargetAt = (at: number) => setTargets(settings.value.targets.filter((_, j) => j !== at));
