//! What we allow resolution to do.

#[derive(Debug, Clone, Copy)]
pub struct Options {
    /// Follow dependencies announced by the APIs.
    ///
    /// Disabling them breaks nothing: the catch-up pass, by reading the
    /// jars, finds the same dependencies, just one pass later. This is what
    /// lets us verify that this catch-up works — and rely solely on what
    /// the game will read, when a publication listing is wrong.
    pub follow_declared: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            follow_declared: true,
        }
    }
}
