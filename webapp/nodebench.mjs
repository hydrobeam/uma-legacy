import fs from 'fs';
const [wasmPath, era, ...jobs] = process.argv.slice(2);
const P = era === 'gm' ? '../jp/' : '../pkg/loopopt/'; const EV = 'card_events.json';
const enc=new TextEncoder(), dec=new TextDecoder();
const {instance}=await WebAssembly.instantiate(fs.readFileSync(wasmPath),{env:{clock_ms:()=>performance.now()}}); const ex=instance.exports;
function call(obj){ const b=enc.encode(JSON.stringify(obj)); const p=ex.alloc(b.length); new Uint8Array(ex.memory.buffer,p,b.length).set(b);
  const r=ex.call(p,b.length); const n=ex.result_len(); const out=dec.decode(new Uint8Array(ex.memory.buffer,r,n).slice()); ex.dealloc(p,b.length); return JSON.parse(out); }
call({cmd:'init',data:JSON.parse(fs.readFileSync(P+'data.json')),skills:JSON.parse(fs.readFileSync(P+'web_skills.json')),cards:JSON.parse(fs.readFileSync(P+'web_cards.json')),events:JSON.parse(fs.readFileSync(P+EV))});
for (const j of jobs) { const job=JSON.parse(fs.readFileSync(P+'jobs/'+j+'.json')); const t=Date.now(); const r=call({...job,cmd:'deck',...(process.env.PU?{per_uma:true}:{})});
  console.log(j, Date.now()-t, Math.round(1/r.top[0].loop.p_all)); }
