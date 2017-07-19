#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate serde_derive;

extern crate metaflac;
extern crate id3;
extern crate clap;
extern crate toml;

use clap::{App, Arg};
use std::fs::{File, remove_file, read_dir};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::os::unix::io::{IntoRawFd, FromRawFd};


mod error;
pub use error::{Result, Error, ErrorKind};

mod tags;
pub use tags::flac_to_mp3;

fn make_command<'a>(cmdline: &Vec<String>, filepath: &'a Path) -> Command {
    if cmdline.len() == 0 {
        panic!("Invalid cmdline, no args!");
    }
    // if we can't convert the filepath into a string I'm ok with panic'ing.
    let pathstr = filepath.to_str().unwrap();
    let mut cmd = Command::new(&cmdline[0]);
    cmd.args(cmdline.iter().skip(1).map(|c| {
            if c == "{{}}" {"{}"}
            else if c == "{}" {pathstr}
            else {c}
        }));
    cmd
}



// #[derive(Debug, Deserialize)]
#[derive(Deserialize)]
struct Converter {
    flac_command: Vec<String>,
    mp3_command: Vec<String>,
    #[serde(default)]
    remove_after: bool,
    #[serde(default)]
    overwrite: bool,
}

fn compute_mp3_path(flac_path: &Path) -> PathBuf {
    flac_path.with_extension("mp3")
}

impl Converter {
    fn new_from_str(data: &str) -> Result<Self> {
        let converter = toml::from_str(data)?;
        Ok(converter)
    }
    // read in from a TOML config
    fn new_from_file(path: &Path) -> Result<Self> {
        let mut f = File::open(path)?;
        let mut data = String::new();
        f.read_to_string(&mut data)?;
        Self::new_from_str(&data)
    }

    fn new_default() -> Result<Self> {
        let data = r#"
        flac_command = ["flac", "-cd", "{}"]
        mp3_command = ["lame", "-V0", "-", "{}"]
        "#;
        Self::new_from_str(&data)
    }

    fn convert_file_only(&self, flac_path: &Path) -> Result<()> {
        let mp3_path = compute_mp3_path(flac_path);
        if mp3_path.exists() && !self.overwrite {
            return Err(Error::from_kind(ErrorKind::PathExists(mp3_path.to_path_buf())));
        }
        let flac_proc = make_command(&self.flac_command, flac_path).stdout(Stdio::piped()).spawn()?;
        let flac_stdout = flac_proc.stdout.unwrap();
        let mp3_stdin = unsafe { Stdio::from_raw_fd(flac_stdout.into_raw_fd())};
        make_command(&self.mp3_command, &mp3_path).stdin(mp3_stdin).status()?;
        Ok(())
    }

    fn convert_tags(&self, flac_path: &Path) -> Result<()> {
        let mp3_path = compute_mp3_path(flac_path);
        flac_to_mp3(flac_path, &mp3_path)
    }
    fn convert_leave_both(&self, flac_path: &Path) -> Result<()> {
        self.convert_file_only(flac_path)?;
        self.convert_tags(flac_path)?;
        Ok(())
    }
    fn convert(&self, flac_path: &Path) -> Result<()> {
        self.convert_leave_both(flac_path)?;
        if self.remove_after {
            remove_file(flac_path)?;
        }
        Ok(())
    }
    fn convert_directory(&self, root: &Path) -> Result<()> {
        for entry in read_dir(&Path::new(root))? {
            let path = entry?.path();
            if !path.is_file() {
                continue;
            }
            if let Some(ext) = path.extension() {
                if ext != "flac" {
                    continue
                }
            } else {
                continue
            }
            self.convert(&path)?;
        }
        Ok(())
    }
}


fn parse_args() -> Result<(Converter, Vec<PathBuf>)>{
    let app = App::new("flac-to-mp3")
        .author("Jacob Eldergill Beck <jacob@ebeck.io>")
        .about("Converts FLAC files to mp3s (using available command line tools) and then rewrites their tags")
        .arg(Arg::with_name("remove")
            .help("If set, remove flac files after conversion (dangerous!)")
            .long("remove")
        )
        .arg(Arg::with_name("no remove")
            .help("If set, negates a remove setting in the config file")
            .long("no-remove")
            .conflicts_with("remove")
        )
        .arg(Arg::with_name("overwrite")
            .help("If set, overwrites existing mp3 files (obviously dangerous!)")
            .long("overwrite")
        )
        .arg(Arg::with_name("no overwrite")
            .help("If set, negates an overwrite setting in the config file")
            .long("no-overwrite")
            .conflicts_with("overwrite")
        )
        .arg(Arg::with_name("config")
            .help("The config file to get commands from, if provided")
            .long("config")
            .short("c")
            .takes_value(true)
        )
        .arg(Arg::with_name("DIRECTORIES")
            .help("Specifies the directories to search")
            .required(true)
            .multiple(true)
            .index(1)
        )
        .get_matches();
    let mut converter = match app.value_of("config") {
        Some(c) => Converter::new_from_file(Path::new(c)),
        None => Converter::new_default(),
    }?;

    if app.is_present("remove") {
        converter.remove_after = true;
    } else if app.is_present("no remove") {
        converter.remove_after = false;
    }
    if app.is_present("overwrite") {
        converter.overwrite = true;
    } else if app.is_present("no overwrite") {
        converter.remove_after = false;
    }
    // This is stupid, if I want the list of paths I have to do it this way...
    let paths: Vec<PathBuf> = match app.values_of("DIRECTORIES") {
        Some(v) => v.map(|s| Path::new(s).to_path_buf()).collect(),
        None => vec![],
    };
    Ok((converter, paths))
}

fn main() {
    let (converter, paths) = parse_args().unwrap();
    for arg in paths {
        if arg.is_dir() {
            converter.convert_directory(&arg).unwrap();
        }
    }
}
