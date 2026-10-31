//! One browser tab: a connection to a hypereffect server, and the rendered
//! session.
//!
//! A new tab asks for an address, and keeps the one it opens for good: other
//! addresses open in other tabs. Restored tabs connect only once they are
//! first shown ([`Tab::load`]). When the connection opens the tab sends `hello`
//! and from then on feeds every frame to the [`Client`], redrawing once per
//! batch of frames. When the transport closes the last state stays on screen
//! with controls disabled, and a banner offers to reconnect.
//!
//! Every address the user opens goes to the history (except the demo apps);
//! the toolbar's star adds it to the favorites, and the popover next to it
//! (and the page of a new tab) lists both, or the demo apps while the history
//! is empty. The metadata the app sends names the tab and gives it an icon,
//! which it keeps until the app sends other metadata, and does the same for
//! its entries there.

use crate::clamp::Clamp;
use crate::debug::{Debugger, Dir};
use crate::library::{self, Library};
use crate::render::Renderer;
use gtk::prelude::*;
use gtk::{gdk, gio, glib, pango};
use gtk4 as gtk;
use hyperb_core::address;
use hyperb_core::hypereffect::client::Client;
use hyperb_core::hypereffect::{Value, json};
use hyperb_core::identity::Identity;
use hyperb_core::library::Entry;
use hyperb_core::net::{Connection, Event};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use url::Url;

/// The widest the rendered page gets; the rest of the window is margin.
const CONTENT_WIDTH: i32 = 800;

pub struct Tab {
    /// The notebook page.
    pub page: gtk::Box,
    /// The notebook tab: spinner or icon, title and close button.
    pub tab_label: gtk::Box,
    pub title: gtk::Label,
    pub close_button: gtk::Button,
    spinner: gtk::Spinner,
    icon: gtk::Image,
    /// Shows the spinner, the icon or nothing, always in the same space.
    lead: gtk::Stack,
    /// The address field of a new tab.
    url_entry: gtk::Entry,
    toolbar: gtk::Box,
    status: gtk::Image,
    address_label: gtk::Label,
    star: gtk::ToggleButton,
    banner: gtk::Box,
    banner_text: gtk::Label,
    content: gtk::Stack,
    debug: Debugger,
    session: RefCell<Session>,
    renderer: RefCell<Renderer>,
    quiet: Cell<bool>,
    /// Restored, and not connected yet.
    pending: Cell<bool>,
    identity: Identity,
    library: Rc<Library>,
    /// Called after the tab opens its address or its title or icon change.
    changed: RefCell<Option<Box<dyn Fn()>>>,
    /// Called with the addresses to open in another tab.
    open_app: RefCell<Box<OpenApp>>,
}

/// What a tab calls with an address to open in another tab, and whether it
/// is a link (from a button), which cannot open local files.
type OpenApp = dyn Fn(&str, bool);

struct Session {
    client: Client,
    conn: Option<Connection>,
    url: Option<Url>,
    /// The title and icon of `url`, from the saved session or its library
    /// entry until the app sends metadata.
    entry: Option<Entry>,
    /// Bumped on every connect, so events from replaced connections are dropped.
    generation: u64,
    state: State,
}

#[derive(Clone, PartialEq)]
enum State {
    Idle,
    Connecting,
    Open,
    Closed(Option<String>),
}

