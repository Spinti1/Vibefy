import asyncio
import json
import sys
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import jam_gateway as module
from fastapi import FastAPI
from fastapi.testclient import TestClient
from starlette.websockets import WebSocketDisconnect

class GatewayTests(unittest.TestCase):
    def setUp(self):
        app = FastAPI()
        app.include_router(module.router)
        self.client = TestClient(app)

    def test_data_reset_requires_origin_and_explicit_action(self):
        for headers in ({}, {"Origin": "https://evil.test", "X-Vibefy-Action": "clear-site-data"}, {"Origin": "https://vibefy-neon.vercel.app"}):
            response = self.client.post("/api/site-data/clear", headers=headers)
            self.assertEqual(response.status_code, 403)
            self.assertNotIn("clear-site-data", response.headers)
        response = self.client.post("/api/site-data/clear", headers={"Origin": "https://vibefy-neon.vercel.app", "X-Vibefy-Action": "clear-site-data"})
        self.assertEqual(response.status_code, 200)
        self.assertEqual(response.headers["clear-site-data"], '"cache", "cookies", "storage"')
        self.assertEqual(response.headers["cache-control"], "no-store")

    def test_invalid_socket_origin_or_code_never_opens_upstream(self):
        with patch.object(module, "connect") as connect:
            for origin, code in (("https://evil.test", "ABCDEFGHJKLM"), ("https://vibefy-neon.vercel.app", "invalid")):
                with self.assertRaises(WebSocketDisconnect) as error:
                    with self.client.websocket_connect("/api/jam/rooms/" + code, headers={"Origin": origin}):
                        pass
                self.assertEqual(error.exception.code, 1008)
            connect.assert_not_called()

    def test_fixed_upstream_preserves_protocol_and_rejects_oversized_frames(self):
        sent = []
        class Relay:
            close_code = None
            async def __aenter__(self):
                self.messages = asyncio.Queue()
                return self
            async def __aexit__(self, *args):
                pass
            async def send(self, text):
                sent.append(json.loads(text))
                await self.messages.put('{"type":"welcome","protocol":2}')
            def __aiter__(self):
                return self.frames()
            async def frames(self):
                while True:
                    yield await self.messages.get()
        with patch.object(module, "connect", return_value=Relay()) as connect:
            with self.client.websocket_connect("/api/jam/rooms/ABCDEFGHJKLM", headers={"Origin": "https://vibefy-neon.vercel.app"}) as ws:
                ws.send_json({"type": "hello", "protocol": 2, "host_token": "a" * 64})
                self.assertEqual(ws.receive_json()["type"], "welcome")
                ws.send_json({"type": "control", "action": "seek", "uri": "vibefy:juicevault:12345678-1234-1234-1234-123456789abc", "position_ms": 42000})
                self.assertEqual(ws.receive_json()["type"], "welcome")
                ws.send_text("a" * 23001)
                with self.assertRaises(WebSocketDisconnect) as error:
                    ws.receive_json()
                self.assertEqual(error.exception.code, 1008)
            self.assertEqual(connect.call_args.args[0], "wss://vibefy-jam.pex-am.workers.dev/rooms/ABCDEFGHJKLM")
            self.assertEqual(connect.call_args.kwargs["max_size"], 64000)
        self.assertEqual(len(sent), 2)
        self.assertEqual(sent[1]["action"], "seek")

if __name__ == "__main__":
    unittest.main()
