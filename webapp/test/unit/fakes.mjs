// Fakes the node tests of the run machines share (loop.test.mjs, careerrun.test.mjs): storage in a Map, a clock and
// timers the test moves, and a way to let pending promise callbacks run.
// a localStorage in a Map
export const memory = () => {
  const map = new Map();
  return { getItem: (key) => map.get(key) ?? null, setItem: (key, value) => void map.set(key, value), map };
};
// timers on a clock the test moves (advance runs what falls due, in order)
export function fakeClock() {
  let now = 0,
    last = 0;
  const timers = new Map();
  const add = (run, ms, every) => {
    timers.set(++last, { at: now + ms, every, run });
    return last;
  };
  return {
    now: () => now,
    setTimer: (run, ms) => add(run, ms, 0),
    every: (run, ms) => add(run, ms, ms),
    clearTimer: (id) => void timers.delete(id),
    // the timers and tickers still set
    pending: () => timers.size,
    advance(ms) {
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
export const flush = () => new Promise((resolve) => setImmediate(resolve));
