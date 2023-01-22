# judge-prototype
judge prototype with rocket rust 

1. install rust: https://www.rust-lang.org/tools/install
2. `git clone` this repository
3. go to this directory with terminal and run `cargo run`

## description
after pressing submit it creates a text file containing the code and a json file with some metadata in folder `/submissions`

all the code is in `src/main.rs` and the htmls are generated by templates in `/template` with Tera template engine

## capabilities
can do login (with gmail) and cookies
can add a task creation page to upload markdown files
https://tabulator.info/examples/5.4#sparklines
