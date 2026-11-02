// @vitest-environment happy-dom
// uma.moe's sync (umamoesetup.ts, umamoe.ts) with uma.moe stood in for by a fetch that answers two listings made from
// the umadump test file's first two veterans: a rejected key says so and is forgotten; a saved key sends one request
// (the key as its header only, parents with all but one target on herself, followable trainers) and is kept under its
// own entry; the listings join the pool as borrowed veterans; Include max borrows asks without the follower limit; a
// kept answer is drawn with no request (a reload); Refresh asks again; a late reply for a target set left before it came
// never lands; the Grand Masters era asks only for the sparks Global has. npx vitest run (in webapp)
import { expect, test, vi } from 'vitest';
import type { UmamoeListing } from '../../src/umadump.ts';
import type { Veteran } from '../../src/veterans/store.ts';
import { CAREER, RECORDS, listing } from '../browser/listings.ts';

vi.mock('#page-data', () => import('./fakepage.ts'));

const ITEMS = RECORDS.slice(0, 2).map((r, i) => listing(r, i + 1));
interface Sent {
  url: URL;
  key: string | null;
  credentials: RequestCredentials | undefined;
}
const sent: Sent[] = [];
// the next answers held back (resolved by the test), else at once
let hold: ((items: UmamoeListing[]) => void) | null = null;
let holding = false;
const answer = (items: UmamoeListing[]) =>
  new Response(JSON.stringify({ items, total: String(items.length), page: 0, limit: 100, total_pages: 1 }), {
    status: 200,
    headers: { 'content-type': 'application/json' },
  });
vi.stubGlobal('fetch', async (input: string, init?: RequestInit) => {
  const key = new Headers(init?.headers).get('x-api-key');
  sent.push({ url: new URL(input), key, credentials: init?.credentials });
  if (key !== 'test-key') return new Response('{}', { status: 401 });
  if (holding) return new Promise<Response>((resolve) => (hold = (items) => resolve(answer(items))));
  return answer(ITEMS);
});

const TARGETS = [{ group: 20049 }, { group: 20046 }, { group: 20159 }, { group: 20166 }];
let listed: readonly Veteran[] = [];
// the modules as a page load makes them (a reload: fresh modules, the same storage)
const fresh = async () => {
  vi.resetModules();
  await (await import('../../src/data.ts')).loadData(CAREER);
  const um = await import('../../src/umamoesetup.ts');
  um.connectUmamoe({ targets: () => TARGETS, setListings: (l) => (listed = l) });
  return um;
};
const settle = () => new Promise((r) => setTimeout(r, 0));

test('a rejected key says so and is forgotten; a saved key sends one request, its header only, and is kept apart', async () => {
  const um = await fresh();
  expect(um.hasKey.value).toBe(false);
  um.saveUmKey('bad-key');
  await settle();
  expect(sent).toHaveLength(1);
  expect(um.umStatus.value.short).toBe('key rejected');
  expect(um.umStatus.value.text).toMatch(/Key rejected/);
  expect(um.hasKey.value).toBe(false);
  expect(localStorage.getItem('llp:umamoe-key')).toBeNull();

  um.saveUmKey('test-key');
  await settle();
  expect(um.umStatus.value.short).toBe('2 veterans');
  expect(sent).toHaveLength(2);
  const [, req] = sent;
  expect(req?.key).toBe('test-key');
  expect(req?.credentials).toBe('omit'); // (no cookie goes)
  expect(req?.url.search).not.toContain('test-key');
  expect(req?.url.searchParams.get('max_follower_num')).toBe('999');
  expect(req?.url.searchParams.getAll('main_white_factors')).toHaveLength(TARGETS.length - 1);
  expect(localStorage.getItem('llp:umamoe-key')).toBe('test-key');
  for (const k of ['llp:settings', 'llp:career']) expect(localStorage.getItem(k) ?? '').not.toContain('test-key');
});

test("the listings join the pool as borrowed veterans from uma.moe, with their lender and uma.moe's order", async () => {
  expect(listed.map((v) => v.id)).toEqual(['u900001', 'u900002']);
  for (const v of listed) {
    expect(v.borrowed).toBe(true);
    expect(v.src).toBe('umamoe');
    expect(v.parents.every((p) => p !== null && p.chara !== null)).toBe(true);
  }
  expect(listed.map((v) => v.lender?.name)).toEqual(['Trainer 1', 'Trainer 2']);
  expect(listed.map((v) => v.lender?.followers)).toEqual([13, 14]);
  const um = await import('../../src/umamoesetup.ts');
  const byWins = [...listed].sort((a, b) => um.SORTS.wins[2](b) - um.SORTS.wins[2](a)).map((v) => v.lender?.name);
  expect(byWins).toEqual(['Trainer 2', 'Trainer 1']);
});

test('Include max borrows asks again without the follower limit; off again, the kept answer (no request)', async () => {
  const um = await import('../../src/umamoesetup.ts');
  um.setUmFull(true);
  await settle();
  expect(sent).toHaveLength(3);
  expect(sent[2]?.url.searchParams.has('max_follower_num')).toBe(false);
  um.setUmFull(false);
  await settle();
  expect(sent).toHaveLength(3);
});

test('a reload draws the kept answer with no request; Refresh asks once more', async () => {
  listed = [];
  const um = await fresh();
  await um.syncListings();
  expect(sent).toHaveLength(3);
  expect(um.umStatus.value.short).toBe('2 veterans');
  expect(listed).toHaveLength(2);
  await um.syncListings(true);
  expect(sent).toHaveLength(4);
});

test('a late reply for a target set left before it came replaces neither the listings on show nor the label', async () => {
  const um = await import('../../src/umamoesetup.ts');
  // (no answer kept for max borrows: its request goes out and is held)
  const kept = JSON.parse(localStorage.getItem('llp:umamoe') ?? '{}');
  for (const k of Object.keys(kept)) if (!k.includes('max_follower_num')) delete kept[k];
  localStorage.setItem('llp:umamoe', JSON.stringify(kept));
  holding = true;
  um.setUmFull(true);
  await settle();
  expect(um.umStatus.value.short).toBe('searching…');
  um.setUmFull(false);
  await settle();
  hold?.([]);
  await settle();
  holding = false;
  expect(sent).toHaveLength(5);
  expect(listed).toHaveLength(2);
  expect(um.umStatus.value.short).toBe('2 veterans');
});

test('the Grand Masters era asks uma.moe only for the sparks Global has', async () => {
  const { targetFactors } = await import('../../src/umamoe.ts');
  const { loadData } = await import('../../src/data.ts');
  const all = targetFactors(TARGETS.map((t) => t.group)).flat();
  const [kept] = all;
  if (kept === undefined) throw new Error('the targets have no factors');
  await loadData({ ...CAREER, moe: [kept] });
  expect(targetFactors(TARGETS.map((t) => t.group))).toEqual([[kept]]);
  await loadData(CAREER);
});
