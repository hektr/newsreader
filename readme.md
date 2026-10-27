# Newsreader CLI & TUI 

A simple and command-line RSS reader featuring a full Terminal User Interface (TUI).

---

## 🚀 Usage

Run the tool directly from your terminal:

```bash
newsreader            # Run the interactive TUI interface
newsreader [COMMAND]  # Run a specific CLI command
```

---

## Commands

| Command | Description |
| :--- | :--- |
| `add <url>` | Add a new RSS feed |
| `remove <id>` | Remove a feed by its ID |
| `list` | Show the list of all added feeds |
| `fetch` | Update all feeds and download new articles |
| `fetch <url>` | Update or add a specific URL |
| `fetch <id>` | Update a specific feed by its ID |
| `fetch last` | Update only the last added feed |
| `import <file.txt>` | Mass-import feeds from a text file (one URL per line) |
| `-h, --help` | Print the help message |

---

## TUI Shortcuts

When running the interactive TUI mode, use the following keys for navigation:

* **`↑ / ↓` or `j / k`** — Navigate the list or scroll through the article text.
* **`Enter` or `→`** — Open and read the selected article.
* **`Esc` or `←`** — Return to the article list.
* **`/`** — Search articles by title.
* **`f`** — Filter articles by a specific source feed.
* **`a`** — Show all feeds (reset the active source filter).
* **`q` or `Esc`** — Exit the program.
* **`F1` or `?`** — Show the built-in help guide inside the TUI.

---

##  Examples

**a) Add a new RSS feed:**
```bash
./newsreader add https://phys.org/rss-feed/
```

**b) Fetch and download all new articles:**
```bash
./newsreader fetch
```

**c) Launch the interactive terminal interface:**
```bash
./newsreader
```

---

## Some Interesting RSS Feeds

Here are some recommended science feeds you can add to your `newsreader` right away:

### Science
* **Phys.org Latest News**
  ```bash
  ./newsreader add https://phys.org/rss-feed/
  ```
* **ScienceDaily Top Science News**
  ```bash
  ./newsreader add https://www.sciencedaily.com/rss/top.xml
  ```
