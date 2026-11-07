//! Desktop shortcuts to an app, which open it on its own
//! ([`crate::standalone`]): a `.desktop` file in the applications menu and on
//! the desktop on Linux, a `.lnk` on the desktop on Windows, and a small
//! `.app` bundle on the desktop on macOS. Each starts hyperb the way this
//! process was started, with the address as its argument, and has the title,
//! description and icon of the app. The shortcut can be removed again, which
//! deletes every file it was made of.
//!
//! The library records the shortcut, so a remote wasm app with one is kept
//! in the data directory and opens from there, offline too; it offers to
//! update when the module at its address changes (see [`crate::tab`]).

use crate::library::Library;
use crate::tab::Tab;
use gtk::gdk_pixbuf::{Colorspace, InterpType, Pixbuf};
use gtk::prelude::*;
use gtk::{gdk, gio, glib};
use gtk4 as gtk;
use hyperb_core::library::Store;
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// The icon of shortcuts to apps without one.
const DEFAULT_ICON: &[u8] = include_bytes!("../../../assets/icon.png");
/// The size shortcut icons are made at.
const ICON_SIZE: i32 = 256;

struct Shortcut {
    address: String,
    title: String,
    description: String,
    /// A PNG of `ICON_SIZE` pixels square.
    icon: Vec<u8>,
}

/// Makes a shortcut to the app of tab `t` and saves its module if it is a
/// remote wasm app, then tells the user where it is, or what went wrong.
pub fn create(window: &gtk::Window, library: &Rc<Library>, t: &Tab) {
    let Some(entry) = t.entry() else { return };
    let Ok(url) = hyperb_core::address::normalize(&entry.address) else {
        return;
    };
    let texture = entry.icon.as_deref().and_then(|i| library.icon(i));
    let s = Shortcut {
        description: t.description().unwrap_or_else(|| entry.address.clone()),
        icon: icon(texture.as_ref()),
        address: entry.address,
        title: app_name(&entry.title),
    };
    let (window, library) = (window.clone(), library.clone());
    glib::spawn_future_local(async move {
        let dialog = match gio::spawn_blocking(move || install(&s)).await {
            Ok(Ok(path)) => {
                library.set_shortcut(&url, true);
                let mut detail = path.display().to_string();
                if library.with(|l| l.keeps_module(&url))
                    && let Err(e) = library.keep(&url).await
                {
                    detail.push_str(&format!(
                        "\n\nThe app could not be saved for offline use ({e}); it will be the next time it opens."
                    ));
                }
                gtk::AlertDialog::builder()
                    .message("Shortcut created")
                    .detail(detail)
                    .build()
            }
            Ok(Err(e)) => gtk::AlertDialog::builder()
                .message("Could not create the shortcut")
                .detail(e.to_string())
                .build(),
            Err(_) => return,
        };
        dialog.show(Some(&window));
    });
}

/// Whether there is a desktop shortcut to the app of tab `t`.
pub fn installed(t: &Tab) -> bool {
    let Some(entry) = t.entry() else { return false };
    shortcut_files(&entry.address, &entry.title)
        .iter()
        .any(|p| is_shortcut(p))
}

/// Removes the desktop shortcut to the app of tab `t`, and its saved module
/// unless a favorite keeps it, then tells the user what was removed, or what
/// went wrong.
pub fn remove(window: &gtk::Window, library: &Rc<Library>, t: &Tab) {
    let Some(entry) = t.entry() else { return };
    let (address, title) = (entry.address, entry.title);
    if let Ok(url) = hyperb_core::address::normalize(&address) {
        library.set_shortcut(&url, false);
    }
    let window = window.clone();
    glib::spawn_future_local(async move {
        let dialog = match gio::spawn_blocking(move || uninstall(&address, &title)).await {
            Ok(Ok(paths)) if paths.is_empty() => gtk::AlertDialog::builder()
                .message("No desktop shortcut to remove")
                .build(),
            Ok(Ok(paths)) => gtk::AlertDialog::builder()
                .message("Shortcut removed")
                .detail(
                    paths
                        .iter()
                        .map(|p| p.display().to_string())
                        .collect::<Vec<_>>()
                        .join("\n"),
                )
                .build(),
            Ok(Err(e)) => gtk::AlertDialog::builder()
                .message("Could not remove the shortcut")
                .detail(e.to_string())
                .build(),
            Err(_) => return,
        };
        dialog.show(Some(&window));
    });
}

/// The name an app goes by on its own: its title, marked as a hyperb app.
pub fn app_name(title: &str) -> String {
    format!("{} (hyperb)", title.trim())
}

