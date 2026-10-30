// uma.moe beside Next career, with uma.moe stood in for by two listings made from the umadump test file's first two
// veterans: one button opens the key popup; a rejected key says so; a saved key sends one request (parent at least
// n - 1 of n targets, followable trainers), as a header only, kept apart from the settings; the listings join the
// planner's pool (not the veteran dialog, not saved, not exported); the parent picker heads them "uma.moe"; a chosen one
// reads "uma.moe · her trainer", with Copy ID in her popup and stays while chosen; the popup's cards open the veteran
// popup; Include max borrows asks again without the follower limit; a reload asks nothing; Refresh asks again
import { readFileSync } from 'node:fs';
import { check, dialog, careerPage, repoUrl } from './helpers.mjs';

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
  const inh = { inheritance_id: 900000 + k, main_parent_id: rec.card_id };
  for (const [s, r] of [
    ['main', rec],
    ['left', parent(10)],
    ['right', parent(20)],
  ]) {
    const x = side(r);
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
  await ctx.route('https://uma.moe/api/v3/search**', (route) => {
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
    return route.fulfill({
      status: 200,
      headers: { ...cors, 'content-type': 'application/json' },
      body: JSON.stringify({ items, total: '2', page: 0, limit: 100, total_pages: 1 }),
    });
  });
  await importDump();
  await done();
  const lastJob = () => pg.evaluate(() => window.__careerJobs.filter((j) => j.cmd === 'career').at(-1));
  const label = () => pg.textContent('#um-open');
  check(
    'career uma.moe: one button beside Next career, its panel shut, nothing in Career setup, nothing sent',
    (await label()) === 'Borrow from uma.moe' &&
      (await pg.locator('.rbtitle #um-open').isVisible()) &&
      !(await pg.evaluate(() => document.getElementById('um-dlg').open)) &&
      (await pg.locator('.setlist #um-row').count()) === 0 &&
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
  await pg.waitForFunction(() => document.getElementById('um-open').textContent === 'uma.moe · 2 parents');
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
      url.searchParams.getAll('main_white_factors').length === job.targets.length - 1,
    url.search.slice(0, 120),
  );
  check(
    'career uma.moe: the key is kept under its own entry, never with the settings or the veterans',
    store.key === 'test-key' && !store.settings.includes('test-key') && !store.career.includes('test-key'),
  );
  check(
    "career uma.moe: the listings join the planner's pool as borrowed veterans, after yours, and are not saved",
    job.legacies.length === 14 &&
      job.legacies.slice(12).every((l) => l.borrowed) &&
      JSON.parse(store.career).global.vets.length === 12,
    String(job.legacies.length),
  );
  await vetsOpen();
  await pg.click('#v-tab-bor');
  check(
    'career uma.moe: the veteran dialog holds yours only (no uma.moe cards, not counted)',
    (await pg.locator('#vets .vet').count()) === 0 && (await pg.textContent('#v-tab-bor')) === 'Borrowed · 0',
  );
  await pg.click('#v-export');
  await dialog(pg, 'vx-dlg');
  const ex = JSON.parse(await pg.inputValue('#vx-text'));
  await pg.locator('#vx-dlg .dclose').click();
  await vetsShut();
  check(
    'career uma.moe: Export leaves the listings out',
    !ex.veterans.some((v) => v.src === 'umamoe') && !/Trainer [12]/.test(JSON.stringify(ex.veterans)),
  );
  // an uma.moe veteran as parent 1, from the picker's uma.moe heading
  const bj = (await lastJob()).legacies.length - 1;
  await node('node:p0');
  await inNode(/^(Choose|Change) parent$/).click();
  await dialog(pg, 'vpick-dlg');
  const heads = await pg.locator('#vp-list .vpgh').allTextContents();
  await pg.locator('#vp-list .vpt').nth(bj).click();
  await done();
  const tileText = (await pg.locator('#tree .pn.parent').first().innerText()).replace(/\s+/g, ' ');
  const boxButtons = await pg.locator('#tree .pn.parent').first().locator('button:not(.ptile)').count();
  await node('node:p0');
  const copyInPopup = await pg.locator('#node-dlg button', { hasText: 'Copy ID' }).count();
  await closeNode();
  check(
    'career uma.moe: the picker heads them uma.moe; a chosen one reads uma.moe · her trainer (no button in her box), Copy ID in her popup',
    heads.join() === 'uma.moe' && /uma\.moe · Trainer 2/.test(tileText) && boxButtons === 0 && copyInPopup === 1,
    tileText.slice(0, 80),
  );
  // the popup: the listings as the game's legacy list draws them (portrait, her three main sparks with stars, name,
  // followers), the next career's first and tagged; one opens her veteran popup, with Copy ID
  await pg.click('#um-open');
  const cards = await pg.$$eval('#um-list .umcell', (l) =>
    l.map((c) => ({
      next: c.classList.contains('umnext'),
      nm: c.querySelector('.nm')?.textContent,
      sub: c.querySelector('.sub')?.textContent,
      tags: [...c.querySelectorAll('.umsp b')].map((b) => b.textContent),
      stars: [...c.querySelectorAll('.umst')].every((st) => +st.style.getPropertyValue('--n') >= 1),
    })),
  );
  await pg.locator('#um-list .umcell').first().click();
  await dialog(pg, 'node-dlg');
  const popupCopy = await pg.locator('#node-dlg button', { hasText: 'Copy ID' }).count(),
    editable = await pg
      .locator('#node-dlg .facebtn, #node-dlg button.vwins, #node-dlg .vskadd, #node-dlg button.vskb')
      .count();
  await closeNode();
  check(
    "career uma.moe: the popup lists them as the game's legacy list does (portrait, three spark tags with stars, her trainer, followers), the next career's first and tagged; one opens her veteran popup, read-only, with Copy ID",
    cards.length === 2 &&
      cards[0].next &&
      cards.every(
        (c) =>
          /^Trainer [12]$/.test(c.nm) &&
          /^1[34] followers$/.test(c.sub) &&
          c.tags.length === 3 &&
          c.tags[2] === 'UNIQ' &&
          c.stars,
      ) &&
      popupCopy === 1 &&
      editable === 0,
    JSON.stringify(cards),
  );
  // full trainers: off by default (followable only); on, the search asks again without that limit
  await pg.check('#um-full');
  await pg.waitForFunction(
    () =>
      /parents/.test(document.getElementById('um-msg').textContent) && !document.getElementById('um-again').disabled,
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
      (await label()) === 'uma.moe · 2 parents' &&
      (await pg.evaluate(() => window.__careerJobs.filter((j) => j.cmd === 'career').length)) === 0 &&
      (await pg.evaluate(() => JSON.parse(localStorage.getItem('llp:career')).global.vets.length)) === 13,
  );
  await pg.click('#um-open');
  await pg.click('#um-again');
  await pg.waitForFunction(() => /^2 parents/.test(document.getElementById('um-msg').textContent));
  check('career uma.moe: Refresh asks uma.moe once more', requests.length === 4);
  await pg.setViewportSize({ width: 375, height: 812 });
  await pg.waitForTimeout(200);
  // (its list made long: the popup's body scrolls inside the window, which stays on screen)
  const fit = await pg.evaluate(() => {
    const list = document.getElementById('um-list');
    for (let i = 0; i < 60; i++) list.appendChild(list.firstElementChild.cloneNode(true));
    const d = document.getElementById('um-dlg').getBoundingClientRect(),
      body = document.querySelector('#um-dlg .dlgscroll');
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
  await ctx.close();
}
