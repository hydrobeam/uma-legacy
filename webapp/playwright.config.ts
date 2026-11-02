// The browser flows (Playwright Test; test/browser/*.spec.ts): webapp/dist served on a free local port, or LLP_URL.
//   npx playwright test                                 every flow in Chromium; those titled @both in Firefox too
//   npx playwright test --project chromium phone        one browser, the spec files whose name matches
// Each flow is a small independent test (fullyParallel), from a state it seeds (fixtures.ts); its solver replays the
// recorded replies and never runs (test/browser/solver.ts; LLP_SOLVER=live records them, with the built solver). An
// action or an assertion waits at most 5 s; a wait on the solver says so (pages.ts SOLVER_WAIT); a wait on the page's
// own timers moves its clock (page.clock). A failed test keeps its trace (npx playwright show-trace
// test-results/<test>/trace.zip); the run's report is test-results/report.json.
import net from 'node:net';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'playwright/test';

// This file loads in the runner and again in each worker; the workers inherit the runner's environment, so the port set
// here once holds in all of them.
// the browsers live in cache/ms-playwright (./build.sh browsers), outside node_modules, which npm ci empties
process.env.PLAYWRIGHT_BROWSERS_PATH ??= fileURLToPath(new URL('../cache/ms-playwright', import.meta.url));
const freePort = () =>
  new Promise<number>((resolve, reject) => {
    const s = net.createServer().listen(0, '127.0.0.1', () => {
      const address = s.address();
      if (address === null || typeof address === 'string') reject(new Error('no port'));
      else s.close(() => resolve(address.port));
    });
  });
if (!process.env.LLP_URL) process.env.LLP_PORT ??= String(await freePort());
// HTTP/2 over TLS, as the host serves it
const PAGE = process.env.LLP_URL || `https://127.0.0.1:${process.env.LLP_PORT}/`;

export default defineConfig({
  testDir: 'test/browser',
  testMatch: '*.spec.ts',
  fullyParallel: true,
  timeout: 60_000,
  expect: { timeout: 5_000 },
  retries: 0,
  reporter: [['list'], ['json', { outputFile: 'test-results/report.json' }]],
  ...(process.env.LLP_URL
    ? {}
    : {
        webServer: {
          command: `npx --no-install vite preview --port ${process.env.LLP_PORT} --host 127.0.0.1 --strictPort`,
          url: PAGE,
          ignoreHTTPSErrors: true, // its certificate is self-signed (vite.config.mjs, basicSsl)
          reuseExistingServer: false,
        },
      }),
  use: {
    baseURL: PAGE,
    ignoreHTTPSErrors: true,
    viewport: { width: 1440, height: 900 },
    // a failed test's trace: its actions, console, network and errors (DOM snapshots and the screencast cost 4.6 times
    // the CPU; `--trace retain-on-failure` on the command line records them for a rerun)
    trace: { mode: 'retain-on-failure', snapshots: false, screenshots: false },
    screenshot: 'only-on-failure',
    actionTimeout: 5_000,
    navigationTimeout: 15_000,
  },
  projects: [
    { name: 'chromium', use: { browserName: 'chromium' } },
    // Firefox only where the engine matters (layout at real widths, keyboard and focus, the worker): the flows with
    // @both in their title or their describe's
    { name: 'firefox', use: { browserName: 'firefox' }, grep: /@both/ },
  ],
});
