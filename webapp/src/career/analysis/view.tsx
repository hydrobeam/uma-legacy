// ---------- Try a run: the veteran analysis' popup (the veteran you got) ----------
// The decision is per veteran: enter the one you got (she starts as a random result, drawn as Reroll draws,
// career/roll.ts) and Compare runs both choices, running this career again or using her in the next one, on the plan's
// terms (expected careers to land). `Analysis` draws it into the popup's mount point (#sim-top) from the analysis store
// (./state.ts `simulation`) and the answer store; Compare and Reroll are the comparison's run machine's (./compare.ts,
// made by the entry and handed in). Her card is ./Details.tsx. This module also makes her draft from a career
// (`draftOf`, `rollOf`: the page's veterans and settings, for the machine's deps) and adds her to the veterans.
import { useLayoutEffect, useRef } from 'preact/hooks';
import { computed, signal } from '@preact/signals';
import { careerTables } from '../../model/data.ts';
import { settings } from '../../state/settings.ts';
import { pct, fmtIn } from '../../model/odds.ts';
import { draft, draftRef } from '../../veterans/store.ts';
import type { Draft, LineageNode, NodeRecord } from '../../veterans/store.ts';
import { umaName } from '../../veterans/names.ts';
import { openWins } from '../../veterans/pickers.ts';
import { fitSparks } from '../../details.ts';
import { must } from '../../must.ts';
import { Runner, UmaAv } from '../../ui/Images.tsx';
import type { DeepReadonly } from '../../state/store.ts';
import type { ScheduleCell } from '../../solver/solver.ts';
import { cellAt } from '../../model/agenda.ts';
import { scheduledRaces, rolledOutcome } from '../roll.ts';
import type { Roll, RollSetup } from '../roll.ts';
import { answer, answerPool, runView, searching as runSearching } from '../state.ts';
import { keyOf } from '../keys.ts';
import { isFull } from '../job.ts';
import { legacyNumber, legacyVet, legacyVetNow, poolNumber, shown } from '../current.ts';
import type { CareerReport } from '../job.ts';
import { fmtN, tableRows } from '../view/parts.ts';
import { openNode } from '../view/dialogs.ts';
import { sayVets } from '../view/poolState.ts';
import { Guard } from '../view/Guard.tsx';
import { Dialog, DialogHead, DialogPanel } from '../../ui/Dialog.tsx';
import { DeckCards } from '../view/Sections.tsx';
import { Details, legaciesOf } from './Details.tsx';
import type { DetailsSpec } from './Details.tsx';
import { simulation, draftJoins } from './state.ts';
import type { SimulationView } from './state.ts';
import type { Comparison } from './compare.ts';
import { Button } from '../../ui/Button.tsx';
import { panel } from '../../ui/Panel.tsx';
import { GameBar } from '../../ui/GameBar.tsx';
import { cx } from 'tailwind-variants/lite';

type Career = DeepReadonly<CareerReport>;
/** her draft on show (the veterans store's) */
type DraftView = DeepReadonly<Draft>;

// ---------- her draft (the machine's deps)
const copyNode = (v: NodeRecord): LineageNode => ({
  chara: v.chara,
  ...(v.outfit ? { outfit: v.outfit } : {}),
  sparks: { ...v.sparks },
  pinks: { ...v.pinks },
  blues: { ...(v.blues || {}) },
  greens: { ...(v.greens || {}) },
  wins: [...v.wins],
});
// what a roll reads of the settings and data (career/roll.ts)
const rollSetup = (): RollSetup => ({
  career: careerTables(),
  targets: settings.peek().targets,
  rank: settings.peek().rank,
});
/** a random result of career `r` */
const rollOf = (r: Career): Roll => rolledOutcome(r, rollSetup());
/** her draft for career `r`: a random result of it, on her two parents' records (the machine's `page.draftOf`); null
 *  when one of them has left the pool under the answer on show (career/current.ts `legacyVet`) */
