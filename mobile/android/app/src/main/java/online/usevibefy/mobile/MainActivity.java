package online.usevibefy.mobile;

import android.app.*;

import android.os.*;

import android.content.*;

import android.graphics.Color;

import android.net.Uri;

import android.view.*;

import android.view.inputmethod.EditorInfo;

import android.widget.*;

import android.graphics.Typeface;

import org.json.*;

import java.util.*;

import androidx.media3.common.*;

import androidx.media3.session.*;
import androidx.media3.session.MediaController;

import com.google.common.util.concurrent.ListenableFuture;



@androidx.media3.common.util.UnstableApi
public final class MainActivity extends Activity {

 private Library library;private MediaController player;private ListenableFuture<MediaController> future;

 private LinearLayout root,content,transport;private TextView status,now;private SeekBar seek;private Button play;private EditText query;

 private final Handler handler=new Handler(Looper.getMainLooper());private long searchGeneration;private JSONArray shown=new JSONArray();private boolean dragging;

 private static final int BG=0xff0e1215,FG=0xfff3f6f7,SECONDARY=0xffa9b3bc,ACCENT=0xff00d4bf;

 private final Runnable clock=new Runnable(){public void run(){if(player!=null){now.setText(player.getCurrentMediaItem()!=null?player.getCurrentMediaItem().mediaMetadata.title:"Choose a song");play.setText(player.isPlaying()?"Pause":"Play");if(!dragging){long duration=player.getDuration();seek.setMax((int)Math.min(Integer.MAX_VALUE,Math.max(1,duration)));seek.setProgress((int)player.getCurrentPosition());}}handler.postDelayed(this,500);}};

 @Override public void onCreate(Bundle state){super.onCreate(state);library=new Library(this);build();library.load(()->runOnUiThread(this::home));

  SessionToken token=new SessionToken(this,new ComponentName(this,PlaybackService.class));future=new MediaController.Builder(this,token).buildAsync();future.addListener(()->{try{player=future.get();player.addListener(new Player.Listener(){@Override public void onPlayerError(PlaybackException e){status.setText("This source could not be played. YouTube engine port is still in development.");}});}catch(Exception e){status.setText("Audio engine could not connect.");}},command->runOnUiThread(command));

 }

 private int dp(int n){return (int)(n*getResources().getDisplayMetrics().density);}

 private TextView label(String text,int size){TextView v=new TextView(this);v.setText(text);v.setTextColor(FG);v.setTextSize(size);v.setPadding(dp(6),dp(6),dp(6),dp(6));return v;}

 private Button button(String title,Runnable action){Button b=new Button(this);b.setText(title);b.setAllCaps(false);b.setTextColor(FG);b.setBackgroundTintList(android.content.res.ColorStateList.valueOf(0xff243039));b.setOnClickListener(v->action.run());return b;}

