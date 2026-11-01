// The options table ("Other careers") under the career on show: the plan's careers by Saves per run (or the search's,
// with a place open), each with Use this, and the Saves popup that explains the plan's figures and its check. It reads
// the answer and the run machine from career.js (an import cycle: this module's body only declares, so nothing of
// career.js is read while the bundle loads); its one write there is `forgetDroppedChoice`.
import { $, el, esc, fmtIn, lazyDialog } from '../dom.js';
import { settings } from '../settings.ts';
import { umaAv } from '../images.js';
import { pct } from '../odds.js';
import { vets, plan, chosen, changed, umaName } from '../vets.js';
import { whyText } from '../proofs.mjs';
import {
  last,
  planState,
  shown,
  run,
  refFor,
  buildJob,
  Blocked,
  chosenSetup,
  placesKey,
  forgetDroppedChoice,
  BORROW_CHOICES,
  keyOf,
  looped,
  btn,
  sectionEl,
  deckEl,
  fmtN,
} from '../career.js';
import { must } from '../must.ts';
/** @import { OptionRow, PlanOption, PlanReport } from '../career.js' */
// an option's role, in plain words: one of the two parents of today's estimate (the route the (i) popup walks through)
// or its last run, else what her run does for it: the side her veteran's routes lift most often where she lowers it,
// with that chance per run, or her own n/n as a side (`lifts_side`); a loop member says so first
const roleText = (o) => {
  const n = settings.value.targets.length;
  if (o.role === 'plan') return 'Makes one of the two parents your fully looped parent needs';
  if (o.role === 'finish') return 'The last run: both her parents are already full veterans';
  const loop = o.role === 'loop' ? 'Already in your loop. ' : '',
    side = o.lifts_side;
  if (!side) return `${loop}Not compared against the plan`;
  if (side.uma != null && side.chance >= side.own)
    return `${loop}Her veteran helps build ${umaName(side.uma)}'s parent: ${pct(side.chance)} of her runs do`;
  if (side.own > 0)
    return `${loop}Her veteran can be one of the two parents (an ${n}/${n}): ${pct(side.own)} of her runs`;
  return `${loop}No run of hers helps the plan`;
};
// δ, the per-decision tolerance (AGENTS.md, The career planner's goal and its bar): a row whose Saves per run is within
// it of the pick's is shown as about equal to the pick. Display only: the pick stays the strict top score
const TIE_CAREERS = 0.25;
// a career's parents, by legacy index, from its id (`kept::career_id`: "trainee:first,second:deck")
const careerParents = (id) => id.split(':')[1].split(',').map(Number);
// the first step of today's estimate (`members`, A then B, as planval's `route_first_step` reads them): the first side
// without a real full veteran of hers, her builder's career where she builds (`builder`) or else her own (`parents`);
// with both sides real, C's run on those two. Its parents by legacy index, and what it is; null without a route, or
// where it builds and names no builder (her Cont given)
/** @type {(planReport: PlanReport) => {what: string, parents: number[]}|null} */
function routeFirstStep(planReport) {
  const members = planReport.members || [];
  if (members.length < 2 || planReport.v0 == null) return null;
  for (const m of members) {
    if (m.real) continue;
    if (m.building_careers > 0)
      return m.builder ? { what: `a builder for ${m.trainee}`, parents: careerParents(m.builder) } : null;
    return m.parents ? { what: `${m.trainee}'s run`, parents: m.parents } : null;
  }
  const route = planReport.v0_plan;
  const real = (route?.sides || []).map((side) => /^real:(\d+)$/.exec(side)).filter((m) => m !== null);
  return route && real.length === 2 ? { what: `${route.c}'s run`, parents: real.map((m) => +m[1]) } : null;
}
// an option's net saving: the careers her veteran saves toward a fully looped parent on average, minus the one it costs
const netText = (x) => (x >= 0 ? '+' : '−') + Math.abs(x).toFixed(1);
// an option's Saves per run: the plan's `score` (her net saving; null when the plan scored nothing), else `index`
const scoreOf = (o) => o.score ?? o.index;

