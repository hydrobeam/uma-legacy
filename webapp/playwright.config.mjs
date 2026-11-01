// The browser suite (Playwright Test; test/groups.spec.mjs): webapp/dist served on a free local port, or LLP_URL.
//   npx playwright test                              every group in Chromium, those tagged @both in Firefox too
//   npx playwright test --project chromium -g roster  one browser, the groups whose title matches (a regular expression)
//   LLP_QUICK=1 npx playwright test --project chromium --grep @quick   the fast tier (./build.sh testq)
// LLP_JOBS groups run in parallel per browser (default 4); LLP_PROF=file appends where each group's time went;
// LLP_SHOTS=dir saves pictures of the scenario button and picker there. Exit code 1 if any check fails. A failed test
// keeps its trace (npx playwright show-trace test-results/<test>/trace.zip); the run's report is test-results/report.json.
import net from 'node:net';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'playwright/test';

// This file loads in the runner and again in each worker; the workers inherit the runner's environment, so what is set
// here once (the port, the run's start) holds in all of them.
// the browsers live in cache/ms-playwright (./build.sh browsers), outside node_modules, which npm ci empties
process.env.PLAYWRIGHT_BROWSERS_PATH ??= fileURLToPath(new URL('../cache/ms-playwright', import.meta.url));
process.env.LLP_T0 ??= String(Date.now()); // the run's start, for LLP_PROF's `start`
const freePort = () =>
  new Promise((resolve) => {
    const s = net.createServer().listen(0, '127.0.0.1', () => {
      const { port } = s.address();
      s.close(() => resolve(port));
    });
  });
if (!process.env.LLP_URL) process.env.LLP_PORT ??= String(await freePort());
process.env.LLP_PAGE ??= process.env.LLP_URL || `https://127.0.0.1:${process.env.LLP_PORT}/`; // helpers.mjs PAGE_URL; HTTP/2 over TLS, as the host
const JOBS = Math.max(1, +process.env.LLP_JOBS || 4); // per browser

export default defineConfig({
  testDir: 'test',
  testMatch: 'groups.spec.mjs',
  // the slowest group (career in Firefox) takes about 85-120 s with every group running, 211 s with the laptop throttled
  timeout: 360_000,
  retries: 0,
  fullyParallel: false,
  workers: 2 * JOBS,
  reporter: [['list'], ['json', { outputFile: 'test-results/report.json' }]],
  webServer: process.env.LLP_URL
    ? undefined
    : {
        command: `node serve.mjs ${process.env.LLP_PORT} 127.0.0.1 h2`,
        url: process.env.LLP_PAGE,
        ignoreHTTPSErrors: true, // its certificate is self-signed (serve.mjs)
        reuseExistingServer: false,
      },
  use: {
    baseURL: process.env.LLP_PAGE,
    // a failed test's trace: its actions, console, network and errors. DOM snapshots and the screencast took 4.6 times
    // the CPU (the career groups: 139 s of user time against 30 s untraced and 35 s like this), enough to throttle the
    // laptop; `--trace retain-on-failure` on the command line records them for a rerun of one group
    trace: { mode: 'retain-on-failure', snapshots: false, screenshots: false },
    screenshot: 'only-on-failure',
    // the library's defaults (Playwright Test's are none): a wait or action that never resolves fails in 30 s
    actionTimeout: 30_000,
    navigationTimeout: 30_000,
  },
  projects: [
    { name: 'chromium', use: { browserName: 'chromium' }, workers: JOBS },
    // Firefox runs the groups where its engine matters (groups.spec.mjs BOTH): the other 17 took 420 of its 650 CPU-s a run
    { name: 'firefox', use: { browserName: 'firefox' }, workers: JOBS, grep: /@both/ },
  ],
});
