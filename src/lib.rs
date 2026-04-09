//! # Snake WASM
//!
//! A professional Snake game implemented in Rust and compiled to WebAssembly.
//!
//! This crate exposes a [`Game`] struct via `wasm-bindgen` that drives all
//! game logic.  The JavaScript host is responsible only for rendering and
//! forwarding user-input events.

mod game;
mod snake;

pub use game::{Game, GameState};
pub use snake::{Direction, Snake};

// Re-export wasm-bindgen for easier access in tests
use wasm_bindgen::prelude::*;

/// Initialise the panic hook so Rust panics show up in the browser console.
#[wasm_bindgen(start)]
pub fn main() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}
