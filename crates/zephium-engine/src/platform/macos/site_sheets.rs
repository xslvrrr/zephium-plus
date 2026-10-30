//! rice fork: Zen Internet's site styles, as WebKit user style sheets.
//!
//! Each Browse page gets one author-level `_WKUserStyleSheet` per host it
//! visits, with the include pattern `*://<host>/*`, added when a main-frame
//! navigation to that host is decided or redirected, before its document
//! exists: no page script, no flash, and a site's CSP does not apply. Sheets
//! are per host, not per site, because the styles' `-domain` entries (any
//! public suffix) cannot be written as match patterns; the desktop's
//! resolver decides the match. Sheets are created without a web view:
//! WebKit drops page-scoped sheets at the next document (see
//! `docs/rice/spikes/2026-10-29-user-style-sheet.md`).
//!
//! A page whose style clears its background stops painting WebKit's
//! default one, so the window shows through; every other page paints it as
//! before. Private API, guarded by `respondsToSelector:`.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use objc2::rc::{Allocated, Retained};
use objc2::runtime::NSObjectProtocol;
use objc2::runtime::{AnyClass, AnyObject, Sel};
use objc2::{msg_send, sel, MainThreadMarker};
use objc2_app_kit::NSColor;
use objc2_foundation::{ns_string, NSArray, NSNumber, NSObjectNSKeyValueCoding, NSString};
use objc2_web_kit::{WKContentWorld, WKUserContentController, WKWebView};

/// `_WKUserStyleLevel`: author, so `!important` rules beat the page's own.
const AUTHOR: isize = 1;

struct Sheet {
    host: String,
    css: String,
    object: Retained<AnyObject>,
}

/// One Browse page's sheets. Created before the view exists, so the
/// navigation hooks can hold it; bound once the view is configured.
#[derive(Default)]
pub(crate) struct PageSheets {
    view: RefCell<Option<objc2::rc::Weak<WKWebView>>>,
    sheets: RefCell<Vec<Sheet>>,
    /// The host of the committed document.
    committed: RefCell<Option<String>>,
    /// The host a main-frame navigation is heading to.
    target: RefCell<Option<String>>,
    transparent: Cell<bool>,
}

thread_local! {
    static PAGES: RefCell<Vec<Weak<PageSheets>>> = const { RefCell::new(Vec::new()) };
}

fn unavailable() {
    thread_local!(static SAID: Cell<bool> = const { Cell::new(false) });
    if !SAID.replace(true) {
        eprintln!("engine: _WKUserStyleSheet unavailable");
    }
}

fn host_of(url: &str) -> Option<String> {
    let url = url::Url::parse(url).ok()?;
    matches!(url.scheme(), "http" | "https")
        .then(|| url.host_str().map(str::to_ascii_lowercase))
        .flatten()
}

fn controller(view: &WKWebView) -> Option<Retained<WKUserContentController>> {
    // SAFETY: public WebKit reads on the main thread.
    let controller = unsafe { view.configuration().userContentController() };
    let selectors = [sel!(_addUserStyleSheet:), sel!(_removeUserStyleSheet:)];
    if selectors
        .iter()
        .all(|selector| controller.respondsToSelector(*selector))
    {
        Some(controller)
    } else {
        unavailable();
        None
    }
}

fn create(host: &str, css: &str) -> Option<Retained<AnyObject>> {
    let mtm = MainThreadMarker::new()?;
    let Some(class) = AnyClass::get(c"_WKUserStyleSheet") else {
        unavailable();
        return None;
    };
    let init: Sel = sel!(initWithSource:forWKWebView:forMainFrameOnly:includeMatchPatternStrings:excludeMatchPatternStrings:baseURL:level:contentWorld:);
    if class.instance_method(init).is_none() {
        unavailable();
        return None;
    }
    let source = NSString::from_str(css);
    let include = NSArray::from_retained_slice(&[NSString::from_str(&format!("*://{host}/*"))]);
    // SAFETY: WebKit's world for page content, read on the main thread.
    let world = unsafe { WKContentWorld::pageWorld(mtm) };
    // SAFETY: the selector was checked above; every argument has the type
    // WebKit's initialiser declares, and nil is allowed for the web view,
    // the exclude patterns and the base URL.
    unsafe {
        let allocated: Allocated<AnyObject> = msg_send![class, alloc];
        msg_send![
            allocated,
            initWithSource: &*source,
            forWKWebView: std::ptr::null::<WKWebView>(),
            forMainFrameOnly: true,
            includeMatchPatternStrings: &*include,
            excludeMatchPatternStrings: std::ptr::null::<NSArray<NSString>>(),
            baseURL: std::ptr::null::<AnyObject>(),
            level: AUTHOR,
            contentWorld: &*world
        ]
    }
}

