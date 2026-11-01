// uma.moe beside Next career, with uma.moe stood in for by two listings made from the umadump test file's first two
// veterans: one button opens the key popup; a rejected key says so; a saved key sends one request (parent at least
// n - 1 of n targets, followable trainers), as a header only, kept apart from the settings; the listings join the
// planner's pool (not the veteran dialog, not saved, not exported); the parent picker heads them "uma.moe"; a chosen one
// reads "uma.moe · her trainer", with Copy ID in her popup and stays while chosen; the popup's cards open the veteran
// popup; Include max borrows asks again without the follower limit; a reload asks nothing; Refresh asks again; a late
// reply never lands over a newer answer; a tab on the other era leaves this era's veterans and settings as they are;
// Remove all on Guests leaves the listings
import { readFileSync } from 'node:fs';
import { check, dialog, careerPage, repoUrl, PAGE_URL } from './helpers.mjs';

// a white spark of a skill none of the test file's veterans carry: not a target (the listing's first gets it)
const FACTORS = JSON.parse(readFileSync(repoUrl('pkg/loopopt/data/global/career.json'), 'utf8')).factors;
// a umadump veteran as an uma.moe search result: herself at main_*, her parents (positions 10, 20) at left_* and right_*
export function listing(rec, k) {
  const parent = (position) => rec.succession_chara_array.find((x) => x.position_id === position);
  const side = (r) => {
    const ids = r.factor_info_array.map((f) => f.factor_id),
      kind = (lo, hi) => ids.find((id) => id >= lo && id < hi) ?? null;
    return {
      white: ids.filter((id) => id >= 1000000 && id < 10000000),
      blue: kind(100, 600),
      pink: kind(1100, 10000),
      green: kind(10000000, 1e9),
      wins: r.win_saddle_id_array || [],
    };
  };
  const inh = {
    inheritance_id: 900000 + k,
    main_parent_id: rec.card_id,
    parent_rank: rec.rank_score,
    win_count: 10 * k,
  };
  for (const [s, r] of [
    ['main', rec],
    ['left', parent(10)],
    ['right', parent(20)],
  ]) {
    const x = side(r);
    if (s === 'main' && k === 1) {
      const carried = new Set(x.white.map((id) => FACTORS[Math.floor(id / 100)]));
      x.white.push(+Object.keys(FACTORS).find((fg) => !carried.has(FACTORS[fg])) * 100 + 1);
    }
    if (s !== 'main') inh[`parent_${s}_id`] = r.card_id;
    Object.assign(inh, {
      [`${s}_white_factors`]: x.white,
      [`${s}_blue_factors`]: x.blue,
      [`${s}_pink_factors`]: x.pink,
      [`${s}_green_factors`]: x.green,
      [`${s}_win_saddles`]: x.wins,
    });
  }
  return {
    account_id: `10000000000${k}`,
    trainer_name: `Trainer ${k}`,
    follower_num: 12 + k,
    last_updated: new Date(Date.now() - 3 * 3600e3).toISOString(),
    inheritance: inh,
  };
}

