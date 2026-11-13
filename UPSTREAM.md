# Moo Downloader local adaptation

Based on the user-supplied Ezoo Download 1.0.11 source by Ahmed Ezzat.
The previous Moo Downloader 1.0.0 source was checked at commit 7a0286a.

Original author attribution, support links and the supplied LICENSE are retained.
The supplied LICENSE is restrictive, although the incoming Cargo metadata said MIT.
This adaptation reflects the supplied LICENSE in Cargo metadata and does not claim
that the new code is MIT licensed. Permission to redistribute the new source has
not been established in this review.

Moo-specific changes:
- Restored Moo icons, product name, installer identity and existing data paths.
- No background app or helper updates, and no executable auto-repair downloads.
- Help opens Moo Releases; the user downloads and runs an installer themselves.
- yt-dlp updates only from the Help menu, one operation at a time, with errors shown.
- FFmpeg is updated by manually installing a new app release.
- Separate Moo native-messaging identity. Original request-header names are retained
  as protocol identifiers, not as product attribution.
- No upstream signed Firefox add-on or automatic add-on update URL is included.
  Chromium uses the bundled unpacked add-on. Firefox supports a separately signed
  Moo add-on, or a temporary local add-on until one is available.
- Startup, default torrent associations and the optional inbound firewall rule are
  unchecked by default for new installations.
- GitHub Actions builds artifacts only, retains test failures, and does not publish.

Verification: see REVIEW.md. The original source directory was not modified.
