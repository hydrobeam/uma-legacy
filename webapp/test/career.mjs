import fs from 'node:fs';
import { drawPink } from '../src/pink.ts';
import { scheduledRaces, rolledBought, starDraw, rolledOutcome, rolledBlueGreen } from '../src/career/roll.ts';
import { searchCaveat, planCaveat } from '../src/proofs.ts';
import { QUICK, check, ART, dialog, audit, contrastCheck, still, careerPage, repoUrl, json } from './helpers.mjs';

// The page's pink draw (`drawPink`) against the solver's pink model (MODEL.md; `Env::pink_gen`): on every reference
// career with legacy pinks (val/ref__global__career_*.out, the solver's outputs that validation pins), 40,000 seeded draws
// per career, each aptitude drawn as often as its `pink_gen` chance, to 5 sigma (at least 1e-3).
// a seeded random source (mulberry32)
function seeded(seed = 0x9e3779b9) {
  let s = seed;
  return () => {
    s = (s + 0x6d2b79f5) | 0;
    let t = Math.imul(s ^ (s >>> 15), 1 | s);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}
// the reference careers' reports (val/ref__global__career_*.out): [file, report]
const referenceCareers = () => {
  const dir = repoUrl('val/');
  return fs
    .readdirSync(dir)
    .filter((f) => /^ref__global__career_.*\.out$/.test(f))
    .sort()
    .flatMap((f) => (JSON.parse(fs.readFileSync(new URL(f, dir), 'utf8')).results || []).map((r) => [f, r]));
};
function pinkPin() {
  const rnd = seeded();
  const N = 40000;
  let aptitudes = 0,
    worst = 0,
    bad = '';
  for (const [f, r] of referenceCareers()) {
    const ap = r.aptitudes;
    if (!ap?.fire.length) continue;
    const drawn = new Map(); // pink group -> draws that gave it
    for (let i = 0; i < N; i++) {
      const g = Object.keys(drawPink(ap, rnd).pinks)[0];
      if (g !== undefined) drawn.set(+g, (drawn.get(+g) || 0) + 1);
    }
    for (const { group, p } of ap.pink_gen) {
      const fr = (drawn.get(group) || 0) / N,
        sd = Math.sqrt((p * (1 - p)) / N);
      aptitudes++;
      if (Math.abs(fr - p) > Math.max(5 * sd, 1e-3)) bad ||= `${f} group ${group}: drawn ${fr} vs ${p}`;
      if (sd > 0) worst = Math.max(worst, Math.abs(fr - p) / sd);
    }
  }
  return { ok: aptitudes > 0 && !bad, detail: bad || `${aptitudes} aptitudes, worst ${worst.toFixed(2)} sigma` };
}
// A roll (career/roll.ts) against the report it is drawn from, on every reference career: over 20,000 seeded rolls each,
// every target lands as often as the report's odds give (its outcome sets' marginal, or its own `p` without them), is
// bought as often as `buy.gold + buy.white` and gold as often as `buy.gold` (the bought draw, conditional on the landing,
// keeps the report's marginals), each G1 is won as often as `scheduledRaces` says, and the stars of the sparks, the
// blue stat and the unique follow the rank band's mix, uniform stats, and the outfit's unique. Each to 5 sigma (at
// least 1e-3). `scheduledRaces` alone: a race run twice is won at least once with 1 - (1 - p1)(1 - p2).
function rollPin() {
  const rnd = seeded(),
    N = 20000,
    rank = '0.20,0.70,0.10',
    mix = rank.split(',').map(Number);
  let figures = 0,
    worst = 0,
    bad = '';
  const near = (what, count, p, n = N) => {
    const fr = count / n,
      sd = Math.sqrt((p * (1 - p)) / n);
    figures++;
    if (Math.abs(fr - p) > Math.max(5 * sd, 1e-3)) bad ||= `${what}: drawn ${fr.toFixed(4)} vs ${p.toFixed(4)}`;
    if (sd > 0) worst = Math.max(worst, Math.abs(fr - p) / sd);
  };
  for (const [f, r] of referenceCareers()) {
    const races = [...new Set((r.schedule || []).map((w) => w.name || w.race))],
      outfit = r.trainee.outfits[0]?.id,
      setup = { career: { races, uniques: { [outfit]: true } }, targets: r.targets, rank };
    const landed = new Map(), // target group -> rolls it landed in
      bought = new Map(),
      gold = new Map(),
      won = new Map(), // race index -> rolls she won it in
      stars = [0, 0, 0],
      blues = [0, 0, 0, 0, 0],
      add = (m, k) => m.set(k, (m.get(k) || 0) + 1);
    let greens = 0;
    for (let i = 0; i < N; i++) {
      const roll = rolledOutcome(r, setup, rnd);
      for (const [g, s] of Object.entries(roll.sparks)) {
        add(landed, +g);
        stars[s - 1]++;
      }
      for (const [g, kind] of Object.entries(roll.bought)) {
        add(bought, +g);
        if (kind === 'gold') add(gold, +g);
      }
      for (const ix of roll.wins) add(won, ix);
      blues[+Object.keys(roll.blues)[0] - 1]++;
      greens += roll.greens[outfit] ? 1 : 0;
    }
    const sets = Array.isArray(r.outcome_sets) ? r.outcome_sets : null;
    r.targets.forEach((t, i) => {
      const p = sets ? sets.reduce((sum, q, mask) => sum + (mask & (1 << i) ? q : 0), 0) : t.p;
      near(`${f} ${r.career} target ${t.group} lands`, landed.get(t.group) || 0, p);
      if (!t.buy) return;
      near(`${f} ${r.career} target ${t.group} bought`, bought.get(t.group) || 0, t.buy.gold + t.buy.white);
      near(`${f} ${r.career} target ${t.group} gold`, gold.get(t.group) || 0, t.buy.gold);
    });
    for (const race of scheduledRaces(r, races))
      near(`${f} ${r.career} ${race.name} won`, won.get(race.ix) || 0, race.p);
    const sparks = stars.reduce((a, b) => a + b, 0);
    mix.forEach((p, k) => {
      near(`${f} ${r.career} ${k + 1}-star sparks`, stars[k], p, sparks);
    });
    blues.forEach((count, k) => {
      near(`${f} ${r.career} blue stat ${k + 1}`, count, 0.2);
    });
    // (a report without an outfit, as before outfits were reported: no unique)
    if (greens !== (outfit ? N : 0)) bad ||= `${f} ${r.career}: her unique in ${greens} of ${N} rolls`;
  }
  // the pieces on their own: a race run at two turns, an unrun one, one not among the races; the star mix; the bought
  // draw of a skill that never lands; no unique for an outfit without one
  const twice = scheduledRaces(
      {
        schedule: [
          { name: 'A', p: 0.5, turn: 1 },
          { name: 'A', p: 0.4, turn: 30 },
          { name: 'B', p: 0.9, turn: 2, run: false },
          { name: 'C', p: 0.9, turn: 3 },
        ],
      },
      ['B', 'A'],
    ),
    star = starDraw('0,1,0', rnd),
    unlanded = rolledBought(
      { targets: [{ group: 7, buy: { land_gold: 0, land_white: 0, gold: 0, white: 0 } }] },
      {},
      rnd,
    ),
    noUnique = rolledBlueGreen({ trainee: { outfits: [{ id: 5 }] } }, { career: { races: [] }, rank }, rnd);
  if (
    !(
      twice.length === 1 &&
      twice[0].ix === 1 &&
      Math.abs(twice[0].p - 0.7) < 1e-12 &&
      twice[0].turns.join() === '1,30' &&
      star() === 2 &&
      Object.keys(unlanded).length === 0 &&
      Object.keys(noUnique.greens).length === 0
    )
  )
    bad ||= `pieces: ${JSON.stringify({ twice, unlanded, noUnique })}`;
  return { ok: figures > 0 && !bad, detail: bad || `${figures} figures, worst ${worst.toFixed(2)} sigma` };
}

export default async function career(b) {
  // the career planner (the front page): the lineage tree, the parent picker, the skills prompt, the settings dialogs,
  // umadump import, the veteran editor, the search and its results
  const {
    ctx,
    pg,
    note,
    done,
    planned,
    oneAnswer,
    showsNow,
    vetsOpen,
    vetsShut,
    vetsSeen,
    node,
    inNode,
    closeNode,
    simStars,
    simOpen,
    simShut,
    importDump,
    saved,
  } = await careerPage(b);
  if (b.browserType().name() === 'chromium') {
    const pin = pinkPin();
    check(
      "career: a roll's pink spark (`drawPink`) is drawn from the solver's pink model: each aptitude as often as its `pink_gen` chance on the reference careers",
      pin.ok,
      pin.detail,
    );
    const rolls = rollPin();
    check(
      "career: a roll (career/roll.ts) keeps its career's report: targets land, are bought and gold, and G1s are won as often as the report says; stars, blue stat and unique as the model draws them",
      rolls.ok,
      rolls.detail,
    );
  }
  // the status line's caveat on constructed reports: a cap breach is named whether or not the cap-free rerun finished,
  // and no cause is asserted that no field establishes
  const breachRerunDone = searchCaveat({
      proven_top: false,
      complete: false,
      cap_exceeded: [[1, 4.0, 1.05]],
      cap_rerun: { searches: 1, unproven: 0, unanswered: 0 },
    }),
    noCause = searchCaveat({ proven_top: false, complete: false }),
    rerunOpen = searchCaveat({
      proven_top: false,
      complete: false,
      cap_exceeded: [[0, 1.2, 1.05]],
      cap_rerun: { searches: 1, unproven: 1, unanswered: 0 },
    }),
    provenBreach = searchCaveat({ proven_top: true, complete: true, cap_exceeded: [[0, 1.2, 1.05]] });
  check(
    "career: the unproven caveat names a cap breach even when its cap-free rerun finished, says when that rerun didn't, never claims the work budget without a field for it, and flags a breach on a proven search",
    /could not finish \(reported: a deck beat its class's assumed cap\)/.test(breachRerunDone) &&
      !/work budget|did not finish|band/.test(breachRerunDone) &&
      /no cause reported/.test(noCause) &&
      /assumed cap; the check without the cap did not finish/.test(rerunOpen) &&
      /beat its class's assumed cap, which the exact check relies on/.test(provenBreach) &&
      searchCaveat({ proven_top: true, complete: true }) === '',
    [breachRerunDone, noCause, rerunOpen, provenBreach].join(' | '),
  );
  // the solver's reason lists: a breach kept from an earlier search of the same deck class, a work budget run out; and
  // the named assumptions a proof rests on, shown apart from any unproven cause
  const breachBefore = searchCaveat({
      proven_top: false,
      complete: false,
      unproven: ['cap_breach'],
      job_unproven: ['cap_breach'],
      cap_breached: [1001],
    }),
    workCap = searchCaveat({ proven_top: false, complete: false, unproven: ['work_cap'] }),
    assumed = searchCaveat({ proven_top: true, complete: true, assumes: ['two_feeder_cap', 'multi_feeder_cap'] }),
    jobAssumed = searchCaveat({ proven_top: true, complete: true, job_assumes: ['multi_feeder_cap'] }),
    given = searchCaveat({ proven_top: false, complete: false, given: true }),
    unprovenAssumed = searchCaveat({
      proven_top: false,
      complete: false,
      unproven: ['band_overflow'],
      assumes: ['two_feeder_cap'],
    });
  check(
    "career: the caveat names a breach kept from an earlier search and a work budget run out from the solver's reasons, and states a proof's named assumptions as a proof, apart from unproven causes",
    /could not finish \(reported: a deck beat its class's assumed cap in an earlier search for these umas\)/.test(
      breachBefore,
    ) &&
      /reported: a search ran out of its work budget/.test(workCap) &&
      /^ Their order is proven under the assumed class cap for two-feeder decks and the assumed bound for nested-chain decks beside other feeders\.$/.test(
        assumed,
      ) &&
      /too large to check in full/.test(unprovenAssumed) &&
      !/proven under/.test(unprovenAssumed) &&
      /^ The job's answer rests on the assumed bound for nested-chain decks beside other feeders\.$/.test(jobAssumed) &&
      /scored as it is: no search compared it/.test(given) &&
      !/could not finish/.test(given),
    [breachBefore, workCap, assumed, unprovenAssumed, jobAssumed, given].join(' | '),
  );
  // the plan's assumptions go on the status line too (the plan table has no note for them), each said once: only those
  // the search's caveat does not state already, all of them when that search proved nothing
  const both = { proven_top: true, complete: true, assumes: ['two_feeder_cap'], job_assumes: ['multi_feeder_cap'] },
    planSame = planCaveat(['two_feeder_cap', 'multi_feeder_cap'], both),
    planMore = planCaveat(['two_feeder_cap', 'multi_feeder_cap'], { proven_top: true, complete: true }),
    planUnproven = planCaveat(['two_feeder_cap'], { proven_top: false, complete: false, assumes: ['two_feeder_cap'] });
  check(
    "career: the plan's named assumptions are stated on the status line, only those the search's caveat does not state",
    planSame === '' &&
      planMore ===
        ' The plan rests on the assumed class cap for two-feeder decks and the assumed bound for nested-chain decks beside other feeders.' &&
      planUnproven === ' The plan rests on the assumed class cap for two-feeder decks.' &&
      planCaveat(undefined, both) === '',
    [planSame, planMore, planUnproven].join(' | '),
  );
  check(
    'career: with no veterans it asks for parents or an import, and offers sample veterans',
    /Choose both parents, or import/.test(await note()) && /Try sample veterans/.test(await pg.textContent('#run-act')),
    await note(),
  );
  check(
    "career: the setup panel shows the shared settings, the Enhance buttons, and Target skills in the open (the planner's editor)",
    (await pg.locator('.rbbar #o-umas.enh .enhimg, .rbbar #o-cards.enh .enhimg').count()) === 2 &&
      (await pg.locator('.rbbar .lptgt #targets .tgt').count()) === 8 &&
      !(await pg.locator('#o-skills, #set-skills').count()),
  );
  // the presets popup inside that panel lays out as on the planner: one column, its tiles the dialog's width, the buttons under them
  await pg.click('#t-preset');
  await pg.locator('#pr-list .pview').first().click();
  const pv = await pg.evaluate(() => {
    const r = (s) => document.querySelector(s).getBoundingClientRect(),
      d = r('#pr-view .dlg'),
      t = r('#prv-tiles'),
      f = r('#pr-view .prfoot');
    return {
      cols: getComputedStyle(document.querySelector('#pr-view .dlg')).gridTemplateColumns.split(' ').length,
      wide: t.width > d.width - 60,
      below: f.top >= t.bottom,
    };
  });
  check(
    'career: the preset popup is laid out as on the planner',
    pv.cols === 1 && pv.wide && pv.below,
    JSON.stringify(pv),
  );
  await pg.locator('#pr-view .dclose').click();
  await pg.locator('#preset-dlg .dclose').click();
  // the pink ribbon toggles it, its edge too (not the bare strip beside it); folded, the summary (the whole bar) opens it
  const hb = await pg.locator('#h-set').boundingBox(),
    bb = await pg.locator('.rbbar').boundingBox();
  await pg.locator('.rbbar').click({ position: { x: hb.x - bb.x + hb.width + 60, y: hb.y - bb.y + hb.height / 2 } });
  const stripStays = !(await pg.locator('.rbbar .setlist').isHidden());
  // (the ribbon measured again: clicking the bar may have scrolled it)
  const hb2 = await pg.locator('#h-set').boundingBox();
  await pg.mouse.click(hb2.x + 6, hb2.y + 4);
  const foldSum = await pg.getAttribute('#set-sum', 'aria-label'),
    foldHid = await pg.locator('.rbbar .setlist').isHidden();
  await pg.click('#set-sum');
  const sumOpen = !(await pg.locator('.rbbar .setlist').isHidden());
  await pg.click('#set-fold');
  const ribFold = await pg.locator('.rbbar .setlist').isHidden();
  await pg.click('#set-fold');
  check(
    'career: the Career setup ribbon (its edge too, not the strip beside it; folded, the summary) folds the panel to a one-line summary and back',
    stripStays &&
      foldHid &&
      sumOpen &&
      ribFold &&
      /^Global · .+ · rank (B–S\+|SS) · 8 target skills$/.test(foldSum) &&
      !(await pg.locator('.rbbar .setlist').isHidden()) &&
      (await pg.getAttribute('#set-fold', 'aria-expanded')) === 'true',
    foldSum,
  );
  // the sample veterans stand in until your own arrive: an import of yours retires them
  await pg.click('#run-act');
  await done();
  const samples = await pg.evaluate(
    () => JSON.parse(localStorage.getItem('llp:career')).global.vets.filter((v) => v.src === 'sample').length,
  );
  await pg.evaluate(() => {
    window.__seq = [];
  });
  await importDump();
  await vetsSeen(); // the veterans dialog draws its cards when open (and on opening)
  check(
    'career: the umadump file imports 12 veterans, and the sample veterans tried before it go',
    /Imported 12 veterans/.test(await pg.textContent('#v-msg')) &&
      (await pg.locator('#vets .umcell').count()) === 12 &&
      samples > 0 &&
      (await pg.evaluate(
        () => !JSON.parse(localStorage.getItem('llp:career')).global.vets.some((v) => v.src === 'sample'),
      )),
    `samples before: ${samples}`,
  );
  // your veterans: the game's Veteran Umamusume button in the setup bar opens them in a dialog
  const vbtn =
    (await pg.getAttribute('#v-toggle', 'aria-label')) === 'Your veterans' &&
    /^Veteran Umamusume$/.test((await pg.textContent('#v-toggle')).trim());
  await pg.click('#v-toggle');
  await dialog(pg, 'set-vets');
  check(
    'career: the Veteran Umamusume button opens your veterans with their import (Export is at the foot of the page)',
    vbtn &&
      (await pg.locator('#set-vets #vets .umcell').count()) === 12 &&
      (await pg.locator('#set-vets #v-paste, #set-vets #v-clear, #set-vets #v-file').count()) === 3 &&
      (await pg.locator('footer #v-export').count()) === 1,
  );
  // (the first plan's report says its scoring was incomplete: the page must say so; later plans are the solver's own)
  await pg.evaluate(() => {
    window.__planPatch = { scoring_complete: false };
  });
  await done();
  const firstRun = await oneAnswer('Any uma/'),
    firstRes = await pg.evaluate(() => window.__careerResults.filter((r) => r.next).at(-1));
  await pg.evaluate(() => {
    window.__planPatch = null;
  });
  // (the table's notes are in its (i) popup)
  await pg.click('#save-why');
  await dialog(pg, 'save-dlg');
  const memNotes = await pg.$$eval('#save-dlg .optnote', (l) =>
    l.map((x) => x.textContent).filter((t) => /ran out of memory/.test(t)),
  );
  await pg.keyboard.press('Escape');
  const incomplete = { count: memNotes.length, text: memNotes[0] };
  check(
    "career: one answer per run: while it searches, then plans, no career is drawn; the status names each phase; the answer is the plan's pick with the veteran drawn once",
    firstRun.ok &&
      firstRun.phases.join() === 'search,plan' &&
      firstRes &&
      (await pg.textContent('#tree .pn.trainee .nm')) === firstRes.next.choice.trainee.name,
    firstRun.detail,
  );
  check(
    "career: a plan whose scoring the memory budget cut short (`scoring_complete` false) says so in the careers table's (i) popup",
    firstRes?.next.scoring_complete === false &&
      incomplete.count === 1 &&
      /ran out of memory.+may be too low/.test(incomplete.text || ''),
    JSON.stringify(incomplete),
  );
  if (QUICK) {
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    return ctx.close();
  } // the fast tier: the one-run answer
  // Export: a dialog with the veterans and the solver job as text; Copy puts it on the clipboard, Download saves it, Paste takes it back
  await pg.evaluate(() => document.getElementById('set-vets').close()); // (Export is on the page, under the dialog)
  await pg.click('#v-export');
  await dialog(pg, 'vx-dlg');
  const exText = await pg.inputValue('#vx-text'),
    ex = JSON.parse(exText);
  const chromium = b.browserType().name() === 'chromium';
  if (chromium) await ctx.grantPermissions(['clipboard-read', 'clipboard-write']);
  await pg.click('#vx-copy');
  await pg.waitForFunction(() => document.getElementById('vx-msg').textContent);
  const copied =
    (await pg.textContent('#vx-msg')) === 'Copied.' &&
    (!chromium || (await pg.evaluate(() => navigator.clipboard.readText())) === exText);
  const [dl] = await Promise.all([pg.waitForEvent('download'), pg.click('#vx-dl')]);
  const dlSame = fs.readFileSync(await dl.path(), 'utf8') === exText;
  await pg.locator('#vx-dlg .dclose').click();
  // umadump's borrow list (rental_chara_data.json: the lender's viewer id as owner_viewer_id, trained_chara_ids that
  // can repeat yours) goes to the Borrowed tab beside your own veterans; a later borrow list replaces it; your own file
  // merges (a partial one removes nothing); Remove all takes only the open tab's
  const dump = JSON.parse(fs.readFileSync(repoUrl('val/umadump/U_trained_chara_data.json'), 'utf8'));
  const lend = (rows, id) => rows.map((r) => ({ ...r, viewer_id: id, owner_viewer_id: id }));
  const paste = async (t) => {
    await vetsOpen();
    await pg.click('#v-paste');
    await dialog(pg, 'paste-dlg');
    await pg.fill('#paste-text', t);
    await pg.click('#paste-load');
  };
  const seen = async () =>
    [
      await pg.textContent('#v-msg'),
      await pg.getAttribute('#v-tab-bor', 'aria-selected'),
      await pg.locator('#vets .umcell').count(),
      (await pg.textContent('#v-tab-own')).trim(),
      (await pg.textContent('#v-tab-bor')).trim(),
    ].join(' | ');
  await paste(JSON.stringify(lend(dump.slice(0, 3), 900000000001)));
  const lent1 = await seen();
  await paste(JSON.stringify(lend(dump.slice(0, 2), 900000000002)));
  const lent2 = await seen();
  await paste(JSON.stringify(dump.slice(0, 5)));
  const part = await seen();
  check(
    'career: a borrow list goes to the Borrowed tab and the next one replaces it; your partial file merges and removes nothing',
    lent1 === 'Replaced the borrow list with 3 borrowed veterans. | true | 3 | Veteran Umamusume · 12 | Guests · 3' &&
      lent2 ===
        'Replaced the borrow list with 2 borrowed veterans (it had 3). | true | 2 | Veteran Umamusume · 12 | Guests · 2' &&
      part ===
        'Imported 5 veterans of yours: 5 already here, updated. | false | 12 | Veteran Umamusume · 12 | Guests · 2',
    `${lent1} / ${lent2} / ${part}`,
  );
  await pg.click('#v-tab-bor');
  pg.once('dialog', (d) => d.accept());
  await pg.click('#v-clear');
  const cleared = await seen(),
    emptyHint = await pg.textContent('#vets');
  await pg.click('#v-tab-own');
  check(
    'career: Remove all on the Borrowed tab removes only the borrowed, and the empty tab says how to fill it',
    /^Removed 2 borrowed veterans\. \| true \| 0 \| Veteran Umamusume · 12 \| Guests · 0$/.test(cleared) &&
      /None yet/.test(emptyHint) &&
      (await pg.locator('#vets .umcell').count()) === 12,
    cleared,
  );
  await vetsShut();
  // an Export of the other era's data is refused: it would replace this era's imports with veterans of other data
  const storedIds = () =>
    pg.evaluate(() =>
      JSON.parse(localStorage.getItem('llp:career'))
        .global.vets.map((v) => v.id)
        .join(),
    );
  const ids0 = await storedIds();
  await paste(JSON.stringify({ ...ex, era: 'gm', veterans: ex.veterans.slice(0, 3) }));
  await saved();
  const refused = {
    msg: await pg.textContent('#v-msg'),
    n: await pg.locator('#vets .umcell').count(),
    same: (await storedIds()) === ids0,
  };
  check(
    "career: an Export of the other era's data is refused, and this era's veterans stay",
    ex.era === 'global' &&
      /^Not imported: that export is of the .+ data\.$/.test(refused.msg) &&
      refused.n === 12 &&
      refused.same,
    JSON.stringify(refused),
  );
  // an Export with a veteran whose records lack fields (cut or edited by hand) draws, the fields taken as empty
  const errs0 = pg.errors.length;
  await paste(JSON.stringify({ ...ex, veterans: [...ex.veterans, { id: 'x', parents: [{}, {}] }] }));
  await pg.click('#vet-12'); // (her card beside the list draws her parents)
  await saved();
  const cut = { n: await pg.locator('#vets .umcell').count(), errors: pg.errors.slice(errs0) };
  check(
    'career: an Export with a veteran missing her sparks, wins and character draws without errors',
    cut.n === 13 && !cut.errors.length,
    JSON.stringify(cut).slice(0, 300),
  );
  await vetsShut();
  await paste(exText); // an Export replaces every import: the 12 again (the cut one goes)
  await saved();
  check(
    'career: Export shows the veterans and the search job as text, Copy copies it, Download saves it, and Paste takes it back',
    ex.app === 'uma-legacy-veterans' &&
      ex.veterans.length === 12 &&
      ex.job &&
      ex.job.cmd === 'career' &&
      ex.job.legacies.length === 12 &&
      copied &&
      dlSame &&
      /Imported 12 veterans from an export/.test(await pg.textContent('#v-msg')) &&
      (await pg.locator('#vets .umcell').count()) === 12 &&
      (await storedIds()) === ex.veterans.map((v) => v.id).join() &&
      !(await pg.locator('#paste-dlg[open]').count()),
    `copy: ${await pg.textContent('#vx-msg')}, download same: ${dlSame}`,
  );
  await done();
  await planned(); // the pasted export's plan (the same pool)
  if (chromium) {
    // the page with a plan on show (the lineage tree, the targets, the options table, the status line), both themes
    await vetsShut();
    for (const theme of ['light', 'dark']) {
      if ((await pg.evaluate(() => document.documentElement.getAttribute('data-theme'))) !== theme)
        await pg.click('#theme-btn');
      await still(pg);
      contrastCheck(`career: ${theme} mode with a plan on show`, [await audit(pg)]);
    }
    await pg.click('#theme-btn');
  }
  check(
    'career: the planner weighs every career, checked on exact odds (only a check that could not finish is mentioned)',
    /Weighed 3,618 careers/.test(await note()) && !/could not finish|checked exact/.test(await note()),
    await note(),
  );
  check(
    "career: the tree shows the planner's trainee and parents",
    (await pg.locator('#tree .pn.ghost').count()) === 3 && (await pg.locator('#tree .pn.gp').count()) === 4,
  );
  // the odds lead the next career; under the targets, the chance of landing exactly each count, all 8 first
  const oddsTxt = await pg.textContent('.rbtitle .lodds #odds-open');
  const oddRows = await pg.$$eval('#tree .rtgt .tgcounts .tgc', (l) =>
    l.map((c) => [c.querySelector('.tgck').textContent, c.querySelector('b').textContent]),
  );
  const avgTxt = await pg.textContent('#tree .rtgt .tgavg b');
  check(
    'career: the odds lead the next career; under the targets, the chance of landing exactly each count, in percent, and the average roll',
    /^1 in [\d,.]+ to land all 8/.test(oddsTxt) &&
      oddRows.length === 9 &&
      oddRows[0][0] === '8/8' &&
      oddRows[1][0] === '7/8' &&
      oddRows[8][0] === '0/8' &&
      oddRows.every((r) => /^[<\d.]+%$/.test(r[1])) &&
      /^Avg \d\.\d$/.test(avgTxt),
    JSON.stringify({ rows: oddRows.slice(0, 3), avgTxt }),
  );
  // the targets, lowest first: the game's skill tile with the chance to land; under it her six legacies' faces and the
  // cards that give it; a tile opens Skill Details (her six legacies, the cards)
  const tgRows = await pg.$$eval('#tree .rtgt .ctg', (l) =>
    l.map((x) => ({
      land: Number.parseFloat(x.querySelector('.udsk .tgp')?.textContent),
      bar: x.querySelectorAll('.tgbar i').length,
      legs: x.querySelector('.tgfig')?.title,
    })),
  );
  await pg.locator('#tree .rtgt .ctg button.udsk').first().click();
  await dialog(pg, 'sk-dlg');
  const skFaces = await pg.locator('#sk-dlg .gmdfaces .av').count(),
    skText = await pg.textContent('#sk-dlg');
  await pg.keyboard.press('Escape');
  check(
    "career: the six-card deck in its own place, shown outright; the targets lowest first, each a tile with its chance to land, its gold / white bar and the legacies' chance under it; a tile opens Skill Details with her six legacies",
    (await (async () => {
      await pg.click('#deck-open');
      await dialog(pg, 'deck-dlg');
      const n = await pg.locator('#deck-dlg #tree-deck .dc').count();
      await pg.keyboard.press('Escape');
      return n;
    })()) === 6 &&
      tgRows.length === 8 &&
      tgRows.every((t, i) => !i || t.land >= tgRows[i - 1].land) &&
      tgRows.every((t) => t.bar === 2 && /^Inherited: [<\d.]+%$/.test(t.legs)) &&
      skFaces === 6 &&
      /Lands as a spark: [<\d.]+%/.test(skText) &&
      /Legacies carrying it/.test(skText),
    JSON.stringify(tgRows.slice(0, 2)),
  );
  // the deck's chance of hinting a target is one figure wherever it shows: its gold, or else its white (the solver's
  // `deck_white` is given no gold), `deckHint`, from the career's own fields: each row's Deck figure, and Skill Details'
  // two deck lines (its gold; its white) adding up to it
  {
    const hint = (b) => 1 - (1 - (b.deck_gold || 0)) * (1 - (b.deck_white || 0));
    const targets = await pg.evaluate(() =>
      [...window.__careerResults.findLast((r) => r.next?.choice).next.choice.targets].sort((a, c) => a.p - c.p),
    );
    const rowDeck = await pg.$$eval('#tree .rtgt .ctg', (l) => l.map((x) => x.querySelector('.tgdk')?.title || ''));
    // (pct's rounding: whole percents from 10%, one decimal from 1%)
    const shows = (text, p) => {
      const v = parseFloat(text.replace(/^Deck: /, '')),
        pc = p * 100;
      return Math.abs(v - pc) <= (pc >= 10 ? 0.5 : pc >= 1 ? 0.05 : 0.005) + 1e-9;
    };
    const rows = targets.map((t, i) => ({ text: rowDeck[i], p: hint(t.buy || {}) })).filter((r) => r.text);
    // Skill Details of the target whose deck gives both its gold and its white most
    const both = targets
      .map((t, i) => ({ i, gw: (t.buy?.deck_gold || 0) * (t.buy?.deck_white || 0), p: hint(t.buy || {}) }))
      .sort((a, c) => c.gw - a.gw)[0];
    await pg.locator('#tree .rtgt .ctg button.udsk').nth(both.i).click();
    await dialog(pg, 'sk-dlg');
    const lines = await pg.$$eval('#sk-dlg .gmdlist li', (l) =>
      l.map((x) => [x.querySelector('span')?.textContent, x.querySelector('b')?.textContent]),
    );
    if (chromium) contrastCheck('career: Skill Details (a game-style popup)', [await audit(pg)]);
    await pg.keyboard.press('Escape');
    const deckLines = lines.filter(([k]) => /^Deck/.test(k)).map(([, v]) => parseFloat(v) || 0);
    check(
      "career: the deck's hint chance (gold, else white) is the same figure in the target rows and in Skill Details, from the result's fields",
      rows.length > 0 &&
        rows.every((r) => shows(r.text, r.p)) &&
        both.gw > 0.01 &&
        deckLines.length === 2 &&
        Math.abs(deckLines[0] + deckLines[1] - both.p * 100) <= 1,
      JSON.stringify({ rows: rows.slice(0, 4), both, lines }),
    );
  }
  check(
    'career: the legacies carry no affinity numbers or labels; the trainee shows her total',
    !(await pg.locator('#tree .gate').count()) &&
      (await pg.locator('#tree .pn:not(.trainee) .aff').count()) === 0 &&
      (await pg.locator('#tree .pn.trainee .aff').count()) === 1,
  );
  const affs = [+(await pg.textContent('#tree .pn.trainee .aff')).replace(/\D/g, '')];
  await node('node:t');
  const affRows = await pg.$$eval('#node-dlg .afftab tbody tr', (l) => l.length),
    affTot = +(await pg.textContent('#node-dlg .afftab tfoot .aff')).replace(/\D/g, '');
  await closeNode();
  check(
    'career: … her popup breaks it down legacy by legacy, summing to the same',
    affRows === 6 && Math.abs(affTot - affs[0]) <= 1,
    `${affRows} rows, ${affTot} vs ${affs[0]}`,
  );
  // the planner's pick: the K best careers by odds, then the plan over them and a few more
  const lastJob = await pg.evaluate(() =>
    window.__careerJobs.filter((j) => j.cmd === 'career' && j.next && !j.compare).at(-1),
  );
  const lastRes = await pg.evaluate(() => window.__careerResults.filter((r) => r.next).at(-1));
  check(
    'career: with places open the planner picks the next career (the highest saving per run) after the six best by odds',
    lastJob &&
      lastJob.top === 6 &&
      lastJob.per_trainee === false &&
      !lastJob.rollout &&
      lastRes &&
      lastRes.next.options.length >= 2 &&
      /^index$/.test(lastRes.next.rule) &&
      /Planned in/.test(await note()),
  );
  check(
    'career: the trainee names one outfit (never "any outfit")',
    !/any outfit/i.test(await pg.textContent('#tree')) && !/any outfit/i.test(await pg.textContent('#results')),
  );
  check(
    'career: … the career on show is its pick, with why under the odds',
    (await pg.textContent('#tree .pn.trainee .nm')) === lastRes.next.choice.trainee.name &&
      (await pg.locator('.alts .otag.pick').count()) === 1,
    await pg.textContent('#tree .pn.trainee .nm'),
  );
  // the deck (its popup): a card's panel, as in the planner; marking it not owned searches again without it
  await pg.click('#deck-open');
  await dialog(pg, 'deck-dlg');
  const dc = pg.locator('#tree-deck .dc').first(),
    dcid = await dc.getAttribute('data-cid');
  await dc.click();
  await pg.waitForSelector('.cpop:not([hidden]) [data-act="own"]');
  const preEdit = await showsNow();
  await pg.evaluate(() => {
    window.__seq = [];
  });
  await pg.locator('.cpop [data-act="own"]').uncheck();
  await done();
  const edit = await oneAnswer(preEdit);
  check(
    'career: an edit keeps the old answer on screen, dimmed, through both phases (no interim career), then draws the new one',
    edit.ok && edit.phases.join() === 'search,plan',
    edit.detail,
  );
  // an edit undone within the pause runs nothing and never dims the answer
  if (!(await pg.locator('.cpop:not([hidden]) [data-act="own"]').count()))
    await pg.locator('#tree-deck .dc').first().click();
  await pg.waitForSelector('.cpop:not([hidden]) [data-act="own"]');
  const pre3 = await showsNow();
  await pg.evaluate(() => {
    window.__seq = [];
  });
  await pg.evaluate(() => {
    const c = document.querySelector('.cpop [data-act="own"]');
    c.click();
    c.click();
  });
  await done();
  const seq3 = await pg.evaluate(() => window.__seq);
  check(
    'career: an edit undone within the pause runs nothing and never dims the answer',
    seq3.length > 0 && !seq3.some((s) => s.stale) && (await showsNow()) === pre3,
    JSON.stringify(seq3.map((s) => [s.busy, s.stale])).slice(0, 120),
  );
  check(
    "career: a deck card's panel marks it not owned; the search drops it or borrows it",
    (await pg.evaluate((id) => JSON.parse(localStorage.getItem('llp:settings')).global.cardsOff[id] === false, dcid)) &&
      (!(await pg.locator(`#tree-deck .dc[data-cid="${dcid}"]`).count()) ||
        (await pg.locator(`#tree-deck .dc.bor[data-cid="${dcid}"]`).count()) === 1),
  );
  {
    // with a card borrowed, every deck on the page (the tree's, Career Info's) lists it last: the sixth slot, as in the game
    const rows = await pg.$$eval('.dcards, .udcards', (l) =>
      l
        .map((r) => [...r.querySelectorAll(':scope > .dc')].map((e) => e.classList.contains('bor')))
        .filter((k) => k.length),
    );
    const bad = rows.filter((k) => k.some((b, i) => b && k.slice(i + 1).some((x) => !x))).length,
      nb = rows.filter((k) => k.includes(true)).length;
    check(
      'career: decks list the borrowed card last',
      nb > 0 && bad === 0,
      `${rows.length} decks, ${nb} with a borrowed card, ${bad} out of order`,
    );
  }
  if (await pg.evaluate(() => document.getElementById('deck-dlg')?.open))
    await pg.locator('#deck-dlg .gmclose').click();
  await pg.click('#o-cards');
  await dialog(pg, 'set-cards');
  await pg.click('#c-all');
  await pg.locator('#set-cards .dclose').click();
  await done();
  // the schedule and the deck: buttons under the trainee, the schedule window (the game's calendar) and the deck's popup
  check(
    'career: the race schedule and the deck are buttons under the trainee, not sections of the page',
    (await pg.locator('#tree .pn.trainee #sched-open').count()) === 1 &&
      (await pg.locator('#tree .pn.trainee #deck-open').count()) === 1 &&
      !(await pg.locator('#tree .lraces, #tree .ldeck').count()),
  );
  await pg.click('#sched-open');
  await dialog(pg, 'sched-dlg');
  const nRun = +(await pg.getAttribute('#sched-open', 'title')).match(/She runs (\d+) G1s/)[1];
  check(
    "career: her schedule opens in the planner's schedule window as the game's Scheduled Races: only the races she runs, the three years at once without scrolling, Junior's April to June grey, no plus (nothing to add there), the parent-won legend",
    nRun > 0 &&
      (await pg.locator('#sched-dlg .ag-slot.on').count()) === nRun &&
      (await pg.locator('#sched-dlg .ag-race').count()) === nRun &&
      !(await pg.locator('#sched-dlg .ag-race.wbtn').count()) &&
      /A parent won it/.test(await pg.textContent('#sched-dlg .agkey')) &&
      (await pg.locator('#sched-dlg .stab').count()) === 1 &&
      (await pg.locator('#sched-dlg .ag-year').count()) === 3 &&
      (await pg.locator('#sched-dlg .ag-slot.pre').count()) === 12 &&
      !(await pg.locator('#sched-dlg .ag-plus, #sched-dlg .ag-add').count()) &&
      (await pg.evaluate(() => {
        const d = document.querySelector('#sched-dlg .dlg');
        return d.scrollHeight <= d.clientHeight + 1;
      })),
    `runs ${nRun}`,
  );
  // Win table: the community table as published, a column per aptitude pair (AA to GG, in the game's letters), a row
  // per races in a row, uncapped as it prints them; on a phone it scrolls inside its panel
  await pg.locator('#sched-dlg button', { hasText: 'Win table' }).click();
  const wt = await pg.$$eval('#sched-wintable tbody tr', (rs) =>
    rs.map((r) => [...r.children].map((c) => c.textContent)),
  );
  const heads = await pg.$$eval('#sched-wintable thead th', (hs) =>
    hs.slice(1).map((h) => h.querySelectorAll('.udg').length),
  );
  const vp = pg.viewportSize();
  await pg.setViewportSize({ width: 375, height: 812 });
  const wtPhone = await pg.evaluate(() => {
    const w = document.querySelector('#sched-wintable .wtscroll'),
      width = (sel) => Math.round(document.querySelector(sel).getBoundingClientRect().width);
    return {
      page: document.documentElement.scrollWidth,
      scrolls: w.scrollWidth > w.clientWidth,
      dialog: width('#sched-dlg'),
      panel: width('#sched-wintable'),
      scroller: w.clientWidth,
      table: width('#sched-wintable table'),
    };
  });
  await pg.setViewportSize(vp);
  check(
    "career: Win table is the community table as published: aptitude pairs AA to GG in the game's letters by 1 to 6 or more races in a row, 1 Race 110% to 0%, scrolling inside its panel at 375 px",
    wt.length === 6 &&
      heads.length === 13 &&
      heads.every((n) => n === 2) &&
      wt[0].join() === '1 Race,110%,100%,90%,80%,70%,50%,20%,10%,0%,0%,0%,0%,0%' &&
      wt[2].slice(0, 5).join() === '3 Races,105%,95%,85%,75%' &&
      wt[5].slice(0, 7).join() === '6 or more Races,60%,50%,40%,30%,20%,0%' &&
      wtPhone.page <= 375 &&
      wtPhone.scrolls,
    `${JSON.stringify(wt[0])} ${JSON.stringify(wtPhone)}`,
  );
  await pg.keyboard.press('Escape');
  // the options: per option this career's odds and its expected careers to land, the pick marked; no after-this-career section
  const opts = await pg.$$eval('.alts .optab tbody tr', (ls) =>
    ls.map((l) => [...l.children].map((c) => c.textContent)),
  );
  check(
    'career: Other careers lists each with "1 in X" and its extra careers against the pick (≈0 within 0.25; – for the pick), the pick marked and shown above',
    opts.length >= 2 &&
      opts.every((c) => /^1 in [\d,.]+$/.test(c[1]) && /^(–|≈0|[+−]\d+\.\d)$/.test(c[2])) &&
      opts.filter((c) => c[2] === '–').length >= 1 &&
      (await pg.textContent('.alts > h3')).startsWith('Other careers') &&
      (await pg.locator('#save-why').count()) === 1 &&
      (await pg.locator('.alts tr.on', { hasText: 'shown above' }).count()) === 1 &&
      (await pg.locator('.alts .otag.pick').count()) === 1 &&
      !(await pg.locator('#ahead-sec').count()),
    opts.map((c) => c.join(' / ')).join(' | '),
  );
  // what each row's run does (its Saves per run's tooltip): one of the route's two parents or its last run as before;
  // else the side her routes lift most often with that chance per run (`lifts_side`), never "not on the route"
  const tableRes = await pg.evaluate(() => window.__careerResults.filter((r) => r.next?.options).at(-1));
  const roles = await pg.evaluate((next) => {
    const titles = [...document.querySelectorAll('.alts .optab tbody .onum[title]')].map((t) => t.title),
      byId = new Map(next.options.map((o) => [o.trainee.id, o.trainee.name])),
      shown = next.options.slice(0, 8).filter((o) => o.role !== 'plan' && o.role !== 'finish' && o.index != null);
    const lifted = shown
      .filter((o) => o.lifts_side?.uma != null && o.lifts_side.chance >= o.lifts_side.own && byId.has(o.lifts_side.uma))
      .map((o) => `Her veteran helps build ${byId.get(o.lifts_side.uma)}'s parent: `);
    return { titles, lifted, fields: next.options.every((o) => o.lifts_side === null || 'own' in o.lifts_side) };
  }, tableRes.next);
  const roleForm =
    /^(Makes one of the two parents your fully looped parent needs|The last run: both her parents are already full veterans|(Already in your loop\. )?(Her veteran helps build .+'s parent: [\d.<%]+ of her runs do|Her veteran can be one of the two parents \(an \d+\/\d+\): [\d.<%]+ of her runs|No run of hers helps the plan|Not compared against the plan))(\. In all, .+ of her runs leave a veteran that helps)?$/;
  check(
    "career: each row's Saves per run says what her run does: the side her routes lift most often and its chance per run (`lifts_side`), or her place on the route",
    roles.fields &&
      roles.titles.length > 0 &&
      roles.titles.every((t) => roleForm.test(t)) &&
      roles.lifted.every((text) => roles.titles.some((t) => t.startsWith(text) || t.includes(`. ${text}`))),
    JSON.stringify(roles),
  );
  // ties and a negative top, as the plan's own scores make them: every row within 0.25 of the pick tagged about equal,
  // the note under the table where any is; the line above it only where the top score is below 0
  const scored = tableRes.next.options.filter((o) => o.score != null),
    pickScore = tableRes.next.options.find((o) => o.chosen)?.score,
    tiedCount = scored.filter((o) => !o.chosen && Math.abs(pickScore - o.score) <= 0.25).length,
    topScore = Math.max(...scored.map((o) => o.score));
  // (each listed row's extra careers as shown: ≈0 for a row within 0.25 of the pick, else at least 0.25 up to the rounding)
  const tables = await pg.evaluate(() => {
    const rows = [...document.querySelectorAll('.alts .optab tbody tr')].filter(
      (tr) => !tr.querySelector('.otag.pick'),
    );
    return {
      extras: rows.map((tr) => tr.querySelector('.onum').textContent),
      ties: document.querySelectorAll('.alts .tienote, .alts .otag.tie').length,
      negative: 0,
    };
  });
  const near = tables.extras.filter((x) => x === '≈0').length;
  check(
    'career: rows within 0.25 careers of the pick show ≈0 (the pick stays the top score; no tag or note), the others their extra careers, and its (i) popup says no career saves its run only where the top score is below 0',
    pickScore === topScore &&
      near <= tiedCount &&
      (tiedCount === 0 || near >= 1) &&
      tables.extras.every((x) => x === '≈0' || x === '–' || Math.abs(Number(x.replace('−', '-'))) >= 0.2) &&
      tables.ties === 0 &&
      (await (async () => {
        await pg.click('#save-why');
        await dialog(pg, 'save-dlg');
        const n = await pg.$$eval(
          '#save-dlg .optnote',
          (l) => l.filter((x) => /saves its own run/.test(x.textContent)).length,
        );
        await pg.keyboard.press('Escape');
        return n;
      })()) === (topScore < 0 ? 1 : 0),
    JSON.stringify({ tiedCount, topScore, pickScore, tables }),
  );
  // the trio: each step's line (her odds now and the lifts building pays for, or your full veteran at no cost)
  await pg.click('#save-why');
  await dialog(pg, 'save-dlg');
  const memTxt = await pg.textContent('#save-dlg .oloop').catch(() => ''),
    aboutLink = await pg.locator('#save-dlg a[href="about#career-planner"]').count();
  await pg.keyboard.press('Escape');
  check('career: What these mean links to the career planner in detail on /about', aboutLink === 1);
  check(
    'career: the plan names its route and, per step, the odds now and what building pays for',
    !lastRes.next.members?.length || /(your \d+\/\d+ of .+ \(no careers\)|1 in [\d,]+)/.test(memTxt),
    memTxt || 'no trio',
  );
  check(
    "career: the odds and Try a run beside Next career; the next career (trainee and legacies, then her targets), the other careers under it, her card in Try a run's popup",
    await pg.evaluate(() => {
      const t = document.getElementById('tree'),
        order = [...t.children].map((k) => k.className.split(' ').find((c) => /^(ltrainee|llegacies|rtgt)$/.test(c)));
      const r = document.getElementById('results'),
        dlg = document.getElementById('sim-dlg');
      return (
        order.join() === 'ltrainee,llegacies,rtgt' &&
        !!document.querySelector('.rbtitle .lodds #odds-open') &&
        !!document.querySelector('.rbtitle .lodds #sim-try') &&
        t.nextElementSibling === r &&
        !r.querySelector('#sim-top, #goal-note, .lhead') &&
        !!dlg.querySelector('#sim-top #simulation #sim-compare') &&
        dlg.querySelector('#simulation > h3')?.textContent === 'Veteran analysis' &&
        !dlg.open
      );
    }),
  );
  // her card, as the game's Umamusume Details screen: portrait and outfit, aptitude grades, the tabs Skills / Inspiration
  // (her sparks beside her portrait, then her parents' under Legacy Origin) / Career Info (the G1 wins button); on a wide
  // screen and a phone, with no sideways scroll
  const simLayout = () =>
    pg.evaluate(() => {
      const c = document.querySelector('#simulation .ud');
      if (!c) return 'no card';
      const f = c.querySelector('.udtop .av'),
        tabs = [...c.querySelectorAll('[role="tab"]')],
        ins = c.querySelector('#udpane-insp'),
        who = ins.querySelector('.simwho'),
        s = who.querySelector('.simskills'),
        me = who.querySelector(':scope > .av'),
        fr = me.getBoundingClientRect(),
        sr = s.getBoundingClientRect();
      const apt = c.querySelector('.udapt');
      return (
        f.getAttribute('role') === 'img' &&
        /\S/.test(f.getAttribute('aria-label') || '') &&
        Math.round(f.getBoundingClientRect().width) === 96 &&
        tabs.map((t) => t.textContent).join('|') === 'Skills|Inspiration|Career Info' &&
        tabs.filter((t) => t.getAttribute('aria-selected') === 'true').length === 1 &&
        (!apt ||
          (apt.querySelectorAll('[role="row"]').length === 3 && apt.querySelectorAll('[role="cell"]').length === 10)) &&
        fr.right <= sr.left &&
        fr.top < sr.bottom &&
        s.getAttribute('aria-label') === 'Her sparks' &&
        ins.querySelectorAll('.udrow.par').length === 2 &&
        c.querySelector('#udpane-info').contains(document.getElementById('sim-wins')) &&
        c.getBoundingClientRect().right <= innerWidth &&
        document.documentElement.scrollWidth <= document.documentElement.clientWidth
      );
    });
  await simOpen(); // her card: Try a run's popup (open through her checks)
  const wide = await simLayout();
  await pg.setViewportSize({ width: 375, height: 812 });
  await pg.waitForTimeout(200);
  const phone = await simLayout();
  await pg.setViewportSize({ width: 1440, height: 1000 });
  await pg.waitForTimeout(200);
  check(
    "career: the veteran analysis is the game's details card: portrait (named for screen readers), aptitude grades, Skills / Inspiration / Career Info tabs, her sparks beside her portrait and her parents' under Legacy Origin, also on a 375 px phone (iPhone 13 mini) with no sideways scroll",
    wide === true && phone === true,
    `wide ${wide}, phone ${phone}`,
  );
  // Skills: her learned targets (a spark of one) and the scenario's golds, as skill tiles; the tabs switch by click and arrow keys
  await pg.click('#udtab-skills');
  // every target she could acquire, as the game's tiles (name only), acquired bright, others greyed; a spark means she
  // acquired it (at least as many acquired tiles as skill sparks); a tile opens the game-style skill popup
  const tiles = await pg.$$eval('#udpane-skills .udsk', (l) =>
    l.map((t) => [t.classList.contains('off'), t.getAttribute('aria-label') || '']),
  );
  const sparkN = await pg.locator('#udpane-insp .simskills .vsk:not(.pink)').count();
  check(
    "career: the Skills tab lists the target skills she could acquire as the game's tiles, acquired ones bright",
    !(await pg.locator('#udpane-skills').isHidden()) &&
      (await pg.locator('#udpane-insp').isHidden()) &&
      tiles.length > 0 &&
      tiles.every((t) => /: (acquired|not acquired)/.test(t[1]) && t[0] === /: not acquired/.test(t[1])) &&
      tiles.filter((t) => !t[0]).length >= sparkN,
    JSON.stringify(tiles) + ' | sparks ' + sparkN,
  );
  await pg.locator('#udpane-skills .udsk').first().click();
  await dialog(pg, 'sk-dlg');
  const pop = await pg.textContent('#sk-dlg');
  await pg.keyboard.press('Escape');
  check(
    'career: … a skill opens its details (acquired or not, where it comes from, how likely), and closing returns to the tile',
    /Skill Details/.test(pop) &&
      /(Acquired|Not acquired)( \(gold\))? in this result/.test(pop) &&
      /Where it comes from/.test(pop) &&
      /Lands as a spark/.test(pop) &&
      (await pg.evaluate(() => document.activeElement?.matches('#udpane-skills .udsk'))),
    pop.slice(0, 160),
  );
  await pg.focus('#udtab-skills');
  await pg.keyboard.press('ArrowRight');
  check(
    'career: … an arrow key moves to the next tab',
    await pg.evaluate(
      () =>
        document.activeElement.id === 'udtab-insp' &&
        document.getElementById('udtab-insp').getAttribute('aria-selected') === 'true',
    ),
  );
  const draft = () =>
    pg.$$eval('#simulation .simskills .vskb', (l) => l.map((b) => b.getAttribute('aria-label')).join('|'));
  const policyJobs = () => pg.evaluate(() => window.__careerJobs.filter((j) => j.compare).length);
  const d0 = await draft();
  check(
    'career: the veteran you got starts as a random result (no default), and nothing is compared yet',
    d0.length > 0 && (await policyJobs()) === 0 && !(await pg.locator('#sim-verdict').count()),
  );
  // her pink spark: her roll carries at most one (an aptitude she ends at A or S)
  const pinkChips = () =>
    pg.$$eval('#simulation .simskills .vsk.pink .vskb', (l) => l.map((b) => b.getAttribute('aria-label')));
  const pk0 = await pinkChips();
  check(
    'career: her roll carries at most one pink spark',
    pk0.length <= 1 && pk0.every((x) => / \(aptitude\), \d stars?/.test(x)),
    pk0.join(' / '),
  );
  const original = await pg.evaluate(() => JSON.parse(localStorage.getItem('llp:career')).global.vets);
  await simStars(3);
  check(
    'career: editing her leaves your veterans untouched and runs nothing',
    (await pg.evaluate(
      (old) => JSON.stringify(JSON.parse(localStorage.getItem('llp:career')).global.vets) === JSON.stringify(old),
      original,
    )) && (await policyJobs()) === 0,
  );
  // an answer for the same career keeps the veteran being entered: an SR card not in its deck marked not owned (a new
  // search, the same career) and owned again (the answer before it, kept)
  {
    const offCard = await pg.evaluate(
      (cards) => {
        const shown = window.__careerResults.findLast((r) => r.next?.choice).next.choice,
          inDeck = new Set(shown.deck.card_ids);
        // (the picker lists the cards that give a target; a tile is known by its card's name, its picture's title)
        const sr = new Set(cards.filter((c) => c.rar === 2 && !inDeck.has(c.id)).map((c) => c.name.toLowerCase()));
        const title = [...document.querySelectorAll('#cards .ct .cav')].find((t) =>
          sr.has(t.title.toLowerCase()),
        ).title;
        return { title, id: cards.find((c) => c.name === title).id };
      },
      await json(pg, 'data-cards'),
    );
    // the card's stored mark: false while it is marked not owned, else none
    const storedMark = () =>
      pg.evaluate((id) => JSON.parse(localStorage.getItem('llp:settings')).global.cardsOff?.[id], offCard.id);
    const toggleCard = async () => {
      await pg.evaluate(
        (name) =>
          [...document.querySelectorAll('#cards .ct .cav')]
            .find((t) => t.title === name)
            .closest('.ct')
            .click(),
        offCard.title,
      );
      await done();
    };
    const entered = await showsNow();
    await toggleCard();
    const afterSearch = await showsNow(),
      markOff = await storedMark();
    await toggleCard();
    const afterUndo = await showsNow(),
      markOn = await storedMark();
    // (each click is seen to write: done() returns at once when no search starts)
    check(
      'career: an answer for the same career (a card not in its deck unowned, then owned again) keeps the veteran being entered',
      /Uma Stan[^|]*3 stars/.test(entered) &&
        markOff === false &&
        markOn === undefined &&
        afterSearch === entered &&
        afterUndo === entered,
      JSON.stringify({
        offCard,
        markOff,
        markOn,
        entered: entered.slice(0, 80),
        afterSearch: afterSearch.slice(0, 80),
        afterUndo: afterUndo.slice(0, 80),
      }),
    );
  }
  await simStars(3);
  await pg.click('#sim-compare');
  await pg.waitForSelector('#sim-verdict', { timeout: 180000 });
  check(
    'career: Compare says, exactly, what she changes for the next career and shows that career with her',
    /^(Saves \d+\.\d careers|No careers saved|Better odds|Parent of the next career|\d+\/\d+: step 1 done|Fully looped parent)/.test(
      await pg.textContent('#sim-verdict'),
    ) &&
      /→/.test(await pg.textContent('#sim-result')) &&
      !/Careers until one lands|Your best career today →/.test(await pg.textContent('#sim-result')),
  );
  // (careers to a fully looped parent: an estimate, whole careers with "about"; a saving at one decimal)
  const loopFigure = await pg.textContent('#sim-result .simodds .tc .tv').catch(() => '');
  check(
    'career: … its careers to loop read "about N → M" (whole careers), a saving one decimal',
    /^(about [\d,]+|\?) → ([\d,]+|\?)$/.test(loopFigure) &&
      !/Saves [\d,]+ careers|rises by [\d,]+ with/.test(await pg.textContent('#sim-verdict')),
    loopFigure,
  );
  check(
    'career: … the best career with her folded under "Best career with her": portraits (trainee, then her two parents, her marked) and its deck; its odds outside',
    (await pg.locator('#sim-result details.nextdeck:not([open]) .nextc .cface').count()) === 3 &&
      (await pg.locator('#sim-result .nextc .cface .her').count()) === 1 &&
      (await pg.locator('#sim-result details.nextdeck:not([open]) .deck .dc').count()) === 6 &&
      (await pg.locator('#sim-result > .simodds .tc').count()) === 2 &&
      /^Best career with her$/.test(await pg.textContent('#sim-result details.nextdeck summary')),
  );
  check(
    'career: the comparison sends her fixed stars, race indices and her parents',
    await pg.evaluate(() => {
      const j = window.__careerJobs.filter((j) => j.compare && !j.next).at(-1),
        v = j.legacies.at(-1);
      return (
        j.compare === true &&
        j.fix.length === 1 &&
        j.fix[0] === j.legacies.length - 1 &&
        v.sparks.some(([g, s]) => g === 20159 && s === 3) &&
        v.wins.every(Number.isInteger) &&
        v.parents.length === 2 &&
        v.parents.every((p) => p.chara > 0 && p.sparks.length > 0)
      );
    }),
  );
  check(
    "career: … and prices the pool with her under today's plan's fit (the plan's `fit`, passed on unchanged), with a line saying Add refits",
    (await pg.evaluate(() => {
      const j = window.__careerJobs.filter((j) => j.compare && j.next).at(-1),
        r = window.__careerResults.filter((r) => r.next?.fit && !r.compare).at(-1);
      return !!j && !!r && JSON.stringify(j.next) === JSON.stringify({ fit: r.next.fit });
    })) && (await pg.locator('#sim-result > .simfit').count()) === 1,
  );
  if (chromium) {
    // one engine is enough for colours
    for (const theme of ['light', 'dark']) {
      // (clicked from script: the popup, open, keeps the page behind it inert)
      if ((await pg.evaluate(() => document.documentElement.getAttribute('data-theme'))) !== theme)
        await pg.evaluate(() => document.getElementById('theme-btn').click());
      await still(pg); // the theme's colour transitions
      contrastCheck(`career: ${theme} mode with the outcome table and a comparison`, [await audit(pg)]);
    }
    await pg.evaluate(() => document.getElementById('theme-btn').click());
  }
  // a roll: a random result of this career, compared at once (stubbed random numbers: nothing lands, then everything at 3 stars)
  const roll = async (u) => {
    const before = await policyJobs();
    await pg.evaluate((u) => {
      const r = Math.random;
      Math.random = () => u;
      document.getElementById('sim-roll').click();
      Math.random = r;
    }, u);
    await pg.waitForFunction(
      (n) =>
        window.__careerJobs.filter((j) => j.compare).length > n &&
        document.querySelector('#sim-verdict') &&
        !document.querySelector('#sim-result .simbest'),
      before,
      { timeout: 180000 },
    );
  };
  await roll(0.999999999);
  const rolledAll = await pg.$$eval('#simulation .simskills .vsk:not(.pink) .vskb', (l) =>
    l.map((b) => b.getAttribute('aria-label')),
  );
  check(
    'career: "Reroll" draws from the career\'s odds and compares at once',
    rolledAll.length === 8 &&
      rolledAll.every((x) => /3 stars/.test(x)) &&
      (await pg.evaluate(
        (old) => JSON.stringify(JSON.parse(localStorage.getItem('llp:career')).global.vets) === JSON.stringify(old),
        original,
      )),
  );
  // the roll's saving comes from the solver in a call of its own, before the search for the best career with her (the
  // plan job on show with `next.roll`, her stars per target), from the plan's kept state: no plan runs again
  const lastRoll = () =>
    pg.evaluate(() => {
      const j = window.__careerJobs.filter((j) => j.next?.roll).at(-1),
        r = window.__careerResults.filter((r) => r.next?.roll).at(-1);
      const ps = window.__careerJobs.filter((j) => j.next && !j.next.roll && !j.compare).map((p) => JSON.stringify(p));
      const plan = j && JSON.parse(JSON.stringify(j));
      if (plan) delete plan.next.roll;
      const all = window.__careerJobs,
        ir = all.lastIndexOf(j),
        ic = all.findLastIndex((x) => x.compare && !x.next);
      return {
        same: !!j && ps.includes(JSON.stringify(plan)),
        stars: j?.next.roll,
        dist: j?.star_dist.join(),
        roll: r?.next.roll,
        first: ir >= 0 && (ic > ir || r.next.roll.y >= 0.5),
        verdict: document.getElementById('sim-verdict')?.textContent || '',
      };
    });
  const lr = await lastRoll();
  check(
    'career: … its saving is worked out by the solver for that one roll, from the plan on show (kept, not planned again), in its own call',
    lr.same &&
      lr.first &&
      lr.stars.length === 8 &&
      lr.stars.every((s) => s === 3) &&
      typeof lr.roll?.y === 'number' &&
      !lr.roll.planned &&
      (lr.roll.y >= 0.5 || /^No (careers saved|saving found yet)/.test(lr.verdict)),
    JSON.stringify(lr),
  );
  // a zero (the solver's answer replaced: y 0; every zero is checked) shows no saving while it is checked,
  // and the exact comparison starts once the roll has stood 0.8 s
  await pg.waitForFunction(() => !document.querySelector('#sim-result .simrefine'), null, { timeout: 180000 }); // (the last roll's check done)
  const refines = () => pg.evaluate(() => window.__careerJobs.filter((j) => j.compare && j.next).length),
    rf0 = await refines();
  // (what shows, and whether the exact comparison has started, read the moment "Checking…" renders: a DOM observer runs
  // before the 0.8 s timer can, where reading it after `roll` returned raced that timer under load)
  await pg.evaluate(() => {
    window.__rollPatch = { y: 0 };
    window.__provSeen = null;
    const o = new MutationObserver(() => {
      const r = document.getElementById('sim-result');
      if (window.__provSeen || !r || !/Checking…/.test(r.textContent)) return;
      window.__provSeen = {
        verdict: document.getElementById('sim-verdict')?.textContent || '',
        n: window.__careerJobs.filter((j) => j.compare && j.next).length,
      };
      o.disconnect();
    });
    o.observe(document.body, { subtree: true, childList: true, characterData: true });
  });
  await roll(0); // (nothing lands: not n/n, so the verdict is the saving's)
  const provAt = await (await pg.waitForFunction(() => window.__provSeen, null, { timeout: 30000 })).jsonValue();
  const prov = { shown: /^No saving found yet/.test(provAt.verdict), early: provAt.n === rf0 };
  prov.started = await pg
    .waitForFunction((n) => window.__careerJobs.filter((j) => j.compare && j.next).length > n, rf0, { timeout: 30000 })
    .then(
      () => true,
      () => false,
    );
  await pg.waitForFunction(() => !document.querySelector('#sim-result .simrefine'), null, { timeout: 180000 });
  await pg.evaluate(() => {
    delete window.__rollPatch;
  });
  check(
    'career: … a provisional zero (y < 0.5) shows "No saving found yet", checking, and runs the exact comparison after a moment',
    prov.shown && prov.early && prov.started,
    JSON.stringify(prov),
  );
  // a roll during an exact comparison: its worker goes, and the next one takes the state stored promptly after the plan,
  // so the roll is answered from the kept roll state, not planned again (planning took most of a second)
  const ins0 = await pg.evaluate(() => window.__careerJobs.filter((j) => j.cmd === 'state_in').length),
    ex0 = await refines();
  // (the second roll is clicked by the page in the task after the comparison's job is posted: one clicked from here,
  // a few round trips later, could find the comparison already done on a loaded machine)
  await pg.evaluate(() => {
    const jobs = window.__careerJobs,
      push = jobs.push,
      roll = () => {
        const r = Math.random;
        Math.random = () => 0;
        document.getElementById('sim-roll').click();
        Math.random = r;
      };
    jobs.push = function (...x) {
      const n = push.apply(this, x);
      if (x.some((j) => j?.compare && j.next)) {
        jobs.push = push;
        setTimeout(() => {
          delete window.__rollPatch;
          window.__nr = window.__careerResults.filter((r) => r.next?.roll).length;
          roll();
        }, 0);
      }
      return n;
    };
    window.__rollPatch = { y: 0.9 };
    roll();
  });
  const exStarted = await pg
    .waitForFunction(
      (n) => window.__careerJobs.filter((j) => j.compare && j.next).length > n && window.__nr !== undefined,
      ex0,
      { timeout: 30000 },
    )
    .then(
      () => true,
      () => false,
    );
  const nr = await pg.evaluate(() => window.__nr);
  await pg.waitForFunction((n) => window.__careerResults.filter((r) => r.next?.roll).length > n, nr, {
    timeout: 180000,
  });
  const after = await pg.evaluate(() => ({
    ins: window.__careerJobs.filter((j) => j.cmd === 'state_in').length,
    roll: window.__careerResults.filter((r) => r.next?.roll).at(-1).next.roll,
  }));
  check(
    'career: … a roll that stops an exact comparison (its worker goes) is answered from the restored state, not planned again',
    exStarted && after.ins > ins0 && typeof after.roll.y === 'number' && !after.roll.planned,
    JSON.stringify({ exStarted, ins0, after }),
  );
  await pg.waitForFunction(
    () =>
      document.querySelector('#sim-verdict') && !document.querySelector('#sim-result .simbest, #sim-result .simrefine'),
    null,
    { timeout: 180000 },
  );
  await roll(0);
  check(
    'career: … a low roll lands nothing',
    (await pg.locator('#simulation .simskills .vsk:not(.pink)').count()) === 0,
  );
  // … and draws her pink: every legacy pink fires at both inspirations (+1 grade each), the first aptitude at A or S, 1 star
  const pkLow = await pinkChips();
  // … her blue spark: one stat, stars by the rank band; its bar opens the picker (the five stats, the stars)
  const blueLbl = await pg
    .getAttribute('#udpane-insp .simskills .vsb.blue:not(.none) .vskb', 'aria-label')
    .catch(() => '');
  await pg.click('#udpane-insp .simskills .vsb.blue .vskb');
  await dialog(pg, 'col-dlg');
  const blueOpts = await pg.$$eval('#col-dlg .colgrid .vskb', (l) => l.map((b) => b.textContent));
  await pg.locator('#col-dlg .spstar').nth(2).click();
  const blue3 = await pg.getAttribute('#udpane-insp .simskills .vsb.blue .vskb', 'aria-label').catch(() => '');
  await pg.keyboard.press('Escape');
  check(
    'career: … a roll draws her blue spark; its bar opens a picker of the five stats and the stars',
    /^(Speed|Stamina|Power|Guts|Wit) \(stat\), [123] stars?/.test(blueLbl) &&
      blueOpts.join() === 'Speed,Stamina,Power,Guts,Wit' &&
      /, 3 stars/.test(blue3),
    `${blueLbl} | ${blueOpts.join()} | ${blue3}`,
  );
  check(
    'career: … a roll draws at most one pink spark for her (stars from 20/70/10: a low draw is 1 star)',
    pkLow.length <= 1 && pkLow.every((x) => /, 1 star\./.test(x)),
    pkLow.join(' / '),
  );
  // rolls while a comparison runs: each cancels the one in flight (its worker goes), and only the last roll's runs and shows
  const policyResults = () => pg.evaluate(() => window.__careerResults.filter((r) => r.compare).length);
  // a comparison is one job for the best career with her (no trainee set) plus, when she doesn't raise the best odds,
  // the planner's candidates (a `next` job when no plan ran) and one job per candidate
  const mainJobs = () =>
    pg.evaluate(() => window.__careerJobs.filter((j) => j.compare && !j.trainees.length && !j.next).length);
  const [rj0, rr0, rm0] = [await policyJobs(), await policyResults(), await mainJobs()];
  const usable = await pg.evaluate(() => {
    const r = Math.random,
      ok = [];
    for (const u of [0.999999999, 0.999999999, 0]) {
      const b = document.getElementById('sim-roll');
      ok.push(!b.disabled);
      Math.random = () => u;
      b.click();
      Math.random = r;
    }
    return ok.every(Boolean) && !!document.querySelector('#sim-result .spin, #sim-result .runner');
  });
  await pg.waitForFunction(
    () => document.querySelector('#sim-verdict') && !document.querySelector('#sim-result .spin, #sim-result .runner'),
    null,
    { timeout: 180000 },
  );
  await pg.waitForTimeout(1500); // time for a stray comparison to answer, were one still running
  const [rj, rr, rm] = [(await policyJobs()) - rj0, (await policyResults()) - rr0, (await mainJobs()) - rm0];
  // an exact comparison in flight is killed (one runs per roll: 3); a quick one (the roll's saving from the plan's kept
  // state) runs out with its answer ignored, so the worker keeps its caches, and only the last roll goes on: its best
  // career with her, then, since every quick zero is provisional, the exact comparison once the roll has stood 0.8 s,
  // which searches the best career with her again (2). Either way only the last roll's shows
  check(
    "career: rolling again while a comparison runs supersedes it: only the last roll's comparison shows",
    usable &&
      (rm === 2 || rm === 3) &&
      rr === rj &&
      (await pg.locator('#simulation .simskills .vsk:not(.pink)').count()) === 0 &&
      (await pg.evaluate(
        () =>
          window.__careerJobs
            .filter((j) => j.compare && !j.next)
            .at(-1)
            .legacies.at(-1).sparks.length === 0,
      )),
    `usable ${usable}, comparisons ${rm}, jobs ${rj}, results ${rr}`,
  );
  check(
    'career: there is no "back to the most likely result" button; Compare, Roll and Add sit in her card\'s header',
    !(await pg.locator('#sim-reset').count()) &&
      (await pg
        .locator(
          '#simulation .ud .udtop #sim-compare, #simulation .ud .udtop #sim-roll, #simulation .ud .udtop #sim-add',
        )
        .count()) === 3,
  );
  // an n/n veteran: Compare says step 1 of the goal is done (no odds verdict)
  for (let i = 0; i < 12; i++) {
    await pg.locator('#simulation .vskadd').click();
    const b = pg.locator('#add-pop .addsk').first();
    if (!(await b.count())) {
      await pg.locator('#simulation .vskadd').click();
      break;
    }
    await b.click();
  }
  await pg.click('#sim-compare');
  await pg.waitForFunction(
    () => document.querySelector('#sim-verdict') && !document.querySelector('#sim-result .spin, #sim-result .runner'),
    null,
    { timeout: 180000 },
  );
  check(
    'career: an n/n veteran: Compare says step 1 of the goal is done (or, with n/n parents, a fully looped parent)',
    /^((\d+)\/\2: step 1 done|Fully looped parent)/.test(await pg.textContent('#sim-verdict')),
    await pg.textContent('#sim-verdict'),
  );
  await roll(0); // (no reset: a roll that lands nothing leaves her short of n/n)
  await simStars(3);
  await pg.click('#sim-compare');
  await pg.waitForSelector('#sim-verdict', { timeout: 180000 });
  const nPolicy = await policyJobs();
  await simStars(1);
  check(
    'career: an edit after Compare marks the comparison out of date without running another',
    /before your last edit/.test(await pg.textContent('#sim-result')) && (await policyJobs()) === nPolicy,
  );
  const baselineRuns = () =>
    pg.evaluate(
      () =>
        window.__careerJobs.filter(
          (j) => j.cmd === 'career' && !j.trainees.length && !j.fix.length && j.top === 1 && !j.next,
        ).length,
    );
  const cachedRuns = await baselineRuns();
  await simShut();
  await node('node:t');
  await inNode(/^(Choose|Change) trainee$/).click();
  await dialog(pg, 'pick-dlg');
  await pg.evaluate(() => {
    window.__seq = [];
  });
  await pg.fill('#pick-filter', 'Agnes Digital');
  await pg.locator('#pick-list .upick:visible').first().click();
  await done();
  await pg.waitForSelector('#sim-compare:not([disabled])', { state: 'attached', timeout: 180000 }); // (in Try a run's popup)
  const fixedRun = await oneAnswer('Agnes Digital/');
  check(
    'career: with the trainee chosen the run is the search alone (no planning phase), its best career the one answer',
    fixedRun.ok && fixedRun.phases.join() === 'search',
    fixedRun.detail,
  );
  // with the trainee chosen there is nothing for the plan to pick: Compare pressed twice restarts the comparison,
  // which finishes, and no plan job is sent
  const aheadJobs = () => pg.evaluate(() => window.__careerJobs.filter((j) => j.next && !j.compare).length),
    la0 = await aheadJobs();
  await pg.evaluate(() => {
    document.getElementById('sim-compare').click();
    document.getElementById('sim-compare').click();
  });
  await pg.waitForFunction(
    () => document.querySelector('#sim-verdict') && !document.querySelector('#run-note .spin, #run-note .runner'),
    null,
    { timeout: 180000 },
  );
  check(
    'career: with the trainee chosen no plan runs, and Compare pressed twice finishes',
    (await aheadJobs()) === la0 &&
      !/Planning|Planned in/.test(await note()) &&
      /^(Saves \d+\.\d careers|No careers saved|Better odds|Parent of the next career|\d+\/\d+: step 1 done|Fully looped parent)/.test(
        await pg.textContent('#sim-verdict'),
      ),
    await note(),
  );
  check(
    "career: with the trainee fixed, Compare reuses today's best instead of searching it again",
    (await baselineRuns()) === cachedRuns,
  );
  // her G1 wins: the G1 wins editor (each win in its slot; a click removes it, an empty slot's menu adds one); an edit
  // after Compare marks it out of date, nothing is saved to the pool, and the focus goes back to the wins button
  const winsN = async () => {
    const m = (await pg.textContent('#sim-wins')).match(/^(No|\d+) G1 win/);
    return m[1] === 'No' ? 0 : +m[1];
  };
  const pj0 = await policyJobs(),
    w0 = await winsN();
  await simOpen();
  await pg.click('#udtab-info');
  // (Try a run's dialog is outside the page's .rb: its deck keeps the six-column row, each card at most 64 px)
  const deckCards = await pg.$$eval('#udpane-info .udcards.deck .udcard', (l) =>
    l.map((c) => {
      const r = c.getBoundingClientRect();
      return [Math.round(r.width), Math.round(r.top)];
    }),
  );
  check(
    "career: Career Info's support cards sit in one row, each at most 64 px wide",
    deckCards.length >= 1 && deckCards.every(([w]) => w <= 64) && new Set(deckCards.map(([, top]) => top)).size === 1,
    JSON.stringify(deckCards),
  );
  const rec = await pg.$$eval('#udpane-info .udrace', (l) => l.map((r) => r.classList.contains('won')));
  const recLine = await pg.textContent('#udpane-info .udrec'),
    racesShut = await pg.locator('#ud-races').isHidden();
  await pg.click('#udpane-info .udmore');
  check(
    'career: the Career Info tab shows her support cards, her two legacies with their parents, and her record (the races behind a button) matching the wins button',
    (await pg.locator('#udpane-info .udcard').count()) >= 1 &&
      (await pg.locator('#udpane-info .udleg').count()) === 2 &&
      (await pg.locator('#udpane-info .udleggx').count()) === 4 &&
      rec.length > 0 &&
      rec.filter(Boolean).length === w0 &&
      new RegExp(`Wins: ${w0}\\b`).test(recLine) &&
      racesShut &&
      !(await pg.locator('#ud-races').isHidden()),
    `${rec.length} races, ${rec.filter(Boolean).length} won, button ${w0}, ${recLine}`,
  );
  // a borrowed card there wears the friend frame within its own card (`all: unset` on these cards once dropped the
  // container the frame's 100cqw is measured on, and the frame grew to the page's width)
  if (ART) {
    const fr = await pg.evaluate(() => {
      const row = document.querySelector('#udpane-info .udcards'),
        c = row.querySelector('.udcard').cloneNode(true);
      c.classList.add('bor');
      c.appendChild(Object.assign(document.createElement('span'), { className: 'udfr', textContent: 'Friends' }));
      row.appendChild(c);
      const r = c.getBoundingClientRect(),
        a = getComputedStyle(c, '::after'),
        out = [
          parseFloat(a.width),
          parseFloat(a.height),
          r.width,
          r.height,
          getComputedStyle(c).boxShadow,
          getComputedStyle(c.querySelector('.udfr')).display,
        ];
      c.remove();
      return out;
    });
    // and wears it alone: the old pink ring and Friends tag (for cards without art) once framed the frame
    check(
      "career: in Career Info a borrowed card's friend frame stays on its card, without the ring and tag",
      fr[0] <= fr[2] + 10 && fr[1] <= fr[3] + 20 && fr[4] === 'none' && fr[5] === 'none',
      `frame ${fr[0]}x${fr[1]}, card ${fr[2]}x${fr[3]}, ring ${fr[4]}, tag ${fr[5]}`,
    );
  }
  await pg.click('#sim-wins');
  await dialog(pg, 'wins-dlg');
  const wb0 = pg.locator('#wins-dlg .ag-race.wbtn').first(),
    wname = (await wb0.getAttribute('aria-label')).replace(/, won: remove$/, '');
  const nShown = await pg.locator('#wins-dlg .ag-race.wbtn').count();
  await wb0.click();
  const freed = await pg.evaluate(
    () => document.activeElement.matches('#wins-dlg .ag-add') && document.activeElement.getAttribute('aria-label'),
  );
  check(
    "career: the entered veteran's G1 wins open on the calendar, one per slot; clicking one removes it, its slot offers to add one, the count follows",
    nShown === w0 &&
      (await pg.locator('#wins-dlg .agenda .ag-year').count()) === 3 &&
      /G1 wins/.test(await pg.textContent('#wins-title')) &&
      /: add a G1 win$/.test(freed || '') &&
      (await winsN()) === w0 - 1 &&
      (await pg.textContent('#wins-count')).startsWith(String(w0 - 1 || 'No')),
    `wins ${w0} -> ${await winsN()}, ${freed}`,
  );
  await pg.keyboard.press('Enter');
  const picks = await pg.$$eval('#wins-dlg .wmenu:not([hidden]) .wpick', (l) => l.map((e) => e.textContent));
  await pg.locator('#wins-dlg .wpick', { hasText: wname }).click();
  check(
    "career: … the empty slot's menu lists the G1s held then (her schedule's first, with its odds); a pick puts the win back in that slot",
    picks[0]?.startsWith(wname) &&
      /her schedule · \d+% win/.test(picks[0]) &&
      (await winsN()) === w0 &&
      (await pg.evaluate((n) => document.activeElement.getAttribute('aria-label') === n + ', won: remove', wname)),
    picks.join(' | '),
  );
  check(
    'career: … which marks the comparison out of date without running another, and saves nothing to your veterans',
    /before your last edit/.test(await pg.textContent('#sim-result')) &&
      (await policyJobs()) === pj0 &&
      (await pg.evaluate(
        (old) => JSON.stringify(JSON.parse(localStorage.getItem('llp:career')).global.vets) === JSON.stringify(old),
        original,
      )),
  );
  await pg.keyboard.press('Escape');
  check(
    'career: … closing it puts the focus back on the wins button',
    await pg
      .waitForFunction(() => document.activeElement?.id === 'sim-wins', null, { timeout: 2000 })
      .then(
        () => true,
        () => false,
      ),
  );
  await pg.click('#udtab-insp');
  check('no page errors', !pg.errors.length, pg.errors.join(' | '));
  await ctx.close();
}
