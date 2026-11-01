import { check, artCheck, open, dialog, settle } from './helpers.mjs';

// (runs in the page) every search's first result's first deck card becomes Seiun Sky [Foolproof Plan] (30008), its
// story ended at link 1, in the replies the page gets (solverreplay.mjs keeps them as the solver sent them)
const endAtLink = () => {
  window.__workerHooks.message.push((data) => {
    const deck = data?.r?.top?.[0]?.deck;
    if (!deck?.card_ids?.length) return;
    deck.card_ids[0] = 30008;
    deck.end_chain_at[0] = 1;
  });
};

export default async function popups(b) {
  const pg = await open(b, { init: endAtLink });
  await pg.locator('#results').getByRole('button', { name: 'Race schedule', exact: true }).first().click();
  await dialog(pg, 'sched-dlg');
  check(
    'schedule: a tab per member of the rotation plus "All three"',
    (await pg.locator('#sched-dlg').getByRole('tab').count()) === 4 &&
      (await pg.locator('#sched-dlg').getByRole('tab').last().innerText()) === 'All three',
  );
  const grids = await pg.evaluate(() =>
    [...document.querySelectorAll('.ag-grid')].map((g) => {
      const r = g.getBoundingClientRect();
      return Math.round(r.width) + 'x' + Math.round(r.height);
    }),
  );
  check('agenda: three identical year grids', grids.length === 3 && new Set(grids).size === 1, grids.join(' '));
  check(
    "agenda: Junior Year opens with the game's grey cells for April to June",
    (await pg.locator('#sched-dlg .ag-year.y0 .ag-slot.pre').count()) === 12,
  );
  check(
    'agenda: fits the window without scrolling',
    await pg.evaluate(() => {
      const d = document.querySelector('#sched-dlg .dlg');
      return d.scrollHeight <= d.clientHeight + 1;
    }),
  );
  artCheck(
    'agenda: every race has its banner',
    (await pg.locator('#sched-dlg .ag-race').count()) === (await pg.locator('#sched-dlg .ag-race img.banner').count()),
  );
  await pg.mouse.move(0, 0); // (off any race: a hovered one's title is held while its balloon shows)
  check(
    'agenda: a race\'s "x% win" sits in its turn bar, its tooltip says why',
    await pg.evaluate(() =>
      [...document.querySelectorAll('#sched-dlg .ag-slot.on .ag-when')].every(
        (w) =>
          /^(Early|Late) [A-Z][a-z]{2}\d+% win$/.test(w.textContent) &&
          /^\d+% win: 110/.test(w.querySelector('b').title),
      ),
    ),
  );
  await pg.locator('#sched-dlg').getByRole('tab').last().click();
  check(
    'All three shows the comparison table, a column per member',
    (await pg.locator('#sched-dlg table.sched').count()) > 0 &&
      (await pg.locator('#sched-dlg table.sched').first().locator('th.mh').count()) === 3,
  );
  check(
    'All three: turns written as on the member tabs ("Early Apr"), no shared/optional key',
    (await pg.evaluate(() =>
      [...document.querySelectorAll('#sched-dlg td.when')].every((t) =>
        /^(Early|Late) [A-Z][a-z]{2}$/.test(t.textContent),
      ),
    )) && (await pg.locator('#sched-dlg .agkey').isHidden()),
  );
  check(
    'All three: each win % says why on hover, as its note promises',
    await pg.evaluate(() => {
      const c = [...document.querySelectorAll('#sched-dlg table.sched tbody td')].filter((t) =>
        /^\*?\d+$/.test(t.textContent),
      );
      return (
        c.length > 0 &&
        c.every((t) => /^\d+% win: 110/.test(t.title)) &&
        /hover for why/.test(document.querySelector('#sched-dlg .tablewrap .hint').textContent)
      );
    }),
  );
  await pg.keyboard.press('Escape');
  await pg.locator('#results').getByRole('button', { name: 'Affinity', exact: true }).first().click();
  await dialog(pg, 'why-dlg');
  // the career page's table: each parent, then her two parents (relation, points from shared G1s, affinity), a Total row
  const at = await pg.evaluate(() => {
    const t = document.querySelector('#why-dlg table'),
      rows = [...t.tBodies].flatMap((b) => [...b.rows]);
    const n = (x) => +x.replace(/\D/g, ''),
      aff = rows.map((r) => n(r.cells[3].textContent));
    return {
      careers: document.querySelectorAll('#why-dlg [aria-label="Trainee"] [role="radio"]').length,
      rows: rows.length,
      own: rows.filter((r) => /her own earlier veteran/.test(r.textContent)).map((r) => n(r.cells[1].textContent)),
      sum: aff.reduce((a, x) => a + x, 0),
      total: n(t.tFoot.rows[0].cells[3].textContent),
      prose: document.querySelectorAll('#why-dlg p').length,
    };
  });
  check(
    'affinity: a rotation has 3 careers; each shows six legacies and a Total that sums them, no prose',
    at.careers === 3 && at.rows === 6 && Math.abs(at.sum - at.total) <= 6 && at.prose === 0,
    JSON.stringify(at),
  );
  check(
    'affinity: in a rotation the trainee is her own grandparent twice, marked, with 0 relation points',
    at.own.length === 2 && at.own.every((x) => x === 0),
    JSON.stringify(at.own),
  );
  // gmtip.ts over a modal dialog: the balloon is its own element in the top layer (never put inside the dialog, which a
  // component draws), the title held while it shows; a title drawn again meanwhile (as a render that changes it does)
  // is kept when the balloon goes, not overwritten by the one held
  const pill = pg.locator('#why-dlg [aria-label="Trainee"] [role="radio"]').first();
  await pill.hover();
  await pg.waitForSelector('.gmtip:not([hidden])');
  const tip = await pg.evaluate(() => {
    const t = document.querySelector('.gmtip'),
      held = document.querySelector('#why-dlg [aria-label="Trainee"] [role="radio"]');
    const shown = {
      inBody: t.parentElement === document.body,
      top: t.matches(':popover-open'),
      text: t.textContent,
      held: !held.hasAttribute('title'),
    };
    held.title = 'Drawn again';
    return shown;
  });
  await pg.mouse.move(0, 0);
  const after = await pg.evaluate(() => ({
    title: document.querySelector('#why-dlg [aria-label="Trainee"] [role="radio"]').title,
    hidden: document.querySelector('.gmtip').hidden,
    top: document.querySelector('.gmtip').matches(':popover-open'),
  }));
  check(
    'tooltip over a modal dialog: the balloon in the top layer, outside the dialog; a title drawn again meanwhile is kept',
    tip.inBody &&
      tip.top &&
      tip.held &&
      tip.text.length > 0 &&
      after.title === 'Drawn again' &&
      after.hidden &&
      !after.top,
    JSON.stringify({ tip, after }),
  );
  await pg.keyboard.press('Escape');
  // a deck card the search ends at a link: its panel names the skill IT needs prioritised to take the ending choice.
  // Seiun Sky [Foolproof Plan] ends at link 1 for Second Wind; Dodging Danger is on the continuing choice. The card is
  // the first result's first deck card, retagged in each of the solver's replies (endAtLink; the searches here end no
  // chain): opened by its click, read once the panel has drawn, closed by a second click
  const endPanel = async () => {
    const chip = pg.locator('#results button[data-cid="30008"][data-end="1"]').first();
    await chip.click();
    const text = await pg.locator('[data-pop="card"]:not([hidden])').innerText();
    await chip.click();
    return text;
  };
  const plain = await endPanel();
  check(
    'chain end: the panel names the skill to prioritise and when IT may not take it',
    plain.includes('Prioritise Second Wind in IT: it takes the chain-ending choice at link 1.') &&
      plain.includes(
        "IT may not take it if Dodging Danger is also prioritised, or if Second Wind's hint is already Lv 5.",
      ),
    plain,
  );
  await pg.fill('#t-add', 'Dodging Danger');
  await pg.keyboard.press('Enter');
  await settle(pg);
  const rival = await endPanel();
  check(
    'chain end: a target on the continuing choice is named as making the end uncertain',
    rival.includes('Dodging Danger is also a target: prioritising both makes the end uncertain.'),
    rival,
  );
  await settle(pg);
  check('no page errors', !pg.errors.length, pg.errors.join(' | '));
  await pg.context().close();
}
