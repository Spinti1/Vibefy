package online.usevibefy.mobile;

import android.app.Activity;
import android.app.AlertDialog;
import android.content.Context;
import android.net.nsd.NsdManager;
import android.net.nsd.NsdServiceInfo;
import android.os.SystemClock;
import java.net.ServerSocket;
import java.net.Socket;
import java.nio.charset.StandardCharsets;
import java.util.UUID;
import java.util.function.Consumer;

/** Foreground-only discovery and a bounded invitation, never an audio server. */
final class NearbyReceiver implements AutoCloseable {
 private final Activity activity;private final Consumer<String> join;
 private final String token=UUID.randomUUID().toString().replace("-","");
 private final NsdManager manager;private volatile boolean stopped;private volatile ServerSocket server;
 private volatile boolean registered;private boolean prompt;private AlertDialog alert;private long lastPrompt=-10000;
 private final NsdManager.RegistrationListener registration=new NsdManager.RegistrationListener(){
  public void onServiceRegistered(NsdServiceInfo info){registered=true;if(stopped)unregister();}
  public void onRegistrationFailed(NsdServiceInfo info,int code){close();}
  public void onServiceUnregistered(NsdServiceInfo info){registered=false;}
  public void onUnregistrationFailed(NsdServiceInfo info,int code){registered=false;}
 };
 NearbyReceiver(Activity activity,Consumer<String> join){this.activity=activity;this.join=join;manager=(NsdManager)activity.getSystemService(Context.NSD_SERVICE);new Thread(this::listen,"vibefy-nearby").start();}
 private void listen(){
  try(ServerSocket listener=new ServerSocket(0,2)){
   server=listener;if(stopped)return;
   NsdServiceInfo info=new NsdServiceInfo();info.setServiceName("Vibefy Android");info.setServiceType("_vibefy._tcp.");info.setPort(listener.getLocalPort());info.setAttribute("v","1");info.setAttribute("platform","android");info.setAttribute("token",token);
   manager.registerService(info,NsdManager.PROTOCOL_DNS_SD,registration);
   while(!stopped){try(Socket socket=listener.accept()){
    socket.setSoTimeout(2500);byte[] line=new byte[128];int count=0,value;
    while(count<line.length&&(value=socket.getInputStream().read())!=-1){if(value==10)break;line[count++]=(byte)value;}
    String packet=new String(line,0,count,StandardCharsets.US_ASCII),prefix="VIBEFY_PAIR_V1 "+token+" ";
    if(stopped||!packet.startsWith(prefix))continue;String code=packet.substring(prefix.length());
    if(!code.matches("[ABCDEFGHJKLMNPQRSTUVWXYZ23456789]{12}"))continue;
    socket.getOutputStream().write("READY\n".getBytes(StandardCharsets.US_ASCII));
    activity.runOnUiThread(()->confirm(code));
   }catch(java.io.IOException ignored){}}
  }catch(Exception ignored){}finally{server=null;activity.runOnUiThread(this::unregister);}
 }
 private void confirm(String code){if(stopped||activity.isFinishing()||prompt||SystemClock.elapsedRealtime()-lastPrompt<10000)return;
  prompt=true;lastPrompt=SystemClock.elapsedRealtime();
  alert=new AlertDialog.Builder(activity).setTitle("Connect to Vibefy desktop?").setMessage("A device on your Wi-Fi invited this phone to shared playback. Connect only if you just selected this phone on your computer. Both devices can control the music.")
   .setNegativeButton("Cancel",null).setPositiveButton("Connect",(d,w)->{if(!stopped)join.accept(code);}).setOnDismissListener(d->prompt=false).show();
 }
 private void unregister(){if(registered){registered=false;try{manager.unregisterService(registration);}catch(Exception ignored){}}}
 public void close(){stopped=true;activity.runOnUiThread(()->{if(alert!=null){alert.dismiss();alert=null;}});ServerSocket s=server;if(s!=null)try{s.close();}catch(Exception ignored){}activity.runOnUiThread(this::unregister);}
}
