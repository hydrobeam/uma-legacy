// Find a borrow on uma.moe (Borrowed tab), with uma.moe stood in for by two listings made from the umadump test file's
// first two veterans: the key is asked for, sent only as a header and kept apart from the settings and the veterans;
// one press is one request (main parent at least n - 1 of n targets), ranked by the planner; Add puts her on the borrow
// list with her lender, whom Export leaves out; the 24 h cache answers again after a reload; a rejected key says so
import { readFileSync } from 'node:fs';
import { check, dialog, careerPage, repoUrl } from './helpers.mjs';

// a umadump veteran as an uma.moe search result: herself at main_*, her parents (positions 10, 20) at left_* and right_*
function listing(rec, k) {
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
  const { ctx, pg, done, planned, vetsOpen, vetsShut, importDump } = await careerPage(b);
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
  await planned();
  const openSearch = async () => {
    await vetsOpen();
    await pg.click('#v-tab-bor');
    await pg.click('#um-open');
    await dialog(pg, 'um-dlg');
  };
  const ranked = () => pg.waitForFunction(() => /ranked/.test(document.getElementById('um-msg').textContent));
  await openSearch();
  await pg.click('#um-go');
  check(
    'career uma.moe: Search without a key asks for one and sends nothing',
    requests.length === 0 && /key first/.test(await pg.textContent('#um-msg')),
  );
  await pg.fill('#um-key', 'bad-key');
  await pg.click('#um-go');
  await pg.waitForFunction(() => /rejected/.test(document.getElementById('um-msg').textContent));
  check('career uma.moe: a key uma.moe refuses says so', requests.length === 1);
  await pg.fill('#um-key', 'test-key');
  await pg.click('#um-go');
  await ranked();
  const url = new URL(requests[1].url),
    n = await pg.evaluate(() => window.__careerJobs.at(-1).targets.length);
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
      url.searchParams.getAll('main_white_factors').length === n - 1,
    url.search.slice(0, 120),
  );
  check(
    'career uma.moe: the key is kept under its own entry, never with the settings or the veterans',
    store.key === 'test-key' && !store.settings.includes('test-key') && !store.career.includes('test-key'),
  );
  const rows = pg.locator('#um-res .umrow');
  const texts = await rows.allInnerTexts();
  check(
    "career uma.moe: every listing ranked by the planner: her best next career's odds, her targets, the trainer",
    texts.length === 2 && texts.every((t) => /Best next career: .+%/.test(t) && /Trainer [12]/.test(t)),
    texts.map((t) => t.split('\n').slice(0, 3).join(' | ')).join(' || '),
  );
  check(
    "career uma.moe: the top rows priced in today's plan (careers to a fully looped parent)",
    texts.every((t) => /In the plan: /.test(t)),
    texts.map((t) => t.match(/In the plan: [^\n]*/)?.[0]).join(' || '),
  );
  await pg.setViewportSize({ width: 375, height: 812 });
  check(
    'career uma.moe: at 375 px the dialog fits its width (no sideways scroll)',
    await pg.evaluate(() => {
      const d = document.querySelector('#um-dlg .dlg');
      return (
        d.scrollWidth <= d.clientWidth + 1 && document.getElementById('um-dlg').getBoundingClientRect().right <= 376
      );
    }),
  );
  await pg.setViewportSize({ width: 1440, height: 1000 });
  const before = await pg.evaluate(() => JSON.parse(localStorage.getItem('llp:career')).global.vets.length);
  await rows.first().getByRole('button', { name: 'Add to borrow list' }).click();
  await pg.locator('#um-dlg .dclose').click();
  await vetsShut();
  await done();
  const added = await pg.evaluate(() => JSON.parse(localStorage.getItem('llp:career')).global.vets.at(-1));
  check(
    'career uma.moe: Add puts her on the borrow list, from uma.moe, with her lender',
    added.src === 'umamoe' &&
      added.borrowed === true &&
      /^u9000/.test(added.id) &&
      /^Trainer/.test(added.lender?.name) &&
      (await pg.evaluate(() => JSON.parse(localStorage.getItem('llp:career')).global.vets.length)) === before + 1,
  );
  await vetsOpen();
  await pg.click('#v-export');
  await dialog(pg, 'vx-dlg');
  const ex = JSON.parse(await pg.inputValue('#vx-text'));
  await pg.locator('#vx-dlg .dclose').click();
  await vetsShut();
  check(
    "career uma.moe: Export leaves out her lender's name and ID",
    ex.veterans.some((v) => v.src === 'umamoe') && !JSON.stringify(ex).includes('Trainer 1'),
  );
  // a reload: the same search answers from the 24 h cache
  await pg.reload();
  await done();
  await openSearch();
  await pg.click('#um-go');
  await ranked();
  check(
    'career uma.moe: after a reload the same search answers from the cache (no request), the added one marked',
    requests.length === 2 &&
      /cache/.test(await pg.textContent('#um-msg')) &&
      (await pg.locator('#um-res .umrow', { hasText: 'In your borrow list' }).count()) === 1,
  );
  await ctx.close();
}
