// ---------- the setup both planner pages share (panels.template.html): its editors, and the Umas and Support cards ----------
// dialogs. `mountSetup` draws each editor into its mount point, and makes the Umas and Support cards buttons open their
// dialogs (Done, Escape or a click on the backdrop closes one), each button with its count. Called once, by each page's
// entry; each editor draws inside a render guard (ui/RenderGuard.tsx).
import { $, $dialog, backdropCloses } from '../dom.ts';
import { on } from '../store.ts';
import { rosterCounts } from '../roster.ts';
import { cardsOwned } from '../cards.ts';
import { mountRoster } from './Roster.tsx';
import { mountCards } from './Cards.tsx';
import { mountTargets } from './Targets.tsx';
import { mountRank, mountScenario } from './Scenario.tsx';

/** `fadeMore`: the roster's and the card list's bottom edge fades while there is more below (the loop page) */
export function mountSetup({ fadeMore = false } = {}) {
  mountScenario();
  mountRank();
  mountRoster(fadeMore);
  mountCards(fadeMore);
  mountTargets();
  for (const [buttonId, dialogId] of [
    ['o-umas', 'set-umas'],
    ['o-cards', 'set-cards'],
  ]) {
    const dialog = $dialog(dialogId),
      opener = $(buttonId);
    opener.onclick = () => dialog.showModal();
    backdropCloses(dialog);
    dialog.querySelectorAll<HTMLButtonElement>('.dclose').forEach((done) => {
      done.onclick = () => dialog.close();
    });
  }
  // the counts on the buttons: everything counts as owned until you say otherwise
  const umasBadge = (n: typeof rosterCounts.value) => {
    const b = $('o-umas');
    b.dataset.n = `${n.outfitsOwned}/${n.outfits}`;
    b.title = `${n.outfitsOwned} of ${n.outfits} outfits owned`;
  };
  const cardsBadge = (n: typeof cardsOwned.value) => {
    const b = $('o-cards');
    b.dataset.n = `${n.owned}/${n.all}`;
    b.title = `${n.owned} of ${n.all} support cards owned`;
  };
  umasBadge(rosterCounts.peek());
  cardsBadge(cardsOwned.peek());
  on([rosterCounts], umasBadge);
  on([cardsOwned], cardsBadge);
}
