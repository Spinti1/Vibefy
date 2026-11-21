import UIKit
import Network

/// Foreground-only Bonjour discovery; only a bounded Jam invitation crosses LAN.
final class NearbyReceiver {
 private weak var controller:UIViewController?
 private let join:(String)->Void
 private let token=UUID().uuidString.replacingOccurrences(of:"-",with:"").lowercased()
 private var listener:NWListener?
 private var connections:[ObjectIdentifier:NWConnection]=[:]
 private var running=true
 private var lastPrompt=Date.distantPast
 private weak var alert:UIAlertController?
 init(controller:UIViewController,join:@escaping(String)->Void){
  self.controller=controller;self.join=join
  do {
   let parameters=NWParameters.tcp;parameters.includePeerToPeer=false
   let listener=try NWListener(using:parameters,on:.any);self.listener=listener
   let txt=NetService.data(fromTXTRecord:["v":Data("1".utf8),"platform":Data("ios".utf8),"token":Data(token.utf8)])
   listener.service=NWListener.Service(name:"Vibefy iPhone",type:"_vibefy._tcp",domain:nil,txtRecord:txt)
   listener.newConnectionHandler={[weak self] connection in
    guard let self,self.running,self.connections.count<4 else{connection.cancel();return}
    self.connections[ObjectIdentifier(connection)]=connection;connection.start(queue:.main)
    self.read(connection,Data())
    DispatchQueue.main.asyncAfter(deadline:.now()+3){[weak self,weak connection] in if let connection{self?.finish(connection)}}
   }
   listener.start(queue:.main)
  }catch{running=false}
 }
 private func finish(_ connection:NWConnection){connections.removeValue(forKey:ObjectIdentifier(connection));connection.cancel()}
 private func read(_ connection:NWConnection,_ accumulated:Data){
  connection.receive(minimumIncompleteLength:1,maximumLength:128){[weak self] data,_,complete,error in
   guard let self,self.running else{connection.cancel();return}
   var bytes=accumulated;if let data{bytes.append(data)}
   guard bytes.count<=128,error==nil else{self.finish(connection);return}
   if let end=bytes.firstIndex(of:10),let line=String(data:bytes[..<end],encoding:.ascii){
    let prefix="VIBEFY_PAIR_V1 "+self.token+" "
    guard line.hasPrefix(prefix) else{self.finish(connection);return}
    let code=String(line.dropFirst(prefix.count))
    guard code.count==12,code.utf8.allSatisfy({Data("ABCDEFGHJKLMNPQRSTUVWXYZ23456789".utf8).contains($0)}) else{self.finish(connection);return}
    connection.send(content:Data("READY\n".utf8),completion:.contentProcessed{[weak self] _ in self?.finish(connection)})
    self.confirm(code)
   }else if complete||bytes.count==128{self.finish(connection)}else{self.read(connection,bytes)}
  }
 }
 private func confirm(_ code:String){
  guard running,UIApplication.shared.applicationState == .active,Date().timeIntervalSince(lastPrompt)>10,alert==nil,let controller,controller.presentedViewController==nil else{return}
  lastPrompt=Date()
  let alert=UIAlertController(title:"Connect to Vibefy desktop?",message:"A device on your Wi-Fi invited this phone to shared playback. Connect only if you just selected this phone on your computer. Both devices can control the music.",preferredStyle:.alert)
  alert.addAction(UIAlertAction(title:"Cancel",style:.cancel))
  alert.addAction(UIAlertAction(title:"Connect",style:.default){[weak self] _ in guard let self,self.running else{return};self.join(code)})
  self.alert=alert;controller.present(alert,animated:true)
 }
 func close(){running=false;alert?.dismiss(animated:false);listener?.cancel();listener=nil;for connection in connections.values{connection.cancel()};connections.removeAll()}
 deinit{listener?.cancel();for connection in connections.values{connection.cancel()}}
}