export default async function careerumamoe(b) {
  const { ctx, pg, done, planned, vetsOpen, vetsShut, node, inNode, closeNode, importDump } = await careerPage(b);
  const recs = JSON.parse(readFileSync(repoUrl('val/umadump/U_trained_chara_data.json'), 'utf8'));
  const items = [listing(recs[0], 1), listing(recs[1], 2)];
  const requests = [];
  // `late`: the next request without the follower limit is answered 1.5 s late with these items, then `late.done()`
  let late = null;
  await ctx.route('https://uma.moe/api/v3/search**', async (route) => {
    const req = route.request(),
      cors = {
        'access-control-allow-origin': '*',
        'access-control-allow-headers': 'x-api-key',
        'access-control-allow-methods': 'GET',
      };
    if (req.method() === 'OPTIONS') return route.fulfill({ status: 204, headers: cors });
    const key = req.headers()['x-api-key'];
    requests.push({ url: req.url(), key, cookie: req.headers().cookie });
    if (key !== 'test-key') return route.fulfill({ status: 401, headers: cors, body: '{}' });
    if (late && !new URL(req.url()).searchParams.has('max_follower_num')) {
      const l = late;
      late = null;
      await new Promise((r) => setTimeout(r, 1500));
      await route.fulfill({
        status: 200,
        headers: { ...cors, 'content-type': 'application/json' },
        body: JSON.stringify({ items: l.items, total: String(l.items.length), page: 0, limit: 100, total_pages: 1 }),
      });
      return l.done();
    }
    return route.fulfill({
      status: 200,
      headers: { ...cors, 'content-type': 'application/json' },
      body: JSON.stringify({ items, total: '2', page: 0, limit: 100, total_pages: 1 }),
    });
  });
  await done();
  const shutBefore = await pg.locator('#um-open').isHidden(); // (no veteran of yours yet)
  await importDump();
  await done();
  const lastJob = () => pg.evaluate(() => window.__careerJobs.filter((j) => j.cmd === 'career').at(-1));
  const label = () => pg.textContent('#um-open');
  check(
    'career uma.moe: one button beside Next career once you have a veteran (none before), its panel shut, nothing in Career setup, nothing sent',
    shutBefore &&
      (await label()) === 'Borrow from uma.moe' &&
      (await pg.locator('#um-root #um-open').isVisible()) &&
      !(await pg.evaluate(() => document.getElementById('um-dlg').open)) &&
      (await pg.locator('#set-body #um-row').count()) === 0 &&
      requests.length === 0,
  );
  await pg.click('#um-open');
  check('career uma.moe: the button opens the key field', await pg.locator('#um-key').isVisible());
  await pg.fill('#um-key', 'bad-key');
  await pg.click('#um-save');
  await pg.waitForFunction(() => /Key rejected/.test(document.getElementById('um-err').textContent));
  check(
    'career uma.moe: a key uma.moe refuses says so, is forgotten, and the key field comes back for another',
    requests.length === 1 &&
      (await label()) === 'uma.moe · key rejected' &&
      (await pg.locator('#um-key').isVisible()) &&
      !(await pg.evaluate(() => localStorage.getItem('llp:umamoe-key'))),
  );
  await pg.fill('#um-key', 'test-key');
  await pg.press('#um-key', 'Enter');
  await pg.waitForFunction(() => document.getElementById('um-open').textContent === 'uma.moe · 2 veterans');
  await pg.keyboard.press('Escape');
  check('career uma.moe: Escape shuts the popup', !(await pg.evaluate(() => document.getElementById('um-dlg').open)));
  await done();
  await planned();
  const job = await lastJob(),
    url = new URL(requests[1].url);
  const store = await pg.evaluate(() => ({
    key: localStorage.getItem('llp:umamoe-key'),
    settings: localStorage.getItem('llp:settings') || '',
    career: localStorage.getItem('llp:career') || '',
  }));
  check(
    'career uma.moe: one request, the key as its X-API-Key header only (no cookie, not in the URL), parents with all but one target on herself, followable trainers',
    requests.length === 2 &&
      requests[1].key === 'test-key' &&
      !requests[1].cookie &&
      !url.search.includes('test-key') &&
      url.searchParams.get('max_follower_num') === '999' &&
      url.searchParams.getAll('main_white_factors').length === job.skills.targets.length - 1,
    url.search.slice(0, 120),
  );
  check(
    'career uma.moe: the key is kept under its own entry, never with the settings or the veterans',
    store.key === 'test-key' && !store.settings.includes('test-key') && !store.career.includes('test-key'),
  );
  check(
    "career uma.moe: the listings join the planner's pool as borrowed veterans, after yours, and are not saved",
    job.career.legacies.length === 14 &&
      job.career.legacies.slice(12).every((l) => l.borrowed) &&
      JSON.parse(store.career).global.vets.length === 12,
    String(job.career.legacies.length),
  );
  await vetsOpen();
  await pg.click('#v-tab-bor');
  check(
    "career uma.moe: Veteran Umamusume's Guests tab holds the listings under an uma.moe heading, counted",
    (await pg.locator('#vets [data-vet]').count()) === 2 &&
      (await pg.textContent('#vets h4')) === 'uma.moe' &&
      (await pg.textContent('#v-tab-bor')) === 'Guests · 2',
  );
  // the pane's Copy ID is the veteran's on show: another listing chosen after a copy reads Copy ID, never the first
  // one's "Copied" or (a failed copy) her trainer's ID
  await pg.locator('#vets [data-vet]').nth(0).click();
  await pg.locator('#v-pane button', { hasText: 'Copy ID' }).click();
  // (the copy is a promise: its answer is drawn after it)
  // (Copy ID, then what the copy said: Copied, or her trainer's ID)
  const copy = pg.locator('#v-pane [data-who]').getByRole('button', { name: /^(Copy ID|Copied|\d+)$/ });
  await copy.filter({ hasNotText: 'Copy ID' }).waitFor();
  const copied = await copy.textContent();
  await pg.locator('#vets [data-vet]').nth(1).click();
  const next = await copy.textContent();
  check(
    'career uma.moe: Copy ID in the pane, then another listing: hers reads Copy ID (no state carried to her)',
    copied !== 'Copy ID' && next === 'Copy ID',
    `${copied} -> ${next}`,
  );
  await pg.click('#v-tab-own');
  await vetsShut();
  await pg.click('#v-export');
  await dialog(pg, 'vx-dlg');
  const ex = JSON.parse(await pg.inputValue('#vx-text'));
  await pg.locator('#vx-dlg').getByRole('button', { name: 'Close', exact: true }).click();
  check(
    'career uma.moe: Export leaves the listings out',
    !ex.veterans.some((v) => v.src === 'umamoe') && !/Trainer [12]/.test(JSON.stringify(ex.veterans)),
  );
  // an uma.moe veteran as parent 1, from the picker's uma.moe heading
  const bj = (await lastJob()).legacies.length - 1;
  await node('node:p0');
  await inNode(/^(Choose|Change) parent$/).click();
  await dialog(pg, 'vpick-dlg');
  const heads = await pg.locator('#vp-list h4').allTextContents();
  await pg.locator('#vp-list button').nth(bj).click();
  await done();
  const tileText = (await pg.locator('#tree [data-node="parent"]').first().innerText()).replace(/\s+/g, ' ');
  const boxButtons = await pg.locator('#tree [data-node="parent"]').first().locator('button:not([data-k])').count();
  await node('node:p0');
  const copyInPopup = await pg.locator('#node-dlg button', { hasText: 'Copy ID' }).count();
  await closeNode();
  check(
    'career uma.moe: the picker heads them uma.moe; a chosen one reads uma.moe · her trainer (no button in her box), Copy ID in her popup',
    heads.join() === 'uma.moe' && /uma\.moe · Trainer 2/.test(tileText) && boxButtons === 0 && copyInPopup === 1,
    tileText.slice(0, 80),
  );
  // the popup: the listings as the game's legacy list draws them (portrait, her three main sparks with stars, her
  // trainer), in uma.moe's order until another is chosen; one opens her veteran popup, with followers and Copy ID
  await pg.click('#um-open');
  const cards = await pg.$$eval('#um-list [data-vet]', (l) =>
    l.map((c) => ({
      nm: c.querySelector('[data-part="name"]')?.textContent,
      tags: [...c.querySelectorAll('img[data-part="tag"]')].map((t) => t.alt),
      tagArt: [...c.querySelectorAll('img[data-part="tag"]')].every((t) =>
        /ftag_(blue_\d|pink_\d+|green)\./.test(t.src),
      ),
      stars: [...c.querySelectorAll('img[data-part="stars"]')].every((st) =>
        /fstar_(blue|pink|green)_[123]\./.test(st.src),
      ),
      rank: /vrank_\d+\./.test(c.querySelector('img[data-part="rank"]')?.src || ''),
    })),
  );
  const names = () => pg.$$eval('#um-list [data-vet] [data-part="name"]', (l) => l.map((x) => x.textContent).join());
  const matchOrder = await names();
  await pg.click('#um-sort');
  await pg.click('#um-sort-menu button[data-k="wins"]');
  const winsOrder = await names();
  await pg.click('#um-sort-menu button[data-k="match"]'); // (the menu stays open, as the umas' and cards' do)
  await pg.keyboard.press('Escape');
  await pg.locator('#um-list [data-vet]').first().click();
  await dialog(pg, 'node-dlg');
  const tag = await pg.locator('#node-dlg [data-src="umamoe"]').textContent(),
    sparks = await pg.$$eval('#node-dlg .udrow.me .vsk:not(.pink)', (l) => l.map((c) => c.classList.contains('off')));
  const popupCopy = await pg.locator('#node-dlg button', { hasText: 'Copy ID' }).count(),
    editable = await pg
      .locator('#node-dlg [data-face], #node-dlg button[data-k$=":wins"], #node-dlg .vskadd, #node-dlg button.vskb')
      .count();
  await closeNode();
  check(
    "career uma.moe: the popup lists them as the game's legacy list does (portrait, three spark tags with stars, her trainer), her spark tags the game's own images, in uma.moe's order or by wins; one opens her veteran popup, read-only, with her trainer's followers and Copy ID, her one non-target spark last and grey",
    cards.length === 2 &&
      cards.every(
        (c) =>
          /^Trainer [12]$/.test(c.nm) && c.tags.length === 3 && c.tags[2] === 'UNIQ' && c.tagArt && c.stars && c.rank,
      ) &&
      matchOrder === 'Trainer 1,Trainer 2' &&
      winsOrder === 'Trainer 2,Trainer 1' &&
      /· 13 followers$/.test(tag) &&
      sparks.length > 1 &&
      sparks.at(-1) &&
      sparks.filter(Boolean).length === 1 &&
      popupCopy === 1 &&
      editable === 0,
    JSON.stringify({ cards, matchOrder, winsOrder, tag, sparks }),
  );
  // full trainers: off by default (followable only); on, the search asks again without that limit
  await pg.check('#um-full');
  await pg.waitForFunction(
    () =>
      /veterans found/.test(document.getElementById('um-msg').textContent) &&
      !document.getElementById('um-again').disabled,
  );
  const fullUrl = new URL(requests.at(-1).url);
  check(
    'career uma.moe: Include max borrows asks again without the follower limit',
    requests.length === 3 && !fullUrl.searchParams.has('max_follower_num'),
    fullUrl.search.slice(0, 80),
  );
  await pg.uncheck('#um-full');
  await pg.keyboard.press('Escape');
  await done();
  await planned(); // (its answer is the one a reload draws)
  await pg.reload();
  await done();
  check(
    'career uma.moe: after a reload the listings come back with no request (the answer drawn at once, no solver job), the chosen one saved and not doubled',
    requests.length === 3 &&
      (await label()) === 'uma.moe · 2 veterans' &&
      (await pg.evaluate(() => window.__careerJobs.filter((j) => j.cmd === 'career').length)) === 0 &&
      (await pg.evaluate(() => JSON.parse(localStorage.getItem('llp:career')).global.vets.length)) === 13,
  );
  await pg.click('#um-open');
  await pg.click('#um-again');
  await pg.waitForFunction(() => /^2 veterans found/.test(document.getElementById('um-msg').textContent));
  check('career uma.moe: Refresh asks uma.moe once more', requests.length === 4);
  // a reply still out when the target set goes back to the one on show is dropped: Include max borrows on (no answer
  // kept for it: its request goes, answered late with none) and off at once (the listings on show are its answer)
  await pg.evaluate(() => {
    const c = JSON.parse(localStorage.getItem('llp:umamoe'));
    for (const k of Object.keys(c)) if (!k.includes('max_follower_num')) delete c[k];
    localStorage.setItem('llp:umamoe', JSON.stringify(c));
  });
  const lateDone = new Promise((done) => {
    late = { items: [], done };
  });
  await pg.check('#um-full');
  await pg.uncheck('#um-full');
  await lateDone;
  // (the late answer is kept as it arrives, before the page decides whether it is stale)
  await pg.waitForFunction(() =>
    Object.keys(JSON.parse(localStorage.getItem('llp:umamoe'))).some((k) => !k.includes('max_follower_num')),
  );
  await pg.evaluate(() => new Promise((r) => setTimeout(r)));
  // (the listings in the pool, read on Guests; the button's label back to the answer on show, not "searching…")
  await pg.keyboard.press('Escape');
  await vetsOpen();
  const guests = (await pg.textContent('#v-tab-bor')).trim();
  await vetsShut();
  const shown = await label();
  await pg.click('#um-open');
  check(
    'career uma.moe: a late reply for a target set left before it came does not replace the listings on show, nor its label',
    guests === 'Guests · 2' && !/searching/.test(shown),
    `${guests} | ${shown}`,
  );

  await pg.setViewportSize({ width: 375, height: 812 });
  await pg.waitForTimeout(200);
  // (its list made long: the popup's body scrolls inside the window, which stays on screen)
  const fit = await pg.evaluate(() => {
    const list = document.getElementById('um-list');
    for (let i = 0; i < 60; i++) list.appendChild(list.firstElementChild.cloneNode(true));
    const d = document.getElementById('um-dlg').getBoundingClientRect(),
      body = document.getElementById('um-body');
    return {
      left: d.left,
      right: d.right,
      bottom: d.bottom,
      scrolls: body.scrollHeight > body.clientHeight && getComputedStyle(body).overflowY === 'auto',
      page: document.documentElement.scrollWidth,
    };
  });
  check(
    'career uma.moe: at 375 px the popup stays on screen and its long body scrolls inside it, no sideways scroll',
    fit.left >= 0 && fit.right <= 375 && fit.bottom <= 812 && fit.scrolls && fit.page <= 375,
    JSON.stringify(fit),
  );
  // a parent place naming a veteran not here (an uma.moe one an earlier build never saved): she goes back to the
  // planner and the page plans (it stopped before its first search)
  await pg.setViewportSize({ width: 1440, height: 1000 });
  await pg.evaluate(() => {
    const o = JSON.parse(localStorage.getItem('llp:career'));
    o.global.plan = { trainee: null, parents: ['u999999', null] };
    localStorage.setItem('llp:career', JSON.stringify(o));
  });
  await pg.reload();
  await done();
  check(
    'career uma.moe: a parent place naming a veteran not here goes back to the planner, and the page still plans',
    (await pg.evaluate(() => document.getElementById('rhead').dataset.kind)) !== 'loading' &&
      !!(await pg.textContent('#tree [data-node="trainee"] [data-nm]')) &&
      !(pg.errors || []).some((e) => /TypeError|ReferenceError/.test(e)), // (the bad key's 401 is logged by the browser)
    `${(await pg.textContent('#run-note')).slice(0, 40)} | ${await pg.evaluate(() => document.getElementById('rhead').dataset.kind)} | ${await pg.textContent('#tree [data-node="trainee"] [data-nm]')} | ${(pg.errors || []).join(' / ').slice(0, 200)}`,
  );
  // the Grand Masters era borrows from uma.moe too, asking only for the targets' sparks Global has; a tab still on the
  // Global data (opened before the switch) saves its own era's part only
  const globalTab = await ctx.newPage();
  await globalTab.goto(PAGE_URL);
  await globalTab.waitForFunction(() => {
    const n = document.getElementById('run-note');
    return !n.querySelector('[data-spinner]') && n.textContent.length > 0;
  });
  const before = requests.length;
  await Promise.all([pg.waitForEvent('load'), pg.click('#era-gm')]);
  await done();
  await importDump();
  await done();
  await pg.waitForFunction(() => /^uma\.moe · \d+ veterans$/.test(document.getElementById('um-open').textContent));
  const asked = requests.slice(before).map((r) => new URL(r.url).searchParams.get('optional_white_sparks') || '');
  check(
    'career uma.moe: the Grand Masters era borrows from uma.moe too, asking only for sparks Global has',
    (await pg.locator('#um-root #um-open').isVisible()) &&
      asked.length > 0 &&
      asked.every((q) => q.split(',').every((f) => FACTORS[Math.floor(+f / 10)] !== undefined)),
    `${await label()} | ${asked.join(' / ').slice(0, 120)}`,
  );
  // the Global tab: a star on one of its veterans and a rank, each saved
  const stored = () =>
    pg.evaluate(() => ({
      gm: JSON.parse(localStorage.getItem('llp:career')).gm?.vets?.length ?? 0,
      era: JSON.parse(localStorage.getItem('llp:settings')).era,
    }));
  const tabsBefore = await stored();
  await globalTab.click('#v-toggle');
  await globalTab.locator('#v-pane .udrow.me .vsk:not(.pink) .vskb').first().click();
  await globalTab.evaluate(() => document.querySelector('#rank button[data-v="ss_and_up"]').click());
  await globalTab.evaluate(() => new Promise((r) => requestAnimationFrame(() => setTimeout(r))));
  await globalTab.close();
  const tabsAfter = await stored();
  check(
    "career uma.moe: a tab on the other era saving its veterans and settings leaves this era's veterans and the era chosen",
    tabsBefore.gm > 0 && tabsBefore.era === 'gm' && JSON.stringify(tabsAfter) === JSON.stringify(tabsBefore),
    `${JSON.stringify(tabsBefore)} -> ${JSON.stringify(tabsAfter)}`,
  );
  // Remove all on Guests takes the borrow list only: uma.moe's listings stay (they are its answer for the targets)
  await vetsOpen();
  await pg.click('#v-tab-bor');
  const listed = await pg.locator('#vets [data-vet]').count();
  await pg.click('#v-paste');
  await dialog(pg, 'paste-dlg');
  await pg.fill(
    '#paste-text',
    JSON.stringify([{ ...recs[2], viewer_id: 900000000003, owner_viewer_id: 900000000003 }]),
  );
  await pg.click('#paste-load');
  await pg.click('#v-tab-bor');
  let confirmText = '';
  pg.once('dialog', (d) => {
    confirmText = d.message();
    d.accept();
  });
  await pg.click('#v-clear');
  const kept = await pg.locator('#vets [data-vet]').count();
  check(
    'career uma.moe: Remove all on Guests removes the borrow list, not the uma.moe listings',
    listed > 0 && confirmText === 'Remove all 1 borrowed veteran?' && kept === listed,
    `${listed} listings | ${confirmText} | ${kept} left`,
  );
  await ctx.close();
}
