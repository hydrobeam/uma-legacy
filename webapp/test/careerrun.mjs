// the career planner's run (the 12 test veterans): an edit that leaves the job as it is lets the plan under way go on,
// the status line says how far the plan has got, and after a reload the stored main search is taken (no search again)
import { check, until, dialog, careerPage } from './helpers.mjs';

export default async function careerrun(b) {
  const { ctx, pg, note, done, planned, importDump } = await careerPage(b);
  // the status line's texts, and the workers started (a stopped plan takes its worker with it)
  await pg.evaluate(() => {
    window.__notes = [];
    const n = document.getElementById('run-note');
    new MutationObserver(() => {
      if (window.__notes.at(-1) !== n.textContent) window.__notes.push(n.textContent);
    }).observe(n, { subtree: true, childList: true, characterData: true });
    const W = window.Worker;
    window.__workers = 0;
    // biome-ignore lint/complexity/useArrowFunction: the Worker stand-in is called with `new`, which an arrow function cannot be
    window.Worker = function (...a) {
      window.__workers++;
      return new W(...a);
    };
  });
  // the first plan's report edited once (`__planEdit`): every score moved so the top (the pick's) is -0.4, and another
  // row put 0.1 below the pick on the pick's parents (the table's numbers only: no job reads them back)
  await pg.evaluate(() => {
    window.__planEdit = (next) => {
      window.__planEdit = null;
      const pick = next.options.find((o) => o.chosen),
        other = next.options.find((o) => !o.chosen && o.score != null);
      if (pick?.score == null || !other) return;
      const shift = -0.4 - pick.score;
      for (const o of next.options) if (o.score != null) o.score += shift;
      other.score = pick.score - 0.1;
      other.parents = pick.parents.slice();
      window.__edited = { pick: pick.score, other: other.score, options: next.options.map((o) => o.score) };
    };
  });
  await importDump();
  await pg.waitForFunction(() => window.__careerJobs.some((j) => j.next)); // the plan job is in the worker
  // a preset saved (a settings save that leaves the job as it is), all at once while the plan runs
  const edit = await pg.evaluate(async () => {
    const plan = () => /Planning your next career/.test(document.getElementById('run-note').textContent),
      was = plan();
    const jobs = window.__careerJobs.filter((j) => j.cmd !== 'state_out').length,
      workers = window.__workers; // (a state save, `engineSaveState`, is no job of the plan's)
    // (each dialog is a component: it is drawn a task after the click that opens it)
    const drawn = () => new Promise((r) => setTimeout(r));
    document.getElementById('t-preset').click();
    await drawn();
    document.getElementById('pr-save').click();
    await drawn();
    document.getElementById('prn-save').click();
    await drawn();
    [...document.querySelectorAll('#preset-dlg button')].find((b) => b.textContent === 'Done').click();
    return {
      was,
      now: plan(),
      jobs,
      workers,
      presets: JSON.parse(localStorage.getItem('llp:settings') || '{}').common?.presets?.length,
    };
  });
  await planned();
  // the careers table's notes, from its (i) popup
  const tableNotes = async () => {
    await pg.click('#save-why');
    await dialog(pg, 'save-dlg');
    const t = await pg.$$eval('#save-dlg .optnote', (l) => l.map((x) => x.textContent));
    await pg.keyboard.press('Escape');
    return t;
  };
  const edited = await pg.evaluate(() => {
    const alts = document.querySelector('section.alts'),
      noNotes = !alts?.querySelector('p.optnote');
    return {
      edited: window.__edited,
      noNotes,
      extras: [...(alts?.querySelectorAll('tbody .onum') || [])].map((x) => x.textContent),
      tieTags: alts?.querySelectorAll('.otag.tie, .tienote').length ?? -1,
    };
  });
  edited.negative = (await tableNotes()).find((t) => /saves its own run/.test(t)) || '';
  check(
    "career: where no career you can run saves more than its own run (the top score below 0), the careers table's (i) popup says so in one line (nothing under the table)",
    !!edited.edited && edited.noNotes && edited.negative === 'No career saves its own run today: the pick costs least.',
    JSON.stringify(edited),
  );
  // (the rows within 0.25 of the pick that the table lists are tagged, at most all of them: the edited one among them,
  // on the pick's parents)
  const near = (edited.edited?.options || []).filter((s) => s != null && edited.edited.pick - s <= 0.25).length - 1;
  check(
    'career: a row within 0.25 careers of the pick shows ≈0 extra careers (no tag, no note); the pick shows –',
    edited.extras[0] === '–' &&
      edited.extras.filter((x) => x === '≈0').length >= 1 &&
      edited.extras.filter((x) => x === '≈0').length <= near &&
      edited.tieTags === 0,
    JSON.stringify({ near, edited }),
  );
  const after = await pg.evaluate(() => ({
    jobs: window.__careerJobs.filter((j) => j.cmd === 'career').length,
    posted: window.__careerJobs.filter((j) => j.cmd !== 'state_out').length,
    workers: window.__workers,
    plans: window.__careerResults.filter((r) => r.next).length,
  }));
  check(
    'career: a preset saved during the plan (the job as it was) lets the plan go on: no new job, no new worker, one plan answered',
    edit.was &&
      edit.now &&
      after.posted === edit.jobs &&
      after.workers === edit.workers &&
      after.plans === 1 &&
      edit.presets > 1,
    JSON.stringify({ edit, after }),
  );
  // the plan's robustness check, once the pick is shown: background jobs, one variant each. An edit and its undoing
  // within the pause (the answer on show taken again, `resumeCheck`) leaves the worker as it was, and the check goes on
  // to its end, its report on the plan: every career it names in the table, marked, or none where the pick is stable
  await pg.waitForFunction(() => window.__checkJobs.length > 0, null, { timeout: 180000 });
  // (each check report says a scoring of the check was incomplete: the table must say so once the check is in)
  await pg.evaluate(() => {
    window.__checkPatch = { scoring_complete: false };
  });
  // the keyboard on the table's (i) while the check writes its report into the table (review-8c 5)
  await pg.focus('#save-why');
  const beforeEdit = await pg.evaluate(() => ({ workers: window.__workers, jobs: window.__checkJobs.length }));
  await pg.evaluate(() => {
    document.querySelector('#rank button[data-v="0.20,0.70,0.10"]').click();
    document.querySelector('#rank button[data-v="0.50,0.45,0.05"]').click();
  });
  // (or until a variant finds no kept plan: a new worker took the old one's place, which this check fails)
  await pg.waitForFunction(
    () => window.__checkResults.some((r) => r.next.robust.pending === 0 || r.next.robust.unavailable),
    null,
    { timeout: 180000 },
  );
  await pg.waitForTimeout(700); // (past the pause: the edit's undoing ran its search, the answer on show)
  check(
    "career: the focus stays on the careers table's (i) while the check writes its report",
    await pg.evaluate(() => document.activeElement?.id === 'save-why'),
    await pg.evaluate(() => document.activeElement?.outerHTML.slice(0, 80) ?? 'none'),
  );
  const checked = await pg.evaluate(() => {
    const robust = window.__checkResults.findLast((r) => r.next.robust.pending === 0)?.next.robust || {
      variants: [],
      gone: window.__checkResults.find((r) => r.next.robust.unavailable)?.next.robust.error,
    };
    const alts = document.querySelector('section.alts');
    return {
      robust,
      workers: window.__workers,
      jobs: window.__checkJobs.length,
      rank: document.getElementById('rank').dataset.value, // the edit undone: the rank as it was
      varies: alts ? [...alts.querySelectorAll('.otag')].filter((t) => t.textContent === 'pick varies').length : -1,
    };
  });
  {
    const notes = await tableNotes();
    checked.unavailable = notes.some((t) => /did not finish/.test(t));
    checked.incomplete = notes.filter((t) => /ran out of memory/.test(t)).length;
  }
  await pg.evaluate(() => {
    window.__checkPatch = null;
  });
  check(
    "career: a check whose scoring the memory budget cut short (`robust.scoring_complete` false) says so in the careers table's (i) popup",
    checked.robust.scoring_complete === false && checked.incomplete === 1,
    JSON.stringify({ complete: checked.robust.scoring_complete, notes: checked.incomplete }),
  );
  // the check's variants, in its order (plan/robust.rs): the plan's own entry, the 7 fixed variants (`fixed_variants`:
  // the nominal control, 4 shape corners, 2 ratio ends), then one differential scenario per competitor, the
  // DIFFERENTIAL_COMPETITORS (3) best-scoring other umas among the checked rows, so min(3, the other umas there); a row
  // id starts with its uma's id, and the plan's entry picks the strict argmax whose uma the scenarios scale down
  const variantKinds = checked.robust.variants.map((v) => v.kind);
  const umaOf = (id) => String(id).split(':')[0];
  const pickUma = umaOf(checked.robust.variants[0]?.pick);
  const otherUmas = new Set(checked.robust.rows.map(umaOf).filter((uma) => uma !== pickUma)).size;
  const expectedKinds = [
    'plan',
    'nominal',
    'shape',
    'shape',
    'shape',
    'shape',
    'ratio',
    'ratio',
    ...Array(Math.min(3, otherUmas)).fill('differential'),
  ];
  check(
    "career: the pick's check runs in the background after the plan, goes on through an edit and its undoing (no new worker), and ends with its report on the table",
    JSON.stringify(variantKinds) === JSON.stringify(expectedKinds) &&
      checked.rank === '0.50,0.45,0.05' &&
      checked.workers === beforeEdit.workers &&
      checked.jobs >= 6 &&
      !checked.unavailable &&
      checked.varies === (checked.robust.stable ? 0 : checked.robust.contenders.length),
    JSON.stringify({
      variantKinds,
      expectedKinds,
      beforeEdit,
      checked: { ...checked, robust: { ...checked.robust, variants: undefined } },
    }),
  );
  const prog = await pg.evaluate(() =>
    window.__notes.filter((t) => /Planning your next career… \(.+\)/.test(t)).map((t) => t.match(/\((.+?)\)/)[1]),
  );
  check(
    'career: the plan says how far it has got ("scoring k of n umas"), the counts rising',
    prog.some((t) => /^scoring \d+ of \d+ umas$/.test(t)) &&
      prog
        .filter((t) => /^scoring/.test(t))
        .map((t) => +t.match(/\d+/)[0])
        .every((k, i, a) => !i || k >= a[i - 1]),
    [...new Set(prog)].slice(0, 8).join(' | '),
  );
  // the worker's kept work, stored once the worker is idle after the plan; then a reload: the main search is the stored one
  // (an evaluate awaits the promise; a waitForFunction would take the promise itself as true)
  const stored = () =>
    pg.evaluate(
      () =>
        new Promise((res) => {
          const rq = indexedDB.open('llp-solver');
          rq.onsuccess = () => {
            try {
              const g = rq.result.transaction('state').objectStore('state').getAll();
              // (any format version: the solver's STATE_VERSION, career/state.rs, moves with its fields; the reload below
              // shows the solver took this one in)
              g.onsuccess = () =>
                res(g.result.some((s) => Number.isInteger(s.v) && s.solve?.length > 0 && s.rolls?.length > 0));
              g.onerror = () => res(false);
            } catch {
              res(false);
            }
          };
          rq.onerror = () => res(false);
        }),
    );
  let saved = false;
  for (let i = 0; i < 80; i++) {
    saved = await stored();
    if (saved) break;
    await pg.waitForTimeout(250);
  }
  const before = await note();
  // (the last answer, kept to be drawn at once, forgotten: this reload plans, on the solver's stored work)
  const forgetAnswer = () =>
    pg.evaluate(
      () =>
        new Promise((resolve) => {
          const open = indexedDB.open('llp-solver', 2);
          open.onsuccess = () => {
            const t = open.result.transaction('answer', 'readwrite');
            t.objectStore('answer').clear(); // every era's
            t.oncomplete = () => resolve(true);
            t.onerror = () => resolve(false);
          };
          open.onerror = () => resolve(false);
        }),
    );
  await forgetAnswer();
  await pg.reload();
  await done();
  await planned();
  const re = await pg.evaluate(() => ({
    restored: window.__careerResults.find((r) => r.solve !== undefined),
    search: window.__careerResults.find((r) => r.results),
  }));
  check(
    'career: after a reload the stored main search is taken: the search job searches no deck',
    saved && re.restored?.solve > 0 && re.search?.work?.nodes === 0 && /Planned in/.test(await note()),
    `stored ${saved} | state_in ${JSON.stringify(re.restored)} | nodes ${re.search?.work?.nodes} | before: ${before} | after: ${await note()}`,
  );
  // a reload with nothing changed: the last answer drawn at once, no search or plan job
  const shownBefore = await pg.textContent('#tree .pn.trainee .nm');
  await pg.reload();
  await done();
  check(
    'career: a reload with nothing changed draws the last answer at once: no search or plan runs',
    (await pg.evaluate(() => window.__careerJobs.filter((j) => j.cmd === 'career').length)) === 0 &&
      (await pg.textContent('#tree .pn.trainee .nm')) === shownBefore &&
      /Planned in/.test(await note()),
    `${shownBefore} | ${await note()}`,
  );
  // a roll in a new worker (the stored roll state taken in): answered from it, the plan not run again
  await pg.waitForSelector('#sim-roll:not([disabled])', { state: 'attached', timeout: 180000 }); // (in Try a run's popup, shut)
  await pg.evaluate(() => {
    const r = Math.random;
    Math.random = () => 0.999999999;
    document.getElementById('sim-roll').click();
    Math.random = r;
  });
  await pg.waitForFunction(() => window.__careerResults.some((r) => r.next?.roll), null, { timeout: 180000 });
  const ro = await pg.evaluate(() => window.__careerResults.find((r) => r.next?.roll).next.roll),
    restoredNow = await pg.evaluate(() => window.__careerResults.find((r) => r.solve !== undefined));
  check(
    'career: after a reload a roll is answered from the stored roll state (no plan runs again)',
    restoredNow?.rolls > 0 && typeof ro?.y === 'number' && !ro.planned,
    JSON.stringify({ rolls: restoredNow?.rolls, ro }),
  );
  // a new plan (another rank): a roll asks that plan; back on the first answer (no run), a roll asks the first plan
  // again, which the worker still keeps
  const rollNow = async () => {
    const n = await pg.evaluate(() => window.__careerResults.filter((r) => r.next?.roll).length);
    await pg.waitForSelector('#sim-roll:not([disabled])', { state: 'attached', timeout: 180000 }); // (in Try a run's popup, shut)
    await pg.evaluate(() => {
      const r = Math.random;
      Math.random = () => 0;
      document.getElementById('sim-roll').click();
      Math.random = r;
    });
    await pg.waitForFunction((n) => window.__careerResults.filter((r) => r.next?.roll).length > n, n, {
      timeout: 180000,
    });
    return pg.evaluate(() => {
      const j = window.__careerJobs.filter((j) => j.next?.roll).at(-1),
        p = window.__careerJobs.filter((j) => j.next && !j.next.roll && !j.compare).at(-1);
      const plan = JSON.parse(JSON.stringify(j));
      delete plan.next.roll;
      return {
        same: JSON.stringify(plan) === JSON.stringify(p),
        dist: j.star_dist.join(),
        roll: window.__careerResults.filter((r) => r.next?.roll).at(-1).next.roll,
      };
    });
  };
  const so0 = await pg.evaluate(() => window.__careerJobs.filter((j) => j.cmd === 'state_out').length);
  await pg.click('#rank button[data-v="0.20,0.70,0.10"]');
  await done();
  // the plan's kept work is stored as soon as the worker is idle (a reroll that kills it soon after loses nothing)
  check(
    "career: after a plan the worker's state is stored within 1.3 s (it waited for 1.5 s of idleness)",
    await until(pg, (n) => window.__careerJobs.filter((j) => j.cmd === 'state_out').length > n, so0, 1300),
  );
  const lrNew = await rollNow();
  await pg.click('#rank button[data-v="0.50,0.45,0.05"]');
  await done();
  const lrBack = await rollNow();
  check(
    'career: … a roll after a new plan asks the new plan; back on the first answer it asks the first plan, still kept',
    lrNew.same &&
      lrNew.dist === '0.2,0.7,0.1' &&
      typeof lrNew.roll?.y === 'number' &&
      !lrNew.roll.planned &&
      lrBack.dist === '0.5,0.45,0.05' &&
      typeof lrBack.roll?.y === 'number' &&
      !lrBack.roll.planned,
    JSON.stringify({ lrNew, lrBack }),
  );
  check('no page errors', !pg.errors.length, pg.errors.join(' | '));
  await ctx.close();
}
