// Vitest's setup for every node test file (vite.config.mjs): a file on happy-dom's document unmounts what
// @testing-library/preact rendered after each test, so the next test starts from an empty body.
import { afterEach } from 'vitest';

if (typeof document !== 'undefined') {
  const { cleanup } = await import('@testing-library/preact');
  afterEach(cleanup);
}
