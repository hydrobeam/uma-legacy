// The results column (#results): the options table ("Other careers") under the career on show, and its Saves popup
// (#save-root). What the table shows is career/options.js's; each row draws again only when what it shows changes.
import { signal } from '@preact/signals';
import { Fragment } from 'preact';
import { useLayoutEffect, useRef } from 'preact/hooks';
import { fmtIn } from '../../dom.js';
import { settings } from '../../settings.ts';
import { pct } from '../../odds.js';
import { vets, places as yourPlaces, backToPick } from '../../veterans/store.ts';
import { umaName } from '../../veterans/names.js';
import { borLast } from '../../cards.js';
import { must } from '../../must.ts';
import { Dialog } from '../../ui/Dialog.tsx';
import { UmaAv } from '../../ui/Images.tsx';
import { answer, searching } from '../state.ts';
import { keyOf } from '../keys.js';
import { looped, refFor, shown } from '../job.js';
import type { OptionRow as Row, PlanReport } from '../job.js';
import { TIE_CAREERS, extraText, optionsTable, roleText, takeCareer, unsettled, netText } from '../options.js';
import type { DeepReadonly } from '../../store.ts';
import { fmtN } from './parts.js';
import { DeckCard } from './Sections.tsx';
import type { ShownCareer } from './Sections.tsx';
import { Guard } from './Guard.tsx';

type Report = DeepReadonly<PlanReport>;
type Places = { readonly trainee: number | null; readonly parents: readonly (string | null)[] };

/** the results column: the table, with the plan's careers or the search's with a place open (more than one), or the last
 *  plan's with every place chosen */
