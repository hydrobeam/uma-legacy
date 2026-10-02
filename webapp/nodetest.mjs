import fs from 'fs';
const P='../pkg/loopopt/';
const bytes=fs.readFileSync(P+'target/wasm32-unknown-unknown/release/loopopt.wasm');
const {instance}=await WebAssembly.instantiate(bytes,{});
const ex=instance.exports; const enc=new TextEncoder(), dec=new TextDecoder();
function call(obj){ const b=enc.encode(JSON.stringify(obj)); const p=ex.alloc(b.length); new Uint8Array(ex.memory.buffer,p,b.length).set(b);
  const r=ex.call(p,b.length); const n=ex.result_len(); const out=dec.decode(new Uint8Array(ex.memory.buffer,r,n)); ex.dealloc(p,b.length); return JSON.parse(out); }
let t=Date.now();
console.log(call({cmd:'init',data:JSON.parse(fs.readFileSync(P+'data.json')),skills:JSON.parse(fs.readFileSync(P+'web_skills.json'))}), Date.now()-t,'ms');
t=Date.now();
const r=call({cmd:'skills',targets:[{group:20159},{group:20049,guaranteed:2},{group:20166,guaranteed:2},{group:20046,guaranteed:2},{group:20161,deck_white:0.8},{group:21010},{group:20165,deck_gold:0.8},{group:20203,guaranteed:2}],extra:[[1078,20165,1],[1003,20190,1]],top:5});
console.log('skills', Date.now()-t,'ms', r.error||'', r.loops&&r.loops.map(l=>Math.round(1/l.p_all)+' '+l.members.map(m=>m.short).join('/')), r.proven_top, r.searched);
t=Date.now();
const a=call({cmd:'affinity',top:5});
console.log('affinity', Date.now()-t,'ms', a.loops.map(l=>l.score.toFixed(2)+' '+l.members.map(m=>m.short).join('/')));
console.log(JSON.stringify(r.loops[0].outfits), JSON.stringify(r.loops[0].schedule[0]));
