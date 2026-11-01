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
    <Dialog id="odds-dlg" skin="game" labelledBy="odds-title" open={open} onClose={close}>
      <RenderGuard input={shownSpec} what="The odds">
        {spec && <Odds spec={spec} />}
      </RenderGuard>
    </Dialog>
  );
}
function Odds({ spec: { title, n, ex, pAll, expected } }: { spec: OddsSpec }) {
  // n landed first, down to none
  const counts = Array.from({ length: n + 1 }, (_, i) => n - i);
  return (
    <>
      <DialogTitle id="odds-title">{title}</DialogTitle>
      <DialogBody>
        <table class="oddtab">
          <thead>
            <tr>
              <th scope="col">Targets landed</th>
              <th scope="col">Chance</th>
            </tr>
          </thead>
          <tbody>
            {counts.map((k) => (
              <tr key={k} class={k === n ? 'all' : undefined}>
                <th scope="row">{`${k}/${n}`}</th>
                <td>{pct(k === n ? pAll : ex[k] || 0)}</td>
              </tr>
            ))}
          </tbody>
        </table>
        <p class="gmdstat">{`${expected.toFixed(2)} of ${n} expected.`}</p>
      </DialogBody>
      <DialogFoot>
        <DialogClose onClick={close} />
      </DialogFoot>
    </>
  );
}
