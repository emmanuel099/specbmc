#[macro_use]
extern crate clap;
use clap::Arg;
use std::path::Path;

fn main() {
    let matches = app_from_crate!()
        .arg(
            Arg::with_name("FILE")
                .help("Input file to use")
                .required(true)
                .index(1),
        )
        .get_matches();

    let _filename = Path::new(matches.value_of("FILE").unwrap());
}
