// Settings fixtures for the loop page's jobs, as stored (saved.ts's object, Global era), over the fake page's game
// (fakepage.mjs). GOLDEN (loopgolden.json) holds the job each made, or the status that blocked it, recorded from the
// page's code before src/loop/job.ts (9307950): a job's JSON, its field order included, is the key of the kept result
// and of the suite's recorded solver replies, so it must not change.
const stored = (common, owned = {}) => ({ v: 2, era: 'global', common, global: owned });
export const FIXTURES = {
  // nothing saved: the defaults (the example's job)
  defaults: stored({}),
  // some umas, outfits and cards not owned (Uma E has no outfit owned, so she is out), another scenario and rank,
  // three decks, one deck per loop, three targets
  partlyOwned: stored(
    {
      targets: [{ group: 20049 }, { group: 20159 }, { group: 20166 }],
      rank: 'ss_and_up',
      scen: 'gc',
      decksTop: '3',
      perUma: false,
    },
    { roster: { 1002: 'out' }, outfits: { 100101: false, 100501: false, 100502: false }, cardsOff: { 30002: false } },
  ),
  // no card owned (the solver reads an empty list as all, so it gets [0]); Unity Cup's two golds among the targets
  noCards: stored(
    { targets: [{ group: 20049 }, { group: 20046 }] },
    { cardsOff: { 30001: false, 30002: false, 30003: false, 30004: false, 30005: false } },
  ),
  // the Grand Masters scenario (no guaranteed gold), every card owned
  grandMasters: stored({ targets: [{ group: 20049 }], scen: 'grandmasters', rank: 'ss_and_up' }),
  // blocked: no target
  noTargets: stored({ targets: [] }),
  // blocked: two umas owned
  twoUmas: stored({}, { roster: { 1001: 'out', 1002: 'out', 1003: 'out' } }),
};
