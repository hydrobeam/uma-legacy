import { check, artCheck, open, dialog, } from './helpers.mjs';

export default async function popups(b) {
  const pg = await open(b);
  await pg.locator('.loop .lacts button', { hasText: 'Race schedule' }).first().click(); await dialog(pg, 'sched-dlg');
  check('schedule: a tab per member of the rotation plus "All three"', (await pg.locator('#sched-dlg .stab').count()) === 4 && (await pg.locator('#sched-dlg .stab').last().innerText()) === 'All three');
  const grids = await pg.evaluate(() => [...document.querySelectorAll('.ag-grid')].map(g => { const r = g.getBoundingClientRect(); return Math.round(r.width) + 'x' + Math.round(r.height); }));
  check('agenda: three identical year grids', grids.length === 3 && new Set(grids).size === 1, grids.join(' '));
  check('agenda: Junior Year opens with the game\'s grey cells for April to June', (await pg.locator('#sched-dlg .ag-year.y0 .ag-slot.pre').count()) === 12);
  check('agenda: fits the window without scrolling', await pg.evaluate(() => { const d = document.querySelector('#sched-dlg .dlg'); return d.scrollHeight <= d.clientHeight + 1; }));
  artCheck('agenda: every race has its banner', (await pg.locator('#sched-dlg .ag-race').count()) === (await pg.locator('#sched-dlg .ag-race img.banner').count()));
  check('agenda: a race\'s "x% win" sits in its turn bar, its tooltip says why', await pg.evaluate(() => [...document.querySelectorAll('#sched-dlg .ag-slot.on .ag-when')].every(w => /^(Early|Late) [A-Z][a-z]{2}\d+% win$/.test(w.textContent) && /^\d+% win: 110/.test(w.querySelector('b').title || w.querySelector('b').dataset.tip))));   // (a hovered one's title is held in data-tip while its balloon shows)
  await pg.locator('#sched-dlg .stab').last().click(); check('All three shows the comparison table, a column per member', (await pg.locator('#sched-dlg table.sched').count()) > 0
    && (await pg.locator('#sched-dlg table.sched').first().locator('th.mh').count()) === 3);
  check('All three: turns written as on the member tabs ("Early Apr"), no shared/optional key', await pg.evaluate(() => [...document.querySelectorAll('#sched-dlg td.when')].every(t => /^(Early|Late) [A-Z][a-z]{2}$/.test(t.textContent)))
    && await pg.locator('#sched-dlg .agkey').isHidden());
  check('All three: each win % says why on hover, as its note promises', await pg.evaluate(() => { const c = [...document.querySelectorAll('#sched-dlg table.sched tbody td')].filter(t => /^\*?\d+$/.test(t.textContent));
    return c.length > 0 && c.every(t => /^\d+% win: 110/.test(t.title)) && /hover for why/.test(document.querySelector('#sched-dlg .tablewrap .hint').textContent); }));
  await pg.keyboard.press('Escape');
  await pg.locator('.loop .lacts button', { hasText: 'Affinity' }).first().click(); await dialog(pg, 'why-dlg');
  // the career page's table: each parent, then her two parents (relation, G1 wins shared, affinity), a Total row
  const at = await pg.evaluate(() => { const t = document.querySelector('#why-dlg .afftab'), rows = [...t.tBodies].flatMap(b => [...b.rows]);
    const n = (x) => +x.replace(/\D/g, ''), aff = rows.map(r => n(r.cells[3].textContent));
    return { careers: document.querySelectorAll('#why-dlg .afpick:not(.afpar) button').length, rows: rows.length, own: rows.filter(r => /her own earlier veteran/.test(r.textContent)).map(r => n(r.cells[1].textContent)),
      sum: aff.reduce((a, x) => a + x, 0), total: n(t.tFoot.rows[0].cells[3].textContent), prose: document.querySelectorAll('#why-dlg p').length }; });
  check('affinity: a rotation has 3 careers; each shows six legacies and a Total that sums them, no prose', at.careers === 3 && at.rows === 6 && Math.abs(at.sum - at.total) <= 6 && at.prose === 0, JSON.stringify(at));
  check('affinity: in a rotation the trainee is her own grandparent twice, marked, with 0 relation points', at.own.length === 2 && at.own.every(x => x === 0), JSON.stringify(at.own));
  check('no page errors', !pg.errors.length, pg.errors.join(' | '));
  await pg.context().close();
}