impl Tab {
    pub fn new(identity: Identity, library: Rc<Library>) -> Rc<Tab> {
        Rc::new_cyclic(|weak: &std::rc::Weak<Tab>| {
            // Tab strip entry.
            // Fixed sizes, so the icon and the close button line up in
            // every tab whatever the title.
            let spinner = gtk::Spinner::new();
            let icon = gtk::Image::new();
            icon.set_pixel_size(16);
            let lead = gtk::Stack::new();
            lead.set_size_request(16, 16);
            lead.set_valign(gtk::Align::Center);
            lead.add_named(
                &gtk::Box::new(gtk::Orientation::Horizontal, 0),
                Some("none"),
            );
            lead.add_named(&spinner, Some("spinner"));
            lead.add_named(&icon, Some("icon"));
            lead.set_visible_child_name("none");
            let title = gtk::Label::new(Some("New Tab"));
            title.set_width_chars(18);
            title.set_max_width_chars(18);
            title.set_xalign(0.0);
            title.set_ellipsize(pango::EllipsizeMode::End);
            let close_button = gtk::Button::from_icon_name("window-close-symbolic");
            close_button.add_css_class("flat");
            close_button.add_css_class("circular");
            close_button.add_css_class("hb-tab-close");
            close_button.set_tooltip_text(Some("Close tab"));
            close_button.set_valign(gtk::Align::Center);
            let tab_label = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            tab_label.append(&lead);
            tab_label.append(&title);
            tab_label.append(&close_button);

            // Toolbar.
            let reload = gtk::Button::from_icon_name("view-refresh-symbolic");
            reload.set_tooltip_text(Some("Reconnect (F5)"));
            let status = gtk::Image::new();
            status.set_visible(false);
            let address_label = gtk::Label::new(None);
            address_label.set_hexpand(true);
            address_label.set_xalign(0.0);
            address_label.set_selectable(true);
            address_label.set_ellipsize(pango::EllipsizeMode::Middle);
            let star = gtk::ToggleButton::new();
            star.set_icon_name("non-starred-symbolic");
            star.set_tooltip_text(Some("Add to favorites"));
            star.set_sensitive(false);
            let library_button = gtk::MenuButton::new();
            library_button.set_icon_name("document-open-recent-symbolic");
            library_button.set_tooltip_text(Some("Favorites and history"));
            let toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            toolbar.add_css_class("hb-toolbar");
            toolbar.append(&reload);
            toolbar.append(&status);
            toolbar.append(&address_label);
            toolbar.append(&star);
            toolbar.append(&library_button);
            // A new tab has nothing to reload or star.
            toolbar.set_visible(false);

            // Favorites and history, in the popover and on the welcome page:
            // the tab opens its address there, or a new tab if it has one.
            let open = |popover: Option<gtk::Popover>| {
                let w = weak.clone();
                move |addr: &str| {
                    if let Some(p) = &popover {
                        p.popdown();
                    }
                    if let Some(t) = w.upgrade() {
                        t.navigate(addr);
                    }
                }
            };
            let url_entry = gtk::Entry::new();
            url_entry.set_placeholder_text(Some("Server address"));
            url_entry.set_input_purpose(gtk::InputPurpose::Url);
            let popover = gtk::Popover::new();
            let popover_scroll = gtk::ScrolledWindow::new();
            popover_scroll.set_hscrollbar_policy(gtk::PolicyType::Never);
            popover_scroll.set_propagate_natural_height(true);
            popover_scroll.set_max_content_height(560);
            popover_scroll.set_size_request(420, -1);
            popover_scroll.set_child(Some(&library::view(
                &library,
                50,
                open(Some(popover.clone())),
            )));
            popover.set_child(Some(&popover_scroll));
            library_button.set_popover(Some(&popover));

            // Disconnected banner.
            let banner_text = gtk::Label::new(None);
            banner_text.set_hexpand(true);
            banner_text.set_xalign(0.0);
            banner_text.set_wrap(true);
            let banner_reload = gtk::Button::with_label("Reconnect");
            let banner = gtk::Box::new(gtk::Orientation::Horizontal, 12);
            banner.add_css_class("hb-banner");
            banner.append(&gtk::Image::from_icon_name("network-offline-symbolic"));
            banner.append(&banner_text);
            banner.append(&banner_reload);
            banner.set_visible(false);

            // Rendered session: the root column scrolls, dialogs overlay it.
            let main = gtk::Box::new(gtk::Orientation::Vertical, 12);
            main.add_css_class("he-root");
            let scroll = gtk::ScrolledWindow::new();
            scroll.set_hscrollbar_policy(gtk::PolicyType::Never);
            scroll.set_vexpand(true);
            scroll.set_child(Some(&Clamp::new(CONTENT_WIDTH, &main)));
            let overlay = gtk::Overlay::new();
            overlay.set_child(Some(&scroll));

            let content = gtk::Stack::new();
            content.set_vexpand(true);
            content.add_named(
                &welcome(&url_entry, library::view(&library, 12, open(None))),
                Some("welcome"),
            );
            content.add_named(&overlay, Some("page"));
            content.set_visible_child_name("welcome");

            let page = gtk::Box::new(gtk::Orientation::Vertical, 0);
            page.append(&toolbar);
            page.append(&banner);
            let debug = Debugger::new(weak.clone());
            let body = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            body.append(&content);
            body.append(&debug.widget);
            // The debugger only takes space the page does not use.
            let panel = debug.widget.clone();
            body.add_tick_callback(move |body, _| {
                let room = body.width() >= CONTENT_WIDTH + Debugger::WIDTH;
                if panel.is_visible() != room {
                    panel.set_visible(room);
                }
                glib::ControlFlow::Continue
            });
            page.append(&body);

            {
                let w = weak.clone();
                url_entry.connect_activate(move |e| {
                    if let Some(t) = w.upgrade() {
                        t.navigate(&e.text());
                    }
                });
            }
            {
                let w = weak.clone();
                star.connect_toggled(move |b| {
                    if let Some(t) = w.upgrade() {
                        t.set_favorite(b.is_active());
                    }
                });
            }
            {
                let w = weak.clone();
                library.subscribe(move || match w.upgrade() {
                    Some(t) => {
                        t.update_star();
                        let entry = t.session.borrow().entry.clone();
                        if let Some(e) = entry {
                            t.show_icon(&e);
                        }
                        true
                    }
                    None => false,
                });
            }
            for b in [&reload, &banner_reload] {
                let w = weak.clone();
                b.connect_clicked(move |_| {
                    if let Some(t) = w.upgrade() {
                        t.reload();
                    }
                });
            }
            {
                let w = weak.clone();
                let keys = gtk::EventControllerKey::new();
                keys.set_propagation_phase(gtk::PropagationPhase::Capture);
                keys.connect_key_pressed(move |_, key, _, _| match w.upgrade() {
                    Some(t) if key == gdk::Key::Escape && t.dismiss_dialog() => {
                        glib::Propagation::Stop
                    }
                    _ => glib::Propagation::Proceed,
                });
                overlay.add_controller(keys);
            }

            Tab {
                page,
                tab_label,
                title,
                close_button,
                spinner,
                icon,
                lead,
                url_entry,
                toolbar,
                status,
                address_label,
                star,
                banner,
                banner_text,
                content,
                debug,
                session: RefCell::new(Session {
                    client: Client::new(),
                    conn: None,
                    url: None,
                    entry: None,
                    generation: 0,
                    state: State::Idle,
                }),
                renderer: RefCell::new(Renderer::new(weak.clone(), overlay, main)),
                quiet: Cell::new(false),
                pending: Cell::new(false),
                identity,
                library,
                changed: RefCell::new(None),
                open_app: RefCell::new(Box::new(|_, _| {})),
            }
        })
    }

