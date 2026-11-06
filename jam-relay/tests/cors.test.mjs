import {test} from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
const source=fs.readFileSync(new URL('../src/worker.js',import.meta.url),'utf8').split('export class JamRoom')[0].replace(/^import .*;\r?\n/gm,'').replace('export default','globalThis.worker=');
const context={Response,Request,URL};vm.createContext(context);vm.runInContext(source,context);
test('Jam reflects only the exact allowed web origins, including the public fallback',async()=>{for(const origin of ['https://usevibefy.online','https://www.usevibefy.online','https://vibefy-neon.vercel.app']){for(const method of ['GET','OPTIONS']){const r=await context.worker.fetch(new Request('https://relay/health',{method,headers:{Origin:origin}}),{});assert.equal(r.status,200);assert.equal(r.headers.get('Access-Control-Allow-Origin'),origin);assert.equal(r.headers.get('Vary'),'Origin');}}});
test('other websites cannot create or preflight Jam sessions',async()=>{for(const origin of ['https://evil.test','https://usevibefy.online.evil.test','https://random.vercel.app']){const r=await context.worker.fetch(new Request('https://relay/rooms',{method:'OPTIONS',headers:{Origin:origin}}),{});assert.equal(r.status,403);assert.notEqual(r.headers.get('Access-Control-Allow-Origin'),origin);}});
