//! The news feed, as the page receives it.

use super::Error;

/// Where the feed and its images are cached.
///
/// Under DATA and not under config: it's rebuildable, and erasing it only
/// costs a reload. Config, on the other hand, is what must not be lost.
fn cache() -> std::path::PathBuf {
    mc_paths::current().data.join("news")
}

/// The feed, from the network or the last known copy.
///
/// The images are fetched here, in Rust, and served as `data:` — which
/// avoids opening `img-src` to a remote host, and re-requesting the image
/// on every page open.
#[tauri::command]
pub async fn news() -> Result<mc_news::Feed, Error> {
    let cache = cache();
    let dl = mc_dl::Downloader::new(mc_dl::USER_AGENT)?;

    let mut feed = mc_news::load(mc_pack::source::default_url(), &cache, &dl).await?;

    // The images, within what we allow ourselves. A failed image does NOT
    // take its post down with it: the page will display it without an
    // illustration, which is very much preferable to a hole in the feed.
    let mut fetched = 0usize;
    for post in &mut feed.posts {
        let Some(url) = post.image.clone() else {
            continue;
        };
        if fetched >= mc_news::IMAGES_MAX {
            post.image = None;
            continue;
        }
        match mc_news::fetch_image(&url, &cache, &dl).await {
            Ok(path) => match mc_news::to_data_url(&path) {
                Ok(data) => {
                    post.image = Some(data);
                    fetched += 1;
                }
                Err(error) => {
                    tracing::warn!(url, error = %error, "unreadable image, post without illustration");
                    post.image = None;
                }
            },
            Err(error) => {
                tracing::warn!(url, error = %error, "image not fetched");
                post.image = None;
            }
        }
    }

    Ok(feed)
}
