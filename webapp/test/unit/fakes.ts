// Fakes the node tests of the run machines share (loop.test.ts, careerrun.test.ts): storage in a Map, a clock and
// timers the test moves, and a way to let pending promise callbacks run.
// a localStorage in a Map
export const memory = () => {
  const map = new Map<string, string>();
  return {
    getItem: (key: string) => map.get(key) ?? null,
    setItem: (key: string, value: string) => void map.set(key, value),
    removeItem: (key: string) => void map.delete(key),
    map,
  };
};
interface Timer {
  at: number;
  every: number;
  run: () => void;
}
// timers on a clock the test moves (advance runs what falls due, in order)
export function fakeClock() {
  let now = 0,
    last = 0;
  const timers = new Map<number, Timer>();
  const add = (run: () => void, ms: number, every: number) => {
    timers.set(++last, { at: now + ms, every, run });
    return last;
  };
  return {
    now: () => now,
    setTimer: (run: () => void, ms: number) => add(run, ms, 0),
    every: (run: () => void, ms: number) => add(run, ms, ms),
    clearTimer: (id: number | undefined) => void (id !== undefined && timers.delete(id)),
    /** the timers set and not cleared or fired: 'once' or 'every' each */
    pending: () => [...timers.values()].map((t) => (t.every ? 'every' : 'once')),
    advance(ms: number) {
      const end = now + ms;
      for (;;) {
        const due = [...timers].filter(([, t]) => t.at <= end).sort((a, b) => a[1].at - b[1].at)[0];
        if (!due) break;
        const [id, timer] = due;
        now = timer.at;
        if (timer.every) timer.at += timer.every;
        else timers.delete(id);
        timer.run();
      }
      now = end;
    },
  };
}
// every promise callback already due runs
export const flush = () => new Promise<void>((resolve) => setImmediate(resolve));
