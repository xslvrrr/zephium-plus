//! rice fork: the page's own view takes every pointer move over it, so the
//! chrome beneath (or raised with input limited to small regions) never sees
//! the pointer reach the notch or the window controls. Reading the moves
//! app-wide tells the chrome when the pointer enters or leaves each of its
//! named zones, wherever in the window it is.

use std::cell::RefCell;
use std::ptr::NonNull;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::{NSEvent, NSEventMask, NSWindow};
use objc2_foundation::MainThreadMarker;
use tauri::{Manager, WebviewWindow};

use crate::reach_zones::{Tracker, Zone};

struct Watched {
    window: WebviewWindow,
    number: isize,
    tracker: Tracker,
}

thread_local! {
    static WATCHED: RefCell<Vec<Watched>> = const { RefCell::new(Vec::new()) };
    static MONITOR: RefCell<Option<Retained<AnyObject>>> = const { RefCell::new(None) };
}

type Changes = Vec<(WebviewWindow, Vec<(String, bool)>)>;

/// Where the event's pointer is in its window's content view, in top-left
/// logical points, or `None` when it is outside the content.
fn location(event: &NSEvent, window: &NSWindow) -> Option<(f64, f64)> {
    let size = window.contentView()?.frame().size;
    let point = event.locationInWindow();
    let (x, y) = (point.x, size.height - point.y);
    (x >= 0.0 && y >= 0.0 && x < size.width && y < size.height).then_some((x, y))
}

fn observe(event: &NSEvent) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    // The event belongs to at most one window; every other watched window
    // has lost the pointer, since app-local monitors see no exits.
    let target = event
        .window(mtm)
        .map(|window| (window.windowNumber(), location(event, &window)));
    let changes: Option<Changes> = WATCHED.with(|watched| {
        let mut watched = watched.try_borrow_mut().ok()?;
        Some(
            watched
                .iter_mut()
                .map(|entry| {
                    let changed = match target {
                        Some((number, Some((x, y)))) if number == entry.number => {
                            entry.tracker.moved(x, y)
                        }
                        _ => entry.tracker.exited(),
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

/// Replaces `window`'s zones; no zones stops watching it. Runs on the main
/// thread.
pub fn set(window: &WebviewWindow, zones: Vec<Zone>) {
    if MainThreadMarker::new().is_none() {
        return;
    }
    let Ok(pointer) = window.ns_window() else {
        return;
    };
    // SAFETY: Tao's live NSWindow, borrowed for this main-thread call only.
    let number = unsafe { &*(pointer as *const NSWindow) }.windowNumber();
    let (left, watching) = WATCHED.with(|watched| {
        let mut watched = watched.borrow_mut();
        let index = match watched.iter().position(|entry| entry.number == number) {
            Some(index) => index,
            None => {
                watched.push(Watched {
                    window: window.clone(),
                    number,
                    tracker: Tracker::default(),
                });
                watched.len() - 1
            }
        };
        let left = watched[index].tracker.set(zones);
        if watched[index].tracker.is_empty() {
            watched.remove(index);
        }
        (left, !watched.is_empty())
    });
    MONITOR.with(|monitor| {
        let mut monitor = monitor.borrow_mut();
        match (watching, monitor.is_some()) {
            (true, false) => {
                let handler = RcBlock::new(|event: NonNull<NSEvent>| -> *mut NSEvent {
                    // SAFETY: the local monitor lends its event for this call.
                    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        observe(unsafe { event.as_ref() })
                    }));
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
    emit(window, left);
}
