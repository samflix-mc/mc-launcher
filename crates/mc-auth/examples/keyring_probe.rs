//! Writes then reads a keyring entry, to tell a real store from a fake one.
//!
//! Run it TWICE: the first call writes, the second reads back in a fresh
//! process. A store that only lives in memory answers the first and not the
//! second — which is exactly how a session that "saves" can still be gone at
//! the next launch.

fn main() -> anyhow::Result<()> {
    let mode = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "read".to_string());
    let service = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "samflix-mc-probe".to_string());
    let user = std::env::args()
        .nth(3)
        .unwrap_or_else(|| "probe".to_string());
    let entry = keyring::Entry::new(&service, &user)?;

    match mode.as_str() {
        "write" => {
            entry.set_password("hello")?;
            println!("written");
        }
        "erase" => {
            entry.delete_credential()?;
            println!("erased");
        }
        _ => match entry.get_password() {
            Ok(value) => println!("read back: {value}"),
            Err(error) => println!("nothing read: {error}"),
        },
    }
    Ok(())
}
