//! rice fork: the rice texture, drawn once per window beneath the chrome and
//! the pages. The chrome is raised over pages to show the notch; a texture
//! inside the chrome would then cover the page or vanish from behind it, so
//! it lives here instead, where nothing moves it.
//!
//! The view sits directly above the window's material and below every
//! webview. The material is always inserted at the very bottom of the content
//! view (`window_vibrancy` adds it below everything, and reinstalls it the
//! same way), while the chrome and the engine's stages are added on top, and
//! raising the chrome only swaps it with a stage. So once in place, the
//! backdrop stays between them however the views around it change.
//!
//! The texture shows only over a native material: with the Solid material
//! (`ui.material=none`) or Reduce Transparency the chrome paints its opaque
//! ground over the whole window, so the backdrop is cleared and never read.

use std::cell::{Cell, RefCell};
use std::ptr::{null_mut, NonNull};
use std::sync::Mutex;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject, ProtocolObject};
use objc2::{define_class, msg_send, AllocAnyThread, DefinedClass, MainThreadOnly, Message};
use objc2_app_kit::{
    NSAutoresizingMaskOptions, NSImage, NSResponder, NSView, NSViewLayerContentsPlacement,
    NSViewLayerContentsRedrawPolicy, NSWindowOrderingMode, NSWorkspace,
    NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification,
};
use objc2_foundation::{
    MainThreadMarker, NSData, NSNotification, NSObjectProtocol, NSPoint, NSRect,
};
use objc2_quartz_core::kCAGravityResizeAspectFill;
use tauri::{AppHandle, Manager, WebviewWindow};

/// The tags `window_vibrancy` gives the vibrancy and liquid glass views.
const MATERIAL_TAGS: [isize; 2] = [91_376_254, 96_945_937];
/// The frame's default for `ui.texture-opacity`.
const DEFAULT_OPACITY: u8 = 15;
/// The corner `material.rs` gives the window's material (vibrancy, glass and
/// the tint wash), so the texture never paints into the window's corners.
const CORNER_RADIUS: f64 = 12.0;

/// The texture and its strength as last projected. A change arrives as a
/// projection, possibly ahead of the store's write, so each shadows the
/// stored value, as `material::choose` does for the material.
static PROJECTED: Mutex<Projected> = Mutex::new(Projected {
    texture: None,
    opacity: None,
});

struct Projected {
    texture: Option<String>,
    opacity: Option<String>,
}

pub struct Ivars {
    /// The digest whose picture `image` holds, or empty.
    shown: RefCell<String>,
    /// The digest last asked for. A read that finishes for any other is
    /// dropped, so only the newest choice lands.
    wanted: RefCell<String>,
    image: RefCell<Option<Retained<NSImage>>>,
    opacity: Cell<f32>,
}

define_class!(
    #[unsafe(super(NSView, NSResponder, NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "ZephiumRiceBackdrop"]
    #[ivars = Ivars]
    pub struct RiceBackdrop;

    impl RiceBackdrop {
        // The backdrop is never a target: clicks go to the views above it.
        #[unsafe(method(hitTest:))]
        fn hit_test(&self, _point: NSPoint) -> *mut NSView {
            null_mut()
        }

        // The layer is drawn from the ivars, never through `drawRect:`, so
        // AppKit keeps the layer's geometry and this keeps its contents.
        #[unsafe(method(wantsUpdateLayer))]
        fn wants_update_layer(&self) -> bool {
            true
        }

        #[unsafe(method(updateLayer))]
        fn update_layer(&self) {
            let Some(layer) = self.layer() else {
                return;
            };
            let image = self.ivars().image.borrow();
            let contents: Option<&AnyObject> = image.as_deref().map(|image| image.as_ref());
            // SAFETY: an NSImage is a valid layer contents on macOS, and the
            // gravity is one of Core Animation's own constants.
            unsafe {
                layer.setContents(contents);
                layer.setContentsGravity(kCAGravityResizeAspectFill);
            }
            layer.setCornerRadius(CORNER_RADIUS);
            layer.setMasksToBounds(true);
            layer.setOpacity(self.ivars().opacity.get());
        }
    }
);

