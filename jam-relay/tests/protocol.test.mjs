import test from 'node:test';
import assert from 'node:assert/strict';
import {CODE,randomCode,secret,track,snapshot,control} from '../src/protocol.mjs';
test('invites are high entropy and separate from host secrets',()=>{
 const codes=new Set(Array.from({length:1000},randomCode));assert.equal(codes.size,1000);
 for(const code of codes)assert.match(code,CODE);assert.match(secret(),/^[a-f0-9]{64}$/);
});
const song={uri:'vibefy:youtube:abcdefghijk',title:'Example',artists:[{name:'Artist'}]};
test('only exact supported source identifiers and bounded metadata cross the relay',()=>{
 assert.ok(track(song));assert.equal(track({...song,uri:'file:///secret'}),null);
 assert.equal(track({...song,uri:'https://127.0.0.1/private'}),null);
 assert.equal(track({...song,title:'x'.repeat(301)}),null);
 assert.equal(track({...song,art_url:'https://attacker.example/pixel'}).art_url,null);
 assert.throws(()=>snapshot({track:song,position_ms:-1,playing:true}));
 assert.throws(()=>snapshot({track:song,position_ms:1,playing:'yes'}));
 assert.equal(snapshot(null),null);
});
test('public JuiceVault identifiers are allowed but URLs, credentials and malformed IDs are excluded',()=>{
 assert.ok(track({...song,uri:'vibefy:juicevault:12345678-1234-1234-1234-123456789abc'}));
 for(const uri of ['vibefy:juicevault:../secret','vibefy:juicevault:123','https://api.juicevault.xyz/music/stream/id'])assert.equal(track({...song,uri}),null);
});
test('shared controls reject stale songs, stale queue positions and non-integer seeks',()=>{
 const current=snapshot({track:{...song,duration_ms:120000},playing:true,position_ms:1000}),queue=[song];
 assert.deepEqual(control({action:'seek',uri:song.uri,position_ms:200000},current,queue),{action:'seek',uri:song.uri,position_ms:120000});
 assert.equal(control({action:'seek',uri:'vibefy:youtube:bbbbbbbbbbb',position_ms:1000},current,queue),null);
 for(const position_ms of [-1,0.5,Infinity,'20'])assert.equal(control({action:'seek',uri:song.uri,position_ms},current,queue),null);
 assert.ok(control({action:'play_queue',index:0,uri:song.uri},current,queue));
 assert.equal(control({action:'play_queue',index:1,uri:song.uri},current,queue),null);
 assert.equal(control({action:'play_queue',index:0,uri:'file:///secret'},current,queue),null);
});

test('Jam artwork permits public source covers and strips tokenized or untrusted URLs',()=>{
 const cover='https://api.juicevault.xyz/cdn/covers/example.webp';
 assert.equal(track({...song,cover_url:cover}).cover_url,cover);
 for(const cover_url of ['https://api.juicevault.xyz/cdn/example.webp?token=private','https://api.juicevault.xyz.evil/cdn/pixel.jpg','http://i.ytimg.com/vi/abcdefghijk/hqdefault.jpg','https://user:secret@i.ytimg.com/vi/abcdefghijk/hqdefault.jpg'])assert.equal(track({...song,cover_url}).cover_url,null);
});
