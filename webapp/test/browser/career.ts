// The career page in the browser flows (career.spec.ts): the hooks on its solver's messages, what it shows as it
// changes, and the steps the flows share (the scenario's export in, the veterans dialog, the node popup, Try a run).
import path from 'node:path';
import { expect, type Page } from 'playwright/test';
import type { CareerReply, Request } from '../../src/gen/solver.ts';
import { careerDone, closeEditor, openEditor, SOLVER_WAIT } from './pages.ts';

/** What the page shows at one change: busy or not, the phase named, the trainee and her veteran's sparks, dimmed. */
interface Snap {
  busy: boolean;
  phase: string;
  who: string;
  draft: string;
  stale: boolean;
  k: string;
}
declare global {
  interface Window {
    /** the jobs sent (the plan's robustness check aside: `__checkJobs`) and the career results */
    __careerJobs: Request[];
    __careerResults: CareerReply[];
    __checkJobs: Request[];
    /** a stand-in for a roll's saving, put over the solver's before the page reads it */
    __rollPatch: { y: number } | null;
    /** a stand-in for a plan's `scoring_complete`, the same way */
    __planIncomplete: boolean;
    /** what the page shows, each time it changes */
    __seq: Snap[];
  }
}

/** career-small's four veterans as the game exports them (test/fixtures/solver/export-small.json: made up) */
export const EXPORT = path.resolve(import.meta.dirname, '../fixtures/solver/export-small.json');

/** (runs in the page) the hooks: the jobs and results, the patches, the snapshots of what shows */
export function careerHooks() {
  window.__careerJobs = [];
  window.__careerResults = [];
  window.__checkJobs = [];
  window.__rollPatch = null;
  window.__planIncomplete = false;
  const hooks = window.__workerHooks;
  hooks.message.push((d) => {
    if (d.type !== 'result') return;
    const r = d.r;
    if (r.kind === 'roll' || r.kind === 'search') {
      const roll = r.kind === 'roll' ? r.roll : r.next?.kind === 'plan' ? r.next.roll : undefined;
      if (window.__rollPatch && roll?.kind === 'answer') roll.outcome = { kind: 'saving', y: window.__rollPatch.y };
      if (window.__planIncomplete && r.kind === 'search' && r.next?.kind === 'plan') r.next.scoring_complete = false;
    }
    if (
      r.kind !== 'check' &&
      (r.kind === 'roll' || r.kind === 'search' || r.kind === 'refusal' || r.kind === 'carried')
    )
      window.__careerResults.push(r);
  });
  hooks.post.push((m) => {
    if (m.type !== 'run') return undefined;
    (m.job.cmd === 'career' && m.job.career.next?.robust ? window.__checkJobs : window.__careerJobs).push(m.job);
    return undefined;
  });
  window.__seq = [];
  const snap = () => {
    const n = document.getElementById('run-note');
    if (!n) return;
    const text = n.textContent ?? '';
    const s = {
      busy: !!n.querySelector('[data-spinner]'),
      phase: /Planning your next career…/.test(text) ? 'plan' : /Searching careers…/.test(text) ? 'search' : '',
      who: document.querySelector('#tree [data-node="trainee"] [data-nm]')?.textContent ?? '',
      draft: document.querySelector('#simulation [data-details]')
        ? `v:${[...document.querySelectorAll('#simulation [aria-label="Her sparks"] [data-bar]')].map((b) => b.getAttribute('aria-label')).join('|')}`
        : '',
      stale: document.getElementById('results')?.hasAttribute('data-stale') ?? false,
    };
    const k = JSON.stringify(s);
    if (window.__seq.at(-1)?.k !== k) window.__seq.push({ ...s, k });
  };
  new MutationObserver(snap).observe(document, {
    subtree: true,
    childList: true,
    characterData: true,
    attributes: true,
    attributeFilter: ['class', 'data-stale'],
  });
}

/** The answer on show: the trainee's name and the veteran you got's sparks. */
export const showsNow = (page: Page) =>
  page.evaluate(
    () =>
      `${document.querySelector('#tree [data-node="trainee"] [data-nm]')?.textContent ?? ''}/${
        document.querySelector('#simulation [data-details]')
          ? `v:${[...document.querySelectorAll('#simulation [aria-label="Her sparks"] [data-bar]')].map((b) => b.getAttribute('aria-label')).join('|')}`
          : ''
      }`,
  );