/// `texture`, or hyperb's icon, as a PNG of `ICON_SIZE` pixels square.
fn icon(texture: Option<&gdk::Texture>) -> Vec<u8> {
    let bytes = match texture {
        Some(t) => t.save_to_png_bytes(),
        None => glib::Bytes::from_static(DEFAULT_ICON),
    };
    let scaled = || -> Option<Vec<u8>> {
        let stream = gio::MemoryInputStream::from_bytes(&bytes);
        let p = Pixbuf::from_stream(&stream, gio::Cancellable::NONE).ok()?;
        let scale = ICON_SIZE as f64 / p.width().max(p.height()) as f64;
        let (w, h) = (
            ((p.width() as f64 * scale).round() as i32).max(1),
            ((p.height() as f64 * scale).round() as i32).max(1),
        );
        let (x, y) = ((ICON_SIZE - w) / 2, (ICON_SIZE - h) / 2);
        let out = Pixbuf::new(Colorspace::Rgb, true, 8, ICON_SIZE, ICON_SIZE)?;
        out.fill(0);
        p.scale(
            &out,
            x,
            y,
            w,
            h,
            x as f64,
            y as f64,
            scale,
            scale,
            InterpType::Bilinear,
        );
        out.save_to_bufferv("png", &[]).ok()
    };
    scaled().unwrap_or_else(|| bytes.to_vec())
}

/// The command that starts hyperb the way this process was started: the
/// AppImage or Flatpak it runs from, or else its executable.
fn launcher() -> io::Result<Vec<OsString>> {
    #[cfg(target_os = "linux")]
    {
        if let Some(a) = std::env::var_os("APPIMAGE") {
            return Ok(vec![a]);
        }
        if let Some(id) = std::env::var_os("FLATPAK_ID") {
            return Ok(vec!["flatpak".into(), "run".into(), id]);
        }
    }
    let exe = std::env::current_exe()?;
    // An executable replaced since it started is still where it was.
    let exe = match exe.to_str().and_then(|e| e.strip_suffix(" (deleted)")) {
        Some(e) => PathBuf::from(e),
        None => exe,
    };
    Ok(vec![exe.into()])
}

/// Where the icons of shortcuts are kept.
#[cfg_attr(target_os = "macos", allow(dead_code))]
fn icons_dir() -> io::Result<PathBuf> {
    let dir = Store::default_dir()
        .ok_or_else(|| io::Error::other("no data directory"))?
        .join("shortcuts");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn desktop_dir() -> io::Result<PathBuf> {
    glib::user_special_dir(glib::UserDirectory::Desktop)
        .filter(|d| d.is_dir() && *d != glib::home_dir())
        .ok_or_else(|| io::Error::other("there is no desktop folder"))
}

/// Where the applications menu keeps its `.desktop` files.
#[cfg(all(unix, not(target_os = "macos")))]
fn applications_dir() -> io::Result<PathBuf> {
    let data = Store::default_dir().ok_or_else(|| io::Error::other("no data directory"))?;
    let share = data
        .parent()
        .ok_or_else(|| io::Error::other("no data directory"))?;
    Ok(share.join("applications"))
}

/// `title` as a file name, without the characters some systems refuse.
#[cfg_attr(all(unix, not(target_os = "macos")), allow(dead_code))]
fn file_name(title: &str) -> String {
    let name: String = title
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => ' ',
            c if c.is_control() => ' ',
            c => c,
        })
        .take(80)
        .collect();
    let name = name.trim().trim_end_matches('.');
    if name.is_empty() {
        "hyperb app".into()
    } else {
        name.into()
    }
}

/// The files a shortcut to the app at `address` is made of.
#[cfg(all(unix, not(target_os = "macos")))]
fn shortcut_files(address: &str, _title: &str) -> Vec<PathBuf> {
    let name = format!("{}.desktop", crate::standalone::app_id(address));
    let mut files = Vec::new();
    if let Ok(dir) = applications_dir() {
        files.push(dir.join(&name));
    }
    if let Ok(dir) = desktop_dir() {
        files.push(dir.join(&name));
    }
    files
}

/// The files a shortcut to the app at `address` is made of.
#[cfg(windows)]
fn shortcut_files(_address: &str, title: &str) -> Vec<PathBuf> {
    desktop_dir()
        .map(|d| vec![d.join(format!("{}.lnk", file_name(title)))])
        .unwrap_or_default()
}

/// The files a shortcut to the app at `address` is made of.
#[cfg(target_os = "macos")]
fn shortcut_files(_address: &str, title: &str) -> Vec<PathBuf> {
    desktop_dir()
        .map(|d| vec![d.join(format!("{}.app", file_name(title)))])
        .unwrap_or_default()
}

