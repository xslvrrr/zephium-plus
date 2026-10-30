//! rice fork: the notch toolbar cut into the page.
//!
//! In notch mode the notch is not drawn over the page: the page's own top
//! edge turns down into a hole the notch's shape, so the window's ground
//! (its material and texture) runs on into it and the notch's controls stand
//! on that ground. Whatever the page has beneath the hole is not shown; that
//! is the look. The hole is a mask on the stage's layer, so every pane in it
//! is cut alike, and it slides in and out with the chrome's notch on the
//! same curve.

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::ClassType;
use objc2_app_kit::{NSBezierPath, NSView, NSWorkspace};
use objc2_foundation::{ns_string, NSPoint, NSRect, NSSize};
use objc2_quartz_core::{
    kCAFillRuleEvenOdd, CABasicAnimation, CALayer, CAMediaTiming, CAMediaTimingFunction,
    CAShapeLayer, CATransaction,
};

/// Where the notch cuts the page, in window top-left points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PageNotch {
    /// The hole's left wall.
    pub left: f64,
    /// From wall to wall.
    pub width: f64,
    /// The hole's floor, down from the window's top edge.
    pub depth: f64,
    /// The floor's corners.
    pub radius: f64,
    /// The fillet where each wall meets the page's top edge.
    pub flare: f64,
    /// Cut now, or waiting above the page's edge.
    pub down: bool,
}

/// The chrome's notch slides on this, for this long (the interface's
/// --ease-out).
const SECONDS: f64 = 0.18;
/// A circle's quarter as one cubic.
const K: f64 = 0.552_284_75;

/// Applies `notch` to the stage's layer: a mask with the hole, moved there
/// from wherever the last one is when `animate`; none takes the mask away.
pub(crate) fn place(stage: &NSView, notch: Option<PageNotch>, animate: bool) {
    let Some(layer) = stage.layer() else {
        return;
    };
    let Some(notch) = notch.filter(|notch| {
        [
            notch.left,
            notch.width,
            notch.depth,
            notch.radius,
            notch.flare,
        ]
        .iter()
        .all(|value| value.is_finite())
    }) else {
        // SAFETY: main-thread layer mutation.
        unsafe { layer.setMask(None) };
        return;
    };
    let Some(path) = outline(stage, &layer, notch) else {
        unsafe { layer.setMask(None) };
        return;
    };
    let mask = layer
        .mask()
        .and_then(|mask| mask.downcast::<CAShapeLayer>().ok())
        .unwrap_or_else(|| {
            let mask = CAShapeLayer::layer();
            // SAFETY: a framework constant.
            mask.setFillRule(unsafe { kCAFillRuleEvenOdd });
            // SAFETY: main-thread layer mutation; the mask is the layer's own.
            unsafe { layer.setMask(Some(mask.as_super())) };
            mask
        });
    CATransaction::begin();
    CATransaction::setDisableActions(true);
    mask.setFrame(layer.bounds());
    // SAFETY: a main-thread read of the mask's in-flight rendering state.
    let shown = unsafe { mask.presentationLayer() }
        .and_then(|shown| shown.downcast::<CAShapeLayer>().ok())
        .and_then(|shown| shown.path())
        .or_else(|| mask.path());
    let cg = path.CGPath();
    mask.setPath(Some(&cg));
    mask.removeAnimationForKey(ns_string!("rice-notch"));
    if animate && !NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion() {
        if let Some(from) = shown {
            let animation = CABasicAnimation::animationWithKeyPath(Some(ns_string!("path")));
            // SAFETY: a CGPath is the value Core Animation expects for `path`.
            unsafe {
                animation.setFromValue(Some(&*(Retained::as_ptr(&from).cast::<AnyObject>())));
                animation.setToValue(Some(&*(Retained::as_ptr(&cg).cast::<AnyObject>())));
            }
            animation.setDuration(SECONDS);
            animation.setTimingFunction(Some(&CAMediaTimingFunction::functionWithControlPoints(
                0.22, 1.0, 0.36, 1.0,
            )));
            mask.addAnimation_forKey(&animation, Some(ns_string!("rice-notch")));
        }
    }
    CATransaction::commit();
}

/// The stage's whole area, far past its edges so a resize never shows its
/// border, with the hole taken out of it (even-odd). Up, the hole waits
/// wholly above the page's edge, so the shape keeps the same points and a
/// slide between the two moves it rather than reshaping it.
fn outline(stage: &NSView, layer: &CALayer, notch: PageNotch) -> Option<Retained<NSBezierPath>> {
    let window = stage.window()?;
    let content = window.contentView()?;
    let frame = stage.convertRect_toView(stage.bounds(), None);
    let window_height = content.bounds().size.height;
    let stage_left = frame.origin.x;
    let stage_top = window_height - (frame.origin.y + frame.size.height);
    let bounds = layer.bounds();
    let (w, h) = (bounds.size.width, bounds.size.height);
    let flipped = layer.contentsAreFlipped();

    let left = notch.left - stage_left;
    let right = left + notch.width;
    let floor = notch.depth - stage_top;
    let r = notch.radius.max(0.0).min(notch.width / 2.0);
    let f = notch.flare.max(0.0);
    if notch.width <= 2.0 * r || floor <= f + r {
        return None;
    }
    // Up by the notch's whole depth, as the chrome's notch slides, so the
    // two move together on every frame.
    let shift = if notch.down {
        0.0
    } else {
        -(notch.depth + 2.0)
    };
    // Top-left page points to the layer's own.
    let at = |x: f64, y: f64| {
        let y = y + shift;
        NSPoint::new(x, if flipped { y } else { h - y })
    };

    let path = NSBezierPath::bezierPath();
    path.appendBezierPathWithRect(NSRect::new(
        NSPoint::new(-w, -h),
        NSSize::new(3.0 * w, 3.0 * h),
    ));
    let curve = |to: NSPoint, c1: NSPoint, c2: NSPoint| {
        path.curveToPoint_controlPoint1_controlPoint2(to, c1, c2);
    };
    path.moveToPoint(at(left - f, 0.0));
    curve(at(left, f), at(left - f + K * f, 0.0), at(left, f - K * f));
    path.lineToPoint(at(left, floor - r));
    curve(
        at(left + r, floor),
        at(left, floor - r + K * r),
        at(left + r - K * r, floor),
    );
    path.lineToPoint(at(right - r, floor));
    curve(
        at(right, floor - r),
        at(right - r + K * r, floor),
        at(right, floor - r + K * r),
    );
    path.lineToPoint(at(right, f));
    curve(
        at(right + f, 0.0),
        at(right, f - K * f),
        at(right + f - K * f, 0.0),
    );
    path.closePath();
    Some(path)
}
