// The type of the Global era's career tables the build writes (vite/pagedata.mjs '#career-data', imported by the career
// page alone; tsconfig.json's paths name this file for the type checker).
import type { CareerTables } from './model/data.ts';

declare const career: CareerTables;
export default career;
