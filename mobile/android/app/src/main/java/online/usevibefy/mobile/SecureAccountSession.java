package online.usevibefy.mobile;
import android.content.Context;
import android.security.keystore.KeyGenParameterSpec;
import android.security.keystore.KeyProperties;
import android.util.Base64;
import org.json.JSONObject;
import java.nio.charset.StandardCharsets;
import java.security.KeyStore;
import javax.crypto.Cipher;
import javax.crypto.KeyGenerator;
import javax.crypto.SecretKey;
import javax.crypto.spec.GCMParameterSpec;

/** Device-bound AES-GCM; preferences contain only authenticated ciphertext. */
final class SecureAccountSession {
 private static final String ALIAS="vibefy-google-session-v1";
 private final Context context;
 SecureAccountSession(Context context){this.context=context.getApplicationContext();}
 private SecretKey key()throws Exception{KeyStore store=KeyStore.getInstance("AndroidKeyStore");store.load(null);if(store.containsAlias(ALIAS))return (SecretKey)store.getKey(ALIAS,null);KeyGenerator generator=KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES,"AndroidKeyStore");generator.init(new KeyGenParameterSpec.Builder(ALIAS,KeyProperties.PURPOSE_ENCRYPT|KeyProperties.PURPOSE_DECRYPT).setBlockModes(KeyProperties.BLOCK_MODE_GCM).setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE).setKeySize(256).setRandomizedEncryptionRequired(true).build());return generator.generateKey();}
 static boolean valid(JSONObject value){if(value==null||value.optInt("version")!=1)return false;String token=value.optString("access_token"),scope=value.optString("scope");long expiry=value.optLong("expires_at"),now=System.currentTimeMillis();return !token.isEmpty()&&token.length()<=8192&&java.util.Arrays.asList(scope.split(" ")).contains("https://www.googleapis.com/auth/youtube.readonly")&&expiry>0&&expiry<=now+86400000&&(expiry>now+5000||(!value.optString("refresh_token").isEmpty()&&value.optString("refresh_token").length()<=4096));}
 synchronized void save(JSONObject value)throws Exception{if(!valid(value))throw new IllegalArgumentException();byte[] raw=value.toString().getBytes(StandardCharsets.UTF_8);if(raw.length>16384)throw new IllegalArgumentException();Cipher cipher=Cipher.getInstance("AES/GCM/NoPadding");cipher.init(Cipher.ENCRYPT_MODE,key());byte[] iv=cipher.getIV(),data=cipher.doFinal(raw),blob=new byte[iv.length+data.length];if(iv.length!=12)throw new IllegalStateException();System.arraycopy(iv,0,blob,0,12);System.arraycopy(data,0,blob,12,data.length);if(!context.getSharedPreferences(ALIAS,0).edit().putString("blob",Base64.encodeToString(blob,Base64.NO_WRAP)).commit())throw new java.io.IOException();}
 synchronized JSONObject load()throws Exception{String encoded=context.getSharedPreferences(ALIAS,0).getString("blob",null);if(encoded==null)return null;try{if(encoded.length()>24000)throw new IllegalArgumentException();byte[] blob=Base64.decode(encoded,Base64.NO_WRAP);if(blob.length<29)throw new IllegalArgumentException();Cipher cipher=Cipher.getInstance("AES/GCM/NoPadding");cipher.init(Cipher.DECRYPT_MODE,key(),new GCMParameterSpec(128,blob,0,12));JSONObject value=new JSONObject(new String(cipher.doFinal(blob,12,blob.length-12),StandardCharsets.UTF_8));if(!valid(value)){clear();return null;}return value;}catch(javax.crypto.AEADBadTagException|IllegalArgumentException e){clear();return null;}}
 synchronized void clear()throws Exception{if(!context.getSharedPreferences(ALIAS,0).edit().remove("blob").commit())throw new java.io.IOException();}
}
