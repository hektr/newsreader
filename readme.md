**USAGE:**  
  newsreader              Run the TUI interface
  newsreader [COMMAND]    Run a specific command                                                                                                                   
                                                                                                                                                                   
**COMMANDS:**                                                                                                                                                            
  add <url>               Add a new feed                                                                                                                           
  remove <id>             Remove a feed by its ID                                                                                                                  
  list                    Show the list of added feeds                                                                                                             
  fetch                   Update all feeds and download new articles                                                                                               
  fetch <url>             Update or add a specific URL                                                                                                             
  fetch <id>              Update feed by ID                                                                                                                  
  fetch last              Update only the last added feed                                                                                                          
  import <file.txt>       Add feeds from a text file (one URL per line)                                                                                            
  -h, --help              Print this help message                                                                                                                  

**TUI SHORTCUTS:**  
  ↑/↓ or j/k              Navigate list, scroll text
  Enter or →              Open article
  Esc or ←                Return to list
  /                       Search by title
  f                       Select feed (filter by source)
  a                       Show all feeds (reset filter)
  q or Esc                Exit program
  F1 or ?                 Show help inside TUI

**Example:** 
a) Add rss feed:  
./newsreader add https://habr.com/ru/rss/all

b) Get all articles:  
./newsreader fetch

c) Run TUI
./newsreader
