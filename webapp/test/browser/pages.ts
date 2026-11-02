// The two planner pages as the browser flows open and wait on them. The flows' base URL is the page under test
// (playwright.config.ts): '/' is the career planner, '/loop' the loop planner.
import { expect, type Page } from 'playwright/test';

/** How long a flow waits on the solver: a replayed or filled reply, or the live worker's search. */
export const SOLVER_WAIT = 120_000;

/** The loop planner, its first search done. */
export async function openLoop(page: Page): Promise<void> {
  await page.goto('/loop');
  await loopSettled(page);
}
/** The loop page's search has finished: a search has run, or (no job sent) a saved result of these settings shows. */
export async function loopSettled(page: Page, timeout = SOLVER_WAIT): Promise<void> {
  await page.waitForFunction(
    () => {
      const n = document.getElementById('run-note');
      if (!n) return false;
      const text = n.textContent ?? '';
      return (
        (window.__runs.length > 0 || /^Checked [\d,]+ decks/.test(text)) &&
        !n.hasAttribute('data-busy') &&
        /Checked [\d,]+ decks|example/i.test(text)
      );
    },
    null,
    { timeout },
  );
}

/** The career planner, its first run done. */
export async function openCareer(page: Page): Promise<void> {
  await page.goto('/');
  await careerDone(page);
}
/** The career page's run has ended (an action that starts none returns at once). */
export async function careerDone(page: Page, timeout = SOLVER_WAIT): Promise<void> {
  await page.waitForFunction(
    () => {
      const n = document.getElementById('run-note');
      return !!n && !n.querySelector('[data-spinner]') && (n.textContent ?? '').length > 0;
    },
    null,
    { timeout },
  );
}

/** The settings editors, dialogs their buttons open: the Umas and Support cards (both pages), the veterans (career). */
const EDITORS = {
  umas: { button: '#o-umas', id: 'set-umas' },
  cards: { button: '#o-cards', id: 'set-cards' },
  vets: { button: '#v-toggle', id: 'set-vets' },
} as const;
export type Editor = keyof typeof EDITORS;
/** Opens the editor by its button, unless it is open. */
export async function openEditor(page: Page, which: Editor): Promise<void> {
  const { button, id } = EDITORS[which];
  const editor = page.locator(`#${id}`);
  if ((await editor.getAttribute('open')) !== null) return;
  await page.click(button);
  await expect(editor).toHaveAttribute('open');
}
/** Closes the editor by its Done, if it is open. */
export async function closeEditor(page: Page, which: Editor): Promise<void> {
  const editor = page.locator(`#${EDITORS[which].id}`);
  if ((await editor.getAttribute('open')) === null) return;
  await editor.getByRole('button', { name: 'Done', exact: true }).click();
  await expect(editor).not.toHaveAttribute('open');
}

/** What scrolls sideways (runs in the page): the page, an open dialog's body, or an open dialog that leaves the screen. */
export const SIDEWAYS = (): string[] => {
  const d = document.documentElement;
  const out: string[] = [];
  if (d.scrollWidth > d.clientWidth) out.push(`page ${d.scrollWidth} > ${d.clientWidth}`);
  for (const g of document.querySelectorAll('dialog[open]')) {
    const r = g.getBoundingClientRect();
    if (r.left < -0.5 || r.right > d.clientWidth + 0.5) out.push(`#${g.id} leaves the screen`);
    for (const s of g.querySelectorAll(':scope > *'))
      if (s.scrollWidth > s.clientWidth + 1)
        out.push(`#${g.id} scrolls sideways (${s.scrollWidth} > ${s.clientWidth})`);
  }
  return out;
};
