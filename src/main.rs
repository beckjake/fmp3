
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;


extern crate duct;
extern crate env_logger;
extern crate num_cpus;
extern crate metaflac;
extern crate id3;
extern crate toml;
extern crate scoped_pool;

use std::fs::{File, remove_file};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;

use clap::Arg;
use scoped_pool::Pool;

mod error;
pub use error::{Result, Error, ErrorKind};

mod fs;

mod tags;
pub use tags::flac_to_mp3;

fn make_command<'a>(cmdline: &Vec<String>, filepath: &'a Path) -> duct::Expression {
    if cmdline.len() == 0 {
        panic!("Invalid cmdline, no args!");
    }
    // if we can't convert the filepath into a strng I'm ok with panic'ing.
    let pathstr = filepath.to_str().unwrap();
    let exe = &cmdline[0];
    let args = cmdline.iter().skip(1).map(|c| {
        if c == "{{}}" {
            "{}"
        } else if c == "{}" {
            pathstr
        } else {
            c
        }
    });
    duct::cmd(exe, args)
}


#[derive(Deserialize, Debug, Clone)]
struct Converter {
    flac_command: Vec<String>,
    mp3_command: Vec<String>,
    #[serde(default)]
    remove_after: bool,
    #[serde(default)]
    overwrite: bool,
    #[serde(default)]
    workers: usize,
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

    fn convert_file_only(&self, flac_path: &Path, mp3_path: &Path) -> Result<()> {
        debug!("Starting flac conversion for {:?}", flac_path);
        if mp3_path.exists() && !self.overwrite {
            debug!("Skipping destination file {:?}, it exists", mp3_path);
            return Err(Error::from_kind(ErrorKind::PathExists(mp3_path.to_path_buf())));
        }
        let flac_proc = make_command(&self.flac_command, flac_path);
        let mp3_proc = make_command(&self.mp3_command, mp3_path);
        debug!("Starting mp3 conversion for {:?}", mp3_path);
        let result = flac_proc.pipe(mp3_proc).read()?;
        debug!("Got mp3 status for {:?}: {:?}", mp3_path, result);
        Ok(())
    }

    fn convert_tags(&self, flac_path: &Path, mp3_path: &Path) -> Result<()> {
        debug!("converting tags");
        let ret = flac_to_mp3(flac_path, mp3_path);
        debug!("converted tags");
        ret
    }
    fn convert_leave_both<P>(&self, flac_path: P) -> Result<()>
        where P: AsRef<Path>
    {
        let flac_path = flac_path.as_ref();
        let mp3_path = &flac_path.with_extension("mp3");
        info!("converting file {:?} to {:?}", flac_path, mp3_path);
        self.convert_file_only(flac_path, mp3_path)?;
        self.convert_tags(flac_path, mp3_path)?;
        Ok(())
    }
    fn convert<P>(&self, flac_path: P) -> Result<()>
        where P: AsRef<Path>
    {
        let flac_path = flac_path.as_ref();
        info!("Starting conversion process for {:?}", flac_path);
        self.convert_leave_both(flac_path)?;
        if self.remove_after {
            remove_file(flac_path)?;
        }
        debug!("Finished conversion process for {:?}", flac_path);
        Ok(())
    }

    fn convert_directories_serial(&self, directories: Vec<PathBuf>) -> Vec<Error> {
        let mut errors = Vec::new();
        for flac in fs::MultiReadFlacDir::new(directories) {
            match flac {
                Ok(path) => {
                    if let Err(e) = self.convert(&path) {
                        errors.push(e);
                    }
                }
                Err(e) => errors.push(Error::from(e)),
            };
        }
        errors
    }

    fn convert_directories_parallel(&self, directories: Vec<PathBuf>) -> Vec<Error> {
        let pool = Pool::new(self.workers);
        let (sender, receiver) = channel();
        pool.scoped(|scope| {
            for flac in fs::MultiReadFlacDir::new(directories) {
                match flac {
                    // if these sends don't work, it's pretty reasonable to panic
                    Ok(path) => {
                        let sender = sender.clone();
                        let path = path.clone();
                        scope.execute(move || {
                            sender.send(match self.convert(&path) {
                                    Ok(_) => None,
                                    Err(e) => Some(Error::from(e)),
                                })
                                .unwrap();
                        });
                    }
                    Err(e) => {
                        trace!("Reading directory got error {:?}", e);
                        sender.send(Some(Error::from(e))).unwrap();
                    }
                }
            }
            drop(sender);
        });
        trace!("Completed loop");
        let r = receiver.iter().filter_map(|x| x).collect();
        trace!("Finished waiting");
        r
    }

    fn convert_directories(&self, directories: Vec<PathBuf>) -> Vec<Error> {
        debug!("what?");
        if self.workers == 0 {
            return vec![Error::from_kind(ErrorKind::BadWorkers)];
        } else if self.workers == 1 {
            self.convert_directories_serial(directories)
        } else {
            self.convert_directories_parallel(directories)
        }
    }
}



fn parse_args() -> Result<(Converter, Vec<PathBuf>)> {
    let app = app_from_crate!()
        .about("Converts FLAC files to mp3s (using available command line tools) and then \
                rewrites their tags")
        .arg(Arg::with_name("remove")
            .help("If set, remove flac files after conversion (dangerous!)")
            .long("remove"))
        .arg(Arg::with_name("no remove")
            .help("If set, negates a remove setting in the config file")
            .long("no-remove")
            .conflicts_with("remove"))
        .arg(Arg::with_name("overwrite")
            .help("If set, overwrites existing mp3 files (obviously dangerous!)")
            .long("overwrite"))
        .arg(Arg::with_name("no overwrite")
            .help("If set, negates an overwrite setting in the config file")
            .long("no-overwrite")
            .conflicts_with("overwrite"))
        .arg(Arg::with_name("workers")
            .help("The number of workers to use, i.e. 6. Defaults to 1. Set 0 for the number of \
                   CPUs.")
            .long("num-workers")
            .short("j")
            .takes_value(true))
        .arg(Arg::with_name("config")
            .help("The config file to get commands from, if provided")
            .long("config")
            .short("c")
            .takes_value(true))
        .arg(Arg::with_name("DIRECTORIES")
            .help("Specifies the directories to search")
            .required(true)
            .multiple(true)
            .index(1))
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
    match value_t!(app, "workers", usize) {
        Ok(0) => converter.workers = num_cpus::get(),
        Ok(w) => converter.workers = w,
        Err(ref e) if e.kind == clap::ErrorKind::ArgumentNotFound => converter.workers = 1,
        Err(e) => return Err(Error::from(e)),
    };
    // This is stupid, if I want the list of paths I have to do it this way...
    let paths: Vec<PathBuf> = match app.values_of("DIRECTORIES") {
        Some(v) => v.map(|s| Path::new(s).to_path_buf()).collect(),
        None => vec![],
    };
    Ok((converter, paths))
}


fn main() {
    env_logger::init().unwrap();
    let (converter, paths) = parse_args().expect("Parsing argument failed");
    let errors = converter.convert_directories(paths);
    if errors.len() > 0 {
        for e in errors {
            error!("{:?}", e);
        }
    }
}
