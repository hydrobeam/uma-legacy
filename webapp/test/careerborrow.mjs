// Borrowed parent (Career setup) on the 12 test veterans with the plan's first pick's parent made borrowed: Not this
// career and Never send borrowed_parent ('later', 'never'; Allowed sends none), are saved with the settings, and leave
// every career the planner offers without a borrowed parent; a borrowed parent you chose is blocked, also after a reload
import { check, dialog, careerPage } from './helpers.mjs';

export default async function careerborrow(b) {
  const { ctx, pg, note, done, planned, vetsOpen, vetsShut, node, inNode, importDump } = await careerPage(b);
  await importDump();
  await done();
  await planned();
  const lastPlan = () =>
    pg.evaluate(() => ({
      job: window.__careerJobs.filter((j) => j.cmd === 'career' && j.next && !j.compare).at(-1),
      r: window.__careerResults.filter((r) => r.next).at(-1),
      s: window.__careerResults.filter((r) => !r.next && r.results?.length).at(-1),
    }));
  const withBor = (p) =>
    [p.r.next.choice, ...p.r.next.options, ...p.s.results].filter((r) =>
      r?.parents.some((i) => p.job.legacies[i].borrowed),
    ).length;
  const exported = async () => {
    await vetsOpen();
    await pg.click('#v-export');
    await dialog(pg, 'vx-dlg');
    const x = JSON.parse(await pg.inputValue('#vx-text'));
    await pg.locator('#vx-dlg .dclose').click();
    await vetsShut();
    return x;
  };
  const bi = (await lastPlan()).r.next.choice.parents[0],
    ex = await exported();
  ex.veterans[bi].borrowed = true;
  await vetsOpen();
  await pg.click('#v-paste');
  await dialog(pg, 'paste-dlg');
  await pg.fill('#paste-text', JSON.stringify(ex));
  await pg.click('#paste-load');
  await vetsShut();
  await done();
  await planned();
  const a = await lastPlan();
  const pressed = () =>
    pg.$$eval('#bp button', (bs) =>
      bs
        .filter((b) => b.getAttribute('aria-pressed') === 'true')
        .map((b) => b.dataset.v)
        .join(),
    );
  const saved = () => pg.evaluate(() => JSON.parse(localStorage.getItem('llp:settings')).common.borrowPar);
  check(
    "career: Borrowed parent starts at Allowed, its job without borrowed_parent, and the planner's careers may borrow (the premise)",
    (await pressed()) === 'allow' && !('borrowed_parent' in a.job) && withBor(a) > 0,
    `${await pressed()} ${a.job.borrowed_parent} ${withBor(a)}`,
  );
  // the "later" plan's report edited once (`__planEdit`): every score moved so the top is -0.4, and the estimate's first
  // step (its A side, her own career, no building) put on the borrowed veteran and another
  const borrowedAt = a.job.legacies.findIndex((l) => l.borrowed);
  await pg.evaluate((borrowedAt) => {
    window.__planEdit = (next) => {
      window.__planEdit = null;
      const top = Math.max(...next.options.map((o) => o.score ?? Number.NEGATIVE_INFINITY));
      for (const o of next.options) if (o.score != null) o.score += -0.4 - top;
      const step = {
        ...(next.members?.[0] || {}),
        trainee: 'Stand-in',
        real: false,
        building_careers: 0,
        parents: [borrowedAt, borrowedAt ? 0 : 1],
      };
      next.members = [step, next.members?.[1] || step];
      next.v0 ??= 100;
    };
  }, borrowedAt);
  await pg.click('#bp button[data-v="later"]');
  await done();
  await planned();
  const l = await lastPlan();
  const laterLine = await pg.evaluate(() => document.querySelector('section.alts > .negtop')?.textContent || '');
  check(
    "career: under None left today, where the estimate's next step needs a borrowed parent (its first side's parents in the output), the line above the table says so",
    new RegExp(
      `^No career you can run now saves more than the run itself on today's estimate: the pick is the one that costs least \\(−0\\.4 per run\\)\\. The estimate's next step, Stand-in's run on #${borrowedAt + 1} \\+ #${borrowedAt ? 1 : 2}, needs a borrowed parent \\(#${borrowedAt + 1} [^)]+\\), which None left today keeps out of this career\\.$`,
    ).test(laterLine),
    laterLine,
  );
  check(
    'career: None left today sends borrowed_parent "later", is saved, and no career the planner offers has a borrowed parent',
    l.job.borrowed_parent === 'later' &&
      (await pressed()) === 'later' &&
      (await saved()) === 'later' &&
      withBor(l) === 0 &&
      l.r.next.options.length > 0,
    `${l.job.borrowed_parent} ${await saved()} ${withBor(l)}`,
  );
  await pg.click('#bp button[data-v="never"]');
  await done();
  await planned();
  const n = await lastPlan();
  check(
    'career: Never sends borrowed_parent "never", and no career the planner offers has a borrowed parent',
    n.job.borrowed_parent === 'never' && (await saved()) === 'never' && withBor(n) === 0,
    `${n.job.borrowed_parent} ${withBor(n)}`,
  );
  await pg.click('#bp button[data-v="allow"]');
  await done();
  check(
    'career: back at Allowed the job has no borrowed_parent',
    !('borrowed_parent' in (await exported()).job) && (await saved()) === 'allow',
  );
  // the borrowed veteran chosen as parent 1, then borrowing turned off: blocked, and the picker greys her out (her place
  // in the list as the job has it: the paste may have moved her)
  const bj = a.job.legacies.findIndex((l) => l.borrowed);
  await node('node:p0');
  await inNode(/^(Choose|Change) parent$/).click();
  await dialog(pg, 'vpick-dlg');
  await pg.locator('#vp-list .vpt').nth(bj).click();
  await done();
  await pg.click('#bp button[data-v="never"]');
  await done();
  const blockedNote = /is borrowed, and Borrowed parent is set to Never: choose one of your veterans/;
  const n1 = await note();
  await node('node:p0');
  await inNode(/^(Choose|Change) parent$/).click();
  await dialog(pg, 'vpick-dlg');
  const greyed = await pg
    .locator('#vp-list .vpt')
    .nth(bj)
    .evaluate((b) => b.disabled && /Can't: borrowed/.test(b.textContent));
  await pg.locator('#vpick-dlg .dclose').click();
  check(
    'career: a borrowed parent you chose is blocked when borrowing is off, and the parent picker greys her out',
    blockedNote.test(n1) && greyed,
    n1,
  );
  await pg.reload();
  await done();
  check(
    'career: … after a reload the choice and the block stand',
    (await pressed()) === 'never' && blockedNote.test(await note()),
    await note(),
  );
  check('no page errors', !pg.errors.length, pg.errors.join(' | '));
  await ctx.close();
}