export function draftOf(r: Career): Draft | null {
  const parents: LineageNode[] = [];
  for (const i of r.parents) {
    const v = legacyVetNow(i);
    if (!v) return null;
    parents.push(copyNode(v));
  }
  // (her outfit: the one the career names, where it names one)
  const [outfit] = r.trainee.outfits;
  return {
    chara: r.trainee.id,
    ...(outfit ? { outfit: outfit.id } : {}),
    borrowed: false,
    ...rollOf(r),
    winAt: {},
    parents,
  };
}

// ---------- her G1 wins and Add to veterans
// the veteran analysis' G1 wins: the G1 wins editor of the pool (a win in its slot, click to remove; an empty slot lists
// the G1s she could enter then). Her schedule's races keep their win odds in the turn bar; a win shows at the turn
// picked for it, else the turn her schedule runs it. A race held in Classic and Senior is one win saddle (one race
// index). Its edits land on her through the store (`draftRef`: on her alone, not on a draft that replaces her while
// it is open). `back`: the focus on close
function openDraftWins(d: DraftView, r: Career, back: () => void) {
  // the G1s she could enter, by turn (heldAt), and those her schedule runs, by race (runs: the first turn and its cell)
  const rows = tableRows(r),
    runs = new Map<number, { turn: number; cell: ScheduleCell | null }>(),
    heldAt = new Map<number, number[]>();
  for (const w of rows) {
    const ix = careerTables().races.indexOf(w.name || w.race);
    if (ix < 0) continue;
    if (!heldAt.has(w.turn)) heldAt.set(w.turn, []);
    heldAt.get(w.turn)?.push(ix);
    if (w.run !== false && !runs.has(ix)) runs.set(ix, { turn: w.turn, cell: cellAt(w, 0) });
  }
  const parentWon = (ix: number) => d.parents.some((p) => p.wins.includes(ix));
  openWins(draftRef(), umaName(d.chara), {
    offer: rows.length ? (t: number) => heldAt.get(t) || [] : null,
    info: (ix: number) => {
      const x = runs.get(ix);
      return x || parentWon(ix) ? { ...(x || {}), parent: parentWon(ix) } : null;
    },
    onClose: back,
  });
}
// Add to veterans: she joins the pool (./state.ts `draftJoins`), back to the next career
function addDraft() {
  if (runView.peek().searching || !draft.peek()) return;
  closeTryRun();
  const chara = draftJoins();
  sayVets(`Added ${umaName(chara)} to your veterans. The planner now picks the next career from all of them.`);
}

// ---------- the comparison panel
// a career as portraits: trainee (outfit, in bolder type), then her two parents (her marked, data-her), joined by "with"
// and "+"
const FACE = {
  trainee: 'text-ui inline-flex items-center gap-1.25 font-extrabold',
  parent: 'text-ui inline-flex items-center gap-1.25 font-semibold',
} as const;
const WITH = 'text-muted text-note';
function Faces({
  tid,
  outfit,
  outfitLabel,
  parents,
}: {
  tid: number;
  outfit: number | undefined;
  outfitLabel: string;
  parents: { chara: number | null; outfit?: number | undefined; label: string; her?: boolean }[];
}) {
  return (
    <span class="inline-flex flex-wrap items-center gap-1">
      <span class={FACE.trainee}>
        <UmaAv key={`${tid}:${outfit}`} uma={tid} outfit={outfit} size={32} />
        {umaName(tid) + (outfitLabel ? ` ${outfitLabel}` : '')}
      </span>
      <span class={WITH}>with</span>
      {parents.map((p, j) => [
        // biome-ignore lint/suspicious/noArrayIndexKey: her two parents, in their fixed places
        <span key={`f${j}`} class={FACE.parent}>
          <UmaAv key={`${p.chara}:${p.outfit}`} uma={p.chara} outfit={p.outfit} size={26} />
          {p.label}
          {p.her && (
            <span class="font-data text-caption border-accent text-ink rounded-xs border px-1" data-her="">
              her
            </span>
          )}
        </span>,
        !j && (
          <span key="with" class={WITH}>
            +
          </span>
        ),
      ])}
    </span>
  );
}
// a parent of a future career by legacy index: the index past the answer's pool is the veteran she leaves (`who`: the
// comparison's jobs add her last, compare.ts); one who has left the pool under the answer (`legacyVet`), her number alone
const parentFace = (i: number, who: DraftView) => {
  if (i === answerPool.value.length)
    return { chara: who.chara, outfit: who.outfit, label: umaName(who.chara), her: true };
  const v = legacyVet(i);
  return v
    ? { chara: v.chara, outfit: v.outfit, label: `${poolNumber(v)} ${umaName(v.chara)}` }
    : { chara: null, label: `#${i + 1}` };
};
// a figure's direction: 'up' (better with her), 'down' (worse) or 'same' (within the estimate's noise, or unknown)
function direction(now: number | null, withHer: number | null, lowerIsBetter: boolean): keyof typeof FIGURE_TONE {
  if (now == null || withHer == null) return 'same';
  const gain = lowerIsBetter ? now - withHer : withHer - now;
  return Math.abs(gain) < (lowerIsBetter ? 0.5 : 1e-9 * now) ? 'same' : gain > 0 ? 'up' : 'down';
}
// a figure's words by its direction; faded while the exact comparison is still working (`held`)
const FIGURE_TONE = { up: 'text-sim-up', down: 'text-sim-down', same: 'text-gm-ink2' } as const;
/** one figure per row, ruled off above: its label, then now → with her, green when she makes it better, red when worse
 *  (`direction`) */
