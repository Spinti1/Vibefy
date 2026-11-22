package online.usevibefy.mobile;

import android.app.Activity;
import android.content.ComponentName;
import android.net.Uri;
import android.os.Handler;
import android.os.Looper;
import androidx.media3.common.*;
import androidx.media3.session.*;
import com.google.common.util.concurrent.ListenableFuture;
import org.json.*;
import java.util.*;
import java.util.function.Consumer;

@androidx.media3.common.util.UnstableApi
final class HybridAudioBridge {
 private final Activity app;private final Consumer<JSONObject> publish;
 private final Handler handler=new Handler(Looper.getMainLooper());
 private final ListenableFuture<MediaController> future;
 private final ArrayDeque<JSONObject> pending=new ArrayDeque<>();
 private MediaController player;private boolean foreground,closed;private long generation;
 private final Runnable clock=new Runnable(){public void run(){if(closed||!foreground)return;emit();handler.postDelayed(this,500);}};
 HybridAudioBridge(Activity app,Consumer<JSONObject> publish){this.app=app;this.publish=publish;
  future=new MediaController.Builder(app,new SessionToken(app,new ComponentName(app,HybridPlaybackService.class))).buildAsync();
  future.addListener(()->{try{if(closed)return;player=future.get();player.addListener(new Player.Listener(){@Override public void onEvents(Player p,Player.Events events){emit();}});while(!pending.isEmpty())command(pending.removeFirst());emit();}catch(Exception error){failure();}},app::runOnUiThread);
 }
 boolean handle(JSONObject packet){if(!"nativeMedia".equals(packet.optString("action")))return false;app.runOnUiThread(()->command(packet));return true;}
 private void command(JSONObject packet){
  if(closed)return;if(player==null){if(pending.size()<20)pending.addLast(packet);return;}
  try{switch(packet.optString("command")){
   case "queue": {
    JSONArray rows=packet.getJSONArray("tracks");if(rows.length()==0||rows.length()>10000)throw new IllegalArgumentException();
    ArrayList<MediaItem> items=new ArrayList<>();
    for(int i=0;i<rows.length();i++){JSONObject row=rows.getJSONObject(i);String id=row.optString("id");if(!NativeMediaPolicy.supported(id))throw new IllegalArgumentException();
     MediaMetadata.Builder metadata=new MediaMetadata.Builder().setTitle(row.optString("title").substring(0,Math.min(300,row.optString("title").length()))).setArtist(row.optString("artist").substring(0,Math.min(300,row.optString("artist").length())));
     String art=row.optString("image");if(art.length()<2000&&art.matches("https://(?:i\\.ytimg\\.com|[A-Za-z0-9.-]+\\.googleusercontent\\.com|[A-Za-z0-9.-]+\\.ggpht\\.com|api\\.juicevault\\.xyz)/[^\\s]*"))metadata.setArtworkUri(Uri.parse(art));
     items.add(new MediaItem.Builder().setMediaId(id).setUri(NativeMediaPolicy.uri(id)).setMediaMetadata(metadata.build()).build());
    }
    int index=packet.optInt("index");if(index<0||index>=items.size())throw new IllegalArgumentException();generation=packet.optLong("generation");
    app.getSharedPreferences("native-audio-preferences",0).edit().putBoolean("auto-next",packet.optBoolean("autoNext",true)).apply();
    player.setShuffleModeEnabled(packet.optBoolean("shuffle"));player.setRepeatMode(packet.optBoolean("repeat")?Player.REPEAT_MODE_ALL:Player.REPEAT_MODE_OFF);
    player.setMediaItems(items,index,Math.max(0,Math.min(86400000,packet.optLong("position"))));player.prepare();player.setPlayWhenReady(!packet.optBoolean("paused"));break;
   }
   case "play":player.play();break;
   case "pause":player.pause();break;
   case "seek":player.seekTo(Math.max(0,Math.min(86400000,packet.optLong("position"))));break;
   case "volume":player.setVolume((float)Math.max(0,Math.min(1,packet.optDouble("value",.8))));break;
   case "modes":player.setShuffleModeEnabled(packet.optBoolean("shuffle"));player.setRepeatMode(packet.optBoolean("repeat")?Player.REPEAT_MODE_ALL:Player.REPEAT_MODE_OFF);break;
   case "next":player.seekToNextMediaItem();break;
   case "previous":player.seekToPrevious();break;
   case "clear":player.stop();player.clearMediaItems();break;
   case "state":break;
   default:return;
  }emit();}catch(Exception error){failure();}
 }
 private void failure(){try{publish.accept(new JSONObject().put("generation",generation).put("error","The native audio player could not load this recording."));}catch(JSONException ignored){}}
 private void emit(){if(closed||player==null)return;try{MediaItem item=player.getCurrentMediaItem();JSONObject s=new JSONObject().put("generation",generation).put("id",item==null?"":item.mediaId).put("index",player.getCurrentMediaItemIndex()).put("position",Math.max(0,player.getCurrentPosition())/1000.0).put("duration",Math.max(0,player.getDuration())/1000.0).put("paused",!player.getPlayWhenReady()).put("playing",player.isPlaying()).put("ready",player.getPlaybackState()==Player.STATE_READY).put("ended",player.getPlaybackState()==Player.STATE_ENDED).put("error",player.getPlayerError()==null?JSONObject.NULL:"This recording could not be played.");if(item!=null)s.put("title",String.valueOf(item.mediaMetadata.title)).put("artist",String.valueOf(item.mediaMetadata.artist)).put("image",item.mediaMetadata.artworkUri==null?"":item.mediaMetadata.artworkUri.toString());publish.accept(s);}catch(JSONException ignored){}}
 void foreground(boolean value){foreground=value;handler.removeCallbacks(clock);if(value&&!closed)handler.post(clock);}
 void close(){closed=true;handler.removeCallbacks(clock);pending.clear();MediaController.releaseFuture(future);}
}
