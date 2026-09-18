//! Fetch the feed, keep a copy, and fetch its images.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::contract::{Feed, Post, RawFeed, SCHEMA, is_valid_date, sort};

/// Maximum size of a post image.
///
/// **A bound, and one was needed.** `Downloader::bytes` has NO limit at all
/// and accumulates the whole body in memory, under a three-hundred-second
/// timeout. A faulty — or compromised — host would serve a file several
/// gigabytes long, and the launcher would grow until the system kills it,
/// with no message at all.
///
/// **The bound protects memory, not the display.** Measured on September 18,
/// 2026 in the window, under WebKitGTK with DMA-BUF disabled: a `data:` of
/// **567 KiB decodes and displays in 5 ms**. Transport is therefore not what
/// justifies this cap, and raising it wouldn't make the page slow — it would
/// only make the launcher vulnerable to a host that served a never-ending
/// file.
pub const IMAGE_MAX: usize = 512 * 1024;

/// How many images we fetch for a feed.
///
/// Eight illustrated posts already come to four megabytes; beyond that,
/// opening the page would cost more than it shows.
pub const IMAGES_MAX: usize = 8;

/// The feed's address, DERIVED from the pack's.
///
/// The same pattern as `lock_url_for`: recopying the three published
/// addresses would recreate exactly the flaw their comment describes — a
/// preproduction that downloads production's content, and therefore proves
/// none of what it's supposed to prove.
pub fn feed_url(pack_url: &str) -> String {
    match pack_url.rsplit_once('/') {
        Some((base, _)) => format!("{base}/news.json"),
        None => "news.json".to_string(),
    }
}

/// The feed, from the network or the cache.
///
/// Only returns an error if both fail: an empty news page is better than a
/// window that refuses to open, but a page that lies about its freshness
/// would be worse than either.
pub async fn load(pack_url: &str, cache: &Path, dl: &mc_dl::Downloader) -> Result<Feed> {
    let url = feed_url(pack_url);
    let copy = cache.join("news.json");

    let bytes = match dl.bytes(&url).await {
        Ok(bytes) => {
            // Only cached after being RE-READ: a truncated response or an
            // HTML error page would otherwise replace a valid feed with
            // nothing.
            if read(&bytes, &url).is_ok()
                && let Err(error) = write_cache(&copy, bytes.clone()).await
            {
                tracing::warn!(error = %error, "feed copy not written");
            }
            bytes
        }
        // A host that ANSWERS "it's not there" doesn't publish a feed: the
        // page opens empty, and that's the right response. Falling back to
        // the copy here would show posts that someone removed on purpose.
        Err(error) if error.downcast_ref::<mc_dl::Absent>().is_some() => {
            tracing::info!(url, "no feed published on this host");
            return Ok(Feed {
                posts: Vec::new(),
                offline: false,
                discarded: Vec::new(),
            });
        }
        Err(error) => {
            tracing::warn!(url, error = %error, "feed unreachable, falling back to the copy");
            let read_copy = mc_dl::read_off_thread(&copy)
                .await
                .with_context(|| format!("feed {url} unreachable, and no copy"))?;
            let mut feed = read(&read_copy, &url)?;
            feed.offline = true;
            return Ok(feed);
        }
    };

    read(&bytes, &url)
}

async fn write_cache(copy: &Path, bytes: Vec<u8>) -> Result<()> {
    mc_dl::write_off_thread(copy, bytes).await
}

/// Parses a feed, post by post.
///
/// A faulty post is discarded ALONE. That's the difference between "the news
/// page has a gap" and "the news page is empty", and the second reads like a
/// launcher failure — when the cause is a comma in a file we don't control at
/// the same pace as the code.
pub fn read(bytes: &[u8], feed_url: &str) -> Result<Feed> {
    let raw: RawFeed = serde_json::from_slice(bytes).context("unreadable news feed")?;

    let mut kept = Vec::new();
    let mut discarded = Vec::new();

    if raw.schema != SCHEMA {
        // In the REPORT, and not just in the log.
        //
        // A feed served under a different schema still reads — the fields we
        // know still parse — but someone needs to learn about it. The log
        // alone isn't enough: nobody opens it as long as nothing looks
        // broken, and that's exactly the case here.
        //
        // The news page already shows `discarded` in a discreet fallback:
        // that's exactly the right place, and it costs nobody anything.
        discarded.push(format!(
            "the feed announces schema {} instead of {SCHEMA}: what parses is kept",
            raw.schema
        ));
    }

    for post in raw.posts {
        if !is_valid_date(&post.date) {
            discarded.push(format!(
                "{}: date \"{}\" outside the expected format (RFC 3339 in UTC)",
                post.id, post.date
            ));
            continue;
        }
        if post.title.trim().is_empty() {
            discarded.push(format!("{}: empty title", post.id));
            continue;
        }

        let image = post.image.as_deref().and_then(|image| {
            match crate::links::absolute_image(image, feed_url) {
                Some(absolute) => Some(absolute),
                None => {
                    discarded.push(format!(
                        "{}: image \"{image}\" outside the feed's host, ignored",
                        post.id
                    ));
                    None
                }
            }
        });

        kept.push(Post {
            body: crate::parsing::parse(&post.body, feed_url),
            id: post.id,
            title: post.title,
            date: post.date,
            pinned: post.pinned,
            image,
        });
    }

    for reason in &discarded {
        tracing::warn!(reason, "post discarded from the feed");
    }

    Ok(Feed {
        posts: sort(kept),
        offline: false,
        discarded,
    })
}

