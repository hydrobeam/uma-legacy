// The page's pink draw (model/pink.ts `drawPink`) on the solver's report of a career whose start is not one set of
// grades (career/reply.rs `AptitudeStart::Outcomes`: a veteran before her result is known among the legacies): one
// number first draws the outcome by its chance, as the solver's own draw does (career/schedule.rs `draw_pink`), then
// the draw goes on from that outcome's start.
import assert from 'node:assert/strict';
import { test } from 'vitest';
import { drawPink } from '../../src/model/pink.ts';
import { Aptitude, type Aptitudes, type Grade, type Grades } from '../../src/gen/solver.ts';

// every aptitude in the solver's order, the sprint first (`pink_gen`)
const pinkGen = [
  Aptitude.Sprint,
  Aptitude.Mile,
  Aptitude.Medium,
  Aptitude.Long,
  Aptitude.Turf,
  Aptitude.Dirt,
  Aptitude.FrontRunner,
  Aptitude.PaceChaser,
  Aptitude.LateSurger,
  Aptitude.EndCloser,
].map((aptitude) => ({ aptitude, p: 0.1 }));
// her grades: `a` at sprint, G elsewhere
const grades = (a: Grade): Grades => ({ distance: [a, 1, 1, 1], surface: [1, 1], style: [1, 1, 1, 1] });
const sequence = (values: number[]) => {
  let i = 0;
  return () => values[i++] ?? 0.5;
};

test('drawPink draws a several-outcome start by its chances first', () => {
  const ap: Aptitudes = {
    base: grades(1),
    pink_gen: pinkGen,
    start: {
      kind: 'outcomes',
      outcomes: [
        { p: 0.25, outcome: { start: grades(1), fire: [] } },
        { p: 0.75, outcome: { start: grades(8), fire: [] } },
      ],
    },
  };
  // 0.5 passes the first outcome's 0.25: the second, which starts at S on the first aptitude (its pink then drawn
  // from the A-or-S aptitudes, the sprint alone)
  const drawn = drawPink(ap, sequence([0.5, 0.1, 0.5]));
  assert.deepEqual(drawn.apt, grades(8));
  assert.deepEqual(drawn.pinks, { [Aptitude.Sprint]: 2 });
  // 0.1 falls within the first: no aptitude at A, no pink
  assert.deepEqual(drawPink(ap, sequence([0.1])), { pinks: {}, apt: grades(1) });
});

test('drawPink on one known start reads no outcome number', () => {
  const ap: Aptitudes = { base: grades(1), pink_gen: pinkGen, start: { kind: 'known', start: grades(8), fire: [] } };
  assert.deepEqual(drawPink(ap, sequence([0.1, 0.5])), { pinks: { [Aptitude.Sprint]: 2 }, apt: grades(8) });
});
