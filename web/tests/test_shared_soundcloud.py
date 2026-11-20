import unittest,sys
from pathlib import Path
from unittest.mock import patch
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from fastapi.testclient import TestClient
import app as module
class SharedSoundCloudTests(unittest.TestCase):
 def setUp(self):module.CACHE.clear();module.LIMITS.clear();self.client=TestClient(module.app)
 def test_external_private_and_malformed_urls_never_reach_extractor(self):
  with patch.object(module.yt_dlp,'YoutubeDL') as extractor:
   for url in ['http://soundcloud.com/a/b','https://127.0.0.1/a/b','https://soundcloud.com.evil.example/a/b','https://user:password@soundcloud.com/a/b','https://soundcloud.com/a/b?secret_token=private','https://soundcloud.com/a/b#fragment','https://soundcloud.com/a']:
    self.assertEqual(self.client.get('/api/soundcloud/resolve',params={'url':url}).status_code,400)
   extractor.assert_not_called()
 def test_public_track_resolves_only_metadata_and_reuses_bounded_cache(self):
  with patch.object(module.yt_dlp,'YoutubeDL') as extractor:
   reader=extractor.return_value.__enter__.return_value;reader.extract_info.return_value={'id':'123456','title':'Example','uploader':'Artist','duration':180,'url':'never-return-audio','thumbnail':'https://i1.sndcdn.com/example.jpg','cookies':'never-return-credentials'}
   for _ in range(2):
    r=self.client.get('/api/soundcloud/resolve',params={'url':'https://soundcloud.com/artist/song'});self.assertEqual(r.status_code,200);self.assertEqual(r.json()['track']['id'],'sc:123456');self.assertNotIn('never-return',r.text)
   self.assertEqual(reader.extract_info.call_count,1)
if __name__=='__main__':unittest.main()
