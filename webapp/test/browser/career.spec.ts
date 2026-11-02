// The career planner (the front page) on the page's real wiring: from no veterans to umadump's 12 and their plan, then
// what the page draws of it at real widths; and Try a run on that plan (Compare, rolls, the worker's kept state, her
// card's editors and their focus). The logic behind each piece is pinned in node (test/unit: careerio, careerroll,
// proofs, careerrun, analysis); these flows hold the page's own drawing, layout, focus and worker.
import fs from 'node:fs';
import { chip, FRIENDS_TAG } from '../../src/ui/deckChip.ts';
import {
  careerHooks,
  importDump,
  jobCount,
  node,
  oneAnswer,
  planned,
  resetSeq,
  showsNow,
  simOpen,
  simStars,
  storedVets,
  vetsShut,
  withPlan,
} from './career.ts';
import { expect, test } from './fixtures.ts';
import { careerDone, SOLVER_WAIT } from './pages.ts';

test.use({ viewport: { width: 1440, height: 1000 }, seed: { 'llp:setup-folded': '0' } });
test.beforeEach(async ({ context }) => {
  await context.addInitScript(careerHooks);
});

const ART = fs.existsSync(new URL('../../img/uma', import.meta.url));
// each flow's limit: replayed, a few seconds of solver waits; recording (LLP_SOLVER=live), the live solver's plans
const LIMIT = process.env.LLP_SOLVER === 'live' ? 900_000 : 120_000;

