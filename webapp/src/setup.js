// ---------- the setup panel both planner pages share (panels.template.html) ----------
// Umas and Support cards: buttons in the setup panel, each opening its editor in a dialog; Done, Escape or a click on the
// backdrop closes it. Target skills has the same pair (o-skills, set-skills: panels.template.html) but both pages show
// its editor in the open, so neither page has them.
import { $, $dialog, backdropCloses } from './dom.js';
export function initSetup() {
  for (const [buttonId, dialogId] of [['o-skills', 'set-skills'], ['o-umas', 'set-umas'], ['o-cards', 'set-cards']]) {
    const dialog = $dialog(dialogId), opener = $(buttonId);
    if (!dialog || !opener) continue;   // Target skills: in the open on both pages
    opener.onclick = () => dialog.showModal();
    backdropCloses(dialog);
    dialog.querySelectorAll('.dclose').forEach(done => { done.onclick = () => dialog.close(); });
  }
}
