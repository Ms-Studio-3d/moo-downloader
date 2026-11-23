//! Queue membership must not depend on which UI added the download.
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use swiftget_engine::config::MAIN_QUEUE;
use swiftget_engine::{DownloadKind, Manager, NewDownload, ProbeResult, Status};
use tokio::net::TcpListener;

fn root(name: &str) -> std::path::PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!("moo-scheduler-{name}-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
    std::fs::create_dir_all(&path).unwrap();
    path
}

#[tokio::test]
async fn new_downloads_are_visible_without_starting_or_setting_a_schedule() {
    let dir = root("membership");
    let mgr = Manager::new(tokio::runtime::Handle::current(), dir.join("app")).unwrap();
    let schedule = mgr.settings().queue(MAIN_QUEUE).unwrap().schedule.clone();
    let first = mgr.add(NewDownload {
        url: "http://127.0.0.1:9/file.zip".into(),
        file_name: Some("file.zip".into()),
        save_dir: Some(dir.clone()),
        ..Default::default()
    });
    assert_eq!(mgr.get(first).unwrap().queue, Some(MAIN_QUEUE));
    assert_eq!(mgr.get(first).unwrap().status, Status::Paused);
    assert_eq!(mgr.queue_files(MAIN_QUEUE).len(), 1);
    assert_eq!(mgr.settings().queue(MAIN_QUEUE).unwrap().schedule, schedule);

    let custom = mgr.add_queue("Course", 2, None).unwrap();
    let second = mgr.add(NewDownload {
        url: "http://127.0.0.1:9/lesson.mp4".into(),
        file_name: Some("lesson.mp4".into()),
        save_dir: Some(dir),
        queue: Some(custom),
        ..Default::default()
    });
    assert_eq!(mgr.get(second).unwrap().queue, Some(custom));
    assert_eq!(mgr.queue_files(custom).len(), 1);
    mgr.shutdown();
}

#[tokio::test]
async fn existing_unassigned_downloads_are_restored_to_the_scheduler_and_saved() {
    let dir = root("legacy");
    let app = dir.join("app");
    let mgr = Manager::new(tokio::runtime::Handle::current(), app.clone()).unwrap();
    let id = mgr.add(NewDownload {
        url: "http://127.0.0.1:9/old.zip".into(),
        file_name: Some("old.zip".into()),
        save_dir: Some(dir),
        ..Default::default()
    });
    // Reproduce a downloads.json written by the older version.
    mgr.set_queue(id, None);
    mgr.shutdown();
    let restored = Manager::new(tokio::runtime::Handle::current(), app.clone()).unwrap();
    assert_eq!(restored.get(id).unwrap().queue, Some(MAIN_QUEUE));
    assert_eq!(restored.get(id).unwrap().status, Status::Paused);
    restored.shutdown();
    let records = swiftget_engine::store::load_downloads(&app.join("downloads.json"));
    assert_eq!(records[0].queue, Some(MAIN_QUEUE));
    assert_eq!(records[0].file_name, "old.zip");
}

#[tokio::test]
async fn immediate_downloads_stay_parallel_and_the_scheduler_can_stop_them() {
    let dir = root("immediate");
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    // Keep connections open without responding, so the transfers stay live.
    let connections = Arc::new(AtomicU64::new(0));
    let seen = connections.clone();
    let server = tokio::spawn(async move {
        let mut sockets = Vec::new();
        loop {
            let (sock, _) = listener.accept().await.unwrap();
            sockets.push(sock);
            seen.fetch_add(1, Ordering::SeqCst);
        }
    });
    let mgr = Manager::new(tokio::runtime::Handle::current(), dir.join("app")).unwrap();
    let mut settings = mgr.settings();
    settings.proxy_mode = 0;
    settings.max_active = 0;
    settings.queues.iter_mut().find(|q| q.id == MAIN_QUEUE).unwrap().max_parallel = 1;
    mgr.set_settings(settings).unwrap();
    let mut ids = Vec::new();
    for i in 0..2 {
        let url = format!("http://{addr}/{i}.bin");
        ids.push(mgr.add(NewDownload {
            url: url.clone(),
            file_name: Some(format!("{i}.bin")),
            save_dir: Some(dir.clone()),
            connections: Some(1),
            probe: Some(ProbeResult {
                final_url: url,
                file_name: format!("{i}.bin"),
                total_size: Some(1024),
                resumable: false,
                content_type: Some("application/octet-stream".into()),
                kind: DownloadKind::Http,
                last_modified: None,
                etag: None,
                headers_used: None,
                digest: None,
            }),
            start: true,
            ..Default::default()
        }));
    }
    tokio::time::timeout(Duration::from_secs(5), async {
        while connections.load(Ordering::SeqCst) < 2 {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }).await.expect("both immediate downloads must connect despite the main queue limit of one");
    assert_eq!(mgr.queue_files(MAIN_QUEUE).len(), 2);
    for &id in &ids {
        assert!(mgr.get(id).unwrap().status.is_running());
    }
    mgr.stop_queue(MAIN_QUEUE);
    for &id in &ids {
        assert_eq!(mgr.get(id).unwrap().status, Status::Paused);
    }
    tokio::time::timeout(Duration::from_secs(5), async {
        while mgr.queue_running(MAIN_QUEUE) {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }).await.unwrap();
    mgr.start_queue(MAIN_QUEUE);
    // Explicitly starting the queue restores its configured serial limit.
    assert!(mgr.get(ids[0]).unwrap().status.is_running());
    assert_eq!(mgr.get(ids[1]).unwrap().status, Status::Queued);
    mgr.shutdown();
    server.abort();
}