export function Results() {
  const r = shown();
  if (!r) return null;
  const plan = yourPlaces.value,
    open = !plan.trainee || plan.parents.some((p) => !p),
    { last, planState } = answer.value;
  const rows =
    planState.state === 'done' ? planState.r.options : must(last, 'the column draws a career on show').r.results;
  return (open && rows.length > 1) || refFor() ? <OptionsTable /> : null;
}
function OptionsTable() {
  const { on, planReport, places, pick, list, mine, recommended, notes, nameOf, back } = optionsTable(),
    n = settings.value.targets.length;
  return (
    <section class="rbsec alts">
      <h3>
        Other careers
        {/* "Saves per run": what it means, and why the best odds aren't the loop's best, with the plan behind the
            numbers (the popup behind a small (i) right after the section's title, in its header bar) */}
        {planReport && (
          <button
            type="button"
            class="oinfo savewhy"
            id="save-why"
            data-k="save-why"
            aria-haspopup="dialog"
            aria-label="What these numbers mean"
            title="What these numbers mean"
            onClick={() => openSaveWhy(planReport, notes)}
          >
            i
          </button>
        )}
      </h3>
      <div class="optwrap">
        <table class="cartab optab">
          <thead>
            <tr>
              <th scope="col">Career</th>
              {/* both columns explain themselves on hover or keyboard focus (gmtip.js); the (i) popup says more */}
              <th scope="col">
                <span
                  class="thtip"
                  title={`Her odds of landing all ${n} targets in this one run: 1 in N means about N runs of this career for one ${n}/${n}.`}
                  // biome-ignore lint/a11y/noNoninteractiveTabindex: today's markup: focusable, so the keyboard gets its balloon (gmtip.js shows a title on focus); a button would offer an action it has not
                  tabIndex={0}
                >
                  Odds
                </span>
              </th>
              <th scope="col">
                <span
                  class="thtip"
                  title={`How many more careers than the pick this one costs, on the planner's estimate (≈0: within ${TIE_CAREERS}, as good).`}
                  // biome-ignore lint/a11y/noNoninteractiveTabindex: today's markup: focusable, so the keyboard gets its balloon (gmtip.js shows a title on focus); a button would offer an action it has not
                  tabIndex={0}
                >
                  Extra careers
                </span>
              </th>
              <th scope="col">
                <span class="sr">Choose</span>
              </th>
            </tr>
          </thead>
          <tbody>
            {list.map((r) => (
              <OptionRow
                key={`${keyOf(r)}:${r === mine ? 'mine' : ''}`}
                r={r}
                cur={!!on && keyOf(r) === keyOf(on)}
                mine={r === mine}
                planReport={planReport}
                places={places}
                pick={pick}
              />
            ))}
          </tbody>
        </table>
      </div>
      {recommended && (
        <div class="deck">
          <div class="dtitle">
            <b>{`Deck of the pair search's career: ${nameOf(recommended)}`}</b>
          </div>
          <div class="dcards">
            {borLast([...recommended.deck.cards]).map((i: number) => (
              <DeckCard key={`${i}:${recommended.deck.cards[i]}`} r={recommended as unknown as ShownCareer} i={i} />
            ))}
          </div>
        </div>
      )}
      {back && (
        <div class="optfoot">
          <button
            type="button"
            class="btn"
            id="opt-back"
            disabled={searching.value}
            onClick={() => backToPick(back as Parameters<typeof backToPick>[0])}
          >
            Back to the planner's pick
          </button>
        </div>
      )}
    </section>
  );
}
interface RowProps {
  r: DeepReadonly<Row>;
  /** the career on show */
  cur: boolean;
  /** your career, which the plan didn't list */
  mine: boolean;
  /** the plan's report (null: no Saves per run) */
  planReport: Report | null;
  /** the places the table's careers fill */
  places: Places;
  /** the plan's pick (its extra careers are measured against it) */
  pick: DeepReadonly<Row> | null;
}
// One career of the table: who it places, its tags, its odds now, its extra careers, and Use this
function OptionRow({ r, cur, mine, planReport, places, pick }: RowProps) {
  const n = settings.value.targets.length,
    lift = r.lift ?? 0,
    tip =
      planReport && r.index != null
        ? `${roleText(r)}${lift > 0 ? `. In all, ${pct(lift)} of her runs leave a veteran that helps` : ''}`
        : undefined;
  const saves =
    r.pair_search_score == null ? '' : ` (Saves ${netText(r.pair_search_score)} per run in the pair search)`;
  const tags = [
    mine && (
      <span key="mine" class="otag">
        your choice
      </span>
    ),
    planReport && r.chosen && (
      <span key="pick" class="otag pick">
        planner's pick
      </span>
    ),
    planReport && r.best_odds && (
      <span key="odds" class="otag">
        best odds now
      </span>
    ),
    planReport && r.pair_search && (
      <span
        key="pair"
        class="otag"
        title={`The pair search’s recommendation${saves}, see below: Use this runs it with this deck`}
      >
        pair search
      </span>
    ),
    planReport && unsettled(planReport).has(keyOf(r)) && (
      <span
        key="varies"
        class="otag"
        title="At least one tested variant of the planner’s model scores this career highest (its building curve, the all-target chances’ calibration, or one uma’s calibration against another’s), and the variants disagree"
      >
        pick varies
      </span>
    ),
    looped(r) && (
      <span
        key="looped"
        class="otag"
        title={`Both parents are ${n}/${n}s: if she lands all ${n}, her veteran is a fully looped parent`}
      >
        {`${n}/${n} parents`}
      </span>
    ),
  ].filter(Boolean);
  return (
    <tr class={cur ? 'on' : undefined}>
      <th scope="row">
        <PickWhat r={r} places={places} />
        {tags.length > 0 && <span class="otags">{tags}</span>}
      </th>
      <td>{fmtIn(r.p_all)}</td>
      {/* the number carries the tooltip (its balloon points at the number, not the middle of the cell) */}
      <td class="orole">
        {/* (keyed by having a tip: Preact sets `title` as a property, so a title taken away would stay as title="") */}
        <span key={tip ? 'tip' : 'none'} class="onum" tabIndex={tip ? 0 : undefined} title={tip}>
          {extraText(r, pick, planReport)}
        </span>
      </td>
      <td>
        {cur ? (
          <span class="hint">shown above</span>
        ) : (
          // (an old answer's option, while a run works on the next)
          <button type="button" class="btn" disabled={searching.value} onClick={() => takeCareer(r, places)}>
            Use this
          </button>
        )}
      </td>
    </tr>
  );
}
// who an option places: only the places left open, the trainee and the parents
function PickWhat({ r, places }: { r: DeepReadonly<Row>; places: Places }) {
  const all = vets.value,
    open = r.parents.filter((i) => !places.parents.includes(all[i].id));
  return (
    <div class="altw">
      {!places.trainee && (
        <>
          <UmaAv key={r.trainee.id} uma={r.trainee.id} outfit={r.trainee.outfits[0]?.id} size={32} />
          {` ${r.trainee.name}`}
        </>
      )}
      {open.length > 0 && (places.trainee ? 'Parents ' : ', parents ')}
      {open.map((i, j) => (
        <Fragment key={i}>
          {j > 0 && ' + '}
          <UmaAv key={`${all[i].chara}:${all[i].outfit}`} uma={all[i].chara} outfit={all[i].outfit} size={24} />
          {` #${i + 1} ${umaName(all[i].chara)}`}
        </Fragment>
      ))}
    </div>
  );
}

