package online.usevibefy.mobile;

import android.app.PendingIntent;
import android.content.Intent;
import android.net.Uri;
import androidx.media3.common.AudioAttributes;
import androidx.media3.common.C;
import androidx.media3.datasource.DefaultDataSource;
import androidx.media3.datasource.ResolvingDataSource;
import androidx.media3.exoplayer.ExoPlayer;
import androidx.media3.exoplayer.source.DefaultMediaSourceFactory;
import androidx.media3.session.MediaSession;
import androidx.media3.session.MediaSessionService;

/** Audio and queue belong to Android, independently of the WebView lifecycle. */
@androidx.media3.common.util.UnstableApi
public final class HybridPlaybackService extends MediaSessionService {
 private MediaSession session;
 @Override public void onCreate(){
  super.onCreate();NativeAudioResolver resolver=new NativeAudioResolver();
  ResolvingDataSource.Factory sources=new ResolvingDataSource.Factory(new DefaultDataSource.Factory(this),spec->{
   Uri uri=spec.uri;String id=uri.getLastPathSegment();
   if("vibefy".equals(uri.getScheme())&&"audio".equals(uri.getHost())&&NativeMediaPolicy.youtube(id)){
    try{return spec.withUri(Uri.parse(resolver.resolve(id)));}catch(Exception error){throw new java.io.IOException("This recording has no available audio stream.",error);}
   }
   if(NativeMediaPolicy.juiceUrl(uri.toString()))return spec;
   throw new java.io.IOException("Unsupported audio source");
  });
  ExoPlayer player=new ExoPlayer.Builder(this)
   .setMediaSourceFactory(new DefaultMediaSourceFactory(sources))
   .setMaxSeekToPreviousPositionMs(Long.MAX_VALUE)
   .build();
  player.setAudioAttributes(new AudioAttributes.Builder().setUsage(C.USAGE_MEDIA).setContentType(C.AUDIO_CONTENT_TYPE_MUSIC).build(),true);
  player.addListener(new androidx.media3.common.Player.Listener(){@Override public void onEvents(androidx.media3.common.Player p,androidx.media3.common.Player.Events events){boolean pause=!getSharedPreferences("native-audio-preferences",0).getBoolean("auto-next",true);if(player.getPauseAtEndOfMediaItems()!=pause)player.setPauseAtEndOfMediaItems(pause);}});
  player.setHandleAudioBecomingNoisy(true);player.setWakeMode(C.WAKE_MODE_NETWORK);
  PendingIntent open=PendingIntent.getActivity(this,0,new Intent(this,MainActivity.class),PendingIntent.FLAG_IMMUTABLE|PendingIntent.FLAG_UPDATE_CURRENT);
  session=new MediaSession.Builder(this,player).setSessionActivity(open).build();
 }
 @Override public MediaSession onGetSession(MediaSession.ControllerInfo info){return getPackageName().equals(info.getPackageName())||info.isTrusted()?session:null;}
 @Override public void onTaskRemoved(Intent root){if(!isPlaybackOngoing())stopSelf();}
 @Override public void onDestroy(){if(session!=null){session.getPlayer().release();session.release();session=null;}super.onDestroy();}
}
