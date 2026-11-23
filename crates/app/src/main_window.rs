//! Main window event handlers.

use super::*;

pub(super) fn wire_main(ui: &Ui) {
    let Some(main) = ui.ctx.main.upgrade() else { return };
    let mgr = ui.ctx.mgr.clone();
    {
        let ctx = ui.ctx.clone();
        main.on_add_url(move || {
            let url = arboard::Clipboard::new()
                .ok()
                .and_then(|mut c| c.get_text().ok())
                .and_then(|t| clipboard::downloadable_url(&t).or_else(|| is_http_url(&t)))
                .unwrap_or_default();
            open_add_dialog(&ctx, url, RequestHeaders::default(), None, VideoExtra::default());
        });
    }
    {
        let ui = ui.clone();
        main.on_add_batch(move || open_batch(&ui, String::new()));
    }
    {
        let ui = ui.clone();
        main.on_add_batch_clipboard(move || {
            let text = arboard::Clipboard::new().ok().and_then(|mut c| c.get_text().ok()).unwrap_or_default();
            open_batch(&ui, text);
        });
    }
    {
        let ui = ui.clone();
        main.on_resume(move |id| {
            let targets = targets(&ui, id as u64);
            for &t in &targets {
                ui.ctx.mgr.start(t);
            }
            if targets.len() <= 8 {
                for &t in &targets {
                    show_progress_for_start(&ui, t);
                }
            }
            refresh(&ui);
        });
    }
    {
        let ui = ui.clone();
        main.on_pause(move |id| {
            for t in targets(&ui, id as u64) {
                ui.ctx.mgr.pause(t);
            }
            refresh(&ui);
        });
    }
    {
        let mgr = mgr.clone();
        main.on_resume_all(move || mgr.start_all());
    }
    {
        let mgr = mgr.clone();
        main.on_stop_all(move || mgr.pause_all());
    }
    {
        let ui = ui.clone();
        main.on_delete(move |id| {
            // not inside the key press / click itself: the message box runs
            // its own message loop (holding Delete down asked many times)
            if modal_open() {
                return;
            }
            let ids = targets(&ui, id as u64);
            let ui = ui.clone();
            Timer::single_shot(Duration::from_millis(1), move || {
                if !modal_open() {
                    delete_downloads(&ui, ids);
                    refresh(&ui);
                }
            });
        });
    }
    {
        let ui = ui.clone();
        main.on_delete_completed(move || {
            let ui2 = ui.clone();
            confirm(&ui.ctx, APP, &t("Remove every finished download from the list?\n(The files stay on the disk.)"), move || {
                ui2.ctx.mgr.remove_completed();
                refresh(&ui2);
            });
        });
    }
    {
        let ui = ui.clone();
        main.on_view_changed(move |_| refresh(&ui));
    }
    {
        let ui = ui.clone();
        main.on_search_changed(move || refresh(&ui));
    }
    {
        let ui = ui.clone();
        main.on_open_settings(move |tab| options::open(&ui, tab));
    }
    {
        let ui = ui.clone();
        main.on_open_limiter(move || options::open(&ui, 8));
    }
    {
        let ui = ui.clone();
        main.on_open_scheduler(move || scheduler::open(&ui, None));
    }
    {
        let ui = ui.clone();
        main.on_toggle_limiter(move || toggle_limiter(&ui));
    }
    {
        let ui = ui.clone();
        main.on_sort(move |key| {
            let mut s = ui.ctx.mgr.settings();
            if s.sort_by == key.as_str() {
                s.sort_desc = !s.sort_desc;
            } else {
                s.sort_by = key.to_string();
                s.sort_desc = matches!(key.as_str(), "date" | "size" | "added");
            }
            if let Some(m) = ui.ctx.main.upgrade() {
                m.set_sort_by(s.sort_by.clone().into());
                m.set_sort_desc(s.sort_desc);
            }
            let _ = ui.ctx.mgr.set_settings(s);
            refresh(&ui);
        });
    }
    {
        let ui = ui.clone();
        main.on_set_theme(move |m| {
            let mut s = ui.ctx.mgr.settings();
            s.theme = m.clamp(0, 12) as u32;
            let _ = ui.ctx.mgr.set_settings(s);
            apply_theme(&ui, m);
        });
    }
    {
        let ui = ui.clone();
        main.on_set_language(move |lang| {
            let mut s = ui.ctx.mgr.settings();
            s.language = lang.to_string();
            let _ = ui.ctx.mgr.set_settings(s);
            change_language(&ui, &lang);
        });
    }
    {
        let ui = ui.clone();
        main.on_toggle_categories(move || {
            let mut s = ui.ctx.mgr.settings();
            s.hide_categories = !s.hide_categories;
            if let Some(m) = ui.ctx.main.upgrade() {
                m.set_hide_categories(s.hide_categories);
            }
            let _ = ui.ctx.mgr.set_settings(s);
        });
    }
    {
        let ui = ui.clone();
        main.on_set_toolbar_small(move |small| {
            let mut s = ui.ctx.mgr.settings();
            s.toolbar_small = small;
            if let Some(m) = ui.ctx.main.upgrade() {
                m.set_toolbar_small(small);
            }
            let _ = ui.ctx.mgr.set_settings(s);
        });
    }
    {
        let ui = ui.clone();
        main.on_start_queue(move |q| {
            ui.ctx.mgr.start_queue(q as u64);
            refresh(&ui);
        });
    }
    {
        let ui = ui.clone();
        main.on_stop_queue(move |q| {
            ui.ctx.mgr.stop_queue(q as u64);
            refresh(&ui);
        });
    }
    {
        let ui = ui.clone();
        main.on_new_queue(move || scheduler::new_queue(&ui));
    }
    {
        let ui = ui.clone();
        main.on_edit_queue(move |q| scheduler::open(&ui, Some(q as u64)));
    }
    {
        let ui = ui.clone();
        main.on_delete_queue(move |q| scheduler::delete_queue(&ui, q as u64));
    }
    {
        let ui = ui.clone();
        main.on_row_activated(move |id| {
            let Some(v) = ui.ctx.mgr.get(id as u64) else { return };
            if ui.ctx.mgr.settings().double_click == 1 {
                extra::open_properties(&ui, v.id);
            } else if v.status == Status::Completed {
                open_download(&v.save_path);
            } else {
                open_progress(&ui, v.id);
            }
        });
    }
    {
        let ui = ui.clone();
        main.on_row_action(move |id, action| {
            let targets = targets(&ui, id as u64);
            match action.as_str() {
                // these work on every selected download
                "resume" | "stop" | "restart" | "delete" | "copy" if targets.len() > 1 => {
                    multi_action(&ui, &targets, action.as_str())
                }
                a => row_action(&ui, id as u64, a),
            }
            refresh(&ui);
        });
    }
    {
        let ui = ui.clone();
        main.on_row_to_queue(move |id, q| {
            for t in targets(&ui, id as u64) {
                ui.ctx.mgr.set_queue(t, (q > 0).then_some(q as u64));
            }
            refresh(&ui);
        });
    }
    {
        let ui = ui.clone();
        main.on_move_selection(move |step, shift| {
            let order = visible_ids(&ui);
            let Some(m) = ui.ctx.main.upgrade() else { return };
            if order.is_empty() {
                return;
            }
            let at = order.iter().position(|i| *i as i32 == m.get_selected_id());
            let next = match (step, at) {
                (s, _) if s <= -2 => 0,
                (s, _) if s >= 2 => order.len() - 1,
                (s, None) if s < 0 => 0,
                (_, None) => order.len() - 1,
                (s, Some(i)) => (i as i64 + s as i64).clamp(0, order.len() as i64 - 1) as usize,
            };
            row_clicked(&ui, order[next], false, shift);
            refresh(&ui);
        });
    }
    main.on_show_shortcuts(|| message(&t(SHORTCUTS)));
    {
        let ui = ui.clone();
        main.on_row_clicked(move |id, ctrl, shift| {
            // Ctrl / Shift as Windows has them now (a menu or another window
            // may have kept the program from seeing one let go)
            let (ctrl, shift) = winfx::ctrl_shift_held().unwrap_or((ctrl, shift));
            row_clicked(&ui, id as u64, ctrl, shift);
            refresh(&ui);
        });
    }
    {
        let ui = ui.clone();
        main.on_row_right(move |id| {
            if !ui.sel.borrow().ids.contains(&(id as u64)) {
                select_single(&ui, id as u64);
            }
            if let Some(m) = ui.ctx.main.upgrade() {
                m.set_selected_id(id);
            }
            refresh(&ui);
        });
    }
    main.set_native_menu(winmenu::available());
    {
        let ui = ui.clone();
        main.on_row_menu(move |id| {
            // the newly selected row is drawn first (a Windows menu stops
            // the drawing of the program while it is open), then the menu
            let ui = ui.clone();
            if let Some(m) = ui.ctx.main.upgrade() {
                m.window().request_redraw();
            }
            slint::Timer::single_shot(Duration::from_millis(60), move || row_menu(&ui, id));
        });
    }
    {
        let ui = ui.clone();
        main.on_row_drag(move |id, offset| {
            // only while the left button is really held: Windows may keep the
            // release of a click that closed a menu, and the list must not
            // select rows when the mouse just moves
            if !winfx::left_button_down() {
                return;
            }
            // select with the mouse: from the row where the drag started
            let order = visible_ids(&ui);
            let Some(start) = order.iter().position(|x| *x == id as u64) else { return };
            let end = (start as i64 + offset as i64).clamp(0, order.len() as i64 - 1) as usize;
            let mut sel = ui.sel.borrow_mut();
            sel.ids = order[start.min(end)..=start.max(end)].to_vec();
            sel.anchor = Some(id as u64);
            drop(sel);
            refresh(&ui);
        });
    }
    {
        // the header: drag an edge to resize a column (live), let go to keep it
        let ui = ui.clone();
        main.on_column_resized(move |i, w| {
            if let Some(mut c) = ui.columns_model.row_data(i.max(0) as usize) {
                c.width = w.clamp(36.0, 2000.0);
                ui.columns_model.set_row_data(i as usize, c);
            }
        });
    }
    {
        let ui = ui.clone();
        main.on_columns_changed(move || save_columns(&ui));
    }
    {
        // the header: a title dragged onto another place moves the column there
        let ui = ui.clone();
        main.on_column_dropped(move |from, x| {
            let widths: Vec<f32> = (0..ui.columns_model.row_count()).filter_map(|i| ui.columns_model.row_data(i)).map(|c| c.width).collect();
            let to = column_at(&widths, x);
            let from = from.max(0) as usize;
            if from < widths.len() && to != from {
                if let Some(c) = ui.columns_model.row_data(from) {
                    ui.columns_model.remove(from);
                    ui.columns_model.insert(to.min(ui.columns_model.row_count()), c);
                    save_columns(&ui);
                }
            }
        });
    }
    {
        let ui = ui.clone();
        main.on_row_drag_out(move |id| {
            if !winfx::left_button_down() {
                return;
            }
            // a finished file pulled out of the list: Windows takes it from here
            let Some(v) = ui.ctx.mgr.get(id as u64) else { return };
            if v.status != Status::Completed || !v.save_path.is_file() {
                return;
            }
            let Some(m) = ui.ctx.main.upgrade() else { return };
            winfx::drag_file_out(m.window(), &v.save_path);
            // the mouse was let go outside the list: the row must know too
            m.window().dispatch_event(slint::platform::WindowEvent::PointerReleased {
                position: slint::LogicalPosition::new(0.0, 0.0),
                button: slint::platform::PointerEventButton::Left,
            });
            m.window().dispatch_event(slint::platform::WindowEvent::PointerExited);
        });
    }
    {
        let ui = ui.clone();
        main.on_select_all(move || {
            let ids = visible_ids(&ui);
            let mut sel = ui.sel.borrow_mut();
            sel.anchor = ids.first().copied();
            sel.ids = ids;
            drop(sel);
            refresh(&ui);
        });
    }
    {
        let mgr = mgr.clone();
        main.on_open_download_folder(move || {
            let dir = mgr.settings().download_dir;
            let _ = std::fs::create_dir_all(&dir);
            let _ = open::that_detached(&dir);
        });
    }
    {
        let mgr = mgr.clone();
        main.on_export_list(move |kind| extra::export_list(&mgr, kind.as_str()));
    }
    {
        let ui = ui.clone();
        main.on_import_list(move |kind| extra::import_list(&ui, kind.as_str()));
    }
    {
        let ui = ui.clone();
        main.on_set_all_done(move |i| {
            ui.ctx.mgr.set_when_all_done(menu_done_from(i));
            if let Some(m) = ui.ctx.main.upgrade() {
                m.set_all_done(i);
            }
        });
    }
    {
        let ui = ui.clone();
        main.on_open_columns(move || extra::open_columns(&ui));
    }
    {
        let ui = ui.clone();
        main.on_add_category(move || extra::open_category(&ui, None));
    }
    {
        let ui = ui.clone();
        main.on_category_props(move |name| extra::open_category(&ui, Some(name.to_string())));
    }
    {
        let ui = ui.clone();
        main.on_delete_category(move |name| extra::delete_category(&ui, name.as_str()));
    }
    {
        let ui = ui.clone();
        main.on_find(move || extra::open_find(&ui));
    }
    {
        let ui = ui.clone();
        main.on_find_next(move || extra::find_next(&ui));
    }
    {
        let ui = ui.clone();
        main.on_grabber(move || grab::open_grabber(&ui));
    }
    {
        let ui = ui.clone();
        main.on_clip_part(move || clipui::open(&ui, None));
    }
    {
        let ui = ui.clone();
        main.on_tips(move || extra::show_tip(&ui, true));
    }
    {
        let ui = ui.clone();
        main.on_playlist(move || playlist::open(&ui));
    }
    {
        let ui = ui.clone();
        main.on_torrent(move || torrent_ui::open(&ui, None));
    }
    {
        let ui = ui.clone();
        main.on_browsers(move || browser::open(&ui));
    }
    let (tools_client, tools_rt) = (mgr.client(), ui.ctx.rt.clone());
    let tools_busy = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    main.on_update_tools(move || {
        use std::sync::atomic::Ordering;
        if tools_busy.swap(true, Ordering::SeqCst) { return; }
        let busy = tools_busy.clone();
        let client = tools_client.clone();
        tools_rt.spawn(async move {
            let result = swiftget_engine::tools::update_all(client).await;
            busy.store(false, Ordering::SeqCst);
            let _ = slint::invoke_from_event_loop(move || {
                let text = match result {
                    Ok(out) => tf("Moo Downloader {}\n\nThe video downloader (yt-dlp) is up to date.\n\n{}", &[&env!("CARGO_PKG_VERSION"), &out]),
                    Err(e) => tf("Could not update yt-dlp:\n\n{}", &[&format!("{e:#}")]),
                };
                message(&text);
            });
        });
    });
    {
        let ui2 = ui.clone();
        main.on_check_app_update(move || update::check_now(&ui2));
    }
    if let Some(d) = ui.updater.upgrade() {
        let w = d.as_weak();
        d.on_close_window(move || {
            if let Some(d) = w.upgrade() {
                let _ = d.hide();
            }
        });
    }
    main.on_about(about);
    main.on_quit(|| {
        let _ = slint::quit_event_loop();
    });
}