test('from no veterans to a plan for 12: the setup panel, the import, one answer per run, and the plan drawn @both', async ({
  page,
  context,
  browserName,
}) => {
  test.setTimeout(LIMIT);
  await page.goto('/');
  await careerDone(page);
  const runNote = page.locator('#run-note');
  await expect(runNote, 'with no veterans it asks for parents or an import').toHaveText(
    /Choose both parents, or import/,
  );
  await expect(page.locator('#run-act'), '… and offers sample veterans').toHaveText(/Try sample veterans/);
  // the setup panel: the shared settings, the Enhance buttons, Target skills in the open
  const setup = page.getByRole('region', { name: 'Career setup' });
  await expect(setup.locator('#o-umas, #o-cards')).toHaveCount(2);
  await expect(setup.locator('#skills-panel #targets > [data-g]')).toHaveCount(8);
  await expect(page.locator('#o-skills, #set-skills')).toHaveCount(0);
  // the presets popup in that panel: one column, its tiles the dialog's width, the buttons under them
  await page.click('#t-preset');
  await page.locator('#pr-list button[aria-haspopup="dialog"]').first().click();
  const preset = await page.evaluate(() => {
    const tiles = document.getElementById('prv-tiles'),
      panel = tiles?.parentElement,
      rename = document.getElementById('prv-rename')?.parentElement;
    if (!tiles || !panel || !rename) return null;
    const d = panel.getBoundingClientRect(),
      t = tiles.getBoundingClientRect();
    return {
      cols: getComputedStyle(panel).gridTemplateColumns.split(' ').length,
      wide: t.width > d.width - 60,
      below: rename.getBoundingClientRect().top >= t.bottom,
    };
  });
  expect(preset, 'the preset popup is laid out as on the planner').toEqual({ cols: 1, wide: true, below: true });
  await page.locator('#pr-view').getByRole('button', { name: 'Close', exact: true }).click();
  await page.locator('#preset-dlg').getByRole('button', { name: 'Done', exact: true }).click();
  // the pink ribbon folds the panel to a one-line summary (its edge too, not the strip beside it); the summary opens it
  const gameData = page.getByRole('group', { name: 'Game data' });
  const ribbon = await page.locator('#h-set').boundingBox();
  const bar = await setup.boundingBox();
  if (!ribbon || !bar) throw new Error('the setup panel and its ribbon are drawn');
  await setup.click({ position: { x: ribbon.x - bar.x + ribbon.width + 60, y: ribbon.y - bar.y + ribbon.height / 2 } });
  await expect(gameData, 'the strip beside the ribbon leaves it open').toBeVisible();
  const edge = await page.locator('#h-set').boundingBox();
  if (!edge) throw new Error('the ribbon is drawn');
  await page.mouse.click(edge.x + 6, edge.y + 4);
  await expect(gameData, "the ribbon's edge folds it").toBeHidden();
  await expect(page.locator('#set-sum')).toHaveAttribute(
    'aria-label',
    /^Global · .+ · rank (B–S\+|SS) · 8 target skills$/,
  );
  await page.click('#set-sum');
  await expect(gameData, 'the summary opens it').toBeVisible();
  await page.click('#set-fold');
  await expect(gameData).toBeHidden();
  await page.click('#set-fold');
  await expect(gameData).toBeVisible();
  await expect(page.locator('#set-fold')).toHaveAttribute('aria-expanded', 'true');

  // the sample veterans, then umadump's file: the first plan is one answer
  await page.click('#run-act');
  await careerDone(page);
  await resetSeq(page);
  await importDump(page);
  const vbtn = page.locator('#v-toggle');
  await expect(vbtn).toHaveAttribute('aria-label', 'Your veterans');
  await expect(vbtn, 'named as in the game').toHaveText(/^\s*Veteran Umamusume\s*$/);
  await expect(page.locator('#set-vets #vets [data-vet]')).toHaveCount(12);
  await expect(page.locator('footer #v-export'), 'Export at the foot of the page').toHaveCount(1);
  await careerDone(page);
  await planned(page);
  const first = await oneAnswer(page, 'Any uma/');
  expect(first.ok, first.detail).toBe(true);
  expect(first.phases, 'it searches, then plans').toEqual(['search', 'plan']);
  await vetsShut(page);
  const plan = await page.evaluate(() => {
    const r = window.__careerResults.findLast((x) => x.kind === 'search' && x.next?.kind === 'plan');
    return r?.kind === 'search' && r.next?.kind === 'plan' ? r.next : null;
  });
  if (!plan) throw new Error('a plan answered');
  const trainee = page.locator('#tree [data-node="trainee"] [data-nm]');
  await expect(trainee, 'the trainee on show is the pick').toHaveText(plan.choice?.trainee.name ?? '(no pick)');
  await page.click('#save-why');
  await expect(page.locator('#save-dlg [data-note]', { hasText: /ran out of memory/ }), 'no memory note').toHaveCount(
    0,
  );
  await expect(page.locator('#save-dlg a[href="about#career-planner"]'), 'What these mean links to /about').toHaveCount(
    1,
  );
  // (the route's line, in the fold under How it is worked out)
  const route = (await page.locator('#save-dlg [data-route]').first().textContent()) ?? '';
  if (plan.members.length)
    expect(route, 'the route, per step its odds now').toMatch(/your \d+\/\d+ of .+ \(no careers\)|1 in [\d,]+/);
  const negativeTop = await page.locator('#save-dlg [data-note]', { hasText: /saves its own run/ }).count();
  await page.keyboard.press('Escape');

  // Export: Copy puts the text on the clipboard (Chromium grants it), Download saves it
  await page.click('#v-export');
  await expect(page.locator('#vx-dlg')).toHaveAttribute('open');
  const exported = await page.inputValue('#vx-text');
  if (browserName === 'chromium') await context.grantPermissions(['clipboard-read', 'clipboard-write']);
  await page.click('#vx-copy');
  await expect(page.locator('#vx-msg')).toHaveText('Copied.');
  if (browserName === 'chromium') expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(exported);
  const [download] = await Promise.all([page.waitForEvent('download'), page.click('#vx-dl')]);
  expect(fs.readFileSync(await download.path(), 'utf8'), 'Download saves the same text').toBe(exported);
  await page.locator('#vx-dlg').getByRole('button', { name: 'Close', exact: true }).click();

  // the plan drawn: the tree, the odds and counts, the targets and their Skill Details, the deck
  await expect(runNote).toHaveText(/Weighed [\d,]+ careers/);
  await expect(runNote, 'only a check that could not finish is mentioned').not.toHaveText(
    /could not finish|checked exact/,
  );
  await expect(page.locator('#tree [data-ghost]')).toHaveCount(3);
  await expect(page.locator('#tree [data-node="gp"]')).toHaveCount(4);
  await expect(page.locator('#lodds #odds-open')).toHaveText(/^1 in [\d,.]+ to land all 8/);
  await expect(page.locator('#tree [data-sec="targets"] [data-avg] b')).toHaveText(/^Avg \d\.\d$/);
  const counts = await page.$$eval('#tree [data-sec="targets"] [data-counts] [data-count]', (l) =>
    l.map((c) => [c.lastElementChild?.textContent ?? '', c.querySelector('b')?.textContent ?? '']),
  );
  expect(counts.map(([n]) => n)).toEqual(['8/8', '7/8', '6/8', '5/8', '4/8', '3/8', '2/8', '1/8', '0/8']);
  for (const [, p] of counts) expect(p).toMatch(/^[<\d.]+%$/);
  const rows = await page.$$eval('#tree [data-target]', (l) =>
    l.map((x) => ({
      land: Number.parseFloat(x.querySelector(':scope > button b')?.textContent ?? ''),
      bar: x.querySelectorAll('[title^="Acquired"] i').length,
      legs: x.querySelector('[title^="Inherited"]')?.getAttribute('title') ?? '',
      deck: x.querySelector('[data-deck-figure]')?.getAttribute('title') ?? '',
    })),
  );
  expect(rows).toHaveLength(8);
  rows.forEach((t, i) => {
    expect(t.land, 'the targets lowest first').toBeGreaterThanOrEqual(rows[i - 1]?.land ?? 0);
    expect(t.bar).toBe(2);
    expect(t.legs).toMatch(/^Inherited: [<\d.]+%$/);
  });
  // the deck's chance of hinting a target is one figure wherever it shows (odds.ts deckHint: its gold, else its white)
  const targets = [...(plan.choice?.targets ?? [])].sort((a, b) => a.p - b.p);
  // (the deck's part of a sourced buy; a scenario's skill has none)
  const deckOf = (t: (typeof targets)[number]) =>
    t.buy?.kind === 'sourced' ? { gold: t.buy.deck_gold, white: t.buy.deck_white } : { gold: 0, white: 0 };
  const hint = (t: (typeof targets)[number]) => 1 - (1 - deckOf(t).gold) * (1 - deckOf(t).white);
  const pct = (text: string, p: number) => {
    const v = Number.parseFloat(text.replace(/^Deck: /, '')),
      pc = p * 100;
    return Math.abs(v - pc) <= (pc >= 10 ? 0.5 : pc >= 1 ? 0.05 : 0.005) + 1e-9;
  };
  const shown = targets.map((t, i) => ({ text: rows[i]?.deck ?? '', p: hint(t) })).filter((r) => r.text);
  expect(shown.length).toBeGreaterThan(0);
  for (const r of shown) expect(pct(r.text, r.p), JSON.stringify(r)).toBe(true);
  const both = targets
    .map((t, i) => ({ i, gw: deckOf(t).gold * deckOf(t).white, p: hint(t) }))
    .sort((a, b) => b.gw - a.gw)[0];
  if (!both) throw new Error('targets');
  expect(both.gw).toBeGreaterThan(0.01);
  await page.locator('#tree [data-target] > button').nth(both.i).click();
  const sk = page.locator('#sk-dlg');
  await expect(sk).toHaveAttribute('open');
  await expect(sk.locator('[data-faces] [data-av]'), "Skill Details' six legacies").toHaveCount(6);
  await expect(sk).toHaveText(/Lands as a spark: [<\d.]+%/);
  await expect(sk).toHaveText(/Legacies carrying it/);
  const deckLines = await page.$$eval('#sk-dlg ul li', (l) =>
    l
      .filter((x) => /^Deck/.test(x.querySelector('span')?.textContent ?? ''))
      .map((x) => Number.parseFloat(x.querySelector('b')?.textContent ?? '') || 0),
  );
  expect(deckLines, "Skill Details' two deck lines").toHaveLength(2);
  expect(Math.abs((deckLines[0] ?? 0) + (deckLines[1] ?? 0) - both.p * 100)).toBeLessThanOrEqual(1);
  await page.keyboard.press('Escape');
  await page.click('#deck-open');
  await expect(page.locator('#deck-dlg #tree-deck button[data-name]'), "the deck's six cards").toHaveCount(6);
  await page.keyboard.press('Escape');
  // affinity: the trainee's total alone in the tree; her popup breaks it down legacy by legacy, to the same sum
  await expect(page.locator('#tree').getByLabel(/^Affinity \d/)).toHaveCount(1);
  const affinity = page.locator('#tree [data-node="trainee"]').getByLabel(/^Affinity \d/);
  const total = Number((await affinity.textContent())?.replace(/\D/g, ''));
  await node(page, 'node:t');
  const table = page.locator('#node-dlg').getByRole('table', { name: 'Affinity' });
  await expect(table.locator('tbody tr')).toHaveCount(6);
  const sum = Number(
    (
      await table
        .locator('tfoot')
        .getByLabel(/^Affinity \d+$/)
        .textContent()
    )?.replace(/\D/g, ''),
  );
  expect(Math.abs(sum - total)).toBeLessThanOrEqual(1);
  await page.keyboard.press('Escape');
  // the plan job: the six best by odds, then the plan; the pick on show, tagged; one outfit named
  const job = await page.evaluate(() =>
    window.__careerJobs.findLast((j) => j.cmd === 'career' && !!j.career.next && !j.career.compare),
  );
  expect(job?.cmd === 'career' && job.career.top === 6 && job.career.per_trainee === false).toBe(true);
  expect(plan.options.length).toBeGreaterThanOrEqual(2);
  expect(plan.rule).toBe('index');
  await expect(page.locator('#tree')).not.toHaveText(/any outfit/i);
  await expect(page.locator('#results')).not.toHaveText(/any outfit/i);
  await expect(page.locator('[data-sec="options"] [data-tag="pick"]')).toHaveCount(1);

  // her schedule: the game's Scheduled Races, three years at once without scrolling; the Win table scrolls at 375 px
  await expect(page.locator('#tree [data-node="trainee"] #sched-open')).toHaveCount(1);
  await expect(page.locator('#tree [data-node="trainee"] #deck-open')).toHaveCount(1);
  const runs = Number(/She runs (\d+) G1s/.exec((await page.getAttribute('#sched-open', 'title')) ?? '')?.[1]);
  expect(runs).toBeGreaterThan(0);
  await page.click('#sched-open');
  const sched = page.locator('#sched-dlg');
  await expect(sched).toHaveAttribute('open');
  await expect(sched.locator('[data-slot~="on"]')).toHaveCount(runs);
  await expect(sched.locator('[data-race]')).toHaveCount(runs);
  await expect(sched.locator('button[data-race]'), 'no win buttons').toHaveCount(0);
  await expect(sched.locator('[data-sched-key]').first()).toHaveText(/A parent won it/);
  await expect(sched.getByRole('tab')).toHaveCount(1);
  await expect(sched.locator('[data-year]')).toHaveCount(3);
  await expect(sched.locator('[data-slot~="pre"]'), "Junior's April to June grey").toHaveCount(12);
  await expect(sched.locator('button[data-k^="add:"]'), 'no plus').toHaveCount(0);
  expect(
    await page.evaluate(() => {
      const d = document.getElementById('sched-body');
      return !!d && d.scrollHeight <= d.clientHeight + 1;
    }),
    'no scrolling',
  ).toBe(true);
  await sched.locator('button', { hasText: 'Win table' }).click();
  const wt = await page.$$eval('#sched-wintable tbody tr', (rs) =>
    rs.map((r) => [...r.children].map((c) => c.textContent)),
  );
  const heads = await page.$$eval('#sched-wintable thead th', (hs) =>
    hs.slice(1).map((h) => h.querySelectorAll('[data-grade]').length),
  );
  expect(heads).toEqual(Array(13).fill(2));
  expect(wt).toHaveLength(6);
  expect(wt[0]?.join()).toBe('1 Race,110%,100%,90%,80%,70%,50%,20%,10%,0%,0%,0%,0%,0%');
  expect(wt[2]?.slice(0, 5).join()).toBe('3 Races,105%,95%,85%,75%');
  expect(wt[5]?.slice(0, 7).join()).toBe('6 or more Races,60%,50%,40%,30%,20%,0%');
  await page.setViewportSize({ width: 375, height: 812 });
  const phone = await page.evaluate(() => {
    const w = document.querySelector('#sched-wintable > div');
    return { page: document.documentElement.scrollWidth, scrolls: !!w && w.scrollWidth > w.clientWidth };
  });
  expect(phone, 'at 375 px the Win table scrolls inside its panel').toEqual({ page: 375, scrolls: true });
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.keyboard.press('Escape');

  // Other careers: "1 in X" and the extra careers against the pick (≈0 within 0.25, – for the pick), the pick above
  const opts = await page.$$eval('[data-sec="options"] tbody tr', (ls) =>
    ls.map((l) => [...l.children].map((c) => c.textContent ?? '')),
  );
  expect(opts.length).toBeGreaterThanOrEqual(2);
  for (const c of opts) {
    expect(c[1]).toMatch(/^1 in [\d,.]+$/);
    expect(c[2]).toMatch(/^(–|≈0|[+−]\d+\.\d)$/);
  }
  expect(opts.filter((c) => c[2] === '–').length).toBeGreaterThanOrEqual(1);
  await expect(page.locator('[data-sec="options"] > h3').first()).toHaveText(/^Other careers/);
  await expect(page.locator('#save-why')).toHaveCount(1);
  await expect(
    page.locator('[data-sec="options"] tbody tr[aria-current="true"]', { hasText: 'shown above' }),
  ).toHaveCount(1);
  await expect(page.locator('#ahead-sec'), 'no after-this-career section').toHaveCount(0);
  // each row's Saves per run says what her run does: the side her routes lift and its chance, or her place on the route
  const roleForm =
    /^(Makes one of the two parents your fully looped parent needs|The last run: both her parents are already full veterans|(Already in your loop\. )?(Her veteran helps build .+'s parent: [\d.<%]+ of her runs do|Her veteran can be one of the two parents \(an \d+\/\d+\): [\d.<%]+ of her runs|No run of hers helps the plan|Not compared against the plan))(\. In all, .+ of her runs leave a veteran that helps)?$/;
  const titles = await page.$$eval('[data-sec="options"] tbody [data-extra][title]', (l) =>
    l.map((t) => t.getAttribute('title') ?? ''),
  );
  expect(titles.length).toBeGreaterThan(0);
  for (const t of titles) expect(t).toMatch(roleForm);
  const names = new Map(plan.options.map((o) => [o.report.trainee.id, o.report.trainee.name]));
  for (const o of plan.options.slice(0, 8)) {
    if (o.role === 'plan' || o.role === 'finish' || o.index == null) continue;
    const side = o.lifts_side;
    if (!side || side.uma == null || side.chance < side.own || !names.has(side.uma)) continue;
    const text = `Her veteran helps build ${names.get(side.uma)}'s parent: `;
    expect(
      titles.some((t) => t.startsWith(text) || t.includes(`. ${text}`)),
      text,
    ).toBe(true);
  }
  // rows within 0.25 of the pick show ≈0 and no tie tag; the (i) popup's line only where the top score is below 0
  const scored = plan.options.filter((o) => o.score != null);
  const pickScore = plan.options.find((o) => o.chosen)?.score ?? null;
  const top = Math.max(...scored.map((o) => o.score ?? Number.NEGATIVE_INFINITY));
  expect(pickScore).toBe(top);
  const tied = scored.filter(
    (o) => !o.chosen && pickScore !== null && Math.abs(pickScore - (o.score ?? 0)) <= 0.25,
  ).length;
  const extras = opts.filter((c) => c[2] !== '–').map((c) => c[2] ?? '');
  const near = extras.filter((x) => x === '≈0').length;
  expect(near).toBeLessThanOrEqual(tied);
  if (tied) expect(near).toBeGreaterThanOrEqual(1);
  for (const x of extras) if (x !== '≈0') expect(Math.abs(Number(x.replace('−', '-')))).toBeGreaterThanOrEqual(0.2);
  await expect(page.locator('[data-sec="options"] [data-tag="tie"]')).toHaveCount(0);
  expect(negativeTop).toBe(top < 0 ? 1 : 0);
  // the page's order: the odds and Try a run beside Next career, the tree then the other careers, her card in a popup
  expect(
    await page.evaluate(() => {
      const t = document.getElementById('tree'),
        r = document.getElementById('results'),
        dlg = document.getElementById('sim-dlg');
      return (
        !!t &&
        !!r &&
        dlg instanceof HTMLDialogElement &&
        [...t.children].map((k) => (k instanceof HTMLElement ? k.dataset.sec : '')).join() ===
          'trainee,legacies,targets' &&
        !!document.querySelector('#lodds #odds-open') &&
        !!document.querySelector('#lodds #sim-try') &&
        t.nextElementSibling === r &&
        !r.querySelector('#sim-top, #goal-note') &&
        !!dlg.querySelector('#sim-top #simulation #sim-compare') &&
        dlg.querySelector('#simulation > h3')?.textContent === 'Veteran analysis' &&
        !dlg.open
      );
    }),
  ).toBe(true);

  // her card, the game's Umamusume Details screen, wide and on a 375 px phone with no sideways scroll
  const card = () =>
    page.evaluate(() => {
      const c = document.querySelector('#simulation [data-details]');
      const face = c?.querySelector('[data-part="head"] [data-av]');
      const ins = c?.querySelector('#udpane-insp');
      const me = ins?.querySelector('[data-row="me"]');
      const sparks = me?.querySelector('[aria-label="Her sparks"]');
      const portrait = me?.querySelector(':scope > [data-av]');
      if (!c || !face || !ins || !sparks || !portrait) return 'no card';
      const tabs = [...c.querySelectorAll('[role="tab"]')];
      const apt = c.querySelector('[role="table"][aria-label="Her aptitudes"]');
      const fr = portrait.getBoundingClientRect(),
        sr = sparks.getBoundingClientRect();
      const wrong = [
        face.getAttribute('role') !== 'img' && 'portrait role',
        !/\S/.test(face.getAttribute('aria-label') ?? '') && 'portrait name',
        Math.round(face.getBoundingClientRect().width) !== 96 && 'portrait 96 px',
        tabs.map((t) => t.textContent).join('|') !== 'Skills|Inspiration|Career Info' && 'tabs',
        tabs.filter((t) => t.getAttribute('aria-selected') === 'true').length !== 1 && 'one tab selected',
        !!apt &&
          (apt.querySelectorAll('[role="row"]').length !== 3 || apt.querySelectorAll('[role="cell"]').length !== 10) &&
          'aptitudes',
        !(fr.right <= sr.left && fr.top < sr.bottom) && 'sparks beside her portrait',
        ins.querySelectorAll('[data-row="parent"]').length !== 2 && 'two parents',
        !c.querySelector('#udpane-info')?.contains(document.getElementById('sim-wins')) && 'wins in Career Info',
        c.getBoundingClientRect().right > innerWidth && 'card on screen',
        document.documentElement.scrollWidth > document.documentElement.clientWidth && 'sideways scroll',
      ].filter(Boolean);
      return wrong.join(', ') || 'ok';
    });
  await simOpen(page);
  await expect.poll(card, 'wide').toBe('ok');
  await page.setViewportSize({ width: 375, height: 812 });
  await expect.poll(card, 'at 375 px').toBe('ok');
  await page.setViewportSize({ width: 1440, height: 1000 });
  // Skills: the targets she could acquire as the game's tiles, acquired ones bright; a tile opens Skill Details, and
  // closing it returns the focus to a tile; an arrow key moves to the next tab
  await page.click('#udtab-skills');
  await expect(page.locator('#udpane-skills')).toBeVisible();
  await expect(page.locator('#udpane-insp')).toBeHidden();
  const tiles = await page.$$eval('#udpane-skills button[data-k^="sim:sk"]', (l) =>
    l.map((t) => [t.hasAttribute('data-off'), t.getAttribute('aria-label') ?? ''] as const),
  );
  const whites = await page.locator('#udpane-insp [aria-label="Her sparks"] [data-spark="white"]').count();
  expect(tiles.length).toBeGreaterThan(0);
  for (const [off, label] of tiles) {
    expect(label).toMatch(/: (acquired|not acquired)/);
    expect(off).toBe(/: not acquired/.test(label));
  }
  expect(tiles.filter(([off]) => !off).length).toBeGreaterThanOrEqual(whites);
  await page.locator('#udpane-skills button[data-k^="sim:sk"]').first().click();
  await expect(sk).toHaveText(/Skill Details/);
  await expect(sk).toHaveText(/(Acquired|Not acquired)( \(gold\))? in this result/);
  await expect(sk).toHaveText(/Where it comes from/);
  await expect(sk).toHaveText(/Lands as a spark/);
  await page.keyboard.press('Escape');
  await expect(page.locator('#udpane-skills button[data-k^="sim:sk"]:focus')).toHaveCount(1);
  await page.focus('#udtab-skills');
  await page.keyboard.press('ArrowRight');
  await expect(page.locator('#udtab-insp')).toBeFocused();
  await expect(page.locator('#udtab-insp')).toHaveAttribute('aria-selected', 'true');
  // the veteran you got starts as a random result with at most one pink spark, and nothing is compared yet
  const pinks = await page.$$eval('#simulation [aria-label="Her sparks"] [data-spark="pink"] [data-bar]', (l) =>
    l.map((b) => b.getAttribute('aria-label') ?? ''),
  );
  expect(pinks.length).toBeLessThanOrEqual(1);
  for (const p of pinks) expect(p).toMatch(/ \(aptitude\), \d stars?/);
  expect(await jobCount(page, 'compare')).toBe(0);
  await expect(page.locator('#sim-verdict')).toHaveCount(0);
});

const VERDICTS =
  /^(Saves \d+\.\d careers|No careers saved|Better odds|Parent of the next career|\d+\/\d+: step 1 done|Fully looped parent)/;

test("Try a run: an edit's one dimmed answer, Compare and its job, rolls from the kept plan, her card's editors and their focus", async ({
  page,
}) => {
  test.setTimeout(LIMIT);
  // (the page's clock: a roll's exact check starts once the roll has stood 0.8 s, moved past here at once, so each
  // roll's jobs go out in one order however fast the replies come)
  await page.clock.install();
  await withPlan(page);
  // the deck's card panel marks a card not owned: the old answer stays dimmed through both phases, then the new one
  await page.click('#deck-open');
  const deckCard = page.locator('#tree-deck button[data-name]').first();
  const cid = await deckCard.getAttribute('data-cid');
  await deckCard.click();
  const own = page.locator('[data-pop="card"] [data-act="own"]');
  await expect(page.locator('[data-pop="card"]:not([hidden]) [data-act="own"]')).toHaveCount(1);
  const pre = await showsNow(page);
  await resetSeq(page);
  // (this plan's report says its scoring was cut short by the memory budget: the page must say so)
  await page.evaluate(() => {
    window.__planIncomplete = true;
  });
  await own.uncheck();
  await careerDone(page);
  await planned(page);
  const edit = await oneAnswer(page, pre);
  expect(edit.ok, edit.detail).toBe(true);
  expect(edit.phases).toEqual(['search', 'plan']);
  await page.evaluate(() => {
    window.__planIncomplete = false;
  });
  if (await page.evaluate(() => document.querySelector('#deck-dlg[open]') !== null))
    await page.locator('#deck-dlg').getByRole('button', { name: 'Close', exact: true }).click();
  await page.click('#save-why');
  await expect(
    page.locator('#save-dlg [data-note]', { hasText: /ran out of memory.+may be too low/ }),
    "a plan whose scoring was cut short says so in the careers table's (i) popup",
  ).toHaveCount(1);
  await page.keyboard.press('Escape');
  await page.click('#deck-open');
  await deckCard.click();
  // an edit undone within the pause runs nothing and never dims the answer
  if (!(await page.locator('[data-pop="card"]:not([hidden]) [data-act="own"]').count())) await deckCard.click();
  const pre2 = await showsNow(page);
  await resetSeq(page);
  await page.evaluate(() => {
    const c = document.querySelector('[data-pop="card"] [data-act="own"]');
    if (c instanceof HTMLElement) {
      c.click();
      c.click();
    }
  });
  await careerDone(page);
  const seq = await page.evaluate(() => window.__seq);
  expect(seq.length).toBeGreaterThan(0);
  expect(
    seq.some((s) => s.stale),
    'never dimmed',
  ).toBe(false);
  expect(await showsNow(page)).toBe(pre2);
  expect(
    await page.evaluate(
      (id) => JSON.parse(localStorage.getItem('llp:settings') ?? '{}').global?.cardsOff?.[id ?? ''],
      cid,
    ),
    'the card stored not owned',
  ).toBe(false);
  const own2 = page.locator(`#tree-deck button[data-cid="${cid}"]`);
  if (await own2.count())
    await expect(page.locator(`#tree-deck button[data-tag="borrowed"][data-cid="${cid}"]`), 'or borrowed').toHaveCount(
      1,
    );
  // every deck on the page lists a borrowed card last: the sixth slot, as in the game
  const decks = await page.$$eval('[data-deck-cards]', (l) =>
    l
      .map((r) =>
        [...r.querySelectorAll(':scope > button')].map((e) => e instanceof HTMLElement && e.dataset.tag === 'borrowed'),
      )
      .filter((k) => k.length),
  );
  expect(decks.filter((k) => k.some((b, i) => b && k.slice(i + 1).some((x) => !x)))).toEqual([]);
  if (await page.evaluate(() => document.querySelector('#deck-dlg[open]') !== null))
    await page.locator('#deck-dlg').getByRole('button', { name: 'Close', exact: true }).click();

  // Compare: what she changes for the next career, that career with her; its job fixes her stars, wins and parents
  const original = await storedVets(page);
  await simStars(page, 3);
  expect(await storedVets(page), 'editing her leaves your veterans untouched').toBe(original);
  expect(await jobCount(page, 'compare'), '… and runs nothing').toBe(0);
  await page.click('#sim-compare');
  const verdict = page.locator('#sim-verdict'),
    result = page.locator('#sim-result');
  await expect(verdict).toHaveText(VERDICTS, { timeout: SOLVER_WAIT });
  await expect(result).toHaveText(/→/);
  await expect(result).not.toHaveText(/Careers until one lands|Your best career today →/);
  await expect(page.locator('#sim-result [data-odds] b').first()).toHaveText(/^(about [\d,]+|\?) → ([\d,]+|\?)$/);
  await expect(verdict).not.toHaveText(/Saves [\d,]+ careers|rises by [\d,]+ with/);
  await expect(page.locator('#sim-result details:not([open]) [data-av]')).toHaveCount(3);
  await expect(page.locator('#sim-result details [data-her]')).toHaveCount(1);
  await expect(page.locator('#sim-result details:not([open]) button[data-name]')).toHaveCount(6);
  await expect(page.locator('#sim-result > [data-odds] > div')).toHaveCount(2);
  await expect(page.locator('#sim-result details summary')).toHaveText(/^Best career with her$/);
  await expect(page.locator('#sim-result > [data-fit]'), 'a line saying Add refits').toHaveCount(1);
  const compareJob = await page.evaluate(() => {
    const j = window.__careerJobs.findLast((x) => x.cmd === 'career' && !!x.career.compare && !x.career.next);
    const priced = window.__careerJobs.findLast((x) => x.cmd === 'career' && !!x.career.compare && !!x.career.next);
    const fit = window.__careerResults.findLast((r) => r.kind === 'search' && r.next?.kind === 'plan' && !r.compare);
    if (j?.cmd !== 'career' || priced?.cmd !== 'career' || fit?.kind !== 'search' || fit.next?.kind !== 'plan')
      return null;
    const her = j.career.legacies.at(-1);
    return {
      fixed: j.career.compare === true && (j.career.fix ?? []).join() === String(j.career.legacies.length - 1),
      stars: (her?.sparks ?? []).some(({ group, stars }) => group === 20159 && stars === 3),
      wins: !!her && (her.wins ?? []).every(Number.isInteger),
      parents:
        her?.parents?.length === 2 && her.parents.every((p) => !!p && p.chara > 0 && (p.sparks ?? []).length > 0),
      fit: JSON.stringify(priced.career.next) === JSON.stringify({ fit: fit.next.fit }),
    };
  });
  expect(
    compareJob,
    "the comparison fixes her stars, wins and parents and prices the pool under the plan's fit",
  ).toEqual({
    fixed: true,
    stars: true,
    wins: true,
    parents: true,
    fit: true,
  });
  // a roll: a random result of the career (the page's random numbers stubbed), compared at once
  const roll = async (u: number) => {
    const before = await jobCount(page, 'compare');
    await page.evaluate((x) => {
      const r = Math.random;
      Math.random = () => x;
      document.getElementById('sim-roll')?.click();
      Math.random = r;
    }, u);
    await page.waitForFunction(
      (n) =>
        window.__careerJobs.filter((j) => j.cmd === 'career' && j.career.compare).length > n &&
        !!document.querySelector('#sim-verdict') &&
        !document.querySelector('#sim-result [data-run="best"]'),
      before,
      { timeout: SOLVER_WAIT },
    );
    await page.clock.runFor(1000);
    await page.waitForFunction(
      () =>
        !!document.querySelector('#sim-verdict') &&
        !document.querySelector('#sim-result [data-run], #sim-result [data-spinner]'),
      null,
      { timeout: SOLVER_WAIT },
    );
  };
  await roll(0.999999999);
  const all = await page.$$eval('#simulation [aria-label="Her sparks"] [data-spark="white"] [data-bar]', (l) =>
    l.map((b) => b.getAttribute('aria-label') ?? ''),
  );
  expect(all, 'Reroll draws from the odds: everything at 3 stars').toHaveLength(8);
  for (const a of all) expect(a).toMatch(/3 stars/);
  expect(await storedVets(page)).toBe(original);
  // its saving is the solver's for that roll, from the plan on show (kept, not planned again), in a call of its own
  const lr = await page.evaluate(() => {
    const jobs = window.__careerJobs;
    const j = jobs.findLast((x) => x.cmd === 'career' && !!x.career.next?.roll);
    const r = window.__careerResults.findLast((x) => x.kind === 'roll');
    if (j?.cmd !== 'career' || r?.kind !== 'roll' || r.roll.kind !== 'answer') return null;
    const { roll: _, ...plain } = j.career.next ?? {};
    const plan = JSON.stringify({ ...j, career: { ...j.career, next: plain } });
    const ir = jobs.lastIndexOf(j),
      ic = jobs.findLastIndex((x) => x.cmd === 'career' && !!x.career.compare && !x.career.next);
    const y = r.roll.outcome.kind === 'saving' ? r.roll.outcome.y : -1;
    return {
      same: jobs.some((p) => p.cmd === 'career' && !!p.career.next && !p.career.compare && JSON.stringify(p) === plan),
      first: ic > ir || y >= 0.5,
      stars: j.career.next?.roll ?? [],
      y,
      planned: !!r.roll.planned,
      verdict: document.getElementById('sim-verdict')?.textContent ?? '',
    };
  });
  if (!lr) throw new Error('a roll answered');
  expect(lr.same && lr.first && !lr.planned, JSON.stringify(lr)).toBe(true);
  expect(lr.stars).toEqual(Array(8).fill(3));
  if (lr.y < 0.5) expect(lr.verdict).toMatch(/^No (careers saved|saving found yet)/);
  // (a roll that stops an exact comparison, whose next worker restores the stored state, is not replayable: which state
  // the page stored before the kill is a matter of its store's timer; it is left to a recording run)
  await page.waitForFunction(() => !document.querySelector('#sim-result [data-run]'), null, { timeout: SOLVER_WAIT });
  // a low roll lands nothing, draws her pink at 1 star, and her blue spark (its bar opens the five stats and the stars)
  await roll(0);
  await expect(page.locator('#simulation [aria-label="Her sparks"] [data-spark="white"]')).toHaveCount(0);
  const low = await page.$$eval('#simulation [aria-label="Her sparks"] [data-spark="pink"] [data-bar]', (l) =>
    l.map((b) => b.getAttribute('aria-label') ?? ''),
  );
  expect(low.length).toBeLessThanOrEqual(1);
  for (const p of low) expect(p).toMatch(/, 1 star\./);
  const blue = page.locator('#udpane-insp [aria-label="Her sparks"] [data-spark="blue"] [data-bar]').first();
  await expect(blue).toHaveAttribute('aria-label', /^(Speed|Stamina|Power|Guts|Wit) \(stat\), [123] stars?/);
  await blue.click();
  await expect(page.locator('#col-dlg')).toHaveAttribute('open');
  expect(await page.$$eval('#col-dlg [data-bar]', (l) => l.map((b) => b.textContent))).toEqual([
    'Speed',
    'Stamina',
    'Power',
    'Guts',
    'Wit',
  ]);
  await page.locator('#col-dlg').getByRole('button', { name: '3 stars', exact: true }).click();
  await expect(blue).toHaveAttribute('aria-label', /, 3 stars/);
  await page.keyboard.press('Escape');
  await expect(page.locator('#sim-reset'), 'no "back to the most likely result" button').toHaveCount(0);
  await expect(
    page.locator('#simulation [data-part="head"]').locator('#sim-compare, #sim-roll, #sim-add'),
    "Compare, Roll and Add in her card's header",
  ).toHaveCount(3);
  // an n/n veteran: Compare says step 1 of the goal is done (or, with n/n parents, a fully looped parent)
  for (let i = 0; i < 12; i++) {
    await page.locator('#simulation [data-k$=":add"]').click();
    const b = page.locator('#add-pop button').first();
    if (!(await b.count())) {
      await page.locator('#simulation [data-k$=":add"]').click();
      break;
    }
    await b.click();
  }
  await page.click('#sim-compare');
  await expect(verdict).toHaveText(/^((\d+)\/\2: step 1 done|Fully looped parent)/, { timeout: SOLVER_WAIT });
  await page.waitForFunction(() => !document.querySelector('#sim-result [data-spinner]'), null, {
    timeout: SOLVER_WAIT,
  });

  // Career Info: her cards in one row, each at most 64 px, a card's panel opens and closes; her record matches the
  // wins button; a borrowed card's friend frame stays on its card
  const wins = async () => {
    const m = /^(No|\d+) G1 win/.exec((await page.textContent('#sim-wins')) ?? '');
    return m?.[1] === 'No' ? 0 : Number(m?.[1]);
  };
  const w0 = await wins();
  await page.click('#udtab-info');
  const deckCells = await page.$$eval('#udpane-info [data-deck-cards] > button', (l) =>
    l.map((c) => {
      const r = c.getBoundingClientRect();
      return [Math.round(r.width), Math.round(r.top)];
    }),
  );
  expect(deckCells.length).toBeGreaterThanOrEqual(1);
  for (const [w] of deckCells) expect(w).toBeLessThanOrEqual(64);
  expect(new Set(deckCells.map(([, top]) => top)).size, 'one row').toBe(1);
  const infoCard = page.locator('#udpane-info [data-deck-cards] > button').first();
  await infoCard.click();
  await expect(page.locator('#sim-dlg [data-pop="card"]:not([hidden])')).toHaveCount(1);
  await infoCard.click();
  await expect(page.locator('#sim-dlg [data-pop="card"]:not([hidden])')).toHaveCount(0);
  const record = await page.$$eval('#ud-races > li', (l) =>
    l.map((r) => r.querySelector('[aria-label]')?.getAttribute('aria-label') === 'won'),
  );
  await expect(page.locator('#ud-races'), 'the races behind a button').toBeHidden();
  expect(record.length).toBeGreaterThan(0);
  expect(record.filter(Boolean).length).toBe(w0);
  await expect(page.locator('#udpane-info')).toHaveText(new RegExp(`Wins: ${w0}(?!\\d)`));
  await page.click('#udpane-info [data-k="sim:races"]');
  await expect(page.locator('#ud-races')).toBeVisible();
  await expect(page.locator('#udpane-info button[aria-label^="Legacy "]')).toHaveCount(2);
  await expect(page.locator('#udpane-info [data-av][role="img"]'), 'her legacies with their parents').toHaveCount(4);
  if (ART) {
    const frame = await page.evaluate(
      ({ look, tag }) => {
        const row = document.querySelector('#udpane-info [data-deck-cards]');
        const c = row?.querySelector(':scope > button')?.cloneNode(true);
        if (!row || !(c instanceof HTMLElement)) return null;
        c.className = look;
        c.appendChild(Object.assign(document.createElement('span'), { className: tag, textContent: 'Friends' }));
        row.appendChild(c);
        const r = c.getBoundingClientRect(),
          a = getComputedStyle(c, '::after'),
          last = c.querySelector(':scope > span:last-child');
        const out = {
          fits: Number.parseFloat(a.width) <= r.width + 10 && Number.parseFloat(a.height) <= r.height + 20,
          ring: getComputedStyle(c).boxShadow,
          tag: last ? getComputedStyle(last).display : '',
        };
        c.remove();
        return out;
      },
      { look: chip({ tag: 'borrowed', friends: true }), tag: FRIENDS_TAG },
    );
    expect(frame, "a borrowed card's friend frame stays on its card, without the ring and tag").toEqual({
      fits: true,
      ring: 'none',
      tag: 'none',
    });
  }
  // her G1 wins: one per slot; a click removes one and its slot offers to add; the menu puts it back; closing returns the
  // focus to the wins button; the comparison goes out of date, nothing runs and nothing is saved to your veterans
  const compared = await jobCount(page, 'compare');
  const stored = await storedVets(page);
  await page.click('#sim-wins');
  const winsDlg = page.locator('#wins-dlg');
  await expect(winsDlg).toHaveAttribute('open');
  await expect(winsDlg.locator('button[data-race]')).toHaveCount(w0);
  await expect(winsDlg.locator('[data-year]')).toHaveCount(3);
  await expect(page.locator('#wins-title')).toHaveText(/G1 wins/);
  const first = winsDlg.locator('button[data-race]').first();
  const race = ((await first.getAttribute('aria-label')) ?? '').replace(/, won: remove$/, '');
  await first.click();
  expect(
    await page.evaluate(() =>
      document.activeElement?.matches('#wins-dlg button[data-k^="add:"]')
        ? document.activeElement.getAttribute('aria-label')
        : '',
    ),
  ).toMatch(/: add a G1 win$/);
  expect(await wins()).toBe(w0 - 1);
  await expect(page.locator('#wins-count')).toHaveText(new RegExp(`^${w0 - 1 || 'No'}\\b`));
  await page.keyboard.press('Enter');
  const picks = await page.$$eval('#wins-dlg [data-pop="wins"]:not([hidden]) button', (l) =>
    l.map((e) => e.textContent ?? ''),
  );
  expect(picks[0]?.startsWith(race), "the slot's menu lists the race held then first").toBe(true);
  expect(picks[0]).toMatch(/her schedule · \d+% win/);
  await page.locator('#wins-dlg [data-pop="wins"] button', { hasText: race }).click();
  expect(await wins()).toBe(w0);
  expect(await page.evaluate(() => document.activeElement?.getAttribute('aria-label'))).toBe(`${race}, won: remove`);
  await expect(result, 'the comparison out of date').toHaveText(/before your last edit/);
  expect(await jobCount(page, 'compare'), 'none run').toBe(compared);
  expect(await storedVets(page), 'none saved').toBe(stored);
  await page.keyboard.press('Escape');
  await expect(page.locator('#sim-wins'), 'closing puts the focus back on the wins button').toBeFocused();
});
