// Run from the source root. Reads package metadata, never credentials or profiles.
const fs=require('node:fs');const path=require('node:path');const {execFileSync}=require('node:child_process');
const metadata=JSON.parse(execFileSync('cargo',['metadata','--offline','--locked','--format-version','1','--filter-platform','x86_64-pc-windows-msvc'],{encoding:'utf8',maxBuffer:32*1024*1024}));
let output='VIBEFY THIRD-PARTY NOTICES\n\nGenerated from Cargo.lock and local dependency license files. Includes dependencies for multiple targets, not only the Windows executable. See CREDITS.md for adaptations, services and asset provenance. Third-party artwork is not relicensed by Vibefy.\n';
const seen=new Set();
function append(label,file){try{const stat=fs.statSync(file);if(stat.isFile()&&stat.size<256*1024){const value=fs.readFileSync(file,'utf8');const key=label+'\n'+value;if(!seen.has(key)){seen.add(key);output+='\n\n===== '+label+' =====\n'+value+'\n';}}}catch{}}
for(const pkg of metadata.packages.sort((a,b)=>a.name.localeCompare(b.name))){output+='\n\n--- '+pkg.name+' '+pkg.version+' ---\nLicense: '+(pkg.license||'See upstream notice')+'\nRepository: '+(pkg.repository||'https://crates.io/crates/'+pkg.name)+'\n';let dir=path.dirname(pkg.manifest_path),found=false;for(let depth=0;depth<4;depth++){for(const name of fs.readdirSync(dir)){if(/^(LICENSE|LICENCE|COPYING|NOTICE|UNLICENSE)([._-]|$)/i.test(name)){append(pkg.name+' / '+name,path.join(dir,name));found=true;}}if(found||!pkg.source?.startsWith('git+'))break;dir=path.dirname(dir);}}
function walk(dir){for(const item of fs.readdirSync(dir,{withFileTypes:true})){const file=path.join(dir,item.name);if(item.isDirectory())walk(file);else if(/LICENSE|OFL|NOTICE/i.test(item.name))append(path.relative(process.cwd(),file).replaceAll('\\','/'),file);}}
walk('assets');
for(const name of ['license-whisper.txt','license-ggml.txt','license-model.txt'])append('Whisper / '+name,path.join('..','Vibefy-App','auto-sync',name));
fs.writeFileSync('THIRD_PARTY_LICENSES.txt',output);console.log('Collected notices for '+metadata.packages.length+' Rust packages; '+Buffer.byteLength(output)+' bytes.');
