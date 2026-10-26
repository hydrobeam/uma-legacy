// What the page says when a career job's searches proved less than they could, from the fields the solver reports for
// every search of the job (career/run.rs `note_job_certificates`): a deck band too large to check in full (certified at
// a lower cap only), a scored deck past its class's assumed cap (reported whether or not the cap-free rerun settled
// it), a breach left unanswered, a cap-free rerun that proved nothing. The report is the job's, not one search's, and
// no field reports the work budget, so a cause is named only when a field establishes it; with none, the text says
// so. The browser suite's `career` group checks these texts on constructed reports.

/**
 * @typedef {object} ProofFields
 * @property {boolean} [proven_top]  the best odds' order is certified
 * @property {boolean} [complete]  false: a search of the job was cut short, or certified at a lower deck cap only
 * @property {number} [band_overflows]
 * @property {[number, number, number][]} [cap_exceeded]  per deck class past its assumed cap: [class, ratio, cap]
 * @property {{searches: number, unproven: number, unanswered: number}} [cap_rerun]
 */

/** @type {(r: ProofFields) => string[]} the causes the job's report establishes */
export const unprovenWhy = (r) => [
  ...(r.band_overflows > 0 ? ['a deck band was too large to check in full'] : []),
  ...(r.cap_exceeded?.length ? ["a deck beat its class's assumed cap"] : []),
  ...(r.cap_rerun?.unanswered > 0 ? ['a breach of a cap was not checked without the cap'] : []),
  ...(r.cap_rerun?.unproven > 0 ? ['the check without the cap did not finish'] : []),
];

/** @type {(why: string[]) => string} */
export const whyText = (why) =>
  why.length ? `reported: ${why.join('; ')}` : 'no cause reported; its work budget is one possibility';

/** The status line's caveat for the search of the best odds ('' when it proved its order and saw no breach).
 * @type {(r: ProofFields) => string} */
export function searchCaveat(r) {
  if (!(r.proven_top && r.complete !== false))
    return ` The exact check of the best odds could not finish (${whyText(unprovenWhy(r))}): their order may not be final.`;
  if (r.cap_exceeded?.length)
    return " A deck beat its class's assumed cap, which the exact check relies on: the best odds' order may not be final.";
  return '';
}
