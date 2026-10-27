mod db;
mod fetch;
mod ui;

use db::Db;
use std::env;

fn print_help() {
    println!("newsreader 0.1.0");
    println!("Author: hektr <hektr992@gmail.com>");
    println!("GitHub: https://github.com/hektr/newsreader");
    println!();
    println!("USAGE:");
    println!("  newsreader              Run the TUI interface");
    println!("  newsreader [COMMAND]    Run a specific command");
    println!();
    println!("COMMANDS:");
    println!("  add <url>               Add a new feed");
    println!("  remove <id>             Remove a feed by its ID");
    println!("  list                    Show the list of added feeds");
    println!("  fetch                   Update all feeds and download new articles");
    println!("  fetch <url>             Update or add a specific URL");
    println!("  fetch <id>             	Update feed by ID");
    println!("  fetch last              Update only the last added feed");
    println!("  import <file.txt>       Add feeds from a text file (one URL per line)");
    println!("  -h, --help              Print this help message");
    println!();
    println!("TUI SHORTCUTS:");
    println!("  ↑/↓ or j/k              Navigate list, scroll text");
    println!("  Enter or →              Open article");
    println!("  Esc or ←                Return to list");
    println!("  /                       Search by title");
    println!("  f                       Select feed (filter by source)");
    println!("  a                       Show all feeds (reset filter)");
    println!("  q or Esc                Exit program");
    println!("  F1 or ?                 Show help inside TUI");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    // CHECK FOR HELP FLAG FIRST, BEFORE OPENING THE DATABASE
    if matches!(args.get(1).map(|s| s.as_str()), Some("-h" | "--help")) {
        print_help();
        return Ok(());
    }

    let db = Db::open()?;

    match args.get(1).map(|s| s.as_str()) {
        None => {
            // Run TUI
            ui::run(&db)?;
        }
        Some("add") => {
            let url = args.get(2).ok_or("usage: newsreader add <url>")?;
            db.add_feed(url)?;
            println!("Feed added: {}", url);
        }
        Some("remove") | Some("delete") => {
            let id_str = args.get(2).ok_or("usage: newsreader remove <id>")?;
            let id: i64 = id_str.parse().map_err(|_| "ID must be a number")?;
            if db.remove_feed(id)? {
                println!("Feed #{} and all its articles successfully deleted.", id);
            } else {
                println!("Feed with ID {} not found.", id);
            }
        }
        Some("list") => {
            let feeds = db.list_feeds()?;
            if feeds.is_empty() {
                println!("Feed list is empty. Add one: newsreader add <url>");
            } else {
                println!("ID | Title | URL");
                println!("---|-----------|----");
                for (id, url, title) in feeds {
                    let t = title.unwrap_or_else(|| "—".into());
                    println!("{}  | {} | {}", id, t, url);
                }
            }
        }
        Some("fetch") => {
    		let target = args.get(2).map(|s| s.as_str());
    		let target_url = if let Some("last") = target {
        		match db.get_last_feed()? {
            		Some((_, url)) => Some(url),
            		None => return Err("No feeds added".into()),
        		}
    		} else if let Some(arg) = target {
        		if let Ok(id) = arg.parse::<i64>() {
            		match db.get_feed_by_id(id)? {
                		Some((_, url)) => Some(url),
                		None => return Err(format!("Feed with ID {} not found", id).into()),
            		}
        		} else {
            		Some(arg.to_string())
        		}
    		} else {
        		None
    		};
    		let new_count = fetch::fetch_feeds(&db, target_url.as_deref())?;
    		println!("Done. Total new articles: {}", new_count);
		}
        Some("import") => {
            let file_path = args.get(2).ok_or("usage: newsreader import <file.txt>")?;
            let content = std::fs::read_to_string(file_path)
                .map_err(|e| format!("Failed to read file: {}", e))?;
            let mut added = 0;
            let mut skipped = 0;
            for line in content.lines() {
                let url = line.trim();
                if url.is_empty() || url.starts_with('#') {
                    continue;
                }
                match db.add_feed(url) {
                    Ok(_) => added += 1,
                    Err(_) => skipped += 1,
                }
            }
            println!("Done! Feeds added: {}, skipped (already exist): {}", added, skipped);
        }
        Some(cmd) => {
            eprintln!("Unknown command: {}", cmd);
            print_help();
            std::process::exit(1);
        }
    }
    Ok(())
}
