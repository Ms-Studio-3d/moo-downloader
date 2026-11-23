//! Run explicitly on a Windows desktop. The transfer and its files are local.
use super::*;
use slint::{ComponentHandle, Model};
use std::{io::{Read, Result as IoResult}, sync::atomic::{AtomicBool, Ordering}, time::Instant};

struct SlowFile(usize);

impl Read for SlowFile {
    fn read(&mut self, out: &mut [u8]) -> IoResult<usize> {
        let count = self.0.min(out.len()).min(8192);
        if count > 0 {
            std::thread::sleep(Duration::from_millis(10));
            out[..count].fill(7);
            self.0 -= count;
        }
        Ok(count)
    }
}

fn advance(ms: u64) {
    Timer::single_shot(Duration::from_millis(ms), || { let _ = slint::quit_event_loop(); });
    slint::run_event_loop_until_quit().unwrap();
}

#[test]
#[ignore = "opens native Windows dialogs"]
fn confirmation_is_centered_over_main_window() {
    use windows_sys::Win32::Foundation::{HWND, RECT};
    use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect;
    slint::BackendSelector::new().backend_name("winit".into()).renderer_name("software".into()).select().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let dir = std::env::temp_dir().join(format!("moo-confirm-test-{}", std::process::id()));
    let mgr = Manager::new(rt.handle().clone(), dir.clone()).unwrap();
    let main = crate::MainWindow::new().unwrap();
    let add = crate::AddDialog::new().unwrap();
    let ctx = Ctx { mgr: mgr.clone(), rt: rt.handle().clone(), main: main.as_weak(), add: add.as_weak(), add_state: Default::default() };
    main.show().unwrap();
    main.window().set_position(slint::PhysicalPosition::new(250, 180));
    advance(200);
    crate::confirm(&ctx, "Moo Downloader", "Remove finished downloads?", || {});
    advance(400);
    let dialog = crate::CONFIRMS.with(|dialogs| dialogs.borrow().last().unwrap().as_weak()).upgrade().unwrap();
    let rect = |window: &slint::Window| {
        let handle = winfx::hwnd(window).unwrap() as HWND;
        let mut area: RECT = unsafe { std::mem::zeroed() };
        assert_ne!(unsafe { GetWindowRect(handle, &mut area) }, 0);
        area
    };
    let parent = rect(main.window());
    let child = rect(dialog.window());
    assert!(((parent.left + parent.right) - (child.left + child.right)).abs() <= 4);
    assert!(((parent.top + parent.bottom) - (child.top + child.bottom)).abs() <= 4);
    let _ = dialog.hide();
    let _ = main.hide();
    mgr.shutdown();
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
#[ignore = "opens the native Windows monitor"]
fn monitor_tracks_transfer_while_main_is_hidden() {
    slint::BackendSelector::new().backend_name("winit".into()).renderer_name("software".into()).select().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let dir = std::env::temp_dir().join(format!("moo-mini-test-{}", std::process::id()));
    let mgr = Manager::new(rt.handle().clone(), dir.clone()).unwrap();
    let mut settings = mgr.settings();
    settings.connections = 1;
    settings.download_dir = dir.join("downloads");
    mgr.set_settings(settings).unwrap();
    let main = crate::MainWindow::new().unwrap();
    let add = crate::AddDialog::new().unwrap();
    let mini = MiniMonitor::new().unwrap();
    let ctx = Ctx { mgr: mgr.clone(), rt: rt.handle().clone(), main: main.as_weak(), add: add.as_weak(), add_state: Default::default() };
    let _timer = wire(&ctx, &mini);
    main.show().unwrap();
    crate::i18n::apply("en");
    main.hide().unwrap();
    advance(1200);
    assert!(!mini.window().is_visible());

    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}/sample.bin", server.server_addr());
    let stop = Arc::new(AtomicBool::new(false));
    let running = stop.clone();
    let http = std::thread::spawn(move || {
        const SIZE: usize = 16 * 1024 * 1024;
        while !running.load(Ordering::Relaxed) {
            let Ok(Some(request)) = server.recv_timeout(Duration::from_millis(100)) else { continue };
            let kind = tiny_http::Header::from_bytes("Content-Type", "application/octet-stream").unwrap();
            let response = tiny_http::Response::new(tiny_http::StatusCode(200), vec![kind], SlowFile(SIZE), Some(SIZE), None);
            let _ = request.respond(response.with_chunked_threshold(usize::MAX));
        }
    });
    let id = mgr.add(crate::NewDownload { url: url.clone(), file_name: Some("sample.bin".into()), start: true, ..Default::default() });
    let deadline = Instant::now() + Duration::from_secs(12);
    let row = loop {
        advance(250);
        if mini.window().is_visible() {
            if let Some(row) = mini.get_rows().row_data(0).filter(|r| r.progress > 0.0) { break row; }
        }
        assert!(Instant::now() < deadline, "no progress for {:?}", mgr.get(id));
    };
    assert_eq!(row.name, "sample.bin");
    assert!(row.detail.contains("/s"));
    assert_eq!(mini.get_summary(), "1 active download");
    crate::i18n::apply("ar");
    advance(1200);
    assert!(mini.global::<Theme>().get_rtl());
    assert_eq!(mini.get_summary(), "تحميل واحد نشط");
    assert!(mini.get_rows().row_data(0).unwrap().progress > row.progress);
    assert!(mini.window().take_snapshot().is_ok());
    mini.invoke_open_main();
    assert!(main.window().is_visible());
    advance(1200);
    assert!(!mini.window().is_visible());
    main.hide().unwrap();
    advance(1200);
    assert!(mini.window().is_visible());

    mini.window().set_position(slint::PhysicalPosition::new(80, 100));
    advance(200);
    let position = winfx::main_position(mini.window()).unwrap();
    mini.window().dispatch_event(slint::platform::WindowEvent::CloseRequested);
    assert!(!mini.window().is_visible());
    assert_eq!(mgr.settings().mini_position, Some(position));
    advance(1200);
    assert!(!mini.window().is_visible(), "closing must not stop or immediately reopen the monitor");
    assert!(mgr.get(id).unwrap().status.is_running());

    mgr.pause(id);
    advance(1200);
    assert!(!mini.window().is_visible());
    let next = mgr.add(crate::NewDownload { url, file_name: Some("next.bin".into()), start: true, ..Default::default() });
    advance(1800);
    assert!(mini.window().is_visible(), "the next active download should reopen the monitor");
    assert_eq!(winfx::main_position(mini.window()), Some(position));
    mgr.pause(next);
    advance(1200);
    assert!(!mini.window().is_visible());
    mgr.shutdown();
    stop.store(true, Ordering::Relaxed);
    http.join().unwrap();
    let _ = std::fs::remove_dir_all(dir);
}
