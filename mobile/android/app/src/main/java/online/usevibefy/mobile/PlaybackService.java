package online.usevibefy.mobile;
import androidx.media3.exoplayer.ExoPlayer;
import androidx.media3.session.MediaSession;
import androidx.media3.session.MediaSessionService;
import androidx.media3.common.AudioAttributes;
import androidx.media3.common.C;

public final class PlaybackService extends MediaSessionService {
 private MediaSession session;
 @Override public void onCreate(){super.onCreate();ExoPlayer player=new ExoPlayer.Builder(this).build();player.setAudioAttributes(new AudioAttributes.Builder().setUsage(C.USAGE_MEDIA).setContentType(C.AUDIO_CONTENT_TYPE_MUSIC).build(),true);player.setHandleAudioBecomingNoisy(true);session=new MediaSession.Builder(this,player).build();}
 @Override public MediaSession onGetSession(MediaSession.ControllerInfo controller){return getPackageName().equals(controller.getPackageName())||controller.isTrusted()?session:null;}
 @Override public void onDestroy(){if(session!=null){session.getPlayer().release();session.release();session=null;}super.onDestroy();}
}
