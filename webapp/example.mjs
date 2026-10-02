// Precompute the example shown before the first search: deck + loop search, Unity Cup targets, everything owned.
import fs from 'fs';
const P='../pkg/loopopt/';
const {instance}=await WebAssembly.instantiate(fs.readFileSync('loopopt.opt.wasm'),{});
const ex=instance.exports; const enc=new TextEncoder(), dec=new TextDecoder();
function call(obj){ const b=enc.encode(JSON.stringify(obj)); const p=ex.alloc(b.length); new Uint8Array(ex.memory.buffer,p,b.length).set(b);
  const r=ex.call(p,b.length); const n=ex.result_len(); const out=dec.decode(new Uint8Array(ex.memory.buffer,r,n).slice()); ex.dealloc(p,b.length); return JSON.parse(out); }
const J=(f)=>JSON.parse(fs.readFileSync(P+f));
call({cmd:'init',data:J('data.json'),skills:J('web_skills.json'),cards:J('web_cards.json'),events:J('card_events.json')});
const r=call({cmd:'deck',targets:[{group:20159},{group:20049,guaranteed:2},{group:20166},{group:20046,guaranteed:2},{group:20161},{group:21010},{group:20165},{group:20203}],
  extra:[[1003,20190,1],[1078,20165,1]],star_dist:[0.45,0.5,0.05],borrow:true,top_decks:1});
fs.writeFileSync('example.json', JSON.stringify(r)); console.log(r.error||'', r.top.map(t=>Math.round(1/t.loop.p_all)));
