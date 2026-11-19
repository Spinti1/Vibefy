package online.usevibefy.mobile;
import android.content.Context;
import android.util.AtomicFile;
import org.json.*;
import java.io.*;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.*;

final class Library {
 static final String API="https://vibefy-neon.vercel.app";
 final ExecutorService io=Executors.newSingleThreadExecutor();
 final AtomicFile file;
 JSONArray lists=new JSONArray();
 Library(Context context){file=new AtomicFile(new File(context.getFilesDir(),"native-library.json"));}
 void load(Runnable done){io.execute(()->{try(FileInputStream in=file.openRead()){byte[] data=read(in,8000001);if(data.length>8000000)throw new IOException("Library is too large");lists=new JSONArray(new String(data,StandardCharsets.UTF_8));}catch(FileNotFoundException e){}catch(Exception e){android.util.Log.w("Vibefy","Library could not be read; retained on disk");}done.run();});}
 void save(){String json=lists.toString();io.execute(()->{FileOutputStream out=null;try{out=file.startWrite();out.write(json.getBytes(StandardCharsets.UTF_8));file.finishWrite(out);}catch(Exception e){if(out!=null)file.failWrite(out);}});}
 void create(String name)throws JSONException{lists.put(new JSONObject().put("id",java.util.UUID.randomUUID().toString()).put("title",name).put("tracks",new JSONArray()));save();}
 void add(int index,JSONObject song)throws JSONException{JSONArray tracks=lists.getJSONObject(index).getJSONArray("tracks");String id=song.getString("id");for(int i=0;i<tracks.length();i++)if(id.equals(tracks.getJSONObject(i).optString("id")))return;tracks.put(song);save();}
 static byte[] read(InputStream in,int bound)throws IOException {ByteArrayOutputStream out=new ByteArrayOutputStream();byte[] buffer=new byte[8192];int size;while((size=in.read(buffer,0,Math.min(buffer.length,bound-out.size())))>0){out.write(buffer,0,size);if(out.size()>=bound)break;}return out.toByteArray();}
 static JSONObject get(String route)throws Exception{
  javax.net.ssl.HttpsURLConnection c=(javax.net.ssl.HttpsURLConnection)new java.net.URL(API+route).openConnection();c.setInstanceFollowRedirects(false);c.setConnectTimeout(12000);c.setReadTimeout(15000);c.setRequestProperty("Accept","application/json");
  try{if(c.getResponseCode()!=200)throw new IOException("Service unavailable ("+c.getResponseCode()+")");try(InputStream in=c.getInputStream()){byte[] b=read(in,2000001);if(b.length>2000000)throw new IOException("Response too large");return new JSONObject(new String(b,StandardCharsets.UTF_8));}}finally{c.disconnect();}
 }
}
