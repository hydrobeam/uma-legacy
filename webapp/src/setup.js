// ---------- the setup panel both planner pages share (panels.template.html) ----------
// Target skills, Umas and Support cards: buttons in the setup panel, each opening its editor in a dialog; Done, Escape or
// a click on the backdrop closes it.
import { backdropCloses } from './dom.js';
export function initSetup() {
  for (const [b, id] of [['o-skills', 'set-skills'], ['o-umas', 'set-umas'], ['o-cards', 'set-cards']]) {
    const d = document.getElementById(id), o = document.getElementById(b); if (!d || !o) continue;   // the loop planner shows Target skills in the open
    o.onclick = () => d.showModal();
    backdropCloses(d);
    d.querySelectorAll('.dclose').forEach(x => { x.onclick = () => d.close(); });
  }
}
