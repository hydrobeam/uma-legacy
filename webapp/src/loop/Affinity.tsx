// ---------- affinity popup (loop planner): a career's affinity, legacy by legacy, as on the career page ----------
// The loop's careers as the game's pill switches: one per trainee, the most affinity first; in a four-uma loop each
// trainee heads three careers, so a second row picks her parents (both faces and that career's affinity). Under them the
// chosen career's table (ui/AffTable.tsx): each parent, then her two parents, with their relation points, the G1 wins
// they share (expected: from the schedules' win chances, x3) and the affinity, and the total. In a three-uma rotation
// the trainee is her own grandparent twice (her earlier veteran): no relation points with her (a trio counts distinct
// umas), only the G1s it shares with its child.
import { signal, untracked } from '@preact/signals';
import { useState } from 'preact/hooks';
import { pickOutfit } from '../roster.ts';
import { must } from '../must.ts';
import { Dialog, DialogHead, DialogPanel } from '../ui/Dialog.tsx';
import { SegmentedControl } from '../ui/SegmentedControl.tsx';
import { AffHeart, AffTable } from '../ui/AffTable.tsx';
import type { AffinityLeg } from '../ui/AffTable.tsx';
import { UmaAv } from '../ui/Images.tsx';
import { RenderGuard } from '../ui/RenderGuard.tsx';
import type { DeepReadonly } from '../store.ts';
import type * as Report from './types.ts';
import { loopCareers, loopMembers } from './members.ts';
import type { Career, Member } from './members.ts';

/** a result's loop, as the store hands it (read-only) */
type ResultLoop = DeepReadonly<Report.ResultLoop>;

// the loop shown at the last opening (`n` counts them: each opening starts at the best career), and whether it is open
const shown = signal<{ n: number; loop: ResultLoop | null; open: boolean }>({ n: 0, loop: null, open: false });
export function openWhy(loop: ResultLoop) {
  shown.value = { n: shown.peek().n + 1, loop, open: true };
}
const close = () => {
  shown.value = { ...shown.peek(), open: false };
};
export function AffinityDialog() {
  const { n, loop, open } = shown.value;
  return (
    <Dialog size="lg" id="why-dlg" labelledBy="why-title" open={open} onClose={close}>
      <DialogPanel id="why-body">
        <RenderGuard input={shown} what="The affinity">
          {loop && <Careers key={n} loop={loop} />}
        </RenderGuard>
      </DialogPanel>
    </Dialog>
  );
}
// the loop's careers, the most affinity first, each with its place in that order (`n`: the pills' value)
type Ranked = Career & { n: number };
function Careers({ loop }: { loop: ResultLoop }) {
  const members = loopMembers(loop),
    ranked: Ranked[] = loopCareers(loop, members)
      .sort((a, b) => b.affinity - a.affinity)
      .map((career, n) => ({ ...career, n }));
  // trainee (her place k) -> her careers, the most affinity first; the career shown first is the best of all
  const careersOf = new Map<number, [Ranked, ...Ranked[]]>();
  for (const career of ranked) {
    const hers = careersOf.get(career.trainee.k);
    if (hers) hers.push(career);
    else careersOf.set(career.trainee.k, [career]);
  }
  const careersFor = (trainee: number) => must(careersOf.get(trainee), 'each trainee heads a career');
  const careerAt = (n: number) => must(ranked[n], 'a pill names one of the careers drawn');
  const [at, setAt] = useState(0);
  const career = careerAt(at),
    careers = careersFor(career.trainee.k);
  // her outfit's portrait, picked as her result card picks it, from the settings as they are when the dialog opens (the
  // card's is from when the result was drawn: an outfit owned since can differ)
  const face = (m: Member, size: number) => {
    const outfit = untracked(() => pickOutfit(m.id, m.outfits).list[0]?.id ?? null);
    return <UmaAv key={`${m.id}:${outfit}`} uma={m.id} outfit={outfit} size={size} />;
  };
  // a career's table rows: each parent, then her two parents, with their relation points, shared G1s and affinity
  const legacies = (c: Career): AffinityLeg[] =>
    c.legacies.map((leg) => ({
      face: face(leg.member, 30),
      name: leg.member.name,
      rel: leg.rel,
      race: leg.race,
      aff: leg.aff,
      gps: leg.grandparents.map((g) => ({
        face: face(g.member, 24),
        name: g.member.name,
        tag: g.member.k === c.trainee.k ? 'her own earlier veteran' : null,
        rel: g.rel,
        race: g.race,
        aff: g.aff,
      })),
    }));
  return (
    <>
      <DialogHead id="why-title" title="Affinity" close="Close" onClose={close} />
      <SegmentedControl
        kind="radio"
        shape="apart"
        class="mt-1 mb-3"
        label="Trainee"
        value={career.trainee.k}
        onPick={(trainee) => setAt(careersFor(trainee)[0].n)}
        options={[...careersOf.values()].map(([best, ...others]) => {
          const [p0, p1] = best.parents,
            her = best.trainee;
          const name = others.length ? `${her.name} as the trainee` : `${her.name}, parents ${p0.name} and ${p1.name}`;
          return {
            v: her.k,
            label: name,
            title: name,
            body: (
              <>
                {face(her, 28)}
                <span>{her.short || her.name}</span>
                <AffHeart affinity={best.affinity} />
              </>
            ),
          };
        })}
      />
      <SegmentedControl
        kind="radio"
        shape="apart"
        class="-mt-1.5 mb-3"
        label="Parents"
        hidden={careers.length < 2}
        lead={careers.length > 1 && <span class="text-note text-gm-ink2 mr-0.5 font-bold">Parents</span>}
        value={at}
        onPick={setAt}
        options={
          careers.length < 2
            ? []
            : careers.map((c) => {
                const [p0, p1] = c.parents,
                  name = `parents ${p0.name} and ${p1.name}`;
                return {
                  v: c.n,
                  label: name,
                  title: `${c.trainee.name}: ${name}`,
                  body: (
                    <>
                      {face(p0, 24)}
                      {face(p1, 24)}
                      <AffHeart affinity={c.affinity} />
                    </>
                  ),
                };
              })
        }
      />
      <AffTable legs={legacies(career)} caption={`${career.trainee.name}: her six legacies`} />
    </>
  );
}
