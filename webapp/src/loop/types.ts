// ---------- the loop page's types: the solver's loop report as the page reads it, and the status line's record ----------
// Types only (no values), shared by the page's model, store, run machine and views; the report's fields are the
// solver's (loop_planner/search/report.rs, deck/pdeck.rs).
import type { ScheduleRow } from '../solver.ts';

/**
 * A deck in a result (the solver's `PDeck::out`, deck/pdeck.rs). Parallel lists: `cards[i]` is named with ` (borrowed)` or
 * ` (end chain at link N)` appended where that applies; `card_ids[i]` is its id (plain pals have a name but no entry).
 */
export interface ResultDeck {
  cards: string[];
  card_ids: number[];
  /** the cards in it that are pals, by name */
  pal_cards?: string[];
  /** per card: the chain link to end its story at (0: none) */
  end_chain_at?: number[];
  /** per card: the skill to prioritise at its chain link */
  choices?: string[];
  /** per target: the chance the deck hints its white, its gold */
  odds: { group: number; white: number; gold: number }[];
  /** the plain pals' ids */
  plain_ids?: number[];
  /** how many pals it holds */
  pals?: number;
  /** the borrowed card, by name (null: none) */
  borrowed?: string | null;
}
/** a member of a loop (chart.rs `member_json`) */
export interface LoopMember {
  id: number;
  name: string;
  short: string;
}
/** one career in the solver's affinity detail (`loop.detail.lineages`); uma numbers index the loop's members */
export interface Lineage {
  /** the trainee */
  t: number;
  /** her parents */
  p: [number, number];
  /** her six legacies */
  slots: LegacySlot[];
}
export interface LegacySlot {
  kind: 'parent' | 'grandparent';
  /** [parent] or [parent, grandparent] */
  who: number[];
  /** relation points */
  rel: number;
  /** the G1 wins shared (expected), as points */
  race: number;
  /** affinity */
  aff: number;
}
/** a loop in a result (loop_planner/search/report.rs `loop_json`); member k is `members[k]`, with `outfits[k]` and `per_trainee[k]` */
export interface ResultLoop {
  members: LoopMember[];
  /** the chance every member lands every target */
  p_all: number;
  /** the chance each lands all but at most one */
  p_minus1: number;
  /** what the results are ranked by */
  rank_value: number;
  /** per member: her outfits that cover a target, and what they cover */
  outfits: { outfits: { id: number; label: string }[]; covers: { group: number; level: number }[] }[];
  /** per member: her chance of landing every target */
  per_trainee: number[];
  /** the chance of landing exactly k targets, k = 0 ..= n, averaged over the loop's careers */
  landed?: number[];
  /** the race schedule (`chart_rows`) */
  schedule: ScheduleRow[];
  /** the affinity detail */
  detail: { lineages: Lineage[] };
}
/** one result of the loop search: the loop and its deck */
export interface LoopResult {
  loop: ResultLoop;
  /** the deck most members share (one deck for the loop: that deck) */
  deck: ResultDeck;
  /** per member, when each has her own */
  decks?: ResultDeck[];
}
/** the loop search's report (loop_planner/search/report.rs); a refusal is a SolverRefusal (solver.ts) */
export interface LoopReport {
  /** the best loops, best first */
  top: LoopResult[];
  /** a deck per member (false: one deck for the loop) */
  per_uma: boolean;
  /** decks the search built */
  decks_built: number;
  /** loop structures the job's sizes allow */
  foursomes: number;
  /** the search ended by its stop rule: no unexamined loop can win */
  proven: boolean;
  /** what the answer rests on (`exact_json`): the highest cap certified, the one sought, and whether the search ended by
   *  its stop rule */
  exact: { rho: number; rho_target: number; complete: boolean };
}
/** a report with loops: the only kind the page draws, keeps and exports (run.ts `hasLoops` checks it) */
export type DrawnReport = LoopReport & { top: [LoopResult, ...LoopResult[]] };

/** the button beside the status line: `run` called, else the button whose id is `click` pressed (`click` also names
 *  the button, so a status that offers the same one keeps it, and its focus) */
export interface Act {
  label: string;
  click?: string;
  run?: () => void;
}
/** the status line when no search runs */
export interface Status {
  kind: 'done' | 'example' | 'error' | 'empty' | 'stopped' | 'pending' | 'loading';
  text: string;
  act?: Act;
}
/** the result on show, for Export: the solver's report, and whether it is the precomputed example */
export interface ShownResult {
  kind: 'deck';
  r: DrawnReport;
  meta: { example?: boolean };
}
