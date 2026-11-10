//! rice fork: the notch toolbar cut into the page.
//!
//! In notch mode the notch is not drawn over the page: the page's own top
//! edge turns down into a hole the notch's shape, so the window's ground
//! (its material and texture) runs on into it and the notch's controls stand
//! on that ground. Whatever the page has beneath the hole is not shown; that
//! is the look. The hole is a mask on the stage's layer, so every pane in it
//! is cut alike, and it slides in and out with the chrome's notch on the
//! same curve. The cut carries the page's own hairline, so a page whose edge
//! shows has it round the notch too. Up and settled, there is no mask.

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::ClassType;
use objc2_app_kit::{NSBezierPath, NSColor, NSView, NSWorkspace};
use objc2_core_graphics::CGPath;
use objc2_foundation::{ns_string, NSCopying, NSPoint, NSRect, NSSize, NSString};
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
///
/// A mask renders everything under it offscreen on every frame WindowServer
/// composes the page, so the mask stands only while the notch is down or
/// sliding: up and settled, the page has none.
pub(crate) fn place(stage: &NSView, notch: Option<PageNotch>, animate: bool) {
    let Some(layer) = stage.layer() else {
        return;
    };
    let notch = notch.filter(|notch| {
        [
            notch.left,
            notch.width,
            notch.depth,
            notch.radius,
            notch.flare,
        ]
        .iter()
        .all(|value| value.is_finite())
    });
    let current = layer
        .mask()
        .and_then(|mask| mask.downcast::<CAShapeLayer>().ok());
    let Some((notch, cut)) = notch.and_then(|notch| Some((notch, outline(stage, &layer, notch)?)))
    else {
        unmask(&layer);
        return;
    };
    let moving =
        animate && !NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion();
    if !notch.down && !(moving && current.is_some()) {
        unmask(&layer);
        return;
    }
    let (mask, edge) = match (current, edge_of(&layer)) {
        (Some(mask), Some(edge)) => (mask, edge),
        _ => {
            unmask(&layer);
            let mask = CAShapeLayer::layer();
            // SAFETY: a framework constant.
            mask.setFillRule(unsafe { kCAFillRuleEvenOdd });
            // SAFETY: main-thread layer mutation; the mask is the layer's own.
            unsafe { layer.setMask(Some(mask.as_super())) };
            let edge = CAShapeLayer::layer();
            edge.setName(Some(edge_name()));
            edge.setFillColor(None);
            // The page's own hairline (`native::round_page`): a 1 pt line
            // inside the page; the mask takes the stroke's outer half.
            edge.setStrokeColor(Some(&NSColor::colorWithWhite_alpha(1.0, 0.09).CGColor()));
            edge.setLineWidth(2.0);
            edge.setZPosition(1_000.0);
            layer.addSublayer(&edge);
            (mask, edge)
        }
    };
    CATransaction::begin();
    CATransaction::setDisableActions(true);
    mask.setFrame(layer.bounds());
    // The page draws its own hairline along its top edge; where the cut's
    // flares run into that edge the two would lie on each other and read
    // twice as bright. The cut's line stops short of the top 1 pt band and
    // leaves it to the page's. A rectangular clip, so no offscreen pass.
    let bounds = layer.bounds();
    let band = 1.0_f64.min(bounds.size.height);
    let clip = NSRect::new(
        NSPoint::new(
            bounds.origin.x,
            bounds.origin.y
                + if layer.contentsAreFlipped() {
                    band
                } else {
                    0.0
                },
        ),
        NSSize::new(bounds.size.width, bounds.size.height - band),
    );
    edge.setFrame(clip);
    edge.setBounds(clip);
    edge.setMasksToBounds(true);
    // SAFETY: a main-thread read of the mask's in-flight rendering state.
    let shown = unsafe { mask.presentationLayer() }
        .and_then(|shown| shown.downcast::<CAShapeLayer>().ok())
        .and_then(|shown| shown.path())
        .or_else(|| mask.path())
        .or_else(|| {
            // Newly made for a notch coming down: it slides from up.
            let up = PageNotch {
                down: false,
                ..notch
            };
            outline(stage, &layer, up).map(|(hole, _)| hole.CGPath())
        });
    let (hole, line) = cut;
    let hole = hole.CGPath();
    let line = line.CGPath();
    // SAFETY: a main-thread read of the hairline's in-flight rendering state.
    let from_line = unsafe { edge.presentationLayer() }
        .and_then(|shown| shown.downcast::<CAShapeLayer>().ok())
        .and_then(|shown| shown.path())
        .or_else(|| edge.path());
    mask.setPath(Some(&hole));
    edge.setPath(Some(&line));
    mask.removeAnimationForKey(ns_string!("rice-notch"));
    edge.removeAnimationForKey(ns_string!("rice-notch"));
    if !notch.down {
        // Once up, the mask goes, unless the notch has since moved again.
        let target = layer.clone();
        let placed = hole.clone();
        let done = RcBlock::new(move || {
            let still = target
                .mask()
                .and_then(|mask| mask.downcast::<CAShapeLayer>().ok())
                .and_then(|mask| mask.path())
                .is_some_and(|path| std::ptr::eq(&*path, &*placed));
            if still {
                unmask(&target);
            }
        });
        // SAFETY: the block runs on the main thread, with the transaction.
        unsafe { CATransaction::setCompletionBlock(Some(&done)) };
    }
    if moving {
        let slide = |from: &CGPath, to: &CGPath| {
            let animation = CABasicAnimation::animationWithKeyPath(Some(ns_string!("path")));
            // SAFETY: a CGPath is the value Core Animation expects for `path`.
            unsafe {
                animation.setFromValue(Some(&*(std::ptr::from_ref(from).cast::<AnyObject>())));
                animation.setToValue(Some(&*(std::ptr::from_ref(to).cast::<AnyObject>())));
            }
            animation.setDuration(SECONDS);
            animation.setTimingFunction(Some(&CAMediaTimingFunction::functionWithControlPoints(
                0.22, 1.0, 0.36, 1.0,
            )));
            animation
        };
        if let Some(from) = shown {
            mask.addAnimation_forKey(&slide(&from, &hole), Some(ns_string!("rice-notch")));
        }
        let from_line = from_line.or_else(|| {
            let up = PageNotch {
                down: false,
                ..notch
            };
            outline(stage, &layer, up).map(|(_, line)| line.CGPath())
        });
        if let Some(from) = from_line {
            edge.addAnimation_forKey(&slide(&from, &line), Some(ns_string!("rice-notch")));
        }
    }
    CATransaction::commit();
}

