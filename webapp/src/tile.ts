// A target as the game's skill tile, the target skills editor's (targets.js, which adds its remove button) and the
// Presets view's (presets.js, the tiles alone): here so presets.js takes it without importing targets.js, which
// imports presets.js.
import { gameData } from './data.js';
import { settings } from './settings.ts';
import { goldGiven } from './job.js';
import { skillIcon } from './images.js';

// a skill group's names: white, and gold where it has one
export const namesOf = (group: number) => gameData().skillNames.get(group) || { w: `Skill ${group}`, gd: null };
// a target as the game's skill tile: icon and name; a gold the scenario guarantees shows as that gold (It's On! for
// Ramp Up), in gold (the editor adds its remove button; the Presets view shows the tiles alone)
export function tileEl(group: number): HTMLDivElement {
  const names = namesOf(group);
  const tile = document.createElement('div');
  tile.className = 'tgt';
  tile.dataset.g = goldGiven(settings.value.scen, group) ? '2' : '0';
  const name = document.createElement('span');
  name.className = 'name';
  tile.appendChild(name);
  const gold = goldGiven(settings.value.scen, group) && !!names.gd;
  if (gold) tile.classList.add('gold');
  const icon = skillIcon(group, gold);
  if (icon) name.appendChild(icon);
  const label = (gold && names.gd) || names.w; // (gold: it has a gold name)
  name.append(label);
  if (label.length > 18) tile.classList.add('long'); // (the game scales a long name down to fit)
  if (gold) name.title = `Gold of ${names.w}, guaranteed by the scenario`; // a plain target's name is on the tile: no hover
  return tile;
}
