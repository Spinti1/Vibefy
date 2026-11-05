const fs=require('fs'),crypto=require('crypto'),path=require('path');
const root=path.resolve(__dirname,'..'),out=path.join(__dirname,'packages');
const names={nyan:'Nyan',wavelink:'WaveLink',juicevault:'JuiceVault',oneko:'Oneko',catjam:'CatJam',youtube:'YouTube',vivy:'Vivy',backmusic:'Backmusic',trackpeek:'TrackPeek',soundvisualizer:'SoundVisualizer'};
const asset=n=>fs.readFileSync(path.join(root,'assets/mods',n)).toString('base64');
let credits=fs.readFileSync(path.join(root,'assets/mods/README.md'),'utf8');for(const n of fs.readdirSync(path.join(root,'assets/mods')).filter(n=>n.startsWith('LICENSE') && n!=='LICENSE-SoundVisualizer.txt'))credits+='\n'+fs.readFileSync(path.join(root,'assets/mods',n),'utf8');
const catalog=[];
for(const [slug,id] of Object.entries(names)) {
 const sprites=slug==='nyan'?[{files:[1,2,3,4,5,6].map(i=>asset(`nyan_${i}.png`)),columns:1,rows:1,frames:6}]:slug==='catjam'?[{files:[asset('catjam-atlas.png')],columns:16,rows:21,frames:325}]:slug==='oneko'?[{files:[asset('oneko.png')],columns:8,rows:4,frames:32}]:[];
 const data=Buffer.from(JSON.stringify({schema:1,id,wasm:fs.readFileSync(path.join(out,slug+'.wasm')).toString('base64'),sprites,credits:credits+(slug==='soundvisualizer'?'\n'+fs.readFileSync(path.join(root,'assets/mods/LICENSE-SoundVisualizer.txt'),'utf8'):'')}));
 fs.writeFileSync(path.join(out,slug+'.vmod'),data);catalog.push({id,slug,bytes:data.length,sha256:crypto.createHash('sha256').update(data).digest('hex')});
}
fs.writeFileSync(path.join(__dirname,'catalog.json'),JSON.stringify(catalog,null,2)+'\n');console.log(catalog.map(v=>`${v.slug}: ${v.bytes} bytes`).join('\n'));
