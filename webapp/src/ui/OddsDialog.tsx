// ---------- the odds popup (the loop planner's result cards): the chance of landing exactly k of the n targets ----------
// One dialog, there from load (empty); a result card's Odds fills and opens it. Escape, the backdrop and Close all close
// it, and the browser gives the focus back to the button that opened it.
import { computed, signal } from '@preact/signals';
import { pct } from '../odds.ts';
import { Dialog, DialogBody, DialogClose, DialogFoot, DialogTitle } from './Dialog.tsx';
import { RenderGuard } from './RenderGuard.tsx';

/** what the popup shows: `ex[k]` the chance of exactly k (k = 0 .. n); `pAll` the all-n figure shown (the solver's
 *  exact one); `expected` the mean count */
export interface OddsSpec {
  title: string;
  n: number;
  ex: readonly number[];
  pAll: number;
  expected: number;
}
// the odds on show (kept after a close, as the dialog's contents were) and whether the popup is open
const shown = signal<{ spec: OddsSpec | null; open: boolean }>({ spec: null, open: false });
// its guard draws again for new odds only (a close is no reason to try failing content again)
const shownSpec = computed(() => shown.value.spec);
export function openOdds(spec: OddsSpec) {
  shown.value = { spec, open: true };
}
const close = () => {
  shown.value = { ...shown.peek(), open: false };
};
export function OddsDialog() {
  const { spec, open } = shown.value;
  return (
    <Dialog size="md" id="odds-dlg" kind="popup" labelledBy="odds-title" open={open} onClose={close}>
      <RenderGuard input={shownSpec} what="The odds">
        {spec && <Odds spec={spec} />}
      </RenderGuard>
    </Dialog>
  );
}
// the table: a row per count on the game's white cells, the all-n row in the game's gold; each cell kind's whole look
const ROW = {
  all: {
    count: 'bg-gm-gold text-gm-gold-ink border-gm-line rounded-l-md border-y border-l px-2.5 py-1.25 text-left',
    chance: 'bg-gm-gold text-gm-gold-ink border-gm-line rounded-r-md border-y border-r px-2.5 py-1.25 text-right',
  },
  other: {
    count: 'bg-gm-cell border-gm-line rounded-l-md border-y border-l px-2.5 py-1.25 text-left',
    chance: 'bg-gm-cell border-gm-line rounded-r-md border-y border-r px-2.5 py-1.25 text-right',
  },
} as const;
function Odds({ spec: { title, n, ex, pAll, expected } }: { spec: OddsSpec }) {
  // n landed first, down to none
  const counts = Array.from({ length: n + 1 }, (_, i) => n - i);
  return (
    <>
      <DialogTitle id="odds-title">{title}</DialogTitle>
      <DialogBody>
        <table
          class="w-full border-separate border-spacing-x-0 border-spacing-y-1 font-bold tabular-nums"
          aria-labelledby="odds-title"
        >
          <thead>
            <tr>
              <th class="text-note text-gm-ink2 px-2.5 py-1.25 text-left" scope="col">
                Targets landed
              </th>
              <th class="text-note text-gm-ink2 px-2.5 py-1.25 text-right" scope="col">
                Chance
              </th>
            </tr>
          </thead>
          <tbody>
            {counts.map((k) => (
              <tr key={k}>
                <th class={ROW[k === n ? 'all' : 'other'].count} scope="row">{`${k}/${n}`}</th>
                <td class={ROW[k === n ? 'all' : 'other'].chance}>{pct(k === n ? pAll : ex[k] || 0)}</td>
              </tr>
            ))}
          </tbody>
        </table>
        <p class="text-gm-ink2 max-w-[75ch] font-bold">{`${expected.toFixed(2)} of ${n} expected.`}</p>
      </DialogBody>
      <DialogFoot>
        <DialogClose onClick={close} />
      </DialogFoot>
    </>
  );
}
