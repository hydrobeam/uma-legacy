// ---------- the Target skills panel: the Presets button, the adder, one tile per target, the Presets dialogs ----------
// The tiles follow the targets (and the scenario: a guaranteed gold shows as that gold). Drawn into the template's
// #skills-panel (`mountTargets`).
import { renderGuarded } from './RenderGuard.tsx';
import type { RefObject } from 'preact';
import { useLayoutEffect, useRef, useState } from 'preact/hooks';
import { $ } from '../dom.ts';
import { fields, settings } from '../settings.ts';
import { addTarget, namesOf, removeTargetAt, suggest } from '../targets.ts';
import { must } from '../must.ts';
import { SkillIcon } from './Images.tsx';
import { PresetDialogs, openPresets } from './Presets.tsx';
import { TargetTile, groupKeys } from './TargetTile.tsx';

// Add a target: typing two characters or more lists the skills that match. Enter adds the first; the arrows walk the
// list (Up from the first goes back to the box); Escape or a click outside the box and the list closes it.
function TargetAdder({ input }: { input: RefObject<HTMLInputElement | null> }) {
  const [typed, setTyped] = useState(''),
    [shown, setShown] = useState(false),
    list = useRef<HTMLDivElement>(null);
  const offer = suggest(typed, fields.targets.value);
  useLayoutEffect(() => {
    if (!shown) return;
    const onClick = (e: MouseEvent) => {
      if (!list.current?.contains(e.target as Node) && e.target !== input.current) setShown(false);
    };
    document.addEventListener('click', onClick);
    return () => document.removeEventListener('click', onClick);
  }, [shown, input]);
  const items = () => [...must(list.current, 'the list is mounted').querySelectorAll('button')];
  const add = (group: number) => {
    setTyped('');
    setShown(false);
    addTarget(group);
    input.current?.focus();
  };
  // the box's keys: Escape closes the list (not the dialog the editor sits in), Enter adds the first, Down enters it
  const boxKeys = (e: KeyboardEvent) => {
    if (e.key === 'Escape' && shown) {
      e.preventDefault();
      setShown(false);
    } else if (e.key === 'Enter') {
      if (shown && offer.hits.length) {
        e.preventDefault();
        add(offer.hits[0].group);
      }
    } else if (e.key === 'ArrowDown' && shown && offer.hits.length) {
      e.preventDefault();
      items()[0].focus();
    }
  };
  // a suggestion's keys: the arrows walk the list, Escape closes it back to the box
  const listKeys = (e: KeyboardEvent) => {
    const all = items(),
      at = all.indexOf(document.activeElement as HTMLButtonElement);
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      setShown(false);
      input.current?.focus();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (at < all.length - 1) all[at + 1].focus();
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      (at > 0 ? all[at - 1] : input.current)?.focus();
    }
  };
  return (
    <div class="adder">
      <input
        type="search"
        id="t-add"
        placeholder="Add a skill, e.g. Corner Recovery"
        aria-label="Add a target skill"
        autocomplete="off"
        ref={input}
        value={typed}
        onInput={(e) => {
          const text = e.currentTarget.value;
          setTyped(text);
          setShown(text.trim().length >= 2);
        }}
        onKeyDown={boxKeys}
      />
      <div class="sugg" id="t-sugg" ref={list} hidden={!shown}>
        {offer.hits.map((hit) => (
          <button key={hit.group} type="button" onClick={() => add(hit.group)} onKeyDown={listKeys}>
            <SkillIcon group={hit.group} gold={hit.viaGold} />
            {hit.white}
            {hit.viaGold ? <span class="tg">{` → ${hit.gold}`}</span> : null}
          </button>
        ))}
        {offer.have.map(({ group, white }) => (
          <p key={group} class="snote">{`${white} is already a target`}</p>
        ))}
        {offer.none ? <p class="snote">{`No skill matches “${typed.trim()}”`}</p> : null}
      </div>
    </div>
  );
}
// one compact tile per target, two per row: icon and name, remove. After a removal the focus goes to the next row's
// remove button, else the last row's, else the add box, so the keyboard doesn't fall back to the page top.
/** `addBox`: the adder's box */
function TargetList({ addBox }: { addBox: RefObject<HTMLInputElement | null> }) {
  const targets = fields.targets.value,
    keys = groupKeys(targets.map((t) => t.group)),
    box = useRef<HTMLDivElement>(null),
    refocus = useRef<number | null>(null);
  useLayoutEffect(() => {
    if (refocus.current === null) return;
    const removes = must(box.current, 'the list is mounted').querySelectorAll<HTMLElement>('.x');
    (removes[Math.min(refocus.current, removes.length - 1)] || addBox.current)?.focus();
    refocus.current = null;
  });
  return (
    <div class="targets" id="targets" ref={box}>
      {targets.length ? null : <p class="hint">No target skills yet. Add one above.</p>}
      {targets.map((t, at) => {
        const white = namesOf(t.group).w;
        return (
          <TargetTile key={keys[at]} group={t.group} id={`g-${t.group}`}>
            <button
              class="x"
              type="button"
              aria-label={`Remove ${white}`}
              title={`Remove ${white}`}
              onClick={() => {
                refocus.current = at;
                removeTargetAt(at);
              }}
            >
              ×
            </button>
          </TargetTile>
        );
      })}
    </div>
  );
}
function TargetSkills() {
  const addBox = useRef<HTMLInputElement>(null);
  return (
    <>
      <div class="row between">
        <h2 id="h-skills">Target skills</h2>
        <button class="btn" id="t-preset" type="button" aria-haspopup="dialog" onClick={openPresets}>
          Presets
        </button>
      </div>
      <TargetAdder input={addBox} />
      <TargetList addBox={addBox} />
      <PresetDialogs />
    </>
  );
}
export function mountTargets() {
  renderGuarded(<TargetSkills />, $('skills-panel'), 'The target skills', settings);
}
