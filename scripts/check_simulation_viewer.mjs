// Isolated JavaScript behavior checks, NOT browser rendering or visual QA.
// Uses a minimal in-memory DOM surface; no browser, server, network, or packages.
import fs from 'node:fs';
import vm from 'node:vm';
import assert from 'node:assert/strict';

const path = process.argv[2];
if (!path) throw new Error('Usage: node scripts/check_simulation_viewer.mjs RUN/simulation.html');
const html = fs.readFileSync(path, 'utf8');
class Element {
  constructor() { this.children = []; this.attributes = {}; this.listeners = {}; this.value = ''; this.checked = false; this.style = {}; this.ownText = ''; }
  set textContent(value) { this.ownText = String(value); this.children = []; }
  get textContent() { return this.ownText + this.children.map(c => c.textContent).join(''); }
  appendChild(child) { this.children.push(child); return child; }
  append(...children) { this.children.push(...children); }
  replaceChildren(...children) { this.ownText = ''; this.children = children; }
  setAttribute(key, value) { this.attributes[key] = String(value); }
  addEventListener(name, fn) { this.listeners[name] = fn; }
}
const ids = new Map([...html.matchAll(/\bid="([^"]+)"/g)].map(m => [m[1], new Element()]));
const get = id => { assert(ids.has(id), `Missing HTML element ${id}`); return ids.get(id); };
get('simulation-data').textContent = html.match(/<script id="simulation-data" type="application\/json">([\s\S]*?)<\/script>/)[1];
const data = JSON.parse(get('simulation-data').textContent);
assert.equal(data.config.scenario, 'mixed', 'This behavior fixture expects the mixed scenario');
assert.equal(data.config.duration_ms, 60000);
get('speed').value = '5';
const raf = [];
const context = vm.createContext({document:{getElementById:get,createElement:()=>new Element(),createElementNS:()=>new Element()},requestAnimationFrame:fn=>raf.push(fn),console});
const scripts = [...html.matchAll(/<script>([\s\S]*?)<\/script>/g)];
assert.equal(scripts.length, 1);
vm.runInContext(scripts[0][1], context, {timeout:2000,filename:'simulation-controller.js'});
const seek = time => { get('seek').value = String(time); get('seek').listeners.input(); };
assert.equal(get('viewer-status').hidden, true, 'Viewer must finish initialization');
assert.equal(get('play').disabled, false);
assert.match(get('clock').textContent, /^0\.00/);
assert.match(get('verifiers').textContent, /Waiting/);
seek(20000);
assert.match(get('verifiers').textContent, /Expired/);
assert.match(get('a-state').textContent, /offline/);
const withoutPeriodic = get('events').children.length;
get('include-checks').checked = true; get('include-checks').listeners.change();
assert(get('events').children.length > withoutPeriodic);
seek(30000);
assert.equal(get('link-state').textContent, 'Network partition');
assert.match(get('verifiers').textContent, /Verified current/);
seek(51250);
assert.match(get('verifiers').textContent, /Unconfirmed/);
seek(60000);
assert.match(get('verifiers').textContent, /Verified current/);
assert.equal(get('totals').children[0].children[0].textContent, '12');
get('restart').listeners.click();
assert.match(get('clock').textContent, /^0\.00/);
get('play').listeners.click(); assert.equal(get('play').textContent, 'Pause');
raf.shift()(100); raf.shift()(1100);
assert.match(get('clock').textContent, /^5\.00/);
get('play').listeners.click(); assert.equal(get('play').textContent, 'Play');
assert(get('co2').children.length > 10);
assert(get('light').children.length > 10);
console.log('Playback script checks passed: initialization, expiry, partition, delayed claim, timeline filtering, seeking, restart, play/pause, speed, charts. No browser rendering performed.');

get('simulation-data').textContent = '{invalid';
vm.runInContext(scripts[0][1], vm.createContext({document:{getElementById:get},console:{error:()=>{}}}), {timeout:2000});
assert.equal(get('viewer-status').hidden, false);
assert.match(get('viewer-status').textContent, /Playback could not start/);
console.log('Initialization errors produce a visible message.');