 private void build(){root=new LinearLayout(this);root.setOrientation(LinearLayout.VERTICAL);root.setPadding(dp(14),dp(12),dp(14),0);root.setBackgroundColor(BG);setContentView(root);

  TextView brand=label("Vibefy",28);brand.setTypeface(null,Typeface.BOLD);root.addView(brand);

  LinearLayout nav=new LinearLayout(this);nav.addView(button("Home",this::home));nav.addView(button("Queue",this::queue));nav.addView(button("Settings",this::settings));root.addView(nav);

  LinearLayout search=new LinearLayout(this);query=new EditText(this);query.setSingleLine();query.setHint("Songs and artists");query.setTextColor(FG);query.setHintTextColor(SECONDARY);query.setImeOptions(EditorInfo.IME_ACTION_SEARCH);query.setOnEditorActionListener((v,a,event)->{if(a==EditorInfo.IME_ACTION_SEARCH){search(false);return true;}return false;});search.addView(query,new LinearLayout.LayoutParams(0,-2,1));search.addView(button("Search",()->search(false)));root.addView(search);

  status=label("Loading your library…",12);status.setTextColor(SECONDARY);root.addView(status);

  ScrollView scroll=new ScrollView(this);content=new LinearLayout(this);content.setOrientation(LinearLayout.VERTICAL);scroll.addView(content);root.addView(scroll,new LinearLayout.LayoutParams(-1,0,1));

  transport=new LinearLayout(this);transport.setOrientation(LinearLayout.VERTICAL);now=label("Choose a song",16);transport.addView(now);seek=new SeekBar(this);transport.addView(seek);seek.setOnSeekBarChangeListener(new SeekBar.OnSeekBarChangeListener(){public void onStartTrackingTouch(SeekBar s){dragging=true;}public void onProgressChanged(SeekBar s,int value,boolean user){}public void onStopTrackingTouch(SeekBar s){if(player!=null)player.seekTo(s.getProgress());dragging=false;}});

  LinearLayout controls=new LinearLayout(this);controls.setGravity(Gravity.CENTER);controls.addView(button("Previous",()->{if(player!=null)player.seekToPreviousMediaItem();}));play=button("Play",()->{if(player!=null){if(player.isPlaying())player.pause();else {player.prepare();player.play();}}});play.setBackgroundTintList(android.content.res.ColorStateList.valueOf(ACCENT));play.setTextColor(BG);controls.addView(play);controls.addView(button("Next",()->{if(player!=null)player.seekToNextMediaItem();}));transport.addView(controls);root.addView(transport);

 }

 private void home(){content.removeAllViews();status.setText("Native development build · playlists saved on this device");content.addView(label("Your music",24));content.addView(button("Create playlist",this::create));content.addView(button("Import public YouTube playlist",()->importList(false)));content.addView(button("Import public Spotify playlist",()->importList(true)));content.addView(button("JuiceVault search",()->search(true)));

  for(int i=0;i<library.lists.length();i++){final int n=i;JSONObject list=library.lists.optJSONObject(i);content.addView(button(list.optString("title")+"  ·  "+list.optJSONArray("tracks").length()+" songs",()->show(list.optJSONArray("tracks"),list.optString("title"))));}

 }

 private void create(){EditText field=new EditText(this);new AlertDialog.Builder(this).setTitle("Create playlist").setView(field).setPositiveButton("Create",(d,w)->{String name=field.getText().toString().trim();if(!name.isEmpty())try{library.create(name);home();}catch(Exception e){status.setText("Could not save playlist");}}).setNegativeButton("Cancel",null).show();}

 private void importList(boolean spotify){EditText field=new EditText(this);field.setSingleLine();field.setHint("Public playlist link");new AlertDialog.Builder(this).setTitle(spotify?"Import public Spotify playlist":"Import YouTube playlist").setView(field).setPositiveButton("Import",(d,w)->{try{Uri uri=Uri.parse(field.getText().toString());String id=spotify?uri.getLastPathSegment():uri.getQueryParameter("list");if(id==null||!id.matches(spotify?"[A-Za-z0-9]{22}":"[A-Za-z0-9_-]{10,200}"))throw new Exception();status.setText("Importing playlist…");library.io.execute(()->{try{JSONObject result=Library.get((spotify?"/api/import/spotify?id=":"/api/playlist?id=")+id);JSONArray songs=result.optJSONArray("tracks");if(songs==null)throw new Exception("No tracks returned");JSONObject list=new JSONObject().put("id",UUID.randomUUID().toString()).put("title",result.optString("title","Imported playlist")).put("tracks",songs);runOnUiThread(()->{library.lists.put(list);library.save();home();});}catch(Exception e){runOnUiThread(()->status.setText("Playlist import failed. Try a public playlist."));}});}catch(Exception e){status.setText("Enter a valid public playlist link.");}}).setNegativeButton("Cancel",null).show();}

