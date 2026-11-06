const {test}=require('node:test');
const assert=require('node:assert/strict');
const vm=require('node:vm');
const fs=require('node:fs');
const path=require('node:path');
const source=fs.readFileSync(path.join(__dirname,'../public/device.js'),'utf8');
function route(ua,pathname,search=''){let redirect;const ctx={window:{},document:{documentElement:{dataset:{}}},navigator:{userAgent:ua},location:{pathname,search,replace:target=>{redirect=target;}},URLSearchParams};vm.runInNewContext(source,ctx);return {phone:ctx.window.vibefyPhone,redirect};}
test('desktop browsers can open explicit player routes',()=>{for(const ua of ['Windows NT','Macintosh','Linux x86_64'])for(const p of ['/app','/web','/phone'])assert.deepEqual(route(ua,p),{phone:false,redirect:undefined});});
test('the root remains a presentation on phones and desktops',()=>{for(const ua of ['iPhone','Android Mobile','Windows NT'])assert.equal(route(ua,'/').redirect,undefined);});
test('phone users can explicitly read the desktop information page',()=>{assert.equal(route('iPhone','/','?site=1').redirect,undefined);});
test('an Android tablet is not mistaken for a phone',()=>{assert.equal(route('Mozilla/5.0 (Linux; Android 14) Safari/537.36','/').phone,false);});

test('player invitations do not leave the player route',()=>{assert.equal(route('Windows NT','/web','?jam=ABCDEFGHJKLM').redirect,undefined);});
