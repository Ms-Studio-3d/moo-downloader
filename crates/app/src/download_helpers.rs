//! Address handling and file-type presentation helpers.

use swiftget_engine::filename;

pub(super) fn is_http_url(t: &str) -> Option<String> {
    let t = t.trim();
    is_download_address(t).then(|| t.to_string()).filter(|s| !s.contains(char::is_whitespace))
}

/// An address the program downloads from: http(s), or a file on an FTP server.
pub(crate) fn is_download_address(t: &str) -> bool {
    let l = t.trim_start().to_ascii_lowercase();
    ["http://", "https://", "ftp://", "ftps://"].iter().any(|p| l.starts_with(p))
}

/// Every http(s) link in a block of text, one per line or separated by spaces.
pub(crate) fn extract_links(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|l| l.trim_matches(|c| c == '"' || c == '\'' || c == '<' || c == '>' || c == ','))
        .filter(|l| is_download_address(l) && url::Url::parse(l).is_ok())
        .map(|l| l.to_string())
        .collect()
}

/// File-type icon for a file name.
pub(crate) fn kind_for(name: &str) -> &'static str {
    match filename::extension(name).as_str() {
        "mp4" | "mkv" | "avi" | "mov" | "wmv" | "flv" | "webm" | "m4v" | "ts" | "3gp" | "mpg" | "mpeg" | "vob"
        | "rm" | "rmvb" | "ogv" | "asf" => "video",
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "wma" | "opus" | "mid" | "mka" => "audio",
        "zip" | "rar" | "7z" | "tar" | "gz" | "tgz" | "bz2" | "xz" | "zst" | "cab" | "arj" | "lzh" => "archive",
        "iso" | "img" | "dmg" => "disc",
        "pdf" => "pdf",
        "doc" | "docx" | "odt" | "rtf" | "epub" => "doc",
        "xls" | "xlsx" | "csv" | "ods" => "sheet",
        "ppt" | "pptx" | "odp" | "pps" => "slides",
        "txt" | "md" | "log" | "json" | "xml" => "text",
        "exe" | "msi" | "msix" | "appx" | "bat" | "jar" | "deb" | "rpm" => "exe",
        "apk" | "xapk" => "apk",
        "torrent" => "torrent",
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "svg" | "tif" | "tiff" | "heic" | "ico" | "psd" => "image",
        _ => "general",
    }
}

/// Sidebar icon of a category.
pub(super) fn kind_of_category(name: &str) -> &'static str {
    match name {
        "Compressed" => "archive",
        "Documents" => "doc",
        "Music" => "audio",
        "Programs" => "exe",
        "Video" => "video",
        "Images" => "image",
        "General" => "general",
        _ => "folder",
    }
}