// who an option places: only the places left open, the trainee and the parents (places: the places the table's careers
// fill: yours, or the plan's when the table is the last plan's)
function pickWhat(r, places = plan) {
  const what = el('div', 'altw');
  if (!places.trainee) {
    what.append(umaAv(r.trainee.id, r.trainee.outfits[0]?.id, 32), ` ${r.trainee.name}`);
  }
  const open = r.parents.filter((i) => !places.parents.includes(vets[i].id));
  if (open.length) {
    if (!places.trainee) what.append(', parents ');
    else what.append('Parents ');
    open.forEach((i, j) => {
      if (j) what.append(' + ');
      what.append(umaAv(vets[i].chara, vets[i].outfit, 24), ` #${i + 1} ${umaName(vets[i].chara)}`);
    });
  }
  return what;
}
function useBtn(r, places = plan) {
  const b = btn('Use this', '', () => {
    // her trainee, and the career's parents in the places left open (in the career's order)
    plan.trainee = r.trainee.id;
    const parents = places.parents.slice();
    const unplaced = r.parents.filter((i) => !parents.includes(vets[i].id));
    for (let j = 0; j < 2; j++) if (!parents[j]) parents[j] = vets[unplaced.shift()].id;
    plan.parents = parents;
    plan.fromTable = true; // (a change of setup hands the places back to the planner: career.js initSettings)
    // a builder's other career, or the pair search's recommendation, keeps its deck: the job then reports that career
    // (`career`) instead of searching, while the setup it was chosen on stands (`chosenApplies`)
    chosen.career = null;
    forgetDroppedChoice();
    if ((r.alternative || r.pair_search) && r.career) {
      try {
        chosen.career = { places: placesKey(), id: r.career, setup: chosenSetup(buildJob(settings.value)) };
      } catch (e) {
        if (!(e instanceof Blocked)) throw e;
        // settings that make no job: nothing is chosen
      }
    }
    changed();
  });
  b.disabled = run.searching; // (an old answer's option, while a run works on the next)
  return b;
}
const OPT_SHOW = 8;
// where no row the plan lists (every one a career you can run now) saves more than its own run (top Saves per run
// below 0): the line above the table, with the pick's net; under None left today, where the estimate's first step
// (`routeFirstStep`) takes a borrowed parent, it says so. Null otherwise
function negativeTopText(planReport, pick) {
  if (!planReport || !pick) return null;
  const top = Math.max(...planReport.options.map((o) => scoreOf(o) ?? Number.NEGATIVE_INFINITY));
  if (!(Number.isFinite(top) && top < 0)) return null;
  const text = 'No career saves its own run today: the pick costs least.';
  const job = planState.state === 'done' ? planState.job : last?.job,
    step = job?.borrowed_parent === 'later' ? routeFirstStep(planReport) : null,
    borrowed = step ? step.parents.filter((i) => vets[i]?.borrowed) : [];
  if (!borrowed.length) return text;
  const who = borrowed.map((i) => `#${i + 1} ${umaName(vets[i].chara)}`).join(' and ');
  return `${text} Its next step needs a borrowed parent (${who}), kept out by ${BORROW_CHOICES.later}.`;
}
export function optionsEl() {
  const box = sectionEl('alts', 'Other careers');
  // (today's best by odds outside the places you chose is in the plan's output, not offered here)
  // with the plan: the 8 highest savings (every uma is scored), the best odds, the pick and the career on show; with
  // every place chosen, the last plan's table on the same setup and veterans, with your career in it (from the search
  // when the plan didn't list it: no saving), and a way back to the places it ran with
  const on = shown(),
    ref = refFor(),
    mineOnly = !!plan.trainee && plan.parents.every(Boolean);
  const planReport = planState.state === 'done' ? planState.r : mineOnly && ref ? ref.result : null,
    places = planState.state !== 'done' && mineOnly && ref ? ref.places : plan;
  const pick = planReport?.options.find((o) => o.chosen) || null;
  /** @type {OptionRow[]} the plan's rows, or the search's careers (no plan fields) */
  const list = planReport
    ? planReport.options.filter(
        (o, i) =>
          i < OPT_SHOW ||
          o.best_odds ||
          o.chosen ||
          unsettled(planReport).has(keyOf(o)) ||
          (on && keyOf(o) === keyOf(on)),
      )
    : must(last, 'the options table draws a career on show').r.results.slice();
  // the pair search's recommendation where the plan's rows don't hold it: a row run as it is, its deck kept (`useBtn`)
  // (a row the table already has gets its flag, pair-search score and id merged in, as a copy: the plan's own rows stay)
  const recommended = pairSearchRow(planReport);
  if (recommended) {
    const at = list.findIndex((o) => keyOf(o) === keyOf(recommended));
    if (at < 0) list.push(recommended);
    else
      list[at] = {
        ...list[at],
        pair_search: true,
        pair_search_score: recommended.pair_search_score,
        career: recommended.career,
      };
  }
  const mine = planReport && on && !list.some((o) => keyOf(o) === keyOf(on)) ? on : null;
  if (mine) list.unshift(mine);
  const tb = el('table', 'cartab optab');
  tb.innerHTML =
    '<thead><tr><th scope="col">Career</th><th scope="col"></th><th scope="col"></th><th scope="col"><span class="sr">Choose</span></th></tr></thead><tbody></tbody>';
  // both columns explain themselves on hover or keyboard focus (gmtip.js); the (i) popup says more about extra careers
  const n = settings.value.targets.length,
    ths = must(tb.tHead, 'the table is made with its head').rows[0].cells;
  const t = el('span', 'thtip', 'Odds');
  t.title = `Her odds of landing all ${n} targets in this one run: 1 in N means about N runs of this career for one ${n}/${n}.`;
  t.tabIndex = 0;
  const sp = el('span', 'thtip', 'Extra careers');
  sp.title = `How many more careers than the pick this one costs, on the planner's estimate (≈0: within ${TIE_CAREERS}, as good).`;
  sp.tabIndex = 0;
  ths[1].appendChild(t);
  ths[2].appendChild(sp);
  // "Saves per run": what it means, and why the best odds aren't the loop's best, with the plan behind the numbers
  // (the popup behind a small (i) right after the section's title, in its header bar)
  if (planReport) {
    const q = el('button', 'oinfo savewhy', 'i');
    q.type = 'button';
    q.id = 'save-why';
    q.dataset.k = 'save-why';
    q.setAttribute('aria-haspopup', 'dialog');
    q.setAttribute('aria-label', 'What these numbers mean');
    q.title = 'What these numbers mean';
    q.onclick = () => openSaveWhy(planReport, notes);
    must(box.querySelector(':scope > h3'), 'the box is made with its heading').append(q);
  }
  for (const r of list) tb.tBodies[0].appendChild(optionRow(r, { on, mine, planReport, places, pick }));
  const nameOf = (o) => `${o.trainee.name}${o.trainee.outfits[0] ? ` ${o.trainee.outfits[0].label}` : ''}`;
  // no row saves more than its own run on today's estimate: said above the table, with the pick's net
  // the table's notes (no career saves its run; the variants, the pair search, a check that didn't finish, memory, an
  // unproven search): in the (i) popup, not under the table
  const notes = [];
  const negative = negativeTopText(planReport, pick);
  if (negative) notes.push(negative);
  const wrap = el('div', 'optwrap');
  wrap.appendChild(tb);
  box.appendChild(wrap);
  // the robustness check found the rule's pick changes with the tested variants (`robust`): every career it selects is in
  // the table; the pick stays the planner's rule's
  if (planReport && unsettled(planReport).size) {
    const names = list.filter((o) => unsettled(planReport).has(keyOf(o))).map(nameOf);
    notes.push(
      `The tested variants of the planner’s model (its building curve at the ends of its fitted range, every all-target chance at the ends of its calibration interval, and per-uma scenarios: the top-scoring uma’s all-target chance lower and a rival uma’s higher, by a split inferred from the spread measured between umas) score different careers highest, among the careers shown: ${names.join(', ')}. The pick is the rule’s under the model as fitted.`,
    );
  }
  for (const text of pairSearchNotes(planReport, nameOf)) notes.push(text);
  if (recommended) box.appendChild(deckEl(recommended, `Deck of the pair search's career: ${nameOf(recommended)}`));
  if (planReport?.check_unavailable)
    notes.push(
      'The check of this pick under tested variants of the model did not finish (the solver was stopped or restarted before it was done, or a variant needed more memory than it may take).',
    );
  // the plan's scorings, or its check's (its variants and pair search: `robust.scoring_complete`), left something out for
  // memory (false; absent from a solver without it): every saving shown, or the check's, may be low, so the pick or the
  // recommendation may not be the model's
  if (planReport?.scoring_complete === false || planReport?.robust?.scoring_complete === false)
    notes.push(
      'The planner ran out of memory before it could weigh every route or partner for these careers: their Saves per run may be too low, and the pick may not be the best.',
    );
  // a row whose career search was not complete (`proven: false`: its work cap, a band overflow, or her deck class
  // breached its cap, in this job or an earlier one, whatever the cap-free rerun found): the best it found, maybe not
  // her best. The rows don't say which
  // cause was theirs; the plan job's report names the causes it saw in any of its searches (`unprovenWhy`)
  if (planReport && list.some((o) => o.proven === false))
    notes.push(
      `The search could not prove the best career for some of these umas (${whyText(planReport.unproven_why || [])}): a better one may exist for them.`,
    );
  // (a row proven under a named assumption is proven: the run note names the job's assumptions, the plan's searches
  // included, `searchCaveat`, so the table adds no note for them)
  if (ref && placesKey(ref.places) !== placesKey()) {
    const back = btn("Back to the planner's pick", '', () => {
      plan.trainee = ref.places.trainee;
      plan.parents = ref.places.parents.slice();
      changed();
    });
    back.id = 'opt-back';
    back.disabled = run.searching;
    const f = el('div', 'optfoot');
    f.appendChild(back);
    box.appendChild(f);
  }
  return box;
}
// the pair search's recommendation as a row of the table (`pairs.recommendation.report`, with its pair-search score and
// `pair_search`, so Use this keeps its deck): where it is not the plan's pick, once the check of this pick is complete
/** @type {(planReport: PlanReport|null) => OptionRow|null} */
const pairSearchRow = (planReport) => {
  const recommendation = planReport && checkDone(planReport)?.pairs?.recommendation;
  if (!recommendation?.report || recommendation.source === 'plan_pick') return null;
  const score = recommendation.score ?? null;
  return { ...recommendation.report, score, pair_search: true, pair_search_score: score };
};
// the careers the tested variants score highest where they disagree (`robust.contenders`, `pick_varies`): none until
// every variant is in, where they agree, or where the check is of another pick
const checkDone = (planReport) => {
  const check = planReport.robust;
  return check && check.pending === 0 && check.plan === planReport.outcomes?.career ? check : null;
};
const unsettled = (planReport) => {
  const check = checkDone(planReport);
  return new Set(check?.pick_varies ? check.contenders : []);
};
// the pair search's findings (`robust.pairs`): one line per trainee it ran for as the pick's or a per-uma scenario's
// (`reason` "pick" or "scenario"): her better parents (`better_parents`), by how much they beat her shown career (the pick,
// for the pick's trainee), or that none did; then, where the best (trainee, pair) it found by score (`pairs.best`) is
// another trainee's, a line naming it against the pick (`versus_pick`): better where it scores above the pick as fitted
// (the solver's `recommendation` then), with the per-uma scenarios that reverse that (`varies_under`) named as a
// sensitivity, else not better. Nothing until the check is complete, or where it is of another pick. nameOf: an
// option's trainee as the table names her
/** @type {(planReport: PlanReport|null, nameOf: (o: PlanOption) => string) => string[]} */
function pairSearchNotes(planReport, nameOf) {
  if (!planReport) return [];
  const check = checkDone(planReport);
  if (!check?.pairs) return [];
  const vetName = (i) => (vets[i] ? `#${i + 1} ${umaName(vets[i].chara)}` : `#${i + 1}`);
  const shownOf = (found) => planReport.options.find((o) => keyOf(o) === found.shown);
  // (a bound on the pairs scored per uma, `pair_bound`: past it, her pairs with the highest all-target ceilings)
  const bound = check.pairs.pair_bound;
  const truncatedText = (found) =>
    found.pairs_truncated
      ? `; of her ${found.pairs_runnable} pairs only the ${found.pairs_scored} with the highest all-target ceilings were checked`
      : '';
  const lines = check.pairs.trainees
    .filter((found) => (found.reason ?? 'pick') !== 'top')
    .map((found) => {
      const shown = shownOf(found),
        who = shown ? nameOf(shown) : found.trainee,
        others = Math.max(0, found.pairs_scored - 1);
      // (a gain on her shown parents themselves is a better deck, not better parents: the solver's `better_parents`)
      if (!found.better_parents)
        return `No better parents found for ${who} among the ${found.pairs_scored} pairs checked (each with the best deck found so far for it${truncatedText(found)}).`;
      const varies = (found.corner_margins || []).some((margin) => !(margin > 0))
        ? '; not better under every tested building curve'
        : '';
      // (her own odds read high or low by the measured per-uma spread: both careers scaled alike, `scale_margins`)
      const scaleVaries = (found.scale_margins || []).some((margin) => !(margin > 0))
        ? '; not better if her odds are off by the measured spread'
        : '';
      const overOthers =
        found.margin_over_other_pairs == null
          ? ''
          : ` and ${netText(found.margin_over_other_pairs)} vs the best of the ${others} other pairs checked`;
      return `Better parents found for ${who}: ${found.best_parents.map(vetName).join(' + ')} (Saves ${netText(found.margin_over_shown)} per run vs the shown parents${overOthers}; not certified best${varies}${scaleVaries}${truncatedText(found)}).`;
    });
  const best = check.pairs.best,
    pickFound = check.pairs.trainees[0],
    versus = best?.versus_pick;
  if (best && pickFound && versus && best.trainee !== pickFound.trainee) {
    const bestFound = check.pairs.trainees.find((found) => found.trainee === best.trainee),
      bestShown = bestFound && shownOf(bestFound),
      pickShown = shownOf(pickFound),
      who = bestShown ? nameOf(bestShown) : best.trainee,
      pickWho = pickShown ? nameOf(pickShown) : pickFound.trainee,
      side = `the pick (${pickWho})`,
      shared = check.pairs.shared_coverage,
      truncated = check.pairs.trainees.filter((found) => found.pairs_truncated).length,
      searched = truncated
        ? `${check.pairs.trainees.length} umas on their pairs (${truncated} on only their ${bound} with the highest all-target ceilings)`
        : `${check.pairs.trainees.length} umas on all their pairs`,
      coverage = shared ? `${searched}, ${shared.trainees} more on the best pairs found` : searched,
      where = `${who} on ${best.best_parents.map(vetName).join(' + ')}`,
      { nominal, pick_down: pickDown, pick_up: pickUp } = versus.margins;
    // (the per-uma scenarios that reverse the margin: a sensitivity of the choice, named, deciding nothing)
    const varies = versus.varies_under?.length
      ? ` The choice varies under the per-uma scenarios: ${netText(pickDown)} vs the pick with ${pickWho}'s all-target chance ×0.76 and hers ×1.32, ${netText(pickUp)} the reverse.`
      : '';
    if (nominal > 0)
      lines.push(
        `Better career found by the pair search (${coverage}): ${where} (Saves ${netText(best.score)} per run, ${netText(nominal)} vs ${side}; not certified best).${varies}`,
      );
    else
      lines.push(
        `No other uma's career found by the pair search (${coverage}) beats ${side}: the best, ${where}, saves ${netText(best.score)} per run, ${netText(nominal)} vs it.`,
      );
  }
  return lines;
}
// a row's extra careers against the pick (the pick's Saves per run minus the row's): "≈0" within the tie tolerance, "–"
// for the pick itself or where a score is missing
function extraText(r, pick, planReport) {
  if (!planReport || !pick || r === pick || keyOf(r) === keyOf(pick)) return '–';
  const a = scoreOf(pick),
    b = scoreOf(r);
  if (a == null || b == null) return '–';
  const extra = a - b;
  return Math.abs(extra) < TIE_CAREERS ? '≈0' : netText(extra);
}
// One career of the table: who it places, its tags, its odds now, its Saves per run, and Use this. on: the career on
// show; mine: your career when the plan didn't list it; planReport: the plan's report (null: no Saves per run); places:
// the places the table's careers fill; pick: the plan's pick (its extra careers are measured against it)
function optionRow(r, { on, mine, planReport, places, pick }) {
  const n = settings.value.targets.length;
  const cur = on && keyOf(r) === keyOf(on),
    row = el('tr', cur ? 'on' : ''),
    c0 = el('th');
  c0.scope = 'row';
  c0.appendChild(pickWhat(r, places));
  const tags = el('span', 'otags');
  if (r === mine) tags.appendChild(el('span', 'otag', 'your choice'));
  if (planReport && r.chosen) tags.appendChild(el('span', 'otag pick', "planner's pick"));
  if (planReport && r.best_odds) tags.appendChild(el('span', 'otag', 'best odds now'));
  if (planReport && r.pair_search) {
    const a = el('span', 'otag', 'pair search');
    const saves =
      r.pair_search_score == null ? '' : ` (Saves ${netText(r.pair_search_score)} per run in the pair search)`;
    a.title = `The pair search’s recommendation${saves}, see below: Use this runs it with this deck`;
    tags.appendChild(a);
  }
  if (planReport && unsettled(planReport).has(keyOf(r))) {
    const a = el('span', 'otag', 'pick varies');
    a.title =
      'At least one tested variant of the planner’s model scores this career highest (its building curve, the all-target chances’ calibration, or one uma’s calibration against another’s), and the variants disagree';
    tags.appendChild(a);
  }
  if (looped(r)) {
    const a = el('span', 'otag', `${n}/${n} parents`);
    a.title = `Both parents are ${n}/${n}s: if she lands all ${n}, her veteran is a fully looped parent`;
    tags.appendChild(a);
  }
  if (tags.childElementCount) c0.appendChild(tags);
  // the number carries the tooltip (its balloon points at the number, not the middle of the cell)
  const sv = el('td', 'orole'),
    num = el('span', 'onum', extraText(r, pick, planReport));
  sv.append(num);
  if (planReport && r.index != null) {
    num.tabIndex = 0;
    num.title = `${roleText(r)}${r.lift > 0 ? `. In all, ${pct(r.lift)} of her runs leave a veteran that helps` : ''}`;
  }
  row.append(c0, el('td', '', fmtIn(r.p_all)), sv);
  const act = el('td');
  act.appendChild(cur ? el('span', 'hint', 'shown above') : useBtn(r, places));
  row.appendChild(act);
  return row;
}
// "Saves per run" explained, in plain words, with the plan the numbers come from
const saveDialog = lazyDialog('gmdlg', (dialog) => {
  dialog.id = 'save-dlg';
  dialog.setAttribute('aria-labelledby', 'save-title');
  dialog.addEventListener('close', () => $('save-why')?.focus());
});
function openSaveWhy(planReport, notes = []) {
  const saveDlg = saveDialog.get();
  saveDlg.textContent = '';
  const n = settings.value.targets.length;
  const h = el('h3', 'gmdhdr', 'What these mean');
  h.id = 'save-title';
  saveDlg.appendChild(h);
  const body = el('div', 'gmdbody savebody');
  // what the number is, the yardstick (the plan's route from today's veterans, step by step, with its careers) and how a
  // row is measured against it (run this career once first, then follow the route), with a worked example from the plan
  const routePlan = planReport.v0_plan,
    routeMembers = planReport.members || [],
    careersToday = planReport.v0 != null ? fmtN(planReport.v0) : null;
  const routeStep = (m, i) => {
    const careers = routePlan?.careers?.[i];
    const c = careers != null ? ` (about ${fmtN(careers)} careers)` : '';
    if (m.real) return `your ${n}/${n} of ${m.trainee} (no careers)`;
    const who = m.has_full_veteran ? `a fresh ${n}/${n} of ${m.trainee} (yours is the poorer parent): ` : '';
    // the career her fresh side is from: her best today, or (`alternative`) another of her top careers, which the
    // cheapest route takes for its parents
    const on = m.parents ? ` on parents ${m.parents.map((j) => `#${j + 1}`).join(' + ')}` : '',
      career = m.alternative
        ? `another of ${m.trainee}'s careers${on} (not her best today: the cheapest route takes it)`
        : `${m.trainee}'s best career${on}`;
    return m.building_careers > 0
      ? `${who}lift ${m.trainee}'s odds from ${career} (${fmtIn(m.p)} now) with about ${fmtN(m.building_careers)} building careers, to about ${fmtIn(m.p_switch)}, then rerun her until she lands ${n}/${n}${c}`
      : `${who}rerun ${career} (${fmtIn(m.p)}) until she lands ${n}/${n}${c}`;
  };
  const route =
    routePlan && routeMembers.length >= 2 && careersToday
      ? `First ${routeStep(routeMembers[0], 0)}; then ${routeStep(routeMembers[1], 1)}; then ${routePlan.c} with those two ${n}/${n}s as parents${routePlan.careers?.[2] != null ? ` (about ${fmtN(routePlan.careers[2])} careers)` : ''}. About ${careersToday} careers in all.`
      : 'No estimate: no route from the veterans you have now.';
  // the cheapest routes (`trios`, at most three, today's first) and how many are within the assumed 2% band of it
  // (`near`): too close for the estimate to say which is cheapest
  const trios = planReport.trios || [],
    nearCount = trios.filter((t) => t.near).length;
  const trioText =
    trios.length > 1
      ? `The ${trios.length} cheapest routes: ${trios.map((t) => `${t.a} and ${t.b}, then ${t.c} (about ${fmtN(t.value)} careers)`).join('; ')}. ${
          nearCount > 1
            ? `The first ${nearCount} are within 2% of each other (a band assumed for the estimate's error between routes): too close for it to say which is cheapest.`
            : `The others are more than 2% dearer (a band assumed for the estimate's error between routes).`
        }`
      : '';
  // the example: the route's first step of an uma without a full veteran yet, her rerun odds (after its building careers, if any) lifted by
  // 6% (one helpful parent)
  const example = routeMembers.find((m) => !m.real && (m.building_careers > 0 ? m.p_switch : m.p) > 0),
    exampleOdds = example && (example.building_careers > 0 ? example.p_switch : example.p);
  const exampleText = example
    ? `Say her veteran, as a parent, lifts ${esc(example.trainee)}'s odds for her reruns from ${fmtIn(exampleOdds)} to ${fmtIn(exampleOdds * 1.06)}. Those reruns then take <b>about ${(1 / exampleOdds - 1 / (exampleOdds * 1.06)).toFixed(1)} fewer careers</b>. Most of her possible results lift nothing, so the planner averages over all of them, weighted by how likely each is.`
    : '';
  // for anyone: the two columns in a sentence each, the key points in bold and italics, how the saving is worked out in
  // three plain sentences; today's estimate and a worked example behind dropdowns (no formula)
  const para = (html, cls = '') => {
    const x = el('p', cls);
    x.innerHTML = html;
    return x;
  };
  const drop = (title, ...kids) => {
    const d = el('details', 'savemore');
    d.append(el('summary', '', title), ...kids);
    return d;
  };
  body.append(
    para(`<b>This career</b>: her chance of landing all ${n} targets in this one run.`),
    para(
      `<b>Extra careers</b>: how many more careers than the pick this one costs on the planner's estimate (below) of the careers you still need for a <i>fully looped parent</i> (an ${n}/${n} whose parents are both ${n}/${n}). <b>+0.5</b> means about half a career more; <b>≈0</b>, within ${TIE_CAREERS}: as good. It is for <i>comparing careers</i>, not a forecast.`,
    ),
    para(
      `The best odds aren't always the best choice: a career can be worth more for <b>the veteran she leaves behind</b>, who becomes a parent for your next runs.`,
    ),
    para(
      `The pick is the career with the <b>most Saves per run</b>. Better odds don't break a near-tie: the Saves per run already count the runs where she lands every target.`,
    ),
    el('div', 'gmdsub', 'How it is worked out'),
    para(
      `The planner makes a <b>rough estimate</b> of how many careers you still need${careersToday ? ` (<b>about ${careersToday}</b> today)` : ''}. Then, for every way this career could turn out, it adds her veteran to your parents and estimates again. <i>How much the estimate drops</i>, minus the run itself, is the saving.`,
    ),
    drop(
      "Today's estimate, step by step",
      el('p', 'oloop', route),
      ...(trioText ? [el('p', 'oloop', trioText)] : []),
      para(
        `A <i>yardstick</i>, not advice: it assumes the simplest road (${n}/${n}s of two umas, then a career of a third with them as parents), and only its total is used.`,
      ),
    ),
  );
  if (exampleText) body.append(drop('An example', para(exampleText)));
  body.append(
    para(`The whole method: <a href="about#career-planner">How this works, the career planner in detail</a>.`, 'hint'),
  );
  // the table's notes, where any
  if (notes.length) {
    body.appendChild(el('div', 'gmdsub', 'Notes')); // (as the popup's other heading: no heading margins)
    for (const text of notes) body.appendChild(el('p', 'optnote', text));
  }
  saveDlg.appendChild(body);
  const f = el('div', 'gmdfoot'),
    c = el('button', 'btn gmclose', 'Close');
  c.type = 'button';
  c.onclick = () => saveDlg.close();
  f.appendChild(c);
  saveDlg.appendChild(f);
  saveDlg.showModal();
  c.focus();
}
