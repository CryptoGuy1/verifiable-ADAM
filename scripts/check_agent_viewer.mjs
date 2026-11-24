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
seek(data.rounds[1].completed_at_ms);assert.match(get('advisory').textContent,/advisory raised/);
seek(data.rounds[2].completed_at_ms);assert.equal(get('sensor-vote').textContent,'NO ADVISORY');assert.equal(get('aggregator-vote').textContent,'ADVISORY');
assert.match(get('sensor-ballot').textContent,/Signed ballot delivered/);
seek(data.rounds[3].completed_at_ms);assert.match(get('advisory').textContent,/withheld/);
seek(data.rounds[5].completed_at_ms);assert.match(get('advisory').textContent,/withheld/);assert.match(get('decision-vote').textContent,/No eligible response/);
seek(data.duration_ms);assert.match(get('advisory').textContent,/withdrawn/);assert.match(get('freshness').textContent,/remaining lifetime: 0.0/);
assert.match(get('verification').textContent,/Checker: expired/);
get('restart').listeners.click();assert.match(get('clock').textContent,/^0.00/);
get('play').listeners.click();assert.equal(get('play').textContent,'Pause');raf.shift()(0);raf.shift()(1000);assert.match(get('clock').textContent,/^5.00/);
get('play').listeners.click();assert.equal(get('play').textContent,'Play');
get('round-select').value='1';get('round-select').listeners.change();assert.match(get('advisory').textContent,/advisory raised/);
console.log('Agent viewer checks passed: initialization, clear/raised/withheld/withdrawn, dissent, ballots, absence, expiry, playback, restart and round selection.');
