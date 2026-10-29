//! rice fork: the page's own view takes every pointer move over it, so the
//! chrome beneath (or raised with input limited to small regions) never sees
//! the pointer reach the notch or the window controls. Reading the moves
//! app-wide tells the chrome when the pointer enters or leaves each of its
//! named zones, wherever in the window it is.
//!
//! Moves only say where the pointer is while it is over this app. The
//! notch's usual way out is up past the window's top edge, onto the menu bar
//! or another app, where no move arrives; a tracking area over each watched
//! window's content view reports that exit.

use std::cell::RefCell;
use std::collections::HashSet;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr::NonNull;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{define_class, msg_send, AllocAnyThread, DefinedClass, MainThreadOnly};
use objc2_app_kit::{
    NSEvent, NSEventMask, NSTrackingArea, NSTrackingAreaOptions, NSView, NSWindow,
};
use objc2_foundation::{MainThreadMarker, NSObject, NSObjectProtocol, NSRect};
use tauri::{Manager, WebviewWindow};

use crate::reach_zones::{Tracker, Zone};

struct Watched {
    window: WebviewWindow,
    number: isize,
    tracker: Tracker,
    view: Retained<NSView>,
    area: Retained<NSTrackingArea>,
    /// A tracking area does not retain its owner.
    _exit: Retained<ExitWatcher>,
}

impl Drop for Watched {
    fn drop(&mut self) {
        self.view.removeTrackingArea(&self.area);
    }
}

thread_local! {
    static WATCHED: RefCell<Vec<Watched>> = const { RefCell::new(Vec::new()) };
    static MONITOR: RefCell<Option<Retained<AnyObject>>> = const { RefCell::new(None) };
    /// The window numbers whose `Destroyed` already forgets them.
    static HOOKED: RefCell<HashSet<isize>> = RefCell::new(HashSet::new());
}

type Changes = Vec<(WebviewWindow, Vec<(String, bool)>)>;

struct ExitIvars {
    number: isize,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "ZephiumReachZoneExit"]
    #[ivars = ExitIvars]
    struct ExitWatcher;

    unsafe impl NSObjectProtocol for ExitWatcher {}

    impl ExitWatcher {
        /// Entering is read from the moves, which say where.
        #[unsafe(method(mouseEntered:))]
        fn mouse_entered(&self, _event: &NSEvent) {}

        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, _event: &NSEvent) {
            let number = self.ivars().number;
            let _ = catch_unwind(AssertUnwindSafe(|| left(number)));
        }
    }
);

impl ExitWatcher {
    fn new(mtm: MainThreadMarker, number: isize) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(ExitIvars { number });
        // SAFETY: NSObject's designated initializer.
        unsafe { msg_send![super(this), init] }
    }
}

/// Where the pointer is in the watched window's content view, in top-left
/// logical points, or `None` when it is outside the content.
fn location(entry: &Watched) -> Option<(f64, f64)> {
    let window = entry.view.window()?;
    let size = entry.view.frame().size;
    let point = window.mouseLocationOutsideOfEventStream();
    let (x, y) = (point.x, size.height - point.y);
    (x >= 0.0 && y >= 0.0 && x < size.width && y < size.height).then_some((x, y))
}

fn observe() {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    // Moves can be tagged with the key window while the pointer is over
    // another; the window under the pointer is the one it is in, and every
    // other watched window has lost it.
    let under =
        NSWindow::windowNumberAtPoint_belowWindowWithWindowNumber(NSEvent::mouseLocation(), 0, mtm);
    let changes: Option<Changes> = WATCHED.with(|watched| {
        let mut watched = watched.try_borrow_mut().ok()?;
        Some(
            watched
                .iter_mut()
                .map(|entry| {
                    let at = (entry.number == under).then(|| location(entry)).flatten();
                    let changed = match at {
                        Some((x, y)) => entry.tracker.moved(x, y),
                        None => entry.tracker.exited(),
                    };
                    (entry.window.clone(), changed)
                })
                .filter(|(_, changed)| !changed.is_empty())
                .collect(),
        )
    });
    for (window, changed) in changes.unwrap_or_default() {
        emit(&window, changed);
    }
}

/// The pointer left window `number`'s content view.
fn left(number: isize) {
    let changed = WATCHED.with(|watched| {
        let mut watched = watched.try_borrow_mut().ok()?;
        let entry = watched.iter_mut().find(|entry| entry.number == number)?;
        Some((entry.window.clone(), entry.tracker.exited()))
    });
    if let Some((window, changed)) = changed {
        emit(&window, changed);
    }
}

