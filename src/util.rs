use std::{
    fs::{read_dir, File},
    path::{Path, PathBuf}, io::Read,
};

use crate::{Error, FFResult};

enum ErrorOrIterator<E, I> {
    Error(Option<E>),
    Iter(I),
}

impl<T, E, I> Iterator for ErrorOrIterator<E, I>
where
    I: Iterator<Item = Result<T, E>>,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            ErrorOrIterator::Error(opt) => opt.take().map(Err),
            ErrorOrIterator::Iter(i) => i.next(),
        }
    }
}

fn list_recovery_files_inner(
    override_home: impl AsRef<Path>,
) -> FFResult<impl Iterator<Item = Result<PathBuf, std::io::Error>>> {
    // similar to (?), but returns ErrorOrIterator::Error on error
    macro_rules! try_eoi {
        ($result:expr) => {
            match $result {
                Ok(ok) => ok,
                Err(e) => return Some(ErrorOrIterator::Error(Some(e))),
            }
        };
    }

    // similar to (?), but returns error inside an option (needed for filter_map)
    macro_rules! try_some {
        ($result:expr) => {
            match $result {
                Ok(ok) => ok,
                Err(e) => return Some(Err(e)),
            }
        };
    }

    let firefox_root = override_home.as_ref().join(".mozilla/firefox");
    // top level result (fails if firefox_root does not exist / is not accessible)
    let iter = read_dir(firefox_root)?
        .filter_map(|entry_res| {
            // try_eoi! will exit closure with error in case of file system changes
            // while transversing
            let entry = try_eoi!(entry_res);
            let is_default_dir = try_eoi!(entry.file_type()).is_dir()
                && entry
                    .file_name()
                    .to_str()
                    .map(|s| s.contains("default"))
                    .unwrap_or_default();

            // if not *default* dir, we are not interested
            if !is_default_dir {
                return None;
            }

            // information should be inside sessionstore-backups
            let backups = entry.path().join("sessionstore-backups");

            // it is possible (_acceptable_) that sessionstore-backups does not exist in a folder with
            // *default* in the name; that's why it is not a hard error, but it's skipped: .ok()?
            let iter = read_dir(backups).ok()?.filter_map(|entry_res| {
                // this can also be considered transversal error
                let entry = try_some!(entry_res);
                let is_recovery_file = try_some!(entry.file_type()).is_file()
                    && entry
                        .file_name()
                        .to_str()
                        .map(|s| s.starts_with("recovery.js"))
                        .unwrap_or_default();

                // if filename starts with recovery.js, return as possible path
                is_recovery_file.then(|| Ok(entry.path()))
            });
            Some(ErrorOrIterator::Iter(iter))
        })
        // iterator of iterators, we only want paths
        .flatten();
    Ok(iter)
}

/// Returns iterator of viable recovery files
///
/// - Ok(Iterator<Result<PathBuf>>): each element of the iterator could fail in case file system changes occur while transversing
/// - Err(_): in case mozila data dir is not found / is not accessible
pub fn list_recovery_files() -> FFResult<impl Iterator<Item = Result<PathBuf, std::io::Error>>> {
    let home_dir = dirs::home_dir().ok_or_else(|| Error::FFDirNotFound("home"))?;
    list_recovery_files_inner(home_dir)
}

pub fn decompress_lz4(p: impl AsRef<Path>) -> FFResult<Vec<u8>> {
    let mut f = File::open(p)?;
    let mut buf = vec![];
    f.read_to_end(&mut buf)?;
    Ok(lz4_flex::decompress_size_prepended(&buf[8..])?)
}


#[cfg(test)]
mod test {
    use std::{path::PathBuf, str::FromStr};

    use super::list_recovery_files_inner;

    #[test]
    fn list_recovery() {
        let files: Vec<_> =list_recovery_files_inner("assets/test")
            .unwrap()
            .collect();
        let expected = ["assets/test/.mozilla/firefox/5w5airb6.default-release/sessionstore-backups/recovery.jsonlz4"]; 
        let fullmatch = files.iter()
            .zip(&expected)
            .all(|(listed, expected)| listed.as_ref().unwrap() == &PathBuf::from_str(expected).unwrap());
        eprintln!("expected:{expected:?}");
        eprintln!("files:{files:?}");
        assert!(fullmatch);
    }
}
