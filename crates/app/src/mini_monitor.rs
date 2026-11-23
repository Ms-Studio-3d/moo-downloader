//! Floating view of active downloads. Keep the model across ticks so that
//! scrolling and keyboard focus survive progress updates.
use crate::{
    bring_to_front, early_download, fmt_size,
    i18n::{t, tf},
    set_theme_of, update_model, winfx, Ctx, DownloadView, Look, Manager, MiniDownloadRow,
    MiniMonitor, Theme,
};
use slint::{ComponentHandle, ModelRc, Timer, TimerMode, VecModel};
use std::{rc::Rc, sync::Arc, time::Duration};

#[cfg(all(test, windows))]
#[path = "mini_monitor_test.rs"]
mod tests;

pub(crate) fn remember_position(mgr: &Arc<Manager>, window: &slint::Window) {
    let Some(position) = winfx::main_position(window) else { return };
    let mut settings = mgr.settings();
    if settings.mini_position != Some(position) {
        settings.mini_position = Some(position);
        let _ = mgr.set_settings(settings);
    }
}

fn row(view: DownloadView) -> MiniDownloadRow {
    let progress = view.progress().clamp(0.0, 1.0);
    let amount = if view.total_size.is_some_and(|size| size > 0) || view.parts_total > 0 {
        format!("{:.0}%", progress * 100.0)
    } else {
        fmt_size(view.downloaded)
    };
    let speed = fmt_size(view.speed.max(0.0) as u64);
    MiniDownloadRow {
        name: view.file_name.into(),
        detail: tf("{}  •  {}/s", &[&amount, &speed]).into(),
        progress,
    }
}

pub(crate) fn wire(ctx: &Ctx, mini: &MiniMonitor) -> Timer {
    let model = Rc::new(VecModel::default());
    mini.set_rows(ModelRc::from(model.clone()));
    let refresh = {
        let ctx = ctx.clone();
        let weak = mini.as_weak();
        Rc::new(move || {
            let Some(mini) = weak.upgrade() else { return };
            if !mini.window().is_visible() || mini.window().is_minimized() { return; }
            set_theme_of(mini.global::<Theme>(), &Look::of(&ctx.mgr.settings()));
            let pending = early_download(&ctx);
            let rows: Vec<_> = ctx.mgr.running_downloads().into_iter()
                .filter(|v| Some(v.id) != pending).map(row).collect();
            let summary = match rows.len() {
                0 => t("No active downloads"),
                1 => t("1 active download"),
                n => tf("{} active downloads", &[&n]),
            };
            mini.set_summary(summary.into());
            update_model(&model, rows);
        })
    };
    if let Some(main) = ctx.main.upgrade() {
        let weak = mini.as_weak();
        let ctx = ctx.clone();
        let refresh = refresh.clone();
        main.on_open_mini(move || {
            let Some(mini) = weak.upgrade() else { return };
            let opening = !mini.window().is_visible();
            if opening { winfx::send_away(mini.window()); }
            if mini.show().is_err() { return; }
            mini.window().set_minimized(false);
            if opening {
                winfx::keep_drawn(&mini);
                winfx::place_mini(weak.clone(), ctx.mgr.settings().mini_position);
            }
            refresh();
        });
    }
    let mgr = ctx.mgr.clone();
    let weak = mini.as_weak();
    mini.window().on_close_requested(move || {
        if let Some(mini) = weak.upgrade() { remember_position(&mgr, mini.window()); }
        slint::CloseRequestResponse::HideWindow
    });
    let main_ctx = ctx.clone();
    mini.on_open_main(move || bring_to_front(&main_ctx));
    let timer = Timer::default();
    timer.start(TimerMode::Repeated, Duration::from_secs(1), move || refresh());
    timer
}