    /// Calls `f` when the tab opens its address, and each time its title
    /// or icon change.
    pub fn on_change(&self, f: impl Fn() + 'static) {
        *self.changed.borrow_mut() = Some(Box::new(f));
    }

    fn changed(&self) {
        if let Some(f) = &*self.changed.borrow() {
            f();
        }
    }

    /// Calls `f` with each address to open in another tab: the hyperb
    /// addresses buttons ask to open (links), and the ones the user picks in
    /// a tab that has an address already.
    pub fn on_open_app(&self, f: impl Fn(&str, bool) + 'static) {
        *self.open_app.borrow_mut() = Box::new(f);
    }

    /// The address the tab shows, with the title and icon it has, if it
    /// has opened one.
    pub fn entry(&self) -> Option<Entry> {
        self.session.borrow().entry.clone()
    }

    pub fn has_address(&self) -> bool {
        self.session.borrow().url.is_some()
    }

    /// Focuses the address field of a new tab; false if the tab has an
    /// address already.
    pub fn focus_url(&self) -> bool {
        if self.has_address() {
            return false;
        }
        self.url_entry.grab_focus();
        true
    }

    /// Connects to what the user typed or picked. A tab that has an
    /// address already opens it in another tab instead.
    pub fn navigate(self: &Rc<Self>, input: &str) {
        if self.has_address() {
            (self.open_app.borrow())(input, false);
            return;
        }
        self.go(address::normalize(input));
    }

