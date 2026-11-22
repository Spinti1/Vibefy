import ast, asyncio, re, unittest
from pathlib import Path

class HTTPException(Exception):
    def __init__(self, status, message): self.status = status
class Query:
    def __init__(self, **kw): pass
class App:
    def get(self, *args): return lambda fn: fn

class RecordingArtistTests(unittest.TestCase):
    def load(self, rows):
        calls = []
        class Music:
            def __init__(self, **kw): pass
            def get_watch_playlist(self, **kw): calls.append(kw); return {'tracks': rows}
        async def worker(fn): return fn()
        tree = ast.parse((Path(__file__).resolve().parents[1] / 'app.py').read_text(encoding='utf-8'))
        fn = next(n for n in tree.body if isinstance(n, ast.AsyncFunctionDef) and n.name == 'recording_artist')
        scope = dict(app=App(), Query=Query, VIDEO=re.compile(r'^[A-Za-z0-9_-]{11}$'), re=re, HTTPException=HTTPException, YTMusic=Music, worker=worker, cached=lambda key, ttl, fn: fn())
        exec(compile(ast.Module(body=[fn], type_ignores=[]), '<recording_artist>', 'exec'), scope)
        return scope['recording_artist'], calls
    def test_artist_comes_from_the_selected_recording(self):
        artist = {'id':'UC'+'a'*22,'name':'Example Band'}
        fn, calls = self.load([{'videoId':'abcdefghijk','artists':[artist]}])
        self.assertEqual(asyncio.run(fn('abcdefghijk')), artist)
        self.assertEqual(calls, [{'videoId':'abcdefghijk', 'limit':1}])
    def test_related_recordings_cannot_supply_a_wrong_artist(self):
        fn, _ = self.load([{'videoId':'lmnopqrstuv','artists':[{'id':'UC'+'a'*22}]}])
        with self.assertRaises(HTTPException) as failure: asyncio.run(fn('abcdefghijk'))
        self.assertEqual(failure.exception.status, 404)

if __name__ == '__main__': unittest.main()
