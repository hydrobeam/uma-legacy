// the career planner on pasted pools: the goal's steps (full veterans of two umas), a borrowed or owned fully looped
// veteran, the trio path (from the 12 test veterans' export, changed)
import { expect } from 'playwright/test';
import { check, dialog, careerPage } from './helpers.mjs';

export default async function careerpool(b) {
  const { ctx, pg, done, vetsOpen, vetsShut, importDump } = await careerPage(b);
  await importDump();
  await done();
  await vetsShut(); // (Export is on the page, under the dialog)
  await pg.click('#v-export');
  await dialog(pg, 'vx-dlg');
  const exText = await pg.inputValue('#vx-text');
  await pg.locator('#vx-dlg').getByRole('button', { name: 'Close', exact: true }).click();
  // the goal's steps: full veterans of two different umas -> step 3 in the note; the planner still picks (nothing fixed),
  // pricing them as real full-veteran sides of its trio
  const goalEx = JSON.parse(exText),
    all8 = Object.fromEntries(goalEx.job.skills.targets.map((t) => [t.group, 2]));
  for (const v of goalEx.veterans) for (const g of v.parents) g.sparks = {}; // (the test file's parents carry every spark: they would make a looped parent at once)
  const two = [0, goalEx.veterans.findIndex((v) => v.chara !== goalEx.veterans[0].chara)];
  for (const k of two) goalEx.veterans[k].sparks = { ...all8 };
  await vetsOpen();
  await pg.click('#v-paste');
  await dialog(pg, 'paste-dlg');
  await pg.fill('#paste-text', JSON.stringify(goalEx));
  await pg.click('#paste-load');
  await vetsShut();
  await done();
  await pg.waitForFunction(() => /Planned in/.test(document.getElementById('run-note').textContent), null, {
    timeout: 300000,
  });
  const goalJob = await pg.evaluate(() =>
    window.__careerJobs.filter((j) => j.cmd === 'career' && !j.career.next && !j.career.compare).at(-1),
  );
  const goalRes = await pg.evaluate(() => window.__careerResults.filter((r) => r.next).at(-1));
  const goalCheck =
    'career: with full veterans of two umas nothing is fixed (no goal steps), and the planner prices them in its trio and picks';
  await expect.soft(pg.locator('#goal-note'), `${goalCheck}: no goal steps`).toHaveCount(0);
  await expect.soft(pg.locator('[data-sec="options"] [data-tag="pick"]'), `${goalCheck}: one pick`).toHaveCount(1);
  check(
    goalCheck,
    goalJob &&
      goalJob.fix.length === 0 &&
      goalRes &&
      goalRes.next.mode === 'trio' &&
      goalRes.next.members.length === 2 &&
      goalRes.next.members.every((m) => typeof m.real === 'boolean'),
    `${JSON.stringify(goalJob?.fix)} | ${goalRes && JSON.stringify(goalRes.next.v0_plan)} | pick ${goalRes && JSON.stringify(goalRes.next.choice && [goalRes.next.choice.trainee.name, goalRes.next.choice.parents])}`,
  );
  // each uma also gets her career on two of those full veterans (only there is her own full veteran a fully looped parent), the trio's last
  // run is one, and the table tags them
  const nTags = await pg.locator('[data-sec="options"] tbody [data-tag]', { hasText: /^\d+\/\d+ parents$/ }).count();
  check(
    "career: with full veterans the plan offers careers on two of them, the route's last run among them, tagged in the table",
    goalRes?.next.options.some((o) => o.looped) &&
      goalRes.next.options.filter((o) => o.role === 'finish').every((o) => o.looped) &&
      nTags >= 1,
    `${goalRes?.next.options.filter((o) => o.looped).length} looped rows, ${nTags} tagged`,
  );
  // a borrowed fully looped veteran is a parent to use, not the goal: the note stays at step 1 and the planner plans on
  const borEx = JSON.parse(exText);
  for (const v of borEx.veterans) for (const g of v.parents) g.sparks = {};
  Object.assign(borEx.veterans[0], { borrowed: true, sparks: { ...all8 } });
  for (const g of borEx.veterans[0].parents) g.sparks = { ...all8 };
  await vetsOpen();
  await pg.click('#v-paste');
  await dialog(pg, 'paste-dlg');
  await pg.fill('#paste-text', JSON.stringify(borEx));
  await pg.click('#paste-load');
  await vetsShut();
  await done();
  await pg.waitForFunction(() => /Planned in/.test(document.getElementById('run-note').textContent), null, {
    timeout: 300000,
  });
  const borRes = await pg.evaluate(() => window.__careerResults.filter((r) => r.next).at(-1));
  check(
    'career: a borrowed fully looped veteran is a parent to use: the planner plans on, every career scored',
    borRes && borRes.next.mode === 'trio' && borRes.next.options.every((o) => o.score != null),
    `${borRes?.next.mode}`,
  );
  // the same veteran as yours: the goal goes on (the next fully looped parent, any uma), every career still scored
  borEx.veterans[0].borrowed = false;
  await vetsOpen();
  await pg.click('#v-paste');
  await dialog(pg, 'paste-dlg');
  await pg.fill('#paste-text', JSON.stringify(borEx));
  await pg.click('#paste-load');
  await vetsShut();
  await done();
  await pg.waitForFunction(() => /Planned in/.test(document.getElementById('run-note').textContent), null, {
    timeout: 300000,
  });
  const ownRes = await pg.evaluate(() => window.__careerResults.filter((r) => r.next).at(-1));
  const ownCheck =
    'career: with a fully looped parent of yours the planner goes for the next one and scores every career';
  check(
    ownCheck,
    ownRes && ownRes.next.mode === 'trio' && ownRes.next.options.every((o) => o.score != null),
    `${ownRes?.next.mode}`,
  );
  await expect.soft(pg.locator('[data-sec="options"] [data-tag="pick"]'), `${ownCheck}: one pick`).toHaveCount(1);
  // the trio path: the same veterans without one target and with their parents' sparks cleared: every career scored
  // against the cheapest trio, each step named, the pick explained
  const trioEx = JSON.parse(exText),
    drop = String(trioEx.job.skills.targets.find((x) => !x.guaranteed).group);
  for (const v of trioEx.veterans) {
    delete v.sparks[drop];
    for (const g of v.parents) g.sparks = {};
  }
  await vetsOpen();
  await pg.click('#v-paste');
  await dialog(pg, 'paste-dlg');
  await pg.fill('#paste-text', JSON.stringify(trioEx));
  await pg.click('#paste-load');
  await vetsShut();
  await done();
  await pg.waitForFunction(() => /Planned in/.test(document.getElementById('run-note').textContent), null, {
    timeout: 300000,
  });
  const trioRes = await pg.evaluate(() => window.__careerResults.filter((r) => r.next).at(-1));
  const trioCells = await pg.$$eval('[data-sec="options"] tbody tr', (ls) => ls.map((l) => l.children[2].textContent));
  // the plan behind the numbers sits in the "What do these mean?" popup: saves per run defined first, then the route and its formula
  await pg.click('#save-why');
  await dialog(pg, 'save-dlg');
  const trioLoop = await pg.textContent('#save-dlg [data-route]'),
    saveTxt = await pg.textContent('#save-dlg');
  await pg.mouse.click(5, 5); // a click on the backdrop closes it
  const trioCheck =
    'career: with no looped parent the plan scores every career against the cheapest trio; its explanation popup names each step and says what saves per run means';
  await expect
    .soft(pg.locator('#save-dlg'), `${trioCheck}: the popup closed by its backdrop`)
    .not.toHaveAttribute('open');
  await expect.soft(pg.locator('[data-sec="options"] [data-tag="pick"]'), `${trioCheck}: one pick`).toHaveCount(1);
  // (no caveat under the table: every row is proven, and a named assumption a proof rests on is the run note's to say;
  // the tie note and the negative-top line are statements about the scores, not caveats)
  await expect
    .soft(
      pg.locator('[data-sec="options"] p, [data-sec="options"] [data-route]'),
      `${trioCheck}: no caveat under the table`,
    )
    .toHaveCount(0);
  check(
    trioCheck,
    !!trioRes &&
      trioRes.next.mode === 'trio' &&
      trioRes.next.members.length === 2 &&
      /^First .+; then .+; then .+ with those two \d+\/\d+s as parents.*\. About [\d,]+ careers in all\.$/.test(
        trioLoop,
      ) &&
      /1 in [\d,]+/.test(trioLoop) &&
      /Extra careers: how many more careers than the pick this one costs on the planner's estimate \(below\) of the careers you still need for a fully looped parent .+\. .+ It is for comparing careers, not a forecast\./.test(
        saveTxt,
      ) &&
      /The planner makes a rough estimate of how many careers you still need \(about [\d,]+ today\)/.test(saveTxt) &&
      trioCells.length >= 2 &&
      trioCells.every((c) => /^(–|≈0|[+−]\d+\.\d)$/.test(c)),
    `${trioRes?.next.mode} | ${trioLoop} | ${trioCells.join(' ')}`,
  );
  check('no page errors', !pg.errors.length, pg.errors.join(' | '));
  await ctx.close();
}
