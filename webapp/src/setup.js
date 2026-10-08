// ---------- the setup panel both planner pages share (panels.template.html) ----------
// Target skills, Umas and Support cards: buttons in the setup panel, each opening its editor in a dialog; Done, Escape or
// a click on the backdrop closes it.
export function initSetup() {
  for (const [b, id] of [['o-skills', 'set-skills'], ['o-umas', 'set-umas'], ['o-cards', 'set-cards']]) {
    const d = document.getElementById(id), o = document.getElementById(b); if (!d || !o) continue;   // the loop planner shows Target skills in the open
    // the lists' fading foot (.more) is measured on scroll: a list shown only now is measured once it opens
    o.onclick = () => { d.showModal(); d.querySelectorAll('.roster, .cards').forEach(e => e.dispatchEvent(new Event('scroll'))); };
    d.addEventListener('click', (e) => { if (e.target === d) d.close(); });
    d.querySelectorAll('.dclose').forEach(x => { x.onclick = () => d.close(); });
  }
}
