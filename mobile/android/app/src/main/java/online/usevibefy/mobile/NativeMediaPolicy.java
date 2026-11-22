package online.usevibefy.mobile;
final class NativeMediaPolicy {
 static boolean youtube(String id){return id!=null&&id.matches("[A-Za-z0-9_-]{11}");}
 static boolean supported(String id){return youtube(id)||id!=null&&id.matches("jv:[0-9a-fA-F]{8}(?:-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12}");}
 static boolean juiceUrl(String uri){return uri!=null&&uri.matches("https://api\\.juicevault\\.xyz/music/stream/[0-9a-fA-F]{8}(?:-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12}");}
 static String uri(String id){if(!supported(id))throw new IllegalArgumentException("Unsupported recording");return id.startsWith("jv:")?"https://api.juicevault.xyz/music/stream/"+id.substring(3):"vibefy://audio/"+id;}
}
