// Runs inline right after the pre-rendered roster (the Global tiles in default order, written by the build), before the
// browser paints it: puts the tiles in the saved order with the saved owned / not owned / must include states, and moves
// the eager loading to the new first screenful. The page script (roster.js) then takes these tiles over as they are.
// In the GM era the tiles are cleared: that roster comes from the GM data, built by the page script.
import { compareTiles, tileState, tileLabel, EAGER } from './rostercore.js';
(function () {
  const box = document.getElementById('roster'); if (!box) return;
  let s = null; try { s = JSON.parse(localStorage.getItem('llp:settings') || 'null'); } catch (e) { s = null; }
  let gm = false; try { gm = s ? s.era === 'gm' : localStorage.getItem('llp:era') === 'gm'; } catch (e) {}
  if (gm) { box.textContent = ''; return; }
  const c = (s && s.common) || {}, g = (s && s.global) || {}, roster = g.roster || {}, outfits = g.outfits || {};
  const tiles = [...box.children];
  if ((c.rsort || 'rel') !== 'rel' || (c.rdir || 'desc') !== 'desc') {
    tiles.sort(compareTiles(c.rsort || 'rel', c.rdir || 'desc'));
    tiles.forEach((t, i) => { const im = t.querySelector('img'); if (!im) return;
      im.loading = i < EAGER ? 'eager' : 'lazy'; if (i < EAGER) im.fetchPriority = 'high'; else im.removeAttribute('fetchpriority'); });
    box.append(...tiles);
  }
  for (const t of tiles) {
    const st = tileState(roster, outfits, t.dataset.uma, t.dataset.o), b = t.firstElementChild;
    if (st === 'own') continue;
    t.dataset.s = st; const lab = tileLabel(t.querySelector('.un').textContent, t.querySelector('.uol') && t.querySelector('.uol').textContent, st);
    b.setAttribute('aria-label', lab); b.title = lab;
  }
})();
