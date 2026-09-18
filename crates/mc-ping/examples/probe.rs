//! Pings a real server, from the command line.
//!
//! The suite exercises the protocol against fake listeners written in this
//! crate — which proves the framing is self-consistent, and nothing more. A
//! self-consistent encoder and decoder agree on a protocol they invented
//! together; only a real server says whether it's Minecraft's.
//!
//! ```text
//! cargo run -p mc-ping --example probe -- 78.46.100.5 25565
//! ```

use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let host = args.next().unwrap_or_else(|| "127.0.0.1".to_string());
    let port: u16 = args
        .next()
        .unwrap_or_else(|| "25565".to_string())
        .parse()
        .unwrap_or(25565);

    println!("pinging {host}:{port}…");
    match mc_ping::ping(&host, port, Duration::from_secs(5)).await {
        Ok(status) => {
            let count = |value: Option<u32>| value.map_or("—".to_string(), |n| n.to_string());
            println!(
                "  online  : {} / {}",
                count(status.online),
                count(status.max)
            );
            println!("  version : {}", status.version.as_deref().unwrap_or("—"));
            println!("  latency : {} ms", status.latency_ms);
        }
        Err(error) => println!("  offline : {error:#}"),
    }
    Ok(())
}
