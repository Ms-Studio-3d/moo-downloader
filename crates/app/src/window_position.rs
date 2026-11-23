//! Keep native workspace coordinates paired with Get/SetWindowPlacement.
//! Screen coordinates would drift when the taskbar is at the top or left.

use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowPlacement, GetWindowRect, SetWindowPlacement, WINDOWPLACEMENT};

fn placement(hwnd: HWND) -> Option<WINDOWPLACEMENT> {
    // SAFETY: WINDOWPLACEMENT is plain data; Windows writes it after checking
    // its length and the window handle.
    let mut p: WINDOWPLACEMENT = unsafe { std::mem::zeroed() };
    p.length = std::mem::size_of::<WINDOWPLACEMENT>() as u32;
    (unsafe { GetWindowPlacement(hwnd, &mut p) } != 0).then_some(p)
}

/// The normal position survives hiding, minimizing and maximizing.
pub(crate) fn read(hwnd: HWND, away: (i32, i32)) -> Option<(i32, i32)> {
    let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    // SAFETY: `rect` is writable and Windows validates the handle.
    if unsafe { GetWindowRect(hwnd, &mut rect) } == 0 || (rect.left, rect.top) == away {
        // Never persist the temporary off-screen position used before paint.
        return None;
    }
    let p = placement(hwnd)?;
    Some((p.rcNormalPosition.left, p.rcNormalPosition.top))
}

/// Keep the current size/show state, moving only the normal position.
/// Windows brings an off-screen placement back if a monitor was removed.
pub(crate) fn restore(hwnd: HWND, position: (i32, i32)) -> bool {
    let Some(mut p) = placement(hwnd) else { return false };
    let Some(width) = p.rcNormalPosition.right.checked_sub(p.rcNormalPosition.left) else { return false };
    let Some(height) = p.rcNormalPosition.bottom.checked_sub(p.rcNormalPosition.top) else { return false };
    let Some(right) = position.0.checked_add(width) else { return false };
    let Some(bottom) = position.1.checked_add(height) else { return false };
    if width <= 0 || height <= 0 {
        return false;
    }
    p.rcNormalPosition = RECT { left: position.0, top: position.1, right, bottom };
    // SAFETY: the initialized placement retains the window's own show state
    // and size. Its coordinates use the same units as GetWindowPlacement.
    unsafe { SetWindowPlacement(hwnd, &p) != 0 }
}
