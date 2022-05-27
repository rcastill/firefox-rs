mod util;


use lz4_flex::block::DecompressError;
use util::{decompress_lz4, list_recovery_files};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Firefox data dir not found: {0}")]
    FFDirNotFound(&'static str),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("lz4 error: {0}")]
    Lz4Decompression(#[from] DecompressError),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("multiple errors: {0}")]
    Multi(String),
}

// Firefox Result
pub type FFResult<T> = Result<T, Error>;

mod recovery {
    use serde::Deserialize;

    #[derive(Deserialize, Debug)]
    pub struct TopLevel {
        pub windows: Vec<Window>,
    }

    #[derive(Deserialize, Debug)]
    pub struct Window {
        pub tabs: Vec<Tab>,
    }

    #[derive(Deserialize, Debug)]
    pub struct Tab {
        entries: Vec<Entry>,
        index: usize,
    }

    impl Tab {
        pub fn into_entry(mut self) -> Entry {
            self.entries.swap_remove(self.index - 1)
        }
    }

    #[derive(Deserialize, Debug)]
    pub struct Entry {
        pub title: String,
        pub url: String,
    }
}

#[derive(Debug)]
pub struct Tab {
    title: String,
    url: String,
}

impl From<recovery::Entry>  for  Tab {
    fn from(e: recovery::Entry) -> Self {
        Tab{title:e.title,url:e.url}
    }
}

impl Tab {
    pub fn focus(&self) -> FFResult<()> {
        todo!()
    }
}

pub fn list_tabs() -> FFResult<Vec<Tab>> {
    let mut errors = Vec::with_capacity(0);
    // in case of multi error; add errors accumulated in iterations to error vec
    macro_rules! try_add {
        ($result:expr) => {
            match $result {
                Ok(ok) => ok,
                Err(e) => {
                    errors.push(Error::from(e));
                    continue;
                }
            }
        };
    }
    for path_res in list_recovery_files()? {
        let path = path_res?;

        // decompression and deserialization are errors that cause to skip this path
        // -- not causing to cancel list_tabs()
        let buf = try_add!(decompress_lz4(path));
        let topl: recovery::TopLevel = try_add!(serde_json::from_slice(&buf));

        // this should be error free
        // TODO: if index is out of bounds in recovery.json -- this crashes
        let tabs = topl
            .windows
            .into_iter()
            .flat_map(|window| window.tabs.into_iter().map(recovery::Tab::into_entry))
            .map(Tab::from)
            .collect();
        return Ok(tabs)
    }

    // TODO: is this really necessary? are there more than one "recovery.json" to worry about?
    match errors.len() {
        0 => Err(Error::FFDirNotFound("recovery.json*")),
        1 => Err(errors.swap_remove(0)),
        _ => Err(Error::Multi({
            let mut errors_s = String::new();
            for (i, e) in errors.into_iter().enumerate() {
                errors_s += &format!("({i}) {e} ");
            }
            errors_s
        }))
    }
}

#[cfg(test)]
mod tests {}
