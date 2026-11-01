// The tree's node popup: the trainee, a parent place, a veteran (also the veterans dialog's pane), a grandparent.
import { $, el, lazyDialog } from '../../dom.js';
import { settings } from '../../settings.ts';
import { isOwned } from '../../job.js';
import { umaAv } from '../../images.js';
import { affTable as affTableOf } from '../../afftable.js';
import {
  vets,
  plan,
  changed,
  umaName,
  sparksEl,
  winsBtn,
  winsText,
  changeUma,
  removeVet,
  openUmaPicker,
} from '../../vets.js';
import { inspView, fitSparks } from '../../details.js';
import { must } from '../../must.ts';
import { idxOf, shown, parentAt, listed } from '../job.js';
import { btn } from './parts.js';
import { openParentPicker } from './ParentPicker.js';

function affTable(r) {
  const nm = (x) => x.name || umaName(x.chara);
  return affTableOf(
    r.parents
      .map((ix) => r.slots.find((x) => x.kind === 'parent' && x.legacy === ix))
      .filter(Boolean)
      .map((p) => ({
        face: umaAv(p.chara || 0, null, 30),
        name: nm(p),
        rel: p.rel,
        race: p.race,
        aff: p.aff,
        gps: r.slots
          .filter((x) => x.kind === 'grandparent' && x.legacy === p.legacy)
          .sort((a, b) => a.gp - b.gp)
          .map((g) => ({ face: umaAv(g.chara || 0, null, 24), name: nm(g), rel: g.rel, race: g.race, aff: g.aff })),
      })),
  );
}
// the node popup (game style): one dialog, refilled on every redraw while open
// nodeOpen, the node on show: {kind: 't'} the trainee, {kind: 'p', k} parent place k, {kind: 'g', k, j} parent k's
// parent j, {kind: 'v', id} a veteran of the pool
/** @typedef {{kind: 't' | 'p' | 'g' | 'v', k?: number, j?: number, id?: string}} NodeSpec */
/** @type {NodeSpec | null} */
let nodeOpen = null;
// the data-k of the tree's (or the pool's) button for a node
const nodeKey = (spec) =>
  spec.kind === 't'
    ? 'node:t'
    : spec.kind === 'p'
      ? `node:p${spec.k}`
      : spec.kind === 'v'
        ? `node:v${spec.id}`
        : `node:p${spec.k}g${spec.j}`;