    /// Connects to an address from a link, which cannot open local files.
    pub fn open_link(self: &Rc<Self>, input: &str) {
        self.url_entry.set_text(input);
        self.go(address::from_link(input));
    }

    fn go(self: &Rc<Self>, url: Result<Url, String>) {
        match url {
            Ok(url) => {
                self.content.grab_focus();
                self.library.visit(&url);
                let entry = self
                    .library
                    .entry(&url)
                    .unwrap_or_else(|| Entry::new(&url, 0));
                self.set_address(&url, entry);
                self.connect(url);
                self.changed();
            }
            Err(e) => {
                self.banner_text.set_text(&format!("Invalid address: {e}"));
                self.banner.set_visible(true);
            }
        }
    }

    /// Shows the tab of a saved session, which connects once [`Tab::load`]
    /// is called. False if its address is not valid any more.
    pub fn restore(&self, entry: Entry) -> bool {
        let Ok(url) = address::normalize(&entry.address) else {
            return false;
        };
        self.set_address(&url, entry);
        self.session.borrow_mut().url = Some(url);
        self.content.set_visible_child_name("page");
        self.update_star();
        self.pending.set(true);
        true
    }

    /// Connects a restored tab, the first time it is shown.
    pub fn load(self: &Rc<Self>) {
        if self.pending.replace(false) {
            self.reload();
        }
    }

    fn set_address(&self, url: &Url, entry: Entry) {
        let addr = address::hyperb(url);
        self.title.set_tooltip_text(Some(&addr));
        self.address_label.set_text(&addr);
        self.address_label.set_tooltip_text(Some(&addr));
        self.toolbar.set_visible(true);
        self.show_entry(&entry);
        self.session.borrow_mut().entry = Some(entry);
    }

    pub fn reload(self: &Rc<Self>) {
        let url = self.session.borrow().url.clone();
        match url {
            Some(url) => {
                self.pending.set(false);
                self.connect(url);
            }
            None => self.navigate(&self.url_entry.text()),
        }
    }

    fn connect(self: &Rc<Self>, url: Url) {
        let conn = self.library.connect(&url, &self.identity);
        let events = conn.events();
        let generation = {
            let mut s = self.session.borrow_mut();
            // Until the new transport opens, nothing may be sent: hello must come first.
            s.client.close();
            s.conn = Some(conn);
            s.url = Some(url);
            s.generation += 1;
            s.state = State::Connecting;
            s.generation
        };
        self.content.set_visible_child_name("page");
        self.render();
        self.update_chrome();
        self.update_star();

        let weak = Rc::downgrade(self);
        glib::spawn_future_local(async move {
            while let Ok(ev) = events.recv().await {
                let Some(t) = weak.upgrade() else { break };
                if t.session.borrow().generation != generation {
                    break;
                }
                // Process everything that has arrived before redrawing.
                t.handle(ev);
                while let Ok(ev) = events.try_recv() {
                    t.handle(ev);
                }
                t.render();
                t.update_chrome();
            }
        });
    }

    fn handle(&self, ev: Event) {
        let mut s = self.session.borrow_mut();
        let s = &mut *s;
        match ev {
            Event::Open => {
                let params = s
                    .conn
                    .as_ref()
                    .map(|c| c.params().clone())
                    .unwrap_or_default();
                let hello = s.client.hello_with(&params);
                if let Some(c) = &s.conn {
                    self.debug.log(Dir::Sent, &hello);
                    c.send(hello);
                }
                s.state = State::Open;
            }
            Event::Data(lines) => {
                for l in lines.lines() {
                    self.debug.log(Dir::Received, l);
                }
                s.client.receive(lines.as_bytes())
            }
            Event::Closed(reason) => {
                s.client.close();
                s.conn = None;
                s.state = State::Closed(reason);
            }
        }
    }

