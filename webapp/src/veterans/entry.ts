// ---------- a veteran entered by hand, step by step (the parent picker's "A new one") ----------
// Her uma (picked in the parent picker), her sparks (the skills prompt), her pink (the colour picker), then each
// parent's uma (the uma picker), sparks and pink: what the game shows of her and her Legacy Origin. She joins the pool
// in the parent place she was entered for as soon as her uma is picked, and each step lands on her record at once. A
// step dismissed (Escape, Close, the colour picker shut with no pink) ends the entry, and she stays as far as it got:
// incomplete (complete.ts), left out of the jobs, until her popup's editors finish her.
import { batch } from '@preact/signals';
import type { Uma } from '../model/data.ts';
import { notEntered } from './complete.ts';
import { umaName } from './names.ts';
import { askSkills, openUmaPicker } from './pickers.ts';
import { currentSpark, openColour } from './sparks.ts';
import { newVet, parentRef, setParent, vetById, vetRef } from './store.ts';
import type { NodeRef, ParentPlace } from './store.ts';

/** one step: it opens its editor, and calls `next` once its choice has landed (never, when it is dismissed) */
type Step = (next: () => void) => void;
// the entry under way (a staleness token: the colour picker's step goes on in a later task, which a new entry voids)
let entries = 0;

// her sparks: the target skills at the stars chosen, or none
const sparksStep =
  (at: NodeRef): Step =>
  (next) => {
    const nd = at.node();
    if (!nd) return;
    askSkills(umaName(nd.chara), false, (sparks) => {
      if (sparks) at.edit(() => ({ sparks }));
      next();
    });
  };
// her pink, in the colour picker, named for `who`; once it has closed with her pink set, the next step, in a task of
// its own (the next editor opens after the picker has closed and given the focus back, not on top of it)
const pinkStep =
  (at: NodeRef, who: string, entry: number): Step =>
  (next) =>
    openColour(at, 'pink', {
      who,
      onClose: () =>
        setTimeout(() => {
          const nd = at.node();
          if (entry === entries && nd && currentSpark(nd, 'pink')) next();
        }),
    });
// parent j's uma: any but hers and her other parent's
const umaStep =
  (id: string, j: ParentPlace): Step =>
  (next) => {
    const her = vetById(id);
    if (!her) return;
    const other = her.parents[j === 0 ? 1 : 0],
      allowed = (u: Uma) => u.id !== her.chara && u.id !== other?.chara;
    openUmaPicker(
      `Parent ${j + 1} of ${umaName(her.chara)}: which uma?`,
      (chara) => {
        parentRef(id, j).edit(() => ({ chara, outfit: undefined }));
        next();
      },
      allowed,
    );
  };
// parent j's steps: her uma, her sparks, her pink
const parentSteps = (id: string, j: ParentPlace, entry: number): Step[] => {
  const at = parentRef(id, j);
  return [
    umaStep(id, j),
    sparksStep(at),
    (next) => pinkStep(at, `Parent ${j + 1}, ${umaName(at.node()?.chara)}`, entry)(next),
  ];
};

/** A new veteran of uma `chara` for parent place `k`: she joins the pool there now, and the entry asks for the rest. */
export function enterVeteran(chara: number, k: number): void {
  const entry = ++entries,
    v = batch(() => {
      const added = newVet({ chara, sparks: {}, pinks: {}, wins: [], parents: [notEntered(), notEntered()] }, 'hand');
      setParent(k, added.id, false);
      return added;
    }),
    at = vetRef(v.id);
  const steps: Step[] = [
    sparksStep(at),
    pinkStep(at, umaName(chara), entry),
    ...parentSteps(v.id, 0, entry),
    ...parentSteps(v.id, 1, entry),
  ];
  const from = (i: number) => steps[i]?.(() => from(i + 1));
  from(0);
}
