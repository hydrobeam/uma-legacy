// ---------- affinity popup (loop planner): a career's affinity, legacy by legacy, as on the career page ----------
// The loop's careers as the game's pill switches: one per trainee, the most affinity first; in a four-uma loop each
// trainee heads three careers, so a second row picks her parents (both faces and that career's affinity). Under them the
// chosen career's table (ui/AffTable.tsx): each parent, then her two parents, with their relation points, the G1 wins
// they share (expected: from the schedules' win chances, x3) and the affinity, and the total. In a three-uma rotation
// the trainee is her own grandparent twice (her earlier veteran): no relation points with her (a trio counts distinct
// umas), only the G1s it shares with its child.
import { signal, untracked } from '@preact/signals';
import { useState } from 'preact/hooks';
import { pickOutfit } from '../roster.js';
import { must } from '../must.ts';
import { Dialog } from '../ui/Dialog.tsx';
import { RadioPills } from '../ui/RadioPills.tsx';
import { AffHeart, AffTable } from '../ui/AffTable.tsx';
import type { AffinityLeg } from '../ui/AffTable.tsx';
import { UmaAv } from '../ui/Images.tsx';
import type { Lineage, ResultLoop } from './types.ts';

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
    <Dialog id="why-dlg" labelledBy="why-title" open={open} onClose={close}>
      <div class="dlg" id="why-body">
        {loop && <Careers key={n} loop={loop} />}
      </div>
    </Dialog>
  );
}
const careerTotal = (lineage: Lineage) => lineage.slots.reduce((sum, slot) => sum + slot.aff, 0);
function Careers({ loop }: { loop: ResultLoop }) {
  const lineages = loop.detail.lineages,
    members = loop.members;
  // trainee -> her careers (lineage indices), the most affinity first; the career shown first is the best of all
  const byAffinity = lineages.map((_, i) => i).sort((a, b) => careerTotal(lineages[b]) - careerTotal(lineages[a]));
  const careersOf = new Map<number, number[]>();
  for (const i of byAffinity) careersOf.set(lineages[i].t, [...(careersOf.get(lineages[i].t) || []), i]);
  const careersFor = (trainee: number) => must(careersOf.get(trainee), 'each trainee heads a career');
  const [at, setAt] = useState(byAffinity[0]);
  const lineage = lineages[at],
    careers = careersFor(lineage.t);
  // her outfit's portrait, as the result names it (the outfit as the settings are now, as her result card says)
  const face = (i: number, size: number) => {
    const outfit = untracked(() => pickOutfit(members[i].id, loop.outfits[i].outfits).list[0]?.id ?? null);
    return <UmaAv key={`${members[i].id}:${outfit}`} uma={members[i].id} outfit={outfit} size={size} />;
  };
  const shortName = (i: number) => members[i].short || members[i].name;
  // a career's table rows: each parent, then her two parents, with their relation points, shared G1s and affinity
  const legacies = (career: Lineage): AffinityLeg[] =>
    career.p.map((parent) => {
      const slot = must(
        career.slots.find((s) => s.kind === 'parent' && s.who[0] === parent),
        'a parent has her legacy',
      );
      return {
        face: face(parent, 30),
        name: members[parent].name,
        rel: slot.rel,
        race: slot.race,
        aff: slot.aff,
        gps: career.slots
          .filter((s) => s.kind === 'grandparent' && s.who[0] === parent)
          .map((g) => ({
            face: face(g.who[1], 24),
            name: members[g.who[1]].name,
            tag: g.who[1] === career.t ? 'her own earlier veteran' : null,
            rel: g.rel,
            race: g.race,
            aff: g.aff,
          })),
      };
    });
  return (
    <>
      <div class="row between dhead">
        <h3 id="why-title">Affinity</h3>
        <button type="button" class="btn" onClick={close}>
          Close
        </button>
      </div>
      <RadioPills
        class="afpick"
        label="Trainee"
        value={lineage.t}
        onPick={(trainee) => setAt(careersFor(trainee)[0])}
        pills={[...careersOf].map(([trainee, its]) => {
          const [p0, p1] = lineages[its[0]].p;
          const name =
            its.length > 1
              ? `${members[trainee].name} as the trainee`
              : `${members[trainee].name}, parents ${members[p0].name} and ${members[p1].name}`;
          return {
            v: trainee,
            label: name,
            title: name,
            body: (
              <>
                {face(trainee, 28)}
                <span class="afpn">{shortName(trainee)}</span>
                <AffHeart affinity={careerTotal(lineages[its[0]])} />
              </>
            ),
          };
        })}
      />
      <RadioPills
        class="afpick afpar"
        label="Parents"
        hidden={careers.length < 2}
        lead={careers.length > 1 && <span class="aflab">Parents</span>}
        value={at}
        onPick={setAt}
        pills={
          careers.length < 2
            ? []
            : careers.map((j) => {
                const [p0, p1] = lineages[j].p,
                  name = `parents ${members[p0].name} and ${members[p1].name}`;
                return {
                  v: j,
                  label: name,
                  title: `${members[lineages[j].t].name}: ${name}`,
                  body: (
                    <>
                      {face(p0, 24)}
                      {face(p1, 24)}
                      <AffHeart affinity={careerTotal(lineages[j])} />
                    </>
                  ),
                };
              })
        }
      />
      <div class="afview">
        <AffTable legs={legacies(lineage)} caption={`${members[lineage.t].name}: her six legacies`} />
      </div>
    </>
  );
}
