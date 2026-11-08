//! The tab sidebar: the strip of tabs on the left, whose width the user
//! drags.
//!
//! GTK sizes a notebook's tab strip to its tabs. Here the strip is the
//! notebook's header (its first child), and a width request gives it a width
//! of its own; since the tab titles ellipsize ([`crate::tab`]), it can be
//! both narrower and wider than the titles. A press on the strip's right
//! edge starts a resize. The click is claimed before the notebook sees it,
//! so dragging the edge does not reorder the tab under it instead.

use gtk::gdk;
use gtk::prelude::*;
use gtk4 as gtk;
use std::cell::Cell;
use std::rc::Rc;

/// The width the sidebar starts at, before the user drags it.
pub const DEFAULT_WIDTH: i32 = 220;
/// The narrowest and widest the sidebar gets.
const MIN_WIDTH: i32 = 140;
const MAX_WIDTH: i32 = 520;
/// How far before and after the strip's edge a press starts a resize. The
/// part before is clear of the tab controls, which end a few pixels in.
const GRIP_IN: f64 = 2.0;
const GRIP_OUT: f64 = 12.0;

/// Lets the user change the width of the tab strip of `notebook` by dragging
/// its right edge.
pub fn resizable(notebook: &gtk::Notebook) {
    let Some(header) = notebook.first_child() else {
        return;
    };
    header.set_size_request(DEFAULT_WIDTH, -1);

    let start_x = Rc::new(Cell::new(0.0));
    let start_width = Rc::new(Cell::new(DEFAULT_WIDTH));
    let resizing = Rc::new(Cell::new(false));

    let press = gtk::GestureClick::new();
    press.set_button(gdk::BUTTON_PRIMARY);
    press.set_propagation_phase(gtk::PropagationPhase::Capture);
    {
        let notebook = notebook.clone();
        let header = header.clone();
        let start_x = start_x.clone();
        let start_width = start_width.clone();
        let resizing = resizing.clone();
        press.connect_pressed(move |gesture, _, x, _| {
            if !at_edge(&header, x) {
                return;
            }
            resizing.set(true);
            start_x.set(x);
            start_width.set(header.width());
            gesture.set_state(gtk::EventSequenceState::Claimed);
            notebook.set_cursor_from_name(Some("col-resize"));
        });
    }
    {
        let notebook = notebook.clone();
        let resizing = resizing.clone();
        press.connect_released(move |_, _, _, _| {
            if resizing.replace(false) {
                notebook.set_cursor_from_name(None);
            }
        });
    }
    {
        let notebook = notebook.clone();
        let resizing = resizing.clone();
        press.connect_cancel(move |_, _| {
            if resizing.replace(false) {
                notebook.set_cursor_from_name(None);
            }
        });
    }
    notebook.add_controller(press);

    // Motion moves the edge while the button is down, and shows that it can
    // be dragged while it is not.
    let motion = gtk::EventControllerMotion::new();
    motion.set_propagation_phase(gtk::PropagationPhase::Capture);
    {
        let notebook = notebook.clone();
        let header = header.clone();
        let start_x = start_x.clone();
        let start_width = start_width.clone();
        let resizing = resizing.clone();
        motion.connect_motion(move |_, x, _| {
            if resizing.get() {
                let width = start_width.get() + (x - start_x.get()) as i32;
                header.set_size_request(width.clamp(MIN_WIDTH, MAX_WIDTH), -1);
            } else {
                let name = at_edge(&header, x).then_some("col-resize");
                notebook.set_cursor_from_name(name);
            }
        });
    }
    notebook.add_controller(motion);
}

/// Whether `x`, in the notebook's coordinates, is on the strip's right edge.
/// Most of the grip is on the page's side of the edge, where no tab control
/// is.
fn at_edge(header: &gtk::Widget, x: f64) -> bool {
    let edge = header.width() as f64;
    (edge - GRIP_IN..=edge + GRIP_OUT).contains(&x)
}
