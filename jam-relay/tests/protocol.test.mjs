import test from 'node:test';
import assert from 'node:assert/strict';
import {CODE,randomCode,secret,track,snapshot} from '../src/protocol.mjs';
test('invites are high entropy and separate from host secrets',()=>{
 const codes=new Set(Array.from({length:1000},randomCode));assert.equal(codes.size,1000);
 for(const code of codes)assert.match(code,CODE);assert.match(secret(),/^[a-f0-9]{64}$/);
});
const song={uri:'vibefy:youtube:abcdefghijk',title:'Example',artists:[{name:'Artist'}]};
test('only exact YouTube identifiers and bounded metadata cross the relay',()=>{
 assert.ok(track(song));assert.equal(track({...song,uri:'file:///secret'}),null);
 assert.equal(track({...song,uri:'https://127.0.0.1/private'}),null);
 assert.equal(track({...song,title:'x'.repeat(301)}),null);
 assert.equal(track({...song,art_url:'https://attacker.example/pixel'}).art_url,null);
 assert.throws(()=>snapshot({track:song,position_ms:-1,playing:true}));
 assert.throws(()=>snapshot({track:song,position_ms:1,playing:'yes'}));
 assert.equal(snapshot(null),null);
});
