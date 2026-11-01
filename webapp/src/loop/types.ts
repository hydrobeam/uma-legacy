// ---------- the loop page's types: the solver's loop report as the page reads it, and the status line's record ----------
// Types only (no values), shared by the page's model, store, run machine and views. The solver's report is generated
// from its Rust structs (src/gen/solver.ts: loop_planner/output.rs and what it holds); re-exported here under the names
// the loop page reads.
import type { LoopReport, LoopResult } from '../gen/solver.ts';

export type {
  LegacySlot,
  Lineage,
  LoopMember,
  LoopReport,
  LoopResult,
  ResultDeck,
  ResultLoop,
} from '../gen/solver.ts';

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
