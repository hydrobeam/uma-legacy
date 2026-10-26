// What the page says when a career job's searches proved less than they could, or proved it only under a named
// assumption, from the fields the solver reports (career/run.rs `search_output`, `note_job_certificates`; the plan's
// rows, plan/output.rs). Unproven causes: the main search's `unproven` reasons and the job's `job_unproven` (`work_cap`,
// its work budget ran out; `band_overflow`, a deck band too large to check in full, certified at a lower cap only;
// `cap_breach`, a scored deck past its class's assumed cap, in this job or an earlier one on the same deck class,
// `cap_breached`), and the job's counts (`band_overflows`, `cap_exceeded`, `cap_rerun`). A cause is named only when a
// field establishes it; with none, the text says so. Named assumptions (`assumes`, `job_assumes`, a row's `assumes`:
// `two_feeder_cap`, `multi_feeder_cap`) qualify a proof and are shown apart from the unproven causes: a proof under
// them is still a proof. The browser suite's `career` group checks these texts on constructed reports.

/**
 * @typedef {object} ProofFields
 * @property {boolean} [proven_top]  the best odds' order is certified
 * @property {boolean} [complete]  false: a search of the job was cut short, or certified at a lower deck cap only
 * @property {boolean} [given]  the career the job named, scored as it is (career/given.rs): no search ran
 * @property {string[]} [unproven]  why the main search is not complete
 * @property {string[]} [job_unproven]  why any search of the job is not complete
 * @property {string[]} [assumes]  the named assumptions the main search's proof rests on
 * @property {string[]} [job_assumes]  the named assumptions any search of the job rests on
 * @property {number} [band_overflows]
 * @property {[number, number, number][]} [cap_exceeded]  per deck class past its assumed cap: [class, ratio, cap]
 * @property {number} [cap_breaches]  capped deck searches of the job that scored a deck past its cap
 * @property {number[]} [cap_breached]  trainees (chara ids) whose deck class breached its cap, in this job or before
 * @property {{searches: number, unproven: number, unanswered: number}} [cap_rerun]
 */

/** @type {(r: ProofFields) => Set<string>} the unproven reasons the main search and the job name */
const reasons = (r) => new Set([...(r.unproven || []), ...(r.job_unproven || [])]);

/** @type {(r: ProofFields) => string[]} the causes the job's report establishes */
export const unprovenWhy = (r) => {
  const named = reasons(r);
  const breachHere = !!r.cap_exceeded?.length || r.cap_breaches > 0;
  const breachBefore = !breachHere && (named.has('cap_breach') || !!r.cap_breached?.length);
  return [
    ...(named.has('work_cap') ? ['a search ran out of its work budget'] : []),
    ...(r.band_overflows > 0 || named.has('band_overflow') ? ['a deck band was too large to check in full'] : []),
    ...(breachHere ? ["a deck beat its class's assumed cap"] : []),
    ...(breachBefore ? ["a deck beat its class's assumed cap in an earlier search for these umas"] : []),
    ...(r.cap_rerun?.unanswered > 0 ? ['a breach of a cap was not checked without the cap'] : []),
    ...(r.cap_rerun?.unproven > 0 ? ['the check without the cap did not finish'] : []),
  ];
};

/** @type {(why: string[]) => string} */
export const whyText = (why) =>
  why.length ? `reported: ${why.join('; ')}` : 'no cause reported; its work budget is one possibility';

/** What each named assumption means, as the page states it
 * @type {Record<string, string>} */
const ASSUMED = {
  two_feeder_cap: 'the assumed class cap for two-feeder decks',
  multi_feeder_cap: 'the assumed bound for nested-chain decks beside other feeders',
};

/** @type {(names: string[] | undefined) => string[]} the named assumptions, as text (unknown names as they are) */
const assumedWhy = (names) => [...new Set(names || [])].map((name) => ASSUMED[name] || name);

/** @type {(names: string[] | undefined) => string} '' without any */
const assumedText = (names) => {
  const why = assumedWhy(names);
  return why.length ? `proven under ${why.join(' and ')}` : '';
};

/** @type {(r: ProofFields) => boolean} the search of the best odds proved its order and saw no breach */
const proven = (r) => !!r.proven_top && r.complete !== false && !r.cap_exceeded?.length;

/** The status line's caveat for the search of the best odds ('' when it proved its order under no named assumption
 * and saw no breach).
 * @type {(r: ProofFields) => string} */
export function searchCaveat(r) {
  if (r.given) return ' The career you chose, scored as it is: no search compared it with other careers or decks.';
  if (!(r.proven_top && r.complete !== false))
    return ` The exact check of the best odds could not finish (${whyText(unprovenWhy(r))}): their order may not be final.`;
  if (r.cap_exceeded?.length)
    return " A deck beat its class's assumed cap, which the exact check relies on: the best odds' order may not be final.";
  const assumed = assumedText(r.assumes);
  // (the job's other searches may rest on assumptions the main search's does not)
  const jobOnly = assumedWhy(r.job_assumes).filter((why) => !assumedWhy(r.assumes).includes(why));
  return (
    (assumed ? ` Their order is ${assumed}.` : '') +
    (jobOnly.length ? ` The job's answer rests on ${jobOnly.join(' and ')}.` : '')
  );
}

/** The plan's caveat on the status line: the named assumptions the plan job's searches rest on (its `job_assumes`) that
 * the search's caveat (`searchCaveat` of `r`, the search job's report) does not state already ('' without any). The
 * plan table adds no note for them: a proof under a named assumption is still a proof.
 * @type {(planAssumes: string[] | undefined, r: ProofFields) => string} */
export function planCaveat(planAssumes, r) {
  const stated = proven(r) ? assumedWhy([...(r.assumes || []), ...(r.job_assumes || [])]) : [];
  const extra = assumedWhy(planAssumes).filter((why) => !stated.includes(why));
  return extra.length ? ` The plan rests on ${extra.join(' and ')}.` : '';
}
