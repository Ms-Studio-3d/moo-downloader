// Regression checks for the fork's explicit-update policy and install identity.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const root = path.resolve(__dirname, '..');
const read = name => fs.readFileSync(path.join(root, name), 'utf8');
function files(dir) {
  return fs.readdirSync(dir, {withFileTypes:true}).flatMap(e => {
    const p = path.join(dir, e.name);
    return e.isDirectory() ? files(p) : [p];
  });
}
const rust = files(path.join(root, 'crates')).filter(p => p.endsWith('.rs'));
const sources = rust.map(p => ({p, text: fs.readFileSync(p, 'utf8')}));
for (const {p,text} of sources) {
  assert.doesNotMatch(text, /spawn_updater|update_if_due|update::spawn|run_installer|UPDATE_PUBLIC_KEY/, p);
  assert.doesNotMatch(text, /AhmedEzo686\/ezoo-download/, p);
}
const main = read('crates/app/src/main.rs');
const tools = read('crates/engine/src/tools.rs');
const updater = read('crates/app/src/update.rs');
assert.match(updater, /open::that_detached\("https:\/\/github\.com\/Ms-Studio-3d\/moo-downloader\/releases"\)/);
assert.doesNotMatch(updater, /reqwest|Command::new|download\(|spawn\(/);
assert.doesNotMatch(read('crates/engine/src/selfupdate.rs'), /reqwest|Command|download|https?:\/\//);
const localProbe = tools.match(/pub async fn restore_ffmpeg\([^]*?\n\}/)?.[0];
assert.ok(localProbe, 'FFmpeg local probe exists');
assert.doesNotMatch(localProbe, /\.await|download\(|get\(|Command/);
assert.match(localProbe, /bundled\("ffmpeg"\)\.is_some\(\)/);
const manualBlock = main.slice(main.indexOf('main.on_update_tools'), main.indexOf('main.on_check_app_update'));
assert.match(manualBlock, /tools::update_all\(client\)\.await/);
assert.match(manualBlock, /tools_busy.swap/);
assert.equal(sources.filter(s => /tools::update_all\(/.test(s.text)).length, 1, 'only the explicit Help callback updates tools');
assert.equal(sources.filter(s => /ytdlp::update_now/.test(s.text)).length, 1, 'only the manual helper action updates yt-dlp');
const firefox = JSON.parse(read('extension/manifest.firefox.json'));
const chrome = JSON.parse(read('extension/manifest.json'));
assert.equal(firefox.browser_specific_settings.gecko.id, 'moo-downloader@ms-studio-3d');
assert.equal(firefox.browser_specific_settings.gecko.update_url, undefined);
for (const manifest of [firefox, chrome]) {
  assert.equal(manifest.name, 'Moo Downloader');
  assert.equal(manifest.version, '1.0.11');
  assert.equal(manifest.update_url, undefined);
  for (const script of manifest.content_scripts.flatMap(c => c.js)) {
    assert.ok(fs.existsSync(path.join(root, 'extension', script)), script);
  }
}
assert.match(read('crates/app/src/nativehost.rs'), /com\.moo\.downloader/);
assert.match(read('extension/background.js'), /com\.moo\.downloader/);
assert.doesNotMatch(read('crates/app/src/browser.rs'), /fetch_firefox_xpi|fetch_small|update_url/);
assert.equal(fs.readdirSync(path.join(root,'extension-signed')).filter(x=>x.endsWith('.xpi')).length, 0, 'no upstream signed add-on');
const installer = read('installer/ezoo.iss');
assert.match(installer, /AppId=\{\{A42D7D38-4C98-4A0A-9C2E-6B6D2F4B7A10\}/);
assert.match(installer, /Software\\Moo\\MooDownloader/);
assert.match(installer, /DefaultDirName=\{localappdata\}\\Programs\\Moo Downloader/);
for (const option of ['autostart', 'torrents', 'firewall']) {
  assert.match(installer, new RegExp('Name: "' + option + '";[^\\n]*Flags: unchecked'));
}
assert.match(read('crates/app/ui/dialogs.slint'), /Coded by Ahmed Ezzat/);
assert.match(read('LICENSE'), /Copyright \(c\) 2026 Ezoo Tech \(Ahmed Ezzat\)/);
const workflow = read('.github/workflows/build.yml');
assert.doesNotMatch(workflow, /gh release (create|delete)|UPDATE_SIGNING_KEY|IS_RELEASE/);
assert.match(workflow, /exit \$code/);
assert.match(workflow, /- name: Package\s+shell: pwsh\s+run: \|/);
assert.match(workflow, /actions\/upload-artifact/);
assert.match(workflow, /Copy-Item target\/release\/MooDownloader.exe/);
assert.match(workflow, /Copy-Item target\/release\/moo-host.exe/);
assert.match(workflow, /Compress-Archive.*MooDownloader-windows.zip/);
assert.doesNotMatch(read('crates/app/src/assoc.rs'), /ezoodownload\.exe/);
console.log('Manual-update policy, local FFmpeg checks, add-on identity, attribution and upgrade paths: passed.');
