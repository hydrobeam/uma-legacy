// ---------- owned umas and cards from umadump: Import in the Umas and Support cards dialogs ----------
// Either dialog's Import takes card_data.json (owned outfits) or support_card_data.json (owned support cards), or both
// (umadump.js parseOwned). A file is the game's whole list of what you own, so it replaces this era's marks: what it
// lists is owned, everything else in the data not owned. Ids the data doesn't have (R cards it leaves out, another
// era's) are skipped.
import { $, $input } from './dom.js';
import { gameData } from './data.js';
import { state } from './state.js';
import { save } from './save.js';
import { paintRoster } from './roster.js';
import { paintCards } from './cards.js';
import { parseOwned } from './umadump.js';

// Mark owned exactly the outfits in `owned` (outfit ids), and an uma the data lists without outfits when the file has an
// outfit of hers (character id = outfit id / 100). Returns how many of the file's ids the data has; 0 changes nothing.
/** @param {Set<number>} owned @returns {number} */
function setOwnedOutfits(owned) {
  const outfitless = new Set(
      gameData()
        .umas.filter((u) => !u.outfits.length)
        .map((u) => u.id),
    ),
    outfitIds = new Set(gameData().umas.flatMap((u) => u.outfits.map((o) => o.id))),
    known = [...owned].filter((id) => outfitIds.has(id) || outfitless.has(Math.floor(id / 100))).length;
  if (!known) return 0;
  const charas = new Set([...owned].map((id) => Math.floor(id / 100)));
  state.roster = {};
  state.outfits = {};
  for (const u of gameData().umas) {
    if (!u.outfits.length && !charas.has(u.id)) state.roster[u.id] = 'out';
    for (const o of u.outfits) if (!owned.has(o.id)) state.outfits[o.id] = false;
  }
  return known;
}
// Mark owned exactly the support cards in `owned` (card ids). Returns how many of them the data has; 0 changes nothing.
/** @param {Set<number>} owned @returns {number} */
function setOwnedCards(owned) {
  const known = gameData().cards.filter((c) => owned.has(c.id)).length;
  if (known)
    state.cardsOff = Object.fromEntries(
      gameData()
        .cards.filter((c) => !owned.has(c.id))
        .map((c) => [c.id, false]),
    );
  return known;
}
// one file's import, as the dialog's status says it ("54 outfits owned (2 not in the data)."); throws when nothing in
// it is in the data
/** @param {string} text @returns {string} */
function importOwned(text) {
  const list = parseOwned(text),
    owned = new Set(list.ids),
    known = list.kind === 'cards' ? setOwnedCards(owned) : setOwnedOutfits(owned),
    noun = list.kind === 'cards' ? 'cards' : 'outfits';
  if (!known) throw new Error(`None of its ${noun} are in this data.`);
  return `${known} ${noun} owned${owned.size > known ? ` (${owned.size - known} not in the data)` : ''}.`;
}
// the picked files, each in turn; the status names a file that failed
/** @param {HTMLInputElement} input @param {HTMLElement} status */
async function importFiles(input, status) {
  const said = [];
  let changed = false;
  for (const file of input.files) {
    try {
      said.push(importOwned(await file.text()));
      changed = true;
    } catch (e) {
      said.push(`${file.name}: ${e.message}`);
    }
  }
  input.value = '';
  if (changed) {
    save();
    paintRoster();
    paintCards();
  }
  status.textContent = said.join(' ');
}
export function initOwnedImport() {
  for (const panel of ['r', 'c']) {
    const input = $input(`${panel}-file`);
    input.onchange = () => importFiles(input, $(`${panel}-msg`));
  }
}
