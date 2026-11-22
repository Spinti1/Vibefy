import ast, asyncio, json, re, unittest
from pathlib import Path
from types import SimpleNamespace
class SpotifyIdentityTests(unittest.TestCase):
 def test_public_adapter_retains_recording_identity_for_incremental_sync(self):
  identity='1'*22;track='2'*22
  html='<script id="__NEXT_DATA__">'+json.dumps({'props':{'pageProps':{'state':{'data':{'entity':{'uri':'spotify:playlist:'+identity}},'settings':{'session':{'accessToken':'transient-fixture'}}}}}})+'</script>'
  page={'data':{'playlistV2':{'uri':'spotify:playlist:'+identity,'name':'Example','content':{'items':[{'itemV2':{'data':{'__typename':'Track','uri':'spotify:track:'+track,'name':'Example song'}}}],'totalCount':1,'pagingInfo':{'nextOffset':None}}}}}
  class Response:
   status_code=200
   def __init__(self,content):self.content=content
   async def __aenter__(self):return self
   async def __aexit__(self,*args):pass
   async def aiter_bytes(self):yield self.content.encode()
  class Client:
   def __init__(self,**kw):pass
   async def __aenter__(self):return self
   async def __aexit__(self,*args):pass
   def stream(self,method,url,**kw):return Response(html if method=='GET' else json.dumps(page))
  tree=ast.parse((Path(__file__).resolve().parents[1]/'services.py').read_text(encoding='utf-8'));selected=[n for n in tree.body if isinstance(n,ast.AsyncFunctionDef) and n.name in ['bounded','spotify_page']]
  scope={'json':json,'re':re,'httpx':SimpleNamespace(AsyncClient=Client),'SPOTIFY_QUERY':'fixture','HTTPException':Exception};exec(compile(ast.Module(body=selected,type_ignores=[]),'services.py','exec'),scope)
  result=asyncio.run(scope['spotify_page'](identity,0));self.assertEqual(result['tracks'][0]['id'],'spotify:track:'+track);self.assertIsNone(result['next'])
  page['data']['playlistV2']['content']['items'][0]['itemV2']['data']['uri']='https://untrusted.test/'
  self.assertEqual(asyncio.run(scope['spotify_page'](identity,0))['tracks'][0]['id'],'')
if __name__=='__main__':unittest.main()
