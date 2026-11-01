// ---------- a veteran as the game's legacy list draws her: the component twin of vetcell.js's vetCell ----------
// The markup vetCell draws (vetcell.js says what each part is): her portrait in its ring (her outfit's, else her icon),
// her career-rank badge, a role tab, her three main sparks as the list's tags, her name and a line under it. vetCell
// stays for the uma.moe popup's listings (step 11); an edit to one goes to the other.
import { useState } from 'preact/hooks';
import { careerTables } from '../data.js';
import { srcFor } from '../images.js';
import { BLUE_TAG, PINK_TAG } from '../vetcell.js';
import { blueName, pinkName, uniqueName, umaName } from '../veterans/names.js';
import type { NodeRecord, VetRecord } from '../veterans/store.ts';
import { UmaAv } from './Images.tsx';

const first = (o: Readonly<Record<number, number>> | undefined) =>
  Object.entries(o || {}).find(([, stars]) => stars > 0);
/** one main spark: the legacy list's tag image (its lettering in it) and its star row, the letters alone where the
 *  image isn't shipped */
function Tag({
  kind,
  label,
  full,
  stars,
  image,
}: {
  kind: string;
  label: string;
  full: string;
  stars: number;
  image: string;
}) {
  const face = srcFor('ui', image),
    row = srcFor('ui', `fstar_${kind}_${Math.min(3, Math.max(1, stars))}`);
  return (
    <span class={`umsp ${kind}`} title={`${full} ${stars}★`}>
      {face ? <img class="umtagimg" src={face} alt={label} /> : <b>{label}</b>}
      {row ? <img class="umst" src={row} alt={`${stars}★`} /> : <i>{'★'.repeat(stars)}</i>}
    </span>
  );
}
/** her blue (stat), pink (aptitude) and green (unique) sparks under her portrait */
function MainSparks({ v }: { v: NodeRecord }) {
  const bl = first(v.blues),
    pk = first(v.pinks),
    gr = first(v.greens);
  return (
    <span class="umsps">
      {bl && (
        <Tag
          kind="blue"
          label={BLUE_TAG[+bl[0] as keyof typeof BLUE_TAG] || '?'}
          full={blueName(+bl[0])}
          stars={bl[1]}
          image={`ftag_blue_${bl[0]}`}
        />
      )}
      {pk && (
        <Tag
          kind="pink"
          label={PINK_TAG[+pk[0] as keyof typeof PINK_TAG] || '?'}
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
/** her outfit's portrait in its ring, her icon if it fails (or she has none) */
function Face({ v }: { v: VetRecord }) {
  const portrait = v.outfit && srcFor('vet', v.outfit),
    [failed, setFailed] = useState(false);
  if (portrait && !failed)
    return <img class="umvet" src={portrait} alt="" loading="lazy" onError={() => setFailed(true)} />;
  return <UmaAv uma={v.chara} outfit={v.outfit} size={88} />;
}
export interface VetCellProps {
  v: VetRecord;
  /** the name under her */
  name: string;
  /** the button's accessible label */
  label: string;
  onClick: () => void;
  /** a role tab over her ("Parent 1") */
  tab?: string;
  /** a line under her name */
  sub?: string;
  /** the pool's: her card's id, data-k, chosen mark and aria-current */
  id?: string;
  k?: string;
  sel?: boolean;
  current?: boolean;
}
export function VetCell({ v, name, label, onClick, tab, sub, id, k, sel, current }: VetCellProps) {
  // her career-rank badge, top right of her portrait (the last rank whose minimum her score reaches)
  const rank = v.rank,
    rankId = rank
      ? ((careerTables().ranks || []) as number[]).reduce((at, min, i) => (rank >= min ? i + 1 : at), 0)
      : 0,
    badge = rankId ? srcFor('ui', `vrank_${rankId}`) : null;
  return (
    <button
      type="button"
      class={sel ? 'umcell sel' : 'umcell'}
      id={id}
      data-k={k}
      aria-haspopup="dialog"
      aria-label={label}
      aria-current={current}
      title={`${umaName(v.chara)} · ${name}`}
      onClick={onClick}
    >
      <span class="umface">
        <Face key={`${v.chara}:${v.outfit}`} v={v} />
        {badge && <img class="umrank" src={badge} alt="Rank" />}
        {tab && <span class="umtab">{tab}</span>}
      </span>
      <MainSparks v={v} />
      <span class="nm">{name}</span>
      {sub && <span class="sub">{sub}</span>}
    </button>
  );
}
