import json, subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[2]
out=root/'mobile/ios/Sources/Assets.xcassets/AppIcon.appiconset';out.mkdir(parents=True,exist_ok=True)
subprocess.run(['sips','-z','1024','1024',str(root/'web/public/assets/icon.png'),'--out',str(out/'icon.png')],check=True)
(out/'Contents.json').write_text(json.dumps({'images':[{'filename':'icon.png','idiom':'universal','platform':'ios','size':'1024x1024'}],'info':{'author':'xcode','version':1}}))
