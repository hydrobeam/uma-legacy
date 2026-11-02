// ---------- a veteran as the game's legacy list draws her (the pool's cards, uma.moe's listings) ----------
// Her portrait in its ring (her outfit's portrait, her head over the top, else her icon), her career-rank badge (top
// right, as in the game: the last rank whose minimum her score reaches), a label tab over her when she has a role ("Parent
// 1"), her three main sparks under it as the list's tags with their stars (blue: her stat, pink: her aptitude, green:
// her unique skill), her name and a line under it. Every part is sized from the portrait's width, --p (76 px, 54 on a
// narrow phone), at the game's measured proportions: the rank badge 0.36 P at the top right, the tags 0.36 P wide each
// with their bottoms on the portrait's, each tag's star row 0.86 of the tag right under it. The button is data-vet
// (data-sel while chosen); its parts data-part: tab, rank, tag, stars, name.
import { useState } from 'preact/hooks';
import { careerTables } from '../model/data.ts';
import { srcFor } from '../model/images.ts';
import { blueName, pinkName, uniqueName, umaName } from './names.ts';
import type { NodeRecord, VetRecord } from './store.ts';
import { UmaAv } from '../ui/Images.tsx';

// her main sparks' short tags, as the game's legacy list letters them
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
const first = (o: Readonly<Record<number, number>> | undefined) =>
  Object.entries(o || {}).find(([, stars]) => stars > 0);
/** one main spark: the legacy list's tag image (its lettering in it) and its star row, the letters alone where the
 *  image isn't shipped (the tags touch, as in the game) */
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
    <span
      class="mx-[calc(var(--p)*-0.012)] flex w-[calc(var(--p)*0.36)] flex-col items-center"
      title={`${full} ${stars}★`}
    >
      {face ? (
        <img class="block h-auto w-full" src={face} alt={label} data-part="tag" />
      ) : (
        <b class="text-caption px-0.75 py-0.5 leading-none font-black text-white">{label}</b>
      )}
      {row ? (
        <img class="mt-[calc(var(--p)*-0.035)] block h-auto w-[85.7%]" src={row} alt={`${stars}★`} data-part="stars" />
      ) : (
        <i class="text-caption text-vet-star not-italic">{'★'.repeat(stars)}</i>
      )}
    </span>
  );
}
/** her blue (stat), pink (aptitude) and green (unique) sparks under her portrait */
function MainSparks({ v }: { v: NodeRecord }) {
  const bl = first(v.blues),
    pk = first(v.pinks),
    gr = first(v.greens);
  return (
    <span class="relative z-1 mt-[calc(var(--p)*-0.18)] flex justify-center">
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
/** her outfit's portrait in its ring (245 of the 256 px icon, centred at x 127.5, y 144, on the face's circle), her
 *  icon if it fails (or she has none) */
function Face({ v }: { v: VetRecord }) {
  const portrait = v.outfit && srcFor('vet', v.outfit),
    [failed, setFailed] = useState(false);
  if (portrait && !failed)
    return (
      <img
        class="absolute top-[calc(var(--p)*-0.088)] left-[calc(var(--p)*-0.0225)] size-[calc(var(--p)*1.045)] max-w-none"
        src={portrait}
        alt=""
        loading="lazy"
        onError={() => setFailed(true)}
      />
    );
  return <UmaAv uma={v.chara} outfit={v.outfit} size={88} shape="vet" />;
}
export interface VetCellProps {
  v: VetRecord;
  /** the name under her */
  name: string;
  /** the button's accessible label */
  label: string;
  onClick: () => void;
  /** a role tab over her ("Parent 1") */
  tab?: string | undefined;
  /** a line under her name, and its tooltip */
  sub?: string | undefined;
  subTitle?: string | undefined;
  /** the pool's: her card's id, data-k, chosen mark and aria-current */
  id?: string;
  k?: string;
  sel?: boolean;
  current?: boolean;
}
export function VetCell({ v, name, label, onClick, tab, sub, subTitle, id, k, sel, current }: VetCellProps) {
  // her career-rank badge, top right of her portrait (the last rank whose minimum her score reaches)
  const rank = v.rank,
    rankId = rank
      ? ((careerTables().ranks || []) as number[]).reduce((at, min, i) => (rank >= min ? i + 1 : at), 0)
      : 0,
    badge = rankId ? srcFor('ui', `vrank_${rankId}`) : null;
  return (
    <button
      type="button"
      class="hover:bg-gm-bright/16 data-sel:not-hover:bg-gm-bright/26 data-sel:inset-ring-gm-green2 focus-visible:outline-accent relative grid min-w-0 cursor-pointer justify-items-center gap-0.5 rounded-xl py-1 text-center [--p:76px] focus-visible:outline-3 focus-visible:outline-offset-2 data-sel:inset-ring-2 max-sm:[--p:54px]"
      id={id}
      data-vet=""
      data-sel={sel ? '' : undefined}
      data-k={k}
      aria-haspopup="dialog"
      aria-label={label}
      aria-current={current}
      title={`${umaName(v.chara)} · ${name}`}
      onClick={onClick}
    >
      <span class="relative block size-(--p)">
        <Face key={`${v.chara}:${v.outfit}`} v={v} />
        {badge && (
          <img
            class="pointer-events-none absolute top-[calc(var(--p)*-0.04)] right-[calc(var(--p)*-0.015)] size-[calc(var(--p)*0.36)]"
            src={badge}
            alt="Rank"
            data-part="rank"
          />
        )}
        {tab && (
          <span
            class="art-vet-tab text-caption shadow-vet-tab absolute top-[calc(var(--p)*-0.1)] left-1/2 z-2 -translate-x-1/2 rounded-md px-1.75 py-px font-black whitespace-nowrap text-white"
            data-part="tab"
          >
            {tab}
          </span>
        )}
      </span>
      <MainSparks v={v} />
      <span class="text-caption text-gm-ink max-w-full truncate font-black" data-part="name">
        {name}
      </span>
      {sub && (
        <span class="text-ui text-gm-ink2 max-w-full truncate" title={subTitle}>
          {sub}
        </span>
      )}
    </button>
  );
}