function Figure({
  label,
  now,
  withHer,
  dir,
  held,
  title,
}: {
  label: string;
  now: string;
  withHer: string;
  dir: keyof typeof FIGURE_TONE;
  held: boolean;
  title: string;
}) {
  return (
    <div class="border-gm-line flex items-baseline justify-between gap-2 border-t pt-1.5" title={title}>
      <span class="text-note text-gm-ink2">{label}</span>
      <b
        class={cx('text-body font-bold tabular-nums', FIGURE_TONE[dir], held && 'opacity-55')}
      >{`${now} → ${withHer}`}</b>
    </div>
  );
}
// what she does for the loop if you enter her: one short line (`saves`: the careers she saves, null: not known). A
// roll's quick zero is the plan's routes' only (compare.ts `rollReq`), always provisional until the exact comparison
// answers: it reads "no saving found yet"
function verdictText(sim: SimulationView, d: DraftView, next: Career | undefined, saves: number | null) {
  const n = settings.value.targets.length,
    fullyLooped = sim.full && d.parents.length === 2 && d.parents.every((p) => isFull(p, settings.value.targets)),
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
/** the comparison's verdict in one line: what she does for the loop, where her saving lands, or a rise said as such */
function ComparisonVerdict({ sim, d, next }: { sim: SimulationView; d: DraftView; next: Career | undefined }) {
  const { now, withHer } = sim.v0 || { now: null, withHer: null },
    saves = now != null && withHer != null ? now - withHer : null,
    head = verdictText(sim, d, next, saves);
  return (
    <p class="text-note text-gm-ink2 stale:opacity-50 leading-snug" id="sim-verdict">
      <strong class="text-title text-gm-ink mb-0.5 block font-black">{head}</strong>
      {/^Saves /.test(head) && sim.where ? sim.where : null}
      {/* a rise (never from a roll's quick figures, which can't rise): shown as it is, and said to be the planner's */}
      {saves != null && saves <= -0.5
        ? `The planner's estimate rises by ${(-saves).toFixed(1)} careers with her, a limit of its route (it offers each uma's few best careers as sides, and she can push the one it used out of them); leaving her out costs nothing.`
        : null}
    </p>
  );
}
// the comparison panel's parts: each top-level one faded while the figures are for her result before an edit (the
// panel's data-stale, the theme's `stale:`), but the note saying so; a run's line (the runner beside its words: `plain`,
// in the site's ink, as it showed; `refine`, smaller, in the game's lighter brown); the gold-ruled note; the odds rows
const RUN_NOTE = {
  plain: 'text-ink stale:opacity-50 flex items-start gap-2 font-medium tabular-nums',
  refine: 'text-note text-gm-ink2 stale:opacity-50 flex items-start gap-2 font-medium tabular-nums',
} as const;
const RUNNER_PLACE = '-mt-1.5 -mb-1.25';
const NOTE = 'border-gold bg-gold-soft text-ink border-l-5 px-3 py-2';
const ODDS = 'stale:opacity-50 grid grid-cols-1 gap-1';
// the comparison on show: the verdict in one line, then the two figures it rests on (now -> with her), then the best
// career with her
function ComparisonResult({ sim, d }: { sim: SimulationView; d: DraftView }) {
  const n = settings.value.targets.length,
    { now: careersNow, withHer: careersWithHer } = sim.v0 || { now: null, withHer: null },
    next = sim.r?.results[0] as Career | undefined, // (none yet while the quick search for the best career with her runs)
    rose = careersNow != null && careersWithHer != null && careersNow - careersWithHer <= -0.5;
  // a parent of the next career with her: her by the place the comparison's job gives her (last, compare.ts), the
  // others by their number in the pool now
  const parentNumber = (i: number) => (i === answerPool.value.length ? `#${i + 1}` : legacyNumber(i));
  const nextWithHer = sim.pickA
    ? `. With her the planner's next career is ${sim.pickA.trainee.name} (${sim.pickA.parents.map(parentNumber).join(' + ')}).`
    : '.';
  const fromTable = sim.quick
    ? " From the planner's table for this career (her sparks exact, her G1 wins at their chances; only the routes the plan kept); Compare checks this exact roll."
    : '';
  const held = !!(sim.refining || sim.standing);
  const loop = (
    <Figure
      label="Careers to loop"
      now={careersNow != null ? `about ${fmtN(careersNow)}` : '?'}
      withHer={careersWithHer != null ? fmtN(careersWithHer) : '?'}
      dir={rose ? 'same' : direction(careersNow, careersWithHer, true)}
      held={held}
      title={`Expected careers to a fully looped parent: with your veterans now → with her too${nextWithHer}${fromTable}`}
    />
  );
  // with her priced under today's fit (`fixedFit`): the plan after Add is fitted to the pool with her, so it can show
  // other figures (more careers or fewer), which this one line says
  const fitNote = sim.fixedFit && careersNow != null && (
    <p class="text-note text-gm-ink2 stale:opacity-50 leading-snug" data-fit="">
      Counted with today's plan; after Add the planner refits it to your veterans with her, so its figures can differ.
    </p>
  );
  const baseline = next && must(sim.baseline, 'figures with her best career have a baseline');
  return (
    <>
      {sim.stale && <p class={NOTE}>These figures are for her result before your last edit. Compare to update them.</p>}
      <ComparisonVerdict sim={sim} d={d} next={next} />
      {(sim.refining || sim.standing || sim.provisional) && (
        <p class={RUN_NOTE.refine} data-run="refine">
          <Runner cls={RUNNER_PLACE} />
          <span class="min-w-0">{sim.provisional ? 'Checking…' : 'Working out how many careers it saves…'}</span>
        </p>
      )}
      {/* the best career with her as a parent: its odds against today's best, then folded its portraits and deck */}
      {next && baseline ? (
        <>
          <div class={ODDS} data-odds="">
            {loop}
            <Figure
              label={`All ${n} odds`}
              now={fmtIn(baseline.p_all)}
              withHer={fmtIn(next.p_all)}
              dir={direction(baseline.p_all, next.p_all, false)}
              held={held}
              title={`All ${n}: today's best career → the best career with her as a parent (${pct(baseline.p_all)} → ${pct(next.p_all)}; ${n - 1} or more: ${fmtIn(baseline.p_minus1)} → ${fmtIn(next.p_minus1)})`}
            />
          </div>
          {fitNote}
          <details class="stale:opacity-50">
            <summary class="text-note cursor-pointer font-bold max-md:py-2.5">Best career with her</summary>
            <div class="text-note my-1.5 flex flex-wrap items-center gap-1">
              <Faces
                tid={next.trainee.id}
                outfit={next.trainee.outfits[0]?.id}
                outfitLabel={next.trainee.outfits[0]?.label || ''}
                parents={next.parents.map((i) => parentFace(i, d))}
              />
            </div>
            <div class={cx(panel, 'relative grid gap-2 p-2.5 max-md:p-2')}>
              <b class="pr-8">Deck</b>
              <DeckCards deck={next.deck} fit="mini" />
            </div>
          </details>
        </>
      ) : (
        <>
          <div class={ODDS} data-odds="">
            {loop}
          </div>
          {fitNote}
          <p class={RUN_NOTE.plain} data-run="best">
            <Runner cls={RUNNER_PLACE} />
            <span class="min-w-0">Finding the best career with her…</span>
          </p>
        </>
      )}
    </>
  );
}
// the panel beside her card: a mini header carrying the chance a roll of this career improves your loop (the plan's
// `lift`: her outcomes that lower V), then the comparison (before any, a line saying what goes here)
function ComparisonPanel({ sim, d, r }: { sim: SimulationView; d: DraftView; r: Career }) {
  const busy = sim.state === 'busy',
    { planState } = answer.value,
    opt = planState.state === 'done' ? planState.r.options?.find((o) => keyOf(o) === keyOf(r)) : null;
  return (
    <div
      class="bg-gm-panel border-gm-edge shadow-panel font-game text-gm-ink text-ui grid gap-2 rounded-xl border p-2.5"
      id="sim-result"
      role="status"
      data-stale={sim.stale ? '' : undefined}
    >
      <div class="art-bar text-shadow-outline text-body stale:opacity-50 -mx-2.5 -mt-2.5 flex items-baseline justify-between gap-2 rounded-t-xl py-0.75 pr-6.5 pl-2.5 font-black text-white">
        <span>Comparison</span>
        {opt && opt.lift != null && (
          <span
            class="text-note font-bold whitespace-nowrap"
            id="sim-chance"
            title={`The planner's chance that a roll of this career lowers the careers to a fully looped parent (${pct(opt.lift)})`}
          >
            {opt.lift > 0 ? `1/${Math.max(1, Math.round(1 / opt.lift)).toLocaleString()} to improve` : 'none improve'}
          </span>
        )}
      </div>
      {!busy && !sim.error && !sim.r && (
        <p class="text-ui text-gm-ink2 stale:opacity-50 leading-snug">
          Compare or reroll to see if she improves your loop.
        </p>
      )}
      {busy ? (
        <p class={RUN_NOTE.plain}>
          <Runner cls={RUNNER_PLACE} />
          <span class="min-w-0">Comparing…</span>
        </p>
      ) : sim.error ? (
        <p class={NOTE}>{sim.error}</p>
      ) : sim.r ? (
        <ComparisonResult sim={sim} d={d} />
      ) : null}
    </div>
  );
}

// ---------- the popup
// the analysis: its bar, her card under it and the comparison beside them once the popup's body (a container) is 820 px
// wide (the bar and card 600 px at most), the card's top aligned from 900 px; one column under that. (Its 22 px top
// margin is the old sheet's section margin, which reached the popup.)
const SECTION =
  'mt-5.5 grid min-w-0 gap-3 @min-[820px]:grid-cols-[minmax(0,600px)_minmax(220px,1fr)] @min-[820px]:gap-x-4 @min-[900px]:items-start';
/** The veteran analysis: her card (./Details.tsx; she starts as a random result of this career) with its actions, and
 *  the comparison panel beside it. */
function Analysis({ comparison }: { comparison: Comparison }) {
  // (the career on show reads the places, and her card her parents' records: the veterans store's signals; her own
  // record redraws by its identity, `simulation`)
  const sim = simulation.value,
    d = draft.value,
    r = shown(),
    searching = runSearching.value, // (not runView: a plan's progress would draw her card again each time)
    section = useRef<HTMLElement>(null),
    wins = useRef<HTMLButtonElement>(null);
  // her spark names shrink to fit (details.ts fitSparks: it measures the names on show and skips a closed popup's;
  // TryRunDialog fits them again on opening)
  useLayoutEffect(() => {
    if (section.current) fitSparks(section.current);
  });
  if (!r) return null;
  if (!d)
    return (
      <section class={SECTION} id="simulation" ref={section}>
        <GameBar shape="card" class="w-full max-w-160">
          Veteran analysis
        </GameBar>
      </section>
    );
  const spec: DetailsSpec = {
    d,
    r,
    races: scheduledRaces(r, careerTables().races),
    raceName: (ix) => careerTables().races[ix] || `G1 ${ix}`,
    targets: settings.value.targets.map((t) => t.group),
    legacies: legaciesOf(r, legacyVet, legacyNumber, (id) => openNode({ kind: 'v', id })),
    numberOf: legacyNumber,
  };
  // Compare and a roll stay usable while a comparison runs: either one restarts it (not during a run, whose answer
  // replaces the career on show)
  const actions = (
    <div class="flex flex-wrap justify-center gap-1.5">
      <Button
        variant="primary"
        size="note"
        id="sim-compare"
        data-k="sim:compare"
        disabled={searching}
        onClick={comparison.compare}
      >
        Compare
      </Button>
      <Button size="note" id="sim-roll" data-k="sim:roll" disabled={searching} onClick={comparison.roll}>
        Reroll
      </Button>
      <Button size="note" id="sim-add" disabled={searching} onClick={addDraft}>
        Add to veterans
      </Button>
    </div>
  );
  const winsButton = (
    <Button
      ref={wins}
      size="caption"
      id="sim-wins"
      data-k="sim:wins"
      aria-haspopup="dialog"
      onClick={() => openDraftWins(d, r, () => wins.current?.focus())}
    >
      {`${d.wins.length ? d.wins.length : 'No'} G1 win${d.wins.length === 1 ? '' : 's'} · edit`}
    </Button>
  );
  return (
    <section class={SECTION} id="simulation" ref={section}>
      <GameBar shape="card" class="w-full max-w-160">
        Veteran analysis
      </GameBar>
      <Details
        spec={spec}
        actions={actions}
        winsButton={winsButton}
        class="-mt-3 w-full max-w-160 @min-[900px]:col-start-1 @min-[900px]:row-start-2"
      />
      <div class="grid min-w-0 content-start gap-2 @min-[820px]:col-start-2 @min-[820px]:row-span-2 @min-[820px]:row-start-1">
        <ComparisonPanel sim={sim} d={d} r={r} />
      </div>
    </section>
  );
}
const tryRunOpen = signal(false);
/** Try a run's popup opens (the tree's button) */
export function openTryRun() {
  tryRunOpen.value = true;
}
const closeTryRun = () => {
  tryRunOpen.value = false;
};
// what Try a run draws: her comparison and her draft (a new one draws it again after a failure)
const simulationDrawn = computed(() => [simulation.value, draft.value]);
/** Try a run's popup (ui/Dialog; the career entry renders it into #sim-root): the analysis behind its guard. Once it is
 *  on show (its dialog's own layout effect has opened it), her spark names are measured. */
export function TryRunDialog({ comparison }: { comparison: Comparison }) {
  const open = tryRunOpen.value,
    body = useRef<HTMLDivElement>(null);
  // biome-ignore lint/correctness/useExhaustiveDependencies: once per opening (`open` is a module signal's value, which Biome reads as constant)
  useLayoutEffect(() => {
    if (open && body.current) fitSparks(body.current);
  }, [open]);
  return (
    <Dialog size="xl" id="sim-dlg" tall labelledBy="sim-title" open={open} onClose={closeTryRun}>
      <DialogPanel>
        <DialogHead id="sim-title" title="Try a run" close="Close" onClose={closeTryRun} />
        <div
          class="@container grid max-h-[calc(100dvh-110px)] min-w-0 gap-2.5 overflow-y-auto overscroll-contain pr-1"
          id="sim-top"
          ref={body}
        >
          <Guard what="Try a run" also={simulationDrawn}>
            <Analysis comparison={comparison} />
          </Guard>
        </div>
      </DialogPanel>
    </Dialog>
  );
}
