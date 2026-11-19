package online.usevibefy.mobile;
import androidx.media3.exoplayer.ExoPlayer;
import androidx.media3.session.MediaSession;
import androidx.media3.session.MediaSessionService;
import androidx.media3.common.AudioAttributes;
import androidx.media3.common.C;

@androidx.media3.common.util.UnstableApi
public final class PlaybackService extends MediaSessionService {
 private MediaSession session;
 private final android.os.Handler handler=new android.os.Handler(android.os.Looper.getMainLooper());
 private final Runnable checkpoint=new Runnable(){public void run(){if(session!=null&&session.getPlayer().isPlaying())save();handler.postDelayed(this,5000);}};
 private void save(){
  if(session==null)return;
  androidx.media3.common.Player player=session.getPlayer();org.json.JSONArray queue=new org.json.JSONArray();
  try {for(int i=0;i<Math.min(1000,player.getMediaItemCount());i++){androidx.media3.common.MediaItem item=player.getMediaItemAt(i);queue.put(new org.json.JSONObject().put("id",item.mediaId).put("title",item.mediaMetadata.title).put("artist",item.mediaMetadata.artist));}
   getSharedPreferences("native-playback",0).edit().putString("queue",queue.toString()).putInt("index",player.getCurrentMediaItemIndex()).putLong("position",player.getCurrentPosition()).putBoolean("shuffle",player.getShuffleModeEnabled()).putInt("repeat",player.getRepeatMode()).apply();
  }catch(org.json.JSONException ignored){}
 }
 private void restore(ExoPlayer player){
  android.content.SharedPreferences prefs=getSharedPreferences("native-playback",0);
  try {org.json.JSONArray queue=new org.json.JSONArray(prefs.getString("queue","[]"));java.util.ArrayList<androidx.media3.common.MediaItem> items=new java.util.ArrayList<>();
   for(int i=0;i<Math.min(1000,queue.length());i++){org.json.JSONObject song=queue.getJSONObject(i);String id=song.optString("id"),uri;
    if(id.matches("jv:[0-9a-fA-F-]{36}"))uri="https://api.juicevault.xyz/music/stream/"+id.substring(3);else if(id.matches("[A-Za-z0-9_-]{11}"))uri=Library.API+"/api/audio/"+id;else continue;
    items.add(new androidx.media3.common.MediaItem.Builder().setMediaId(id).setUri(uri).setMediaMetadata(new androidx.media3.common.MediaMetadata.Builder().setTitle(song.optString("title")).setArtist(song.optString("artist")).build()).build());
   }
   if(!items.isEmpty())player.setMediaItems(items,Math.max(0,Math.min(items.size()-1,prefs.getInt("index",0))),Math.max(0,prefs.getLong("position",0)));
   player.setShuffleModeEnabled(prefs.getBoolean("shuffle",false));int repeat=prefs.getInt("repeat",0);player.setRepeatMode(repeat==1?androidx.media3.common.Player.REPEAT_MODE_ONE:repeat==2?androidx.media3.common.Player.REPEAT_MODE_ALL:androidx.media3.common.Player.REPEAT_MODE_OFF);
  }catch(Exception ignored){}
 }

 @Override public void onCreate(){super.onCreate();ExoPlayer player=new ExoPlayer.Builder(this).build();player.setAudioAttributes(new AudioAttributes.Builder().setUsage(C.USAGE_MEDIA).setContentType(C.AUDIO_CONTENT_TYPE_MUSIC).build(),true);player.setHandleAudioBecomingNoisy(true);restore(player);session=new MediaSession.Builder(this,player).build();player.addListener(new androidx.media3.common.Player.Listener(){@Override public void onEvents(androidx.media3.common.Player p,androidx.media3.common.Player.Events events){save();}});handler.postDelayed(checkpoint,5000);}
 @Override public MediaSession onGetSession(MediaSession.ControllerInfo controller){return getPackageName().equals(controller.getPackageName())||controller.isTrusted()?session:null;}
 @Override public void onDestroy(){handler.removeCallbacks(checkpoint);save();if(session!=null){session.getPlayer().release();session.release();session=null;}super.onDestroy();}
}
