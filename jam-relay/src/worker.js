import {DurableObject} from 'cloudflare:workers';
import {CODE,TOKEN,track,snapshot,randomCode,secret} from './protocol.mjs';
const ORIGINS=new Set(['https://usevibefy.online','https://www.usevibefy.online']);
const json=(value,status=200)=>Response.json(value,{status,headers:{'Cache-Control':'no-store','Access-Control-Allow-Origin':'https://usevibefy.online','Vary':'Origin'}});
export default {
  async fetch(request,env) {
    const origin=request.headers.get('Origin');
    if(origin&&!ORIGINS.has(origin))return json({error:'Origin not allowed'},403);
    const url=new URL(request.url);
    if(request.method==='OPTIONS')return new Response(null,{headers:{'Access-Control-Allow-Origin':'https://usevibefy.online','Access-Control-Allow-Methods':'GET, POST','Access-Control-Allow-Headers':'Content-Type','Access-Control-Max-Age':'600'}});
    if(url.pathname==='/health')return json({service:'Vibefy Jam',protocol:2});
    const ip=request.headers.get('CF-Connecting-IP')||'unknown';
    if(!(await env.LIMITER.limit({key:ip})).success)return json({error:'Too many connection attempts. Try again later.'},429);
    if(url.pathname==='/rooms'&&request.method==='POST') {
      for(let attempt=0;attempt<3;attempt++) {
        const code=randomCode();const room=env.ROOMS.get(env.ROOMS.idFromName(code));
        const result=await room.fetch(new Request('https://room/create',{method:'POST'}));
        if(result.status===409)continue;
        if(!result.ok)return result;
        const data=await result.json();
        return json({code,host_token:data.host_token,expires_at:data.expires_at});
      }
      return json({error:'Please retry'},503);
    }
    const match=url.pathname.match(/^\/rooms\/([A-HJ-NP-Z2-9]{12})$/);
    if(!match||request.headers.get('Upgrade')?.toLowerCase()!=='websocket')return json({error:'Not found'},404);
    return env.ROOMS.get(env.ROOMS.idFromName(match[1])).fetch(request);
  }
};
export class JamRoom extends DurableObject {
  constructor(ctx,env){super(ctx,env);this.ctx=ctx;this.room=null;ctx.blockConcurrencyWhile(async()=>{this.room=await ctx.storage.get('room')||null;});}
  async fetch(request){
    if(new URL(request.url).pathname==='/create'){
      if(this.room)return json({error:'Collision'},409);
      this.room={host_token:secret(),expires_at:Date.now()+6*3600000,snapshot:null,queue:[],updated:Date.now(),persisted:Date.now()};
      await this.ctx.storage.put('room',this.room);await this.ctx.storage.setAlarm(this.room.expires_at);
      return json({host_token:this.room.host_token,expires_at:this.room.expires_at});
    }
    if(!this.room||Date.now()>this.room.expires_at)return json({error:'Jam expired or ended'},404);
    if(this.ctx.getWebSockets().length>=25)return json({error:'Jam is full'},409);
    const pair=new WebSocketPair();const [client,server]=Object.values(pair);
    this.ctx.acceptWebSocket(server);server.serializeAttachment({role:null,last:0,count:0,connected:Date.now(),name:'Listener'});
    await this.ctx.storage.setAlarm(Math.min(this.room.expires_at,Date.now()+15000));
    return new Response(null,{status:101,webSocket:client});
  }
  send(ws,data){try{ws.send(JSON.stringify(data));}catch{}}
  broadcast(data){for(const ws of this.ctx.getWebSockets()){if(ws.deserializeAttachment()?.role)this.send(ws,data);}}
  state(){
    const peers=this.ctx.getWebSockets().map(ws=>ws.deserializeAttachment()).filter(s=>s?.role);
    const host=peers.some(p=>p.role==='host');
    let current=this.room.snapshot?structuredClone(this.room.snapshot):null;
    if(current?.playing)current.position_ms=Math.min(86400000,current.position_ms+Math.max(0,Date.now()-this.room.updated));
    return {type:'state',snapshot:host?current:null,queue:this.room.queue,listeners:peers.filter(p=>p.role==='guest').length,host_present:host,sent:Date.now()};
  }
  async webSocketMessage(ws,message){
    if(typeof message!=='string'||message.length>24000){ws.close(1009,'Message too large');return;}
    if(!this.room||Date.now()>this.room.expires_at){ws.close(1008,'Jam expired');return;}
    let p;try{p=JSON.parse(message);}catch{ws.close(1008,'Invalid message');return;}
    if(!p||typeof p!=='object'||Array.isArray(p)){ws.close(1008,'Invalid message');return;}
    const a=ws.deserializeAttachment();const now=Date.now();
    if(now-a.last>1000){a.last=now;a.count=0;}if(++a.count>8){ws.close(1008,'Too many messages');return;}
    if(!a.role){
      if(p.type!=='hello'||p.protocol!==2||now-a.connected>15000){ws.close(1008,'Authentication required');return;}
      if(p.host_token){
        if(!TOKEN.test(p.host_token)||p.host_token!==this.room.host_token){ws.close(1008,'Invalid host credentials');return;}
        for(const other of this.ctx.getWebSockets())if(other!==ws&&other.deserializeAttachment()?.role==='host')other.close(1000,'Host reconnected');
        a.role='host';
      }else a.role='guest';
      a.name=String(p.name||'Listener').slice(0,60);ws.serializeAttachment(a);this.send(ws,{type:'welcome',role:a.role});this.broadcast(this.state());return;
    }
    ws.serializeAttachment(a);
    if(p.type==='ping'){this.send(ws,{type:'pong',echo:p.echo,sent:now});return;}
    if(p.type==='state'&&a.role==='host'){
      const previous=this.room.snapshot;let next;
      try{next=snapshot(p.snapshot);}catch{ws.close(1008,'Invalid track');return;}
      let queueChanged=false;
      if(p.queue!==undefined){if(!Array.isArray(p.queue)||p.queue.length>100){ws.close(1008,'Queue too large');return;}const queue=p.queue.map(track).filter(Boolean);queueChanged=JSON.stringify(queue)!==JSON.stringify(this.room.queue);this.room.queue=queue;}
      const expected=(previous?.position_ms||0)+(previous?.playing?Math.max(0,now-this.room.updated):0);
      const changed=queueChanged||previous?.track?.uri!==next?.track?.uri||previous?.playing!==next?.playing||Math.abs((next?.position_ms||0)-expected)>1500;
      this.room.snapshot=next;this.room.updated=now;
      // Persist control changes immediately; regular clock ticks need only a
      // checkpoint. The stored timestamp reconstructs the clock after hibernation.
      if(changed||now-(this.room.persisted||0)>=30000){this.room.persisted=now;await this.ctx.storage.put('room',this.room);}
      this.broadcast(this.state());return;
    }
    if(p.type==='add'){
      if(now-(a.lastAdd||0)<2000||(a.additions||0)>=100){this.send(ws,{type:'error',message:'Please wait before adding more songs'});return;}
      a.lastAdd=now;a.additions=(a.additions||0)+1;ws.serializeAttachment(a);
      const t=track(p.track);if(!t||this.room.queue.length>=100){this.send(ws,{type:'error',message:'Queue is full or track is unsupported'});return;}
      const hosts=this.ctx.getWebSockets().filter(s=>s.deserializeAttachment()?.role==='host');
      if(!hosts.length){this.send(ws,{type:'error',message:'Host is disconnected'});return;}
      for(const host of hosts)this.send(host,{type:'add',track:t});
      this.send(ws,{type:'notice',message:'Song sent to the host queue'});return;
    }
    if(p.type==='end'&&a.role==='host'){await this.end();return;}
    this.send(ws,{type:'error',message:'Only the host can control playback'});
  }
  webSocketClose(ws,code){try{ws.close(code,'Disconnected');}catch{}if(this.room)this.broadcast(this.state());}
  webSocketError(ws){try{ws.close(1011,'Connection lost');}catch{}}
  async end(){for(const ws of this.ctx.getWebSockets())ws.close(1000,'Jam ended');await this.ctx.storage.deleteAll();this.room=null;}
  async alarm(){
    if(!this.room||Date.now()>=this.room.expires_at){await this.end();return;}
    let next=this.room.expires_at;
    for(const ws of this.ctx.getWebSockets()){
      const a=ws.deserializeAttachment();
      if(!a?.role){if(Date.now()-a.connected>=15000)ws.close(1008,'Authentication timeout');else next=Math.min(next,a.connected+15000);}
    }
    await this.ctx.storage.setAlarm(next);
  }
}
