// What the page says when a career job's searches proved less than they could, or proved it only under a named
// assumption, from the fields the solver reports (career/run.rs `search_output`, `note_job_certificates`; the plan's
// rows, plan/output.rs). Unproven causes: the main search's `unproven` reasons and the job's `job_unproven` (`work_cap`,
// its work budget ran out; `band_overflow`, a deck band too large to check in full, certified at a lower cap only;
// `cap_breach`, a scored deck past its class's assumed cap, in this job or an earlier one on the same deck class,
// `cap_breached`), and the job's counts (`band_overflows`, `cap_exceeded`, `cap_rerun`). A cause is named only when a
// field establishes it; with none, the text says so. Named assumptions (`assumes`, `job_assumes`, a row's `assumes`:
// `two_feeder_cap`, `multi_feeder_cap`) qualify a proof and are shown apart from the unproven causes: a proof under
// them is still a proof. The browser suite's `career` group checks these texts on constructed reports.

interface ProofFields {
  /** the best odds' order is certified */
  proven_top?: boolean;
  /** false: a search of the job was cut short, or certified at a lower deck cap only */
  complete?: boolean;
  /** the career the job named, scored as it is (career/given.rs): no search ran */
  given?: boolean;
  /** why the main search is not complete */
  unproven?: readonly string[];
  /** why any search of the job is not complete */
  job_unproven?: readonly string[];
  /** the named assumptions the main search's proof rests on */
  assumes?: readonly string[];
  /** the named assumptions any search of the job rests on */
  job_assumes?: readonly string[];
  band_overflows?: number;
  /** per deck class past its assumed cap: [class, ratio, cap] */
  cap_exceeded?: readonly (readonly [number, number, number])[];
  /** capped deck searches of the job that scored a deck past its cap */
  cap_breaches?: number;
  /** trainees (chara ids) whose deck class breached its cap, in this job or before */
  cap_breached?: readonly number[];
  cap_rerun?: { readonly searches: number; readonly unproven: number; readonly unanswered: number };
}

/** the unproven reasons the main search and the job name */
const reasons = (r: ProofFields): Set<string> => new Set([...(r.unproven || []), ...(r.job_unproven || [])]);

/** the causes the job's report establishes */
export const unprovenWhy = (r: ProofFields): string[] => {
  const named = reasons(r);
  // (a count a reply lacks, from a solver before it was reported, is none)
  const breachHere = !!r.cap_exceeded?.length || (r.cap_breaches ?? 0) > 0;
  const breachBefore = !breachHere && (named.has('cap_breach') || !!r.cap_breached?.length);
  return [
    ...(named.has('work_cap') ? ['a search ran out of its work budget'] : []),
    ...((r.band_overflows ?? 0) > 0 || named.has('band_overflow')
      ? ['a deck band was too large to check in full']
      : []),
    ...(breachHere ? ["a deck beat its class's assumed cap"] : []),
    ...(breachBefore ? ["a deck beat its class's assumed cap in an earlier search for these umas"] : []),
    ...((r.cap_rerun?.unanswered ?? 0) > 0 ? ['a breach of a cap was not checked without the cap'] : []),
    ...((r.cap_rerun?.unproven ?? 0) > 0 ? ['the check without the cap did not finish'] : []),
  ];
};

export const whyText = (why: readonly string[]): string =>
  why.length ? `reported: ${why.join('; ')}` : 'no cause reported; its work budget is one possibility';

/** What each named assumption means, as the page states it */
const ASSUMED: Record<string, string> = {
  two_feeder_cap: 'the assumed class cap for two-feeder decks',
  multi_feeder_cap: 'the assumed bound for nested-chain decks beside other feeders',
};

/** the named assumptions, as text (unknown names as they are) */
const assumedWhy = (names: readonly string[] | undefined): string[] =>
  [...new Set(names || [])].map((name) => ASSUMED[name] || name);

/** '' without any */
const assumedText = (names: readonly string[] | undefined): string => {
  const why = assumedWhy(names);
  return why.length ? `proven under ${why.join(' and ')}` : '';
};

/** the search of the best odds proved its order and saw no breach */
const proven = (r: ProofFields): boolean => !!r.proven_top && r.complete !== false && !r.cap_exceeded?.length;

/** The status line's caveat for the search of the best odds ('' when it proved its order under no named assumption
 * and saw no breach). */
export function searchCaveat(r: ProofFields): string {
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
 * plan table adds no note for them: a proof under a named assumption is still a proof. */
export function planCaveat(planAssumes: readonly string[] | undefined, r: ProofFields): string {
  const stated = proven(r) ? assumedWhy([...(r.assumes || []), ...(r.job_assumes || [])]) : [];
  const extra = assumedWhy(planAssumes).filter((why) => !stated.includes(why));
  return extra.length ? ` The plan rests on ${extra.join(' and ')}.` : '';
}
