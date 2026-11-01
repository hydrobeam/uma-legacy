// ---------- the setup both planner pages share (webapp/partials/): its editors, and the Umas and Support cards ----------
// dialogs. `setupViews` lists each editor with its mount point, and each entry renders them (behind a render guard: a
// render into a mount point is an effect, made by the entries only); the Umas and Support cards dialogs are two of
// them, each with its panel inside (Done, Escape or a click on the backdrop closes one). Their template-owned buttons
// open them (`setupButtons`), each with its count (`umasBadge`, `cardsBadge`, which the entries keep current).
import { signal } from '@preact/signals';
import type { ComponentChildren, VNode } from 'preact';
import { $ } from '../dom.ts';
import { settings } from '../settings.ts';
import { rosterCounts } from '../roster.ts';
import { cardsOwned } from '../cards.ts';
import { Dialog } from './Dialog.tsx';
import { RenderGuard } from './RenderGuard.tsx';
import { RosterPanel } from './Roster.tsx';
import { CardsPanel } from './Cards.tsx';
import { TargetSkills } from './Targets.tsx';
import { RankControl, ScenarioPicker } from './Scenario.tsx';

type Which = 'umas' | 'cards';
const openOne = signal<Which | null>(null);
// the Umas or the Support cards dialog opens (its button's click)
const openSetupDialog = (which: Which) => {
  openOne.value = which;
};
const closeSetupDialog = () => {
  openOne.value = null;
};
/** one of the two dialogs: its heading and Done, its panel (a section the panel draws into, behind its own guard) */
function SetupDialog({
  id,
  which,
  label,
  heading,
  panel,
  children,
}: {
  id: string;
  which: Which;
  label: string;
  heading: string;
  panel: { id: string; labelledBy: string; what: string };
  children: ComponentChildren;
}) {
  return (
    <Dialog id={id} cls="setdlg" label={label} open={openOne.value === which} onClose={closeSetupDialog}>
      <div class="dlg">
        <div class="row between dhead">
          <h3>{heading}</h3>
          <button class="btn dclose" type="button" onClick={closeSetupDialog}>
            Done
          </button>
        </div>
        <section class="panel" id={panel.id} aria-labelledby={panel.labelledBy}>
          <RenderGuard input={settings} what={panel.what}>
            {children}
          </RenderGuard>
        </section>
      </div>
    </Dialog>
  );
}
/** Each editor and its mount point (each entry renders them, guarded by the settings); `fadeMore`: the roster's and the
 *  card list's bottom edge fades while there is more below (the loop page). */
export const setupViews = ({ fadeMore = false } = {}): { id: string; what: string; view: VNode }[] => [
  { id: 'scen-dd', what: 'The scenario picker', view: <ScenarioPicker /> },
  { id: 'rank-ctl', what: 'The lineage rank', view: <RankControl /> },
  { id: 'skills-panel', what: 'The target skills', view: <TargetSkills /> },
  {
    id: 'set-umas-root',
    what: 'The Umas dialog',
    view: (
      <SetupDialog
        id="set-umas"
        which="umas"
        label="Umas"
        heading="Umas you own"
        panel={{ id: 'umas-panel', labelledBy: 'h-roster', what: 'The Umas panel' }}
      >
        <RosterPanel fadeMore={fadeMore} />
      </SetupDialog>
    ),
  },
  {
    id: 'set-cards-root',
    what: 'The Support cards dialog',
    view: (
      <SetupDialog
        id="set-cards"
        which="cards"
        label="Support cards"
        heading="Support cards you own"
        panel={{ id: 'cards-panel', labelledBy: 'h-deck', what: 'The Support cards panel' }}
      >
        <CardsPanel fadeMore={fadeMore} />
      </SetupDialog>
    ),
  },
];
/** the Umas and Support cards buttons (the template's) open their dialogs, each with its count drawn now */
export function setupButtons() {
  $('o-umas').onclick = () => openSetupDialog('umas');
  $('o-cards').onclick = () => openSetupDialog('cards');
  umasBadge(rosterCounts.peek());
  cardsBadge(cardsOwned.peek());
}
// the counts on the buttons: everything counts as owned until you say otherwise
export const umasBadge = (n: typeof rosterCounts.value) => {
  const b = $('o-umas');
  b.dataset.n = `${n.outfitsOwned}/${n.outfits}`;
  b.title = `${n.outfitsOwned} of ${n.outfits} outfits owned`;
};
export const cardsBadge = (n: typeof cardsOwned.value) => {
  const b = $('o-cards');
  b.dataset.n = `${n.owned}/${n.all}`;
  b.title = `${n.owned} of ${n.all} support cards owned`;
};