impl PageSheets {
    pub(crate) fn new() -> Rc<Self> {
        let page = Rc::new(Self::default());
        PAGES.with(|pages| {
            let mut pages = pages.borrow_mut();
            pages.retain(|page| page.strong_count() > 0);
            pages.push(Rc::downgrade(&page));
        });
        page
    }

    fn view(&self) -> Option<Retained<WKWebView>> {
        self.view.borrow().as_ref()?.load()
    }

    pub(crate) fn bind(&self, view: &WKWebView) {
        self.view.replace(Some(objc2::rc::Weak::from(view)));
        self.apply();
    }

    /// A main-frame navigation was decided or redirected to `url`.
    pub(crate) fn navigating(&self, url: &str) {
        self.target.replace(host_of(url));
        self.apply();
    }

    /// The main frame committed a document from `url`. Sheets for hosts the
    /// page has left go, and the background follows the new document.
    pub(crate) fn committed(&self, url: &str) {
        let host = host_of(url);
        self.target.replace(host.clone());
        self.committed.replace(host);
        self.apply();
    }

    /// Recomputes every sheet, for changed settings, overrides or styles.
    fn refresh(&self) {
        if let Some(view) = self.view() {
            if let Some(controller) = controller(&view) {
                for sheet in self.sheets.take() {
                    // SAFETY: checked by `controller`.
                    let _: () =
                        unsafe { msg_send![&controller, _removeUserStyleSheet: &*sheet.object] };
                }
            }
        }
        self.apply();
    }

    /// Keeps a sheet for the committed host and the target host, each with
    /// its current style, and drops the rest.
    fn apply(&self) {
        let Some(view) = self.view() else {
            return;
        };
        let Some(controller) = controller(&view) else {
            return;
        };
        let wanted: Vec<(String, Option<(String, bool)>)> = [
            self.committed.borrow().clone(),
            self.target.borrow().clone(),
        ]
        .into_iter()
        .flatten()
        .fold(Vec::new(), |mut hosts, host| {
            if !hosts.iter().any(|(known, _)| *known == host) {
                let style = crate::site_style_for(&host);
                hosts.push((host, style));
            }
            hosts
        });
        let mut sheets = self.sheets.borrow_mut();
        sheets.retain(|sheet| {
            let keep = wanted.iter().any(|(host, style)| {
                *host == sheet.host && style.as_ref().is_some_and(|(css, _)| *css == sheet.css)
            });
            if !keep {
                // SAFETY: checked by `controller`.
                let _: () =
                    unsafe { msg_send![&controller, _removeUserStyleSheet: &*sheet.object] };
            }
            keep
        });
        for (host, style) in &wanted {
            let Some((css, _)) = style else {
                continue;
            };
            if sheets.iter().any(|sheet| sheet.host == *host) {
                continue;
            }
            if let Some(object) = create(host, css) {
                // SAFETY: checked by `controller`.
                let _: () = unsafe { msg_send![&controller, _addUserStyleSheet: &*object] };
                sheets.push(Sheet {
                    host: host.clone(),
                    css: css.clone(),
                    object,
                });
            }
        }
        drop(sheets);
        let committed = self.committed.borrow().clone();
        let transparent = wanted
            .iter()
            .find(|(host, _)| Some(host) == committed.as_ref())
            .and_then(|(_, style)| style.as_ref())
            .is_some_and(|(_, transparent)| *transparent);
        if transparent != self.transparent.replace(transparent) {
            set_transparent(&view, transparent);
        }
    }
}

/// WebKit's default page background, off where a style clears it.
fn set_transparent(view: &WKWebView, transparent: bool) {
    let draws = NSNumber::new_bool(!transparent);
    // SAFETY: `drawsBackground` is a KVC key of WKWebView (wry uses it too);
    // the under-page colour is public API. Both on the main thread.
    unsafe {
        view.setValue_forKey(Some(&draws), ns_string!("drawsBackground"));
        if transparent {
            view.setUnderPageBackgroundColor(Some(&NSColor::clearColor()));
        } else {
            view.setUnderPageBackgroundColor(None);
        }
    }
}

/// Every live page recomputes its sheets and background.
pub(crate) fn refresh_all() {
    let pages: Vec<Rc<PageSheets>> = PAGES.with(|pages| {
        let mut pages = pages.borrow_mut();
        pages.retain(|page| page.strong_count() > 0);
        pages.iter().filter_map(Weak::upgrade).collect()
    });
    for page in pages {
        page.refresh();
    }
}
