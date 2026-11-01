import { expect } from 'playwright/test';
import { check, open, json } from './helpers.mjs';

export default async function targets(b) {
  // the target list by keyboard: removing keeps the focus in the list, the add box takes Enter and the arrows
  const pg = await open(b);
  const skl = await json(pg, 'data-skills'),
    gold = (g) => skl.find((x) => x[0] === g)[2];
  // the add box sits above the list, two targets per row
  const lay = await pg.evaluate(() => {
    const t = [...document.querySelectorAll('#targets .tgt')].map((e) => e.getBoundingClientRect());
    return {
      rows: new Set(t.map((r) => Math.round(r.top))).size,
      n: t.length,
      addFirst: document.getElementById('t-add').getBoundingClientRect().bottom <= Math.min(...t.map((r) => r.top)),
    };
  });
  check('targets: add box first, two per row', lay.addFirst && lay.rows === Math.ceil(lay.n / 2), JSON.stringify(lay));
  const tgts = pg.locator('#targets .tgt'),
    addBox = pg.locator('#t-add'),
    golden = 'a gold the scenario guarantees shows as the gold itself, in gold';
  await expect.soft(pg.locator('#g-20049 .name'), golden).toHaveText(gold(20049));
  await expect.soft(pg.locator('#g-20049'), `${golden}: its class`).toHaveClass(/\bgold\b/);
  await pg.locator('#targets .x').nth(2).focus();
  await pg.keyboard.press('Enter');
  const removed = 'removing a target by keyboard moves the focus to the next remove button';
  await expect.soft(tgts, `${removed}: one fewer`).toHaveCount(7);
  await expect.soft(pg.locator('#targets .x').nth(2), removed).toBeFocused();
  for (let i = 0; i < 7; i++) await pg.keyboard.press('Enter');
  await expect.soft(tgts, '… and to the add box after the last one: none left').toHaveCount(0);
  await expect.soft(addBox, '… and to the add box after the last one').toBeFocused();
  await pg.fill('#t-add', 'corner');
  const first = await pg.locator('#t-sugg button').first().innerText();
  await pg.keyboard.press('ArrowDown');
  await pg.keyboard.press('ArrowDown');
  await pg.keyboard.press('ArrowUp');
  const at = await pg.evaluate(() => [...document.querySelectorAll('#t-sugg button')].indexOf(document.activeElement));
  await pg.keyboard.press('ArrowUp');
  const back = await pg.evaluate(() => document.activeElement.id);
  check(
    'arrows walk the suggestions, Up from the first returns to the box',
    at === 0 && back === 't-add',
    at + ' ' + back,
  );
  await pg.keyboard.press('ArrowDown');
  await pg.keyboard.press('Escape');
  await expect.soft(pg.locator('#t-sugg'), 'Escape on a suggestion closes the list').toBeHidden();
  await expect.soft(addBox, '… and returns to the box').toBeFocused();
  await pg.fill('#t-add', 'corner');
  await pg.keyboard.press('Enter');
  const white = first.split(' → ')[0];
  await expect.soft(tgts, 'Enter adds the first match').toHaveCount(1);
  await expect.soft(tgts, `Enter adds the first match (${first})`).toContainText(white);
  await expect.soft(addBox, '… and keeps the focus in the box').toBeFocused();
  await pg.fill('#t-add', white.slice(0, 8));
  await expect
    .soft(
      pg.locator('#t-sugg .snote', { hasText: `${white} is already a target` }).first(),
      'a skill already listed: "… is already a target"',
    )
    .toHaveText(`${white} is already a target`);
  await pg.fill('#t-add', 'zzqx');
  await expect.soft(pg.locator('#t-sugg'), 'no match: "No skill matches"').toHaveText(/^No skill matches/);
  // a gold matched by its own name shows with its white
  const g = skl.find(
    (x) =>
      x[2] &&
      !x[1].toLowerCase().includes(x[2].toLowerCase().slice(0, 6)) &&
      !skl.some((y) => y !== x && (y[1] + (y[2] || '')).toLowerCase().includes(x[2].toLowerCase())),
  );
  await pg.fill('#t-add', g[2]);
  await expect
    .soft(pg.locator('#t-sugg button').first(), 'a match on the gold\'s name shows "white → gold"')
    .toHaveText(g[1] + ' → ' + g[2], { useInnerText: true });
  check('no page errors', !pg.errors.length, pg.errors.join(' | '));
  await pg.context().close();
}
