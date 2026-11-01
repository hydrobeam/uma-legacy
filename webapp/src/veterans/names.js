// ---------- the names the career page shows: umas, skills, sparks ----------
import { careerTables, gameData } from '../data.js';

/** @type {(id: number|null|undefined) => string} */
export const umaName = (id) => (id ? gameData().names.get(id) || `Uma ${id}` : 'Unknown');
/** @type {(g: number) => string} */
export const skillName = (g) => (gameData().skillNames.get(g) || { w: `Skill ${g}` }).w;
/** @type {(g: number|string) => string} */
export const pinkName = (g) => careerTables().pinks[g] || `Aptitude ${g}`;
/** @type {(g: number|string) => string} */
export const blueName = (g) =>
  careerTables().blues?.[g] || ['', 'Speed', 'Stamina', 'Power', 'Guts', 'Wit'][g] || `Stat ${g}`;
/** @type {(g: number|string) => string|null} */
export const uniqueName = (g) => careerTables().uniques?.[g] || null;
