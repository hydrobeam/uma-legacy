import fs from 'fs';
const P='../pkg/loopopt/'; const enc=new TextEncoder(), dec=new TextDecoder();
const {instance}=await WebAssembly.instantiate(fs.readFileSync('loopopt.opt.wasm'),{}); const ex=instance.exports;
function call(obj){ const b=enc.encode(JSON.stringify(obj)); const p=ex.alloc(b.length); new Uint8Array(ex.memory.buffer,p,b.length).set(b);
  const r=ex.call(p,b.length); const n=ex.result_len(); const out=dec.decode(new Uint8Array(ex.memory.buffer,r,n).slice()); ex.dealloc(p,b.length); return JSON.parse(out); }
call({cmd:'init',data:JSON.parse(fs.readFileSync(P+'data.json')),skills:JSON.parse(fs.readFileSync(P+'web_skills.json')),cards:JSON.parse(fs.readFileSync(P+'web_cards.json')),events:JSON.parse(fs.readFileSync(P+'card_events.json'))});
for (const j of process.argv.slice(2)) {
  const job=JSON.parse(fs.readFileSync(P+'jobs/pal_'+j+'.json'));
  const t=Date.now(); const r=call({...job,cmd:'deck'});
  console.log(j, Date.now()-t,'ms', r.error||'', r.top&&r.top.map(x=>Math.round(1/x.loop.p_all)+' '+x.loop.members.map(m=>m.short||m.name).join('/')));
}