impl RiceBackdrop {
    fn new(mtm: MainThreadMarker, frame: NSRect) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(Ivars {
            shown: RefCell::new(String::new()),
            wanted: RefCell::new(String::new()),
            image: RefCell::new(None),
            opacity: Cell::new(0.0),
        });
        // SAFETY: the designated initializer of NSView.
        let this: Retained<Self> = unsafe { msg_send![super(this), initWithFrame: frame] };
        this.setWantsLayer(true);
        // Resizing only rescales the picture already on the layer.
        this.setLayerContentsRedrawPolicy(NSViewLayerContentsRedrawPolicy::OnSetNeedsDisplay);
        this.setLayerContentsPlacement(NSViewLayerContentsPlacement::ScaleProportionallyToFill);
        this.setAutoresizingMask(
            NSAutoresizingMaskOptions::ViewWidthSizable
                | NSAutoresizingMaskOptions::ViewHeightSizable,
        );
        this.setHidden(true);
        this
    }

    fn clear(&self) {
        let ivars = self.ivars();
        ivars.image.replace(None);
        ivars.shown.replace(String::new());
        ivars.wanted.replace(String::new());
        // A hidden view is not displayed, so `updateLayer` would never let
        // go of the picture; release it here.
        if let Some(layer) = self.layer() {
            // SAFETY: nil is a valid layer contents.
            unsafe { layer.setContents(None) };
        }
        self.setHidden(true);
    }
}

/// The window's content view, which holds the material, the chrome and the
/// stages side by side.
fn content_view(window: &WebviewWindow) -> Option<Retained<NSView>> {
    let pointer = window.ns_view().ok()?;
    // SAFETY: Tao's live content view, retained on the main thread.
    unsafe { Retained::retain(pointer.cast::<NSView>()) }
}

fn find(content: &NSView) -> Option<Retained<RiceBackdrop>> {
    content
        .subviews()
        .iter()
        .find_map(|view| view.downcast_ref::<RiceBackdrop>().map(|b| b.retain()))
}

/// Installs the backdrop on a browser window. Idempotent, and a no-op off the
/// main thread.
pub fn install(window: &WebviewWindow) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let Some(content) = content_view(window) else {
        return;
    };
    if find(&content).is_some() {
        return;
    }
    let backdrop = RiceBackdrop::new(mtm, content.bounds());
    // Only a direct subview is a valid anchor; `viewWithTag:` would also
    // search inside the webviews.
    let material = content
        .subviews()
        .iter()
        .find(|view| MATERIAL_TAGS.contains(&view.tag()));
    match material {
        Some(material) => content.addSubview_positioned_relativeTo(
            &backdrop,
            NSWindowOrderingMode::Above,
            Some(&material),
        ),
        // No material yet, or none at all: the bottom. One installed later
        // goes beneath it.
        None => {
            content.addSubview_positioned_relativeTo(&backdrop, NSWindowOrderingMode::Below, None)
        }
    }
    observe_contrast(window.app_handle());
    refresh(window);
}

/// Takes a projected `ui.texture` or `ui.texture-opacity`.
pub fn follow(key: &str, value: &str) {
    let mut projected = PROJECTED.lock().unwrap_or_else(|e| e.into_inner());
    match key {
        "ui.texture" => projected.texture = Some(value.to_owned()),
        "ui.texture-opacity" => projected.opacity = Some(value.to_owned()),
        _ => {}
    }
}

/// The digest to show and its opacity in percent, or nothing to show.
fn wanted() -> Option<(String, u8)> {
    // Increase Contrast is macOS's forced colours: no picture behind text.
    if NSWorkspace::sharedWorkspace().accessibilityDisplayShouldIncreaseContrast() {
        return None;
    }
    let (texture, opacity) = {
        let projected = PROJECTED.lock().unwrap_or_else(|e| e.into_inner());
        (projected.texture.clone(), projected.opacity.clone())
    };
    let digest = texture
        .or_else(|| crate::app_setting("ui.texture"))
        .filter(|digest| {
            !digest.is_empty() && zephium_core::preferences::value_allowed("ui.texture", digest)
        })?;
    let opacity = opacity
        .or_else(|| crate::app_setting("ui.texture-opacity"))
        .filter(|value| zephium_core::preferences::value_allowed("ui.texture-opacity", value))
        .and_then(|value| value.parse::<u8>().ok())
        .unwrap_or(DEFAULT_OPACITY);
    (opacity > 0).then_some((digest, opacity))
}

/// Whether the window is in the Solid tier, decided as `material::install`
/// decides it: the person chose no material, Reduce Transparency is on, or
/// no material could be installed.
fn solid(window: &WebviewWindow) -> bool {
    crate::material::preference() == crate::material::Preference::None
        || NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceTransparency()
        || crate::material::current(window.label()) == crate::material::Material::None
}

