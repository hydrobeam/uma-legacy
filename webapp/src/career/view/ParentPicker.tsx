// The parent picker: a veteran for a parent place, a new one, or the place handed back to the planner. Open while
// dialogs.ts `pickerFor` names a place.
import { useLayoutEffect, useRef, useState } from 'preact/hooks';
import { settings } from '../../settings.ts';
import { batch } from '@preact/signals-core';
import { vets, places, vetById, newVet, setParent } from '../../veterans/store.ts';
import { starsOf, umaName } from '../../veterans/names.ts';
import { askSkills } from '../../veterans/pickers.ts';
import { UmaList } from '../../veterans/Editors.tsx';
import type { VetRecord } from '../../veterans/store.ts';
import { Dialog } from '../../ui/Dialog.tsx';
import { UmaAv } from '../../ui/Images.tsx';
import { BORROW_CHOICES, borrowChoice, pairBlock } from '../job.ts';
import { pickerFor, pickerClosed } from './dialogs.ts';
import { Guard } from './Guard.tsx';

// why veteran v can't go in place k (or '')
const BLOCKED: Record<string, string> = {
  trainee: 'the trainee herself',
  same: 'same uma as the other parent',
  both: 'the other parent is borrowed',
};
function blockedFor(v: VetRecord, k: number) {
  const plan = places.value,
    otherId = plan.parents[1 - k];
  const why = pairBlock(plan.trainee, v, otherId ? vetById(otherId) : null, settings.value);
  return why === 'setting'
    ? `borrowed (Borrowed parent: ${BORROW_CHOICES[borrowChoice(settings.value)]})`
    : BLOCKED[why] || '';
}
/** one veteran: her face, name, number, targets carried, parents; why she can't go here, if so */
function VetChoice({ v, i, k }: { v: VetRecord; i: number; k: number }) {
  const why = blockedFor(v, k),
    cur = places.value.parents[k] === v.id,
    tg = settings.value.targets.map((t) => t.group),
    have = tg.filter((g) => starsOf(v.sparks, g) > 0),
    stars = have.reduce((a, g) => a + starsOf(v.sparks, g), 0);
  return (
    <button
      type="button"
      class={`vpt${cur ? ' cur' : ''}${v.borrowed ? ' bor' : ''}`}
      disabled={!!why}
      onClick={() =>
        batch(() => {
          pickerClosed();
          setParent(k, v.id, true); // (chosen by hand: it stays)
        })
      }
    >
      <div class="vpth">
        <UmaAv key={`${v.chara}:${v.outfit}`} uma={v.chara} outfit={v.outfit} size={48} />
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
          // biome-ignore lint/suspicious/noArrayIndexKey: her two parents, in their fixed places, each keyed by her uma too
          <UmaAv key={`${j}:${g.chara}:${g.outfit}`} uma={g.chara || 0} outfit={g.outfit} size={22} />
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
  const k = pickerFor.value,
    lastK = useRef<number | null>(null), // the place its title names (closed: the last)
    opening = useRef(0), // its openings so far: a tab chosen holds for the opening it was chosen in
    [chosenTab, setTab] = useState<[Tab, number] | null>(null),
    [typed, setTyped] = useState<[string, number]>(['', 0]), // A new one's filter, and the opening it was typed in
    filter = useRef<HTMLInputElement>(null),
    focusFilter = useRef(false),
    shownFor = useRef<number | null>(null); // the place it was opened for (null: closed)
  if (k !== null && shownFor.current !== k) opening.current++;
  if (k !== null) lastK.current = k;
  const place = lastK.current,
    tab: Tab = chosenTab && chosenTab[1] === opening.current ? chosenTab[0] : vets.value.length ? 'vets' : 'new';
  const close = pickerClosed;
  // A new one: every uma, filterable (Editors.tsx UmaList; the filter hides, never rebuilds), drawn while on show; each
  // opening, and each click on its tab, starts with an empty filter
  const newFilter = typed[1] === opening.current ? typed[0] : '';
  const pickNew = (id: number) => {
    if (k === null) return;
    close();
    askSkills(
      umaName(id),
      false,
      (sp: Record<number, number> | null) =>
        batch(() => {
          const v = newVet(id, sp || {});
          setParent(k, v.id, false);
        }),
      true,
    );
  };
  useLayoutEffect(() => {
    shownFor.current = k;
    if (focusFilter.current) filter.current?.focus();
    focusFilter.current = false;
  });
  let headed = false;
  return (
    <Dialog id="vpick-dlg" labelledBy="vpick-title" open={k !== null} onClose={close}>
      {/* (inside the dialog: a throw leaves it open, and Escape still closes it) */}
      <Guard what="The parent picker" also={pickerFor}>
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
                  if (key === 'new') setTyped(['', opening.current]);
                  setTab([key, opening.current]);
                }}
              >
                {label}
              </button>
            ))}
          </div>
          <div id="vp-vets" role="tabpanel" aria-labelledby="vp-tab-vets" hidden={tab !== 'vets'}>
            <div class="vpgrid" id="vp-list">
              {/* (its rows only while open: a closed picker draws, and redraws, nothing) */}
              {k !== null && !vets.value.length && (
                <p class="hint">No veterans yet: import them in the paddock below, or make a new one.</p>
              )}
              {k !== null &&
                vets.value.flatMap((v: VetRecord, i: number) => {
                  const head = v.src === 'umamoe' && !headed;
                  if (head) headed = true;
                  return [
                    head && (
                      <h4 key="umamoe" class="vpgh">
                        uma.moe
                      </h4>
                    ),
                    <VetChoice key={v.id} v={v} i={i} k={k} />,
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
              value={newFilter}
              onInput={(e) => setTyped([e.currentTarget.value, opening.current])}
            />
            <div class="umapick" id="vp-umas">
              {k !== null && tab === 'new' && <UmaList filter={newFilter} onPick={pickNew} />}
            </div>
          </div>
          <div id="vp-auto" role="tabpanel" aria-labelledby="vp-tab-auto" hidden={tab !== 'auto'}>
            <p>The planner tries every one of your veterans in this place and keeps the best.</p>
            <button
              class="btn primary"
              type="button"
              id="vp-auto-go"
              onClick={() => {
                if (k === null) return;
                batch(() => {
                  close();
                  setParent(k, null, false);
                });
              }}
            >
              Let the planner pick this parent
            </button>
          </div>
        </div>
      </Guard>
    </Dialog>
  );
}
