// ---------- the loop search's job, from the settings it is given (it reads no store: the caller passes their values) ----------
// buildJob: the job for settings `s`, or why they can't be searched (blockedStatus says it); jobKey: the key a job's
// result is kept and recognised by; doneText: the status line of a finished search.
import { ASSETS, gameData } from '../data.ts';
import { deckPart, isOwned, skillsPart } from '../job.ts';
import type { jobTargets } from '../job.ts';
import { ERA } from '../saved.ts';
import type { SettingsView } from '../settings.ts';
import type { Request } from '../gen/solver.ts';
import type { LoopReport, Status } from './types.ts';

/** a target as the job sends it: its skill group, and 2 when the scenario hands out its gold */
export type JobTarget = ReturnType<typeof jobTargets>[number];
/** the loop search's job: the solver's `Request` for `deck` (its targets and roster, `skills`, and its deck search's
 *  settings, `deck`; generated). The field order `buildJob` writes is part of its key (jobKey), which the kept result
 *  and the suite's recorded solver replies are found by: keep it. */
export type DeckJob = Extract<Request, { cmd: 'deck' }>;
/** settings the search cannot start from: no target, or fewer than 3 umas owned (`owned` of them) */
export type Blocked = { blocked: 'targets' } | { blocked: 'umas'; owned: number };
export type Built = { job: DeckJob; targets: JobTarget[] } | Blocked;

/** the search job for settings `s`, or why there is none */
export function buildJob(s: SettingsView): Built {
  if (!s.targets.length) return { blocked: 'targets' };
  const exclude = gameData()
    .umas.filter((u) => !isOwned(u, s))
    .map((u) => u.id);
  const owned = gameData().umas.length - exclude.length;
  if (owned < 3) return { blocked: 'umas', owned };
  const skills = skillsPart(s, exclude);
  const job: DeckJob = {
    cmd: 'deck',
    skills,
    deck: { ...deckPart(s), top_decks: +s.decksTop, per_uma: s.perUma !== false },
  };
  return { job, targets: skills.targets };
}
/** what the status line says of blocked settings, with the button that fixes it (`loadExample`: presets.ts's) */
export const blockedStatus = (blocked: Blocked, loadExample: () => void): Status =>
  blocked.blocked === 'targets'
    ? {
        kind: 'empty',
        text: 'Add a target skill to search.',
        act: { label: 'Load the 8/8 example targets', click: 't-preset', run: loadExample },
      }
    : {
        kind: 'empty',
        text: `Mark at least 3 umas as owned to search (${blocked.owned} owned now).`,
        act: { label: 'Mark all umas owned', click: 'r-all' },
      };
// A job's result is kept with its key: the solver's and the era data's fingerprints, what is still held back for a
// later start (data.ts gameData().later) and the job, so a reload with unchanged settings finds it (`era`: the page's)
export const jobKey = (job: DeckJob, era = ERA) =>
  JSON.stringify([ASSETS.wasm, ASSETS.engine[era === 'gm' ? 'gm' : 'global'], gameData().later, job]);

// What the result is checked against (the solver's `exact`: rho = the cap it is certified under, rho_target the cap
// aimed for; see the model notes on /about): nothing when fully checked, else a short caveat
function certText(exact: LoopReport['exact'] | null | undefined) {
  if (!exact || exact.rho >= exact.rho_target) return '';
  if (exact.rho > 1)
    return ` · exact check: decks within ${Math.round((1 - 1 / exact.rho) * 100)}% of it (ran out of time)`;
  return ' · not checked exactly (ran out of time)';
}
/** the status line of a finished search: what it checked, in how long, and what it could not prove */
export const doneText = (r: LoopReport, ms: number): Status => ({
  kind: 'done',
  text: `Checked ${r.stats.decks_built.toLocaleString()} decks / ${r.foursomes.toLocaleString()} loops in ${(ms / 1000).toFixed(1)} s${r.proven === false ? ' · cut short: not proven best' : ''}${certText(r.exact)}`,
});
