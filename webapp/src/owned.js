// ---------- owned umas and cards from umadump: Import in the Umas and Support cards dialogs (ui/OwnedImport.tsx) ----------
// Either dialog's Import takes card_data.json (owned outfits) or support_card_data.json (owned support cards), or both
// (umadump.js parseOwned). A file is the game's whole list of what you own, so it replaces this era's marks: what it
// lists is owned, everything else in the data not owned. Ids the data doesn't have (R cards it leaves out, another
// era's) are skipped.
import { gameData } from './data.js';
import { setOwned } from './settings.ts';
import { parseOwned } from './umadump.js';
import { must } from './must.ts';

/** @typedef {Parameters<typeof setOwned>[0]} OwnedMarks the marks the files set, saved once they are all read */
// Mark owned exactly the outfits in `owned` (outfit ids), and an uma the data lists without outfits when the file has an
// outfit of hers (character id = outfit id / 100), in `marks`. Returns how many of the file's ids the data has; 0
// changes nothing.
/** @param {Set<number>} owned @param {OwnedMarks} marks @returns {number} */
function markOutfits(owned, marks) {
  const outfitless = new Set(
      gameData()
        .umas.filter((u) => !u.outfits.length)
        .map((u) => u.id),
    ),
    outfitIds = new Set(gameData().umas.flatMap((u) => u.outfits.map((o) => o.id))),
    known = [...owned].filter((id) => outfitIds.has(id) || outfitless.has(Math.floor(id / 100))).length;
  if (!known) return 0;
  const charas = new Set([...owned].map((id) => Math.floor(id / 100)));
  /** @type {Record<number, 'out'>} */
  const roster = {};
  /** @type {Record<number, false>} */
  const outfits = {};
  for (const u of gameData().umas) {
    if (!u.outfits.length && !charas.has(u.id)) roster[u.id] = 'out';
    for (const o of u.outfits) if (!owned.has(o.id)) outfits[o.id] = false;
  }
  Object.assign(marks, { roster, outfits });
  return known;
}
// Mark owned exactly the support cards in `owned` (card ids), in `marks`. Returns how many of them the data has; 0
// changes nothing.
/** @param {Set<number>} owned @param {OwnedMarks} marks @returns {number} */
function markCards(owned, marks) {
  const known = gameData().cards.filter((c) => owned.has(c.id)).length;
  if (known)
    marks.cardsOff = Object.fromEntries(
      gameData()
        .cards.filter((c) => !owned.has(c.id))
        .map((c) => [c.id, false]),
    );
  return known;
}
// one file's import, as the dialog's status says it ("54 outfits owned (2 not in the data)."); throws when nothing in
// it is in the data
/** @param {string} text @param {OwnedMarks} marks @returns {string} */
function importOwned(text, marks) {
  const list = parseOwned(text),
    owned = new Set(list.ids),
    known = list.kind === 'cards' ? markCards(owned, marks) : markOutfits(owned, marks),
    noun = list.kind === 'cards' ? 'cards' : 'outfits';
  if (!known) throw new Error(`None of its ${noun} are in this data.`);
  return `${known} ${noun} owned${owned.size > known ? ` (${owned.size - known} not in the data)` : ''}.`;
}
// the picked files, each in turn, then the input emptied; returns what the panel's status says (it names a file that
// failed)
/** @param {HTMLInputElement} input @returns {Promise<string>} */
export async function importFiles(input) {
  const said = [];
  /** @type {OwnedMarks} */
  const marks = {};
  let changed = false;
  for (const file of must(input.files, 'a file input has its file list')) {
    try {
      said.push(importOwned(await file.text(), marks));
      changed = true;
    } catch (e) {
      said.push(`${file.name}: ${e.message}`);
    }
  }
  input.value = '';
  if (changed) setOwned(marks);
  return said.join(' ');
}
