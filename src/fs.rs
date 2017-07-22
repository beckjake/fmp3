use std::io;
use std::fs::{ReadDir, read_dir};
use std::path::{Path, PathBuf};

#[derive(Debug)]
struct ReadFlacDir {
    reader: Option<ReadDir>,
}

impl Iterator for ReadFlacDir {
    type Item = io::Result<PathBuf>;
    fn next(&mut self) -> Option<Self::Item> {
        debug!("in ReadFlacDir.next() for {:?}", self.reader);
        let mut reader = match self.reader {
            Some(ref mut r) => r,
            None => return None,
        };
        loop {
            match reader.next() {
                Some(r) => {
                    match r {
                        Ok(f) => {
                            let path = f.path();
                            if is_flac(&path) {
                                return Some(Ok(path));
                            }
                        }
                        Err(e) => return Some(Err(e)),
                    }
                }
                None => return None,
            };
        }
    }
}

#[derive(Debug)]
pub struct MultiReadFlacDir {
    directories: Vec<PathBuf>,
    current_dir: ReadFlacDir,
}


impl MultiReadFlacDir {
    pub fn new(directories: Vec<PathBuf>) -> Self {
        MultiReadFlacDir {
            directories: directories,
            current_dir: ReadFlacDir { reader: None },
        }
    }
    fn next_current_dir(&mut self) -> Option<io::Result<ReadFlacDir>> {
        debug!("atttempting to find next current directory on multireader");
        loop {
            let candidate = match self.directories.pop() {
                Some(d) => d,
                // indicate end of iterator
                None => {
                    debug!("iterator complete on multireader");
                    return None;
                },
            };
            // no need to emit an error
            if !candidate.is_dir() {
                println!("{:?}", candidate);
                debug!("found non-directory: {:?}", candidate);
                continue;
            }
            debug!("looking in {:?} on multireader", candidate);
            return match read_dir(candidate) {
                Ok(r) => Some(Ok(ReadFlacDir { reader: Some(r) })),
                Err(e) => Some(Err(e)),
            };
        }
    }
}


impl Iterator for MultiReadFlacDir {
    type Item = io::Result<PathBuf>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(c) = self.current_dir.next() {
                return Some(c);
            }
            self.current_dir = match self.next_current_dir() {
                Some(maybe_dir) => {
                    debug!("Got maybe_dir={:?}", maybe_dir);
                    match maybe_dir {
                        Ok(d) => d,
                        Err(e) => return Some(Err(e)),
                    }
                }
                None => return None,
            };
        }
    }
}


fn is_flac<P>(path: P) -> bool
    where P: AsRef<Path>
{
    let path = path.as_ref();
    if !path.is_file() {
        return false;
    }
    match path.extension() {
        Some(ext) => ext == "flac",
        None => false,
    }
}
