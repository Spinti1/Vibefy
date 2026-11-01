const {test}=require('node:test');
const assert=require('node:assert/strict');
const vm=require('node:vm');
const fs=require('node:fs');
const path=require('node:path');
const source=fs.readFileSync(path.join(__dirname,'../public/device.js'),'utf8');
function route(ua,pathname,search=''){let redirect;const ctx={window:{},document:{documentElement:{dataset:{}}},navigator:{userAgent:ua},location:{pathname,search,replace:target=>{redirect=target;}},URLSearchParams};vm.runInNewContext(source,ctx);return {phone:ctx.window.vibefyPhone,redirect};}
test('desktop browsers cannot initialize the mobile player, including narrow windows',()=>{for(const ua of ['Mozilla/5.0 (Windows NT 10.0; Win64; x64)','Mozilla/5.0 (Macintosh; Intel Mac OS X)','Mozilla/5.0 (X11; Linux x86_64)'])for(const p of ['/app','/app.html','/app/'])assert.deepEqual(route(ua,p),{phone:false,redirect:'/'});});
test('phones enter mobile player and preserve Jam invitations',()=>{for(const ua of ['Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X)','Mozilla/5.0 (Linux; Android 14) Mobile']){assert.deepEqual(route(ua,'/','?jam=vibefy-abc'),{phone:true,redirect:'/app?jam=vibefy-abc'});assert.deepEqual(route(ua,'/app'),{phone:true,redirect:undefined});}});
test('phone users can explicitly read the desktop information page',()=>{assert.equal(route('iPhone','/','?site=1').redirect,undefined);});
test('an Android tablet is not mistaken for a phone',()=>{assert.equal(route('Mozilla/5.0 (Linux; Android 14) Safari/537.36','/').phone,false);});
