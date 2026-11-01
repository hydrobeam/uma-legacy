// uma.moe's button beside Next career (UmButton, into #um-root) and its popup (UmamoeDialog, into #um-dlg-root, ui/Dialog):
// the key until one is saved, then the status, what was searched, Include max borrows, the order menu
// (ui/SortMenu.tsx), every listing as a card (ui/VetCell.tsx: a card opens her veteran popup), Refresh and Disconnect.
// What they show is umamoesetup.ts's (its signals and its sync).
import { computed } from '@preact/signals-core';
import { signal } from '@preact/signals-core';
import type { RefObject } from 'preact';
import { useLayoutEffect, useRef } from 'preact/hooks';
import { Dialog } from '../../ui/Dialog.tsx';
import { Guard } from './Guard.tsx';
import { vets } from '../../veterans/store.ts';
import { umaName } from '../../veterans/names.ts';
import {
  SORTS,
  forgetUmKey,
  hasKey,
  pickUmSort,
  saveUmKey,
  setUmFull,
  syncListings,
  umBusy,
  umFull,
  umSort,
  umStatus,
} from '../../umamoesetup.ts';
import { SortMenu } from '../../ui/SortMenu.tsx';
import { VetCell } from '../../ui/VetCell.tsx';
import { openNode } from './dialogs.ts';

const umOpen = signal(false);
const closeUm = () => {
  umOpen.value = false;
};
/** the popup: its heading and Close, its contents behind their guard; opened while no key is saved, the key field
 *  takes the focus (after its dialog's own layout effect has shown it) */
export function UmamoeDialog() {
  const open = umOpen.value,
    keyBox = useRef<HTMLInputElement>(null);
  // biome-ignore lint/correctness/useExhaustiveDependencies: once per opening (`open` is a module signal's value, which Biome reads as constant)
  useLayoutEffect(() => {
    if (open && !hasKey.peek()) keyBox.current?.focus();
  }, [open]);
  return (
    <Dialog id="um-dlg" cls="umdlg" labelledBy="um-title" open={open} onClose={closeUm}>
      <div class="dlg">
        <div class="row between dhead">
          <h3 id="um-title" title="Veterans other players list on uma.moe join your pool as borrows">
            uma.moe
          </h3>
          <button class="btn dclose" type="button" onClick={closeUm}>
            Close
          </button>
        </div>
        <div class="dlgscroll" id="um-body">
          <Guard what="uma.moe's popup">
            <UmamoePanel keyBox={keyBox} />
          </Guard>
        </div>
      </div>
    </Dialog>
  );
}
/** the button: shown once you have a veteran of your own (a career needs one of yours beside a borrow); it opens the
 *  popup, on the key field while no key is saved */
export function UmButton() {
  const { text, short } = umStatus.value,
    has = hasKey.value;
  return (
    <button
      class="btn"
      type="button"
      id="um-open"
      aria-haspopup="dialog"
      title="Veterans other players list on uma.moe join your pool as borrows"
      hidden={!vets.value.some((v) => !v.borrowed)}
      onClick={() => {
        umOpen.value = true;
      }}
    >
      {has || short ? `uma.moe · ${short || text}` : 'Borrow from uma.moe'}
    </button>
  );
}
const SORT_ITEMS = (['match', 'rank', 'affinity', 'wins', 'sparks', 'updated'] as const).map((k) => ({
  k,
  label: SORTS[k][0],
  hint: SORTS[k][1],
}));
/** the listings in the popup's order */
const listings = computed(() => {
  const { by, reversed } = umSort.value,
    score = SORTS[by][2],
    sign = reversed ? -1 : 1;
  return vets.value.filter((v) => v.src === 'umamoe').sort((x, y) => sign * (score(y) - score(x)));
});
/** the popup's contents; `keyBox`: the key field */
function UmamoePanel({ keyBox: key }: { keyBox: RefObject<HTMLInputElement | null> }) {
  const { sig, text, min, n } = umStatus.value,
    has = hasKey.value,
    full = umFull.value,
    { by, reversed } = umSort.value;
  const save = () => {
    const input = key.current;
    if (!input?.value.trim()) return;
    saveUmKey(input.value.trim());
    input.value = '';
  };
  return (
    <>
      <div id="um-keyrow" hidden={has}>
        <div class="umline">
          <input
            ref={key}
            type="password"
            id="um-key"
            autocomplete="off"
            spellcheck={false}
            placeholder="API key"
            aria-label="uma.moe API key"
            onKeyDown={(e) => {
              if (e.key === 'Enter') save();
            }}
          />
          <button class="btn" type="button" id="um-save" onClick={save}>
            Save
          </button>
        </div>
        <span class="hint umerr" id="um-err" role="status" aria-live="polite">
          {has ? '' : text}
        </span>
        <a href="https://uma.moe/settings" target="_blank" rel="noopener">
          Get a key at uma.moe/settings
        </a>
      </div>
      <div id="um-on" hidden={!has}>
        <div class="umline">
          <b id="um-msg" role="status" aria-live="polite">
            {text}
          </b>
          <button class="btn sm" type="button" id="um-again" disabled={umBusy.value} onClick={() => syncListings(true)}>
            Refresh
          </button>
        </div>
        <p class="hint" id="um-what">
          {sig ? `Parents with ${min}+ of your ${n} targets${full ? '' : ', from trainers you can follow'}` : ''}
        </p>
        <div class="row between">
          <label class="umtog" title="Trainers at 1,000 followers: their veterans can't be borrowed by following them">
            <input type="checkbox" id="um-full" checked={full} onChange={(e) => setUmFull(e.currentTarget.checked)} />
            {' Include max borrows'}
          </label>
          <SortMenu
            id="um-sort"
            cls="btn sm"
            what="order veterans"
            items={SORT_ITEMS}
            value={by}
            arrow={reversed ? '↑' : '↓'}
            title={`${SORTS[by][1]}${reversed ? ', reversed' : ''}`}
            onPick={pickUmSort}
          />
        </div>
        {/* each listing as the game's legacy list draws a veteran, with her trainer's name; a card opens her veteran
            popup (her sparks, her parents, her trainer's followers, Copy ID) */}
        <div class="umgrid" id="um-list">
          {listings.value.map((v) => (
            <VetCell
              key={v.id}
              v={v}
              name={v.lender?.name || umaName(v.chara)}
              label={`${umaName(v.chara)}, ${v.lender?.name || 'uma.moe'}: her sparks and parents`}
              onClick={() => openNode({ kind: 'v', id: v.id })}
            />
          ))}
        </div>
        <button class="linkbtn" type="button" id="um-forget" onClick={forgetUmKey}>
          Disconnect
        </button>
      </div>
    </>
  );
}
