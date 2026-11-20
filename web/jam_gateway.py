"""Fixed-upstream Jam gateway. Room state remains shared with desktop clients."""
import asyncio
import json
import re
import time

import httpx
from fastapi import APIRouter, HTTPException, Request, WebSocket, WebSocketDisconnect
from fastapi.responses import JSONResponse
from websockets.asyncio.client import connect
from websockets.exceptions import ConnectionClosed, WebSocketException

router = APIRouter()
ORIGINS = {"https://usevibefy.online", "https://www.usevibefy.online", "https://vibefy-neon.vercel.app"}
UPSTREAM = "https://vibefy-jam.pex-am.workers.dev"
CODE = re.compile(r"^[A-HJ-NP-Z2-9]{12}$")

def allowed_origin(origin):
    return origin is None or origin in ORIGINS

@router.get("/api/jam/health")
async def jam_health():
    return {"service": "Vibefy Jam gateway", "protocol": 2, "roomStorage": "shared relay"}

@router.post("/api/site-data/clear")
async def clear_site_data(request: Request):
    if request.headers.get("origin") not in ORIGINS or request.headers.get("x-vibefy-action") != "clear-site-data":
        raise HTTPException(403, "Use the confirmed data-reset action in Vibefy Settings.")
    return JSONResponse({"cleared": True}, headers={"Clear-Site-Data": '"cache", "cookies", "storage"', "Cache-Control": "no-store"})

@router.post("/api/jam/rooms")
async def create_room(request: Request):
    if not allowed_origin(request.headers.get("origin")):
        raise HTTPException(403, "Origin not allowed.")
    try:
        async with httpx.AsyncClient(timeout=12, follow_redirects=False) as client:
            response = await client.post(UPSTREAM + "/rooms", headers={"Origin": "https://vibefy-neon.vercel.app"})
        if response.status_code == 429:
            raise HTTPException(429, "Jam is busy. Please retry shortly.")
        if response.status_code != 200 or len(response.content) > 4096:
            raise ValueError()
        data = response.json()
        if not CODE.fullmatch(str(data.get("code", ""))) or not re.fullmatch(r"[a-f0-9]{64}", str(data.get("host_token", ""))):
            raise ValueError()
        return JSONResponse({"code": data["code"], "host_token": data["host_token"], "expires_at": data.get("expires_at")}, headers={"Cache-Control": "no-store"})
    except HTTPException:
        raise
    except (httpx.HTTPError, ValueError, TypeError):
        raise HTTPException(502, "The Jam service could not answer. Try again shortly.") from None

@router.websocket("/api/jam/rooms/{code}")
async def room_socket(socket: WebSocket, code: str):
    if not allowed_origin(socket.headers.get("origin")) or not CODE.fullmatch(code):
        await socket.close(code=1008)
        return
    accepted = False
    close_code = 1012
    try:
        async with connect(UPSTREAM.replace("https:", "wss:") + "/rooms/" + code,
                           origin="https://vibefy-neon.vercel.app", open_timeout=10,
                           close_timeout=3, max_size=64000, max_queue=8, compression=None) as upstream:
            await socket.accept()
            accepted = True
            async def to_relay():
                window = time.monotonic()
                count = 0
                while True:
                    packet = await socket.receive()
                    if packet["type"] == "websocket.disconnect":
                        return
                    text = packet.get("text")
                    if not isinstance(text, str) or len(text.encode("utf-8")) > 23000:
                        await socket.close(code=1008)
                        return
                    now = time.monotonic()
                    if now - window >= 10:
                        window, count = now, 0
                    count += 1
                    if count > 100:
                        await socket.close(code=1008)
                        return
                    try:
                        payload = json.loads(text)
                        if not isinstance(payload, dict) or payload.get("type") not in {"hello", "state", "ping", "add", "control", "end"}:
                            raise ValueError()
                    except (ValueError, TypeError):
                        await socket.close(code=1008)
                        return
                    await upstream.send(text)
            async def to_browser():
                async for text in upstream:
                    if not isinstance(text, str):
                        raise ValueError()
                    await socket.send_text(text)
            tasks = [asyncio.create_task(to_relay()), asyncio.create_task(to_browser())]
            try:
                # Rotate before the Function's 300-second limit; clients retain the
                # host credential and reconnect to the same durable room.
                done, _ = await asyncio.wait(tasks, timeout=265, return_when=asyncio.FIRST_COMPLETED)
                for task in done:
                    task.result()
                if upstream.close_code in (1000, 1008):
                    close_code = upstream.close_code
            finally:
                for task in tasks:
                    task.cancel()
                await asyncio.gather(*tasks, return_exceptions=True)
    except ConnectionClosed as error:
        close_code = error.code if error.code in (1000, 1008) else 1012
    except (OSError, TimeoutError, WebSocketDisconnect, WebSocketException, ValueError, RuntimeError):
        close_code = 1012
    finally:
        if accepted:
            try:
                await socket.close(code=close_code)
            except RuntimeError:
                pass
        else:
            await socket.close(code=1013)
