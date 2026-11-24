// Behavior checks against the exported agent run, not browser rendering.
import fs from 'node:fs';
import vm from 'node:vm';
import assert from 'node:assert/strict';
const html=fs.readFileSync(process.argv[2],'utf8');
class Element {
 constructor(){this.children=[];this.listeners={};this.value='';this.ownText='';this.hidden=false;this.disabled=true;}
 set textContent(v){this.ownText=String(v);this.children=[];}
 get textContent(){return this.ownText+this.children.map(c=>c.textContent).join('');}
 appendChild(c){this.children.push(c);return c;}
 replaceChildren(...c){this.children=c;this.ownText='';}
 addEventListener(k,fn){this.listeners[k]=fn;}
 setAttribute(k,v){this[k]=String(v);}
}
const ids=new Map([...html.matchAll(/\bid="([^"]+)"/g)].map(m=>[m[1],new Element()]));
const get=id=>{assert(ids.has(id),id);return ids.get(id);};
get('simulation-data').textContent=html.match(/<script id="simulation-data" type="application\/json">([\s\S]*?)<\/script>/)[1];
const data=JSON.parse(get('simulation-data').textContent),raf=[];
get('speed').value='5';
const context=vm.createContext({document:{getElementById:get,createElement:()=>new Element()},console,requestAnimationFrame:fn=>raf.push(fn)});
vm.runInContext([...html.matchAll(/<script>([\s\S]*?)<\/script>/g)][0][1],context,{timeout:2000});
assert.equal(get('viewer-status').hidden,true);
assert.equal(get('play').disabled,false);
const seek=t=>{get('seek').value=String(t);get('seek').listeners.input();};
seek(data.rounds[0].completed_at_ms);assert.match(get('advisory').textContent,/No CO₂ advisory/);
assert.equal(get('round-title').textContent,'Low CO₂');
assert(!/<details[^>]*id="technical-details"[^>]*\bopen\b/.test(html),'Technical details must start collapsed');
assert.match(get('reading-a').textContent,/600 ppm/);
get('next').listeners.click();assert.match(get('advisory').textContent,/advisory on/);assert.equal(get('round-select').value,'1');
seek(data.rounds[2].completed_at_ms);assert.equal(get('sensor-vote').textContent,'No advisory');assert.equal(get('aggregator-vote').textContent,'Raise advisory');
assert.match(get('sensor-ballot').textContent,/Signed vote received/);
if(data.rounds[2].reason==='classification_mismatch'){assert.match(get('advisory-detail').textContent,/majority got it wrong/);}else{assert.equal(get('round-title').textContent,'The agents disagree');}
seek(data.rounds[3].completed_at_ms);assert.match(get('advisory').textContent,/blocked/);assert.equal(get('reading-a').textContent,'No reading');
seek(data.rounds[5].completed_at_ms);assert.match(get('advisory').textContent,/blocked/);assert.match(get('decision-vote').textContent,/No response/);
seek(data.duration_ms);assert.match(get('advisory').textContent,/removed/);assert.equal(get('freshness').textContent,'Readings have expired');assert.equal(get('next').disabled,true);
assert.match(get('verification').textContent,/Checker: expired/);
get('restart').listeners.click();assert.equal(get('round-select').value,'0');assert.match(get('advisory').textContent,/No CO₂ advisory/);
get('play').listeners.click();assert.equal(get('play').textContent,'Pause');raf.shift()(0);raf.shift()(50);assert.match(get('clock').textContent,/^0.25/);
get('play').listeners.click();assert.equal(get('play').textContent,'Watch example');
get('round-select').value='1';get('round-select').listeners.change();assert.match(get('advisory').textContent,/advisory on/);
// Replaying an example stops before moving into the next one.
get('play').listeners.click();raf.shift()(2000);raf.shift()(200000);
assert.equal(get('play').textContent,'Watch example');assert.equal(get('round-select').value,'1');
console.log('Demo viewer checks passed: clear first view, collapsed technical details, simple outcomes, dissent, missing readings/agents, expiry, next/select, replay/pause and automatic stop.');
