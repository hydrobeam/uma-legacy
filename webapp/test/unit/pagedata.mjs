// The page data for node tests: what fakepage.mjs sets (globalThis.__LLP_DATA, keyed as the elements that once held it)
// as the exports of '#page-data' and '#career-data', which the Vite build writes for the page (vite/pagedata.mjs).
// package.json's imports name this file for everything but that build (node, esbuild). Read when the first page module
// loads, after fakepage.mjs; the lists are its arrays, so a test may still add to them before `loadData`.
const data = globalThis.__LLP_DATA;
export const assets = data['data-assets'];
export const cards = data['data-cards'];
export const umas = data['data-umas'];
export const skills = data['data-skills'];
export const img = data['data-img'];
export const skillIcons = data['data-skillicon'];
export const raceIds = data['data-raceids'];
export const hintBase = data['data-hintbase'];
export default data['data-career'] ?? null;
