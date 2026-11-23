//! Download information dialog handlers.

use super::*;

pub(super) fn wire_add(ui: &Ui) {
    let Some(add) = ui.ctx.add.upgrade() else { return };
    {
        let ctx = ui.ctx.clone();
        add.on_check_url(move || {
            if let Some(d) = ctx.add.upgrade() {
                let url = d.get_url().trim().to_string();
                // another address: the download started for the old one stops
                if url != ctx.add_state.lock_ok().origin_url {
                    cancel_early(&ctx);
                }
                if swiftget_engine::torrent::is_magnet(&url) {
                    // typed / pasted a magnet link: that is the torrent window's job
                    let _ = d.hide();
                    torrent_ui::open_source(&url);
                    return;
                }
                if !url.is_empty() {
                    start_probe(&ctx, url);
                }
            }
        });
    }
    {
        let ctx = ui.ctx.clone();
        add.on_category_changed(move |i| {
            let Some(d) = ctx.add.upgrade() else { return };
            let cat = ctx
                .add_state
                .lock()
                .unwrap()
                .categories
                .get(i.max(0) as usize)
                .cloned()
                .unwrap_or_else(|| "General".into());
            let current = PathBuf::from(d.get_save_as().as_str());
            let name = current
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "download".into());
            let s = ctx.mgr.settings();
            let path = s.dir_for(&cat).join(&name).display().to_string();
            d.set_save_as(path.clone().into());
            d.set_category_name(t(&cat).into());
            d.set_remember_path(s.category_dirs.contains_key(&cat));
            ctx.add_state.lock_ok().auto_path = path;
        });
    }
    {
        let w = add.as_weak();
        add.on_browse(move || {
            let Some(d) = w.upgrade() else { return };
            let current = PathBuf::from(d.get_save_as().as_str());
            let mut dlg = crate::winfx::file_box();
            if let Some(dir) = current.parent().filter(|p| p.exists()) {
                dlg = dlg.set_directory(dir);
            }
            if let Some(name) = current.file_name() {
                dlg = dlg.set_file_name(name.to_string_lossy());
            }
            // Windows adds the file's own extension when a name is typed without it
            if let Some(ext) = current.extension().map(|e| e.to_string_lossy().into_owned()).filter(|e| !e.is_empty()) {
                dlg = dlg.add_filter(ext.to_ascii_uppercase(), &[ext.as_str()]).add_filter("*", &["*"]);
            }
            if let Some(p) = dlg.save_file() {
                let suggested = current.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                let name = p.file_name().map(|n| with_extension_of(&n.to_string_lossy(), &suggested)).unwrap_or_default();
                let p = p.with_file_name(name);
                d.set_save_as(p.display().to_string().into());
            }
        });
    }
    {
        let ui = ui.clone();
        add.on_start_now(move || confirm_add(&ui, true));
    }
    {
        // the server refused us: give the address back to the browser
        let ctx = ui.ctx.clone();
        add.on_use_browser(move || {
            let Some(d) = ctx.add.upgrade() else { return };
            let url = d.get_url().trim().to_string();
            ctx.add_state.lock_ok().probe_seq += 1;
            cancel_early(&ctx);
            let _ = d.hide();
            if url.starts_with("http://") || url.starts_with("https://") {
                api::leave_to_browser(&url);
                let _ = open::that_detached(&url);
            }
            open_next(&ctx);
        });
    }
    {
        let ui = ui.clone();
        add.on_later(move || confirm_add(&ui, false));
    }
    {
        let ctx = ui.ctx.clone();
        add.on_cancel(move || {
            ctx.add_state.lock_ok().probe_seq += 1; // ignore a check still running
            forget_look(&ctx);
            cancel_early(&ctx);
            if let Some(d) = ctx.add.upgrade() {
                let _ = d.hide();
            }
            open_next(&ctx);
        });
    }
    {
        let ctx = ui.ctx.clone();
        add.window().on_close_requested(move || {
            ctx.add_state.lock_ok().probe_seq += 1;
            forget_look(&ctx);
            cancel_early(&ctx);
            open_next(&ctx);
            slint::CloseRequestResponse::HideWindow
        });
    }
}

// ===================================================================== batch
