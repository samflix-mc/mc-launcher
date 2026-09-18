//! A post's body, as a typed tree.
//!
//! ## Why a tree and not HTML
//!
//! The launcher's CSP loosens `style-src` up to `'unsafe-inline'`. That
//! loosening holds on one condition only: no markup comes from anywhere but
//! the Angular compiler. Rendering HTML produced by a remote host — even our
//! own — in an origin where `invoke` is reachable would void that condition
//! at a stroke.
//!
//! Markdown is therefore parsed HERE, in Rust, into closed variants. The
//! front walks them with a `@switch` and never sees a string it would need to
//! interpret. `pnpm invariants` checks that no `innerHTML` lingers on the
//! front side; this module is the other half of the guarantee.
//!
//! ## Why `#[serde(tag = "type")]`
//!
//! Without it, serde produces the "externally tagged" representation —
//! `{"Paragraph": {...}}` — and the front would have to inspect an object's
//! first KEY to know what it's looking at. With it, the front reads a `type`
//! field and writes an ordinary `@switch`.

use serde::Serialize;

/// A top-level block.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Block {
    Paragraph {
        content: Vec<Inline>,
    },
    /// A heading. The level is bounded to 2..=4: the post's title already
    /// occupies level 1 of the page, and a `#` in the body must not compete
    /// with it.
    Heading {
        level: u8,
        content: Vec<Inline>,
    },
    List {
        items: Vec<Vec<Inline>>,
    },
    /// A separator line.
    Separator,
}

/// A fragment of text.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Inline {
    Text {
        text: String,
    },
    Bold {
        text: String,
    },
    Italic {
        text: String,
    },
    Code {
        text: String,
    },
    /// A link. The URL is already filtered — see `links::acceptable`.
    ///
    /// It will NEVER be followed inside the window: the front hands it to
    /// `ouvrirPage()`, which passes it to the system browser. The navigation
    /// plugin would refuse anyway.
    Link {
        text: String,
        href: String,
    },
}