/** Whether the run just ended showed one answer: while busy, the answer it had before (`pre`), dimmed from its job's
 * start, and the phases named; then the one it ends on. Its phases, in order. */
export async function oneAnswer(page: Page, pre: string): Promise<{ ok: boolean; phases: string[]; detail: string }> {
  const seq = await page.evaluate(() => window.__seq);
  const busy = seq.filter((s) => s.busy);
  const end = seq.at(-1);
  const shows = (s: Snap) => `${s.who}/${s.draft}`;
  const phases = busy.map((s) => s.phase).filter((p, i, a) => p && p !== a[i - 1]);
  const dim = busy.findIndex((s) => s.stale);
  const last = busy.at(-1);
  const ok =
    busy.length > 0 &&
    !!end &&
    !end.busy &&
    busy.every((s) => shows(s) === pre) &&
    (!busy.some((s) => s.draft) || (dim >= 0 && busy.slice(dim).every((s) => !s.draft || s.stale))) &&
    end.draft !== '' &&
    !end.stale &&
    !!last &&
    seq.slice(seq.indexOf(last) + 1).every((s) => shows(s) === shows(end));
  const detail = `phases ${phases.join(' > ')} · before ${pre.slice(0, 60)} · busy shows ${[...new Set(busy.map(shows))].join(' | ').slice(0, 120)} · end ${end ? shows(end).slice(0, 60) : '(none)'}`;
  return { ok, phases, detail };
}
export const resetSeq = (page: Page) =>
  page.evaluate(() => {
    window.__seq = [];
  });

/** The status line says the plan is done. */
export const planned = (page: Page) =>
  expect(page.locator('#run-note')).toHaveText(/Planned in/, { timeout: SOLVER_WAIT });

/** The export of career-small's four veterans, imported through the veterans dialog's file button. */
export async function importExport(page: Page) {
  await openEditor(page, 'vets');
  await page.locator('#v-file').setInputFiles(EXPORT);
  await expect(page.locator('#v-msg')).toHaveText(/Imported 4 veterans/);
}
export const vetsShut = (page: Page) => closeEditor(page, 'vets');

/** The tree's portrait `data-k=key` opens its node popup. */
export async function node(page: Page, key: string) {
  await vetsShut(page);
  await page.click(`#tree [data-k="${key}"]`);
  await expect(page.locator('#node-dlg')).toHaveAttribute('open');
}

/** Try a run's popup (her card and its comparison), opened if shut. */
export async function simOpen(page: Page) {
  if (!(await page.evaluate(() => document.querySelector('#sim-dlg[open]') !== null))) await page.click('#sim-try');
  await expect(page.locator('#sim-dlg')).toHaveAttribute('open');
}
/** The veteran you got's Uma Stan at `n` stars (added first if she lacks it). */
export async function simStars(page: Page, n: number) {
  await simOpen(page);
  const spark = page.locator('[data-k="sim:s20159"]');
  if (!(await spark.count())) {
    await page.locator('#simulation [data-k$=":add"]').click();
    await page.locator('#add-pop button', { hasText: 'Uma Stan' }).click();
  }
  for (let i = 0; i < 3; i++) {
    const stars = /(\d) star/.exec((await spark.getAttribute('aria-label')) ?? '')?.[1];
    if (Number(stars) === n) return;
    await spark.click();
  }
}
/** The career page of career-small (its four veterans seeded: `test.use({ scenario: 'career-small' })`), their
 * first plan on show. */
export async function withPlan(page: Page) {
  await page.goto('/');
  await careerDone(page);
  await planned(page);
  await vetsShut(page);
}
/** The career jobs sent so far that match `kind`: a comparison, a plan job, a roll's. */
export const jobCount = (page: Page, kind: 'compare' | 'compareNext' | 'plan') =>
  page.evaluate(
    (k) =>
      window.__careerJobs.filter((j) => {
        if (j.cmd !== 'career') return false;
        const c = j.career;
        return k === 'compare' ? !!c.compare : k === 'compareNext' ? !!c.compare && !!c.next : !!c.next && !c.compare;
      }).length,
    kind,
  );
/** Your veterans as stored. */
export const storedVets = (page: Page) =>
  page.evaluate(() => JSON.stringify(JSON.parse(localStorage.getItem('llp:career') ?? '{}').global?.vets ?? null));