fn emit(window: &WebviewWindow, changed: Vec<(String, bool)>) {
    for (name, inside) in changed {
        crate::emit_to_privileged(
            window.app_handle(),
            window.label(),
            "zephium:reach-zone",
            &serde_json::json!({ "name": name, "inside": inside }),
        );
    }
}

/// A closed window stops being watched. Its handle is gone, so it is found
/// by number alone. Runs on the main thread.
fn forget(number: isize) {
    HOOKED.with(|hooked| hooked.borrow_mut().remove(&number));
    let watching = WATCHED.with(|watched| {
        let mut watched = watched.borrow_mut();
        watched.retain(|entry| entry.number != number);
        !watched.is_empty()
    });
    monitor(watching);
}

/// Installs the move monitor while any window is watched, and removes it
/// when none is.
fn monitor(watching: bool) {
    MONITOR.with(|monitor| {
        let mut monitor = monitor.borrow_mut();
        match (watching, monitor.is_some()) {
            (true, false) => {
                let handler = RcBlock::new(|event: NonNull<NSEvent>| -> *mut NSEvent {
                    let _ = catch_unwind(AssertUnwindSafe(observe));
                    event.as_ptr()
                });
                // SAFETY: an app-local main-thread monitor that hands every
                // event back untouched.
                *monitor = unsafe {
                    NSEvent::addLocalMonitorForEventsMatchingMask_handler(
                        NSEventMask::MouseMoved | NSEventMask::LeftMouseDragged,
                        &handler,
                    )
                };
            }
            (false, true) => {
                if let Some(old) = monitor.take() {
                    // SAFETY: this exact registration is removed once.
                    unsafe { NSEvent::removeMonitor(&old) };
                }
            }
            _ => {}
        }
    });
}

fn watch(mtm: MainThreadMarker, window: &WebviewWindow, ns_window: &NSWindow) -> Option<Watched> {
    let number = ns_window.windowNumber();
    let view = ns_window.contentView()?;
    let exit = ExitWatcher::new(mtm, number);
    // SAFETY: the owner is kept alive beside the area, and the area is
    // removed from the view before either is released; no user info.
    let area = unsafe {
        NSTrackingArea::initWithRect_options_owner_userInfo(
            NSTrackingArea::alloc(),
            NSRect::ZERO,
            NSTrackingAreaOptions::MouseEnteredAndExited
                | NSTrackingAreaOptions::ActiveAlways
                | NSTrackingAreaOptions::InVisibleRect,
            Some(exit.as_ref()),
            None,
        )
    };
    view.addTrackingArea(&area);
    Some(Watched {
        window: window.clone(),
        number,
        tracker: Tracker::default(),
        view,
        area,
        _exit: exit,
    })
}

/// Replaces `window`'s zones; no zones stops watching it. Runs on the main
/// thread.
pub fn set(window: &WebviewWindow, zones: Vec<Zone>) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let Ok(pointer) = window.ns_window() else {
        return;
    };
    // SAFETY: Tao's live NSWindow, borrowed for this main-thread call only.
    let ns_window = unsafe { &*(pointer as *const NSWindow) };
    let number = ns_window.windowNumber();
    let (changed, watching) = WATCHED.with(|watched| {
        let mut watched = watched.borrow_mut();
        let index = match watched.iter().position(|entry| entry.number == number) {
            Some(index) => {
                // A window number can outlive its window; the handle follows
                // the window that holds it now.
                watched[index].window = window.clone();
                Some(index)
            }
            None if zones.is_empty() => None,
            None => watch(mtm, window, ns_window).map(|entry| {
                watched.push(entry);
                watched.len() - 1
            }),
        };
        let changed = index.map_or_else(Vec::new, |index| {
            let changed = watched[index].tracker.set(zones);
            if watched[index].tracker.is_empty() {
                watched.remove(index);
            }
            changed
        });
        (changed, !watched.is_empty())
    });
    let unhooked = watching && HOOKED.with(|hooked| hooked.borrow_mut().insert(number));
    if unhooked {
        let app = window.app_handle().clone();
        window.on_window_event(move |event| {
            if matches!(event, tauri::WindowEvent::Destroyed) {
                if MainThreadMarker::new().is_some() {
                    forget(number);
                } else {
                    let _ = app.run_on_main_thread(move || forget(number));
                }
            }
        });
    }
    monitor(watching);
    emit(window, changed);
}
