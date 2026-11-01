// What a settings change does: it is saved, then the page reacts (the planner searches again; the career page too).
// The settings editors (targets, umas, cards, scenario) call save(); each page sets its reaction with onSave.
import { writeSaved } from './state.js';
let react = () => {
  // replaced by the page's onSave; until then a change is only saved
};
export function onSave(reaction) {
  react = reaction;
}
export const save = () => {
  react();
  writeSaved();
};