// ---------- "Saves per run" explained, in plain words, with the plan the numbers come from ----------
const saves = signal<{ planReport: Report | null; notes: readonly string[]; open: boolean }>({
  planReport: null,
  notes: [],
  open: false,
});
function openSaveWhy(planReport: Report, notes: readonly string[]) {
  saves.value = { planReport, notes, open: true };
}
const closeSaves = () => {
  saves.value = { ...saves.peek(), open: false };
};
/** the Saves popup (in #save-root): closed, the focus goes back to the (i), as the browser does */
export function SavesPopup() {
  const { planReport, notes, open } = saves.value,
    close = useRef<HTMLButtonElement>(null);
  // (after the dialog's own focus, which would take the first dropdown)
  useLayoutEffect(() => {
    if (open) close.current?.focus();
  }, [open]);
  return (
    <Dialog id="save-dlg" cls="gmdlg" labelledBy="save-title" open={open} onClose={closeSaves}>
      <h3 class="gmdhdr" id="save-title">
        What these mean
      </h3>
      <Guard what="The Saves popup" also={saves}>
        {planReport && <SavesBody planReport={planReport} notes={notes} />}
      </Guard>
      <div class="gmdfoot">
        <button ref={close} type="button" class="btn gmclose" onClick={closeSaves}>
          Close
        </button>
      </div>
    </Dialog>
  );
}
// what the number is, the yardstick (the plan's route from today's veterans, step by step, with its careers) and how a
// row is measured against it (run this career once first, then follow the route), with a worked example from the plan.
// For anyone: the two columns in a sentence each, the key points in bold and italics, how the saving is worked out in
// three plain sentences; today's estimate and a worked example behind dropdowns (no formula)
function SavesBody({ planReport, notes }: { planReport: Report; notes: readonly string[] }) {
  const n = settings.value.targets.length;
  const routePlan = planReport.v0_plan,
    routeMembers = planReport.members || [],
    careersToday = planReport.v0 != null ? fmtN(planReport.v0) : null;
  const routeStep = (m: (typeof routeMembers)[number], i: number) => {
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
  // the example: the route's first step of an uma without a full veteran yet, her rerun odds (after its building
  // careers, if any) lifted by 6% (one helpful parent)
  const example = routeMembers.find((m) => !m.real && (m.building_careers > 0 ? m.p_switch : m.p) > 0),
    exampleOdds = example && (example.building_careers > 0 ? example.p_switch : example.p);
  return (
    <div class="gmdbody savebody">
      <p>
        <b>This career</b>
        {`: her chance of landing all ${n} targets in this one run.`}
      </p>
      <p>
        <b>Extra careers</b>
        {`: how many more careers than the pick this one costs on the planner's estimate (below) of the careers you still need for a `}
        <i>fully looped parent</i>
        {` (an ${n}/${n} whose parents are both ${n}/${n}). `}
        <b>+0.5</b>
        {' means about half a career more; '}
        <b>≈0</b>
        {`, within ${TIE_CAREERS}: as good. It is for `}
        <i>comparing careers</i>
        {', not a forecast.'}
      </p>
      <p>
        {"The best odds aren't always the best choice: a career can be worth more for "}
        <b>the veteran she leaves behind</b>
        {', who becomes a parent for your next runs.'}
      </p>
      <p>
        {'The pick is the career with the '}
        <b>most Saves per run</b>
        {". Better odds don't break a near-tie: the Saves per run already count the runs where she lands every target."}
      </p>
      <div class="gmdsub">How it is worked out</div>
      <p>
        {'The planner makes a '}
        <b>rough estimate</b>
        {' of how many careers you still need'}
        {careersToday && (
          <>
            {' ('}
            <b>{`about ${careersToday}`}</b>
            {' today)'}
          </>
        )}
        {'. Then, for every way this career could turn out, it adds her veteran to your parents and estimates again. '}
        <i>How much the estimate drops</i>
        {', minus the run itself, is the saving.'}
      </p>
      <details class="savemore">
        <summary>Today's estimate, step by step</summary>
        <p class="oloop">{route}</p>
        {trioText && <p class="oloop">{trioText}</p>}
        <p>
          {'A '}
          <i>yardstick</i>
          {`, not advice: it assumes the simplest road (${n}/${n}s of two umas, then a career of a third with them as parents), and only its total is used.`}
        </p>
      </details>
      {example && exampleOdds && (
        <details class="savemore">
          <summary>An example</summary>
          <p>
            {`Say her veteran, as a parent, lifts ${example.trainee}'s odds for her reruns from ${fmtIn(exampleOdds)} to ${fmtIn(exampleOdds * 1.06)}. Those reruns then take `}
            <b>{`about ${(1 / exampleOdds - 1 / (exampleOdds * 1.06)).toFixed(1)} fewer careers`}</b>
            {
              '. Most of her possible results lift nothing, so the planner averages over all of them, weighted by how likely each is.'
            }
          </p>
        </details>
      )}
      <p class="hint">
        {'The whole method: '}
        <a href="about#career-planner">How this works, the career planner in detail</a>
        {'.'}
      </p>
      {/* the table's notes, where any (as the popup's other heading: no heading margins) */}
      {notes.length > 0 && <div class="gmdsub">Notes</div>}
      {notes.map((text) => (
        <p key={text} class="optnote">
          {text}
        </p>
      ))}
    </div>
  );
}
