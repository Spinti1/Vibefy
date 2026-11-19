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
 private WebView web;private FrameLayout frame;private LinearLayout cover;private ProgressBar progress;
 private android.webkit.ValueCallback<Uri[]> chooser;private String export;private String script="";private boolean closed;private boolean mainFailed;private boolean rendererGone;
 private final java.util.concurrent.ExecutorService writer=Executors.newSingleThreadExecutor();
 private final java.util.concurrent.ExecutorService accounts=Executors.newSingleThreadExecutor();
 private boolean trusted(Uri uri){return uri!=null&&"https".equals(uri.getScheme())&&"vibefy-neon.vercel.app".equals(uri.getHost())&&uri.getPort()==-1&&uri.getUserInfo()==null;}
 @Override public void onCreate(Bundle state){super.onCreate(state);
  frame=new FrameLayout(this);frame.setBackgroundColor(Color.rgb(14,18,21));web=new WebView(this);web.setBackgroundColor(Color.rgb(14,18,21));frame.addView(web,new FrameLayout.LayoutParams(-1,-1));
  cover=new LinearLayout(this);cover.setOrientation(LinearLayout.VERTICAL);cover.setGravity(Gravity.CENTER);cover.setBackgroundColor(Color.rgb(14,18,21));
  ImageView logo=new ImageView(this);logo.setImageResource(R.drawable.icon);cover.addView(logo,new LinearLayout.LayoutParams(120,120));TextView title=new TextView(this);title.setText("Vibefy");title.setTextSize(30);title.setTextColor(Color.WHITE);cover.addView(title);frame.addView(cover,new FrameLayout.LayoutParams(-1,-1));
  progress=new ProgressBar(this,null,android.R.attr.progressBarStyleHorizontal);progress.setIndeterminateTintList(android.content.res.ColorStateList.valueOf(0xff00d4bf));FrameLayout.LayoutParams progressLayout=new FrameLayout.LayoutParams(-1,4,Gravity.TOP);frame.addView(progress,progressLayout);setContentView(frame);
  try(InputStream in=getAssets().open("app-shell.js")){ByteArrayOutputStream bytes=new ByteArrayOutputStream();byte[] buffer=new byte[8192];int n;while((n=in.read(buffer))!=-1)bytes.write(buffer,0,n);script=bytes.toString("UTF-8");}catch(IOException ignored){}
  WebSettings settings=web.getSettings();settings.setJavaScriptEnabled(true);settings.setDomStorageEnabled(true);settings.setAllowFileAccess(false);settings.setAllowContentAccess(false);settings.setMixedContentMode(WebSettings.MIXED_CONTENT_NEVER_ALLOW);settings.setMediaPlaybackRequiresUserGesture(true);settings.setSupportMultipleWindows(true);
  updates=new NativeUpdates(this,writer);script="window.vibefyNativeInfo={platform:'android',version:'0.2.4',secureSession:true};\n"+script;
  web.setOverScrollMode(View.OVER_SCROLL_NEVER);
  if(WebViewFeature.isFeatureSupported(WebViewFeature.DOCUMENT_START_SCRIPT))WebViewCompat.addDocumentStartJavaScript(web,script,Collections.singleton(ORIGIN));
  if(WebViewFeature.isFeatureSupported(WebViewFeature.WEB_MESSAGE_LISTENER))WebViewCompat.addWebMessageListener(web,"VibefyMobile",Collections.singleton(ORIGIN),(view,message,source,main,reply)->{if(!main||!trusted(source))return;try{JSONObject packet=new JSONObject(message.getData());if(handleSession(packet))return;else if("checkUpdates".equals(packet.optString("action")))updates.check(true);else if("connectGoogle".equals(packet.optString("action")))openGoogle(packet.optString("url"));else if("exportBackup".equals(packet.optString("action")))exportBackup(packet.optString("content"));}catch(Exception ignored){}});
  web.setWebChromeClient(new WebChromeClient(){
   @Override public void onProgressChanged(WebView view,int value){progress.setVisibility(value<100?View.VISIBLE:View.GONE);progress.setProgress(value);}
   @Override public boolean onShowFileChooser(WebView view,android.webkit.ValueCallback<Uri[]> callback,FileChooserParams params){if(chooser!=null)chooser.onReceiveValue(null);chooser=callback;Intent pick=new Intent(Intent.ACTION_OPEN_DOCUMENT).addCategory(Intent.CATEGORY_OPENABLE).setType("application/json");try{startActivityForResult(pick,91);return true;}catch(ActivityNotFoundException e){chooser=null;callback.onReceiveValue(null);return false;}}
   @Override public boolean onCreateWindow(WebView view,boolean dialog,boolean gesture,Message message){if(gesture&&view.getHitTestResult()!=null&&view.getHitTestResult().getExtra()!=null&&view.getHitTestResult().getExtra().startsWith("https://")){try{startActivity(new Intent(Intent.ACTION_VIEW,Uri.parse(view.getHitTestResult().getExtra())));}catch(ActivityNotFoundException ignored){}return false;}return false;}
  });
  web.setWebViewClient(new WebViewClient(){
   @Override public boolean shouldOverrideUrlLoading(WebView v,WebResourceRequest request){if(!request.isForMainFrame())return false;Uri uri=request.getUrl();if(trusted(uri))return false;if("https".equals(uri.getScheme())&&request.hasGesture())try{startActivity(new Intent(Intent.ACTION_VIEW,uri));}catch(ActivityNotFoundException ignored){}return true;}
   @Override public void onPageStarted(WebView v,String url,android.graphics.Bitmap icon){mainFailed=false;}
   @Override public void onReceivedHttpError(WebView v,WebResourceRequest request,WebResourceResponse response){if(request.isForMainFrame()&&response.getStatusCode()>=400){mainFailed=true;offline();}}
   @Override public void onPageCommitVisible(WebView v,String url){if(!mainFailed&&trusted(Uri.parse(url))){cover.setVisibility(View.GONE);inject();}}
   @Override public void onPageFinished(WebView v,String url){if(!mainFailed&&trusted(Uri.parse(url))){cover.setVisibility(View.GONE);inject();}}
   @Override public void onReceivedError(WebView v,WebResourceRequest request,WebResourceError error){if(request.isForMainFrame()){mainFailed=true;offline();}}
   @Override public boolean onRenderProcessGone(WebView v,RenderProcessGoneDetail detail){rendererGone=true;v.destroy();offline();return true;}
  });
  web.loadUrl(jamLink(getIntent().getData()));new Handler(Looper.getMainLooper()).postDelayed(()->{if(!closed)updates.check(false);},1500);
 }
 private boolean handleSession(JSONObject packet){String action=packet.optString("action"),id=packet.optString("id");if(!java.util.Arrays.asList("readSession","saveSession","clearSession").contains(action))return false;if(!id.matches("session-[a-f0-9]{32}-[0-9]{1,12}"))return true;accounts.execute(()->{Object result=JSONObject.NULL;String error=null;try{SecureAccountSession store=new SecureAccountSession(this);if(action.equals("readSession")){JSONObject record=store.load();result=record==null?JSONObject.NULL:record;}else if(action.equals("saveSession")){store.save(packet.getJSONObject("value"));result=true;}else{store.clear();result=true;}}catch(Exception e){error="storage-unavailable";}Object value=result;String failure=error;runOnUiThread(()->{if(!closed&&trusted(Uri.parse(web.getUrl()==null?"":web.getUrl())))web.evaluateJavascript("window.vibefyMobileSessionResult?.("+JSONObject.quote(id)+","+String.valueOf(value)+","+(failure==null?"null":JSONObject.quote(failure))+")",null);});});return true;}
 private String jamLink(Uri uri){if(uri==null||!"vibefy".equals(uri.getScheme())||!"jam".equals(uri.getHost()))return HOME;String code=uri.getQueryParameter("code");return code!=null&&code.matches("[ABCDEFGHJKLMNPQRSTUVWXYZ23456789]{12}")?HOME+"?jam="+code:HOME;}
 @Override protected void onNewIntent(Intent intent){super.onNewIntent(intent);setIntent(intent);if(!finishGoogle(intent.getData()))web.loadUrl(jamLink(intent.getData()));}
 private String googleState;private long googleStarted;
 private void openGoogle(String raw){try{Uri u=Uri.parse(raw);if(!trusted(u)||!"/mobile-connect.html".equals(u.getPath())||raw.length()>1000)return;Uri fragment=Uri.parse("https://local/?"+u.getFragment());String nonce=fragment.getQueryParameter("state");if(nonce==null||!nonce.matches("[A-Za-z0-9_-]{32}"))return;googleState=nonce;googleStarted=android.os.SystemClock.elapsedRealtime();startActivity(new Intent(Intent.ACTION_VIEW,u).addCategory(Intent.CATEGORY_BROWSABLE));}catch(Exception e){Toast.makeText(this,"No browser is available. Install or enable a browser to connect.",Toast.LENGTH_LONG).show();}}
 private boolean finishGoogle(Uri uri){if(uri==null||!"vibefy".equals(uri.getScheme())||!"connect".equals(uri.getHost()))return false;
  try{if(uri.toString().length()>16000||googleState==null||android.os.SystemClock.elapsedRealtime()-googleStarted>300000)throw new Exception();Uri data=Uri.parse("https://local/?"+uri.getFragment());if(!googleState.equals(data.getQueryParameter("state")))return true;googleState=null;if(trusted(Uri.parse(web.getUrl())))web.evaluateJavascript("window.vibefyMobileAuthResult?.("+JSONObject.quote(uri.toString())+")",null);}catch(Exception e){Toast.makeText(this,"Connection expired. Tap Connect YouTube again.",Toast.LENGTH_LONG).show();}return true;
 }
 private void inject(){if(trusted(Uri.parse(web.getUrl()==null?HOME:web.getUrl())))web.evaluateJavascript(script,null);}
 private void offline(){cover.removeAllViews();TextView message=new TextView(this);message.setText("Cannot connect right now. Your saved playlists are kept.");message.setTextColor(Color.WHITE);message.setPadding(28,20,28,20);cover.addView(message);Button retry=new Button(this);retry.setText("Try again");retry.setOnClickListener(v->{if(!closed){if(rendererGone)recreate();else web.loadUrl(HOME);}});cover.addView(retry);cover.setVisibility(View.VISIBLE);progress.setVisibility(View.GONE);}
 private void exportBackup(String raw){if(export!=null||raw.length()>6000000)return;try{JSONObject data=new JSONObject(raw);if(data.optJSONArray("collections")==null||data.optJSONArray("likes")==null)return;}catch(Exception e){return;}export=raw;Intent save=new Intent(Intent.ACTION_CREATE_DOCUMENT).addCategory(Intent.CATEGORY_OPENABLE).setType("application/json").putExtra(Intent.EXTRA_TITLE,"vibefy-library.json");try{startActivityForResult(save,92);}catch(ActivityNotFoundException e){export=null;Toast.makeText(this,"No document provider available",Toast.LENGTH_LONG).show();}}
 @Override protected void onActivityResult(int request,int result,Intent data){super.onActivityResult(request,result,data);if(request==91){if(chooser!=null){chooser.onReceiveValue(result==RESULT_OK&&data!=null&&data.getData()!=null?new Uri[]{data.getData()}:null);chooser=null;}}else if(request==92){String raw=export;export=null;if(result==RESULT_OK&&data!=null&&data.getData()!=null&&raw!=null){Uri target=data.getData();writer.execute(()->{try(OutputStream out=getContentResolver().openOutputStream(target,"wt")){if(out==null)throw new IOException();out.write(raw.getBytes(StandardCharsets.UTF_8));runOnUiThread(()->Toast.makeText(this,"Playlist backup saved",Toast.LENGTH_SHORT).show());}catch(Exception e){runOnUiThread(()->Toast.makeText(this,"Backup could not be saved",Toast.LENGTH_LONG).show());}});}}}
 @Override public void onBackPressed(){if(rendererGone){moveTaskToBack(true);return;}web.evaluateJavascript("(()=>{const d=document.querySelector('dialog[open]');if(d){d.close();return true;}return false;})()",answer->{if("true".equals(answer))return;if(web.canGoBack())web.goBack();else moveTaskToBack(true);});}
 @Override protected void onPause(){super.onPause();web.onPause();}
 @Override protected void onResume(){super.onResume();if(web!=null)web.onResume();if(updates!=null)updates.resumed();}
 @Override protected void onDestroy(){closed=true;if(chooser!=null)chooser.onReceiveValue(null);if(web!=null){web.stopLoading();web.destroy();}writer.shutdown();accounts.shutdown();super.onDestroy();}
}
