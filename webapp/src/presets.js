// ---------- target skill presets: named sets of targets (the Target skills panel's Presets dialog) ----------
// settings.presets: [{name, groups: [skill group, ...]}], saved with the other shared settings (both pages, both data sets)
// and carried by Export. The 8/8 example comes preloaded (settings.ts DEFAULTS) and is a preset like any other.
// The list: per preset its name and count (a click opens its skills as the editor's tiles, with Rename), Load, Overwrite
// (with the targets now) and a trash can; under it, Save current skills opens a small popup for the name (offered as
// "Preset N+1", N the presets now). Starting from a preset is load, edit the targets, save under a new name.
// Here: the edits, each a name lookup (names are unique); the dialogs are ui/Presets.tsx.
import { PRESET, settings, setPresets, setTargets } from './settings.ts';

const NAME_MAX = 40;
// the same skills, in any order
/** @type {(a: readonly number[], b: readonly number[]) => boolean} */
export const sameSkills = (a, b) =>
  a.length === b.length && [...a].sort((x, y) => x - y).join() === [...b].sort((x, y) => x - y).join();
/** @type {(n: number) => string} */
export const skillCount = (n) => `${n} skill${n === 1 ? '' : 's'}`;
// an edit of the presets saves them without searching again; loading one changes the targets (a search)

/** @param {readonly number[]} groups */
export const loadPreset = (groups) => setTargets(groups.map((group) => ({ group })));
export const loadExample = () => loadPreset(PRESET); // the planner's "no targets" status offers the default eight directly

// imported presets join the player's (an import never deletes one): an identical one is skipped, a different one under
// a taken name comes in as "name (imported)"
/**
 * @typedef {import('./settings.ts').Preset} Preset
 * @param {import('./settings.ts').SettingsView['presets']|null|undefined} mine @param {Preset[]} theirs @returns {Preset[]}
 */
export function mergePresets(mine, theirs) {
  const out = (mine || []).map((p) => ({ name: p.name, groups: [...p.groups] }));
  for (const p of theirs) {
    const name = String(p.name).trim().slice(0, NAME_MAX),
      have = out.find((q) => q.name === name);
    if (have && sameSkills(have.groups, p.groups)) continue;
    // the name cut so the suffix keeps it within NAME_MAX (" (imported)" is 11 characters, " (imported N)" 15 for N up to 999)
    let unique = have ? `${name.slice(0, NAME_MAX - 11)} (imported)` : name;
    for (let i = 2; out.some((q) => q.name === unique); i++) unique = `${name.slice(0, NAME_MAX - 15)} (imported ${i})`;
    out.push({ name: unique, groups: [...p.groups] });
  }
  return out;
}

/** @type {(name: string, except?: string) => boolean} except: a preset whose own name doesn't count */
const nameTaken = (name, except) => settings.value.presets.some((p) => p.name !== except && p.name === name);
// the name offered: "Preset N+1", N the presets now (counting on past a name already taken)
export function suggestName() {
  let i = settings.value.presets.length + 1;
  while (nameTaken(`Preset ${i}`)) i++;
  return `Preset ${i}`;
}
const currentGroups = () => settings.value.targets.map((t) => t.group);
// The current targets as a new preset named `typed` (trimmed, cut to NAME_MAX). Returns what the dialog says: why not,
// or what was saved.
/** @param {string} typed @returns {{saved: false, why: string} | {saved: true, said: string}} */
export function saveCurrent(typed) {
  const name = typed.trim().slice(0, NAME_MAX);
  if (!name) return { saved: false, why: 'Give it a name.' };
  if (nameTaken(name))
    return { saved: false, why: `“${name}” is taken: pick another name, or use that preset's Overwrite.` };
  setPresets([...settings.value.presets, { name, groups: currentGroups() }]);
  return { saved: true, said: `Saved “${name}”.` };
}
/** preset `name`'s skills become the current targets; what the list says @param {string} name @returns {string} */
export function overwritePreset(name) {
  const current = currentGroups();
  setPresets(settings.value.presets.map((p) => (p.name === name ? { ...p, groups: current } : p)));
  return `Overwrote “${name}” with your ${skillCount(current.length)}.`;
}
/** @param {string} name @returns {string} what the list says */
export function deletePreset(name) {
  setPresets(settings.value.presets.filter((p) => p.name !== name));
  return `Deleted “${name}”.`;
}
// Preset `from` renamed to `typed` (trimmed). Returns null when there is nothing to rename (empty, or its own name),
// else what the view says: why not, or that it was renamed.
/** @param {string} from @param {string} typed @returns {null | {renamed: false, why: string} | {renamed: true, name: string, said: string}} */
export function renamePreset(from, typed) {
  const name = typed.trim();
  if (!name || name === from) return null;
  if (nameTaken(name, from)) return { renamed: false, why: `“${name}” is taken: names are unique.` };
  setPresets(settings.value.presets.map((p) => (p.name === from ? { ...p, name } : p)));
  return { renamed: true, name, said: `Renamed “${from}” to “${name}”.` };
}
