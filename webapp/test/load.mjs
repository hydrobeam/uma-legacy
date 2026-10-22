import { check, ART, artCheck, open, settle, jobSent, lastJob, dialog, note, scen, pickScen } from './helpers.mjs';

export default async function load(b) {
  const pg = await open(b);
  check('first search runs by itself, and its status is just the count: checked N decks / M loops in T s (a caveat only when the exact check ran out of time)', /^Checked [\d,]+ decks \/ [\d,]+ loops in [\d.]+ s( · .+)?$/.test(await note(pg)), await note(pg));
  // the result's wording: the headline odds, each member's own odds as trainee
  const head = await pg.evaluate(() => { const l = document.querySelector('#results .loop'); return { odds: l.querySelector('.odds').innerText, sub: !!l.querySelector('.sub'), rank: !!l.querySelector('.rank'),
    own: [...l.querySelectorAll('.mem .own')].map(e => e.innerText), font: getComputedStyle(l.querySelector('.odds')).fontFamily }; });
  check('headline: "1 in 107 runs to land all 8" alone (no 90% line), in the game font; no rank with one result',
    /^1 in 107 runs to land all 8$/.test(head.odds) && !head.sub && !head.rank && /Zen Maru/.test(head.font), JSON.stringify(head));
  // Odds opens the career planner's table for the loop: every count, 8/8 the headline's (1 in 107), 7/8 and 8/8 together
  // the solver's 7-or-better (1 in 12.6), focus back on the button when it closes
  await pg.click('#results .loop .lacts button:has-text("Odds")'); await dialog(pg, 'odds-dlg');
  const lrows = await pg.$$eval('#odds-dlg .oddtab tbody tr', l => l.map(r => [...r.children].map(c => c.textContent)));
  const pv = (s) => parseFloat(s) / 100;
  await pg.keyboard.press('Escape');
  check('loop odds: a popup with every count 8/8 .. 0/8, all 8 at the headline odds, 7 or better at 1 in 12.6, focus back on Odds',
    lrows.length === 9 && lrows[0][0] === '8/8' && lrows[8][0] === '0/8' && Math.abs(1 / pv(lrows[0][1]) - 107) < 2 && Math.abs(1 / (pv(lrows[0][1]) + pv(lrows[1][1])) - 12.6) < 0.3
    && await pg.evaluate(() => document.activeElement?.textContent === 'Odds'), JSON.stringify(lrows.slice(0, 3)));
  // the default result is a three-uma rotation: three members in three columns, a deck for each
  check('members: three (a rotation), "as trainee: 1 in …", in three columns', head.own.length === 3 && head.own.every(x => /^as trainee: 1 in [\d,.]+$/.test(x))
    && await pg.evaluate(() => { const m = document.querySelector('#results .loop .members'); return m.classList.contains('n3') && getComputedStyle(m).gridTemplateColumns.split(' ').length === 3; }), head.own.join(' | '));
  // every member names one outfit, never "any outfit" (her popup says when any would do)
  const ofs = await pg.$$eval('#results .loop .mem', l => l.map(m => [m.querySelector('.of').textContent, m.dataset.outfitNote || '']));
  check('members: each names one outfit (never "any outfit"; her popup carries any note)', ofs.length > 0 && ofs.every(([o]) => o && !/any outfit/i.test(o))
    && !/any outfit/i.test(await pg.textContent('#results')), JSON.stringify(ofs));
  // a pal whose credited event choice needs a priority says so, derived from the event data (Anshinzawa SSR's finale)
  const anshin = await pg.evaluate(() => [...document.querySelectorAll('#results .loop .dc[data-cid="30080"] i')].map(i => i.textContent));
  check('deck: Sasami Anshinzawa SSR says "prioritise Risky Business"', anshin.includes('prioritise Risky Business'), JSON.stringify(anshin));
  // the deck's odds per target, each with its skill icon
  const dd = await pg.evaluate(() => { const l = document.querySelector('#results .loop .dodds'); return { n: l.querySelectorAll('.dsk').length, icons: l.querySelectorAll('.dsk img.sic').length }; });
  artCheck('deck odds: a skill icon on every entry', dd.n > 0 && dd.icons === dd.n, JSON.stringify(dd));
  // the odds sit in a frame the (i) button opens; a swap row names its uma by her character icon
  const oi = pg.locator('#results .loop .deck .oinfo').first();
  check('deck odds start closed', await pg.evaluate(() => document.querySelector('#results .loop .dodds').hidden));
  await oi.click();
  check('(i) opens the deck odds (aria-expanded, names and percentages)', (await oi.getAttribute('aria-expanded')) === 'true' && await pg.evaluate(() => { const o = document.querySelector('#results .loop .dodds'); return !o.hidden && o.querySelectorAll('.dnm').length > 0 && /\d+%/.test(o.innerText); }));
  await oi.click();
  check('(i) closes them again', await pg.evaluate(() => document.querySelector('#results .loop .dodds').hidden));
  const sw = await pg.evaluate(() => [...document.querySelectorAll('#results .loop .deck.swap .swho')].map(w => ({ av: w.querySelectorAll('.av').length, label: w.querySelector('.vh').textContent, tips: [...w.querySelectorAll('.facebtn')].map(a => a.title), shown: [...w.childNodes].filter(n => !(n.classList && (n.classList.contains('vh') || n.classList.contains('facebtn')))).map(n => n.textContent).join('').trim() })));   // .vh: screen-reader text
  const swd = await pg.evaluate(() => [...document.querySelectorAll('#results .loop .deck.swap')].map(d => [...d.querySelectorAll('.swd1')].map(x => x.textContent.trim())));
  check('each swap row says what it changes ("Skill a% → b%")', swd.length > 0 && swd.every(r => r.length > 0 && r.every(t => /\d+% → \d+%$/.test(t))), JSON.stringify(swd));
  // a target on the swapping uma's own skill list is no change for her (her deck drops its hints because she has it):
  // not listed (the default's Agnes Digital [Full-Color Fangirling] has Superstan, so no "Uma Stan 100% → 0%")
  const owned = await pg.evaluate(() => { const skl = self.__LLP_DATA['data-skills'], wOf = new Map(skl.filter(s => s[2]).map(s => [s[2], s[1]]));
    return [...document.querySelectorAll('#results .loop .deck.swap')].map(d => { const ids = [...d.querySelectorAll('.swho .facebtn')].map(f => f.dataset.uma);
      const per = ids.map(id => [...document.querySelectorAll(`#results .loop .mem[data-uma="${id}"] .sk .spark`)].flatMap(s => s.classList.contains('g') ? [s.textContent, wOf.get(s.textContent)] : [s.textContent]));
      return { mine: per.length ? per[0].filter(n => per.every(p => p.includes(n))) : [], swd: [...d.querySelectorAll('.swd1')].map(x => x.textContent.trim()) }; }); });
  check('swap rows leave out the targets their umas have on their own skill list', owned.some(r => r.mine.length) && owned.every(r => r.swd.every(t => !r.mine.some(n => t.startsWith(n + ' ')))), JSON.stringify(owned));
  const more = pg.locator('#results .loop .swd.more').first();
  if (await more.count()) { const n0 = await more.locator('.swd1:visible').count(); await more.click();
    const unfolded = (await more.locator('.swd1:visible').count()) > n0 && (await more.getAttribute('aria-expanded')) === 'true';
    let folded = false; if (unfolded) { await more.click(); folded = (await more.locator('.swd1:visible').count()) === n0; }
    check('clicking a swap\'s changes shows them all, again folds them', unfolded && folded); }
  check('a label marks where the swap rows start', await pg.evaluate(() => { const h = document.querySelector('#results .loop .swhead'), s = document.querySelector('#results .loop .deck.swap'); return !!h && /swaps/i.test(h.textContent) && !!(h.compareDocumentPosition(s) & Node.DOCUMENT_POSITION_FOLLOWING); }));
  // each swap row: the card that goes greyed ➜ the card that comes in, every card's art on one line, the arrow at its middle
  const rows = await pg.evaluate(() => [...document.querySelectorAll('#results .loop .deck.swap')].map(d => { const art = [...d.querySelectorAll('.swc .dc .cav')].map(x => x.getBoundingClientRect()), a = d.querySelector('.swa svg').getBoundingClientRect();
    return { outN: d.querySelectorAll('.swout .dc').length, inN: d.querySelectorAll('.swin .dc').length, tops: art.map(x => Math.round(x.top)),
      mid: Math.round(Math.abs((a.top + a.bottom) / 2 - (art[0].top + art[0].bottom) / 2)), grey: getComputedStyle(d.querySelector('.swout .dc .cav')).filter }; }));
  check('swap rows: the card out greyed ➜ the card in, every card\'s art on one line, the arrow at its middle', rows.length > 0 && rows.every(r =>
    r.outN > 0 && r.outN === r.inN && /grayscale/.test(r.grey) && (!ART || new Set(r.tops).size === 1 && r.mid <= 1)), JSON.stringify(rows));   // the art's alignment only with art
  check('swap rows show the uma as a character icon, named for screen readers', sw.length > 0 && sw.every(x => x.av > 0 && /’s deck:$/.test(x.label) && x.tips.length === x.av && x.tips.every(t => t && !/ and |’s deck/.test(t)) && x.shown === ''), JSON.stringify(sw));
  // two columns: the Loop setup panel (the status row at its foot) beside the results, level at the top; the editors in dialogs
  check('desktop: Loop setup beside the results, their tops level, the whole page in one 900 px screen', await pg.evaluate(() => { const set = document.querySelector('.lp .rbbar').getBoundingClientRect(), res = document.querySelector('#results .loop').getBoundingClientRect();
    return set.right <= res.left && Math.abs(set.top - res.top) <= 1 && document.querySelector('.lpside').getBoundingClientRect().bottom <= innerHeight + 60; }));
  check('the status line and the Auto-search and Deck per uma switches sit at the foot of the setup panel, Results in the results\' corner', await pg.evaluate(() => { const t = document.querySelector('.lp .rbbar');
    return ['run-note', 'x-auto', 'c-peruma', 'rank'].every(id => t.contains(document.getElementById(id))) && document.getElementById('rcol').contains(document.getElementById('c-top')) && document.getElementById('c-peruma').getAttribute('role') === 'switch'; }));
  check('no Loop size control: the solver picks 3-uma rotations or 4-uma loops', !(await pg.locator('#c-size').count()) && await pg.evaluate(() => window.__runs.at(-1).sizes === undefined));
  // the scenario sets the training-hint rate: switching it searches again with the scenario in the job, and the
  // choice is kept (the default setup's odds barely move: its decks get the targets from events, not training hints)
  check('the first search is a Unity Cup job', await pg.evaluate(() => window.__runs.at(-1).scenario === 'unity'));
  await pickScen(pg, 'ura'); await lastJob(pg, 'ura');   // the check reads the job: its (slow) search is not awaited
  // Unity Cup's golds follow the scenario: off under URA, back on under Unity, and the 8/8 example takes them too
  check('URA drops the Unity golds', (await pg.getAttribute('#g-20049', 'data-g')) === '0' && (await pg.getAttribute('#g-20046', 'data-g')) === '0');
  await pg.click('#t-preset'); await pg.locator('#pr-list .prow').first().locator('button', { hasText: 'Load' }).click();
  check('8/8 example under URA: eight targets, no guaranteed golds', (await pg.locator('#targets .tgt').count()) === 8 && (await pg.getAttribute('#g-20049', 'data-g')) === '0');
  // a saved preset: kept across a reload, and an import adds its presets to yours instead of replacing them
  await pg.locator('#targets .tgt .x').first().click(); await pg.click('#t-preset'); await pg.click('#pr-save'); await pg.fill('#prn-name', 'Seven'); await pg.click('#prn-save');
  check('a saved preset is the current one', (await pg.locator('#pr-list .prow.cur .pname').textContent()) === 'Seven');
  await pg.click('#preset-dlg .dclose');
  await pg.reload(); await jobSent(pg, () => window.__runs.length > 0);
  check('scenario survives a reload', (await scen(pg)) === 'ura' && (await pg.getAttribute('#x-scen', 'aria-label')) === 'Scenario: URA Finale');
  await pg.click('#t-preset');
  check('presets survive a reload', (await pg.locator('#pr-list .pname').allTextContents()).join('|') === '8/8 example|Seven');
  await pg.click('#preset-dlg .dclose'); await pg.click('#ex-open');
  const ex = JSON.parse(await pg.inputValue('#ex-text')); ex.settings.presets = [{ name: 'Seven', groups: [20159] }, { name: 'Other', groups: [20049] }];
  await pg.fill('#ex-text', JSON.stringify(ex)); await pg.click('#ex-load'); await pg.click('#ex-close'); await pg.click('#t-preset');
  check('an import merges presets', (await pg.locator('#pr-list .pname').allTextContents()).join('|') === '8/8 example|Seven|Seven (imported)|Other');
  await pg.click('#preset-dlg .dclose');
  await pickScen(pg, 'unity');
  check('Unity Cup marks No Stopping Me! and It\'s On! as guaranteed golds', (await pg.getAttribute('#g-20049', 'data-g')) === '2' && (await pg.getAttribute('#g-20046', 'data-g')) === '2');
  await settle(pg);
  check('no page errors', !pg.errors.length, pg.errors.join(' | '));
  await pg.context().close();
}
