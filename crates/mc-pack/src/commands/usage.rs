//! The help page.

use mc_pack::source;

pub fn usage() {
    eprintln!(
        "usage:\n  \
         mc-pack install [source] [--locked] [--with-server] [--instance NAME] [--data DIR]\n  \
         mc-pack lock    <manifest> [--data DIR]\n  \
         mc-pack verify  [source] [--deep] [--data DIR]\n  \
         mc-pack launch  [source] --username NAME [--server HOST[:PORT]] [--memory MB] [--show]\n  \
         mc-pack diagnostic [--incident-test]\n\n\
         \"source\" is a path to a manifest, or a URL.\n  \
         Default: {}",
        source::default_url()
    );
}

#[cfg(test)]
#[path = "usage.test.rs"]
mod tests;
