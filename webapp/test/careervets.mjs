// the career planner's veteran pool, edited: a veteran's sparks and G1 wins, new parents and grandparents, the trainee,
// Use this, the settings, reloads, the uma.moe tree, generations saved, removals (from the 12 test veterans)
import { check, dialog, careerPage } from './helpers.mjs';

export default async function careervets(b) {
  const { ctx, pg, note, done, ask, vetsOpen, vetsSeen, node, inNode, closeNode, importDump, saved } = await careerPage(b);
  await importDump(); await done();
  // a pool veteran: her card opens her popup, her sparks and her parents' as bars, editable in place; editing keeps the
  // result on screen while the search runs again
  const toteBefore = await pg.locator('#odds-open').count();
  await vetsOpen(); await pg.click('#vet-0 .vtile'); await dialog(pg, 'node-dlg');
  const chip = pg.locator('#node-dlg .udrow.me .vsk:not(.pink) .vskb').first(), ckey = await chip.getAttribute('data-k');
  const stars = async () => +(await pg.locator(`#node-dlg [data-k="${ckey}"]`).getAttribute('aria-label')).match(/(\d) star/)[1];
  const s0 = await stars(); await chip.click();
  check('career: a veteran\'s card opens her sparks and her parents\' as bars; editing one keeps the result on screen until the new one is ready',
    await pg.locator('#node-dlg .udrow.par').count() === 2 && toteBefore === 1 && await pg.locator('#odds-open').count() === 1 && await pg.locator('#tree .pn.ghost').count() === 3);
  await done();
  const s1 = await stars(); await pg.locator(`#node-dlg [data-k="${ckey}"]`).click(); const s2 = await stars(); await pg.locator(`#node-dlg [data-k="${ckey}"]`).click(); const s3 = await stars();
  check('career: clicking a spark cycles its stars back round (1 -> 2 -> 3 -> 1), in place', s1 === s0 % 3 + 1 && s2 === s1 % 3 + 1 && s3 === s2 % 3 + 1, [s0, s1, s2, s3].join(' '));
  check('career: … the focus stays on it, and the change is saved', await pg.evaluate(k => document.activeElement?.dataset?.k === k, ckey) && await pg.evaluate(() => /"sparks"/.test(localStorage.getItem('llp:career'))));
  const nsk = () => pg.locator('#node-dlg .udrow.me .vsk:not(.pink)').count(), n0 = await nsk();
  const first = await pg.locator('#node-dlg .udrow.me .vsk:not(.pink) .vskb').first().getAttribute('aria-label');
  await pg.locator('#node-dlg .udrow.me .vsk:not(.pink)').first().hover();
  const vis = await pg.locator('#node-dlg .udrow.me .vsk:not(.pink) .vskx').first().evaluate(e => getComputedStyle(e).visibility);
  await pg.locator('#node-dlg .udrow.me .vsk:not(.pink) .vskx').first().click();
  check('career: the x shows on hover and removes the skill', vis === 'visible' && await nsk() === n0 - 1);
  await pg.locator('#node-dlg .udrow.me .vskadd').click();
  const offered = await pg.$$eval('#add-pop .addsk', els => els.map(e => e.textContent));
  check('career: "+" offers the missing target only, as a spark cell (her blue, pink and unique have their own bars)', offered.length === 1 && first.startsWith(offered[0])
    && !(await pg.locator('#add-pop .addpk').count()) && await pg.locator('#node-dlg .udrow.me .vsb.blue, #node-dlg .udrow.me .vsk.pink, #node-dlg .udrow.me .vsb.pink').count() >= 2, offered.join(', '));
  await pg.locator('#add-pop .addsk').first().click();
  check('career: … which comes back at 2 stars', await nsk() === n0 && /2 stars/.test(await pg.locator('#node-dlg .udrow.me .vskb', { hasText: offered[0] }).getAttribute('aria-label')));
  await done();
  // a pool veteran's G1 wins: the same editor, from her popup; Arima Kinen (Classic and Senior) is one race, moved from one turn to the other
  await pg.locator('#node-dlg .twho .vwins').first().click(); await dialog(pg, 'wins-dlg');
  const savedWins = async () => { await saved(); return pg.evaluate(() => JSON.parse(localStorage.getItem('llp:career')).global.vets[0].wins); };
  const arima = pg.locator('#wins-dlg .wbtn[data-ix="22"]'), had = (await savedWins()).includes(22), n0w = (await savedWins()).length;
  if (had) { await arima.click(); }
  const gone = !(await arima.count()) && (await savedWins()).length === n0w - (had ? 1 : 0);
  await pg.locator('#wins-dlg .ag-add[aria-label="Senior Year, Late Dec: add a G1 win"]').click();
  await pg.locator('#wins-dlg .wpick', { hasText: 'Arima Kinen' }).click();
  check('career: a veteran\'s G1 wins: a click removes Arima Kinen, the Senior Late Dec menu adds it back there, saved', gone && await arima.count() === 1
    && /Late Dec/.test(await arima.evaluate(e => e.closest('.ag-slot').textContent)) && await arima.evaluate(e => e.closest('.ag-year').classList.contains('y2'))
    && (await savedWins()).includes(22) && /\d+ G1 win/.test(await pg.textContent('#wins-count')), `had ${had}`);
  if (!had) await arima.click();
  await pg.locator('#wins-dlg .dclose').click(); await done();
  await closeNode();
  // parent 1: a new veteran, with the skills prompt
  await node('node:p0'); await inNode(/^(Choose|Change) parent$/).click(); await dialog(pg, 'vpick-dlg');
  check('career: the parent picker lists your veterans', await pg.locator('#vp-list .vpt').count() === 12);
  await pg.click('#vp-tab-new'); await pg.fill('#vp-filter', 'Special Week'); await pg.locator('#vp-umas .upick:visible').first().click();
  await dialog(pg, 'ask-dlg');
  check('career: picking a new uma asks whether to add the skills', /Add skills to Special Week/.test(await pg.textContent('#ask-title')));
  await ask('All 8 at 2★'); await done();
  const p1 = pg.locator('.pf').nth(0).locator('.pn.parent');
  const sparksIn = async (key, n) => { await node(key); const ok = await pg.locator('#node-dlg .udrow.me .vsk:not(.pink) .vskb').evaluateAll((els, n) => els.length === 8 && els.every(e => new RegExp(n + ' stars').test(e.getAttribute('aria-label'))), n); await closeNode(); return ok; };
  check('career: … she is parent 1, with the eight targets at 2 stars (in her popup)', /Special Week/.test(await p1.textContent()) && !(await p1.getAttribute('class')).includes('ghost')
    && await sparksIn('node:p0', 2));
  await vetsSeen();   // the veterans dialog draws its cards when open (and on opening)
  check('career: … her parents are not known yet but carry the same eight sparks, and she joins your veterans', await pg.locator('.pf').nth(0).locator('.pn.gp', { hasText: 'Unknown' }).count() === 2
    && await sparksIn('node:p0g0', 2) && await sparksIn('node:p0g1', 2)
    && await pg.locator('#vets .vet').count() === 13 && /Parent 1/.test(await pg.locator('#vet-12 .vtop').textContent()));
  check('career: the planner fills the rest', /Weighed 804 careers/.test(await note()), await note());
  // a grandparent: pick her uma, then her skills
  await node('node:p0g0'); await pg.locator('#node-dlg .facebtn').click(); await dialog(pg, 'pick-dlg');
  await pg.fill('#pick-filter', 'Gold Ship'); await pg.locator('#pick-list .upick:visible').first().click(); await ask('All 8 at 3★'); await done();
  const g1 = pg.locator('.pf').nth(0).locator('.pn.gp').first();
  check('career: a grandparent takes an uma and the skills chosen for her', /Gold Ship/.test(await g1.textContent()) && await sparksIn('node:p0g0', 3));
  // the other parent can't be the same uma
  await node('node:p1'); await inNode(/^(Choose|Change) parent$/).click(); await dialog(pg, 'vpick-dlg');
  const blocked = await pg.locator('#vp-list .vpt:disabled').count();
  await pg.locator('#vpick-dlg .dclose').click();
  check('career: veterans that can\'t be parent 2 are greyed out', blocked === 0 || await pg.locator('#vp-list .vpt:disabled .vptw').count() === blocked);
  // the trainee
  await node('node:t'); await inNode(/^(Choose|Change) trainee$/).click(); await dialog(pg, 'pick-dlg');
  await pg.fill('#pick-filter', 'Agnes Digital'); await pg.locator('#pick-list .upick:visible').first().click(); await done();
  check('career: the chosen trainee is set and the picks list only parents', !(await pg.locator('#tree .pn.trainee').getAttribute('class')).includes('ghost')
    && (await pg.$$eval('.alts tbody .altw', ws => ws.every(w => /^Parents /.test(w.textContent.trim())))));
  await pg.locator('.alts tbody tr').filter({ has: pg.getByRole('button', { name: 'Use this' }) }).first().getByRole('button', { name: 'Use this' }).click(); await done();
  check('career: "Use this" sets the places; with all set the career is just scored', /Scored in/.test(await note())
    && await pg.locator('#tree .pn.ghost').count() === 0, await note());
  // the last plan's table stays (it once went with the plan, so its other careers were out of reach), her row marked
  const saves = await pg.$$eval('.alts tbody .onum', ns => ns.map(n => n.textContent.trim()));
  check('career: … the last plan\'s careers stay listed, with their savings, the planner\'s pick and hers marked', await pg.locator('.alts').count() === 1
    && /shown above/.test(await pg.locator('.alts tbody tr.on').textContent()) && await pg.locator('.alts .otag.pick').count() === 1
    && saves.some(x => /^[+−]\d/.test(x)), saves.join(' '));
  // the chosen parent's popup shows its actions: the footer (Close, Change parent, Let the planner pick) wraps on narrow
  // popups and once ran past the popup's bottom, hiding the only way to unselect a parent; desktop and phone
  await node('node:p1');
  const footIn = () => pg.evaluate(() => { const d = document.getElementById('node-dlg').getBoundingClientRect();
    return [...document.querySelectorAll('#node-dlg .gmdfoot button')].every(b => { const r = b.getBoundingClientRect(); return r.top >= d.top && r.bottom <= d.bottom + 0.5 && r.bottom <= innerHeight; }); });
  const vp0 = pg.viewportSize(), footWide = await footIn(); await pg.setViewportSize({ width: 375, height: 740 }); const footPhone = await footIn(); await pg.setViewportSize(vp0);
  check('career: a chosen parent\'s popup shows Change parent and Let the planner pick inside it (desktop and phone)', footWide && footPhone
    && await inNode('Let the planner pick').count() === 1, `desktop ${footWide}, phone ${footPhone}`);
  await inNode('Let the planner pick').click(); await closeNode(); await done();
  check('career: "Let the planner pick" hands a place back', await pg.locator('#tree .pn.ghost').count() === 1 && /Weighed/.test(await note()));
  // back to the plan's places, then her again: both answers were drawn before, so neither runs a job
  const jobsNow = () => pg.evaluate(() => window.__careerJobs.filter(j => j.cmd === 'career').length), jobs0 = await jobsNow();
  await pg.click('#opt-back'); await done();
  const backOk = await pg.locator('#tree .pn.ghost').count() === 2 && /Planned in/.test(await note()) && await jobsNow() === jobs0;
  await node('node:t'); await inNode(/^(Choose|Change) trainee$/).click(); await dialog(pg, 'pick-dlg');
  await pg.fill('#pick-filter', 'Agnes Digital'); await pg.locator('#pick-list .upick:visible').first().click(); await done();
  check('career: "Back to the planner\'s pick" restores the plan\'s places, and an answer seen before is drawn without a run', backOk
    && await pg.locator('#tree .pn.ghost').count() === 1 && await jobsNow() === jobs0, `${await note()}; jobs ${jobs0} -> ${await jobsNow()}`);
  // a settings change searches again
  await pg.click('#o-umas'); await dialog(pg, 'set-umas');
  const umasNow = () => pg.evaluate(() => { const g = JSON.parse(localStorage.getItem('llp:settings') || '{}').global || {}; return JSON.stringify([g.roster, g.outfits]); });   // (nothing saved before the first settings change)
  const before = await umasNow(), searches = await pg.evaluate(() => window.__careerJobs.filter(j => j.cmd === 'career').length);
  await pg.locator('#set-umas #roster .ut .up').nth(3).click(); await pg.locator('#set-umas .dclose').click(); await done();
  check('career: the Umas dialog changes the owned umas and searches again', (await umasNow()) !== before
    && await pg.evaluate(n => window.__careerJobs.filter(j => j.cmd === 'career').length > n, searches), before);
  await pg.reload(); await done();
  await vetsSeen();   // the veterans dialog draws its cards when open (and on opening)
  check('career: the veterans and the chosen places outlive a reload', await pg.locator('#vets .vet').count() === 13
    && /Special Week/.test(await pg.locator('.pf').nth(0).textContent()) && /Agnes Digital/.test(await pg.locator('#tree .pn.trainee').textContent()));
  await vetsOpen(); await pg.click('#v-paste'); await dialog(pg, 'paste-dlg'); await pg.fill('#paste-text', '{"x": 1}'); await pg.click('#paste-load');
  check('career: a paste that is not an export says so and changes nothing', /No veterans found/.test(await pg.textContent('#v-msg')) && await pg.locator('#vets .vet').count() === 13);
  await pg.keyboard.press('Escape');
  check('no page errors', !pg.errors.length, pg.errors.join(' | '));
  await ctx.close();
}
