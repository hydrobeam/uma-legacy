// The veteran analysis (the veteran you got): the "Try a run" popup. The decision is per veteran: enter the one
// you got (she starts as a random result, drawn as Reroll draws, career/roll.mjs) and Compare runs both choices,
// running this career again or using her in the next one, on the plan's terms (expected careers to land).
// This module draws it from the analysis store (./state.ts `simulation`) and the answer store; Compare and Reroll are
// the comparison's run machine's (./compare.ts, made by the entry and handed in, `initAnalysis`). It also makes her
// draft from a career (`draftOf`, `rollOf`: the page's veterans and settings, for the machine's deps).
import { $, $dialog, el, fmtIn } from '../../dom.js';
import { careerTables } from '../../data.js';
import { settings } from '../../settings.ts';
import { umaAv, runnerHTML } from '../../images.js';
import { pct } from '../../odds.js';
import { vets, plan, changed, newVet, umaName, openWins } from '../../vets.js';
import { detailsEl, fitSparks } from '../../details.js';
import { scheduledRaces, rolledOutcome } from '../roll.mjs';
import { answer, runView } from '../state.ts';
import { keyOf } from '../keys.js';
import { shown, isFull } from '../job.js';
import { btn, sectionEl, deckEl, fmtN, tableRows } from '../view/parts.js';
import { openNode } from '../view/dialogs.ts';
import { must } from '../../must.ts';
import { simulation, draftEdited, draftAdded } from './state.ts';
/** @import { CareerReport } from '../job.js' */
/** @import { Draft } from './state.ts' */
/** @import { DeepReadonly } from '../../store.ts' */

/** @type {{compare: () => void, roll: () => void} | null} Compare and Reroll (the comparison's machine) */
let comparison = null;
/** @param {{compare: () => void, roll: () => void}} machine the comparison's machine (./compare.ts) */
export function initAnalysis(machine) {
  comparison = machine;
}

// the veteran analysis' G1 wins: the G1 wins editor of the pool (a win in its slot, click to remove; an empty slot lists the
// G1s she could enter then). Her schedule's races keep their win odds in the turn bar; a win shows at the turn picked for
// it, else the turn her schedule runs it. A race held in Classic and Senior is one win saddle (one race index).
function openDraftWins(d, r) {
  // the G1s she could enter, by turn (heldAt), and those her schedule runs, by race (runs: the first turn and its cell)
  const rows = tableRows(r),
    runs = new Map(),
    heldAt = new Map();
  for (const w of rows) {
    const ix = careerTables().races.indexOf(w.name || w.race);
    if (ix < 0) continue;
    if (!heldAt.has(w.turn)) heldAt.set(w.turn, []);
    heldAt.get(w.turn).push(ix);
    if (w.run !== false && !runs.has(ix)) runs.set(ix, { turn: w.turn, cell: w.cells[0] });
  }
  const parentWon = (ix) => d.parents.some((p) => p.wins.includes(ix));
  openWins(d, umaName(d.chara), {
    offer: rows.length ? (t) => heldAt.get(t) || [] : null,
    info: (ix) => {
      const x = runs.get(ix);
      return x || parentWon(ix) ? { ...(x || {}), parent: parentWon(ix) } : null;
    },
    onEdit: editDraft,
    onClose: () => $('sim-wins')?.focus(),
  }); // the button was redrawn by the edits
}
// a career as portraits: trainee (outfit), then her two parents; parents: [{chara, outfit, label, her}]
/**
 * @param {number} tid @param {number | undefined} outfit
 * @param {{chara: number | null, outfit?: number, label: string, her?: boolean}[]} parents
 * @param {string | null} [outfitLabel]
 */
function facesEl(tid, outfit, parents, outfitLabel = null) {
  const box = el('span', 'cfaces');
  const t = el('span', 'cface tr');
  t.append(umaAv(tid, outfit, 32), umaName(tid) + (outfitLabel ? ` ${outfitLabel}` : ''));
  box.appendChild(t);
  box.appendChild(el('span', 'cwith', 'with'));
  parents.forEach((p, j) => {
    const f = el('span', 'cface');
    f.append(umaAv(p.chara, p.outfit, 26), p.label);
    if (p.her) f.appendChild(el('span', 'her', 'her'));
    box.appendChild(f);
    if (!j) box.appendChild(el('span', 'cwith', '+'));
  });
  return box;
}
// a parent of a future career by legacy index (index = vets.length: the veteran she leaves, `who`)
const parentFace = (i, who) =>
  i >= vets.length
    ? { chara: who.chara, outfit: who.outfit, label: umaName(who.chara), her: true }
    : { chara: vets[i].chara, outfit: vets[i].outfit, label: `#${i + 1} ${umaName(vets[i].chara)}` };
