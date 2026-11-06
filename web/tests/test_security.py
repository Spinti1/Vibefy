import asyncio
import sys
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import app as module
from fastapi.testclient import TestClient

class SecurityTests(unittest.TestCase):
    def setUp(self):
        module.LIMITS.clear()
        self.client = TestClient(module.app)

    def test_arbitrary_audio_urls_never_reach_extractor(self):
        with patch.object(module, "audio_url") as extract:
            for path in ["https:evil", "127.0.0.1", "abcdefghij", "abcdefghijkl"]:
                self.assertEqual(self.client.get("/api/audio/" + path).status_code, 400)
            extract.assert_not_called()

    def test_google_config_exposes_only_valid_public_id(self):
        identity = "123456-demo.apps.googleusercontent.com"
        with patch.dict(module.os.environ, {"VIBEFY_GOOGLE_WEB_CLIENT_ID": identity, "GOOGLE_CLIENT_SECRET": "never-public"}):
            r = self.client.get("/api/google-config")
            self.assertEqual(r.json(), {"clientId": identity})
            self.assertNotIn("never-public", r.text)
            self.assertEqual(r.headers["cache-control"], "no-store")
        with patch.dict(module.os.environ, {"VIBEFY_GOOGLE_WEB_CLIENT_ID": "invalid"}):
            self.assertEqual(self.client.get("/api/google-config").json()["clientId"], "")

    def test_new_source_paths_validate_ids_before_network(self):
        for identity in ["https://127.0.0.1", "../private", "a" * 22]:
            if identity == "a" * 22:
                continue
            self.assertEqual(self.client.get("/api/import/spotify", params={"id": identity}).status_code, 400)
            self.assertEqual(self.client.get("/api/juice/playlist", params={"id": identity}).status_code, 400)

    def test_matching_preserves_artist_and_recording_version(self):
        source = {"title": "Original Song", "artist": "Artist", "duration": 200}
        good = {**source, "id": "abcdefghijk"}
        self.assertGreater(module.recording_score(source, good), .9)
        for change in [{"title": "Original Song Clean"}, {"title": "Original Song Live"}, {"artist": "Someone else"}, {"duration": 280}]:
            self.assertEqual(module.recording_score(source, {**good, **change}), 0)

    def test_explicit_player_routes_are_served(self):
        for route in ["/web", "/phone", "/app"]:
            response = self.client.get(route)
            self.assertEqual(response.status_code, 200)
            self.assertIn('id="home-view"', response.text)

    def test_login_payloads_are_bounded(self):
        response = self.client.post("/api/juice/login", json={"username": "x", "password": "x" * 70000})
        self.assertEqual(response.status_code, 413)

    def test_ranges_are_validated_before_network(self):
        with patch.object(module, "audio_url") as extract:
            for value in ["bytes=-100", "bytes=0-1,2-3", "bytes=100-1", "bytes=1234567890123-"]:
                self.assertEqual(self.client.get("/api/audio/abcdefghijk", headers={"Range": value}).status_code, 416)
            extract.assert_not_called()

    def test_playlist_urls_cannot_be_used_as_ssrf_targets(self):
        for identity in ["https://127.0.0.1/secret", "../../etc/passwd", "http://metadata.google.internal"]:
            self.assertEqual(self.client.get("/api/playlist", params={"id": identity}).status_code, 400)

    def test_thumbnail_hosts_are_exact_and_https(self):
        for uri in ["javascript:alert(1)", "http://i.ytimg.com/x", "https://i.ytimg.com.evil.test/x", "https://user@i.ytimg.com/x", "https://i.ytimg.com:8443/x"]:
            self.assertEqual(module.safe_art(uri), "")
        self.assertEqual(module.safe_art("https://i.ytimg.com/vi/abcdefghijk/hqdefault.jpg"), "https://i.ytimg.com/vi/abcdefghijk/hqdefault.jpg")

    def test_music_videos_and_unclassified_rows_are_omitted(self):
        for kind in [None, "MUSIC_VIDEO_TYPE_OMV", "MUSIC_VIDEO_TYPE_UGC"]:
            self.assertIsNone(module.track({"videoId":"abcdefghijk", "videoType":kind}))
        self.assertIsNotNone(module.track({"videoId":"abcdefghijk", "videoType":"MUSIC_VIDEO_TYPE_ATV"}))

    def test_metadata_is_data_not_html(self):
        t = module.track({"videoId":"abcdefghijk", "videoType":"MUSIC_VIDEO_TYPE_ATV", "title":"<script>alert(1)</script>"})
        self.assertEqual(t["title"], "<script>alert(1)</script>")
        source = (Path(module.ROOT)/"public"/"player.js").read_text(encoding="utf-8")
        self.assertNotIn("innerHTML", source)

    def test_operational_errors_do_not_disclose_stack_or_source_url(self):
        with patch.object(module, "audio_url", side_effect=RuntimeError("secret source URL")):
            r = self.client.get("/api/audio/abcdefghijk")
            self.assertEqual(r.status_code, 502)
            self.assertNotIn("secret source", r.text)
            self.assertEqual(r.headers["x-content-type-options"], "nosniff")

    def test_busy_workers_fail_bounded(self):
        for _ in range(4): module.SLOTS.acquire()
        try:
            async def run():
                with self.assertRaises(module.HTTPException) as error:
                    await module.worker(lambda: None)
                self.assertEqual(error.exception.status_code, 503)
            asyncio.run(run())
        finally:
            for _ in range(4): module.SLOTS.release()

    def test_burst_limit_returns_retry_after(self):
        for _ in range(80): self.client.get("/api/capabilities")
        r = self.client.get("/api/capabilities")
        self.assertEqual(r.status_code, 429)
        self.assertEqual(r.headers["retry-after"], "60")

if __name__ == "__main__": unittest.main()