/// Shows the current texture at the current strength, or nothing. A new
/// digest is read off the main thread; the same digest only changes opacity.
pub fn refresh(window: &WebviewWindow) {
    if MainThreadMarker::new().is_none() {
        return;
    }
    let Some(backdrop) = content_view(window).and_then(|content| find(&content)) else {
        return;
    };
    let Some((digest, opacity)) = (!solid(window)).then(wanted).flatten() else {
        backdrop.clear();
        return;
    };
    let ivars = backdrop.ivars();
    ivars.opacity.set(f32::from(opacity) / 100.0);
    backdrop.setNeedsDisplay(true);
    if *ivars.shown.borrow() == digest {
        ivars.wanted.replace(digest);
        backdrop.setHidden(false);
        return;
    }
    if *ivars.wanted.borrow() == digest {
        // Already being read; the new opacity applies when it lands.
        return;
    }
    ivars.wanted.replace(digest.clone());
    if !load(window, digest) {
        ivars.wanted.replace(String::new());
    }
}

/// Every browser window, after a projected change.
pub fn refresh_all(app: &AppHandle) {
    for (label, window) in app.webview_windows() {
        if label == crate::MAIN_LABEL {
            refresh(&window);
        }
    }
}

/// Reads the texture's bytes on a worker and lands them on the main thread.
fn load(window: &WebviewWindow, digest: String) -> bool {
    let app = window.app_handle().clone();
    let label = window.label().to_owned();
    std::thread::Builder::new()
        .name("rice-backdrop".into())
        .spawn(move || {
            let bytes = read(&app, &digest);
            let main = app.clone();
            let _ = app.run_on_main_thread(move || {
                if let Some(window) = main.get_webview_window(&label) {
                    land(&window, &digest, bytes);
                }
            });
        })
        .is_ok()
}

/// The texture's bytes from the current profile's media store, read as the
/// `zephium-media` handler reads them, at full size. At launch the shell is
/// published after the window is set up, so the first read waits for it.
fn read(app: &AppHandle, digest: &str) -> Option<Vec<u8>> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let shell = loop {
        if let Some(shell) = app.try_state::<zephium_app::Handle>() {
            break shell.inner().clone();
        }
        if std::time::Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    };
    let blobs = app.try_state::<crate::media::MediaBlobs>()?.0.clone();
    let profile = crate::rice_theme::current_profile(&shell)?;
    blobs.read(
        profile,
        digest,
        crate::rice_theme::MAX_TEXTURE_BYTES as usize,
    )
}

fn land(window: &WebviewWindow, digest: &str, bytes: Option<Vec<u8>>) {
    let Some(backdrop) = content_view(window).and_then(|content| find(&content)) else {
        return;
    };
    let ivars = backdrop.ivars();
    if *ivars.wanted.borrow() != digest {
        return;
    }
    let image =
        bytes.and_then(|bytes| NSImage::initWithData(NSImage::alloc(), &NSData::from_vec(bytes)));
    let Some(image) = image else {
        crate::write_diagnostic(format_args!("rice-backdrop: texture unreadable, cleared"));
        backdrop.clear();
        return;
    };
    ivars.image.replace(Some(image));
    ivars.shown.replace(digest.to_owned());
    backdrop.setHidden(false);
    backdrop.setNeedsDisplay(true);
}

thread_local! {
    static CONTRAST: RefCell<Option<Retained<ProtocolObject<dyn NSObjectProtocol>>>> =
        const { RefCell::new(None) };
}

/// Increase Contrast and Reduce Transparency can change at any time; follow
/// them. One observer for the app, kept for its life.
fn observe_contrast(app: &AppHandle) {
    CONTRAST.with(|slot| {
        if slot.borrow().is_some() {
            return;
        }
        let app = app.clone();
        let block = RcBlock::new(move |_: NonNull<NSNotification>| {
            let main = app.clone();
            // Two hops: the material's own observer reinstalls it from a
            // main-thread task queued by this same notification, and the
            // installed tier must be read after that task has run.
            let _ = app.run_on_main_thread(move || {
                let last = main.clone();
                let _ = main.run_on_main_thread(move || refresh_all(&last));
            });
        });
        let center = NSWorkspace::sharedWorkspace().notificationCenter();
        // SAFETY: the block captures only a Send handle and dispatches its
        // work to the main thread; the token is kept on the main thread.
        let token = unsafe {
            center.addObserverForName_object_queue_usingBlock(
                Some(NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification),
                None,
                None,
                &block,
            )
        };
        slot.replace(Some(token));
    });
}
