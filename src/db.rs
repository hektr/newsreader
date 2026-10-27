use rusqlite::{params, Connection, OptionalExtension, Result};
use std::path::PathBuf;

pub struct Db {
    conn: Connection,
}

impl Db {
    /// Opens (or creates) the database at ~/.local/share/newsreader/newsreader.db
    pub fn open() -> Result<Self> {
        let path = db_path();
        std::fs::create_dir_all(path.parent().unwrap()).ok();
        
        let conn = Connection::open(path)?;
        
        // Enable WAL mode — faster and safer for concurrent access
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        
        let db = Db { conn };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS feeds (
                id INTEGER PRIMARY KEY,
                url TEXT UNIQUE NOT NULL,
                title TEXT,
                last_fetched_at TEXT
            );

            CREATE TABLE IF NOT EXISTS entries (
                id TEXT PRIMARY KEY,
                feed_id INTEGER NOT NULL REFERENCES feeds(id) ON DELETE CASCADE,
                title TEXT NOT NULL,
                url TEXT,
                published_at TEXT,
                fetched_at TEXT,
                content TEXT,
                status TEXT NOT NULL DEFAULT 'unread'
            );

            CREATE INDEX IF NOT EXISTS idx_entries_feed ON entries(feed_id);
            CREATE INDEX IF NOT EXISTS idx_entries_status ON entries(status);
            CREATE INDEX IF NOT EXISTS idx_entries_published ON entries(published_at DESC);",
        )?;
        Ok(())
    }

    /// Adds a new feed. Returns its id.
    pub fn add_feed(&self, url: &str) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO feeds (url) VALUES (?1)",
            params![url],
        )?;
        Ok(self.conn.last_insert_rowid())
    }
    
    /// Returns a list of feeds with unread counts: (id, title/url, unread_count).
    pub fn feeds_with_unread(&self) -> Result<Vec<(i64, String, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT f.id, COALESCE(f.title, f.url),
                    (SELECT COUNT(*) FROM entries e WHERE e.feed_id = f.id AND e.status = 'unread')
             FROM feeds f
             ORDER BY f.id"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;
        rows.collect()
    }

    /// Returns all feeds: (id, url, title).
    pub fn list_feeds(&self) -> Result<Vec<(i64, String, Option<String>)>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, url, title FROM feeds ORDER BY id"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;
        rows.collect()
    }

    /// Adds or updates an entry. Returns true if it was a new article.
	pub fn upsert_entry(
    	&self,
    	id: &str,
    	feed_id: i64,
    	title: &str,
    	url: Option<&str>,
    	published_at: Option<&str>,
	) -> Result<bool> {
    	// First, check if an article with this URL already exists in this feed
    	if let Some(article_url) = url {
        	let existing: Option<String> = self.conn
            	.query_row(
                	"SELECT id FROM entries WHERE url = ?1 AND feed_id = ?2 LIMIT 1",
                	params![article_url, feed_id],
                	|row| row.get(0),
            	)
            	.optional()?;
        
        	if let Some(existing_id) = existing {
            	// An article with this URL already exists, updating it
            	self.conn.execute(
                	"UPDATE entries SET title = ?1, published_at = ?2 WHERE id = ?3",
                	params![title, published_at, existing_id],
            	)?;
            	return Ok(false); // Not a new article
        	}
    	}
    
    	// URL is not specified or article not found - using standard UPSERT
    	let changed = self.conn.execute(
        	"INSERT INTO entries (id, feed_id, title, url, published_at)
         	VALUES (?1, ?2, ?3, ?4, ?5)
         	ON CONFLICT(id) DO UPDATE SET
            	title = excluded.title,
            	url = COALESCE(excluded.url, entries.url)",
        	params![id, feed_id, title, url, published_at],
    	)?;
    	Ok(changed > 0)
	}

    /// Saves the downloaded article text.
    pub fn save_content(&self, id: &str, content: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE entries SET content = ?1, fetched_at = datetime('now') WHERE id = ?2",
            params![content, id],
        )?;
        Ok(())
    }

    /// Marks the article as read.
    pub fn mark_read(&self, id: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE entries SET status = 'read' WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    /// Returns all articles for TUI: (id, title, feed_title, published_at, status).
    pub fn all_entries(&self) -> Result<Vec<EntryRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT e.id, e.title, COALESCE(f.title, f.url), e.published_at, e.status, e.url
             FROM entries e
             LEFT JOIN feeds f ON f.id = e.feed_id
             ORDER BY e.feed_id, e.published_at DESC, e.rowid DESC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(EntryRow {
                id: row.get(0)?,
                title: row.get(1)?,
                feed: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                published: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                status: row.get(4)?,
                url: row.get::<_, Option<String>>(5)?.unwrap_or_default(),
            })
        })?;
        rows.collect()
    }

    /// Returns the full text of article (if already downloaded).
    pub fn get_content(&self, id: &str) -> Result<Option<String>> {
        self.conn.query_row(
            "SELECT content FROM entries WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .or(Ok(None))
    }

    /// Returns a list of feeds along with their URLs — for fetch.
    pub fn feeds_for_fetch(&self) -> Result<Vec<(i64, String)>> {
        let mut stmt = self.conn.prepare("SELECT id, url FROM feeds")?;
        let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        rows.collect()
    }

    /// Updates last_fetched_at for the feed.
    pub fn touch_feed(&self, feed_id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE feeds SET last_fetched_at = datetime('now') WHERE id = ?1",
            params![feed_id],
        )?;
        Ok(())
    }
    
    /// Deletes the feed and all its associated articles (thanks to ON DELETE CASCADE).
    pub fn remove_feed(&self, id: i64) -> Result<bool> {
        let changed = self.conn.execute(
            "DELETE FROM feeds WHERE id = ?1",
            params![id],
        )?;
        Ok(changed > 0)
    }

    /// Searches for a feed by URL. Returns (id, url) if found.
    pub fn get_feed_by_url(&self, url: &str) -> Result<Option<(i64, String)>> {
        self.conn
            .query_row(
                "SELECT id, url FROM feeds WHERE url = ?1",
                params![url],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
    }
    
    pub fn get_feed_by_id(&self, id: i64) -> Result<Option<(i64, String)>> {
    self.conn
        .query_row(
            "SELECT id, url FROM feeds WHERE id = ?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
	}

    /// Returns the most recent added feed (id, url).
    pub fn get_last_feed(&self) -> Result<Option<(i64, String)>> {
        self.conn
            .query_row(
                "SELECT id, url FROM feeds ORDER BY id DESC LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
    }
}

#[derive(Clone)]
pub struct EntryRow {
    pub id: String,
    pub title: String,
    pub feed: String,
    pub published: String,
    pub status: String,
    pub url: String,
}

fn db_path() -> PathBuf {
    let data_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("newsreader");
    data_dir.join("newsreader.db")
}