/// The hairline along the cut, named so a later placement finds it.
fn edge_name() -> &'static NSString {
    ns_string!("rice-notch-edge")
}

fn edge_of(layer: &CALayer) -> Option<Retained<CAShapeLayer>> {
    // SAFETY: a main-thread read of the layer's own sublayers.
    let sublayers = unsafe { layer.sublayers() }?;
    sublayers.iter().find_map(|sublayer| {
        (sublayer.name().as_deref() == Some(edge_name()))
            .then(|| sublayer.downcast::<CAShapeLayer>().ok())
            .flatten()
    })
}

/// No cut: no mask, and no hairline along it.
fn unmask(layer: &CALayer) {
    // SAFETY: main-thread layer mutation.
    unsafe { layer.setMask(None) };
    if let Some(edge) = edge_of(layer) {
        edge.removeFromSuperlayer();
    }
}

/// The stage's whole area, far past its edges so a resize never shows its
/// border, with the hole taken out of it (even-odd). Up, the hole waits
/// wholly above the page's edge, so the shape keeps the same points and a
/// slide between the two moves it rather than reshaping it.
fn outline(
    stage: &NSView,
    layer: &CALayer,
    notch: PageNotch,
) -> Option<(Retained<NSBezierPath>, Retained<NSBezierPath>)> {
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

    // The hole's own outline, open along the page's top edge; the mask is
    // the stage's whole area with it taken out.
    let path = NSBezierPath::bezierPath();
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
    let line = path.copy();
    path.closePath();
    let hole = NSBezierPath::bezierPathWithRect(NSRect::new(
        NSPoint::new(-w, -h),
        NSSize::new(3.0 * w, 3.0 * h),
    ));
    hole.appendBezierPath(&path);
    Some((hole, line))
}
