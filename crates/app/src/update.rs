//! Application updates are downloaded and installed by the user.
//! No background check, download, installer execution, or upstream signing key.

use crate::{message, Ui};

pub fn check_now(_ui: &Ui) {
    if let Err(error) = open::that_detached("https://github.com/Ms-Studio-3d/moo-downloader/releases") {
        message(&format!("Could not open the download page: {error}"));
    }
}
