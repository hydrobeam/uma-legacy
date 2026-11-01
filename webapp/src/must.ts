// The value the code knows is there (Rust's `expect`): returned as it is, or an error naming why it should have been.
// For a null that would be a bug; a null the page expects (nothing chosen yet, a field older answers lack) is a check
// at the use instead, naming that state.
export function must<T>(value: T, reason: string): NonNullable<T> {
  if (value == null) throw new Error(`missing: ${reason}`);
  return value;
}
