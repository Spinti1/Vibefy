import UIKit
import WebKit

@main final class AppDelegate: UIResponder, UIApplicationDelegate {
 var window: UIWindow?
 func application(_ application: UIApplication, didFinishLaunchingWithOptions options: [UIApplication.LaunchOptionsKey: Any]?) -> Bool {
  let window=UIWindow(frame: UIScreen.main.bounds)
  window.rootViewController=UINavigationController(rootViewController: PlayerController())
  window.makeKeyAndVisible();self.window=window;return true
 }
}
final class PlayerController: UIViewController, WKNavigationDelegate, WKUIDelegate {
 private let home=URL(string:"https://vibefy-neon.vercel.app/web")!
 private var web: WKWebView!
 private func internalURL(_ url:URL)->Bool {url.scheme == "https" && url.host == "vibefy-neon.vercel.app"}
 override func loadView() {
  let config=WKWebViewConfiguration();config.allowsInlineMediaPlayback=true
  config.websiteDataStore = .default()
  web=WKWebView(frame:.zero,configuration:config);web.navigationDelegate=self;web.uiDelegate=self;view=web
 }
 override func viewDidLoad() {
  super.viewDidLoad();title="Vibefy"
  navigationItem.leftBarButtonItem=UIBarButtonItem(title:"Back",style:.plain,target:self,action:#selector(back))
  navigationItem.rightBarButtonItems=[UIBarButtonItem(title:"Browser",style:.plain,target:self,action:#selector(browser)),UIBarButtonItem(barButtonSystemItem:.refresh,target:self,action:#selector(reload))]
  reload()
 }
 @objc private func reload(){web.load(URLRequest(url:home))}
 @objc private func back(){if web.canGoBack {web.goBack()}}
 @objc private func browser(){UIApplication.shared.open(web.url.flatMap{internalURL($0) ? $0:nil} ?? home)}
 func webView(_ webView: WKWebView, decidePolicyFor action: WKNavigationAction, decisionHandler: @escaping (WKNavigationActionPolicy)->Void) {
  guard action.targetFrame?.isMainFrame != false,let url=action.request.url else {decisionHandler(.allow);return}
  if internalURL(url){decisionHandler(.allow)}else{decisionHandler(.cancel);if url.scheme == "https" {UIApplication.shared.open(url)}}
 }
 func webView(_ webView: WKWebView, createWebViewWith configuration: WKWebViewConfiguration, for action: WKNavigationAction, windowFeatures: WKWindowFeatures) -> WKWebView? {
  let alert=UIAlertController(title:"Continue in browser",message:"Google sign-in is not supported inside this preview. Open the web player in Safari to connect. Browser and app playlists are separate; use playlist backups to transfer them.",preferredStyle:.alert)
  alert.addAction(UIAlertAction(title:"Open browser",style:.default){_ in self.browser()});alert.addAction(UIAlertAction(title:"Cancel",style:.cancel));present(alert,animated:true);return nil
 }
 func webView(_ webView: WKWebView,didFailProvisionalNavigation navigation:WKNavigation!,withError error:Error){
  guard (error as NSError).code != NSURLErrorCancelled else{return}
  let alert=UIAlertController(title:"Cannot connect",message:"Check your connection and tap Reload.",preferredStyle:.alert);alert.addAction(UIAlertAction(title:"OK",style:.default));present(alert,animated:true)
 }
}
