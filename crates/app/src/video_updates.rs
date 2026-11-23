//! Check for yt-dlp updates without installing anything until the user agrees.

use crate::{ask, i18n::tf, message, Ctx, Ui};
use slint::{ComponentHandle, Timer, TimerMode};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::{atomic::{AtomicBool, Ordering}, Arc};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const CHECK_INTERVAL: u64 = 24 * 60 * 60;

pub(crate) fn start(ui: &Ui) -> Timer {
    let mgr = ui.ctx.mgr.clone();
    let ctx = ui.ctx.clone();
    let runtime = ctx.rt.clone();
    let main = ui.ctx.main.clone();
    let busy = Arc::new(AtomicBool::new(false));
    let last_attempt = Rc::new(Cell::new(0u64));
    let check: Rc<dyn Fn()> = Rc::new(move || {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        if now.saturating_sub(mgr.settings().video_update_checked_at) < CHECK_INTERVAL
            || now.saturating_sub(last_attempt.get()) < 15 * 60
            || crate::modal_open()
            || !main.upgrade().is_some_and(|w| w.window().is_visible() && !w.window().is_minimized())
            || busy.swap(true, Ordering::SeqCst)
        {
            return;
        }
        last_attempt.set(now);
        let (mgr, main, runtime, busy) = (mgr.clone(), main.clone(), runtime.clone(), busy.clone());
        let ctx = ctx.clone();
        runtime.spawn(async move {
            let result = swiftget_engine::tools::video_update_available(mgr.client()).await;
            busy.store(false, Ordering::SeqCst);
            let _ = slint::invoke_from_event_loop(move || {
                let Ok(available) = result else { return };
                let Some(window) = main.upgrade() else { return };
                if !window.window().is_visible() || window.window().is_minimized() || crate::modal_open() {
                    return;
                }
                let mut settings = mgr.settings();
                settings.video_update_checked_at = now;
                let _ = mgr.set_settings(settings);
                if let Some((current, latest)) = available {
                    let question = tf("A new version of yt-dlp is available ({} → {}). Update it now?", &[&current, &latest]);
                    if ask(&question) {
                        install(&ctx);
                    }
                }
            });
        });
    });
    let first = check.clone();
    Timer::single_shot(Duration::from_secs(12), move || first());
    let timer = Timer::default();
    timer.start(TimerMode::Repeated, Duration::from_secs(60), move || check());
    timer
}

/// A manual click and an accepted update prompt share one installation gate.
pub(super) fn install(ctx: &Ctx) {
    static INSTALLING: AtomicBool = AtomicBool::new(false);
    if INSTALLING.swap(true, Ordering::SeqCst) { return; }
    let client = ctx.mgr.client();
    ctx.rt.spawn(async move {
        let result = swiftget_engine::tools::update_all(client).await;
        INSTALLING.store(false, Ordering::SeqCst);
        let _ = slint::invoke_from_event_loop(move || {
            let text = match result {
                Ok(out) => tf("Moo Downloader {}\n\nThe video downloader (yt-dlp) is up to date.\n\n{}", &[&env!("CARGO_PKG_VERSION"), &out]),
                Err(e) => tf("Could not update yt-dlp:\n\n{}", &[&format!("{e:#}")]),
            };
            message(&text);
        });
    });
}
