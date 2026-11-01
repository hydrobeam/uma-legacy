// Export and Import of the veterans: the page's own export, an uma.moe tree, the game's export.
import { $, $dialog, $textarea } from '../dom.js';
import { ERA } from '../saved.ts';
import { settings } from '../settings.ts';
import { vets, places, importVets, isVeteran, asVeteran, addVets } from '../veterans/store.ts';
import { umaName } from '../veterans/names.js';
import { parseExport, parseTree } from '../umadump.js';
import { orNull } from '../must.ts';
import { Blocked, buildJob } from './job.js';
import { setVTab } from './view/poolState.ts';

// Export: the veterans as the page keeps them, the plan, and the solver job the planner sends for them (to replay a
// search outside the page); Import takes the file back
const EXPORT_APP = 'uma-legacy-veterans';
/** the page's own export, as far as Import reads it before it checks each veteran (`isVeteran`) */
interface OwnExport {
  app: string;
  era?: string;
  veterans: { src?: unknown }[];
}
export function exportVets(): void {
  // (settings that make no job: the export carries none; a bug building it is reported, and the veterans still export)
  const job = orNull(() => buildJob(settings.value), Blocked);
  // (uma.moe's listings are uma.moe's, not yours: left out, their trainers with them)
  const veterans = vets.value.filter((v) => v.src !== 'umamoe');
  const data = {
    app: EXPORT_APP,
    version: 1,
    era: ERA,
    exported: new Date().toISOString(),
    veterans,
    plan: places.value,
    job,
  };
  $textarea('vx-text').value = JSON.stringify(data);
  $('vx-msg').textContent = '';
  $dialog('vx-dlg').showModal();
}
// Copy: the clipboard API, else select the text and the old copy command, else leave it selected for Ctrl+C
export function copyExport(): void {
  const t = $textarea('vx-text'),
    msg = (m: string) => {
      $('vx-msg').textContent = m;
    };
  const fallback = (why: string) => {
    t.focus();
    t.select();
    let ok = false;
    try {
      ok = document.execCommand('copy');
    } catch {
      // the old copy command refused: ok stays false and the text stays selected
    }
    msg(ok ? 'Copied.' : `Could not copy${why ? ` (${why})` : ''}. The text is selected: press Ctrl+C (or ⌘C).`);
  };
  if (!navigator.clipboard?.writeText) {
    fallback('no clipboard access here');
    return;
  }
  navigator.clipboard.writeText(t.value).then(
    () => msg('Copied.'),
    (e) => fallback(e?.message || 'clipboard refused'),
  );
}
export function downloadExport(): void {
  const url = URL.createObjectURL(new Blob([$textarea('vx-text').value], { type: 'application/json' }));
  const a = document.createElement('a');
  a.href = url;
  a.download = `uma-legacy-veterans-${ERA}.json`;
  document.body.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
  $('vx-msg').textContent = 'Downloaded.';
}
function ownExport(text: string): OwnExport | null {
  try {
    const o = JSON.parse(text);
    return o && o.app === EXPORT_APP && Array.isArray(o.veterans) ? o : null;
  } catch {
    return null;
  }
}
export function importText(text: string): boolean {
  const own = ownExport(text);
  if (own) {
    if (own.era && own.era !== ERA) {
      $('v-msg').textContent = `Not imported: that export is of the ${$('era-' + own.era)?.textContent} data.`;
      return true;
    }
    const damaged = own.veterans.filter((v) => !isVeteran(v)).length;
    if (damaged) {
      $('v-msg').textContent =
        `Not imported: ${damaged} veteran${damaged === 1 ? '' : 's'} in that export ${damaged === 1 ? 'is' : 'are'} damaged (no id or two parents).`;
      return true;
    }
    // each keeps her source (one made by hand stays hand-made); an export from before sources were saved: imported
    importVets(
      own.veterans.map((v) => ({ ...asVeteran(v), src: typeof v.src === 'string' ? v.src : 'import' })),
      true,
    );
    setVTab('own');
    $('v-msg').textContent =
      `Imported ${own.veterans.length} veteran${own.veterans.length === 1 ? '' : 's'} from an export of this page.`;
    return true;
  }
  const tree = parseTree(text);
  if (tree) {
    if (!tree.length) {
      $('v-msg').textContent = 'No veteran in that uma.moe tree.';
      return false;
    }
    addVets(tree);
    const first = vets.value.indexOf(tree[0]) + 1;
    $('v-msg').textContent =
      `Added ${tree.map((v) => umaName(v.chara)).join(', ')} from an uma.moe tree, with her parents, as #${first}${tree.length > 1 ? `-#${first + tree.length - 1}` : ''} (marked yours: switch to Borrowed if she is a friend's).`;
    return true;
  }
  try {
    const r = importVets(parseExport(text)),
      counted = (k: number, word: string) => `${k} ${word}${k === 1 ? '' : 's'}`,
      said: string[] = [];
    if (r.added || r.updated)
      said.push(
        `Imported ${counted(r.added + r.updated, 'veteran')} of yours: ${[r.added && `${r.added} new`, r.updated && `${r.updated} already here, updated`].filter(Boolean).join(', ')}`,
      );
    if (r.borrowed)
      said.push(
        `${said.length ? 'r' : 'R'}eplaced the borrow list with ${counted(r.borrowed, 'borrowed veteran')}${r.dropped ? ` (it had ${r.dropped})` : ''}`,
      );
    setVTab(r.added || r.updated ? 'own' : 'bor');
    $('v-msg').textContent = `${said.join('; ')}.`;
    return true;
  } catch (e) {
    // (parseExport throws an Error with the page's message)
    $('v-msg').textContent = (e as Error).message;
    return false;
  }
}
