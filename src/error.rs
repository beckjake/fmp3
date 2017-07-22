use std::path::PathBuf;

error_chain!{
    foreign_links {
        Id3(::id3::Error);
        Flac(::metaflac::Error);
        Toml(::toml::de::Error);
        Io(::std::io::Error) #[cfg(unix)];
    }
    errors {
        PathExists(path: PathBuf) {
            description("The path to be written to exists already")
            display("The path at {:?} already exists", path)
        }
    }
}
