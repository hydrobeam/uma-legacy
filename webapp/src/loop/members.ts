// ---------- a loop result's members, one record each: the solver's parallel lists zipped ----------
// The loop report (loop_planner/search/report.rs `loop_json`) writes a loop's members, their outfit picks, their odds
// as the trainee and (a deck per member) their decks as lists built over the same members, and names members in its
// affinity detail by their place. The page reads them here, each as one record, and its views draw the records; the
// report itself stays as the solver wrote it (it is what the page keeps for a reload and exports).
import { assert, must } from '../must.ts';
import type { DeepReadonly } from '../state/store.ts';
import type * as Report from './types.ts';

type ResultLoop = DeepReadonly<Report.ResultLoop>;
type ResultDeck = DeepReadonly<Report.ResultDeck>;
type LegacySlot = DeepReadonly<Report.LegacySlot>;

/** A member of a loop result. */
export interface Member {
  /** her place among the members: the index the report's lineages and schedule cells use */
  k: number;
  id: number;
  name: string;
  short: string;
  /** her outfits that cover a target (empty: none does) */
  outfits: DeepReadonly<{ id: number; label: string }[]>;
  /** the targets her outfit covers (level 1 its white, 2 its gold) */
  covers: DeepReadonly<{ group: number; level: number }[]>;
  /** her chance of landing every target as the trainee */
  perTrainee: number;
  /** her own deck, where the result has one per member (null: the loop's one deck) */
  deck: ResultDeck | null;
}
// entry k of a list the solver writes one entry per member into (its length checked with the members' first)
const ofMember = <T>(list: readonly T[], k: number): T => must(list[k], 'the solver writes an entry per member');

/** the loop's members, each with her outfit pick, odds and (`decks`: a deck per member) deck */
export function loopMembers(loop: ResultLoop, decks: readonly ResultDeck[] | null = null): Member[] {
  const n = loop.members.length;
  assert(
    loop.outfits.length === n && loop.per_trainee.length === n && (!decks || decks.length === n),
    'report.rs writes one outfit pick, one chance and one deck per member',
  );
  return loop.members.map((m, k) => ({
    k,
    id: m.id,
    name: m.name,
    short: m.short,
    outfits: ofMember(loop.outfits, k).outfits,
    covers: ofMember(loop.outfits, k).covers,
    perTrainee: ofMember(loop.per_trainee, k),
    deck: decks ? ofMember(decks, k) : null,
  }));
}

/** A legacy of a career: the uma, her relation points with the trainee, the G1 wins they share (expected, as points)
 *  and the affinity. */
export interface Legacy {
  member: Member;
  rel: number;
  race: number;
  aff: number;
}
/** A career of the loop's affinity detail (`detail.lineages`), its umas the members. */
export interface Career {
  trainee: Member;
  parents: readonly [Member, Member];
  /** each parent's legacy, with her two parents' */
  legacies: readonly (Legacy & { grandparents: readonly Legacy[] })[];
  /** her total affinity */
  affinity: number;
}
/** the loop's careers (the solver's lineages), each uma the member the report names by her place */
export function loopCareers(loop: ResultLoop, members: readonly Member[]): Career[] {
  const member = (k: number | undefined): Member =>
    must(k === undefined ? undefined : members[k], "a lineage names the loop's members by their place");
  const legacy = (who: number | undefined, slot: LegacySlot): Legacy => ({
    member: member(who),
    rel: slot.rel,
    race: slot.race,
    aff: slot.aff,
  });
  return loop.detail.lineages.map(({ t, p, slots }) => ({
    trainee: member(t),
    parents: [member(p[0]), member(p[1])],
    legacies: p.map((parent) => ({
      ...legacy(
        parent,
        must(
          slots.find((s) => s.kind === 'parent' && s.who[0] === parent),
          'a parent has her legacy',
        ),
      ),
      grandparents: slots
        .filter((s) => s.kind === 'grandparent' && s.who[0] === parent)
        .map((g) => legacy(g.who[1], g)),
    })),
    affinity: slots.reduce((sum, slot) => sum + slot.aff, 0),
  }));
}
