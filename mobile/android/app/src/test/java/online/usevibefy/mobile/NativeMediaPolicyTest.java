package online.usevibefy.mobile;
import org.junit.Test;
import static org.junit.Assert.*;
public class NativeMediaPolicyTest {
 @Test public void onlyPublicRecordingIdentitiesBecomeMediaUris(){assertEquals("vibefy://audio/abcdefghijk",NativeMediaPolicy.uri("abcdefghijk"));assertEquals("https://api.juicevault.xyz/music/stream/12345678-1234-1234-1234-123456789abc",NativeMediaPolicy.uri("jv:12345678-1234-1234-1234-123456789abc"));for(String id:new String[]{"https://evil.test/audio","../private","spotify:abc","jv:../../x",null})assertFalse(NativeMediaPolicy.supported(id));}
 @Test public void juiceCannotRedirectThePlayerToAnotherHostOrPath(){assertTrue(NativeMediaPolicy.juiceUrl("https://api.juicevault.xyz/music/stream/12345678-1234-1234-1234-123456789abc"));for(String uri:new String[]{"http://api.juicevault.xyz/music/stream/12345678-1234-1234-1234-123456789abc","https://api.juicevault.xyz.evil.test/music/stream/12345678-1234-1234-1234-123456789abc","https://api.juicevault.xyz/admin","file:///private"})assertFalse(NativeMediaPolicy.juiceUrl(uri));}
}
