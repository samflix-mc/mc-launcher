//! What a jar truly declares, read from the jar and not from an API.
//!
//! The dependencies announced by Modrinth or CurseForge are entered by hand
//! by the author at publish time. They're often incomplete: a library added
//! between two versions, a dependency considered obvious, an automated
//! upload that leaves the field blank. The game itself only reads
//! `META-INF/neoforge.mods.toml` — and stops at startup as soon as a
//! required dependency is missing from it.
//!
//! So we read the same source NeoForge does. Two details decide the
//! correctness of the result:
//!
//! - **JarJar**: a mod can bundle its libraries in `META-INF/jarjar/`. They
//!   supply their `modId` without existing as a separate file. Ignoring them
//!   would conclude a dependency is missing and install a duplicate — two
//!   versions of the same mod, which NeoForge refuses. But they stay
//!   **separate from the mod's identity**: confusing the two cost even
//!   more, since two mods bundling the same library would then look like a
//!   duplicate. Sodium and Iris share four Fabric shims, and the resolver
//!   was silently dropping one of the two;
//! - **a dependency's `side`**: a dependency declared `side = "CLIENT"` has
//!   no business in the server's `mods` folder.

mod descriptor;
mod reading;
pub(crate) mod side;

pub use descriptor::{JarInfo, PLATFORM_IDS, Requirement, is_platform, parse_descriptor};
pub use reading::{inspect, inspect_bytes};
pub use side::Side;