    /// Takes the title and icon from the metadata the app sent, and moves
    /// the tab to the address it asks for, if any (`SPEC.md`, section 1.9).
    fn describe(&self) {
        let (url, moved, entry, m) = {
            let mut s = self.session.borrow_mut();
            let s = &mut *s;
            let m = s.client.metadata().clone();
            let (Some(url), Some(entry)) = (&mut s.url, &mut s.entry) else {
                return;
            };
            let from = url.clone();
            let moved = hyperb_core::library::moved(url, &m);
            if let Some(to) = &moved {
                *url = to.clone();
                entry.address = address::hyperb(to);
            }
            if !entry.describe(&m) && moved.is_none() {
                return;
            }
            (from, moved, entry.clone(), m)
        };
        self.library.describe(&url, &m);
        match moved {
            Some(to) => {
                self.library.relocate(&url, &to);
                self.set_address(&to, entry);
                self.update_star();
            }
            None => self.show_entry(&entry),
        }
        self.changed();
    }

    fn show_entry(&self, e: &Entry) {
        self.title.set_text(&e.title);
        self.show_icon(e);
    }

    /// Shows the icon, only while the spinner is hidden (see
    /// `update_chrome`). An `https` icon shows once the library has
    /// downloaded it, which calls this again.
    fn show_icon(&self, e: &Entry) {
        let tex = e.icon.as_deref().and_then(|i| self.library.icon(i));
        self.icon.set_paintable(tex.as_ref());
        self.update_lead();
    }

    fn update_lead(&self) {
        let name = if self.spinner.is_spinning() {
            "spinner"
        } else if self.icon.paintable().is_some() {
            "icon"
        } else {
            "none"
        };
        self.lead.set_visible_child_name(name);
    }

    fn set_favorite(&self, on: bool) {
        let url = self.session.borrow().url.clone();
        if let Some(url) = url
            && self.library.is_favorite(&url) != on
        {
            self.library.set_favorite(&url, on);
        }
    }

    /// Shows whether the page is a favorite. Toggling the button to match
    /// calls `set_favorite`, which then has nothing to do.
    fn update_star(&self) {
        let url = self.session.borrow().url.clone();
        let on = url.as_ref().is_some_and(|u| self.library.is_favorite(u));
        self.star.set_sensitive(url.is_some());
        self.star.set_active(on);
        self.star.set_icon_name(if on {
            "starred-symbolic"
        } else {
            "non-starred-symbolic"
        });
        self.star.set_tooltip_text(Some(if on {
            "Remove from favorites"
        } else {
            "Add to favorites"
        }));
    }

    pub fn is_quiet(&self) -> bool {
        self.quiet.get()
    }

    /// User input on element `id`: writes `value` to its bound signal, if
    /// any, then runs the handler in attribute `event`, if any.
    pub fn input(&self, id: &str, value: Option<Value>, event: Option<&str>) {
        let (line, opens) = {
            let mut s = self.session.borrow_mut();
            if let Some(v) = value {
                let bind = s.client.attr(id, "bind");
                if let Some(path) = bind.as_ref().and_then(Value::as_str) {
                    s.client.write(path, v);
                }
            }
            let line = event.and_then(|e| s.client.trigger(id, e));
            (line, s.client.take_opens())
        };
        for url in opens {
            self.open(&url);
        }
        if let Some(line) = line
            && let Some(c) = &self.session.borrow().conn
        {
            self.debug.log(Dir::Sent, &line);
            c.send(line);
        }
        self.render();
    }

