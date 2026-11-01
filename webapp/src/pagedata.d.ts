// The types of the page data the build writes (vite/pagedata.mjs '#page-data'; tsconfig.json's paths name this file
// for the type checker). Each value is JSON the build wrote from the exports, unchecked: this is the one place it gets
// a type.
import type { Assets, PageData } from './data.ts';

export const assets: Assets;
export const cards: PageData['cards'];
export const umas: PageData['umas'];
export const skills: PageData['skills'];
/** the shipped images: their keys ('uma:1001' | 'outfit:100102' | 'card:30116' | ...), the fingerprinted folder (e.g.
 * 'assets/img.1a2b3c4d5e/'), each kind's file extension (avif; webp for the UI sprites) */
export const img: { keys: string[]; base: string; ext: Record<string, string> };
/** skill group -> icon id of its white and of its gold version */
export const skillIcons: { white: Record<number, number>; gold: Record<number, number> };
/** race name -> race id (banner key) */
export const raceIds: Record<string, number>;
/** each scenario's training-hint base, by scenario key */
export const hintBase: Record<string, unknown>;
