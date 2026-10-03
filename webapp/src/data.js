// The data build.py embeds in the page, for the chosen era.
import { $ } from './dom.js';
import { ERA } from './settings.js';
const SFX = ERA === 'gm' ? '-gm' : '';
const parse = (id) => JSON.parse($(id).textContent);
export const CARDS = parse('data-cards' + SFX);     // [{id,name,rar,pal,links,hf,whites,ev:[[group,level,link,p]]}]
export const UMAS = parse('data-umas' + SFX);       // [{id,name,outfits:[{id,label,rel}]}]
export const HINTS = parse('data-hints' + SFX);     // {trainee:[[id,group,p]], deck:[[card,group,level,p]]}
export const CAT = parse('data-skills' + SFX);      // [[group, white, gold|null]]
export const EXAMPLE = parse('data-example');       // precomputed default result
export const SHIPPED = parse('data-img');           // 'uma:1001' | 'outfit:100102' | 'card:30116' -> base64 WebP
export const SKILLICON = parse('data-skillicon');   // skill group -> icon id
export const RACEIDS = parse('data-raceids');       // race name -> race id (banner key)
export const CARD_BY_ID = new Map(CARDS.map(c => [c.id, c]));
export const SK = new Map(CAT.map(([g, w, gd]) => [g, { w, gd }]));
export const NAME = new Map(UMAS.map(u => [u.id, u.name]));
export const PRESET = [
  { group: 20159, guaranteed: 0 }, { group: 20049, guaranteed: 2 }, { group: 20166, guaranteed: 0 }, { group: 20046, guaranteed: 2 },
  { group: 20161, guaranteed: 0 }, { group: 21010, guaranteed: 0 }, { group: 20165, guaranteed: 0 }, { group: 20203, guaranteed: 0 },
];