/// The icon file a shortcut to the app at `address` uses, if it has one.
#[cfg(all(unix, not(target_os = "macos")))]
fn shortcut_icon(address: &str, _title: &str) -> Option<PathBuf> {
    icons_dir()
        .ok()
        .map(|d| d.join(format!("{}.png", crate::standalone::app_id(address))))
}

/// The icon file a shortcut to the app at `address` uses, if it has one.
#[cfg(windows)]
fn shortcut_icon(address: &str, _title: &str) -> Option<PathBuf> {
    icons_dir()
        .ok()
        .map(|d| d.join(format!("{}.ico", crate::standalone::app_id(address))))
}

/// The icon file a shortcut to the app at `address` uses, if it has one.
#[cfg(target_os = "macos")]
fn shortcut_icon(_address: &str, _title: &str) -> Option<PathBuf> {
    None
}

#[cfg(not(target_os = "macos"))]
fn is_shortcut(path: &Path) -> bool {
    path.is_file()
}

#[cfg(target_os = "macos")]
fn is_shortcut(path: &Path) -> bool {
    path.join("Contents/MacOS/launch").is_file()
}

#[cfg(not(target_os = "macos"))]
fn remove_shortcut(path: &Path) -> io::Result<bool> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e),
    }
}

#[cfg(target_os = "macos")]
fn remove_shortcut(path: &Path) -> io::Result<bool> {
    if !is_shortcut(path) {
        return Ok(false);
    }
    std::fs::remove_dir_all(path)?;
    Ok(true)
}

/// Removes the files of the shortcut to the app at `address`, and returns the
/// ones that were there.
fn uninstall(address: &str, title: &str) -> io::Result<Vec<PathBuf>> {
    let mut removed = Vec::new();
    let mut failure = None;
    for path in shortcut_files(address, title) {
        match remove_shortcut(&path) {
            Ok(true) => removed.push(path),
            Ok(false) => {}
            Err(e) => {
                failure.get_or_insert(e);
            }
        }
    }
    if let Some(icon) = shortcut_icon(address, title) {
        match std::fs::remove_file(&icon) {
            Ok(()) => removed.push(icon),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => {
                failure.get_or_insert(e);
            }
        }
    }
    match failure {
        Some(e) => Err(e),
        None => Ok(removed),
    }
}

/// Writes a `.desktop` file to the applications menu, named after the app's
/// application id so the system matches its windows to it, and a copy on the
/// desktop if there is one. Returns where it went.
#[cfg(all(unix, not(target_os = "macos")))]
fn install(s: &Shortcut) -> io::Result<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let id = crate::standalone::app_id(&s.address);
    let icon = icons_dir()?.join(format!("{id}.png"));
    std::fs::write(&icon, &s.icon)?;
    let mut exec = launcher()?;
    exec.push(s.address.clone().into());
    let exec: Vec<String> = exec
        .iter()
        .map(|a| exec_arg(&a.to_string_lossy()))
        .collect();
    let file = format!(
        "[Desktop Entry]\nType=Application\nName={}\nComment={}\nExec={}\nIcon={}\nTerminal=false\nCategories=Network;\nStartupNotify=true\nStartupWMClass={id}\n",
        value(&s.title),
        value(&s.description),
        value(&exec.join(" ")),
        value(&icon.to_string_lossy()),
    );
    let apps = applications_dir()?;
    std::fs::create_dir_all(&apps)?;
    let name = format!("{id}.desktop");
    std::fs::write(apps.join(&name), &file)?;
    let Ok(desktop) = desktop_dir() else {
        return Ok(apps.join(name));
    };
    let path = desktop.join(name);
    std::fs::write(&path, &file)?;
    // Desktops only launch files that are executable, and GNOME's only
    // ones it trusts.
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
    let _ = gio::File::for_path(&path).set_attribute_string(
        "metadata::trusted",
        "true",
        gio::FileQueryInfoFlags::NONE,
        gio::Cancellable::NONE,
    );
    Ok(path)
}

