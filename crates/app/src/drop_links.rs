//! Windows OLE drop target for browser URLs and Explorer torrent files.
use crate::{open_add_dialog, torrent_ui, winfx, Ctx};
use slint::ComponentHandle;
use std::{cell::Cell, ffi::OsString, os::windows::ffi::OsStringExt, path::PathBuf};
use windows::Win32::{
    Foundation::{HGLOBAL, HWND, POINTL},
    System::{
        Com::{IDataObject, DVASPECT_CONTENT, FORMATETC, STGMEDIUM, TYMED_HGLOBAL},
        DataExchange::RegisterClipboardFormatW,
        Memory::{GlobalLock, GlobalSize, GlobalUnlock},
        Ole::{IDropTarget, IDropTarget_Impl, RegisterDragDrop, ReleaseStgMedium, RevokeDragDrop,
            CF_HDROP, CF_UNICODETEXT, DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_LINK, DROPEFFECT_NONE},
        SystemServices::MODIFIERKEYS_FLAGS,
    },
    UI::Shell::{DragQueryFileW, HDROP},
};
use windows_core::{implement, w, Ref, Result};

enum Item { Link(String), Torrent(PathBuf) }

fn links(text: &str) -> Vec<Item> {
    text.lines().map(str::trim).filter(|s| {
        s.len() <= 16_384 && !s.chars().any(|c| c.is_whitespace() || c.is_control())
            && url::Url::parse(s).is_ok_and(|u| matches!(u.scheme(), "http" | "https") && u.has_host())
    }).take(64).map(|s| Item::Link(s.to_owned())).collect()
}

fn format(id: u16) -> FORMATETC {
    FORMATETC { cfFormat: id, dwAspect: DVASPECT_CONTENT.0, lindex: -1, tymed: TYMED_HGLOBAL.0 as u32, ..Default::default() }
}

struct Medium(STGMEDIUM);
impl Drop for Medium {
    fn drop(&mut self) { unsafe { ReleaseStgMedium(&mut self.0) } }
}

fn unicode_text(handle: HGLOBAL) -> Option<String> {
    // The data belongs to the source. Bound the read by the allocated size and
    // release the lock before returning the string.
    unsafe {
        let size = GlobalSize(handle);
        if !(2..=256 * 1024).contains(&size) || size % 2 != 0 { return None; }
        let ptr = GlobalLock(handle).cast::<u16>();
        if ptr.is_null() { return None; }
        let data = std::slice::from_raw_parts(ptr, size / 2);
        let text = data.iter().position(|&c| c == 0).and_then(|end| String::from_utf16(&data[..end]).ok());
        let _ = GlobalUnlock(handle);
        text
    }
}

fn torrents(handle: HGLOBAL) -> Vec<Item> {
    let drop = HDROP(handle.0);
    let mut items = Vec::new();
    // DragQueryFileW takes the HDROP itself and copies each path to our buffer.
    unsafe {
        for i in 0..DragQueryFileW(drop, u32::MAX, None).min(64) {
            let len = DragQueryFileW(drop, i, None);
            if !(1..=32767).contains(&len) { continue; }
            let mut buf = vec![0; len as usize + 1];
            let read = DragQueryFileW(drop, i, Some(&mut buf)) as usize;
            let path = PathBuf::from(OsString::from_wide(&buf[..read]));
            if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("torrent")) {
                items.push(Item::Torrent(path));
            }
        }
    }
    items
}

#[implement(IDropTarget)]
struct BrowserDrop {
    ctx: Ctx,
    formats: [u16; 3],
    accepted: Cell<bool>,
}

impl BrowserDrop_Impl {
    fn effect(&self, ptr: *mut DROPEFFECT, accepted: bool) {
        // The incoming value lists the operations allowed by the source.
        if let Some(effect) = unsafe { ptr.as_mut() } {
            *effect = if accepted && effect.0 & DROPEFFECT_COPY.0 != 0 { DROPEFFECT_COPY }
                else if accepted && effect.0 & DROPEFFECT_LINK.0 != 0 { DROPEFFECT_LINK }
                else { DROPEFFECT_NONE };
        }
    }

