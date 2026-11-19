"""Generate a SideStore source only from a built, structurally validated IPA."""
import argparse, hashlib, json, plistlib, zipfile
from pathlib import Path
from urllib.parse import urlparse
from datetime import datetime, timezone
p=argparse.ArgumentParser();p.add_argument("ipa",type=Path);p.add_argument("--url",required=True);p.add_argument("--output",type=Path,required=True);a=p.parse_args()
u=urlparse(a.url)
if u.scheme!="https" or u.hostname!="github.com" or u.username or u.password: p.error("Use the public GitHub HTTPS release asset URL")
with zipfile.ZipFile(a.ipa) as z:
 names=z.namelist();plists=[n for n in names if n.startswith("Payload/") and n.count("/")==2 and n.endswith(".app/Info.plist")]
 if len(plists)!=1: p.error("Expected exactly one Payload app")
 info=plistlib.loads(z.read(plists[0]));root=plists[0].rsplit("/",1)[0]
 if root+"/"+info["CFBundleExecutable"] not in names:p.error("App executable is missing")
 if info["CFBundleIdentifier"]!="online.usevibefy.mobile":p.error("Unexpected bundle identity")
version={"version":info["CFBundleShortVersionString"],"date":datetime.now(timezone.utc).date().isoformat(),"downloadURL":a.url,"size":a.ipa.stat().st_size,"minOSVersion":info.get("MinimumOSVersion","15.0"),"localizedDescription":"Hybrid mobile preview with system-browser Google authorization and encrypted return to the app. Persistent local playlists and native backup export. Google permissions and verification rules apply; reliable background YouTube playback is not guaranteed."}
source={"name":"Vibefy","identifier":"online.usevibefy.source","apps":[{"name":"Vibefy Preview","bundleIdentifier":info["CFBundleIdentifier"],"developerName":"Spinti1 and contributors","localizedDescription":version["localizedDescription"],"iconURL":"https://raw.githubusercontent.com/Spinti1/Vibefy/main/web/public/assets/icon.png","versions":[version]}],"news":[]}
a.output.write_text(json.dumps(source,indent=2)+"\n",encoding="utf-8")
a.output.with_suffix(".sha256").write_text(hashlib.sha256(a.ipa.read_bytes()).hexdigest()+"  "+a.ipa.name+"\n",encoding="utf-8")
