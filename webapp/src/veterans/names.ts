// ---------- the names the career page shows (umas, skills, sparks), a spark's stars, the target skills' groups ----------
import { careerTables, gameData } from '../model/data.ts';
import type { SettingsView } from '../settings.ts';
import type { DeepReadonly } from '../store.ts';
import type { Sparks } from './store.ts';

/** a spark's stars in `sparks` (0: she has none of it) */
export const starsOf = (sparks: DeepReadonly<Sparks>, group: number): number => sparks[group] ?? 0;

export const umaName = (id: number | null | undefined): string =>
  id ? gameData().names.get(id) || `Uma ${id}` : 'Unknown';
export const skillName = (g: number): string => (gameData().skillNames.get(g) || { w: `Skill ${g}` }).w;
// A spark's group `g` is a number, or its string when read as an object's key (a node's `pinks`, `blues`, `greens`):
// both index the same entry, so the lookups below read it `as number`.
export const pinkName = (g: number | string): string => careerTables().pinks[g as number] || `Aptitude ${g}`;
export const blueName = (g: number | string): string =>
  careerTables().blues?.[g as number] || ['', 'Speed', 'Stamina', 'Power', 'Guts', 'Wit'][g as number] || `Stat ${g}`;
export const uniqueName = (g: number | string): string | null => careerTables().uniques?.[g as number] || null;
/** the target skills' groups, in the targets' order */
export const targetGroups = (targets: SettingsView['targets']): number[] => targets.map((t) => t.group);
