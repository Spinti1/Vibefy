import ast,asyncio,re,unittest
from pathlib import Path
class Query:
 def __init__(self,**kw):self.default=kw.get('default')
class HTTPException(Exception):pass
class Router:
 def get(self,*a):return lambda fn:fn
class SearchTests(unittest.TestCase):
 def scope(self,fail=False):
  calls=[]
  class API:
   def __init__(self,**kw):pass
   def search(self,q,filter=None,limit=40):
    calls.append((q,filter))
    if fail and filter is None:raise HTTPException('primary down')
    if filter=='artists':return [{'resultType':'artist','browseId':'UC'+'a'*22,'artist':'Example Band'}]
    if filter=='videos':return [{'resultType':'video','videoId':'b'*11,'title':'Exact song','artists':[]},{'resultType':'video','videoId':'a'*11,'title':'Other song','artists':[]}]
    return [{'resultType':'song','videoId':'a'*11,'title':'Other song','artists':[]},{'resultType':'artist','browseId':'UC'+'a'*22,'artist':'Example Band'},{'resultType':'artist','browseId':'https://evil','artist':'Bad'}]
  async def worker(fn):return fn()
  tree=ast.parse((Path(__file__).resolve().parents[1]/'app.py').read_text(encoding='utf-8'));functions=[n for n in tree.body if isinstance(n,(ast.FunctionDef,ast.AsyncFunctionDef)) and n.name in ['search','track','safe_art']]
  from urllib.parse import urlparse
  scope={'app':Router(),'Query':Query,'HTTPException':HTTPException,'asyncio':asyncio,'re':re,'VIDEO':re.compile(r'^[A-Za-z0-9_-]{11}$'),'urlparse':urlparse,'YTMusic':API,'worker':worker,'cached':lambda k,t,f:f()}
  exec(compile(ast.Module(body=functions,type_ignores=[]),'search-production','exec'),scope);return scope,calls
 def test_all_merges_clips_deduplicates_ranks_and_keeps_artists(self):
  s,c=self.scope();r=asyncio.run(s['search'](' Exact   song ','all'));self.assertEqual([t['id']for t in r['tracks']],['b'*11,'a'*11]);self.assertEqual(len(r['artists']),1);self.assertEqual(c,[('Exact song',None),('Exact song','videos')])
 def test_artist_category_has_no_playable_tracks(self):
  s,c=self.scope();r=asyncio.run(s['search']('band','artists'));self.assertEqual(r['tracks'],[]);self.assertEqual(r['artists'][0]['name'],'Example Band');self.assertEqual(c,[('band','artists')])
 def test_primary_failure_can_use_clip_results(self):
  s,_=self.scope(True);r=asyncio.run(s['search']('song','all'));self.assertEqual(len(r['tracks']),2)
 def test_single_category_uses_only_one_catalogue_request(self):
  s,c=self.scope();asyncio.run(s['search']('song','songs'));self.assertEqual(c,[('song','songs')])
if __name__=='__main__':unittest.main()