export const nodeDialog = lazyDialog('gmdlg nodedlg', (dialog) => {
  dialog.id = 'node-dlg';
  dialog.setAttribute('aria-labelledby', 'node-title');
  const h = el('h3', 'gmdhdr');
  h.id = 'node-title';
  const body = el('div', 'gmdbody');
  body.id = 'node-body';
  const f = el('div', 'gmdfoot');
  const c = el('button', 'btn gmclose', 'Close');
  c.type = 'button';
  c.onclick = () => dialog.close();
  f.appendChild(c);
  dialog.append(h, body, f);
  dialog.addEventListener('close', () => {
    const sp = nodeOpen;
    nodeOpen = null;
    const key = sp && nodeKey(sp);
    const e = key && document.querySelector(`[data-k="${key}"]`);
    if (e && !document.querySelector('dialog[open]')) e.focus();
  });
});
// the node popup, closed if it is open
const closeNode = () => {
  const nodeDlg = nodeDialog.made();
  if (nodeDlg?.open) nodeDlg.close();
};
export function openNode(spec) {
  const nodeDlg = nodeDialog.get();
  nodeOpen = spec;
  fillNode();
  if (!nodeDlg.open) nodeDlg.showModal();
}
// a dialog of its own from the popup (the uma or parent picker): the popup closes first
const fromNode = (fn) => () => {
  closeNode();
  fn();
};
// Refill the popup for the node on show (`nodeOpen`): its title and body, and its actions in the footer. A node that is
// gone (a removed veteran, a grandparent whose parent place emptied) closes it instead.
export function fillNode() {
  const nodeDlg = nodeDialog.made();
  if (!nodeOpen || !nodeDlg) return;
  const r = shown(),
    body = $('node-body');
  body.textContent = '';
  const sp = nodeOpen;
  const acts = el('div', 'row tacts');
  const fill = { t: fillTraineeNode, p: fillParentNode, v: fillVeteranNode }[sp.kind] || fillGrandparentNode;
  if (!fill(sp, r, body, acts)) {
    nodeDlg.close();
    return;
  }
  // the popup's actions sit in its footer beside Close (no row of their own)
  const foot = must(nodeDlg.querySelector('.gmdfoot'), 'the popup is made with its footer');
  for (const e of foot.querySelectorAll('.nact')) e.remove();
  for (const b of [...acts.children]) {
    b.classList.add('nact');
    foot.appendChild(b);
  }
  fitSparks(nodeDlg);
}
// Each node kind's popup fills the title, `body` (its content) and `acts` (its actions); false: the node is gone.
// The trainee: who she is, her outfit, her affinity's breakdown; choose her or hand her back to the planner.
function fillTraineeNode(_sp, r, body, acts) {
  $('node-title').textContent = plan.trainee ? 'Trainee' : "Trainee · planner's pick";
  const id = plan.trainee || r?.trainee.id;
  if (id) {
    const w = el('div', 'twho'),
      mine = r && r.trainee.id === id;
    const who = el('div');
    who.append(el('div', 'nm', umaName(id)));
    if (mine && r.trainee.outfits[0]) who.append(el('div', 'ctt', r.trainee.outfits[0].label));
    if (mine && r.trainee.outfitNote) who.append(el('div', 'onote', r.trainee.outfitNote));
    w.append(umaAv(id, mine ? r.trainee.outfits[0]?.id : null, 72), who);
    body.appendChild(w);
  } else body.appendChild(el('p', 'tempty', 'The planner tries every uma you own.'));
  if (id && r && r.trainee.id === id) body.appendChild(affTable(r));
  acts.appendChild(
    btn(
      plan.trainee ? 'Change trainee' : 'Choose trainee',
      'primary',
      fromNode(() =>
        openUmaPicker(
          'Trainee: which uma?',
          (u) => {
            plan.trainee = u;
            plan.fromTable = false; // (chosen by hand: it stays)
            changed();
          },
          (u) => isOwned(u, settings.value),
        ),
      ),
    ),
  );
  if (plan.trainee)
    acts.appendChild(
      btn('Let the planner pick', '', () => {
        plan.trainee = null;
        changed();
      }),
    );
  return true;
}
// A parent place: the veteran in it (chosen or the planner's pick) with her sparks and her parents', all editable.
function fillParentNode({ k }, r, body, acts) {
  const at = parentAt(k, r);
  $('node-title').textContent = `Legacy ${k + 1} · parent ${k + 1}${at?.pick ? " · planner's pick" : ''}`;
  if (at) {
    const v = at.v,
      i = idxOf(v.id),
      w = el('div', 'twho');
    const pb = el('button', 'facebtn');
    pb.type = 'button';
    pb.append(umaAv(v.chara, v.outfit, 64));
    pb.title = 'Choose another parent';
    pb.setAttribute('aria-label', `Parent ${k + 1}: ${umaName(v.chara)}. Choose another parent`);
    pb.onclick = fromNode(() => openParentPicker(k));
    const nm = el('div', 'tnm');
    nm.append(
      nameCover(v),
      el(
        'div',
        'sub',
        v.lender
          ? `uma.moe · ${v.lender.name || 'trainer'}`
          : `Veteran #${i + 1}${v.src === 'hand' ? ', made here' : ''}${v.borrowed ? ', borrowed' : ''}`,
      ),
    );
    w.append(pb, nm, winsEl(v, umaName(v.chara), `p${k}`, !listed(v)));
    // borrowed from uma.moe: her trainer to follow in the game before the career starts
    if (v.lender) w.appendChild(copyIdBtn(v, 'sm'));
    body.append(w);
    // her Inspiration view, as the card's: her sparks, then her parents' (the trainee's grandparents), all editable
    const iv = el('div', 'ud udbare');
    iv.appendChild(
      inspView({
        d: v,
        key: `p${k}`,
        onEdit: changed,
        parentKey: (j) => `p${k}g${j}`,
        onEditParent: changed,
        header: false,
        readOnly: listed(v),
      }),
    );
    body.appendChild(iv);
  } else
    body.appendChild(el('p', 'tempty', vets.length >= 2 ? 'The planner picks from your veterans.' : 'No parent yet.'));
  acts.appendChild(
    btn(
      plan.parents[k] ? 'Change parent' : 'Choose parent',
      'primary big',
      fromNode(() => openParentPicker(k)),
    ),
  );
  if (plan.parents[k])
    acts.appendChild(
      btn('Let the planner pick', '', () => {
        plan.parents[k] = null;
        changed();
      }),
    );
  return true;
}
// her face: a button that changes who she is, or (a listed veteran) just her face
const faceEl = (nd, size, label, change) => {
  if (!change) return umaAv(nd.chara || 0, nd.outfit, size);
  const b = el('button', 'facebtn');
  b.type = 'button';
  b.append(umaAv(nd.chara || 0, nd.outfit, size));
  b.title = `${label}: ${umaName(nd.chara)}. Change who she is`;
  b.setAttribute('aria-label', b.title);
  b.onclick = fromNode(() => changeUma(nd, label));
  return b;
};
// her G1 wins: the button that edits them, or (a listed veteran) their count
const winsEl = (nd, who, key, edit) => (edit ? winsBtn(nd, who, key) : el('span', 'vwins', winsText(nd)));
// the trainer to follow in the game for an uma.moe veteran: a Copy trainer ID button
function copyIdBtn(v, cls = '') {
  const b = btn('Copy ID', cls, async () => {
    try {
      await navigator.clipboard.writeText(v.lender.id);
      b.textContent = 'Copied';
    } catch {
      b.textContent = v.lender.id;
    }
  });
  b.title = `${v.lender.name}: ${v.lender.id}`;
  return b;
}
// her name with how many of the targets her own sparks carry, "6/8" (an n/n veteran carries all n)
function nameCover(v) {
  const groups = settings.value.targets.map((t) => t.group),
    x = groups.filter((g) => (v.sparks?.[g] || 0) > 0).length,
    n = el('div', 'nm', umaName(v.chara)),
    c = el('span', 'vxy', `${x}/${groups.length}`);
  c.title = `Her sparks carry ${x} of your ${groups.length} targets`;
  n.append(' ', c);
  return n;
}
// (her trainer's followers too: at 1,000 the veteran can't be borrowed by following)
const lenderText = (l) =>
  `uma.moe · ${l.name || 'trainer'} · ${l.followers >= 1000 ? 'max followers' : `${l.followers} follower${l.followers === 1 ? '' : 's'}`}`;
