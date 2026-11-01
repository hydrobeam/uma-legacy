// ---------- a text box (a search box, a name box): the page's ground in a thin line, round corners ----------
// Its look as one class list, which an input's own place joins: `tap`, at least 44 px tall on a phone (a touch
// target); `short`, 36 px tall everywhere (the target skills' add box, which fits the setup's column).
export const field = {
  tap: 'bg-bg border-line w-full min-w-0 rounded-md border px-2.5 py-1.5 max-md:min-h-11',
  short: 'bg-bg border-line min-h-9 w-full min-w-0 rounded-md border px-2.5 py-1.5',
} as const;
