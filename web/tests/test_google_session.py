import os
import sys
import unittest
from pathlib import Path
from unittest.mock import patch, AsyncMock
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from fastapi import FastAPI
from fastapi.testclient import TestClient
import google_session as module

class GoogleSessionTests(unittest.TestCase):
 def setUp(self):
  app=FastAPI(); app.include_router(module.router); self.client=TestClient(app)
  self.headers={"Origin":"https://vibefy-neon.vercel.app","X-Vibefy-Auth":"1"}
  self.env=patch.dict(os.environ,{"VIBEFY_GOOGLE_WEB_CLIENT_ID":"123-test.apps.googleusercontent.com","VIBEFY_GOOGLE_WEB_CLIENT_SECRET":"server-only-secret"});self.env.start();self.addCleanup(self.env.stop)
 def post(self, data, headers=None): return self.client.post("/api/google-session/refresh",json=data,headers=self.headers if headers is None else headers)
 def test_rejects_cross_origin_and_missing_csrf_header_before_network(self):
  with patch.object(module.httpx.AsyncClient,"post",new_callable=AsyncMock) as network:
   for headers in [{},{"Origin":"https://evil.example","X-Vibefy-Auth":"1"},{"Origin":"https://vibefy-neon.vercel.app"}]: self.assertEqual(self.post({"refresh_token":"private"},headers).status_code,403)
   network.assert_not_called()
 def test_invalid_input_does_not_reach_google(self):
  for value in ["",[],"x"*4097,"bad\nvalue"]: self.assertEqual(self.post({"refresh_token":value}).status_code,400)
 def response(self,status,data):
  from httpx import Response
  return Response(status,json=data)
 def test_refresh_returns_only_needed_fields_and_binds_client(self):
  with patch.object(module.httpx.AsyncClient,"post",new_callable=AsyncMock,return_value=self.response(200,{"access_token":"new-access","expires_in":3600,"id_token":"not-returned"})) as network:
   r=self.post({"refresh_token":"private-refresh"});self.assertEqual(r.json(),{"access_token":"new-access","expires_in":3600});self.assertEqual(r.headers["cache-control"],"no-store")
   self.assertEqual(network.call_args.args[0],"https://oauth2.googleapis.com/token");self.assertEqual(network.call_args.kwargs["data"]["client_secret"],"server-only-secret")
 def test_only_invalid_grant_is_terminal(self):
  for error,status in [("invalid_grant",401),("invalid_client",503),("temporarily_unavailable",503)]:
   with patch.object(module.httpx.AsyncClient,"post",new_callable=AsyncMock,return_value=self.response(400,{"error":error,"error_description":"private details"})):
    r=self.post({"refresh_token":"private"});self.assertEqual(r.status_code,status);self.assertNotIn("private details",r.text)
 def test_exchange_requires_refresh_and_read_permission(self):
  data={"access_token":"access","expires_in":3600,"scope":module.READ}
  with patch.object(module.httpx.AsyncClient,"post",new_callable=AsyncMock,return_value=self.response(200,data)):
   self.assertEqual(self.client.post("/api/google-session/exchange",json={"code":"code"},headers=self.headers).status_code,409)
  data["refresh_token"]="refresh"
  with patch.object(module.httpx.AsyncClient,"post",new_callable=AsyncMock,return_value=self.response(200,data)) as network:
   r=self.client.post("/api/google-session/exchange",json={"code":"code"},headers=self.headers);self.assertEqual(r.status_code,200);self.assertEqual(network.call_args.kwargs["data"]["redirect_uri"],"https://vibefy-neon.vercel.app")
if __name__=="__main__": unittest.main()
