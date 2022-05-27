use firefox_rs::{list_tabs, FFResult};

fn main() -> FFResult<()> {
    let tabs = list_tabs()?;
    eprintln!("{tabs:#?}");
    Ok(())
}
