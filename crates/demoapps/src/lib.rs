//! The demo apps. They need nothing from a server, so they can run natively
//! and as wasm modules in the renderer.

pub mod chart;
pub mod common;
pub mod dbclient;
#[cfg(feature = "grammar")]
pub mod grammar;
pub mod img;
pub mod minecraft;
pub mod pomodoro;
pub mod restaurant;
pub mod saas;
pub mod smarthome;

/// The wasm module ABI the renderer loads (`SPEC.md`, section 4). A wasm build
/// hosts exactly one app, chosen by feature.
#[cfg(target_arch = "wasm32")]
mod wasm;
