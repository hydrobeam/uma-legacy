// uma.moe on the Borrowed tab, with uma.moe stood in for by two listings made from the umadump test file's first two
// veterans: a rejected key says so; a saved key sends one request (parent at least n - 1 of n targets, followable
// trainers), as a header only, kept apart from the settings and the veterans; the listings join the planner's pool as
// borrowed veterans (not saved unless kept), named by trainer on their cards (one size), the parent's tile and popup;
// Keep saves one (Export without her trainer); a reload asks nothing (the answer is kept 24 h); Refresh asks again
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
  const { ctx, pg, done, planned, vetsOpen, vetsShut, node, inNode, importDump } = await careerPage(b);
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
  const status = () => pg.textContent('#um-msg');
  const borrowedTab = async () => {
    await vetsOpen();
    await pg.click('#v-tab-bor');
  };
  check('career uma.moe: Career setup has no uma.moe row', (await pg.locator('.setlist #um-row').count()) === 0);
  await borrowedTab();
  check(
    'career uma.moe: without a key the Borrowed tab offers the key field and sends nothing',
    (await pg.locator('#um-key').isVisible()) && requests.length === 0,
  );
  await pg.fill('#um-key', 'bad-key');
  await pg.click('#um-save');
  await pg.waitForFunction(() => /Key rejected/.test(document.getElementById('um-msg').textContent));
  check(
    'career uma.moe: a key uma.moe refuses says so; Forget key waits behind Key',
    requests.length === 1 && !(await pg.locator('#um-forget').isVisible()),
  );
  await pg.click('.umkey summary');
  await pg.click('#um-forget');
  await pg.fill('#um-key', 'test-key');
  await pg.press('#um-key', 'Enter');
  await pg.waitForFunction(() => /^2 to borrow · 1 h$/.test(document.getElementById('um-msg').textContent));
  await vetsShut();
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
  await borrowedTab();
  const cards = pg.locator('#vets .vet');
  const texts = await cards.allInnerTexts();
  check(
    'career uma.moe: the Borrowed tab shows each with her trainer and Keep, no Remove (the next search replaces her)',
    texts.length === 2 &&
      texts.every((t) => /uma\.moe · Trainer [12] · 3 h/.test(t) && /Keep/.test(t)) &&
      (await cards.locator('.vdel').count()) === 0,
    texts.map((t) => t.replace(/\s+/g, ' ').slice(0, 90)).join(' || '),
  );
  await cards.first().getByRole('button', { name: 'Keep' }).click();
  await vetsShut();
  await done();
  const kept = await pg.evaluate(() => JSON.parse(localStorage.getItem('llp:career')).global.vets);
  check(
    'career uma.moe: Keep saves her, with her trainer',
    kept.length === 13 && kept[12].src === 'umamoe' && kept[12].kept && /^Trainer/.test(kept[12].lender?.name),
  );
  await vetsOpen();
  await pg.click('#v-export');
  await dialog(pg, 'vx-dlg');
  const ex = JSON.parse(await pg.inputValue('#vx-text'));
  await pg.locator('#vx-dlg .dclose').click();
  await vetsShut();
  check(
    'career uma.moe: Export takes the kept one without her trainer, and not the other listing',
    ex.veterans.filter((v) => v.src === 'umamoe').length === 1 && !/Trainer [12]/.test(JSON.stringify(ex)),
  );
  // an uma.moe veteran as parent 1: the tree's tile names her trainer, with Copy ID
  const bj = (await lastJob()).legacies.length - 1;
  await node('node:p0');
  await inNode(/^(Choose|Change) parent$/).click();
  await dialog(pg, 'vpick-dlg');
  await pg.locator('#vp-list .vpt').nth(bj).click();
  await done();
  const tileText = (await pg.locator('#tree [data-k="node:p0"]').innerText()).replace(/\s+/g, ' ');
  check(
    "career uma.moe: an uma.moe parent's tile names her trainer, with Copy ID beside it",
    /uma\.moe · Trainer/.test(tileText) && (await pg.locator('#tree .pcopy').count()) === 1,
    tileText.slice(0, 90),
  );
  // a reload: the listings come back from the kept answer (no request); the kept one is not doubled
  await pg.reload();
  await done();
  check(
    'career uma.moe: after a reload the listings come back with no request, the kept one once',
    requests.length === 2 && (await lastJob()).legacies.length === 14 && /to borrow/.test(await status()),
  );
  await borrowedTab();
  await pg.click('#um-again');
  await pg.waitForFunction(() => /^2 to borrow/.test(document.getElementById('um-msg').textContent));
  check('career uma.moe: Refresh asks uma.moe once more', requests.length === 3);
  const heights = () =>
    pg.evaluate(() =>
      [...document.querySelectorAll('#vets .vet')].map((c) => Math.round(c.getBoundingClientRect().height)),
    );
  const wide = await heights();
  await pg.setViewportSize({ width: 375, height: 812 });
  await pg.waitForTimeout(200);
  const phone = await pg.evaluate(() => {
    const d = document.querySelector('#set-vets .dlg');
    return d.scrollWidth <= d.clientWidth + 1;
  });
  check(
    'career uma.moe: the Borrowed tab fits a 375 px phone, and its cards are one size',
    phone && wide.length === 2 && new Set(wide).size === 1,
    JSON.stringify({ phone, wide }),
  );
  await ctx.close();
}
