FMP3
====

A little toy written in Rust, using jameshurst's excellent
[rust-id3](https://github.com/jameshurst/rust-id3/) and
[rust-metaflac](https://github.com/jameshurst/rust-metaflac) libraries to
perform the following operations:

    - Given a directory, find all the .flac files in it
    - Convert each flac to an mp3
    - Copy the tag data over
    - Remove the flac files

It uses lame + flac by default (and assumes they are in your PATH), but you can
always implement your own converters, pick different bitrates, etc by writing a
config. Something like this in a file you pass to config should be sufficient:

```
flac_command = ["flac", "-cd", "{}"]
mp3_command = ["lame", "-V0", "-", "{}"]
```

The command argument parser only replaces a single standalone `{}`. I guess file
an issue/submit a PR if you want to fix that, but so far it's been fine for me.
If you need a single literal `{}` as an argument for whatever reason, use `{{}}`
in its place.

Why?
====
I wrote this mostly to play with Rust a bit more, but also to convert my music
that I get from bandcamp.com from FLAC to mp3 for my media player.

TODO
====

- tests if I ever decide I care about that for this project
- date -> year
- ???

Usage
=====

```
USAGE:
    fmp3 [FLAGS] [OPTIONS] <DIRECTORIES>...

FLAGS:
    -h, --help            Prints help information
        --no-overwrite    If set, negates an overwrite setting in the config file
        --no-remove       If set, negates a remove setting in the config file
        --overwrite       If set, overwrites existing mp3 files (obviously dangerous!)
        --remove          If set, remove flac files after conversion (dangerous!)
    -V, --version         Prints version information

OPTIONS:
    -c, --config <config>    The config file to get commands from, if provided

ARGS:
    <DIRECTORIES>...    Specifies the directories to search
```