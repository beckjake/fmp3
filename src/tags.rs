extern crate metaflac;
extern crate id3;

use std::path::Path;
use ::Result;

trait TagGettersSetters {
    fn get_artist(&self) -> Option<&str>;
    fn get_album(&self) -> Option<&str>;
    fn get_album_artist(&self) -> Option<&str>;
    fn get_number(&self) -> Option<u32>;
    fn get_title(&self) -> Option<&str>;
    fn get_genre(&self) -> Option<&str>;
    fn set_artist<T: Into<String>>(&mut self, artist: T);
    fn set_album<T: Into<String>>(&mut self, album: T);
    fn set_album_artist<T: Into<String>>(&mut self, album_artist: T);
    fn set_number(&mut self, number: u32);
    fn set_title<T: Into<String>>(&mut self, title: T);
    fn set_genre<T: Into<String>>(&mut self, genre: T);
}

impl TagGettersSetters for id3::Tag {
    fn get_artist(&self) -> Option<&str> {
        self.artist()
    }
    fn get_album(&self) -> Option<&str> {
        self.album()
    }
    fn get_album_artist(&self) -> Option<&str> {
        self.album_artist()
    }
    fn get_number(&self) -> Option<u32> {
        self.track()
    }
    fn get_genre(&self) -> Option<&str> {
        self.genre()
    }
    fn get_title(&self) -> Option<&str> {
        self.title()
    }
    fn set_artist<T: Into<String>>(&mut self, artist: T) {
        self.set_artist(artist);
    }
    fn set_album<T: Into<String>>(&mut self, album: T) {
        self.set_album(album);
    }
    fn set_album_artist<T: Into<String>>(&mut self, album_artist: T) {
        self.set_album_artist(album_artist);
    }
    fn set_number(&mut self, number: u32) {
        self.set_track(number);
    }
    fn set_title<T: Into<String>>(&mut self, title: T) {
        self.set_title(title);
    }
    fn set_genre<T: Into<String>>(&mut self, genre: T) {
        self.set_genre(genre);
    }
}

macro_rules! firstref {
    ($x:expr) => ($x.and_then(|v| v.get(0).and_then(|a| Some(a.as_str()))))
}

impl TagGettersSetters for metaflac::Tag {
    fn get_artist(&self) -> Option<&str> {
        firstref!(self.get_vorbis("artist"))
    }
    fn get_album(&self) -> Option<&str> {
        firstref!(self.get_vorbis("album"))
    }
    fn get_album_artist(&self) -> Option<&str> {
        firstref!(self.get_vorbis("albumartist"))
    }
    fn get_number(&self) -> Option<u32> {
        self.get_vorbis("tracknumber").and_then(|v| v.get(0).and_then(|t| {
            match t.parse::<u32>() {
                Ok(n) => Some(n),
                Err(_) => None
            }
        }))
    }
    fn get_genre(&self) -> Option<&str> {
        firstref!(self.get_vorbis("genre"))
    }
    fn get_title(&self) -> Option<&str> {
        firstref!(self.get_vorbis("title"))
    }
    fn set_artist<T: Into<String>>(&mut self, artist: T) {
        self.set_vorbis("artist", vec![artist]);
    }
    fn set_album<T: Into<String>>(&mut self, album: T) {
        self.set_vorbis("album", vec![album]);
    }
    fn set_album_artist<T: Into<String>>(&mut self, album_artist: T) {
        self.set_vorbis("albumartist", vec![album_artist]);
    }
    fn set_number(&mut self, number: u32) {
        self.set_vorbis("tracknumber", vec![number.to_string()])
    }
    fn set_title<T: Into<String>>(&mut self, title: T) {
        self.set_vorbis("title", vec![title]);
    }
    fn set_genre<T: Into<String>>(&mut self, genre: T) {
        self.set_vorbis("genre", vec![genre]);
    }
}


// both files should exist, but the MP3 file should not have a tag.
// TODO: how does one test this? Even testing components seems hard
pub fn flac_to_mp3(flac_path: &Path, mp3_path: &Path) -> Result<()> {
    let flac_metadata = metaflac::Tag::read_from_path(flac_path)?;
    let mut id3_tag = id3::Tag::new();
    if let Some(artist) = flac_metadata.get_artist() {
        id3_tag.set_artist(artist);
    }
    if let Some(album) = flac_metadata.get_album() {
        id3_tag.set_album(album);
    }
    if let Some(album_artist) = flac_metadata.get_album_artist() {
        id3_tag.set_album_artist(album_artist);
    }
    if let Some(genre) = flac_metadata.get_genre() {
        id3_tag.set_genre(genre);
    }
    if let Some(title) = flac_metadata.get_title() {
        id3_tag.set_title(title);
    }
    if let Some(number) = flac_metadata.get_number() {
        id3_tag.set_number(number);
    }
    // TODO: new versions (git master) of id3 crate changed this
    id3_tag.write_to_path(mp3_path)?;
    Ok(())
}
