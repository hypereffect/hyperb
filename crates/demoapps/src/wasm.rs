//! A demo app as a wasm module for the renderer's wasm sessions (`SPEC.md`,
//! section 4): `receive` runs the app on the lines it is given and passes
//! what the app wrote to the renderer's `hyperb.send`. An app that ticks
//! asks the renderer to `wake` it, again after every tick, for as long as
//! it wants ticks.

/// Selects the single app this module hosts. Exactly one app feature must be
/// enabled: the module exports one `receive` and owns one `Server<App>`, so
/// `just wasm-apps` builds each app separately.
macro_rules! select_app {
    ($($feature:literal => $app:ty),* $(,)?) => {
        $(
            #[cfg(feature = $feature)]
            type App = $app;
        )*

        const _: () = {
            let mut count = 0;
            $( if cfg!(feature = $feature) { count += 1; } )*
            assert!(
                count == 1,
                "build the wasm module with exactly one demo app feature, \
                 e.g. `--no-default-features --features pomodoro`"
            );
        };
    };
}

select_app! {
    "billsplit" => crate::billsplit::BillSplit,
    "chart" => crate::chart::Chart,
    "dbclient" => crate::dbclient::DbClient,
    "finance" => crate::finance::Finance,
    "grammar" => crate::grammar::Grammar,
    "minecraft" => crate::minecraft::Minecraft,
    "pomodoro" => crate::pomodoro::Pomodoro,
    "qr" => crate::qr::Qr,
    "restaurant" => crate::restaurant::Restaurant,
    "saas" => crate::saas::Saas,
    "smarthome" => crate::smarthome::SmartHome,
    "textstats" => crate::textstats::TextStats,
}

use hypereffect::server::Server;
use std::cell::{Cell, RefCell};

#[link(wasm_import_module = "hyperb")]
unsafe extern "C" {
    fn send(ptr: *const u8, len: u32);
    fn now() -> f64;
    fn wake_in(ms: i32);
}

thread_local! {
    static SERVER: RefCell<Server<App>> = RefCell::new(Server::new());
    /// The tick interval of the pending wake-up, if one is.
    static WAKING: Cell<Option<u32>> = const { Cell::new(None) };
}

/// A buffer for the renderer to write a line into; `receive` takes it back.
#[unsafe(no_mangle)]
pub extern "C" fn alloc(len: u32) -> *mut u8 {
    Box::into_raw(vec![0u8; len as usize].into_boxed_slice()) as *mut u8
}

#[unsafe(no_mangle)]
pub extern "C" fn receive(ptr: *mut u8, len: u32) {
    // SAFETY: `ptr` came from `alloc(len)` and is received once.
    let input = unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len as usize)) };
    SERVER.with_borrow_mut(|s| {
        s.receive(&input);
        flush(s);
    });
}

/// The wake-up `flush` asked for.
#[unsafe(no_mangle)]
pub extern "C" fn wake() {
    WAKING.set(None);
    SERVER.with_borrow_mut(|s| {
        // SAFETY: an import without arguments.
        s.tick(unsafe { now() });
        flush(s);
    });
}

/// Sends what the app wrote, and asks for the wake-up of its next tick,
/// or cancels the pending one if it no longer ticks at that interval.
fn flush(s: &mut Server<App>) {
    let out = s.output().take();
    if !out.is_empty() {
        // SAFETY: the buffer is valid for the call.
        unsafe { send(out.as_ptr(), out.len() as u32) };
    }
    let want = s.tick_interval();
    if want != WAKING.get() {
        let ms = want.map_or(-1, |ms| ms.min(i32::MAX as u32) as i32);
        // SAFETY: an import taking a number.
        unsafe { wake_in(ms) };
        WAKING.set(want);
    }
}

/// Random numbers for Harper's models (the `custom` backend of getrandom): the
/// module has no source of entropy, so they come from the clock, through
/// SplitMix64. Not for secrets.
#[cfg(feature = "grammar")]
#[unsafe(no_mangle)]
unsafe extern "Rust" fn __getrandom_v03_custom(
    dest: *mut u8,
    len: usize,
) -> Result<(), getrandom::Error> {
    thread_local! {
        static STATE: Cell<u64> = Cell::new(unsafe { now() }.to_bits());
    }
    // SAFETY: getrandom passes a buffer of `len` bytes.
    let dest = unsafe { std::slice::from_raw_parts_mut(dest, len) };
    for chunk in dest.chunks_mut(8) {
        let x = STATE.get().wrapping_add(0x9e37_79b9_7f4a_7c15);
        STATE.set(x);
        let mut z = x;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^= z >> 31;
        chunk.copy_from_slice(&z.to_le_bytes()[..chunk.len()]);
    }
    Ok(())
}
