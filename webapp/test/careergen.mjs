// the career planner's pool grows: an uma.moe tree added, generations saved from the veteran you got (her parents the
// next one's grandparents), removals (from the 12 test veterans, parent 1 fixed so that saving has places to clear)
import { expect } from 'playwright/test';
import { check, dialog, careerPage } from './helpers.mjs';

export default async function careergen(b) {
  const { ctx, pg, done, vetsOpen, vetsShut, vetsSeen, node, inNode, simStars, simOpen, importDump, saved } =
    await careerPage(b);
  await importDump();
  await saved(); // (its run is cut short by the reload)
  await pg.evaluate(() => {
    const o = JSON.parse(localStorage.getItem('llp:career'));
    o.global.plan = { trainee: null, parents: [o.global.vets[0].id, null] };
    localStorage.setItem('llp:career', JSON.stringify(o));
  });
  await pg.reload();
  await done();
  const nVets = await pg.evaluate(() => JSON.parse(localStorage.getItem('llp:career')).global.vets.length);
  const cards = pg.locator('#vets .umcell'),
    runNote = pg.locator('#run-note');
  // an uma.moe lineage planner tree: one veteran with her parents, added after the others (factorId x 10 + level is the
  // game's factor id: Uma Stan 2, Racing Spirit: Stamina (factor group 21009, skill group 21010) 3, Dirt pink 1; saddle
  // 3 is a title (left out), 165 and 168 single-race G1s)
  await vetsOpen();
  await pg.click('#v-paste');
  await dialog(pg, 'paste-dlg');
  await pg.fill(
    '#paste-text',
    '{"version":1,"type":"lineage-planner","payload":[{"position":"p1","characterId":101901,"sparks":[{"factorId":201590,"level":2},{"factorId":210090,"level":3},{"factorId":120,"level":1}],"manualWinSaddleIds":[3,165,168]},{"position":"p1-1","characterId":102101,"sparks":[{"factorId":201650,"level":2}],"manualWinSaddleIds":[168]},{"position":"p1-2","characterId":102901,"sparks":[{"factorId":200490,"level":1}],"manualWinSaddleIds":[]}]}',
  );
  await pg.click('#paste-load');
  await done();
  const fromTree = await pg.evaluate(() => JSON.parse(localStorage.getItem('llp:career')).global.vets.at(-1));
  const treeAdds = 'career: an uma.moe tree adds her with her parents after your veterans, marked yours';
  await expect.soft(cards, `${treeAdds}: one card more`).toHaveCount(nVets + 1);
  check(
    `${treeAdds}: as stored`,
    fromTree.src === 'tree' &&
      fromTree.chara === 1019 &&
      !fromTree.borrowed &&
      fromTree.sparks[20159] === 2 &&
      fromTree.sparks[21010] === 3 &&
      fromTree.pinks[12] === 1 &&
      fromTree.wins.length === 2 &&
      fromTree.parents.map((p) => p.chara).join() === '1021,1029' &&
      fromTree.parents[0].sparks[20165] === 2 &&
      fromTree.parents[0].wins.length === 1,
    JSON.stringify(fromTree),
  );
  await expect.soft(cards.last(), `${treeAdds}: her card does not name uma.moe`).not.toHaveText(/From uma\.moe/);
  await expect.soft(pg.locator('#v-msg'), `${treeAdds}: the note says so`).toHaveText(/uma\.moe tree/);
  await vetsOpen();
  await cards.last().click(); // (beside the list)
  await expect
    .soft(pg.locator('#v-pane .srctag'), `${treeAdds}: her popup says she came from uma.moe`)
    .toHaveText('From uma.moe');
  await pg.locator('#vets .umcell').last().click();
  await pg.locator('#v-pane .vdel').click();
  await done();
  await vetsShut();
  // Save one generation, then another: a veteran's parents become the next one's grandparents.
  const beforeSimulation = await pg.evaluate(() => JSON.parse(localStorage.getItem('llp:career')).global.vets.length);
  await simStars(3);
  // Compare first: it prices her under today's plan's fit, so Add plans the pool with her afresh (one plan job, refitted)
  await pg.click('#sim-compare');
  await pg.waitForFunction(
    () => document.querySelector('#sim-verdict') && !document.querySelector('#sim-result .spin, #sim-result .runner'),
    null,
    { timeout: 180000 },
  );
  const planJobs = () =>
    pg.evaluate(() => window.__careerJobs.filter((j) => j.career?.next && !j.career?.compare).length);
  const pj0 = await planJobs(),
    cmpPick = await pg.evaluate(
      () => window.__careerResults.filter((r) => r.next?.options).at(-1)?.next.choice?.trainee.id,
    );
  await pg.click('#sim-add');
  await done();
  const addAfter =
    'career: Add to veterans after Compare plans the pool with her afresh (Compare priced her under the fit before her)';
  check(
    `${addAfter}: one plan job`,
    (await planJobs()) === pj0 + 1,
    `plan jobs ${pj0} -> ${await planJobs()}, compare's pick ${cmpPick}`,
  );
  await expect.soft(runNote, `${addAfter}: planned`).toHaveText(/Planned in|Weighed/);
  await vetsSeen(); // the veterans dialog draws its cards when open (and on opening)
  const adding = 'career: adding her saves a labelled veteran and opens the next career';
  check(
    `${adding}: as stored`,
    await pg.evaluate((n) => {
      const c = JSON.parse(localStorage.getItem('llp:career')).global;
      const v = c.vets.at(-1);
      return (
        c.vets.length === n + 1 &&
        v.src === 'entered' &&
        v.sparks[20159] === 3 &&
        v.parents.length === 2 &&
        v.parents.every((p) => p.chara > 0) &&
        c.plan.trainee === null &&
        c.plan.parents.every((p) => p === null)
      );
    }, beforeSimulation),
  );
  await expect.soft(pg.locator('#vets .simtag'), `${adding}: no tag on her card`).toHaveCount(0);
  await vetsOpen();
  await cards.last().click(); // (beside the list)
  await expect.soft(pg.locator('#v-pane .simtag'), `${adding}: her popup labels her`).toHaveText('Entered here');
  const simulated = await pg.evaluate(() => JSON.parse(localStorage.getItem('llp:career')).global.vets.at(-1));
  // Force this simulated veteran as a parent and complete another hypothetical career.
  await pg.evaluate((v) => {
    const o = JSON.parse(localStorage.getItem('llp:career'));
    o.global.plan = { trainee: null, parents: [v.id, null] };
    localStorage.setItem('llp:career', JSON.stringify(o));
  }, simulated);
  await pg.reload();
  await done();
  check(
    'career: her ancestry survives reloading',
    await pg.evaluate(
      (v) => JSON.stringify(JSON.parse(localStorage.getItem('llp:career')).global.vets.at(-1)) === JSON.stringify(v),
      simulated,
    ),
  );
  await simOpen();
  await pg.click('#sim-add');
  await done();
  check(
    'career: another generation keeps her as a grandparent',
    await pg.evaluate(
      (v) => {
        const o = JSON.parse(localStorage.getItem('llp:career')).global,
          next = o.vets.at(-1);
        return (
          o.vets.length === v.count + 2 &&
          next.parents.some((p) => p.chara === v.chara && JSON.stringify(p.sparks) === JSON.stringify(v.sparks))
        );
      },
      { ...simulated, count: beforeSimulation },
    ),
  );

  await node('node:p0');
  await inNode(/^(Choose|Change) parent$/).click();
  await dialog(pg, 'vpick-dlg');
  await pg.locator('#vp-list .vpt:not(:disabled)').last().click();
  await done();
  const removal = await pg.evaluate(() => {
    const c = JSON.parse(localStorage.getItem('llp:career')).global;
    return { id: c.plan.parents.find(Boolean), count: c.vets.length };
  });
  const removalIndex = await pg.evaluate(
    (id) => JSON.parse(localStorage.getItem('llp:career')).global.vets.findIndex((v) => v.id === id),
    removal.id,
  );
  await vetsOpen();
  await pg.locator(`#vet-${removalIndex}`).click();
  await pg.locator('#v-pane .vdel').click();
  await saved();
  // her card had the focus and is gone: the card chosen now takes it (review-8c §6)
  await expect
    .soft(pg.locator('#vets .umcell.sel'), 'focus: after Remove, the card chosen now has the focus (not the page)')
    .toBeFocused();
  const removing = 'career: removing a selected veteran immediately redraws and clears the fixed parent';
  await expect.soft(cards, `${removing}: one card fewer`).toHaveCount(removal.count - 1);
  check(
    `${removing}: as stored`,
    await pg.evaluate(({ id, count }) => {
      const c = JSON.parse(localStorage.getItem('llp:career')).global;
      return c.vets.length === count - 1 && !c.vets.some((v) => v.id === id) && !c.plan.parents.includes(id);
    }, removal),
  );
  await done();
  await pg.reload();
  await done();
  check(
    'career: removed veterans stay removed after reload',
    await pg.evaluate(
      (id) => !JSON.parse(localStorage.getItem('llp:career')).global.vets.some((v) => v.id === id),
      removal.id,
    ),
  );
  // an edit and a reload before its save is due (it lands after the next frame): the page saves it as it goes
  await vetsOpen();
  const quick = await pg.evaluate(async () => {
    const card = [...document.querySelectorAll('#vets .umcell')].at(-1),
      stored = () => JSON.parse(localStorage.getItem('llp:career')).global.vets;
    const id = stored()[+card.id.slice(4)].id;
    card.click(); // beside the list, with Remove
    await new Promise((r) => setTimeout(r)); // (the pane is a component: it draws her after the click's task)
    document.querySelector('#v-pane .vdel').click();
    const due = stored().some((v) => v.id === id);
    dispatchEvent(new PageTransitionEvent('pagehide'));
    return { id, due, flushed: !stored().some((v) => v.id === id) };
  });
  await Promise.all([
    pg.waitForEvent('load'),
    pg
      .evaluate(() => location.reload())
      .catch(() => {
        // the reload ends the evaluate's context: its rejection is expected
      }),
  ]);
  await done();
  check(
    'career: an edit followed at once by a reload is kept (written as the page goes)',
    quick.flushed &&
      (await pg.evaluate(
        (id) => !JSON.parse(localStorage.getItem('llp:career')).global.vets.some((v) => v.id === id),
        quick.id,
      )),
    `written before the page went: ${!quick.due}`,
  );
  await vetsOpen();
  while ((await cards.count()) > 1) {
    await cards.last().click();
    await pg.locator('#v-pane .vdel').click();
  }
  await done();
  const downToOne = 'career: removing down to one veteran clears obsolete indexed results without a page error';
  await expect.soft(cards, `${downToOne}: one card`).toHaveCount(1);
  await expect.soft(runNote, `${downToOne}: the note asks for parents`).toHaveText(/Choose both parents, or import/);
  await expect.soft(pg.locator('#simulation'), `${downToOne}: no Try a run`).toHaveCount(0);

  await pg.keyboard.press('Escape');
  check('no page errors', !pg.errors.length, pg.errors.join(' | '));
  await ctx.close();
}
