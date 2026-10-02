import fs from 'fs';
const P='../pkg/loopopt/'; const enc=new TextEncoder(), dec=new TextDecoder();
const {instance}=await WebAssembly.instantiate(fs.readFileSync('loopopt.opt.wasm'),{}); const ex=instance.exports;
function call(obj){ const b=enc.encode(JSON.stringify(obj)); const p=ex.alloc(b.length); new Uint8Array(ex.memory.buffer,p,b.length).set(b);
  const r=ex.call(p,b.length); const n=ex.result_len(); const out=dec.decode(new Uint8Array(ex.memory.buffer,r,n).slice()); ex.dealloc(p,b.length); return JSON.parse(out); }
let t=Date.now();
console.log(call({cmd:'init',data:JSON.parse(fs.readFileSync(P+'data.json')),skills:JSON.parse(fs.readFileSync(P+'web_skills.json')),cards:JSON.parse(fs.readFileSync(P+'web_cards.json')),events:JSON.parse(fs.readFileSync(P+'card_events.json'))}), Date.now()-t,'ms');
const base=JSON.parse(fs.readFileSync(P+'jobs/deck_unity8.json'));
const cards=JSON.parse(fs.readFileSync(P+'web_cards.json')).cards.map(c=>c.id).filter(i=>i!==30080&&i!==30116);
for (const [lab,extra] of [['all owned, top1',{}],['all owned, top3',{top_decks:3}],['noTI/festa/ines/goldcity + borrow, top1',{exclude:[1021,1034,1049,1031,1040],owned_cards:cards,borrow:true}],['same, top3',{exclude:[1021,1034,1049,1031,1040],owned_cards:cards,borrow:true,top_decks:3}]]) {
  t=Date.now(); const r=call({...base,...extra,cmd:'deck'});
  console.log(lab, Date.now()-t,'ms', r.error||'', r.top&&r.top.map(x=>Math.round(1/x.loop.p_all)+' '+(x.deck.borrowed||'-')+' '+x.loop.members.map(m=>m.short).join('/')));
}
