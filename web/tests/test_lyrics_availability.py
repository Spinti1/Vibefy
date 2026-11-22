import sys, unittest
from pathlib import Path
from unittest.mock import patch, AsyncMock
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import app as module
from fastapi import HTTPException
from fastapi.testclient import TestClient

class MissingLyricsClient:
    async def __aenter__(self): return self
    async def __aexit__(self,*args): pass
    async def get(self,*args,**kwargs):
        class Missing:
            status_code=404
            content=b'{}'
            def json(self): return {}
        return Missing()

class LyricsAvailabilityTests(unittest.TestCase):
    def test_successful_empty_providers_confirm_missing_lyrics(self):
        with patch.object(module.httpx,'AsyncClient',return_value=MissingLyricsClient()),patch.object(module,'worker',AsyncMock(return_value=None)):
            result=TestClient(module.app).get('/api/lyrics',params={'title':'Example','artist':'Artist','id':'abcdefghijk'}).json()
        self.assertTrue(result['complete']);self.assertEqual(result['plain'],'');self.assertEqual(result['synced'],'')
    def test_provider_outage_is_not_a_confirmed_miss(self):
        with patch.object(module.httpx,'AsyncClient',return_value=MissingLyricsClient()),patch.object(module,'worker',AsyncMock(side_effect=HTTPException(502,'Temporarily unavailable'))):
            result=TestClient(module.app).get('/api/lyrics',params={'title':'Example','artist':'Artist','id':'abcdefghijk'}).json()
        self.assertFalse(result['complete'])

if __name__=='__main__':unittest.main()