// where a veteran came from (entered from a career here, an uma.moe tree, an import, made here)
const VET_SOURCE = {
  entered: 'Entered here',
  simulated: 'Entered here',
  tree: 'From uma.moe',
  umamoe: 'From uma.moe',
  import: 'Imported',
  hand: 'Made here',
  sample: 'Sample',
};
// A veteran of yours (the pool, Career Info's legacies): her sparks and her parents', editable in place.
function fillVeteranNode({ id }, _r, body) {
  const title = veteranView(id, body);
  if (title) $('node-title').textContent = title;
  return !!title;
}
// her view (the popup's, and beside the list in Veteran Umamusume): her face, name, targets carried, wins, where she
// came from; Yours / Guest and Remove; her sparks and her parents'. Returns the title, '' when she is gone
export function veteranView(id, body) {
  const i = idxOf(id),
    v = vets[i];
  if (!v) return '';
  const edit = !listed(v),
    w = el('div', 'twho');
  const src = VET_SOURCE[v.src];
  w.append(
    faceEl(v, 56, `Veteran #${i + 1}`, edit),
    nameCover(v),
    winsEl(v, `Veteran #${i + 1}, ${umaName(v.chara)}`, `v${v.id}`, edit),
  );
  if (src)
    w.appendChild(
      el('span', v.src === 'tree' || v.src === 'umamoe' ? 'srctag' : 'simtag', v.lender ? lenderText(v.lender) : src),
    );
  if (v.lender) w.appendChild(copyIdBtn(v, 'sm'));
  body.appendChild(w);
  // yours or borrowed (she moves to that tab of Veteran Umamusume), and Remove
  if (edit) {
    const row = el('div', 'row vctl'),
      seg = el('div', 'seg sm');
    seg.setAttribute('role', 'group');
    seg.setAttribute('aria-label', `Veteran #${i + 1}: yours or a guest`);
    for (const [b, l] of /** @type {[boolean, string][]} */ ([
      [false, 'Yours'],
      [true, 'Guest'],
    ])) {
      const x = el('button', '', l);
      x.type = 'button';
      x.setAttribute('aria-pressed', String(v.borrowed === b));
      x.dataset.k = `v${v.id}:own${b ? 'bor' : ''}`;
      x.onclick = () => {
        if (v.borrowed === b) return;
        v.borrowed = b;
        changed();
      };
      seg.appendChild(x);
    }
    row.append(
      seg,
      btn('Remove', 'vdel', () => {
        const index = idxOf(v.id);
        if (index < 0) return;
        closeNode();
        removeVet(index);
        $('v-msg').textContent = `Removed ${umaName(v.chara)} from your veterans.`;
      }),
    );
    body.appendChild(row);
  }
  const iv = el('div', 'ud udbare');
  iv.appendChild(
    inspView({
      d: v,
      key: `v${v.id}`,
      onEdit: changed,
      parentKey: (j) => `v${v.id}g${j}`,
      onEditParent: changed,
      header: false,
      readOnly: !edit,
    }),
  );
  body.appendChild(iv);
  // her parents: who they are and their G1 wins
  const pr = el('div', 'vpars');
  v.parents.forEach((g, j) => {
    const x = el('div', 'twho'),
      lab = `Parent ${j + 1} of #${i + 1}`;
    x.append(
      faceEl(g, 36, lab, edit),
      el('span', 'nm', g.chara ? umaName(g.chara) : 'Unknown parent'),
      winsEl(g, `${lab}, ${umaName(g.chara)}`, `v${v.id}g${j}`, edit),
    );
    pr.appendChild(x);
  });
  body.appendChild(pr);
  return v.lender ? `uma.moe · ${v.lender.name || 'trainer'}` : `Veteran #${i + 1}${v.borrowed ? ' · guest' : ''}`;
}
// A grandparent (parent k's parent j): who she is, her sparks and G1 wins.
function fillGrandparentNode({ k, j }, r, body) {
  const at = parentAt(k, r),
    g = at?.v.parents[j];
  $('node-title').textContent = `Legacy ${k + 1} · grandparent`;
  if (!g) return false;
  const edit = !listed(at.v),
    w = el('div', 'twho');
  w.append(
    faceEl(g, 56, 'Grandparent', edit),
    el('div', 'nm', g.chara ? umaName(g.chara) : 'Unknown: pick her'),
    winsEl(g, umaName(g.chara), `p${k}g${j}`, edit),
  );
  const iv = el('div', 'ud udbare');
  iv.appendChild(
    inspView({ d: { ...g, parents: [] }, key: `p${k}g${j}`, onEdit: changed, header: false, readOnly: !edit }),
  );
  // (a copy without parents for the view; her sparks object is hers: edits land on her)
  if (edit)
    must(iv.querySelector('.simskills'), 'inspView() draws her sparks').replaceChildren(
      sparksEl(g, `p${k}g${j}`, changed),
    );
  body.append(w, iv);
  return true;
}
