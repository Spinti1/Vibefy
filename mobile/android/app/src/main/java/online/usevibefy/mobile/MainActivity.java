package online.usevibefy.mobile;

import android.app.*;
import android.os.*;
import android.content.*;
import android.net.Uri;
import android.graphics.Color;
import android.view.*;
import android.webkit.*;
import android.widget.*;

public final class MainActivity extends Activity {
 private static final String HOME="https://vibefy-neon.vercel.app/web";
 private WebView web;
 private boolean failed;
 private static boolean internal(Uri uri) {
  return "https".equals(uri.getScheme()) && "vibefy-neon.vercel.app".equals(uri.getHost());
 }
 private void external(Uri uri) {
  if(!"https".equals(uri.getScheme())) return;
  try {startActivity(new Intent(Intent.ACTION_VIEW,uri));}
  catch(ActivityNotFoundException e) {Toast.makeText(this,"No browser available",Toast.LENGTH_SHORT).show();}
 }
 @Override public void onCreate(Bundle state) {
  super.onCreate(state);
  LinearLayout layout=new LinearLayout(this);layout.setOrientation(LinearLayout.VERTICAL);layout.setBackgroundColor(Color.rgb(16,20,18));
  LinearLayout toolbar=new LinearLayout(this);
  Button reload=new Button(this);reload.setText(R.string.reload);reload.setOnClickListener(v->web.loadUrl(HOME));toolbar.addView(reload);
  Button browser=new Button(this);browser.setText(R.string.open_browser);browser.setOnClickListener(v->external(Uri.parse(web.getUrl()!=null&&internal(Uri.parse(web.getUrl()))?web.getUrl():HOME)));toolbar.addView(browser);
  layout.addView(toolbar);
  web=new WebView(this);layout.addView(web,new LinearLayout.LayoutParams(-1,0,1));setContentView(layout);
  web.setBackgroundColor(Color.rgb(16,20,18));
  WebSettings settings=web.getSettings();settings.setJavaScriptEnabled(true);settings.setDomStorageEnabled(true);
  settings.setAllowFileAccess(false);settings.setAllowContentAccess(false);settings.setMixedContentMode(WebSettings.MIXED_CONTENT_NEVER_ALLOW);
  settings.setMediaPlaybackRequiresUserGesture(true);settings.setSupportMultipleWindows(true);
  web.setWebChromeClient(new WebChromeClient(){
   @Override public boolean onCreateWindow(WebView view,boolean dialog,boolean gesture,Message message) {
    if(gesture) new AlertDialog.Builder(MainActivity.this).setTitle("Continue in browser")
     .setMessage("Google sign-in is not supported inside this preview. Open the web player in your browser to connect. Browser and app playlists are stored separately; use playlist backups to transfer them.")
     .setPositiveButton("Open browser",(d,w)->external(Uri.parse(HOME))).setNegativeButton("Cancel",null).show();
    return false;
   }
  });
  web.setWebViewClient(new WebViewClient(){
   @Override public boolean shouldOverrideUrlLoading(WebView view,WebResourceRequest request) {
    if(!request.isForMainFrame())return false;
    if(internal(request.getUrl()))return false;
    external(request.getUrl());return true;
   }
   @Override public void onPageStarted(WebView v,String url,android.graphics.Bitmap icon){failed=false;}
   @Override public void onReceivedError(WebView v,WebResourceRequest request,WebResourceError error){if(request.isForMainFrame()&&!failed){failed=true;Toast.makeText(MainActivity.this,"Cannot connect. Check your connection and tap Reload.",Toast.LENGTH_LONG).show();}}
  });
  web.loadUrl(HOME);
 }
 @Override public void onBackPressed(){if(web.canGoBack())web.goBack();else super.onBackPressed();}
 @Override protected void onDestroy(){web.destroy();super.onDestroy();}
}
