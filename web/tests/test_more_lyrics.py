import sys
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import app as module


class MoreLyricsTests(unittest.IsolatedAsyncioTestCase):
    async def lookup(self, payload, status=200):
        class Client:
            async def __aenter__(self): return self
            async def __aexit__(self, *args): pass
            async def get(self, *args, **kwargs):
                class Response:
                    status_code = status
                    content = b'{}'
                    def json(self): return payload
                return Response()
        with patch.object(module.httpx, 'AsyncClient', return_value=Client()):
            return await module.more_lyrics('Example', 'Artist', 200)

    async def test_verified_timing_and_provider_are_preserved(self):
        result = await self.lookup({'metadata': {'trackName': 'Example', 'artistName': 'Artist'}, 'provider': 'Demo provider', 'isSynced': True, 'lines': [{'words': 'Synthetic test line', 'startTimeMs': '1250'}]})
        self.assertEqual(result['synced'], '[00:01.25]Synthetic test line')
        self.assertEqual(result['source'], 'Demo provider via More Lyrics')

    async def test_missing_or_wrong_metadata_never_displays_random_lyrics(self):
        for metadata in [{}, {'trackName': 'Different recording', 'artistName': 'Other artist'}]:
            self.assertIsNone(await self.lookup({'metadata': metadata, 'lines': [{'words': 'Synthetic test'}]}))

    async def test_unavailable_provider_does_not_break_other_sources(self):
        for status in [403, 404, 429, 503]:
            self.assertIsNone(await self.lookup({}, status))

    async def test_plain_text_never_gets_invented_timing(self):
        result = await self.lookup({'track': 'Example', 'artist': 'Artist', 'lines': [{'words': 'Synthetic plain line', 'startTimeMs': '9000'}]})
        self.assertEqual(result['synced'], '')
        self.assertEqual(result['plain'], 'Synthetic plain line')
