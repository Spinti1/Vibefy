package online.usevibefy.mobile;

import java.io.*;
import java.net.*;
import java.nio.charset.StandardCharsets;
import java.util.*;
import org.schabi.newpipe.extractor.NewPipe;
import org.schabi.newpipe.extractor.ServiceList;
import org.schabi.newpipe.extractor.MediaFormat;
import org.schabi.newpipe.extractor.downloader.*;
import org.schabi.newpipe.extractor.stream.*;

/** Resolves public audio metadata locally. Never requests account cookies or video streams. */
final class NativeAudioResolver {
 private final LinkedHashMap<String,Entry> cache=new LinkedHashMap<>();
 private static final class Entry {final String url;final long until;Entry(String u){url=u;until=System.currentTimeMillis()+240000;}}
 NativeAudioResolver(){NewPipe.init(new DirectDownloader());}
 synchronized String resolve(String video) throws Exception {
  if(!video.matches("[A-Za-z0-9_-]{11}"))throw new IOException("Invalid recording");
  Entry held=cache.get(video);if(held!=null&&held.until>System.currentTimeMillis())return held.url;
  StreamInfo info=StreamInfo.getInfo(ServiceList.YouTube,"https://www.youtube.com/watch?v="+video);
  AudioStream best=null;
  for(AudioStream stream:info.getAudioStreams()){
   if(!stream.isUrl()||stream.getDeliveryMethod()!=DeliveryMethod.PROGRESSIVE_HTTP||stream.getFormat()!=MediaFormat.M4A||!safeMedia(stream.getContent()))continue;
   if(best==null||stream.getAverageBitrate()>best.getAverageBitrate())best=stream;
  }
  if(best==null)throw new IOException("No compatible audio-only stream");
  String url=best.getContent();cache.put(video,new Entry(url));while(cache.size()>8)cache.remove(cache.keySet().iterator().next());return url;
 }
 static boolean safeMedia(String raw){try{URI u=URI.create(raw);String h=u.getHost();return "https".equals(u.getScheme())&&h!=null&&h.endsWith(".googlevideo.com")&&u.getUserInfo()==null&&(u.getPort()==-1||u.getPort()==443);}catch(Exception e){return false;}}
 private static boolean safeMetadata(URL u){String h=u.getHost();return "https".equals(u.getProtocol())&&u.getUserInfo()==null&&(u.getPort()==-1||u.getPort()==443)&&(h.equals("youtube.com")||h.endsWith(".youtube.com")||h.equals("youtubei.googleapis.com")||h.equals("www.youtube-nocookie.com")||h.endsWith(".googlevideo.com")||h.equals("i.ytimg.com"));}
 static final class DirectDownloader extends Downloader {
  @Override public Response execute(Request request) throws IOException {
   URL target=new URL(request.url());
   for(int redirects=0;redirects<=3;redirects++){
    if(!safeMetadata(target))throw new IOException("Unsupported metadata host");
    HttpURLConnection c=(HttpURLConnection)target.openConnection();c.setConnectTimeout(10000);c.setReadTimeout(12000);c.setInstanceFollowRedirects(false);c.setRequestMethod(request.httpMethod());
    c.setRequestProperty("User-Agent","Mozilla/5.0");c.setRequestProperty("Accept-Language","en-US,en;q=0.9");
    for(Map.Entry<String,List<String>> header:request.headers().entrySet())if(!header.getKey().equalsIgnoreCase("Cookie")&&!header.getKey().equalsIgnoreCase("Authorization"))c.setRequestProperty(header.getKey(),String.join(", ",header.getValue()));
    c.setRequestProperty("Accept-Encoding","identity");
    try {
     byte[] data=request.dataToSend();if(data!=null){if(data.length>1048576)throw new IOException("Metadata request too large");c.setDoOutput(true);try(OutputStream out=c.getOutputStream()){out.write(data);}}
     int status=c.getResponseCode();if(status>=300&&status<400){String next=c.getHeaderField("Location");if(next==null)throw new IOException("Invalid redirect");target=new URL(target,next);continue;}
     if(status==429)throw new IOException("Source temporarily rate limited");
     InputStream input=status>=400?c.getErrorStream():c.getInputStream();ByteArrayOutputStream body=new ByteArrayOutputStream();
     if(input!=null)try(InputStream in=input){byte[] buffer=new byte[8192];int size;while((size=in.read(buffer))!=-1){if(Thread.currentThread().isInterrupted())throw new InterruptedIOException();if(body.size()+size>8*1024*1024)throw new IOException("Metadata too large");body.write(buffer,0,size);}}
     return new Response(status,c.getResponseMessage(),c.getHeaderFields(),body.toString(StandardCharsets.UTF_8.name()),target.toString());
    }finally{c.disconnect();}
   }
   throw new IOException("Too many redirects");
  }
 }
}
