use std::io::{stdin, BufRead};

use firefox_rs::list_tabs;

fn main() -> anyhow::Result<()> {
    let mut tabs = list_tabs()?;
    if tabs.is_empty() {
        eprintln!("No open tabs");
        return Ok(());
    }
    for (i, tab) in tabs.iter().enumerate() {
        let title = &tab.title;
        eprintln!("[{i: >2}] - {title}");
    }
    eprint!("\n> ");
    for input in stdin().lock().lines() {
        let line = input?;
        if let Ok(idx) = line.parse::<usize>() {
            if idx < tabs.len() {
                let tab = tabs.swap_remove(idx);
                let title = &tab.title;
                eprintln!("Focusing {title}");
                tab.focus()?;
                return Ok(());
            }
        }
        let hi = tabs.len() - 1;
        eprintln!("Insert valid option [0, {hi}]");
        eprint!("> ");
    }
    Ok(())
}
