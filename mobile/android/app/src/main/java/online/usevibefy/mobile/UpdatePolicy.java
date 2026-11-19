package online.usevibefy.mobile;
import java.net.URL;
import java.security.MessageDigest;
final class UpdatePolicy {
 static int compare(String a,String b){String[] x=a.split("\\."),y=b.split("\\.");for(int i=0;i<3;i++){int n=Integer.compare(Integer.parseInt(x[i]),Integer.parseInt(y[i]));if(n!=0)return n;}return 0;}
 static boolean allowed(URL url){String host=url.getHost();return "https".equals(url.getProtocol())&&url.getUserInfo()==null&&(url.getPort()==-1||url.getPort()==443)&&(host.equals("api.github.com")||host.equals("github.com")||host.equals("release-assets.githubusercontent.com")||host.equals("objects.githubusercontent.com"));}
 static boolean validAsset(String version,String url,String digest,long size){return version.matches("[0-9]{1,4}\\.[0-9]{1,4}\\.[0-9]{1,4}")&&digest.matches("sha256:[a-fA-F0-9]{64}")&&size>0&&size<=50000000&&url.equals("https://github.com/Spinti1/Vibefy/releases/download/mobile-v"+version+"/Vibefy-"+version+".apk");}
 static boolean verified(long expected,long actual,byte[] hash,String releaseDigest){if(expected<=0||actual!=expected||hash.length!=32||!releaseDigest.matches("sha256:[a-fA-F0-9]{64}"))return false;byte[] expectedHash=new byte[32];for(int i=0;i<32;i++)expectedHash[i]=(byte)Integer.parseInt(releaseDigest.substring(7+2*i,9+2*i),16);return MessageDigest.isEqual(hash,expectedHash);}
}
