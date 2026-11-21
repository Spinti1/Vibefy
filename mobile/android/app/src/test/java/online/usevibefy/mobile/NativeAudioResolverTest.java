package online.usevibefy.mobile;
import org.junit.Test;
import static org.junit.Assert.*;
public final class NativeAudioResolverTest {
 @Test public void acceptsOnlyHttpsGoogleMedia(){assertTrue(NativeAudioResolver.safeMedia("https://rr1---sn-test.googlevideo.com/videoplayback?itag=140"));for(String url:new String[]{"http://x.googlevideo.com/a","https://googlevideo.com.evil.test/a","https://127.0.0.1/a","https://name:pass@x.googlevideo.com/a","https://x.googlevideo.com:8443/a","file:///private"})assertFalse(NativeAudioResolver.safeMedia(url));}
}
