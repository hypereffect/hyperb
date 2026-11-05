//! Apps on their own: the addresses given on the command line open each in a
//! window of its own, with no tabs, controls or debugger, as if the app were
//! installed. The window shows the app's address, read-only, with a context
//! menu that makes or removes a desktop shortcut to it, and its title ends in
//! "(hyperb)", as do the names of shortcuts to it, so it is clear where it
//! comes from.
//!
//! Such a process runs under an application id made from the address
//! ([`app_id`]) instead of the browser's, so the system shows it as an app of
//! its own (the desktop shortcuts of [`crate::shortcut`] name the same id),
//! and launching it again brings its window back. Other hyperb apps it opens
//! start the same way, in processes of their own.

use crate::library::Library;
use crate::tab::Tab;
use gtk::prelude::*;
use gtk::{gio, glib};
use gtk4 as gtk;
use hyperb_core::address;
use hyperb_core::identity::Identity;
use std::cell::RefCell;
use std::ffi::OsString;
use std::rc::Rc;

/// The application id of the app at `address`: the browser's, followed by a
/// hash of its `hyperb://` form, so every way of writing it gets the same.
pub fn app_id(address: &str) -> String {
    let address = address::absolute_file(address);
    let address = address::normalize(&address)
        .map(|u| address::hyperb(&u))
        .unwrap_or_else(|_| address.to_string());
    let hash =
        glib::compute_checksum_for_string(glib::ChecksumType::Sha256, &address).expect("sha256");
    format!("{}.u{}", crate::APP_ID, &hash[..16])
}

/// The application id of this process: the one of the first address on the
/// command line, if there is one.
pub fn args_id() -> Option<String> {
    let mut options = true;
    for a in std::env::args().skip(1) {
        if options && a == "--" {
            options = false;
        } else if !(a.is_empty() || options && a.starts_with('-')) {
            return Some(app_id(&a));
        }
    }
    None
}

/// Opens `url` in a window of its own; `link` as for [`Tab::open_link`].
pub fn open(
    app: &gtk::Application,
    identity: Identity,
    library: Rc<Library>,
    url: &str,
    link: bool,
) {
    let t = Tab::new(identity, library, true);
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("hyperb")
        .default_width(960)
        .default_height(760)
        .child(&t.page)
        .build();
    {
        let window = window.downgrade();
        t.title.connect_label_notify(move |l| {
            if let Some(w) = window.upgrade() {
                w.set_title(Some(&crate::shortcut::app_name(&l.label())));
            }
        });
    }
    t.on_open_app(spawn);
    let reload = gio::SimpleAction::new("reload", None);
    {
        let t = Rc::downgrade(&t);
        reload.connect_activate(move |_, _| {
            if let Some(t) = t.upgrade() {
                t.reload();
            }
        });
    }
    window.add_action(&reload);
    let close = gio::SimpleAction::new("close-tab", None);
    {
        let window = window.downgrade();
        close.connect_activate(move |_, _| {
            if let Some(w) = window.upgrade() {
                w.close();
            }
        });
    }
    window.add_action(&close);

    if link {
        t.open_link(url);
    } else {
        t.navigate(url);
    }
    // The tab lives as long as its window.
    let keep = RefCell::new(Some(t));
    window.connect_destroy(move |_| {
        keep.borrow_mut().take();
    });
    window.present();
}

/// Starts another process of this executable for `url`, which then opens
/// it on its own.
fn spawn(url: &str, link: bool) {
    let exe = match std::env::current_exe() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("hyperb: cannot open {url}: {e}");
            return;
        }
    };
    let mut argv: Vec<OsString> = vec![exe.into()];
    if link {
        argv.push("--links".into());
    }
    argv.push(url.into());
    let argv: Vec<&std::ffi::OsStr> = argv.iter().map(OsString::as_os_str).collect();
    if let Err(e) = gio::Subprocess::newv(&argv, gio::SubprocessFlags::NONE) {
        eprintln!("hyperb: cannot open {url}: {e}");
    }
}
