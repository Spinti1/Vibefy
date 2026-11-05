//! Vibefy's own first-party YouTube session window. No third-party app code.
//! The page has no native command bindings. Its only message is the bounded,
//! nonce-correlated result of a request initiated by the native backend.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

pub const ORIGIN: &str = "https://music.youtube.com";
pub const SIGN_IN_URL: &str = "https://accounts.google.com/ServiceLogin?service=youtube&continue=https%3A%2F%2Fmusic.youtube.com%2F";
const MAX_RESPONSE: usize = 8 * 1024 * 1024;
#[derive(Clone, Serialize, Deserialize)]
pub struct Profile {
    id: String,
    #[serde(skip)]
    root: PathBuf,
}
impl Profile {
    pub fn bind(&mut self, config: &Path) -> Result<()> {
        ensure!(valid_id(&self.id), "Invalid YouTube session profile");
        self.root = config.join("youtube-browser");
        Ok(())
    }
    fn directory(&self) -> Result<PathBuf> {
        ensure!(
            valid_id(&self.id) && self.root.is_absolute(),
            "Reconnect YouTube to initialize its profile"
        );
        let dir = self.root.join(&self.id);
        for path in [&self.root, &dir] {
            if let Ok(meta) = std::fs::symlink_metadata(path) {
                ensure!(
                    !meta.file_type().is_symlink(),
                    "YouTube profile paths cannot be links"
                );
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    ensure!(
                        meta.file_attributes() & 0x400 == 0,
                        "YouTube profile paths cannot be junctions"
                    );
                }
            }
        }
        Ok(dir)
    }
}
pub fn valid_id(value: &str) -> bool {
    value.len() == 32 && value.bytes().all(|b| b.is_ascii_hexdigit())
}
fn google_host(host: &str) -> bool {
    // Snapshot from Google's own supported_domains endpoint, not a google.* rule.
    include_str!("../assets/google-signin-domains.txt")
        .lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
        .any(|domain| {
            host.strip_suffix(domain)
                .is_some_and(|prefix| prefix.is_empty() || prefix.ends_with('.'))
        })
}
pub fn allowed_navigation(url: &str) -> bool {
    reqwest::Url::parse(url).is_ok_and(|u| {
        u.scheme() == "https"
            && u.username().is_empty()
            && u.password().is_none()
            && u.port().is_none()
            && u.host_str().is_some_and(|h| {
                google_host(h) || h == "youtube.com" || h.ends_with(".youtube.com")
            })
    })
}
pub fn music_origin(url: &str) -> bool {
    reqwest::Url::parse(url).is_ok_and(|u| {
        u.scheme() == "https"
            && u.host_str() == Some("music.youtube.com")
            && u.port().is_none()
            && u.username().is_empty()
            && u.password().is_none()
    })
}
/// Deliberately excludes URLs, paths, queries, fragment, account names and cookies.
pub fn navigation_diagnostic(url: &str, event: &str, elapsed_seconds: u64) -> Value {
    let host = reqwest::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(str::to_owned));
    serde_json::json!({"event":event,"host":host,"seconds":elapsed_seconds})
}
pub fn valid_endpoint(endpoint: &str) -> bool {
    matches!(
        endpoint,
        "browse"
            | "account/account_menu"
            | "playlist/create"
            | "playlist/delete"
            | "browse/edit_playlist"
    )
}
fn nonce() -> String {
    use rand::RngCore;
    let mut bytes = [0; 16];
    rand::rng().fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
pub struct Connection {
    pub profile: Profile,
    pub account: String,
    pub page: Option<String>,
}
pub struct Response {
    pub status: u16,
    pub retry_after: Option<String>,
    pub body: Value,
}
pub fn response_message(source: &str, expected: &str, message: &str) -> Result<Option<Response>> {
    if !music_origin(source) {
        return Ok(None);
    }
    ensure!(
        message.len() <= MAX_RESPONSE * 2,
        "YouTube response is too large"
    );
    let value: Value =
        serde_json::from_str(message).map_err(|_| anyhow::anyhow!("Invalid YouTube response"))?;
    if value["id"].as_str() != Some(expected) {
        return Ok(None);
    }
    ensure!(
        value["error"].is_null(),
        "YouTube did not confirm the request"
    );
    let status = value["status"]
        .as_u64()
        .filter(|n| (100..600).contains(n))
        .ok_or_else(|| anyhow::anyhow!("Invalid YouTube response status"))? as u16;
    let text = value["body"]
        .as_str()
        .filter(|s| s.len() <= MAX_RESPONSE)
        .ok_or_else(|| anyhow::anyhow!("YouTube response is too large"))?;
    Ok(Some(Response {
        status,
        retry_after: value["retryAfter"].as_str().map(str::to_owned),
        body: serde_json::from_str(text)
            .map_err(|_| anyhow::anyhow!("YouTube did not return a JSON response"))?,
    }))
}

#[cfg(windows)]
pub async fn connect(config: &Path, proxy: &crate::settings::ProxyConfig) -> Result<Connection> {
    let mut profile = Profile {
        id: nonce(),
        root: PathBuf::new(),
    };
    profile.bind(config)?;
    let value = native::submit(
        profile.clone(),
        proxy
            .youtube_browser_arguments()
            .map_err(anyhow::Error::msg)?,
        native::Operation::Connect,
    )
    .await?;
    let account = value["account"].as_str().unwrap_or("0");
    ensure!(
        !account.is_empty() && account.len() <= 3 && account.bytes().all(|b| b.is_ascii_digit()),
        "Invalid YouTube account selector"
    );
    let page = value["page"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(str::to_owned);
    ensure!(
        page.as_ref().is_none_or(|s| s.len() <= 128
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')),
        "Invalid YouTube channel selector"
    );
    Ok(Connection {
        profile,
        account: account.into(),
        page,
    })
}
#[cfg(not(windows))]
pub async fn connect(_config: &Path, _proxy: &crate::settings::ProxyConfig) -> Result<Connection> {
    anyhow::bail!("The dedicated YouTube sign-in window is currently available on Windows only")
}
pub async fn post(
    profile: &Profile,
    proxy: &crate::settings::ProxyConfig,
    endpoint: &str,
    account: &str,
    body: Value,
    continuation: Option<&str>,
) -> Result<Response> {
    ensure!(valid_endpoint(endpoint), "Unsupported YouTube operation");
    #[cfg(windows)]
    {
        let value = native::submit(
            profile.clone(),
            proxy
                .youtube_browser_arguments()
                .map_err(anyhow::Error::msg)?,
            native::Operation::Post {
                endpoint: endpoint.into(),
                account: account.into(),
                body,
                continuation: continuation.map(str::to_owned),
            },
        )
        .await?;
        return response_message(
            ORIGIN,
            value["id"].as_str().unwrap_or_default(),
            &serde_json::to_string(&value)?,
        )?
        .ok_or_else(|| anyhow::anyhow!("YouTube did not confirm the request"));
    }
    #[cfg(not(windows))]
    {
        let _ = (profile, proxy, account, body, continuation);
        anyhow::bail!("Reconnect YouTube on a supported platform")
    }
}
pub async fn forget(profile: &Profile) -> Result<()> {
    #[cfg(windows)]
    {
        native::submit(profile.clone(), None, native::Operation::Forget).await?;
    }
    Ok(())
}

#[cfg(windows)]
mod native {
    use super::*;
    use anyhow::Result;
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
        sync::{
            OnceLock,
            atomic::{AtomicU32, Ordering},
            mpsc,
        },
        time::{Duration, Instant},
    };
    use tokio::sync::oneshot;
    use webview2_com::{Microsoft::Web::WebView2::Win32::*, *};
    use windows_webview::{
        Win32::{
            Foundation::*,
            System::{
                Com::*, LibraryLoader::GetModuleHandleW, Threading::GetCurrentThreadId,
                WinRT::EventRegistrationToken,
            },
            UI::WindowsAndMessaging::*,
        },
        core::{HSTRING, Interface, PCWSTR, PWSTR, w},
    };
    pub enum Operation {
        Connect,
        Post {
            endpoint: String,
            account: String,
            body: Value,
            continuation: Option<String>,
        },
        Forget,
    }
    struct Job {
        profile: Profile,
        args: Option<String>,
        op: Operation,
        reply: oneshot::Sender<std::result::Result<Value, String>>,
    }
    struct Worker {
        sender: mpsc::Sender<Job>,
        thread: AtomicU32,
    }
    static WORKER: OnceLock<Worker> = OnceLock::new();
    pub async fn submit(profile: Profile, args: Option<String>, op: Operation) -> Result<Value> {
        let worker = WORKER.get_or_init(|| {
            let (sender, rx) = mpsc::channel();
            std::thread::Builder::new()
                .name("youtube-session-window".into())
                .spawn(move || run(rx))
                .expect("YouTube session thread");
            Worker {
                sender,
                thread: AtomicU32::new(0),
            }
        });
        let (tx, rx) = oneshot::channel();
        worker
            .sender
            .send(Job {
                profile,
                args,
                op,
                reply: tx,
            })
            .map_err(|_| anyhow::anyhow!("The YouTube window is unavailable"))?;
        let thread = worker.thread.load(Ordering::Acquire);
        if thread != 0 {
            unsafe {
                let _ = PostThreadMessageW(thread, WM_APP, WPARAM(0), LPARAM(0));
            }
        }
        rx.await
            .map_err(|_| anyhow::anyhow!("The YouTube connection was interrupted"))?
            .map_err(anyhow::Error::msg)
    }
    struct WindowState {
        controller: RefCell<Option<ICoreWebView2Controller>>,
        closed: Cell<bool>,
    }
    unsafe extern "system" fn proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        unsafe {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowState;
            if !ptr.is_null() {
                if msg == WM_CLOSE {
                    (*ptr).closed.set(true);
                    let _ = ShowWindow(hwnd, SW_HIDE);
                    return LRESULT(0);
                }
                if msg == WM_SIZE {
                    if let Some(controller) = &*(*ptr).controller.borrow() {
                        let mut r = RECT::default();
                        let _ = GetClientRect(hwnd, &mut r);
                        let _ = controller.SetBounds(r);
                    }
                }
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
    }
    struct NavigationTrace {
        started: Instant,
        events: Vec<Value>,
        sequence: u64,
        blocked: bool,
    }
    impl NavigationTrace {
        fn record(&mut self, url: &str, event: &str) {
            self.sequence = self.sequence.wrapping_add(1);
            if self.events.len() >= 48 {
                self.events.remove(0);
            }
            self.events.push(navigation_diagnostic(
                url,
                event,
                self.started.elapsed().as_secs(),
            ));
        }
    }
    fn flush_trace(browser: &Browser) {
        let trace = browser.trace.borrow();
        let path = browser.profile.root.join("signin-diagnostic.json");
        if let Ok(bytes) = serde_json::to_vec_pretty(
            &serde_json::json!({"version":env!("CARGO_PKG_VERSION"),"events":trace.events}),
        ) {
            let _ = std::fs::write(path, bytes);
        }
    }
    struct Browser {
        controller: ICoreWebView2Controller,
        view: ICoreWebView2,
        _environment: ICoreWebView2Environment,
        profile: Profile,
        args: Option<String>,
        trace: Rc<RefCell<NavigationTrace>>,
    }
    impl Drop for Browser {
        fn drop(&mut self) {
            unsafe {
                let _ = self.controller.Close();
            }
        }
    }
    fn error() -> anyhow::Error {
        anyhow::anyhow!(
            "Cannot initialize the YouTube window. Install or update Microsoft Edge WebView2 Runtime from Microsoft's official website."
        )
    }
    fn com_error(stage: &str, error: windows_webview::core::Error) -> anyhow::Error {
        anyhow::anyhow!(
            "YouTube window: {stage} failed (HRESULT 0x{:08X}). Check Microsoft Edge WebView2 Runtime and folder permissions.",
            error.code().0 as u32
        )
    }
    fn pump() {
        unsafe {
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }
    fn wait<T>(
        rx: mpsc::Receiver<T>,
        reply: &oneshot::Sender<std::result::Result<Value, String>>,
        state: &WindowState,
        seconds: u64,
    ) -> Result<T> {
        let end = Instant::now() + Duration::from_secs(seconds);
        loop {
            pump();
            if let Ok(value) = rx.try_recv() {
                return Ok(value);
            }
            ensure!(
                !reply.is_closed() && !state.closed.get(),
                "YouTube connection cancelled"
            );
            ensure!(Instant::now() < end, "YouTube connection timed out");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    unsafe fn text(
        call: impl FnOnce(*mut PWSTR) -> windows_webview::core::Result<()>,
    ) -> windows_webview::core::Result<String> {
        let mut p = PWSTR::null();
        call(&mut p)?;
        let result = unsafe { p.to_string() }.unwrap_or_default();
        unsafe {
            CoTaskMemFree(Some(p.0.cast()));
        }
        Ok(result)
    }
    fn open(hwnd: HWND, state: &WindowState, job: &Job) -> Result<Browser> {
        let directory = job.profile.directory()?;
        std::fs::create_dir_all(&directory)?;
        let (tx, rx) = mpsc::channel();
        let options: ICoreWebView2EnvironmentOptions =
            CoreWebView2EnvironmentOptions::default().into();
        if let Some(args) = &job.args {
            unsafe { options.SetAdditionalBrowserArguments(&HSTRING::from(args)) }
                .map_err(|_| error())?;
        }
        let handler =
            CreateCoreWebView2EnvironmentCompletedHandler::create(Box::new(move |result, env| {
                let _ =
                    tx.send(result.and_then(|_| {
                        env.ok_or_else(|| windows_webview::core::Error::from(E_FAIL))
                    }));
                Ok(())
            }));
        unsafe {
            CreateCoreWebView2EnvironmentWithOptions(
                PCWSTR::null(),
                &HSTRING::from(directory.as_os_str()),
                &options,
                &handler,
            )
        }
        .map_err(|e| com_error("create environment", e))?;
        let env =
            wait(rx, &job.reply, state, 45)?.map_err(|e| com_error("environment callback", e))?;
        let (tx, rx) = mpsc::channel();
        let handler = CreateCoreWebView2ControllerCompletedHandler::create(Box::new(
            move |result, controller| {
                let _ = tx.send(result.and_then(|_| {
                    controller.ok_or_else(|| windows_webview::core::Error::from(E_FAIL))
                }));
                Ok(())
            },
        ));
        unsafe { env.CreateCoreWebView2Controller(hwnd, &handler) }
            .map_err(|e| com_error("create controller", e))?;
        let controller =
            wait(rx, &job.reply, state, 45)?.map_err(|e| com_error("controller callback", e))?;
        let view = unsafe { controller.CoreWebView2() }.map_err(|e| com_error("get view", e))?;
        unsafe {
            let mut r = RECT::default();
            GetClientRect(hwnd, &mut r).map_err(|_| error())?;
            controller.SetBounds(r).map_err(|_| error())?;
            controller.SetIsVisible(true).map_err(|_| error())?;
        }
        let settings = unsafe { view.Settings() }.map_err(|_| error())?;
        unsafe {
            let _ = settings.SetAreDevToolsEnabled(false);
        }
        if let Ok(settings) = settings.cast::<ICoreWebView2Settings4>() {
            unsafe {
                let _ = settings.SetIsPasswordAutosaveEnabled(false);
                let _ = settings.SetIsGeneralAutofillEnabled(false);
            }
        }
        let trace = Rc::new(RefCell::new(NavigationTrace {
            started: Instant::now(),
            events: Vec::new(),
            sequence: 0,
            blocked: false,
        }));
        let navigation_trace = trace.clone();
        let mut token = EventRegistrationToken::default();
        let nav = NavigationStartingEventHandler::create(Box::new(move |_, args| {
            if let Some(args) = args {
                unsafe {
                    let uri = text(|p| args.Uri(p))?;
                    {
                        let mut trace = navigation_trace.borrow_mut();
                        trace.record(&uri, "starting");
                    }
                    if !allowed_navigation(&uri) {
                        {
                            let mut trace = navigation_trace.borrow_mut();
                            trace.blocked = true;
                            trace.record(&uri, "blocked");
                        }
                        args.SetCancel(true)?;
                    } else if let Ok(url) = reqwest::Url::parse(&uri) {
                        let _ = SetWindowTextW(
                            hwnd,
                            &HSTRING::from(format!(
                                "Vibefy · {}",
                                url.host_str().unwrap_or_default()
                            )),
                        );
                    }
                }
            }
            Ok(())
        }));
        unsafe { view.add_NavigationStarting(&nav, &mut token) }.map_err(|_| error())?;
        let completed_trace = trace.clone();
        let completed = NavigationCompletedEventHandler::create(Box::new(move |sender, args| {
            if let Some(args) = args {
                unsafe {
                    let url = sender
                        .and_then(|v| text(|p| v.Source(p)).ok())
                        .unwrap_or_default();
                    let mut success = BOOL::default();
                    let mut status = COREWEBVIEW2_WEB_ERROR_STATUS::default();
                    args.IsSuccess(&mut success)?;
                    args.WebErrorStatus(&mut status)?;
                    let mut trace = completed_trace.borrow_mut();
                    trace.record(
                        &url,
                        if success.as_bool() {
                            "completed"
                        } else {
                            "failed"
                        },
                    );
                    if let Some(event) = trace.events.last_mut() {
                        event["webErrorStatus"] = status.0.into();
                        if let Ok(args) = args.cast::<ICoreWebView2NavigationCompletedEventArgs2>()
                        {
                            let mut http = 0;
                            if args.HttpStatusCode(&mut http).is_ok() {
                                event["httpStatus"] = http.into();
                            }
                        }
                    }
                }
            }
            Ok(())
        }));
        unsafe { view.add_NavigationCompleted(&completed, &mut token) }.map_err(|_| error())?;
        let popup_trace = trace.clone();
        let popup = NewWindowRequestedEventHandler::create(Box::new(move |_, args| {
            if let Some(args) = args {
                unsafe {
                    let uri = text(|p| args.Uri(p))?;
                    popup_trace.borrow_mut().record(&uri, "popup_requested");
                }
            }
            Ok(())
        }));
        unsafe { view.add_NewWindowRequested(&popup, &mut token) }.map_err(|_| error())?;
        if let Ok(muted) = view.cast::<ICoreWebView2_8>() {
            unsafe {
                let _ = muted.SetIsMuted(true);
            }
        }
        *state.controller.borrow_mut() = Some(controller.clone());
        Ok(Browser {
            controller,
            view,
            _environment: env,
            profile: job.profile.clone(),
            args: job.args.clone(),
            trace,
        })
    }
    fn source(browser: &Browser) -> String {
        unsafe { text(|p| browser.view.Source(p)) }.unwrap_or_default()
    }
    fn script(browser: &Browser, code: &str, job: &Job, state: &WindowState) -> Result<Value> {
        let (tx, rx) = mpsc::channel();
        let handler = ExecuteScriptCompletedHandler::create(Box::new(move |result, value| {
            let _ = tx.send(result.map(|_| value));
            Ok(())
        }));
        unsafe { browser.view.ExecuteScript(&HSTRING::from(code), &handler) }
            .map_err(|_| error())?;
        let value = wait(rx, &job.reply, state, 5)?.map_err(|_| error())?;
        let value: Value = serde_json::from_str(&value).unwrap_or(Value::Null);
        Ok(if let Some(s) = value.as_str() {
            serde_json::from_str(s).unwrap_or(Value::Null)
        } else {
            value
        })
    }
    fn signing_cookie(browser: &Browser, job: &Job, state: &WindowState) -> Result<Option<String>> {
        let view: ICoreWebView2_2 = browser.view.cast().map_err(|_| error())?;
        let manager = unsafe { view.CookieManager() }.map_err(|_| error())?;
        let (tx, rx) = mpsc::channel();
        let handler = GetCookiesCompletedHandler::create(Box::new(move |result, list| {
            let _ = tx.send(
                result.and_then(|_| list.ok_or_else(|| windows_webview::core::Error::from(E_FAIL))),
            );
            Ok(())
        }));
        unsafe { manager.GetCookies(&HSTRING::from(ORIGIN), &handler) }.map_err(|_| error())?;
        let list = wait(rx, &job.reply, state, 5)?.map_err(|_| error())?;
        let mut signing = None;
        let mut sid = false;
        let mut count = 0;
        unsafe { list.Count(&mut count) }.map_err(|_| error())?;
        ensure!(count <= 1000, "Unexpected YouTube cookie count");
        for i in 0..count {
            unsafe {
                let cookie = list.GetValueAtIndex(i).map_err(|_| error())?;
                let name = text(|p| cookie.Name(p)).map_err(|_| error())?;
                if matches!(name.as_str(), "SID" | "__Secure-1PSID" | "__Secure-3PSID") {
                    sid = true;
                }
                if name == "SAPISID" || name == "__Secure-3PAPISID" {
                    let value = text(|p| cookie.Value(p)).map_err(|_| error())?;
                    if !value.is_empty() {
                        signing = Some(value);
                    }
                }
            }
        }
        Ok(if sid { signing } else { None })
    }
    fn ready(
        browser: &Browser,
        job: &Job,
        state: &WindowState,
        visible: bool,
    ) -> Result<(String, Value)> {
        let end = Instant::now() + Duration::from_secs(if visible { 600 } else { 45 });
        let mut next = Instant::now();
        let mut flushed = u64::MAX;
        loop {
            pump();
            ensure!(
                !job.reply.is_closed() && !state.closed.get(),
                "YouTube connection cancelled"
            );
            ensure!(
                Instant::now() < end,
                "YouTube session needs reconnecting. Click Connect YouTube."
            );
            let sequence = browser.trace.borrow().sequence;
            if visible && sequence != flushed {
                flush_trace(browser);
                flushed = sequence;
            }
            ensure!(
                !browser.trace.borrow().blocked,
                "YouTube sign-in encountered an unsupported redirect. Local diagnostic saved in the YouTube profile folder."
            );
            if Instant::now() >= next {
                next = Instant::now() + Duration::from_millis(1200);
                let current = source(browser);
                if music_origin(&current) {
                    if let Some(signing) = signing_cookie(browser, job, state)? {
                        let context = script(
                            browser,
                            "JSON.stringify({account:String(globalThis.ytcfg?.get('SESSION_INDEX') ?? '0'),page:globalThis.ytcfg?.get('DELEGATED_SESSION_ID') ?? null,logged:globalThis.ytcfg?.get('LOGGED_IN') ?? false})",
                            job,
                            state,
                        )?;
                        if context["logged"] == true {
                            return Ok((signing, context));
                        }
                    }
                } else if !visible && source(browser).contains("accounts.google.com") {
                    anyhow::bail!(
                        "Your YouTube session expired. Click Connect YouTube to sign in again."
                    );
                }
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    fn post(
        browser: &Browser,
        job: &Job,
        state: &WindowState,
        endpoint: &str,
        account: &str,
        body: &Value,
        continuation: Option<&str>,
    ) -> Result<Value> {
        let (sapisid, _) = ready(browser, job, state, false)?;
        use sha1::{Digest, Sha1};
        let timestamp = crate::playlist_mirror::now();
        let hash = Sha1::digest(format!("{timestamp} {sapisid} {ORIGIN}").as_bytes());
        let authorization = format!("SAPISIDHASH {timestamp}_{hash:x}");
        let id = nonce();
        let id_check = id.clone();
        let (tx, rx) = mpsc::channel();
        let mut token = EventRegistrationToken::default();
        let handler = WebMessageReceivedEventHandler::create(Box::new(move |_, args| {
            if let Some(args) = args {
                unsafe {
                    let source = text(|p| args.Source(p))?;
                    if music_origin(&source) {
                        let message = text(|p| args.WebMessageAsJson(p))?;
                        if message.len() <= MAX_RESPONSE * 2 {
                            if let Ok(value) = serde_json::from_str::<Value>(&message) {
                                if value["id"].as_str() == Some(&id_check) {
                                    let _ = tx.send(value);
                                }
                            }
                        }
                    }
                }
            }
            Ok(())
        }));
        unsafe { browser.view.add_WebMessageReceived(&handler, &mut token) }
            .map_err(|_| error())?;
        let mut url = reqwest::Url::parse(&format!("{ORIGIN}/youtubei/v1/{endpoint}"))?;
        url.query_pairs_mut().append_pair("alt", "json");
        if let Some(next) = continuation {
            url.query_pairs_mut()
                .append_pair("continuation", next)
                .append_pair("ctoken", next);
        }
        let input = serde_json::json!({"id":id,"url":url.as_str(),"authorization":authorization,"account":account,"body":body});
        let code = format!(
            "(()=>{{const p={input}; const c=globalThis.ytcfg?.get('INNERTUBE_CONTEXT'); if(c?.client)p.body.context={{...c,...p.body.context,client:c.client,user:{{...c.user,...p.body.context?.user}}}}; if(location.origin!=={origin})return; (async()=>{{try{{const r=await fetch(p.url,{{method:'POST',credentials:'include',redirect:'error',headers:{{'Content-Type':'application/json','Authorization':p.authorization,'X-Goog-AuthUser':p.account,'X-Origin':{origin},'X-Youtube-Client-Name':'67','X-Youtube-Client-Version':String(p.body.context.client.clientVersion)}},body:JSON.stringify(p.body),signal:AbortSignal.timeout(20000)}});const t=await r.text();if(t.length>{MAX_RESPONSE})throw Error();chrome.webview.postMessage({{id:p.id,status:r.status,retryAfter:r.headers.get('retry-after'),body:t}});}}catch(_){{chrome.webview.postMessage({{id:p.id,error:true}});}}}})();}})()",
            origin = serde_json::to_string(ORIGIN)?
        );
        let outcome = (|| {
            unsafe { browser.view.ExecuteScript(&HSTRING::from(code), None) }
                .map_err(|_| error())?;
            let value = wait(rx, &job.reply, state, 25)?;
            ensure!(
                !value["error"].as_bool().unwrap_or(false),
                crate::music_session::Failure::Unconfirmed
            );
            Ok(value)
        })();
        unsafe {
            let _ = browser.view.remove_WebMessageReceived(token);
        }
        outcome
    }
    fn clear(browser: &Browser, job: &Job, state: &WindowState) -> Result<()> {
        let v: ICoreWebView2_2 = browser.view.cast().map_err(|_| error())?;
        unsafe { v.CookieManager().map_err(|_| error())?.DeleteAllCookies() }
            .map_err(|_| error())?;
        let view: ICoreWebView2_13 = browser
            .view
            .cast()
            .map_err(|e| com_error("profile interface", e))?;
        let profile: ICoreWebView2Profile2 = unsafe { view.Profile() }
            .map_err(|_| error())?
            .cast()
            .map_err(|_| error())?;
        let (tx, rx) = mpsc::channel();
        let handler = ClearBrowsingDataCompletedHandler::create(Box::new(move |result| {
            let _ = tx.send(result);
            Ok(())
        }));
        unsafe { profile.ClearBrowsingDataAll(&handler) }
            .map_err(|e| com_error("clear profile", e))?;
        // Closing the sign-in window must not cancel removal of its local session.

        let end = Instant::now() + Duration::from_secs(15);
        loop {
            pump();
            if let Ok(result) = rx.try_recv() {
                result.map_err(|_| error())?;
                break;
            }
            ensure!(
                Instant::now() < end,
                "YouTube local session cleanup timed out; try Disconnect again"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        let _ = (job, state);
        Ok(())
    }
    fn run(rx: mpsc::Receiver<Job>) {
        unsafe {
            if CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_err() {
                return;
            }
        }
        let mut state = Box::new(WindowState {
            controller: RefCell::new(None),
            closed: Cell::new(false),
        });
        let instance = unsafe { GetModuleHandleW(None) }
            .map(|h| HINSTANCE(h.0))
            .unwrap_or_default();
        let class = WNDCLASSW {
            hInstance: instance,
            lpfnWndProc: Some(proc),
            lpszClassName: w!("VibefyYouTubeSession"),
            ..Default::default()
        };
        let hwnd = unsafe {
            let _ = RegisterClassW(&class);
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                class.lpszClassName,
                w!("Vibefy · YouTube sign-in"),
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                580,
                780,
                None,
                None,
                instance,
                None,
            )
        };
        let Ok(hwnd) = hwnd else {
            return;
        };
        unsafe {
            SetWindowLongPtrW(
                hwnd,
                GWLP_USERDATA,
                (&mut *state as *mut WindowState) as isize,
            );
            SetTimer(hwnd, 1, 1000, None);
        }
        while WORKER.get().is_none() {
            std::thread::yield_now();
        }
        WORKER
            .get()
            .unwrap()
            .thread
            .store(unsafe { GetCurrentThreadId() }, Ordering::Release);
        let mut browser: Option<Browser> = None;
        let mut last = Instant::now();
        loop {
            while let Ok(job) = rx.try_recv() {
                if job.reply.is_closed() {
                    continue;
                }
                state.closed.set(false);
                if browser
                    .as_ref()
                    .is_some_and(|b| b.profile.id != job.profile.id || b.args != job.args)
                {
                    state.controller.borrow_mut().take();
                    browser = None;
                }
                let result = (|| -> Result<Value> {
                    if browser.is_none() {
                        browser = Some(open(hwnd, &state, &job)?);
                    }
                    let b = browser.as_ref().unwrap();
                    match &job.op {
                        Operation::Connect => {
                            unsafe {
                                let _ = ShowWindow(hwnd, SW_SHOW);
                                let _ = SetForegroundWindow(hwnd);
                                b.view
                                    .Navigate(&HSTRING::from(SIGN_IN_URL))
                                    .map_err(|_| error())?;
                            }
                            let (_, context) = ready(b, &job, &state, true)?;
                            Ok(context)
                        }
                        Operation::Post {
                            endpoint,
                            account,
                            body,
                            continuation,
                        } => {
                            if !music_origin(&source(b)) {
                                unsafe {
                                    b.view
                                        .Navigate(&HSTRING::from(
                                            "https://music.youtube.com/library/playlists",
                                        ))
                                        .map_err(|_| error())?;
                                }
                            }
                            post(
                                b,
                                &job,
                                &state,
                                endpoint,
                                account,
                                body,
                                continuation.as_deref(),
                            )
                        }
                        Operation::Forget => {
                            clear(b, &job, &state)?;
                            Ok(Value::Null)
                        }
                    }
                })();
                unsafe {
                    let _ = ShowWindow(hwnd, SW_HIDE);
                }
                let failed_login = matches!(job.op, Operation::Connect)
                    && (result.is_err() || job.reply.is_closed());
                if failed_login {
                    if let Some(b) = &browser {
                        let _ = clear(b, &job, &state);
                    }
                }
                if failed_login || matches!(job.op, Operation::Forget) {
                    state.controller.borrow_mut().take();
                    browser = None;
                }
                let _ = job.reply.send(result.map_err(|e| e.to_string()));
                last = Instant::now();
            }
            if last.elapsed() > Duration::from_secs(45) {
                state.controller.borrow_mut().take();
                browser = None;
            }
            unsafe {
                let mut msg = MSG::default();
                if GetMessageW(&mut msg, None, 0, 0).0 <= 0 {
                    break;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        state.controller.borrow_mut().take();
        drop(browser);
        unsafe {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            let _ = DestroyWindow(hwnd);
            CoUninitialize();
        }
    }
}
