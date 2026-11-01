// ---------- a target as the game's skill tile: icon and name (the target skills editor's and the Presets view's) ----------
// A gold the chosen scenario guarantees shows as that gold (It's On! for Ramp Up), in gold. The editor puts its remove
// button in as `children`; the Presets view shows the tiles alone.
import type { ComponentChildren } from 'preact';
import { fields } from '../settings.ts';
import { goldGiven } from '../job.js';
import { namesOf } from '../targets.js';
import { SkillIcon } from './Images.tsx';

export function TargetTile({ group, id, children }: { group: number; id?: string; children?: ComponentChildren }) {
  const names = namesOf(group),
    given = goldGiven(fields.scen.value, group),
    gold = given && !!names.gd,
    label = (gold && names.gd) || names.w;
  return (
    // (the game scales a long name down to fit: `long`)
    <div class={`tgt${gold ? ' gold' : ''}${label.length > 18 ? ' long' : ''}`} id={id} data-g={given ? '2' : '0'}>
      {/* a plain target's name is on the tile: no hover. Keyed by gold: Preact sets `title` as a property, so a title
          taken away would stay as title="" */}
      <span
        key={gold ? 'gold' : 'white'}
        class="name"
        title={gold ? `Gold of ${names.w}, guaranteed by the scenario` : undefined}
      >
        <SkillIcon group={group} gold={gold} />
        {label}
      </span>
      {children}
    </div>
  );
}
