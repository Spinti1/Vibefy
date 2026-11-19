import UIKit
import WebKit
import UniformTypeIdentifiers
import AVFAudio

@main final class AppDelegate:UIResponder,UIApplicationDelegate {
 var window:UIWindow?;private var controller:MobileController?
 func application(_ app:UIApplication,didFinishLaunchingWithOptions options:[UIApplication.LaunchOptionsKey:Any]?) -> Bool {
  try? AVAudioSession.sharedInstance().setCategory(.playback,mode:.default)
  let window=UIWindow(frame:UIScreen.main.bounds);let controller=MobileController();window.rootViewController=controller;window.makeKeyAndVisible();self.window=window;self.controller=controller;return true
 }
 func application(_ application:UIApplication,open url:URL,options:[UIApplication.OpenURLOptionsKey:Any]=[:])->Bool {controller?.openLink(url) ?? false}
}
private final class WeakShellHandler:NSObject,WKScriptMessageHandler {
 weak var target:WKScriptMessageHandler?
 init(_ target:WKScriptMessageHandler){self.target=target}
 func userContentController(_ controller:WKUserContentController,didReceive message:WKScriptMessage){target?.userContentController(controller,didReceive:message)}
}
final class MobileController:UIViewController,WKNavigationDelegate,WKUIDelegate,WKScriptMessageHandler,UIDocumentPickerDelegate {
 private let home=URL(string:"https://vibefy-neon.vercel.app/web")!
 private var web:WKWebView!;private let cover=UIView();private let stack=UIStackView();private let progress=UIProgressView(progressViewStyle:.bar);private var loading:NSKeyValueObservation?;private var temporaryExport:URL?;private var failed=false
 override var preferredStatusBarStyle:UIStatusBarStyle{.lightContent}
 private func trusted(_ url:URL?)->Bool {guard let url else{return false};return url.scheme=="https" && url.host=="vibefy-neon.vercel.app" && url.port==nil && url.user==nil && url.password==nil}
 override func viewDidLoad(){super.viewDidLoad();view.backgroundColor=UIColor(red:0.055,green:0.071,blue:0.082,alpha:1)
  let config=WKWebViewConfiguration();config.websiteDataStore = .default();config.allowsInlineMediaPlayback=true
  let script="window.vibefyNativeInfo={platform: 'ios',version: '0.2.3'};\n" + ((try? String(contentsOf:Bundle.main.url(forResource:"app-shell",withExtension:"js")!,encoding:.utf8)) ?? "")
  config.userContentController.addUserScript(WKUserScript(source:script,injectionTime:.atDocumentStart,forMainFrameOnly:true));config.userContentController.add(WeakShellHandler(self),name:"vibefyMobile")
  web=WKWebView(frame:.zero,configuration:config);web.navigationDelegate=self;web.uiDelegate=self;web.isOpaque=false;web.backgroundColor=view.backgroundColor;web.scrollView.backgroundColor=view.backgroundColor;web.scrollView.bounces=false;web.allowsBackForwardNavigationGestures=true;web.translatesAutoresizingMaskIntoConstraints=false
  view.addSubview(web);NSLayoutConstraint.activate([web.topAnchor.constraint(equalTo:view.safeAreaLayoutGuide.topAnchor),web.bottomAnchor.constraint(equalTo:view.bottomAnchor),web.leadingAnchor.constraint(equalTo:view.safeAreaLayoutGuide.leadingAnchor),web.trailingAnchor.constraint(equalTo:view.safeAreaLayoutGuide.trailingAnchor)])
  cover.backgroundColor=view.backgroundColor;cover.translatesAutoresizingMaskIntoConstraints=false;view.addSubview(cover);NSLayoutConstraint.activate([cover.topAnchor.constraint(equalTo:web.topAnchor),cover.bottomAnchor.constraint(equalTo:web.bottomAnchor),cover.leadingAnchor.constraint(equalTo:web.leadingAnchor),cover.trailingAnchor.constraint(equalTo:web.trailingAnchor)])
  stack.axis = .vertical;stack.spacing=16;stack.alignment = .center;stack.translatesAutoresizingMaskIntoConstraints=false;cover.addSubview(stack);NSLayoutConstraint.activate([stack.centerXAnchor.constraint(equalTo:cover.centerXAnchor),stack.centerYAnchor.constraint(equalTo:cover.centerYAnchor),stack.widthAnchor.constraint(lessThanOrEqualTo:cover.widthAnchor,constant:-48)])
  let title=UILabel();title.text="Vibefy";title.font = .systemFont(ofSize:34,weight:.bold);title.textColor = .white;stack.addArrangedSubview(title)
  progress.tintColor=UIColor(red:0,green:0.83,blue:0.75,alpha:1);progress.translatesAutoresizingMaskIntoConstraints=false;view.addSubview(progress);NSLayoutConstraint.activate([progress.topAnchor.constraint(equalTo:web.topAnchor),progress.leadingAnchor.constraint(equalTo:web.leadingAnchor),progress.trailingAnchor.constraint(equalTo:web.trailingAnchor)])
  loading=web.observe(\.estimatedProgress,options:[.new]){[weak self] web,_ in self?.progress.progress=Float(web.estimatedProgress);self?.progress.isHidden=web.estimatedProgress>=1}
  web.load(URLRequest(url:home,cachePolicy:.useProtocolCachePolicy,timeoutInterval:25))
 }
 @discardableResult func openLink(_ url:URL)->Bool {
  if url.scheme=="vibefy" && url.host=="connect" {finishGoogle(url);return true};return openJam(url)
 }
 @discardableResult func openJam(_ url:URL)->Bool {
  guard url.scheme=="vibefy",url.host=="jam",let code=URLComponents(url:url,resolvingAgainstBaseURL:false)?.queryItems?.first(where:{$0.name=="code"})?.value,code.range(of:"^[ABCDEFGHJKLMNPQRSTUVWXYZ23456789]{12}$",options:.regularExpression) != nil else{return false}
  loadViewIfNeeded();web.load(URLRequest(url:URL(string:home.absoluteString+"?jam="+code)!));return true
 }
 func webView(_ webView:WKWebView,didStartProvisionalNavigation navigation:WKNavigation!){failed=false}
 func webView(_ webView:WKWebView,didCommit navigation:WKNavigation!){if trusted(web.url){cover.isHidden=true}}
 func webView(_ webView:WKWebView,didFinish navigation:WKNavigation!){if trusted(web.url) && !failed {cover.isHidden=true;let script=(try? String(contentsOf:Bundle.main.url(forResource:"app-shell",withExtension:"js")!,encoding:.utf8)) ?? "";web.evaluateJavaScript(script,completionHandler:nil)}}
 func webView(_ webView:WKWebView,decidePolicyFor action:WKNavigationAction,decisionHandler:@escaping(WKNavigationActionPolicy)->Void){
  guard action.targetFrame?.isMainFrame != false,let url=action.request.url else{decisionHandler(.allow);return}
  if trusted(url){decisionHandler(.allow)}else{decisionHandler(.cancel);if action.navigationType == .linkActivated && url.scheme=="https"{UIApplication.shared.open(url)}}
 }
 func webView(_ webView:WKWebView,createWebViewWith configuration:WKWebViewConfiguration,for action:WKNavigationAction,windowFeatures:WKWindowFeatures)->WKWebView? {
  if action.navigationType == .linkActivated,let url=action.request.url,url.scheme=="https" {UIApplication.shared.open(url);return nil}
  return nil
 }
 func webView(_ webView:WKWebView,didFailProvisionalNavigation navigation:WKNavigation!,withError error:Error){if (error as NSError).code != NSURLErrorCancelled{offline()}}
 func webView(_ webView:WKWebView,didFail navigation:WKNavigation!,withError error:Error){if (error as NSError).code != NSURLErrorCancelled{offline()}}
 func webViewWebContentProcessDidTerminate(_ webView:WKWebView){web.reload()}
 private func offline(){failed=true;progress.isHidden=true;for child in stack.arrangedSubviews{stack.removeArrangedSubview(child);child.removeFromSuperview()};let label=UILabel();label.text="Cannot connect right now. Your saved playlists are kept.";label.textColor = .white;label.numberOfLines=0;label.textAlignment = .center;stack.addArrangedSubview(label);let retry=UIButton(type:.system);retry.setTitle("Try again",for:.normal);retry.addTarget(self,action:#selector(reload),for:.touchUpInside);stack.addArrangedSubview(retry);cover.isHidden=false}
 @objc private func reload(){web.load(URLRequest(url:home))}
 func userContentController(_ controller:WKUserContentController,didReceive message:WKScriptMessage){
  guard message.frameInfo.isMainFrame,trusted(message.frameInfo.request.url),let packet=message.body as? [String:Any] else{return}
  if packet["action"] as? String=="connectGoogle",let raw=packet["url"] as? String {openGoogle(raw);return}
  guard packet["action"] as? String=="exportBackup",let content=packet["content"] as? String,content.utf8.count<=6_000_000,let data=content.data(using:.utf8),let json=(try? JSONSerialization.jsonObject(with:data)) as? [String:Any],json["collections"] is [Any],json["likes"] is [Any],temporaryExport==nil else{return}
  let file=FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString+"-vibefy-library.json");temporaryExport=file
  DispatchQueue.global(qos:.userInitiated).async {do {try data.write(to:file,options:.atomic);DispatchQueue.main.async {let picker=UIDocumentPickerViewController(forExporting:[file],asCopy:true);picker.delegate=self;self.present(picker,animated:true)}}catch{DispatchQueue.main.async{self.temporaryExport=nil}}}
 }
 private var googleState:String?;private var googleStarted=Date.distantPast
 private func openGoogle(_ raw:String){
  guard raw.count<1000,let url=URL(string:raw),trusted(url),url.path=="/mobile-connect.html",let fragment=url.fragment,let parts=URLComponents(string:"https://local/?"+fragment),let state=parts.queryItems?.first(where:{$0.name=="state"})?.value,state.range(of:"^[A-Za-z0-9_-]{32}$",options:.regularExpression) != nil else{return}
  googleState=state;googleStarted=Date();UIApplication.shared.open(url)
 }
 private func finishGoogle(_ url:URL){
  guard url.absoluteString.count<16000,Date().timeIntervalSince(googleStarted)<300,let state=googleState,let fragment=url.fragment,let parts=URLComponents(string:"https://local/?"+fragment),parts.queryItems?.first(where:{$0.name=="state"})?.value==state else{return}
  googleState=nil;guard trusted(web.url),let data=try? JSONSerialization.data(withJSONObject:[url.absoluteString]),let array=String(data:data,encoding:.utf8) else{return}
  web.evaluateJavaScript("window.vibefyMobileAuthResult?.("+array+"[0])",completionHandler:nil)
 }
 private func cleanExport(){if let file=temporaryExport{try? FileManager.default.removeItem(at:file);temporaryExport=nil}}
 func documentPickerWasCancelled(_ controller:UIDocumentPickerViewController){cleanExport()}
 func documentPicker(_ controller:UIDocumentPickerViewController,didPickDocumentsAt urls:[URL]){cleanExport()}
 deinit{loading?.invalidate()}
}
