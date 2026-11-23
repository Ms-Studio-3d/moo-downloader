//! Persistence for main and progress window geometry.

use super::*;

// ------------------------------------------------------------ window sizes
// The size the user gave a window is kept (Options don't have it: it is
// dragged), and the window opens with it next time.

/// Save before hiding/exiting as well as periodically, including an immediate
/// move then close. Native normal coordinates also work while maximized.
pub(super) fn remember_main_position(mgr: &Arc<Manager>, w: &slint::Window) {
    let Some(position) = winfx::main_position(w) else { return };
    let mut s = mgr.settings();
    if s.main_position != Some(position) {
        s.main_position = Some(position);
        let _ = mgr.set_settings(s);
    }
}

thread_local! {
    // the size each window had at the last look (saved once it stays the same)
    static SEEN_SIZE: RefCell<HashMap<String, (f32, f32)>> = RefCell::new(HashMap::new());
}

/// The size a window had when the program was used last.
pub(crate) fn saved_size(mgr: &Manager, key: &str) -> Option<(f32, f32)> {
    mgr.settings()
        .window_sizes
        .get(key)
        .copied()
        .filter(|&(w, h)| (200.0..10000.0).contains(&w) && (80.0..10000.0).contains(&h))
}

/// The size of `w` now, in the units the windows are made with.
fn size_now(w: &slint::Window) -> Option<(f32, f32)> {
    if !w.is_visible() || w.is_minimized() || w.is_maximized() {
        return None;
    }
    let s = w.size().to_logical(w.scale_factor());
    Some((s.width.round(), s.height.round()))
}

/// Called about once a second: a size that stayed the same since the last
/// look and is new is saved (not while the edge is still being dragged).
pub(crate) fn follow_size(mgr: &Arc<Manager>, key: &str, size: Option<(f32, f32)>) {
    let Some(size) = size else { return };
    let steady = SEEN_SIZE.with(|m| m.borrow_mut().insert(key.to_string(), size) == Some(size));
    if steady && mgr.settings().window_sizes.get(key) != Some(&size) {
        let mut s = mgr.settings();
        s.window_sizes.insert(key.to_string(), size);
        let _ = mgr.set_settings(s);
    }
}

/// The sizes of the main window and the progress windows, once a second.
pub(super) fn follow_sizes(ui: &Ui) {
    if let Some(m) = ui.ctx.main.upgrade() {
        follow_size(&ui.ctx.mgr, "main", size_now(m.window()));
        // Avoid disk writes throughout a drag; close/hide still save at once.
        if !winfx::left_button_down() {
            remember_main_position(&ui.ctx.mgr, m.window());
        }
    }
    for (d, _) in ui.progress.borrow().values() {
        // the height counts only with the details shown (without them the
        // window is as small as it can be)
        if let Some((w, h)) = size_now(d.window()) {
            let h = if d.get_show_details() {
                h
            } else {
                saved_size(&ui.ctx.mgr, "progress").map(|s| s.1).unwrap_or(0.0)
            };
            follow_size(&ui.ctx.mgr, "progress", Some((w, h)));
        }
    }
}
