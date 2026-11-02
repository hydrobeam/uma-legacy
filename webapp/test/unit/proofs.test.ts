// The status line's caveats on a search and a plan (model/proofs.ts `searchCaveat`, `planCaveat`) on constructed
// reports: a cap breach is named whether or not the cap-free rerun finished, no cause is asserted that no field
// establishes, a proof's named assumptions are stated as a proof, and the plan's are said once. npx vitest run (in
// webapp)
import assert from 'node:assert/strict';
import { test } from 'vitest';
import { planCaveat, searchCaveat } from '../../src/model/proofs.ts';

test('the unproven caveat names a cap breach even when its cap-free rerun finished, says when that rerun did not, never claims the work budget without a field for it, and flags a breach on a proven search', () => {
  const rerunDone = searchCaveat({
    proven_top: false,
    complete: false,
    certificates: { cap_exceeded: [[1, 4.0, 1.05]], cap_rerun: { searches: 1, unproven: 0, unanswered: 0 } },
  });
  assert.match(rerunDone, /could not finish \(reported: a deck beat its class's assumed cap\)/);
  assert.doesNotMatch(rerunDone, /work budget|did not finish|band/);
  assert.match(searchCaveat({ proven_top: false, complete: false }), /no cause reported/);
  const rerunOpen = searchCaveat({
    proven_top: false,
    complete: false,
    certificates: { cap_exceeded: [[0, 1.2, 1.05]], cap_rerun: { searches: 1, unproven: 1, unanswered: 0 } },
  });
  assert.match(rerunOpen, /assumed cap; the check without the cap did not finish/);
  const provenBreach = searchCaveat({
    proven_top: true,
    complete: true,
    certificates: { cap_exceeded: [[0, 1.2, 1.05]] },
  });
  assert.match(provenBreach, /beat its class's assumed cap, which the exact check relies on/);
  assert.equal(searchCaveat({ proven_top: true, complete: true }), '');
});

test("the caveat names a breach kept from an earlier search and a work budget run out from the solver's reasons, and states a proof's named assumptions as a proof, apart from unproven causes", () => {
  const breachBefore = searchCaveat({
    proven_top: false,
    complete: false,
    unproven: ['cap_breach'],
    certificates: { job_unproven: ['cap_breach'], cap_breached: [1001] },
  });
  assert.match(
    breachBefore,
    /could not finish \(reported: a deck beat its class's assumed cap in an earlier search for these umas\)/,
  );
  assert.match(
    searchCaveat({ proven_top: false, complete: false, unproven: ['work_cap'] }),
    /reported: a search ran out of its work budget/,
  );
  assert.equal(
    searchCaveat({ proven_top: true, complete: true, assumes: ['two_feeder_cap', 'multi_feeder_cap'] }),
    ' Their order is proven under the assumed class cap for two-feeder decks and the assumed bound for nested-chain decks beside other feeders.',
  );
  const unprovenAssumed = searchCaveat({
    proven_top: false,
    complete: false,
    unproven: ['band_overflow'],
    assumes: ['two_feeder_cap'],
  });
  assert.match(unprovenAssumed, /too large to check in full/);
  assert.doesNotMatch(unprovenAssumed, /proven under/);
  assert.equal(
    searchCaveat({ proven_top: true, complete: true, certificates: { job_assumes: ['multi_feeder_cap'] } }),
    " The job's answer rests on the assumed bound for nested-chain decks beside other feeders.",
  );
  const given = searchCaveat({ proven_top: false, complete: false, given: true });
  assert.match(given, /scored as it is: no search compared it/);
  assert.doesNotMatch(given, /could not finish/);
});

test("the plan's named assumptions are stated on the status line, only those the search's caveat does not state", () => {
  const both = {
    proven_top: true,
    complete: true,
    assumes: ['two_feeder_cap'],
    certificates: { job_assumes: ['multi_feeder_cap'] },
  };
  assert.equal(planCaveat(['two_feeder_cap', 'multi_feeder_cap'], both), '');
  assert.equal(
    planCaveat(['two_feeder_cap', 'multi_feeder_cap'], { proven_top: true, complete: true }),
    ' The plan rests on the assumed class cap for two-feeder decks and the assumed bound for nested-chain decks beside other feeders.',
  );
  assert.equal(
    planCaveat(['two_feeder_cap'], { proven_top: false, complete: false, assumes: ['two_feeder_cap'] }),
    ' The plan rests on the assumed class cap for two-feeder decks.',
  );
  assert.equal(planCaveat(undefined, both), '');
});
