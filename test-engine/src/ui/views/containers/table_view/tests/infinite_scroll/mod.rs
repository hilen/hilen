// Only the wasm gated test in `infinite_scroll` drives it.
#[cfg(not_wasm)]
mod basic_scroll;
mod infinite_cell;
mod infinite_scroll;
mod loading_cells;

pub use infinite_scroll::InfiniteScrollTest;
