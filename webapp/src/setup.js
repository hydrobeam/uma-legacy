// ---------- the setup panel both planner pages share (panels.template.html) ----------
// Target skills, Umas and Support cards: buttons in the setup panel, each opening its editor in a dialog; Done, Escape or
// a click on the backdrop closes it.
import { backdropCloses } from './dom.js';
export function initSetup() {
  for (const [buttonId, dialogId] of [['o-skills', 'set-skills'], ['o-umas', 'set-umas'], ['o-cards', 'set-cards']]) {
    const dialog = document.getElementById(dialogId), opener = document.getElementById(buttonId);
    if (!dialog || !opener) continue;   // the loop planner shows Target skills in the open
    opener.onclick = () => dialog.showModal();
    backdropCloses(dialog);
    dialog.querySelectorAll('.dclose').forEach(done => { done.onclick = () => dialog.close(); });
  }
}
