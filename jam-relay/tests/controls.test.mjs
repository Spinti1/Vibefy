import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
import * as protocol from '../src/protocol.mjs';
const source=fs.readFileSync(new URL('../src/worker.js',import.meta.url),'utf8').replace(/^import .*;\r?\n/gm,'').replace('export default','globalThis.worker=').replace('export class JamRoom','globalThis.JamRoom=class JamRoom').replace('export class PlaylistShare','globalThis.PlaylistShare=class PlaylistShare');
async function room(){
 let now=100000,ready;const sockets=[];
 const ctx={storage:{get:async()=>null,put:async()=>{},setAlarm:async()=>{}},blockConcurrencyWhile:fn=>{ready=fn();},getWebSockets:()=>sockets.filter(s=>!s.closed)};
 const env={};class Clock extends Date{static now(){return now;}}
 const context={...protocol,Date:Clock,DurableObject:class{},Response,Request,URL,structuredClone};vm.createContext(context);vm.runInContext(source,context);
 const room=new context.JamRoom(ctx,env);await ready;
 const song={uri:'vibefy:juicevault:12345678-1234-1234-1234-123456789abc',title:'Synthetic recording',artists:[],duration_ms:200000};
 room.room={host_token:'a'.repeat(64),expires_at:1000000,snapshot:{track:song,position_ms:50000,playing:true},queue:[song],updated:now};
 function socket(role,shared_controls=false){const ws={data:{role,shared_controls,last:0,count:0,connected:now},messages:[],deserializeAttachment(){return this.data;},serializeAttachment(d){this.data=d;},send(s){this.messages.push(JSON.parse(s));},close(){this.closed=true;}};sockets.push(ws);return ws;}
 return {room,song,socket,step:ms=>{now+=ms;},send:(ws,p)=>room.webSocketMessage(ws,JSON.stringify(p))};
}
test('guests cannot publish authoritative state or choose arbitrary tracks',async()=>{
 const p=await room(),host=p.socket('host',true),guest=p.socket('guest');
 await p.send(guest,{type:'state',snapshot:null,queue:[]});assert.ok(p.room.room.snapshot);assert.equal(host.messages.length,0);
 await p.send(guest,{type:'control',action:'play_queue',index:0,uri:'file:///secret'});assert.equal(host.messages.length,0);
});
test('validated seeks reach a capable host and room-wide rate limits bound concurrent guests',async()=>{
 const p=await room(),host=p.socket('host',true),a=p.socket('guest'),b=p.socket('guest');
 await p.send(a,{type:'control',action:'seek',uri:p.song.uri,position_ms:999999,host_token:'never forwarded'});
 assert.deepEqual(JSON.parse(JSON.stringify(host.messages[0])),{type:'control',action:'seek',uri:p.song.uri,position_ms:200000});
 await p.send(b,{type:'control',action:'play_queue',index:0,uri:p.song.uri});assert.equal(host.messages.length,1);assert.equal(b.messages.at(-1).type,'error');
 p.step(500);await p.send(b,{type:'control',action:'play_queue',index:0,uri:p.song.uri});assert.equal(host.messages.length,2);
});
test('legacy host capabilities cannot be enabled by a guest',async()=>{
 const p=await room(),host=p.socket('host',false),guest=p.socket('guest');
 await p.send(guest,{type:'control',action:'seek',uri:p.song.uri,position_ms:1000,shared_controls:true});assert.equal(host.messages.length,0);
 assert.equal(p.room.state().shared_controls,false);
});

test('shared playback is explicit, bound to the recording and never an unchecked toggle',async()=>{
 const p=await room(),host=p.socket('host',true),guest=p.socket('guest');
 await p.send(guest,{type:'control',action:'set_playing',uri:p.song.uri,playing:false});
 assert.deepEqual(host.messages[0],{type:'control',action:'set_playing',uri:p.song.uri,playing:false});
 p.step(500);await p.send(guest,{type:'control',action:'set_playing',uri:p.song.uri,playing:'false'});assert.equal(host.messages.length,1);
 p.step(500);await p.send(guest,{type:'control',action:'set_playing',uri:'vibefy:youtube:abcdefghijk',playing:true});assert.equal(host.messages.length,1);
});
