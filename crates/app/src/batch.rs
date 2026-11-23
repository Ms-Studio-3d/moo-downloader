//! Batch download dialog and wildcard expansion.

use super::*;

pub(super) fn wire_batch(ui: &Ui) {
    let Some(batch) = ui.batch.upgrade() else { return };
    {
        let w = batch.as_weak();
        batch.on_paste(move || {
            let Some(d) = w.upgrade() else { return };
            if let Some(text) = arboard::Clipboard::new().ok().and_then(|mut c| c.get_text().ok()) {
                let mut links = d.get_links().to_string();
                if !links.is_empty() && !links.ends_with('\n') {
                    links.push('\n');
                }
                links.push_str(text.trim());
                d.set_links(links.into());
            }
        });
    }
    {
        let (w, mgr) = (batch.as_weak(), ui.ctx.mgr.clone());
        batch.on_browse(move || {
            let Some(d) = w.upgrade() else { return };
            if let Some(dir) = crate::winfx::file_box().set_directory(mgr.settings().download_dir).pick_folder() {
                d.set_folder(dir.display().to_string().into());
            }
        });
    }
    {
        let w = batch.as_weak();
        batch.on_pattern_changed(move || {
            let Some(d) = w.upgrade() else { return };
            d.set_preview(batch_preview(&d).into());
        });
    }
    {
        let ui = ui.clone();
        batch.on_ok(move || {
            let Some(d) = ui.batch.upgrade() else { return };
            let (links, auth) = if d.get_mode() == 1 {
                match pattern_links(&d) {
                    Ok(l) => {
                        let user = d.get_user().trim().to_string();
                        (l, (!user.is_empty()).then(|| (user, d.get_password().to_string())))
                    }
                    Err(e) => {
                        d.set_message(e.into());
                        return;
                    }
                }
            } else {
                (extract_links(&d.get_links()), None)
            };
            if links.is_empty() {
                d.set_message(t("No valid http:// or https:// links").into());
                return;
            }
            let s = ui.ctx.mgr.settings();
            let queue = s.queues.get(d.get_queue_index().max(0) as usize).map(|q| q.id).unwrap_or(MAIN_QUEUE);
            let folder = d.get_folder().trim().to_string();
            let group = Some(ui.ctx.mgr.new_group());
            for url in links {
                ui.ctx.mgr.add(NewDownload {
                    url,
                    save_dir: (!folder.is_empty()).then(|| PathBuf::from(&folder)),
                    headers: RequestHeaders { auth: auth.clone(), ..Default::default() },
                    queue: Some(queue),
                    group,
                    start: false,
                    ..Default::default()
                });
            }
            if d.get_start_now() {
                ui.ctx.mgr.start_queue(queue);
            }
            if let Some(m) = ui.ctx.main.upgrade() {
                m.set_view(format!("queue:{queue}").into());
            }
            let _ = d.hide();
            refresh(&ui);
        });
    }
    {
        let w = batch.as_weak();
        batch.on_cancel(move || {
            if let Some(d) = w.upgrade() {
                let _ = d.hide();
            }
        });
    }
}

/// "img*.jpg" + from/to -> the list of addresses (numbers or letters).
pub(super) fn expand_pattern(pattern: &str, letters: bool, from: &str, to: &str, size: usize) -> Result<Vec<String>, String> {
    let pattern = pattern.trim();
    if !(pattern.starts_with("http://") || pattern.starts_with("https://")) {
        return Err(t("The address must start with http:// or https://"));
    }
    if !pattern.contains('*') {
        return Err(t("Put a * in the address where the number goes"));
    }
    let mut out = Vec::new();
    if letters {
        let a = from.trim().chars().next().filter(|c| c.is_ascii_alphabetic()).ok_or_else(|| t("From must be a letter"))?;
        let b = to.trim().chars().next().filter(|c| c.is_ascii_alphabetic()).ok_or_else(|| t("To must be a letter"))?;
        let (a, b) = (a as u8, b as u8);
        if a > b {
            return Err(t("From must come before To"));
        }
        for c in a..=b {
            out.push(pattern.replace('*', &(c as char).to_string()));
        }
    } else {
        let a: u64 = from.trim().parse().map_err(|_| t("From must be a number"))?;
        let b: u64 = to.trim().parse().map_err(|_| t("To must be a number"))?;
        if a > b {
            return Err(t("From must be smaller than To"));
        }
        if b - a >= 10_000 {
            return Err(t("That's more than 10000 files"));
        }
        for n in a..=b {
            out.push(pattern.replace('*', &format!("{n:0size$}")));
        }
    }
    Ok(out)
}

fn pattern_links(d: &BatchDialog) -> Result<Vec<String>, String> {
    expand_pattern(
        &d.get_pattern(),
        d.get_letters(),
        &d.get_from_text(),
        &d.get_to_text(),
        d.get_wildcard_size().clamp(1, 10) as usize,
    )
}

fn batch_preview(d: &BatchDialog) -> String {
    match pattern_links(d) {
        Ok(l) if l.is_empty() => String::new(),
        Ok(l) => {
            let mut s = tf("{} files\n\nFirst:  {}", &[&l.len(), &l[0]]);
            if l.len() > 1 {
                s.push_str(&tf("\nSecond:  {}", &[&l[1]]));
            }
            if l.len() > 2 {
                s.push_str(&tf("\nLast:  {}", &[&l[l.len() - 1]]));
            }
            s
        }
        Err(e) => e,
    }
}
