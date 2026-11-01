// The type of the Global era's career tables the build writes (vite/pagedata.mjs '#career-data', imported by the career
// page alone; package.json's imports name this file for the type checker).
import type { CareerTables } from './data.ts';

declare const career: CareerTables;
export default career;
