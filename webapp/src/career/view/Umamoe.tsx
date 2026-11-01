// uma.moe's button beside Next career (UmButton, into #um-root) and its popup's contents (UmamoePanel, into #um-body):
// the key until one is saved, then the status, what was searched, Include max borrows, the order menu
// (ui/SortMenu.tsx), every listing as a card (ui/VetCell.tsx: a card opens her veteran popup), Refresh and Disconnect.
// What they show is umamoesetup.js's (its signals and its sync).
import { useRef } from 'preact/hooks';
import { $dialog } from '../../dom.ts';
import { vets } from '../../veterans/store.ts';
import { umaName } from '../../veterans/names.ts';
import {
  SORTS,
  forgetUmKey,
  hasKey,
  listings,
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
        $dialog('um-dlg').showModal();
        if (!hasKey.peek()) document.getElementById('um-key')?.focus();
      }}
    >
      {has || short ? `uma.moe · ${short || text}` : 'Borrow from uma.moe'}
    </button>
  );
}
const SORT_ITEMS = Object.entries(SORTS).map(([k, [label, hint]]) => ({ k, label, hint }));
/** the popup's contents */
export function UmamoePanel() {
  const { sig, text, min, n } = umStatus.value,
    has = hasKey.value,
    full = umFull.value,
    { by, reversed } = umSort.value,
    key = useRef<HTMLInputElement>(null);
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
