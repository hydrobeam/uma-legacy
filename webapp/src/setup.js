// ---------- the setup panel both planner pages share (panels.template.html) ----------
// Umas and Support cards: buttons in the setup panel, each opening its editor in a dialog; Done, Escape or a click on the
// backdrop closes it.
import { $, $dialog, backdropCloses } from './dom.js';
export function initSetup() {
  for (const [buttonId, dialogId] of [
    ['o-umas', 'set-umas'],
    ['o-cards', 'set-cards'],
  ]) {
    const dialog = $dialog(dialogId),
      opener = $(buttonId);
    opener.onclick = () => dialog.showModal();
    backdropCloses(dialog);
    dialog.querySelectorAll('.dclose').forEach((done) => {
      done.onclick = () => dialog.close();
    });
  }
}
