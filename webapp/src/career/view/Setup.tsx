// The Borrowed parent choice of the career setup (into #bp): three choices, saved with the other settings (a change runs
// the planner again: the entry's effect on the setup).
import { settings, setBorrowPar } from '../../settings.ts';
import { borrowChoice } from '../job.js';

const CHOICES = [
  ['allow', 'Allowed', 'This career may borrow one parent, and so may the careers after it'],
  [
    'later',
    'None left today',
    'Out of borrows for today: this career uses two of your veterans, and the plan still counts on borrowing in the careers after it',
  ],
  ['never', 'Never', "No borrowed parents, in this career or the plan's later ones"],
] as const;
export function BorrowChoice() {
  const now = borrowChoice(settings.value);
  return (
    <>
      {CHOICES.map(([v, label, title]) => (
        <button
          key={v}
          type="button"
          data-v={v}
          aria-pressed={v === now}
          title={title}
          onClick={() => {
            if (v !== borrowChoice(settings.value)) setBorrowPar(v);
          }}
        >
          {label}
        </button>
      ))}
    </>
  );
}
