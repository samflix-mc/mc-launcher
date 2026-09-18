//! The network's news, as the window shows them.
//!
//! ## What this crate guarantees
//!
//! **No HTML reaches the DOM.** Post markdown is parsed here, in Rust, into
//! closed variants that the front walks with a `@switch`. This is the Rust
//! half of the condition under which the CSP was loosened; the other half is
//! `pnpm invariants`, which checks that no `innerHTML` lingers on the front
//! side.
//!
//! ## What it doesn't do
//!
//! It derives NO path. The cache directory is given to it — `mc-paths`
//! decides, and only it. A crate that computes its own path is one more crate
//! that can drift.
//!
//! ## A note on privacy, and it isn't the one you'd expect
//!
//! Fetching images instead of letting the WebView load them does NOT avoid
//! signaling to the host: the "same host" constraint means Rust has just
//! queried that very host for the feed. What fetching avoids is narrower, and
//! worth stating exactly: the host no longer counts PAGE OPENS, since an
//! image already in cache isn't requested again.
//!
//! What it really buys is control: a bounded size, a type deduced from the
//! bytes, and an `img-src` that doesn't need to open up to a remote host.

pub mod contract;
pub mod fetch;
pub mod links;
pub mod parsing;
pub mod tree;

pub use contract::{Feed, Post, SCHEMA};
pub use fetch::{IMAGE_MAX, IMAGES_MAX, feed_url, fetch_image, load, to_data_url};
pub use tree::{Block, Inline};
