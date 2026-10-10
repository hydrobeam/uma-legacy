// Precompute the example shown before the first search: deck + loop search, Unity Cup targets, everything owned,
// R cards off (the page default; R pals stay).
import fs from 'fs';
const P='../pkg/loopopt/data/global/';
const {instance}=await WebAssembly.instantiate(fs.readFileSync('loopopt.opt.wasm'),{env:{clock_ms:()=>performance.now()}});
const ex=instance.exports; const enc=new TextEncoder(), dec=new TextDecoder();
function call(obj){ const b=enc.encode(JSON.stringify(obj)); const p=ex.alloc(b.length); new Uint8Array(ex.memory.buffer,p,b.length).set(b);
  const r=ex.call(p,b.length); const n=ex.result_len(); const out=dec.decode(new Uint8Array(ex.memory.buffer,r,n).slice()); ex.dealloc(p,b.length); return JSON.parse(out); }
const J=(f)=>JSON.parse(fs.readFileSync(P+f));
call({cmd:'init',data:J('data.json'),skills:J('web_skills.json'),cards:J('web_cards.json'),events:J('card_events.json')});
// what the data holds ahead of its start (export/upcoming.py): left out, as the page leaves it out until then
const now=Date.now()/1000, later=(k)=>J('upcoming.json')[k].filter(([,t])=>t>now).map(([id])=>id);
const r=call({cmd:'deck',targets:[{group:20159},{group:20049,guaranteed:2},{group:20166},{group:20046,guaranteed:2},{group:20161},{group:21010},{group:20165},{group:20203}],
  extra:[[1003,20190,1],[1078,20165,1]],star_dist:[0.5,0.45,0.05],borrow:true,top_decks:1,per_uma:true,
  exclude:later('umas'),exclude_outfits:later('outfits'),
  exclude_cards:[...J('web_cards.json').cards.filter(c=>c.rarity===1&&!c.pal).map(c=>c.id),...later('cards')]});
fs.writeFileSync('example.json', JSON.stringify(r)); console.log(r.error||'', r.top.map(t=>Math.round(1/t.loop.p_all)));