const copyNode = (v) => ({
  chara: v.chara,
  ...(v.outfit ? { outfit: v.outfit } : {}),
  sparks: { ...v.sparks },
  pinks: { ...v.pinks },
  blues: { ...(v.blues || {}) },
  greens: { ...(v.greens || {}) },
  wins: [...v.wins],
});
// what a roll reads of the settings and data (career/roll.mjs)
/** @type {() => import('../roll.mjs').RollSetup} */
const rollSetup = () => ({ career: careerTables(), targets: settings.value.targets, rank: settings.value.rank });
// her draft for career `r`: a random result of it, on her two parents' records (the machine's `page.draftOf`)
/** @type {(r: DeepReadonly<CareerReport>) => Draft} */
export const draftOf = (r) => ({
  chara: r.trainee.id,
  outfit: r.trainee.outfits[0]?.id,
  borrowed: false,
  ...rolledOutcome(r, rollSetup()),
  winAt: {},
  parents: r.parents.map((i) => copyNode(vets[i])),
});
// a random result of career `r` (the machine's `page.roll`)
/** @type {(r: DeepReadonly<CareerReport>) => import('../roll.mjs').Roll} */
export const rollOf = (r) => rolledOutcome(r, rollSetup());
// an edit to the entered veteran: the comparison on screen is for the old one
const editDraft = () => draftEdited();
function addDraft() {
  const sim = simulation.peek();
  if (runView.value.searching || !sim.draft) return;
  $dialog('sim-dlg').close(); // she joins your veterans: back to the next career
  const d = structuredClone(sim.draft);
  delete d.apt;
  delete d.bought;
  const v = newVet(d.chara, d.sparks);
  Object.assign(v, d, { src: 'entered', borrowed: false });
  plan.trainee = null;
  plan.parents = [null, null];
  draftAdded(); // she is in the pool now: the next answer starts a new one even for the same career
  $('v-msg').textContent =
    `Added ${umaName(v.chara)} to your veterans. The planner now picks the next career from all of them.`;
  changed();
}
// The veteran analysis: her card (details.js; she starts as a random result of this career) with its actions, and the
// comparison panel beside it.
function simulationBody() {
  const sim = simulation.peek();
  const box = sectionEl('simulation', 'Veteran analysis');
  box.id = 'simulation';
  const d = sim.draft,
    r = shown();
  if (!d || !r) return box;
  box.appendChild(analysisCard(d, r));
  const side = el('div', 'simcol');
  side.appendChild(comparisonPanel(d, r));
  box.appendChild(side);
  return box;
}
// her card, as the game's Umamusume Details screen: aptitudes at the end of this result, Skills / Inspiration (her
// sparks, editable) / Career Info (her G1s, the wins editor); in its header the career's odds and her actions
function analysisCard(d, r) {
  const wb = btn(
    `${d.wins.length ? d.wins.length : 'No'} G1 win${d.wins.length === 1 ? '' : 's'} · edit`,
    'vwins',
    () => openDraftWins(d, r),
  );
  wb.id = 'sim-wins';
  wb.dataset.k = 'sim:wins';
  wb.setAttribute('aria-haspopup', 'dialog');
  const card = detailsEl({
    d,
    outfit: r.trainee.outfits[0]?.label || null,
    apt: d.apt || r.aptitudes?.start || null,
    races: scheduledRaces(r, careerTables().races),
    raceName: (ix) => careerTables().races[ix] || `G1 ${ix}`,
    onEdit: editDraft,
    winsBtn: wb,
    targets: r.targets,
    deck: r.deck,
    legacies: r.parents.map((i) => ({
      v: vets[i],
      label: `#${i + 1}`,
      open: () => openNode({ kind: 'v', id: vets[i].id }),
    })),
    parentLabel: (j) => `#${r.parents[j] + 1}`,
  });
  const actions = el('div', 'row');
  // Compare and a roll stay usable while a comparison runs: either one restarts it (not during a run, whose
  // answer replaces the career on show)
  const go = btn('Compare', 'primary', () => must(comparison, 'the entry hands the machine in at load').compare());
  go.id = 'sim-compare';
  go.dataset.k = 'sim:compare';
  go.disabled = runView.value.searching;
  const roll = btn('Reroll', '', () => must(comparison, 'the entry hands the machine in at load').roll());
  roll.id = 'sim-roll';
  roll.dataset.k = 'sim:roll';
  roll.disabled = runView.value.searching;
  const add = btn('Add to veterans', '', addDraft);
  add.id = 'sim-add';
  add.disabled = runView.value.searching;
  actions.append(go, roll, add);
  const mid = el('div', 'udmid');
  mid.append(actions); // (the career's odds lead the main view)
  card.querySelector('.udtop')?.insertBefore(mid, card.querySelector('.udname'));
  return card;
}
// the panel beside her card: a mini header carrying the chance a roll of this career improves your loop (the plan's
// `lift`: her outcomes that lower V), then the comparison (before any, a line saying what goes here)
function comparisonPanel(d, r) {
  const sim = simulation.peek();
  const busy = sim.state === 'busy';
  const result = el('div', `simresult${sim.stale ? ' stale' : ''}`);
  result.id = 'sim-result';
  result.setAttribute('role', 'status');
  const hdr = el('div', 'simhdr');
  hdr.append(el('span', '', 'Comparison'));
  const { planState } = answer.value,
    opt = planState.state === 'done' ? planState.r.options?.find((o) => keyOf(o) === keyOf(r)) : null;
  if (opt && opt.lift != null) {
    const odds = el(
      'span',
      'simchance',
      opt.lift > 0 ? `1/${Math.max(1, Math.round(1 / opt.lift)).toLocaleString()} to improve` : 'none improve',
    );
    odds.id = 'sim-chance';
    odds.title = `The planner's chance that a roll of this career lowers the careers to a fully looped parent (${pct(opt.lift)})`;
    hdr.append(odds);
  }
  result.appendChild(hdr);
  if (!busy && !sim.error && !sim.r)
    result.appendChild(el('p', 'simidle', 'Compare or reroll to see if she improves your loop.'));
  if (busy) {
    const p = el('p', 'runnote');
    p.innerHTML = `${runnerHTML()}<span>Comparing…</span>`;
    result.appendChild(p);
  } else if (sim.error) result.appendChild(el('p', 'note', sim.error));
  else if (sim.r) comparisonResult(result, d);
  return result;
}
// what she does for the loop if you enter her: one short line (`saves`: the careers she saves, null: not known). A
// roll's quick zero is the plan's routes' only (`rollReq`), always provisional until the exact comparison answers: it
// reads "no saving found yet"
function verdictText(d, next, saves) {
  const sim = simulation.peek();
  const n = settings.value.targets.length,
    fullyLooped = sim.full && d.parents.length === 2 && d.parents.every(isFull),
    none = sim.quick ? 'saving found yet' : 'careers saved';
  return fullyLooped
    ? 'Fully looped parent'
    : sim.full
      ? `${n}/${n}: step 1 done`
      : (sim.refining || sim.standing) && !sim.provisional
        ? 'Checking…'
        : saves != null && saves >= 0.5
          ? `Saves ${saves.toFixed(1)} careers`
          : sim.herParent
            ? 'Parent of the next career'
            : next && next.p_all > must(sim.baseline, 'figures with her best career have a baseline').p_all * (1 + 1e-9)
              ? `Better odds, no ${none}`
              : `No ${none}`;
}
// One figure per row: its label, then now → with her, green when she makes it better, red when worse (`direction`).
function figureEl({ label, now, withHer, direction, title }) {
  const c = el('div', 'tc');
  c.title = title;
  c.append(el('span', 'tl', label), el('b', `tv ${direction}`, `${now} → ${withHer}`));
  return c;
}
// a figure's direction: 'up' (better with her), 'down' (worse) or 'same' (within the estimate's noise, or unknown)
function direction(now, withHer, lowerIsBetter) {
  if (now == null || withHer == null) return 'same';
  const gain = lowerIsBetter ? now - withHer : withHer - now;
  return Math.abs(gain) < (lowerIsBetter ? 0.5 : 1e-9 * now) ? 'same' : gain > 0 ? 'up' : 'down';
}
// the comparison on show: the verdict in one line, then the two figures it rests on (now -> with her), then the best
// career with her
function comparisonResult(result, d) {
  const sim = simulation.peek();
  if (sim.stale)
    result.appendChild(
      el('p', 'note', 'These figures are for her result before your last edit. Compare to update them.'),
    );
  const before = sim.baseline,
    n = settings.value.targets.length,
    { now: careersNow, withHer: careersWithHer } = sim.v0 || { now: null, withHer: null };
  const next = must(sim.r, 'a result is drawn for a comparison').results[0]; // (none yet while the quick search for the best career with her runs)
  const saves = careersNow != null && careersWithHer != null ? careersNow - careersWithHer : null;
  const verdict = el('p', 'simverdict');
  verdict.id = 'sim-verdict';
  const head = verdictText(d, next, saves),
    rose = saves != null && saves <= -0.5;
  verdict.append(el('strong', '', head));
  if (/^Saves /.test(head) && sim.where) verdict.append(sim.where);
  // a rise (never from a roll's quick figures, which can't rise): shown as it is, and said to be the planner's
  if (rose)
    verdict.append(
      `The planner's estimate rises by ${(-saves).toFixed(1)} careers with her, a limit of its route (it offers each uma's few best careers as sides, and she can push the one it used out of them); leaving her out costs nothing.`,
    );
  result.appendChild(verdict);
  if (sim.refining || sim.standing || sim.provisional) {
    const p = el('p', 'runnote simrefine');
    p.innerHTML =
      runnerHTML() +
      (sim.provisional ? '<span>Checking…</span>' : '<span>Working out how many careers it saves…</span>');
    result.appendChild(p);
  }
  const figs = el('div', `simodds${sim.refining || sim.standing ? ' held' : ''}`);
  const nextWithHer = sim.pickA
    ? `. With her the planner's next career is ${sim.pickA.trainee.name} (${sim.pickA.parents.map((i) => `#${i + 1}`).join(' + ')}).`
    : '.';
  const fromTable = sim.quick
    ? " From the planner's table for this career (her sparks exact, her G1 wins at their chances; only the routes the plan kept); Compare checks this exact roll."
    : '';
  figs.appendChild(
    figureEl({
      label: 'Careers to loop',
      now: careersNow != null ? `about ${fmtN(careersNow)}` : '?',
      withHer: careersWithHer != null ? fmtN(careersWithHer) : '?',
      direction: rose ? 'same' : direction(careersNow, careersWithHer, true),
      title: `Expected careers to a fully looped parent: with your veterans now → with her too${nextWithHer}${fromTable}`,
    }),
  );
  // with her priced under today's fit (`fixedFit`): the plan after Add is fitted to the pool with her, so it can show
  // other figures (more careers or fewer), which this one line says
  const fitNote =
    sim.fixedFit && careersNow != null
      ? el(
          'p',
          'simfit',
          "Counted with today's plan; after Add the planner refits it to your veterans with her, so its figures can differ.",
        )
      : null;
  // the best career with her as a parent: its odds against today's best, then folded its portraits and deck
  if (!next) {
    result.appendChild(figs);
    if (fitNote) result.appendChild(fitNote);
    const p = el('p', 'runnote simbest');
    p.innerHTML = `${runnerHTML()}<span>Finding the best career with her…</span>`;
    result.appendChild(p);
    return;
  }
  const dd = el('details', 'nextdeck');
  dd.appendChild(el('summary', '', 'Best career with her'));
  const nx = el('div', 'nextc');
  nx.append(
    facesEl(
      next.trainee.id,
      next.trainee.outfits[0]?.id,
      next.parents.map((i) => parentFace(i, d)),
      next.trainee.outfits[0]?.label || '',
    ),
  );
  const baseline = must(before, 'figures with her best career have a baseline');
  figs.appendChild(
    figureEl({
      label: `All ${n} odds`,
      now: fmtIn(baseline.p_all),
      withHer: fmtIn(next.p_all),
      direction: direction(baseline.p_all, next.p_all, false),
      title: `All ${n}: today's best career → the best career with her as a parent (${pct(baseline.p_all)} → ${pct(next.p_all)}; ${n - 1} or more: ${fmtIn(baseline.p_minus1)} → ${fmtIn(next.p_minus1)})`,
    }),
  );
  result.appendChild(figs);
  if (fitNote) result.appendChild(fitNote);
  dd.append(nx, deckEl(next, 'Deck'));
  result.appendChild(dd);
}
export function renderSimulation() {
  const ae = /** @type {HTMLElement} */ (document.activeElement),
    k = ae?.dataset?.k,
    top = $('sim-top');
  if (top) {
    top.replaceChildren(shown() ? simulationBody() : '');
    fitSparks(top);
  }
  if (k?.startsWith('sim:') && !ae.isConnected) document.querySelector(`[data-k="${CSS.escape(k)}"]`)?.focus();
}
