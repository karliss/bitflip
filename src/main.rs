#[macro_use]
extern crate clap;

extern crate termion;

use clap::{App, Arg};

mod bytegrid;
mod encoding;
mod resource;

fn main() {
    let matches = App::new("ethdec")
        .version(crate_version!())
        .author("Kārlis Seņko <karlis3p70l1ij@gmail.com>")
        .about("Binary bit flip game heavily based on \"Rogue Bit\"")
        .arg(Arg::with_name("encoding").takes_value(true))
        .subcommand(
            clap::SubCommand::with_name("diff")
                .about("Diff two images")
                .arg(Arg::with_name("before")),
        )
        .arg(Arg::with_name("after"))
        .arg(Arg::with_name("output").short("o").takes_value(true))
        .get_matches();
}
