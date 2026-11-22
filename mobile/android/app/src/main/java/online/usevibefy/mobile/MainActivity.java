package online.usevibefy.mobile;
import android.app.*;
import android.os.*;
import android.content.*;
import android.net.Uri;
import android.graphics.Color;
import android.view.*;
import android.webkit.*;
import android.widget.*;
import androidx.webkit.WebViewCompat;
import androidx.webkit.WebViewFeature;
import org.json.JSONObject;
import java.io.*;
import java.nio.charset.StandardCharsets;
import java.util.Collections;
import java.util.concurrent.Executors;

public final class MainActivity extends Activity {
 private static final String ORIGIN="https://vibefy-neon.vercel.app",HOME=ORIGIN+"/web";
 private NativeUpdates updates;
 private NearbyReceiver nearby;
 private void joinNearby(String code){if(closed||web==null||!trusted(Uri.parse(web.getUrl()==null?"":web.getUrl())))return;web.evaluateJavascript("window.vibefyJam?.join?.("+JSONObject.quote(code)+")",result->{if(!"true".equals(result)&&!closed)web.loadUrl(HOME+"?jam="+code);});}
 private WebView web;private FrameLayout frame;private LinearLayout cover;private ProgressBar progress;
 private android.webkit.ValueCallback<Uri[]> chooser;private String export;private String script="";private boolean closed;private boolean mainFailed;private boolean rendererGone;
 private final java.util.concurrent.ExecutorService writer=Executors.newSingleThreadExecutor();
 private final java.util.concurrent.ExecutorService accounts=Executors.newSingleThreadExecutor();
 private boolean trusted(Uri uri){return uri!=null&&"https".equals(uri.getScheme())&&"vibefy-neon.vercel.app".equals(uri.getHost())&&uri.getPort()==-1&&uri.getUserInfo()==null;}
 @Override public void onCreate(Bundle state){super.onCreate(state);
  frame=new FrameLayout(this);frame.setBackgroundColor(Color.rgb(14,18,21));web=new WebView(this);web.setBackgroundColor(Color.rgb(14,18,21));frame.addView(web,new FrameLayout.LayoutParams(-1,-1));
  cover=new LinearLayout(this);cover.setOrientation(LinearLayout.VERTICAL);cover.setGravity(Gravity.CENTER);cover.setBackgroundColor(Color.rgb(14,18,21));
  final float density=getResources().getDisplayMetrics().density;
  ImageView logo=new ImageView(this);logo.setImageResource(R.drawable.icon);logo.setContentDescription("Vibefy");cover.addView(logo,new LinearLayout.LayoutParams((int)(88*density),(int)(88*density)));
  TextView title=new TextView(this);title.setText("Vibefy");title.setTextSize(26);title.setTypeface(null,android.graphics.Typeface.BOLD);title.setTextColor(Color.WHITE);LinearLayout.LayoutParams titleLayout=new LinearLayout.LayoutParams(-2,-2);titleLayout.topMargin=(int)(18*density);cover.addView(title,titleLayout);
  TextView loading=new TextView(this);loading.setText("Opening your music…");loading.setTextSize(13);loading.setTextColor(0xff9bafb2);LinearLayout.LayoutParams loadingLayout=new LinearLayout.LayoutParams(-2,-2);loadingLayout.topMargin=(int)(8*density);cover.addView(loading,loadingLayout);
  progress=new ProgressBar(this,null,android.R.attr.progressBarStyleHorizontal);progress.setIndeterminate(true);progress.setIndeterminateTintList(android.content.res.ColorStateList.valueOf(0xff21dfc0));LinearLayout.LayoutParams progressLayout=new LinearLayout.LayoutParams((int)(112*density),(int)(3*density));progressLayout.topMargin=(int)(24*density);cover.addView(progress,progressLayout);frame.addView(cover,new FrameLayout.LayoutParams(-1,-1));setContentView(frame);
  try(InputStream in=getAssets().open("app-shell.js")){ByteArrayOutputStream bytes=new ByteArrayOutputStream();byte[] buffer=new byte[8192];int n;while((n=in.read(buffer))!=-1)bytes.write(buffer,0,n);script=bytes.toString("UTF-8");}catch(IOException ignored){}
  WebSettings settings=web.getSettings();settings.setSupportZoom(false);settings.setBuiltInZoomControls(false);settings.setDisplayZoomControls(false);settings.setJavaScriptEnabled(true);settings.setDomStorageEnabled(true);settings.setAllowFileAccess(false);settings.setAllowContentAccess(false);settings.setMixedContentMode(WebSettings.MIXED_CONTENT_NEVER_ALLOW);settings.setMediaPlaybackRequiresUserGesture(false);settings.setSupportMultipleWindows(true);
  updates=new NativeUpdates(this,writer);script="window.vibefyNativeInfo={platform:'android',version:'0.2.10',secureSession:true,renewableSession:true,audioOnly:true,qrCamera:true};\n"+script;
  web.setOverScrollMode(View.OVER_SCROLL_NEVER);
  if(WebViewFeature.isFeatureSupported(WebViewFeature.DOCUMENT_START_SCRIPT))WebViewCompat.addDocumentStartJavaScript(web,script,Collections.singleton(ORIGIN));
  if(WebViewFeature.isFeatureSupported(WebViewFeature.WEB_MESSAGE_LISTENER))WebViewCompat.addWebMessageListener(web,"VibefyMobile",Collections.singleton(ORIGIN),(view,message,source,main,reply)->{if(!main||!trusted(source))return;try{JSONObject packet=new JSONObject(message.getData());if(handleAudio(packet))return;else if(handleSession(packet))return;else if("checkUpdates".equals(packet.optString("action")))updates.check(true);else if("connectGoogle".equals(packet.optString("action")))openGoogle(packet.optString("url"));else if("exportBackup".equals(packet.optString("action")))exportBackup(packet.optString("content"));}catch(Exception ignored){}});
  web.setWebChromeClient(new WebChromeClient(){
   @Override public void onPermissionRequest(PermissionRequest request){runOnUiThread(()->cameraPermission(request));}
   @Override public void onPermissionRequestCanceled(PermissionRequest request){if(cameraRequest==request)cameraRequest=null;}
   @Override public void onProgressChanged(WebView view,int value){progress.setVisibility(value<100?View.VISIBLE:View.INVISIBLE);}
   @Override public boolean onShowFileChooser(WebView view,android.webkit.ValueCallback<Uri[]> callback,FileChooserParams params){if(chooser!=null)chooser.onReceiveValue(null);chooser=callback;Intent pick=new Intent(Intent.ACTION_OPEN_DOCUMENT).addCategory(Intent.CATEGORY_OPENABLE).setType("application/json");try{startActivityForResult(pick,91);return true;}catch(ActivityNotFoundException e){chooser=null;callback.onReceiveValue(null);return false;}}
   @Override public boolean onCreateWindow(WebView view,boolean dialog,boolean gesture,Message message){if(gesture&&view.getHitTestResult()!=null&&view.getHitTestResult().getExtra()!=null&&view.getHitTestResult().getExtra().startsWith("https://")){try{startActivity(new Intent(Intent.ACTION_VIEW,Uri.parse(view.getHitTestResult().getExtra())));}catch(ActivityNotFoundException ignored){}return false;}return false;}
  });
  web.setWebViewClient(new WebViewClient(){
   @Override public boolean shouldOverrideUrlLoading(WebView v,WebResourceRequest request){if(!request.isForMainFrame())return false;Uri uri=request.getUrl();if(trusted(uri))return false;if("https".equals(uri.getScheme())&&request.hasGesture())try{startActivity(new Intent(Intent.ACTION_VIEW,uri));}catch(ActivityNotFoundException ignored){}return true;}
   @Override public void onPageStarted(WebView v,String url,android.graphics.Bitmap icon){mainFailed=false;}
   @Override public void onReceivedHttpError(WebView v,WebResourceRequest request,WebResourceResponse response){if(request.isForMainFrame()&&response.getStatusCode()>=400){mainFailed=true;offline();}}
   @Override public void onPageCommitVisible(WebView v,String url){if(!mainFailed&&trusted(Uri.parse(url))){dismissCover();inject();}}
   @Override public void onPageFinished(WebView v,String url){if(!mainFailed&&trusted(Uri.parse(url))){dismissCover();inject();}}
   @Override public void onReceivedError(WebView v,WebResourceRequest request,WebResourceError error){if(request.isForMainFrame()){mainFailed=true;offline();}}
   @Override public boolean onRenderProcessGone(WebView v,RenderProcessGoneDetail detail){rendererGone=true;v.destroy();offline();return true;}
  });
  web.loadUrl(jamLink(getIntent().getData()));new Handler(Looper.getMainLooper()).postDelayed(()->{if(!closed)updates.check(false);},1500);
 }
 private PermissionRequest cameraRequest;
 private boolean cameraAllowed(PermissionRequest request){return !closed&&trusted(request.getOrigin())&&trusted(Uri.parse(web.getUrl()==null?"":web.getUrl()))&&request.getResources().length==1&&PermissionRequest.RESOURCE_VIDEO_CAPTURE.equals(request.getResources()[0]);}
 private void cameraPermission(PermissionRequest request){
  if(!cameraAllowed(request)||cameraRequest!=null){request.deny();return;}
  if(checkSelfPermission(android.Manifest.permission.CAMERA)==android.content.pm.PackageManager.PERMISSION_GRANTED){request.grant(new String[]{PermissionRequest.RESOURCE_VIDEO_CAPTURE});return;}
  cameraRequest=request;requestPermissions(new String[]{android.Manifest.permission.CAMERA},93);
 }
 @Override public void onRequestPermissionsResult(int code,String[] permissions,int[] results){
  super.onRequestPermissionsResult(code,permissions,results);if(code!=93)return;
  PermissionRequest request=cameraRequest;cameraRequest=null;if(request==null)return;
  if(results.length>0&&results[0]==android.content.pm.PackageManager.PERMISSION_GRANTED&&cameraAllowed(request))request.grant(new String[]{PermissionRequest.RESOURCE_VIDEO_CAPTURE});else request.deny();
 }
 private boolean handleSession(JSONObject packet){String action=packet.optString("action"),id=packet.optString("id");if(!java.util.Arrays.asList("readSession","saveSession","clearSession").contains(action))return false;if(!id.matches("session-[a-f0-9]{32}-[0-9]{1,12}"))return true;accounts.execute(()->{Object result=JSONObject.NULL;String error=null;try{SecureAccountSession store=new SecureAccountSession(this);if(action.equals("readSession")){JSONObject record=store.load();result=record==null?JSONObject.NULL:record;}else if(action.equals("saveSession")){store.save(packet.getJSONObject("value"));result=true;}else{store.clear();result=true;}}catch(Exception e){error="storage-unavailable";}Object value=result;String failure=error;runOnUiThread(()->{if(!closed&&trusted(Uri.parse(web.getUrl()==null?"":web.getUrl())))web.evaluateJavascript("window.vibefyMobileSessionResult?.("+JSONObject.quote(id)+","+String.valueOf(value)+","+(failure==null?"null":JSONObject.quote(failure))+")",null);});});return true;}
 private final NativeAudioResolver audioResolver=new NativeAudioResolver();
 private final java.util.concurrent.ThreadPoolExecutor audioJobs=new java.util.concurrent.ThreadPoolExecutor(1,1,0,java.util.concurrent.TimeUnit.SECONDS,new java.util.concurrent.ArrayBlockingQueue<>(1),new java.util.concurrent.ThreadPoolExecutor.DiscardOldestPolicy());
 private boolean handleAudio(JSONObject packet){
  if(!"resolveAudio".equals(packet.optString("action")))return false;
  String id=packet.optString("id"),video=packet.optString("video");
  if(!id.matches("audio-[a-f0-9]{32}-[0-9]{1,12}")||!video.matches("[A-Za-z0-9_-]{11}"))return true;
  audioJobs.execute(()->{String url=null;try{url=audioResolver.resolve(video);}catch(Exception ignored){}final String result=url;runOnUiThread(()->{if(!closed&&trusted(Uri.parse(web.getUrl()==null?"":web.getUrl())))web.evaluateJavascript("window.vibefyMobileAudioResult?.("+JSONObject.quote(id)+","+(result==null?"null":JSONObject.quote(result))+","+(result==null?JSONObject.quote("This recording has no available audio-only stream. Try another recording."):"null")+")",null);});});return true;
 }
 private String jamLink(Uri uri){if(uri!=null&&"vibefy".equals(uri.getScheme())&&"playlist".equals(uri.getHost())){String id=uri.getQueryParameter("id");if(id!=null&&id.matches("[a-f0-9]{32}"))return HOME+"#share="+id;}if(uri==null||!"vibefy".equals(uri.getScheme())||!"jam".equals(uri.getHost()))return HOME;String code=uri.getQueryParameter("code");return code!=null&&code.matches("[ABCDEFGHJKLMNPQRSTUVWXYZ23456789]{12}")?HOME+"?jam="+code:HOME;}
 @Override protected void onNewIntent(Intent intent){super.onNewIntent(intent);setIntent(intent);if(!finishGoogle(intent.getData()))web.loadUrl(jamLink(intent.getData()));}
 private String googleState;private long googleStarted;
 private void openGoogle(String raw){try{Uri u=Uri.parse(raw);if(!trusted(u)||!"/mobile-connect.html".equals(u.getPath())||raw.length()>1000)return;Uri fragment=Uri.parse("https://local/?"+u.getFragment());String nonce=fragment.getQueryParameter("state");if(nonce==null||!nonce.matches("[A-Za-z0-9_-]{32}"))return;googleState=nonce;googleStarted=android.os.SystemClock.elapsedRealtime();startActivity(new Intent(Intent.ACTION_VIEW,u).addCategory(Intent.CATEGORY_BROWSABLE));}catch(Exception e){Toast.makeText(this,"No browser is available. Install or enable a browser to connect.",Toast.LENGTH_LONG).show();}}
 private boolean finishGoogle(Uri uri){if(uri==null||!"vibefy".equals(uri.getScheme())||!"connect".equals(uri.getHost()))return false;
  try{if(uri.toString().length()>16000||googleState==null||android.os.SystemClock.elapsedRealtime()-googleStarted>300000)throw new Exception();Uri data=Uri.parse("https://local/?"+uri.getFragment());if(!googleState.equals(data.getQueryParameter("state")))return true;googleState=null;if(trusted(Uri.parse(web.getUrl())))web.evaluateJavascript("window.vibefyMobileAuthResult?.("+JSONObject.quote(uri.toString())+")",null);}catch(Exception e){Toast.makeText(this,"Connection expired. Tap Connect YouTube again.",Toast.LENGTH_LONG).show();}return true;
 }
 private void dismissCover(){if(cover.getVisibility()==View.VISIBLE)cover.animate().alpha(0f).setDuration(160).withEndAction(()->cover.setVisibility(View.GONE)).start();}
 private void inject(){if(trusted(Uri.parse(web.getUrl()==null?HOME:web.getUrl())))web.evaluateJavascript(script,null);}
 private void offline(){cover.animate().cancel();cover.setAlpha(1f);cover.removeAllViews();TextView message=new TextView(this);message.setText("Cannot connect right now. Your saved playlists are kept.");message.setTextColor(Color.WHITE);message.setPadding(28,20,28,20);cover.addView(message);Button retry=new Button(this);retry.setText("Try again");retry.setOnClickListener(v->{if(!closed){if(rendererGone)recreate();else web.loadUrl(HOME);}});cover.addView(retry);cover.setVisibility(View.VISIBLE);progress.setVisibility(View.GONE);}
 private void exportBackup(String raw){if(export!=null||raw.length()>6000000)return;try{JSONObject data=new JSONObject(raw);if(data.optJSONArray("collections")==null||data.optJSONArray("likes")==null)return;}catch(Exception e){return;}export=raw;Intent save=new Intent(Intent.ACTION_CREATE_DOCUMENT).addCategory(Intent.CATEGORY_OPENABLE).setType("application/json").putExtra(Intent.EXTRA_TITLE,"vibefy-library.json");try{startActivityForResult(save,92);}catch(ActivityNotFoundException e){export=null;Toast.makeText(this,"No document provider available",Toast.LENGTH_LONG).show();}}
 @Override protected void onActivityResult(int request,int result,Intent data){super.onActivityResult(request,result,data);if(request==91){if(chooser!=null){chooser.onReceiveValue(result==RESULT_OK&&data!=null&&data.getData()!=null?new Uri[]{data.getData()}:null);chooser=null;}}else if(request==92){String raw=export;export=null;if(result==RESULT_OK&&data!=null&&data.getData()!=null&&raw!=null){Uri target=data.getData();writer.execute(()->{try(OutputStream out=getContentResolver().openOutputStream(target,"wt")){if(out==null)throw new IOException();out.write(raw.getBytes(StandardCharsets.UTF_8));runOnUiThread(()->Toast.makeText(this,"Playlist backup saved",Toast.LENGTH_SHORT).show());}catch(Exception e){runOnUiThread(()->Toast.makeText(this,"Backup could not be saved",Toast.LENGTH_LONG).show());}});}}}
 @Override public void onBackPressed(){if(rendererGone){moveTaskToBack(true);return;}web.evaluateJavascript("(()=>{const d=document.querySelector('dialog[open]');if(d){d.close();return true;}return false;})()",answer->{if("true".equals(answer))return;if(web.canGoBack())web.goBack();else moveTaskToBack(true);});}
 @Override protected void onPause(){if(nearby!=null){nearby.close();nearby=null;}super.onPause();web.onPause();}
 @Override protected void onResume(){super.onResume();if(web!=null)web.onResume();if(updates!=null)updates.resumed();if(nearby==null&&!closed)nearby=new NearbyReceiver(this,this::joinNearby);}
 @Override protected void onDestroy(){closed=true;if(nearby!=null){nearby.close();nearby=null;}if(cameraRequest!=null){cameraRequest.deny();cameraRequest=null;}if(chooser!=null)chooser.onReceiveValue(null);if(web!=null){web.stopLoading();web.destroy();}writer.shutdown();accounts.shutdown();audioJobs.shutdownNow();super.onDestroy();}
}
