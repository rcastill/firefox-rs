mod util;

use std::{env::temp_dir, fs::File, io::Write, process::Command};

use lz4_flex::block::DecompressError;
use util::{decompress_lz4, list_recovery_files};

/// Crate global errors
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Firefox data folder or needed files inside are not accessible
    #[error("Firefox data dir not found: {0}")]
    FFDirNotFound(&'static str),
    /// Std IO error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Failure to decompress lz4; e.g. recovery.json
    #[error("LZ4 error: {0}")]
    Lz4Decompression(#[from] DecompressError),
    /// Json ser/de error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    /// Composed error; e.g. if list_tabs() failed trying multiple recovery files
    #[error("Multiple errors: {0}")]
    Multi(String),
    #[error("Subcommand failed")]
    ExitStatus,
}

/// Firefox Result
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
        pub image: Option<String>,
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

/// Firefox tab representation
#[derive(Debug)]
pub struct Tab {
    /// Tab's title
    pub title: String,
    /// Tab's url
    pub url: String,
    /// Tab's icon
    pub icon: Option<String>,
}

impl From<recovery::Tab> for Tab {
    fn from(mut t: recovery::Tab) -> Self {
        let icon = t.image.take();
        let recovery::Entry { title, url } = t.into_entry();
        Tab { title, url, icon }
    }
}

impl Tab {
    /// Try to focus this tab using hack.
    ///
    /// Firefox extension: [focusTab](https://addons.mozilla.org/en-US/firefox/addon/focus_tab/) is required
    pub fn focus(&self) -> FFResult<()> {
        let hack = format!(
            "<!DOCTYPE html><body>\
            <script>window.focusTab({{url:'{}'}});\
            open(location, '_self').close();\
            </script></body></html>",
            self.url
        );
        let path = temp_dir().join("firefox-rs-focus.html");
        let mut f = File::create(&path)?;
        f.write_all(hack.as_bytes())?;
        let mut child = Command::new("firefox").arg(path).spawn()?;
        if !child.wait()?.success() {
            return Err(Error::ExitStatus);
        }
        Ok(())
    }
}

/// Returns list of tabs in open firefox instance
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
            .flat_map(|window| window.tabs)
            .map(Tab::from)
            .collect();
        return Ok(tabs);
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
        })),
    }
}

#[cfg(test)]
mod tests {}
