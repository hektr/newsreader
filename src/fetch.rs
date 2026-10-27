use crate::db::Db;
use feed_rs::parser;
use readability::extractor;
use std::io::Cursor;
use std::time::Duration;
use url::Url;

/// Downloads feeds. If target_url is specified, downloads only it.
/// Returns the number of new articles.
pub fn fetch_feeds(db: &Db, target_url: Option<&str>) -> Result<usize, String> {
    let feeds_to_fetch = if let Some(url) = target_url {
        // Check if this feed already exists in the database
        if let Some(feed) = db.get_feed_by_url(url).map_err(|e| e.to_string())? {
            vec![feed]
        } else {
            // If not, add it on the fly
            let id = db.add_feed(url).map_err(|e| e.to_string())?;
            vec![(id, url.to_string())]
        }
    } else {
        db.feeds_for_fetch().map_err(|e| e.to_string())?
    };

    if feeds_to_fetch.is_empty() {
        return Err("No feeds to update. Use: newsreader add <url>".into());
    }

    let mut total_new = 0;
    for (feed_id, url) in feeds_to_fetch {
        eprint!("{} ... ", url);
        match fetch_feed(db, feed_id, &url) {
            Ok(n) => {
                eprintln!("+{} new", n);
                total_new += n;
            }
            Err(e) => eprintln!("error: {}", e),
        }
    }
    Ok(total_new)
}
fn fetch_feed(db: &Db, feed_id: i64, url: &str) -> Result<usize, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent("newsreader/0.1")
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client.get(url).send().map_err(|e| e.to_string())?;
    let bytes = resp.bytes().map_err(|e| e.to_string())?;
    let feed = parser::parse(&*bytes).map_err(|e| e.to_string())?;

    let mut new_count = 0;
    for entry in &feed.entries {
        let title = entry.title.as_ref().map(|t| t.content.as_str()).unwrap_or("(no title)");
        let link = entry.links.iter()
    		.find(|l| l.rel.as_deref().unwrap_or("alternate") == "alternate")
    		.or_else(|| entry.links.first())
    		.map(|l| l.href.as_str())
    		.or_else(|| if entry.id.starts_with("http") { Some(entry.id.as_str()) } else { None });
        let published = entry.published.or(entry.updated)
            .map(|d| d.to_rfc3339());

        let is_new = db
            .upsert_entry(
                &entry.id,
                feed_id,
                title,
                link,
                published.as_deref(),
            )
            .map_err(|e| e.to_string())?;

        if is_new {
            new_count += 1;
            // Download full text for the new article
            if let Some(url) = link {
                if let Some(text) = fetch_article_text(url) {
                    let _ = db.save_content(&entry.id, &text);
                }
            }
        }
    }

    let _ = db.touch_feed(feed_id);
    Ok(new_count)
}

fn fetch_article_text(url: &str) -> Option<String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .build()
        .ok()?;
    let html = client.get(url).send().ok()?.text().ok()?;
    let base_url = Url::parse(url).ok()?;
    let mut cursor = Cursor::new(html);
    extractor::extract(&mut cursor, &base_url).ok().map(|a| a.text)
}