    fn extract(&self, data: &IDataObject) -> Vec<Item> {
        for id in self.formats.into_iter().filter(|&id| id != 0) {
            let Ok(medium) = (unsafe { data.GetData(&format(id)) }) else { continue };
            let medium = Medium(medium);
            if medium.0.tymed != TYMED_HGLOBAL.0 as u32 { continue; }
            // The checked TYMED selects hGlobal from the STGMEDIUM union.
            let handle = unsafe { medium.0.u.hGlobal };
            if handle.is_invalid() { continue; }
            let items = if id == CF_HDROP.0 { torrents(handle) }
                else { unicode_text(handle).map(|text| links(&text)).unwrap_or_default() };
            if !items.is_empty() { return items; }
        }
        Vec::new()
    }
}

#[allow(non_snake_case)]
impl IDropTarget_Impl for BrowserDrop_Impl {
    fn DragEnter(&self, data: Ref<IDataObject>, _: MODIFIERKEYS_FLAGS, _: &POINTL, effect: *mut DROPEFFECT) -> Result<()> {
        let ok = data.as_ref().is_some_and(|d| self.formats.iter().any(|&id| id != 0 && unsafe { d.QueryGetData(&format(id)).is_ok() }));
        self.accepted.set(ok);
        self.effect(effect, ok);
        Ok(())
    }
    fn DragOver(&self, _: MODIFIERKEYS_FLAGS, _: &POINTL, effect: *mut DROPEFFECT) -> Result<()> {
        self.effect(effect, self.accepted.get());
        Ok(())
    }
    fn DragLeave(&self) -> Result<()> { self.accepted.set(false); Ok(()) }
    fn Drop(&self, data: Ref<IDataObject>, _: MODIFIERKEYS_FLAGS, _: &POINTL, effect: *mut DROPEFFECT) -> Result<()> {
        self.accepted.set(false);
        let items = data.as_ref().map(|d| self.extract(d)).unwrap_or_default();
        self.effect(effect, !items.is_empty());
        if !items.is_empty() && unsafe { effect.as_ref() }.is_some_and(|e| *e != DROPEFFECT_NONE) {
            let ctx = self.ctx.clone();
            // Finish the native drag loop before opening a Slint dialog.
            slint::Timer::single_shot(std::time::Duration::ZERO, move || {
                for item in items {
                    match item {
                        Item::Link(url) => open_add_dialog(&ctx, url, Default::default(), None, Default::default()),
                        Item::Torrent(path) if path.is_file() => torrent_ui::open_source(&path.to_string_lossy()),
                        _ => {}
                    }
                }
            });
        }
        Ok(())
    }
}

pub(super) fn install(ctx: &Ctx) {
    let Some(main) = ctx.main.upgrade() else { return };
    let Some(id) = winfx::hwnd(main.window()) else { return };
    let hwnd = HWND(id as _);
    // Winit initializes OLE on this thread and revokes the HWND's registration
    // on destruction. Replace its files-only handler each time a HWND is shown.
    unsafe {
        let url = RegisterClipboardFormatW(w!("UniformResourceLocatorW")) as u16;
        let target: IDropTarget = BrowserDrop { ctx: ctx.clone(), formats: [url, CF_UNICODETEXT.0, CF_HDROP.0], accepted: Cell::new(false) }.into();
        let _ = RevokeDragDrop(hwnd);
        if let Err(e) = RegisterDragDrop(hwnd, &target) { crate::note(&format!("Could not enable link drops: {e}")); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn drop_opens_only_web_links_and_keeps_their_query() {
        let example = "https://example.org/a?q=a%20b\r\nPage title\nfile:///C:/x\njavascript:alert(1)\nhttps://example.org/b";
        let values: Vec<_> = links(example).into_iter().filter_map(|i| match i { Item::Link(s) => Some(s), _ => None }).collect();
        assert_eq!(values, ["https://example.org/a?q=a%20b", "https://example.org/b"]);
    }
}
