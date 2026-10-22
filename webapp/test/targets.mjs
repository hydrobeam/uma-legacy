import { check, open, json } from './helpers.mjs';

export default async function targets(b) {
  // the target list by keyboard: removing keeps the focus in the list, the add box takes Enter and the arrows
  const pg = await open(b);
  const skl = await json(pg, 'data-skills'), gold = (g) => skl.find(x => x[0] === g)[2];
  // the add box sits above the list, two targets per row
  await pg.editor('#targets');
  const lay = await pg.evaluate(() => { const t = [...document.querySelectorAll('#targets .tgt')].map(e => e.getBoundingClientRect());
    return { rows: new Set(t.map(r => Math.round(r.top))).size, n: t.length, addFirst: document.getElementById('t-add').getBoundingClientRect().bottom <= Math.min(...t.map(r => r.top)) }; });
  check('targets: add box first, two per row', lay.addFirst && lay.rows === Math.ceil(lay.n / 2), JSON.stringify(lay));
  check('a gold the scenario guarantees shows as the gold itself, in gold', (await pg.locator('#g-20049 .name').innerText()).trim() === gold(20049) && await pg.locator('#g-20049').evaluate(e => e.classList.contains('gold')), await pg.locator('#g-20049').innerText());
  await pg.locator('#targets .x').nth(2).focus(); await pg.keyboard.press('Enter');
  check('removing a target by keyboard moves the focus to the next remove button', (await pg.locator('#targets .tgt').count()) === 7
    && await pg.evaluate(() => document.activeElement.classList.contains('x') && [...document.querySelectorAll('#targets .x')].indexOf(document.activeElement) === 2));
  for (let i = 0; i < 7; i++) await pg.keyboard.press('Enter');
  check('… and to the add box after the last one', (await pg.locator('#targets .tgt').count()) === 0 && await pg.evaluate(() => document.activeElement.id === 't-add'));
  await pg.fill('#t-add', 'corner');
  const first = await pg.locator('#t-sugg button').first().innerText();
  await pg.keyboard.press('ArrowDown'); await pg.keyboard.press('ArrowDown'); await pg.keyboard.press('ArrowUp');
  const at = await pg.evaluate(() => [...document.querySelectorAll('#t-sugg button')].indexOf(document.activeElement));
  await pg.keyboard.press('ArrowUp'); const back = await pg.evaluate(() => document.activeElement.id);
  check('arrows walk the suggestions, Up from the first returns to the box', at === 0 && back === 't-add', at + ' ' + back);
  await pg.keyboard.press('ArrowDown'); await pg.keyboard.press('Escape');
  check('Escape on a suggestion closes the list and returns to the box', await pg.locator('#t-sugg').isHidden() && await pg.evaluate(() => document.activeElement.id === 't-add'));
  await pg.fill('#t-add', 'corner'); await pg.keyboard.press('Enter');
  check('Enter adds the first match and keeps the focus in the box', (await pg.locator('#targets .tgt').count()) === 1 && (await pg.locator('#targets .tgt').innerText()).includes(first.split(' → ')[0]) && await pg.evaluate(() => document.activeElement.id === 't-add'), first);
  await pg.fill('#t-add', first.split(' → ')[0].slice(0, 8));
  check('a skill already listed: "… is already a target"', (await pg.locator('#t-sugg .snote').allInnerTexts()).some(t => t === first.split(' → ')[0] + ' is already a target'));
  await pg.fill('#t-add', 'zzqx');
  check('no match: "No skill matches"', /^No skill matches/.test(await pg.locator('#t-sugg').innerText()));
  // a gold matched by its own name shows with its white
  const g = skl.find(x => x[2] && !x[1].toLowerCase().includes(x[2].toLowerCase().slice(0, 6)) && !skl.some(y => y !== x && (y[1] + (y[2] || '')).toLowerCase().includes(x[2].toLowerCase())));
  await pg.fill('#t-add', g[2]);
  check('a match on the gold\'s name shows "white → gold"', (await pg.locator('#t-sugg button').first().innerText()) === g[1] + ' → ' + g[2], g[1] + ' → ' + g[2]);
  check('no page errors', !pg.errors.length, pg.errors.join(' | '));
  await pg.context().close();
}