/// Fetches an image into the cache, and returns its local path.
///
/// ## Storage and transport kept separate
///
/// The image is stored as a FILE, named by its URL's digest. The `data:` the
/// window consumes is only built on demand, by [`to_data_url`].
///
/// Keeping images as base64 in the JSON cache would mean a feed of eight
/// illustrated posts weighed three and a half megabytes, re-read and
/// re-serialized on every page open — for a render that only shows the first
/// one before you scroll.
pub async fn fetch_image(url: &str, cache: &Path, dl: &mc_dl::Downloader) -> Result<PathBuf> {
    let destination = cache.join("images").join(file_name(url));
    if destination.is_file() {
        return Ok(destination);
    }

    let bytes = dl.bytes(url).await?;
    if bytes.len() > IMAGE_MAX {
        // In KiB and not bytes: the message will be read in a log by someone
        // looking for why an illustration is missing, and "524289 beyond
        // 524288" compares less quickly than "513 beyond 512".
        anyhow::bail!(
            "image {url}: {} KiB, beyond the {} KiB allowed",
            bytes.len().div_ceil(1024),
            IMAGE_MAX / 1024
        );
    }
    if mime_type(&bytes).is_none() {
        anyhow::bail!("image {url}: these bytes are not a recognized image");
    }

    mc_dl::write_off_thread(&destination, bytes).await?;
    Ok(destination)
}

/// The cache file name of a cached image.
///
/// The URL's digest, not its last segment: two posts might each carry an
/// `image.webp`, and the second would overwrite the first.
fn file_name(url: &str) -> String {
    mc_dl::sha512_of_bytes(url.as_bytes())[..32].to_string()
}

/// The MIME type, DEDUCED FROM THE BYTES and not the extension.
///
/// An extension comes from the URL, hence from the host, hence from
/// something we don't control at the same pace as the code. Magic numbers,
/// on the other hand, are in the file.
pub fn mime_type(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some("image/png");
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some("image/jpeg");
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some("image/gif");
    }
    // WebP: "RIFF" then four size bytes then "WEBP".
    //
    // `>= 12` and not `> 12`: twelve bytes ARE exactly the signature, and
    // that's all it takes to identify it. Requiring a thirteenth would refuse
    // a complete header because it's missing a byte of content — which isn't
    // the question this function is asked.
    if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        return Some("image/webp");
    }
    None
}

/// Builds the `data:` the window consumes, on demand.
pub fn to_data_url(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)?;
    let mime = mime_type(&bytes)
        .with_context(|| format!("{} is no longer a recognized image", path.display()))?;
    Ok(format!("data:{mime};base64,{}", base64(&bytes)))
}

/// Standard base64, no dependency.
///
/// Thirty lines are worth it over one more crate in a binary we ship: it's an
/// encoding fixed since 1987, and this one is used for exactly one thing.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);

    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        // `from_be_bytes` and not three shifts assembled with `|`.
        //
        // The computation is the same, the reading is better, and it turns
        // out to also be the only form that leaves nothing to mutate: on
        // bytes packed into disjoint bit fields, `|` and `^` produce exactly
        // the same number. A mutant that swaps one for the other is
        // therefore EQUIVALENT — no test can kill it, and it would remain
        // among the survivors forever.
        //
        // The reflex would be to exclude it with an attribute. But an
        // equivalent mutant often signals that an expression is taking a
        // detour: here, the detour was assembling by hand what the standard
        // library assembles better.
        let n = u32::from_be_bytes([0, b[0], b[1], b[2]]);
        output.push(ALPHABET[(n >> 18) as usize & 63] as char);
        output.push(ALPHABET[(n >> 12) as usize & 63] as char);
        output.push(if chunk.len() > 1 {
            ALPHABET[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        output.push(if chunk.len() > 2 {
            ALPHABET[n as usize & 63] as char
        } else {
            '='
        });
    }
    output
}

#[cfg(test)]
#[path = "fetch.test.rs"]
mod tests;
