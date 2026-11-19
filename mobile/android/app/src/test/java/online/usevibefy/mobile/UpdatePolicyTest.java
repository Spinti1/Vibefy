package online.usevibefy.mobile;
import org.junit.Test;
import static org.junit.Assert.*;
import java.net.URL;
import java.security.MessageDigest;
public class UpdatePolicyTest {
 @Test public void comparesVersionsNumerically(){assertTrue(UpdatePolicy.compare("0.2.10","0.2.9")>0);assertEquals(0,UpdatePolicy.compare("0.2.3","0.2.3"));assertTrue(UpdatePolicy.compare("0.1.9","0.2.0")<0);}
 @Test public void onlyTrustsGitHubHttpsDownloads()throws Exception{for(String url:new String[]{"http://github.com/a","https://github.com.evil.test/a","https://github.com@evil.test/a","https://evil.test/a","https://github.com:444/a"})assertFalse(url,UpdatePolicy.allowed(new URL(url)));assertTrue(UpdatePolicy.allowed(new URL("https://github.com/Spinti1/Vibefy")));assertTrue(UpdatePolicy.allowed(new URL("https://release-assets.githubusercontent.com/a")));}
 @Test public void rejectsForeignOrMissingReleaseIntegrity(){String good="https://github.com/Spinti1/Vibefy/releases/download/mobile-v0.2.3/Vibefy-0.2.3.apk",digest="sha256:"+"a".repeat(64);assertTrue(UpdatePolicy.validAsset("0.2.3",good,digest,1000));assertFalse(UpdatePolicy.validAsset("0.2.3",good,"",1000));assertFalse(UpdatePolicy.validAsset("0.2.3",good.replace("Spinti1","someone"),digest,1000));assertFalse(UpdatePolicy.validAsset("0.2.3",good,digest,50000001));assertFalse(UpdatePolicy.validAsset("0.2.4",good,digest,1000));}
 @Test public void sizeAndDigestBothHaveToMatch()throws Exception{byte[] hash=MessageDigest.getInstance("SHA-256").digest("abc".getBytes("UTF-8"));String digest="sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";assertTrue(UpdatePolicy.verified(3,3,hash,digest));assertFalse(UpdatePolicy.verified(3,4,hash,digest));hash[0]^=1;assertFalse(UpdatePolicy.verified(3,3,hash,digest));}
}