 private void search(boolean juice){String q=query.getText().toString().trim();if(q.isEmpty()){status.setText("Enter a song or artist first.");return;}final long generation=++searchGeneration;status.setText("Searching…");library.io.execute(()->{try{JSONObject response=Library.get((juice?"/api/juice/search?q=":"/api/search?q=")+Uri.encode(q));JSONArray tracks=response.optJSONArray(juice?"results":"tracks");if(tracks==null)tracks=new JSONArray();if(juice)for(int i=0;i<tracks.length();i++){JSONObject song=tracks.getJSONObject(i);song.put("id","jv:"+song.getString("id"));if(!song.has("artist"))song.put("artist","Juice WRLD");}final JSONArray result=tracks;runOnUiThread(()->{if(generation!=searchGeneration)return;show(result,"Search results");status.setText(result.length()+" results");});}catch(Exception e){runOnUiThread(()->{if(generation==searchGeneration)status.setText("Search unavailable. Please try again.");});}});}

 private void show(JSONArray tracks,String title){shown=tracks;content.removeAllViews();content.addView(label(title,24));for(int i=0;i<tracks.length();i++){JSONObject t=tracks.optJSONObject(i);final int index=i;LinearLayout row=new LinearLayout(this);Button song=button(t.optString("title")+"\n"+t.optString("artist"),()->start(tracks,index));row.addView(song,new LinearLayout.LayoutParams(0,-2,1));row.addView(button("+",()->add(t)));content.addView(row);}}

 private void add(JSONObject song){String[] names=new String[library.lists.length()];for(int i=0;i<names.length;i++)names[i]=library.lists.optJSONObject(i).optString("title");if(names.length==0){status.setText("Create a playlist first.");return;}new AlertDialog.Builder(this).setTitle("Add to playlist").setItems(names,(d,n)->{try{library.add(n,song);status.setText("Added to "+names[n]);}catch(Exception e){status.setText("Could not add song");}}).show();}

 private MediaItem item(JSONObject song){String id=song.optString("id");String uri;if(id.matches("jv:[0-9a-fA-F-]{36}"))uri="https://api.juicevault.xyz/music/stream/"+id.substring(3);else if(id.matches("[A-Za-z0-9_-]{11}"))uri=Library.API+"/api/audio/"+id;else return null;MediaMetadata metadata=new MediaMetadata.Builder().setTitle(song.optString("title")).setArtist(song.optString("artist")).build();return new MediaItem.Builder().setMediaId(id).setUri(uri).setMediaMetadata(metadata).build();}

 private void start(JSONArray tracks,int selected){if(player==null){status.setText("Audio engine is connecting.");return;}ArrayList<MediaItem> items=new ArrayList<>();int target=0;for(int i=0;i<tracks.length();i++){MediaItem item=item(tracks.optJSONObject(i));if(item!=null){if(i<selected)target++;items.add(item);}}if(items.isEmpty())return;player.setMediaItems(items,Math.min(target,items.size()-1),0);player.prepare();player.play();status.setText("Loading audio…");}

 private void queue(){content.removeAllViews();content.addView(label("Queue",24));if(player!=null)for(int i=0;i<player.getMediaItemCount();i++){final int n=i;content.addView(button(String.valueOf(player.getMediaItemAt(i).mediaMetadata.title),()->{player.seekToDefaultPosition(n);player.play();}));}}

 private void settings(){content.removeAllViews();content.addView(label("Settings",24));content.addView(label("Native interface · Media3 audio engine\nLocal playlist metadata is saved automatically. No music files are stored in your library.\nYouTube stream resolution, accounts, Jam and mods are still being ported.",16));content.addView(button("Shuffle",()->{if(player!=null){player.setShuffleModeEnabled(!player.getShuffleModeEnabled());status.setText("Shuffle "+(player.getShuffleModeEnabled()?"on":"off"));}}));content.addView(button("Repeat",()->{if(player!=null){player.setRepeatMode((player.getRepeatMode()+1)%3);status.setText("Repeat mode: "+player.getRepeatMode());}}));}

 @Override protected void onStart(){super.onStart();handler.post(clock);}

 @Override protected void onStop(){handler.removeCallbacks(clock);super.onStop();}

 @Override protected void onDestroy(){MediaController.releaseFuture(future);super.onDestroy();}

}

