package online.usevibefy.mobile;

import android.app.*;
import android.content.*;
import android.net.Uri;
import android.provider.Settings;
import android.widget.Toast;
import androidx.core.content.FileProvider;
import org.json.*;
import java.io.*;
import java.net.*;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.util.concurrent.ExecutorService;

/** GitHub HTTPS metadata, bounded downloads, release size and SHA-256 verification. */
final class NativeUpdates {
 private final Activity app;private final ExecutorService worker;private volatile boolean busy;private File pending;private boolean awaitingPermission;
 private static final String API="https://api.github.com/repos/Spinti1/Vibefy/releases?per_page=30";
 NativeUpdates(Activity app,ExecutorService worker){this.app=app;this.worker=worker;}
 private void ui(Runnable task){app.runOnUiThread(()->{if(!app.isFinishing()&&!app.isDestroyed())task.run();});}
 private static HttpURLConnection open(String raw)throws Exception{
  URL url=new URL(raw);for(int i=0;i<6;i++){if(!UpdatePolicy.allowed(url))throw new IOException("Untrusted update URL");HttpURLConnection c=(HttpURLConnection)url.openConnection();c.setInstanceFollowRedirects(false);c.setConnectTimeout(15000);c.setReadTimeout(20000);c.setRequestProperty("Accept","application/vnd.github+json");c.setRequestProperty("User-Agent","Vibefy-Mobile");int code=c.getResponseCode();if(code>=300&&code<400){String next=c.getHeaderField("Location");c.disconnect();if(next==null)throw new IOException();url=new URL(url,next);continue;}if(code!=200){c.disconnect();throw new IOException("GitHub response "+code);}return c;}throw new IOException("Too many redirects");
 }
 private static byte[] read(String raw,int limit)throws Exception{HttpURLConnection c=open(raw);try(InputStream in=c.getInputStream();ByteArrayOutputStream out=new ByteArrayOutputStream()){byte[] b=new byte[8192];int n,total=0;while((n=in.read(b))!=-1){total+=n;if(total>limit)throw new IOException("Response too large");out.write(b,0,n);}return out.toByteArray();}finally{c.disconnect();}}
 void check(boolean manual){
  if(busy)return;long last=app.getPreferences(0).getLong("mobile-update-checked",0);if(!manual&&System.currentTimeMillis()-last<3600000)return;busy=true;if(manual)Toast.makeText(app,"Checking GitHub for updates…",Toast.LENGTH_SHORT).show();
  worker.execute(()->{try{JSONArray releases=new JSONArray(new String(read(API,1500000),StandardCharsets.UTF_8));JSONObject best=null;String version="0.0.0";
   for(int i=0;i<releases.length();i++){JSONObject r=releases.getJSONObject(i);String tag=r.optString("tag_name");if(r.optBoolean("draft")||!tag.matches("mobile-v[0-9]{1,4}\\.[0-9]{1,4}\\.[0-9]{1,4}"))continue;String candidate=tag.substring(8);if(UpdatePolicy.compare(candidate,version)<=0)continue;JSONArray assets=r.optJSONArray("assets");if(assets==null)continue;for(int j=0;j<assets.length();j++){JSONObject a=assets.getJSONObject(j);if(!a.optString("name").equals("Vibefy-"+candidate+".apk"))continue;String digest=a.optString("digest"),url=a.optString("browser_download_url");long size=a.optLong("size");if(!UpdatePolicy.validAsset(candidate,url,digest,size))continue;best=a;version=candidate;}}
   if(best==null)throw new IOException("No verified Android release is available");
   app.getPreferences(0).edit().putLong("mobile-update-checked",System.currentTimeMillis()).apply();String current=app.getPackageManager().getPackageInfo(app.getPackageName(),0).versionName;
   if(best==null||UpdatePolicy.compare(version,current)<=0){if(manual)ui(()->Toast.makeText(app,"Vibefy is up to date",Toast.LENGTH_SHORT).show());return;}
   JSONObject asset=best;String latest=version;ui(()->new AlertDialog.Builder(app).setTitle("Vibefy "+latest+" is available").setMessage("Download the update from GitHub? Its size and SHA-256 will be verified before Android opens the installer. Your playlists and settings stay saved.").setNegativeButton("Later",null).setPositiveButton("Download update",(d,w)->download(asset)).show());
  }catch(Exception e){if(manual)ui(()->Toast.makeText(app,"Could not check GitHub. Try again later.",Toast.LENGTH_LONG).show());}finally{busy=false;}});
 }
 private void download(JSONObject asset){if(busy)return;busy=true;Toast.makeText(app,"Downloading update…",Toast.LENGTH_SHORT).show();worker.execute(()->{File file=null;try{File dir=new File(app.getCacheDir(),"updates");if(!dir.exists()&&!dir.mkdirs())throw new IOException();file=new File(dir,"Vibefy-update.apk");MessageDigest hash=MessageDigest.getInstance("SHA-256");long expected=asset.getLong("size"),total=0;HttpURLConnection c=open(asset.getString("browser_download_url"));try(InputStream in=c.getInputStream();OutputStream out=new FileOutputStream(file)){byte[] data=new byte[32768];int n;while((n=in.read(data))!=-1){total+=n;if(total>expected)throw new IOException("Size mismatch");out.write(data,0,n);hash.update(data,0,n);}}finally{c.disconnect();}if(!UpdatePolicy.verified(expected,total,hash.digest(),asset.getString("digest")))throw new IOException("Integrity mismatch");pending=file;ui(this::offerInstall);
   }catch(Exception e){if(file!=null)file.delete();ui(()->Toast.makeText(app,"Update could not be verified. Nothing was installed.",Toast.LENGTH_LONG).show());}finally{busy=false;}});}
 private void offerInstall(){if(pending==null)return;if(!app.getPackageManager().canRequestPackageInstalls()){new AlertDialog.Builder(app).setTitle("Allow Vibefy updates").setMessage("Android requires your permission to install this verified APK. Enable Allow from this source, then return to Vibefy.").setNegativeButton("Later",null).setPositiveButton("Open Android settings",(d,w)->{awaitingPermission=true;app.startActivity(new Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES,Uri.parse("package:"+app.getPackageName())));}).show();return;}
  Uri uri=FileProvider.getUriForFile(app,app.getPackageName()+".updates",pending);Intent install=new Intent(Intent.ACTION_VIEW).setDataAndType(uri,"application/vnd.android.package-archive").addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);try{app.startActivity(install);}catch(ActivityNotFoundException e){Toast.makeText(app,"Android installer is unavailable",Toast.LENGTH_LONG).show();}}
 void resumed(){check(false);if(awaitingPermission){awaitingPermission=false;if(app.getPackageManager().canRequestPackageInstalls())offerInstall();}}
}
