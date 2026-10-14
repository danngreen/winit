//! Guards for the pointer fixes this branch carries on top of winit 0.30.13.
//!
//! Each test checks that a fix is still in the source, or runs its policy where that is plain
//! Rust, and fails if a rebase drops it. When a winit release carries an equivalent fix, drop the
//! patch together with its test.

use std::path::Path;

fn source(path: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/platform_impl").join(path);
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()))
}

/// The text after `start`, up to the first `end`.
fn section<'a>(text: &'a str, start: &str, end: &str) -> &'a str {
    let (_, rest) = text.split_once(start).unwrap_or_else(|| panic!("`{start}` not found"));
    rest.split_once(end).map_or(rest, |(body, _)| body)
}

#[test]
fn macos_title_bar_drags_reach_the_app() {
    // NSView answers YES to `mouseDownCanMoveWindow` for a view that isn't opaque. With a
    // transparent title bar over a full-size content view, AppKit then moved the window for any
    // drag that began in the title-bar band, before the app saw it, so tabs drawn there could not
    // be dragged. The view answers with the window's `movable_by_window_background` (NO unless
    // asked); apps move the window from their own title bar with `drag_window`
    // (storytold/pdfcraft#2347).
    let view = source("macos/view.rs");
    let method = section(&view, "#[method(mouseDownCanMoveWindow)]", "\n        }\n");
    assert!(
        method.contains("self.ivars().movable_by_window_background"),
        "mouseDownCanMoveWindow doesn't follow the window's setting"
    );
    let delegate = source("macos/window_delegate.rs");
    assert!(
        delegate.contains("attrs.platform_specific.movable_by_window_background,"),
        "the view isn't given the window's setting"
    );
}

#[test]
fn macos_custom_cursor_images_are_sized_in_points() {
    // AppKit sizes a cursor image in points. winit gave the image its bitmap's size in pixels,
    // so on a Retina display a bitmap drawn at the display's scale showed twice as big, and one
    // drawn at 1x was upscaled and blurred (rust-windowing/winit#4094). The image and its hotspot
    // are the bitmap's pixels over the backing scale of the screen under the pointer.
    let cursor = source("macos/cursor.rs");
    let from_image = section(&cursor, "pub(crate) fn cursor_from_image(", "\n}\n");
    assert!(
        from_image.contains("f64::from(width) / scale, f64::from(height) / scale"),
        "the cursor image is sized in pixels"
    );
    assert!(
        from_image.contains("f64::from(cursor.hotspot_x) / scale"),
        "the hotspot is placed in pixels"
    );
    let scale = section(&cursor, "fn pointer_screen_scale(", "\n}\n");
    assert!(scale.contains("backingScaleFactor()"), "the scale isn't the screen's backing scale");
}

// The scroll-phase policy is plain Rust over winit's own `TouchPhase`, so its tests run here.
#[cfg(wayland_platform)]
mod event {
    pub use winit::event::TouchPhase;
}
#[cfg(wayland_platform)]
#[path = "../src/platform_impl/linux/wayland/seat/pointer/scroll.rs"]
mod scroll;

#[test]
fn wayland_pixel_wheels_report_moved() {
    // winit 0.30.13 starts a finger gesture (`TouchPhase::Started`) for any axis event without
    // discrete steps, and only `axis_stop` ends it, which wheels need not send. A wheel, wheel tilt
    // or continuous source reporting only pixel deltas then left a gesture open, and egui
    // kept the modifiers it started with: Ctrl zoom stayed on after Ctrl was released. Such events
    // report `Moved`, under the rules `scroll::phase` tests (storytold/pdfcraft#1569).
    let pointer = source("linux/wayland/seat/pointer/mod.rs");
    let axis = section(&pointer, "PointerEventKind::Axis {", "pointer_data.phase = phase;");
    assert!(
        axis.contains("let phase = scroll::phase("),
        "axis events don't take their phase from the scroll source"
    );
}
