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
    main.invoke_open_mini();
    assert_eq!(mini.get_summary(), "No active downloads");
    main.hide().unwrap();

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
    let id = mgr.add(crate::NewDownload { url, file_name: Some("sample.bin".into()), start: true, ..Default::default() });
    let deadline = Instant::now() + Duration::from_secs(12);
    let row = loop {
        advance(250);
        if let Some(row) = mini.get_rows().row_data(0).filter(|r| r.progress > 0.0) { break row; }
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

    mini.window().set_position(slint::PhysicalPosition::new(80, 100));
    advance(200);
    let position = winfx::main_position(mini.window()).unwrap();
    mini.window().dispatch_event(slint::platform::WindowEvent::CloseRequested);
    assert!(!mini.window().is_visible());
    assert_eq!(mgr.settings().mini_position, Some(position));
    main.invoke_open_mini();
    advance(400);
    assert_eq!(winfx::main_position(mini.window()), Some(position));

    mgr.pause(id);
    advance(1200);
    assert_eq!(mini.get_summary(), "لا توجد تحميلات نشطة");
    assert_eq!(mini.get_rows().row_count(), 0);
    mgr.shutdown();
    stop.store(true, Ordering::Relaxed);
    http.join().unwrap();
    let _ = std::fs::remove_dir_all(dir);
}
