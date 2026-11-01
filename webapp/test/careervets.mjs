// the career planner's veteran pool, edited: a veteran's sparks and G1 wins, new parents and grandparents, the trainee,
// Use this, the settings, reloads, the uma.moe tree, generations saved, removals (from the 12 test veterans)
import { expect } from 'playwright/test';
import { check, dialog, careerPage, openEditor, closeEditor } from './helpers.mjs';

export default async function careervets(b) {
  const { ctx, pg, note, done, ask, vetsOpen, vetsShut, vetsSeen, node, inNode, closeNode, importDump, saved } =
    await careerPage(b);
  await importDump();
  await done();
  // a pool veteran: her card shows her beside the list, her sparks and her parents' as bars, editable in place; editing keeps the
  // result on screen while the search runs again
  const toteBefore = await pg.locator('#odds-open').count();
  const cards = pg.locator('#vets .umcell'),
    runNote = pg.locator('#run-note'),
    ghosts = pg.locator('#tree .pn.ghost');
  await vetsOpen();
  // (every card marked: one built again loses the mark)
  await pg.evaluate(() => {
    for (const c of document.querySelectorAll('#vets .umcell')) c.__kept = true;
  });
  await pg.click('#vet-1');
  await pg.click('#vet-0'); // (a wide dialog: she shows beside the list)
  const chip = pg.locator('#v-pane .udrow.me .vsk:not(.pink) .vskb').first(),
    ckey = await chip.getAttribute('data-k');
  const stars = async () =>
    +(await pg.locator(`#v-pane [data-k="${ckey}"]`).getAttribute('aria-label')).match(/(\d) star/)[1];
  const s0 = await stars();
  await chip.click();
  check(
    "career: a veteran's card shows her sparks and her parents' as bars beside the list; editing one keeps the result on screen until the new one is ready",
    (await pg.locator('#v-pane .udrow.par').count()) === 2 &&
      toteBefore === 1 &&
      (await pg.locator('#odds-open').count()) === 1 &&
      (await pg.locator('#tree .pn.ghost').count()) === 3,
  );
  await done();
  const rebuilt = await pg.evaluate(() =>
    [...document.querySelectorAll('#vets .umcell')].filter((c) => c.id !== 'vet-0' && !c.__kept).map((c) => c.id),
  );
  const noRebuild = 'career: choosing a veteran beside the list and editing her builds no other card again';
  await expect.soft(cards, `${noRebuild}: 12 cards`).toHaveCount(12);
  check(noRebuild, !rebuilt.length, rebuilt.join());
  const s1 = await stars();
  await pg.locator(`#v-pane [data-k="${ckey}"]`).click();
  const s2 = await stars();
  await pg.locator(`#v-pane [data-k="${ckey}"]`).click();
  const s3 = await stars();
  check(
    'career: clicking a spark cycles its stars back round (1 -> 2 -> 3 -> 1), in place',
    s1 === (s0 % 3) + 1 && s2 === (s1 % 3) + 1 && s3 === (s2 % 3) + 1,
    [s0, s1, s2, s3].join(' '),
  );
  await expect.soft(pg.locator(`#v-pane [data-k="${ckey}"]`), 'career: … the focus stays on it').toBeFocused();
  check(
    'career: … and the change is saved',
    await pg.evaluate(() => /"sparks"/.test(localStorage.getItem('llp:career'))),
  );
  const skills = pg.locator('#v-pane .udrow.me .vsk:not(.pink)'),
    n0 = await skills.count();
  const first = await pg.locator('#v-pane .udrow.me .vsk:not(.pink) .vskb').first().getAttribute('aria-label');
  await pg.locator('#v-pane .udrow.me .vsk:not(.pink)').first().hover();
  const vis = await pg
    .locator('#v-pane .udrow.me .vsk:not(.pink) .vskx')
    .first()
    .evaluate((e) => getComputedStyle(e).visibility);
  await pg.locator('#v-pane .udrow.me .vsk:not(.pink) .vskx').first().click();
  check('career: the x shows on hover', vis === 'visible', vis);
  await expect.soft(skills, 'career: … and removes the skill').toHaveCount(n0 - 1);
  await pg.locator('#v-pane .udrow.me .vskadd').click();
  const offered = await pg.$$eval('#add-pop .addsk', (els) => els.map((e) => e.textContent));
  check(
    'career: "+" offers the missing target only, as a spark cell (her blue, pink and unique have their own bars)',
    offered.length === 1 &&
      first.startsWith(offered[0]) &&
      (await pg
        .locator('#v-pane .udrow.me .vsb.blue, #v-pane .udrow.me .vsk.pink, #v-pane .udrow.me .vsb.pink')
        .count()) >= 2,
    offered.join(', '),
  );
  await pg.locator('#add-pop .addsk').first().click();
  await expect.soft(skills, 'career: … which comes back').toHaveCount(n0);
  await expect
    .soft(pg.locator('#v-pane .udrow.me .vskb', { hasText: offered[0] }), 'career: … at 2 stars')
    .toHaveAttribute('aria-label', /2 stars/);
  await done();
  // a pool veteran's G1 wins: the same editor, from her popup; Arima Kinen (Classic and Senior) is one race, moved from one turn to the other
  await pg.locator('#v-pane .twho .vwins').first().click();
  await dialog(pg, 'wins-dlg');
  const savedWins = async () => {
    await saved();
    return pg.evaluate(() => JSON.parse(localStorage.getItem('llp:career')).global.vets[0].wins);
  };
  const arima = pg.locator('#wins-dlg .wbtn[data-ix="22"]'),
    had = (await savedWins()).includes(22),
    n0w = (await savedWins()).length;
  if (had) {
    await arima.click();
  }
  const gone = !(await arima.count()) && (await savedWins()).length === n0w - (had ? 1 : 0);
  await pg.locator('#wins-dlg .ag-add[aria-label="Senior Year, Late Dec: add a G1 win"]').click();
  await pg.locator('#wins-dlg .wmenu button', { hasText: 'Arima Kinen' }).click();
  const winsEdit =
    "career: a veteran's G1 wins: a click removes Arima Kinen, the Senior Late Dec menu adds it back there, saved";
  await expect.soft(arima, `${winsEdit}: back`).toHaveCount(1);
  await expect.soft(pg.locator('#wins-count'), `${winsEdit}: counted`).toHaveText(/\d+ G1 win/);
  check(
    winsEdit,
    gone &&
      /Late Dec/.test(await arima.evaluate((e) => e.closest('.ag-slot').textContent)) &&
      (await arima.evaluate((e) => e.closest('.ag-year').classList.contains('y2'))) &&
      (await savedWins()).includes(22),
    `had ${had}`,
  );
  if (!had) await arima.click();
  await pg.locator('#wins-dlg').getByRole('button', { name: 'Done', exact: true }).click();
  await done();
  await vetsShut();
  // parent 1: a new veteran, with the skills prompt
  await node('node:p0');
  await inNode(/^(Choose|Change) parent$/).click();
  await dialog(pg, 'vpick-dlg');
  await expect.soft(pg.locator('#vp-list .vpt'), 'career: the parent picker lists your veterans').toHaveCount(12);
  await pg.click('#vp-tab-new');
  await pg.fill('#vp-filter', 'Special Week');
  await pg.locator('#vp-umas .upick:visible').first().click();
  await dialog(pg, 'ask-dlg');
  await expect
    .soft(pg.locator('#ask-title'), 'career: picking a new uma asks whether to add the skills')
    .toHaveText(/Add skills to Special Week/);
  await ask('All 8 at 2★');
  await done();
  const closedEmpty =
    'career: the closed parent picker and node popup hold no rows or node (an edit elsewhere redraws none of them)';
  await expect.soft(pg.locator('#vp-list > *'), `${closedEmpty}: the picker`).toHaveCount(0);
  await expect.soft(pg.locator('#node-body > *'), `${closedEmpty}: the popup`).toHaveCount(0);
  const p1 = pg.locator('.pf').nth(0).locator('.pn.parent');
  const sparksIn = async (key, n) => {
    await node(key);
    const ok = await pg
      .locator('#node-dlg .udrow.me .vsk:not(.pink) .vskb')
      .evaluateAll(
        (els, n) => els.length === 8 && els.every((e) => new RegExp(n + ' stars').test(e.getAttribute('aria-label'))),
        n,
      );
    await closeNode();
    return ok;
  };
  await expect.soft(p1, 'career: … she is parent 1').toHaveText(/Special Week/);
  await expect.soft(p1, 'career: … she is parent 1: chosen').not.toHaveClass(/ghost/);
  check('career: … with the eight targets at 2 stars (in her popup)', await sparksIn('node:p0', 2));
  await vetsSeen(); // the veterans dialog draws its cards when open (and on opening)
  const herParents =
    'career: … her parents are not known yet but carry the same eight sparks, and she joins your veterans';
  await expect
    .soft(pg.locator('.pf').nth(0).locator('.pn.gp', { hasText: 'Unknown' }), `${herParents}: unknown`)
    .toHaveCount(2);
  check(`${herParents}: their sparks`, (await sparksIn('node:p0g0', 2)) && (await sparksIn('node:p0g1', 2)));
  await expect.soft(cards, `${herParents}: 13 veterans`).toHaveCount(13);
  await expect.soft(pg.locator('#vet-12 .umtab'), `${herParents}: her card`).toHaveText(/Parent 1/);
  await expect.soft(runNote, 'career: the planner fills the rest').toHaveText(/Weighed 804 careers/);
  // a grandparent: pick her uma, then her skills
  await node('node:p0g0');
  await pg.locator('#node-dlg .facebtn').click();
  await dialog(pg, 'pick-dlg');
  await pg.fill('#pick-filter', 'Gold Ship');
  await pg.locator('#pick-list .upick:visible').first().click();
  await ask('All 8 at 3★');
  await done();
  const g1 = pg.locator('.pf').nth(0).locator('.pn.gp').first();
  await expect.soft(g1, 'career: a grandparent takes an uma').toHaveText(/Gold Ship/);
  check('career: … and the skills chosen for her', await sparksIn('node:p0g0', 3));
  // the other parent can't be the same uma
  await node('node:p1');
  await inNode(/^(Choose|Change) parent$/).click();
  await dialog(pg, 'vpick-dlg');
  const blocked = await pg.locator('#vp-list .vpt:disabled').count(),
    said = await pg.locator('#vp-list .vpt:disabled .vptw').count(); // (read while open: a closed picker holds no rows)
  await pg.locator('#vpick-dlg').getByRole('button', { name: 'Close', exact: true }).click();
  check("career: veterans that can't be parent 2 are greyed out", blocked === 0 || said === blocked);
  // the trainee
  await node('node:t');
  await inNode(/^(Choose|Change) trainee$/).click();
  await dialog(pg, 'pick-dlg');
  await pg.fill('#pick-filter', 'Agnes Digital');
  await pg.locator('#pick-list .upick:visible').first().click();
  await done();
  await expect.soft(pg.locator('#tree .pn.trainee'), 'career: the chosen trainee is set').not.toHaveClass(/ghost/);
  check(
    'career: … and the picks list only parents',
    await pg.$$eval('.alts tbody .altw', (ws) => ws.every((w) => /^Parents /.test(w.textContent.trim()))),
  );
  await pg
    .locator('.alts tbody tr')
    .filter({ has: pg.getByRole('button', { name: 'Use this' }) })
    .first()
    .getByRole('button', { name: 'Use this' })
    .click();
  await done();
  const useThis = 'career: "Use this" sets the places; with all set the career is just scored';
  await expect.soft(runNote, useThis).toHaveText(/Scored in/);
  await expect.soft(ghosts, `${useThis}: no place left`).toHaveCount(0);
  // the last plan's table stays (it once went with the plan, so its other careers were out of reach), her row marked
  const saves = await pg.$$eval('.alts tbody .onum', (ns) => ns.map((n) => n.textContent.trim()));
  const tableStays =
    "career: … the last plan's careers stay listed, with their savings, the planner's pick and hers marked";
  await expect.soft(pg.locator('.alts'), `${tableStays}: the table`).toHaveCount(1);
  await expect.soft(pg.locator('.alts tbody tr.on'), `${tableStays}: hers`).toHaveText(/shown above/);
  await expect.soft(pg.locator('.alts .otag.pick'), `${tableStays}: the pick`).toHaveCount(1);
  check(
    `${tableStays}: the savings`,
    saves.some((x) => /^[+−]\d/.test(x)),
    saves.join(' '),
  );
  // the chosen parent's popup shows its actions: the footer (Close, Change parent, Let the planner pick) wraps on narrow
  // popups and once ran past the popup's bottom, hiding the only way to unselect a parent; desktop and phone
  await node('node:p1');
  const footIn = () =>
    pg.evaluate(() => {
      const d = document.getElementById('node-dlg').getBoundingClientRect();
      return [...document.getElementById('node-dlg').lastElementChild.querySelectorAll('button')].every((b) => {
        const r = b.getBoundingClientRect();
        return r.top >= d.top && r.bottom <= d.bottom + 0.5 && r.bottom <= innerHeight;
      });
    });
  const vp0 = pg.viewportSize(),
    footWide = await footIn();
  await pg.setViewportSize({ width: 375, height: 740 });
  const footPhone = await footIn();
  await pg.setViewportSize(vp0);
  const footer =
    "career: a chosen parent's popup shows Change parent and Let the planner pick inside it (desktop and phone)";
  check(footer, footWide && footPhone, `desktop ${footWide}, phone ${footPhone}`);
  await expect.soft(inNode('Let the planner pick'), `${footer}: the button`).toHaveCount(1);
  await inNode('Let the planner pick').click();
  await closeNode();
  await done();
  await expect.soft(ghosts, 'career: "Let the planner pick" hands a place back').toHaveCount(1);
  await expect.soft(runNote, 'career: "Let the planner pick" hands a place back: weighed').toHaveText(/Weighed/);
  // back to the plan's places, then her again: both answers were drawn before, so neither runs a job
  const jobsNow = () => pg.evaluate(() => window.__careerJobs.filter((j) => j.cmd === 'career').length),
    jobs0 = await jobsNow();
  await pg.click('#opt-back');
  await done();
  const backTo =
    'career: "Back to the planner\'s pick" restores the plan\'s places, and an answer seen before is drawn without a run';
  await expect.soft(ghosts, `${backTo}: the plan's places`).toHaveCount(2);
  await expect.soft(runNote, `${backTo}: planned`).toHaveText(/Planned in/);
  const backOk = (await jobsNow()) === jobs0;
  await node('node:t');
  await inNode(/^(Choose|Change) trainee$/).click();
  await dialog(pg, 'pick-dlg');
  await pg.fill('#pick-filter', 'Agnes Digital');
  await pg.locator('#pick-list .upick:visible').first().click();
  await done();
  await expect.soft(ghosts, `${backTo}: her place`).toHaveCount(1);
  check(
    `${backTo}: no run`,
    backOk && (await jobsNow()) === jobs0,
    `${await note()}; jobs ${jobs0} -> ${await jobsNow()}`,
  );
  // a settings change searches again
  await openEditor(pg, 'umas');
  const umasNow = () =>
    pg.evaluate(() => {
      const g = JSON.parse(localStorage.getItem('llp:settings') || '{}').global || {};
      return JSON.stringify([g.roster, g.outfits]);
    }); // (nothing saved before the first settings change)
  const before = await umasNow(),
    searches = await pg.evaluate(() => window.__careerJobs.filter((j) => j.cmd === 'career').length);
  await pg.locator('#set-umas #roster .ut .up').nth(3).click();
  await closeEditor(pg, 'umas');
  await done();
  check(
    'career: the Umas dialog changes the owned umas and searches again',
    (await umasNow()) !== before &&
      (await pg.evaluate((n) => window.__careerJobs.filter((j) => j.cmd === 'career').length > n, searches)),
    before,
  );
  await pg.reload();
  await done();
  await vetsSeen(); // the veterans dialog draws its cards when open (and on opening)
  const outlive = 'career: the veterans and the chosen places outlive a reload';
  await expect.soft(cards, `${outlive}: 13 veterans`).toHaveCount(13);
  await expect.soft(pg.locator('.pf').nth(0), `${outlive}: parent 1`).toHaveText(/Special Week/);
  await expect.soft(pg.locator('#tree .pn.trainee'), `${outlive}: the trainee`).toHaveText(/Agnes Digital/);
  await vetsOpen();
  await pg.click('#v-paste');
  await dialog(pg, 'paste-dlg');
  await pg.fill('#paste-text', '{"x": 1}');
  await pg.click('#paste-load');
  const badPaste = 'career: a paste that is not an export says so and changes nothing';
  await expect.soft(pg.locator('#v-msg'), badPaste).toHaveText(/No veterans found/);
  await expect.soft(cards, `${badPaste}: 13 veterans`).toHaveCount(13);
  await pg.keyboard.press('Escape');
  check('no page errors', !pg.errors.length, pg.errors.join(' | '));
  await vetsShut(); // (the table is on the page, under the dialog)
  // a career taken from the table holds for its setup: a settings change hands the places back to the planner
  await pg
    .locator('.alts tbody tr')
    .filter({ has: pg.getByRole('button', { name: 'Use this' }) })
    .first()
    .getByRole('button', { name: 'Use this' })
    .click();
  await done();
  await pg.click('#bp button[data-v="never"]');
  await done();
  const handBack = 'career: a career taken with "Use this" goes back to the planner when a setting changes';
  await expect.soft(ghosts, handBack).toHaveCount(3);
  check(
    `${handBack}: as stored`,
    await pg.evaluate(() => JSON.parse(localStorage.getItem('llp:career')).global.plan.parents.every((p) => !p)),
  );
  await pg.click('#bp button[data-v="allow"]');
  await done();
  await ctx.close();
}