    /// Opens a URL a button asked for (`address::open`): a hyperb address
    /// through `on_open_app`, a web page or an email address in the system's
    /// browser or mail client.
    fn open(&self, url: &str) {
        let url = match address::open(url) {
            Ok(address::Open::External(u)) => u,
            Ok(address::Open::App(u)) => {
                (self.open_app.borrow())(&address::hyperb(&u), true);
                return;
            }
            Err(e) => {
                eprintln!("not opening {url}: {e}");
                return;
            }
        };
        let window = self.page.root().and_downcast::<gtk::Window>();
        gtk::UriLauncher::new(url.as_str()).launch(
            window.as_ref(),
            gio::Cancellable::NONE,
            move |r| {
                if let Err(e) = r {
                    eprintln!("could not open {url}: {e}");
                }
            },
        );
    }

    pub fn select_tab(&self, tabs: &str, tab: &str) {
        self.input(tabs, Some(json!(tab)), Some("onChange"));
    }

    fn dismiss_dialog(&self) -> bool {
        let top = {
            let s = self.session.borrow();
            self.renderer.borrow().top_dialog(&s.client)
        };
        match top {
            Some(id) => {
                self.input(&id, None, Some("onDismiss"));
                true
            }
            None => false,
        }
    }

    fn render(&self) {
        let changes = {
            let mut s = self.session.borrow_mut();
            for d in s.client.take_diagnostics() {
                let url = s.url.as_ref().map(Url::as_str).unwrap_or("-");
                eprintln!("[{url}] {d}");
            }
            s.client.take_changes()
        };
        if changes.is_empty() {
            return;
        }
        if changes.metadata {
            self.describe();
        }
        self.quiet.set(true);
        {
            let s = self.session.borrow();
            self.renderer.borrow_mut().apply(&s.client, changes);
        }
        self.quiet.set(false);
        self.debug.sync_elements(&self.session.borrow().client);
    }

    /// Outlines element `id` in the page, or clears the outline.
    pub fn highlight(&self, id: Option<&str>) {
        if let Ok(mut r) = self.renderer.try_borrow_mut() {
            r.highlight(id);
        }
    }

    fn update_chrome(&self) {
        let s = self.session.borrow();
        let connecting = s.state == State::Connecting;
        self.spinner.set_spinning(connecting);
        self.update_lead();
        let icon = match (&s.state, s.url.as_ref().map(Url::scheme)) {
            (State::Open, Some("https" | "iroh")) => Some("channel-secure-symbolic"),
            (State::Open, _) => Some("channel-insecure-symbolic"),
            (State::Closed(_), _) => Some("network-offline-symbolic"),
            _ => None,
        };
        self.status.set_icon_name(icon);
        self.status.set_visible(icon.is_some());
        self.status.set_tooltip_text(match &s.state {
            State::Open if icon == Some("channel-secure-symbolic") => Some("Connected (encrypted)"),
            State::Open => Some("Connected (not encrypted)"),
            State::Closed(_) => Some("Disconnected"),
            _ => None,
        });
        match &s.state {
            State::Closed(reason) => {
                let text = match reason {
                    Some(r) => format!("Disconnected: {r}"),
                    None => "Disconnected. The server closed the connection.".into(),
                };
                self.banner_text.set_text(&text);
                self.banner.set_visible(true);
            }
            _ => self.banner.set_visible(false),
        }
    }
}

fn welcome(url_entry: &gtk::Entry, library: gtk::Box) -> gtk::Widget {
    let b = gtk::Box::new(gtk::Orientation::Vertical, 12);
    b.add_css_class("hb-welcome");
    let title = gtk::Label::new(Some("hyperb"));
    title.add_css_class("he-v-h1");
    let hint = gtk::Label::new(None);
    hint.set_markup(
        r#"A browser for <a href="https://github.com/hypereffect/hyperb">hyperb</a> apps."#,
    );
    hint.add_css_class("he-v-caption");
    b.append(&title);
    b.append(&hint);
    url_entry.set_margin_top(24);
    b.append(url_entry);
    library.set_margin_top(12);
    b.append(&library);
    let scroll = gtk::ScrolledWindow::new();
    scroll.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroll.set_child(Some(&Clamp::new(560, &b)));
    scroll.upcast()
}
