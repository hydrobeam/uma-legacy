// ---------- the setup both planner pages share: its editors, the Umas and Support cards dialogs, the era switch ----------
// `setupViews` gives each editor behind its render guard, for the page's layout to draw in its place (the scenario
// picker, the lineage rank, the target skills), and the Umas and Support cards dialogs, each with its panel inside
// (Done, Escape or a click on the backdrop closes one). The layout's Umas and Support cards buttons open them
// (`openSetupDialog`), each with its count (`setupBadges`). `pickEra` is the era switch's press on both pages.
import { signal } from '@preact/signals';
import type { ComponentChildren } from 'preact';
import { ERA } from '../saved.ts';
import { settings, switchEra } from '../settings.ts';
import { rosterCounts } from '../roster.ts';
import { cardsOwned } from '../cards.ts';
import { eraScenario } from '../scenario.ts';
import { Dialog, DialogHead, DialogPanel } from './Dialog.tsx';
import { RenderGuard } from './RenderGuard.tsx';
import { RosterPanel } from './Roster.tsx';
import { CardsPanel } from './Cards.tsx';
import { TargetSkills } from './Targets.tsx';
import { RankControl, ScenarioPicker } from './Scenario.tsx';
import type { Badge } from './Shell.tsx';
import type { Era } from '../eras.ts';

type Which = 'umas' | 'cards';
const openOne = signal<Which | null>(null);
/** the Umas or the Support cards dialog opens (its button's click) */
export const openSetupDialog = (which: Which) => {
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
    <Dialog size="xl" id={id} label={label} open={openOne.value === which} onClose={closeSetupDialog}>
      <DialogPanel fit="full">
        <DialogHead title={heading} close="Done" onClose={closeSetupDialog} />
        <section class="panel" id={panel.id} aria-labelledby={panel.labelledBy}>
          <RenderGuard input={settings} what={panel.what}>
            {children}
          </RenderGuard>
        </section>
      </DialogPanel>
    </Dialog>
  );
}
/** an editor behind its guard, drawn again once the settings it failed on are replaced */
const guarded = (what: string, view: ComponentChildren) => (
  <RenderGuard input={settings} what={what}>
    {view}
  </RenderGuard>
);
/** The setup's editors and its two dialogs, for the page's layout; `fadeMore`: the roster's and the card list's bottom
 *  edge fades while there is more below (the loop page); `compact`: the scenario button is its column's width and
 *  shorter (the loop page). */
export const setupViews = ({ fadeMore = false, compact = false } = {}) => ({
  scenario: guarded('The scenario picker', <ScenarioPicker compact={compact} />),
  rank: guarded('The lineage rank', <RankControl compact={compact} />),
  targets: guarded('The target skills', <TargetSkills />),
  umasDialog: guarded(
    'The Umas dialog',
    <SetupDialog
      id="set-umas"
      which="umas"
      label="Umas"
      heading="Umas you own"
      panel={{ id: 'umas-panel', labelledBy: 'h-roster', what: 'The Umas panel' }}
    >
      <RosterPanel fadeMore={fadeMore} />
    </SetupDialog>,
  ),
  cardsDialog: guarded(
    'The Support cards dialog',
    <SetupDialog
      id="set-cards"
      which="cards"
      label="Support cards"
      heading="Support cards you own"
      panel={{ id: 'cards-panel', labelledBy: 'h-deck', what: 'The Support cards panel' }}
    >
      <CardsPanel fadeMore={fadeMore} />
    </SetupDialog>,
  ),
});
/** the counts on the Umas and Support cards buttons: everything counts as owned until you say otherwise */
export function setupBadges(): { umas: Badge; cards: Badge } {
  const u = rosterCounts.value,
    c = cardsOwned.value;
  return {
    umas: { n: `${u.outfitsOwned}/${u.outfits}`, title: `${u.outfitsOwned} of ${u.outfits} outfits owned` },
    cards: { n: `${c.owned}/${c.all}`, title: `${c.owned} of ${c.all} support cards owned` },
  };
}
/** the era switch pressed: the page reloads into `era` with this tab's settings (and the scenario it opens on), after
 *  `keep` saves what else the page holds (the career page's veterans) */
export function pickEra(era: Era, keep: () => void = () => undefined) {
  if (era === ERA) return;
  switchEra(era, eraScenario(era));
  keep();
  location.reload();
}
