import Foundation
import Security

/// App-private, device-bound Keychain record. Never stored in WebKit or exported backups.
enum SecureAccountSession {
 private static let service="online.usevibefy.google-session"
 private static let account="youtube"
 private static var query:[String:Any] {[kSecClass as String:kSecClassGenericPassword,kSecAttrService as String:service,kSecAttrAccount as String:account]}
 static func valid(_ value:[String:Any])->Bool {
  guard (value["version"] as? NSNumber)?.intValue==1,let token=value["access_token"] as? String,!token.isEmpty,token.utf8.count<=8192,let expiry=value["expires_at"] as? NSNumber,let scope=value["scope"] as? String,scope.split(separator:" ").contains("https://www.googleapis.com/auth/youtube.readonly") else{return false}
  let now=Date().timeIntervalSince1970*1000;return expiry.doubleValue>now+5000 && expiry.doubleValue<=now+86_400_000
 }
 static func load() throws -> [String:Any]? {
  var request=query;request[kSecReturnData as String]=true;request[kSecMatchLimit as String]=kSecMatchLimitOne
  var result:CFTypeRef?;let status=SecItemCopyMatching(request as CFDictionary,&result)
  if status==errSecItemNotFound{return nil};guard status==errSecSuccess,let data=result as? Data else{throw NSError(domain:"SecureAccountSession",code:Int(status))}
  guard data.count<=16384,let value=(try? JSONSerialization.jsonObject(with:data)) as? [String:Any],valid(value) else{try clear();return nil};return value
 }
 static func save(_ value:[String:Any]) throws {
  guard valid(value) else{throw NSError(domain:"SecureAccountSession",code:1)}
  let data=try JSONSerialization.data(withJSONObject:value);guard data.count<=16384 else{throw NSError(domain:"SecureAccountSession",code:2)}
  let fields:[String:Any]=[kSecValueData as String:data,kSecAttrAccessible as String:kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly]
  var status=SecItemUpdate(query as CFDictionary,fields as CFDictionary)
  if status==errSecItemNotFound{var item=query;for(k,v) in fields{item[k]=v};status=SecItemAdd(item as CFDictionary,nil)}
  guard status==errSecSuccess else{throw NSError(domain:"SecureAccountSession",code:Int(status))}
 }
 static func clear() throws {let status=SecItemDelete(query as CFDictionary);guard status==errSecSuccess||status==errSecItemNotFound else{throw NSError(domain:"SecureAccountSession",code:Int(status))}}
}