/// A string value of a desktop entry.
#[cfg(all(unix, not(target_os = "macos")))]
fn value(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// One argument of the `Exec` key of a desktop entry, quoted. [`value`]
/// escapes it again.
#[cfg(all(unix, not(target_os = "macos")))]
fn exec_arg(a: &str) -> String {
    let mut q = String::from("\"");
    for c in a.chars() {
        match c {
            '"' | '`' | '$' | '\\' => {
                q.push('\\');
                q.push(c);
            }
            '%' => q.push_str("%%"),
            c => q.push(c),
        }
    }
    q.push('"');
    q
}

/// Makes a `.lnk` on the desktop through PowerShell. Returns where it went.
#[cfg(windows)]
fn install(s: &Shortcut) -> io::Result<PathBuf> {
    use base64::Engine;
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let id = crate::standalone::app_id(&s.address);
    let icon = icons_dir()?.join(format!("{id}.ico"));
    std::fs::write(&icon, ico(&s.icon))?;
    let path = desktop_dir()?.join(format!("{}.lnk", file_name(&s.title)));
    let mut cmd = launcher()?;
    let target = cmd.remove(0);
    cmd.push(s.address.clone().into());
    let args: Vec<String> = cmd
        .iter()
        .map(|a| format!("\"{}\"", a.to_string_lossy().replace('"', "\\\"")))
        .collect();
    // Windows cuts descriptions longer than this.
    let description: String = s.description.chars().take(250).collect();
    let ps = |s: &str| format!("'{}'", s.replace('\'', "''"));
    let script = format!(
        "$s = (New-Object -ComObject WScript.Shell).CreateShortcut({}); $s.TargetPath = {}; $s.Arguments = {}; $s.Description = {}; $s.IconLocation = {}; $s.Save()",
        ps(&path.to_string_lossy()),
        ps(&target.to_string_lossy()),
        ps(&args.join(" ")),
        ps(&description),
        ps(&format!("{},0", icon.display())),
    );
    let utf16: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-EncodedCommand"])
        .arg(base64::engine::general_purpose::STANDARD.encode(utf16))
        .creation_flags(CREATE_NO_WINDOW)
        .output()?;
    if !out.status.success() {
        return Err(io::Error::other(
            String::from_utf8_lossy(&out.stderr).trim().to_string(),
        ));
    }
    Ok(path)
}

/// An `.ico` holding `png`.
#[cfg(windows)]
fn ico(png: &[u8]) -> Vec<u8> {
    let mut f = vec![0, 0, 1, 0, 1, 0];
    // 0 stands for 256 pixels.
    let side = if ICON_SIZE >= 256 { 0 } else { ICON_SIZE as u8 };
    f.extend([side, side, 0, 0, 1, 0, 32, 0]);
    f.extend((png.len() as u32).to_le_bytes());
    f.extend(22u32.to_le_bytes());
    f.extend(png);
    f
}

/// Makes an `.app` bundle on the desktop whose executable starts hyperb.
/// Returns where it went.
#[cfg(target_os = "macos")]
fn install(s: &Shortcut) -> io::Result<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let id = crate::standalone::app_id(&s.address);
    let app = desktop_dir()?.join(format!("{}.app", file_name(&s.title)));
    let contents = app.join("Contents");
    let exe = contents.join("MacOS/launch");
    // Never write into a bundle that is not a shortcut.
    if app.exists() && !exe.exists() {
        return Err(io::Error::other(format!(
            "{} already exists",
            app.display()
        )));
    }
    std::fs::create_dir_all(contents.join("MacOS"))?;
    std::fs::create_dir_all(contents.join("Resources"))?;
    let mut cmd = launcher()?;
    cmd.push(s.address.clone().into());
    let cmd: Vec<String> = cmd
        .iter()
        .map(|a| format!("'{}'", a.to_string_lossy().replace('\'', "'\\''")))
        .collect();
    std::fs::write(&exe, format!("#!/bin/sh\nexec {}\n", cmd.join(" ")))?;
    std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755))?;
    std::fs::write(contents.join("Resources/icon.icns"), icns(&s.icon))?;
    let x = |s: &str| {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    };
    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key><string>launch</string>
  <key>CFBundleIdentifier</key><string>{id}</string>
  <key>CFBundleName</key><string>{}</string>
  <key>CFBundleGetInfoString</key><string>{}</string>
  <key>CFBundleIconFile</key><string>icon</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
</dict>
</plist>
"#,
        x(&s.title),
        x(&s.description),
    );
    std::fs::write(contents.join("Info.plist"), plist)?;
    Ok(app)
}

/// An `.icns` holding `png`, which must be 256 pixels square.
#[cfg(target_os = "macos")]
fn icns(png: &[u8]) -> Vec<u8> {
    let mut f = b"icns".to_vec();
    f.extend((16 + png.len() as u32).to_be_bytes());
    f.extend(b"ic08");
    f.extend((8 + png.len() as u32).to_be_bytes());
    f.extend(png);
    f
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_names() {
        assert_eq!(file_name("Bill: split / share?"), "Bill  split   share");
        assert_eq!(file_name(" ... "), "hyperb app");
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn exec_args() {
        assert_eq!(
            value(&exec_arg(r#"a "b" $c\d 100%"#)),
            r#""a \\"b\\" \\$c\\\\d 100%%""#
        );
    }
}
