"""Google code exchange and renewal. Credentials are never logged or persisted here."""
import os
import re
import httpx
from fastapi import APIRouter, Request
from fastapi.responses import JSONResponse

router = APIRouter()
ORIGINS = frozenset({"https://vibefy-neon.vercel.app", "https://usevibefy.online", "https://www.usevibefy.online"})
READ = "https://www.googleapis.com/auth/youtube.readonly"

def configured():
    return bool(os.getenv("VIBEFY_GOOGLE_WEB_CLIENT_SECRET") and re.fullmatch(r"[0-9]+-[A-Za-z0-9_-]+\.apps\.googleusercontent\.com", os.getenv("VIBEFY_GOOGLE_WEB_CLIENT_ID", "")))

def reply(value, status=200):
    return JSONResponse(value, status, headers={"Cache-Control":"no-store", "Pragma":"no-cache"})

@router.post("/api/google-session/{action}")
async def google_session(action: str, request: Request):
    origin = request.headers.get("origin", "")
    if origin not in ORIGINS or request.headers.get("x-vibefy-auth") != "1" or request.headers.get("content-type", "").split(";")[0] != "application/json":
        return reply({"error":"invalid_origin"}, 403)
    if action not in {"exchange", "refresh"}:
        return reply({"error":"invalid_action"}, 404)
    if not configured():
        return reply({"error":"renewal_unavailable"}, 503)
    raw = await request.body()
    if len(raw) > 8192:
        return reply({"error":"invalid_request"}, 413)
    try:
        import json
        body = json.loads(raw)
        credential = body.get("code" if action == "exchange" else "refresh_token")
        if not isinstance(credential, str) or not 1 <= len(credential) <= 4096 or any(ord(c) < 32 for c in credential):
            raise ValueError()
    except (ValueError, AttributeError):
        return reply({"error":"invalid_request"}, 400)
    fields = {"client_id":os.environ["VIBEFY_GOOGLE_WEB_CLIENT_ID"], "client_secret":os.environ["VIBEFY_GOOGLE_WEB_CLIENT_SECRET"]}
    if action == "exchange":
        fields.update(grant_type="authorization_code", code=credential, redirect_uri=origin)
    else:
        fields.update(grant_type="refresh_token", refresh_token=credential)
    try:
        async with httpx.AsyncClient(timeout=15, follow_redirects=False) as client:
            response = await client.post("https://oauth2.googleapis.com/token", data=fields)
        data = response.json()
    except (httpx.HTTPError, ValueError):
        return reply({"error":"temporarily_unavailable"}, 503)
    if response.status_code != 200:
        # Only confirmed revocation is terminal; quota, server and config faults aren't logout.
        invalid = data.get("error") == "invalid_grant"
        return reply({"error":"invalid_grant" if invalid else "temporarily_unavailable"}, 401 if invalid else 503)
    token, ttl = data.get("access_token"), data.get("expires_in")
    if not isinstance(token, str) or not 1 <= len(token) <= 8192 or not isinstance(ttl, (int,float)) or not 5 < ttl <= 86400:
        return reply({"error":"invalid_response"}, 502)
    result = {"access_token":token, "expires_in":ttl}
    if isinstance(data.get("scope"), str): result["scope"] = data["scope"]
    refresh = data.get("refresh_token")
    if isinstance(refresh, str) and 1 <= len(refresh) <= 4096: result["refresh_token"] = refresh
    if action == "exchange" and (READ not in str(data.get("scope", "")).split() or not result.get("refresh_token")):
        return reply({"error":"offline_access_required"}, 409)
    return reply(result)
