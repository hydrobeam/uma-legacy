// The value the code knows is there (Rust's `expect`): returned as it is, or an error naming why it should have been.
// For a null that would be a bug; a null the page expects (nothing chosen yet, a field older answers lack) is a check
// at the use instead, naming that state.
export function must<T>(value: T, reason: string): NonNullable<T> {
  if (value == null) throw new Error(`missing: ${reason}`);
  return value;
}

// A condition the code knows holds (Rust's `assert!`): nothing when it does, else an error naming why it should have.
// For a fact about values (two lists the solver builds from the same members are as long), where `must` is for one value.
export function assert(cond: unknown, reason: string): asserts cond {
  if (!cond) throw new Error(`failed: ${reason}`);
}

// `run()`'s value, or null when it throws an `expected` error (one the caller goes on without the value for: settings
// that make no job, say). Any other error is a bug: it is reported through the page's error path (errorlog.ts logs it,
// and the browser suite fails on it) and the caller goes on as for an expected one, so a bug costs that value, not the
// page's start or what the caller was doing (Rust's `.ok()` on a Result, with the unexpected errors logged).
export function orNull<T>(run: () => T, expected: abstract new (...args: never[]) => Error): T | null {
  try {
    return run();
  } catch (error) {
    if (!(error instanceof expected)) reportError(error);
    return null;
  }
}
