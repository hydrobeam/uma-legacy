// The parent picker: a veteran for a parent place, a new one, or the place handed back to the planner. Open while
// dialogs.ts `pickerFor` names a place.
import { useLayoutEffect, useRef, useState } from 'preact/hooks';
import { settings } from '../../settings.ts';
import { vets, vetsRev, plan, changed, vetById, umaName, askSkills, newVet, drawUmas, filterUmas } from '../../vets.js';
import type { Veteran } from '../../vets.js';
import { Dialog } from '../../ui/Dialog.tsx';
import { UmaAv } from '../../ui/Images.tsx';
import { BORROW_CHOICES, borrowChoice, pairBlock } from '../job.js';
import { pickerFor, pickerClosed } from './dialogs.ts';

// why veteran v can't go in place k (or '')
const BLOCKED: Record<string, string> = {
  trainee: 'the trainee herself',
  same: 'same uma as the other parent',
  both: 'the other parent is borrowed',
};
function blockedFor(v: Veteran, k: number) {
  const otherId = plan.parents[1 - k];
  const why = pairBlock(plan.trainee, v, otherId && vetById(otherId), settings.value);
  return why === 'setting'
    ? `borrowed (Borrowed parent: ${BORROW_CHOICES[borrowChoice(settings.value)]})`
    : BLOCKED[why] || '';
}
/** one veteran: her face, name, number, targets carried, parents; why she can't go here, if so */
function VetChoice({ v, i, k }: { v: Veteran; i: number; k: number }) {
  vetsRev.value; // (it reads a signal, so a parent's redraw with the same props skips it: the edits in place say when)
  const why = blockedFor(v, k),
    cur = plan.parents[k] === v.id,
    tg = settings.value.targets.map((t) => t.group),
    sparks = v.sparks as Record<number, number>,
    have = tg.filter((g) => sparks[g]),
    stars = have.reduce((a, g) => a + sparks[g], 0);
  return (
    <button
      type="button"
      class={`vpt${cur ? ' cur' : ''}${v.borrowed ? ' bor' : ''}`}
      disabled={!!why}
      onClick={() => {
        plan.parents[k] = v.id;
        plan.fromTable = false; // (chosen by hand: it stays)
        pickerClosed();
        changed();
      }}
    >
      <div class="vpth">
        <UmaAv uma={v.chara} outfit={v.outfit} size={48} />
        <div>
          <div class="nm">{umaName(v.chara)}</div>
          <div class="sub">
            {v.lender
              ? `${v.lender.name}${cur ? ', chosen' : ''}`
              : `#${i + 1}${v.borrowed ? ', borrowed' : ''}${cur ? ', chosen' : ''}`}
          </div>
        </div>
      </div>
      <div class="vpts">{`${have.length} of ${tg.length} target skills, ${stars}★`}</div>
      <div class="vptg">
        Parents:{' '}
        {v.parents.map((g, j) => (
          // biome-ignore lint/suspicious/noArrayIndexKey: her two parents, in their fixed places
          <UmaAv key={j} uma={g.chara || 0} outfit={g.outfit} size={22} />
        ))}
        {` ${v.parents.map((g) => (g.chara ? umaName(g.chara) : 'unknown')).join(', ')}`}
      </div>
      {why && <div class="vptw">{`Can't: ${why}`}</div>}
    </button>
  );
}
const TABS = [
  ['vets', 'Your veterans'],
  ['new', 'A new one (any uma)'],
  ['auto', 'Let the planner pick'],
] as const;
type Tab = (typeof TABS)[number][0];
export function ParentPicker() {
  vetsRev.value;
  const k = pickerFor.value,
    lastK = useRef<number | null>(null), // the place it shows (closed: the last, as a dialog keeps its content)
    opening = useRef(0), // its openings so far: a tab chosen holds for the opening it was chosen in
    [chosenTab, setTab] = useState<[Tab, number] | null>(null),
    umas = useRef<HTMLDivElement>(null),
    filter = useRef<HTMLInputElement>(null),
    focusFilter = useRef(false),
    shownFor = useRef<number | null>(null); // the place it was opened for (null: closed)
  if (k !== null && shownFor.current !== k) opening.current++;
  if (k !== null) lastK.current = k;
  const place = lastK.current,
    tab: Tab = chosenTab && chosenTab[1] === opening.current ? chosenTab[0] : vets.length ? 'vets' : 'new';
  const close = pickerClosed;
  // A new one: every uma, filterable (vets.js drawUmas builds the list into its box; the filter hides, never rebuilds)
  useLayoutEffect(() => {
    const opened = shownFor.current !== k;
    shownFor.current = k;
    if (k === null || tab !== 'new' || !umas.current || !filter.current) return;
    if (opened || focusFilter.current) {
      filter.current.value = '';
      drawUmas(umas.current, '', (id: number) => {
        close();
        askSkills(
          umaName(id),
          false,
          (sp: Record<number, number> | null) => {
            const v = newVet(id, sp || {});
            plan.parents[k] = v.id;
            changed();
          },
          true,
        );
      });
    }
    if (focusFilter.current) filter.current.focus();
    focusFilter.current = false;
  });
  let headed = false;
  return (
    <Dialog id="vpick-dlg" labelledBy="vpick-title" open={k !== null} onClose={close}>
      <div class="dlg">
        <div class="row between dhead">
          <h3 id="vpick-title">{place === null ? 'Choose a parent' : `Choose parent ${place + 1}`}</h3>
          <button type="button" class="btn dclose" onClick={close}>
            Close
          </button>
        </div>
        <div class="seg" role="tablist" aria-label="Where the parent comes from">
          {TABS.map(([key, label]) => (
            <button
              key={key}
              type="button"
              role="tab"
              id={`vp-tab-${key}`}
              aria-selected={tab === key}
              aria-controls={`vp-${key}`}
              onClick={() => {
                focusFilter.current = key === 'new';
                setTab([key, opening.current]);
              }}
            >
              {label}
            </button>
          ))}
        </div>
        <div id="vp-vets" role="tabpanel" aria-labelledby="vp-tab-vets" hidden={tab !== 'vets'}>
          <div class="vpgrid" id="vp-list">
            {place !== null && !vets.length && (
              <p class="hint">No veterans yet: import them in the paddock below, or make a new one.</p>
            )}
            {place !== null &&
              vets.flatMap((v: Veteran, i: number) => {
                const head = v.src === 'umamoe' && !headed;
                if (head) headed = true;
                return [
                  head && (
                    <h4 key="umamoe" class="vpgh">
                      uma.moe
                    </h4>
                  ),
                  <VetChoice key={v.id} v={v} i={i} k={place} />,
                ];
              })}
          </div>
        </div>
        <div id="vp-new" role="tabpanel" aria-labelledby="vp-tab-new" hidden={tab !== 'new'}>
          <input
            ref={filter}
            type="search"
            id="vp-filter"
            placeholder="Filter by name"
            aria-label="Filter umas by name"
            onInput={(e) => umas.current && filterUmas(umas.current, e.currentTarget.value)}
          />
          <div class="umapick" id="vp-umas" ref={umas} />
        </div>
        <div id="vp-auto" role="tabpanel" aria-labelledby="vp-tab-auto" hidden={tab !== 'auto'}>
          <p>The planner tries every one of your veterans in this place and keeps the best.</p>
          <button
            class="btn primary"
            type="button"
            id="vp-auto-go"
            onClick={() => {
              if (k === null) return;
              plan.parents[k] = null;
              close();
              changed();
            }}
          >
            Let the planner pick this parent
          </button>
        </div>
      </div>
    </Dialog>
  );
}
