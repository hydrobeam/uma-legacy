// ---------- a veteran as the game's legacy list draws her ----------
// Her portrait in its ring (her head over the top), her career-rank badge, her three main sparks under it as the list's
// tags with their stars, a name under them; a label tab over her when she has a role (the game's "Trainee"). Used by
// the uma.moe popup's listings and by Veteran Umamusume.
import { render } from 'preact';
import { useState } from 'preact/hooks';
import { careerTables } from './data.js';
import { srcFor } from './images.js';
import { must } from './must.ts';
import { UmaAv } from './ui/Avatars.tsx';
import { umaName, blueName, pinkName, uniqueName } from './vets.js';
import type { Sparks, Veteran } from './vets.js';

// her main sparks as the game's legacy list shows them under a portrait: blue (her stat), pink (her aptitude), green
// (her unique skill), each a short tag of its colour with its stars under it
const BLUE_TAG: Record<string, string> = { 1: 'SPD', 2: 'STA', 3: 'PWR', 4: 'GUTS', 5: 'WIT' },
  PINK_TAG: Record<string, string> = {
    11: 'TURF',
    12: 'DIRT',
    21: 'FRONT',
    22: 'PACE',
    23: 'LATE',
    24: 'END',
    31: 'SPRT',
    32: 'MILE',
    33: 'MED',
    34: 'LONG',
  };
// (the legacy list's own tag image, its lettering included, and its star row; the letters alone where the image isn't
// shipped)
type TagProps = { kind: string; label: string; full: string; stars: number; image: string };
function Tag({ kind, label, full, stars, image }: TagProps) {
  const row = srcFor('ui', `fstar_${kind}_${Math.min(3, Math.max(1, stars))}`),
    face = srcFor('ui', image);
  return (
    <span class={`umsp ${kind}`} title={`${full} ${stars}★`}>
      {face ? <img class="umtagimg" src={face} alt={label} /> : <b>{label}</b>}
      {row ? <img class="umst" src={row} alt={`${stars}★`} /> : <i>{'★'.repeat(stars)}</i>}
    </span>
  );
}
const first = (o: Sparks | undefined) => Object.entries(o || {}).find(([, st]) => st > 0);
function MainSparks({ v }: { v: Veteran }) {
  const bl = first(v.blues),
    pk = first(v.pinks),
    gr = first(v.greens);
  return (
    <span class="umsps">
      {bl && (
        <Tag
          kind="blue"
          label={BLUE_TAG[bl[0]] || '?'}
          full={blueName(+bl[0])}
          stars={bl[1]}
          image={`ftag_blue_${bl[0]}`}
        />
      )}
      {pk && (
        <Tag
          kind="pink"
          label={PINK_TAG[pk[0]] || '?'}
          full={pinkName(+pk[0])}
          stars={pk[1]}
          image={`ftag_pink_${pk[0]}`}
        />
      )}
      {gr && (
        <Tag kind="green" label="UNIQ" full={uniqueName(+gr[0]) || 'Unique skill'} stars={gr[1]} image="ftag_green" />
      )}
    </span>
  );
}
// the name under her, the button's accessible label, what a click does, a role tab over her ("Parent 1"), a line under
// her name; in Veteran Umamusume also her element id, her key (`data-k`, the focus's way back to her) and whether she is
// the chosen one (`sel`, and `current` where she shows beside the list)
export type CellProps = {
  name: string;
  label: string;
  onClick: () => void;
  tab?: string | undefined;
  sub?: string;
  id?: string;
  k?: string;
  sel?: boolean;
  current?: boolean;
};
export function VetCell({ v, name, label, onClick, tab, sub, id, k, sel, current }: CellProps & { v: Veteran }) {
  // her outfit's portrait as the game's legacy list draws a veteran (in its ring, her head over the top), else her icon
  const portrait = v.outfit && srcFor('vet', v.outfit),
    [failed, setFailed] = useState(false);
  // her career-rank badge, top right of her portrait, as in the game (the last rank whose minimum her score reaches)
  const rank = v.rank,
    rankId = rank ? (careerTables().ranks || []).reduce((at, min, i) => (rank >= min ? i + 1 : at), 0) : 0,
    badge = rankId && srcFor('ui', `vrank_${rankId}`);
  return (
    <button
      type="button"
      class={sel ? 'umcell sel' : 'umcell'}
      id={id}
      data-k={k}
      aria-current={current === undefined ? undefined : current ? 'true' : 'false'}
      aria-haspopup="dialog"
      aria-label={label}
      title={`${umaName(v.chara)} · ${name}`}
      onClick={onClick}
    >
      <span class="umface">
        {portrait && !failed ? (
          <img class="umvet" src={portrait} alt="" loading="lazy" onError={() => setFailed(true)} />
        ) : (
          <UmaAv chara={v.chara} outfit={v.outfit} size={88} />
        )}
        {badge ? <img class="umrank" src={badge} alt="Rank" /> : null}
        {tab ? <span class="umtab">{tab}</span> : null}
      </span>
      <MainSparks v={v} />
      <span class="nm">{name}</span>
      {sub ? <span class="sub">{sub}</span> : null}
    </button>
  );
}
// the element version, for the el()-built views (uma.moe's listings): the component rendered into a box of its own
export function vetCell(v: Veteran, o: CellProps): HTMLElement {
  const box = document.createElement('div');
  render(<VetCell v={v} {...o} />, box);
  return must(box.firstElementChild as HTMLElement | null, 'VetCell renders her button');
}
