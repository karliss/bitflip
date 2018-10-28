#[macro_use]
extern crate clap;

extern crate termion;

use clap::{App, Arg, ArgMatches};
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use termion::{color, style};

use bytegrid::{ByteGrid, ByteGridDiff};
use encoding::Encoding;

mod bytegrid;
mod encoding;
mod resource;

fn run_diff(args: &ArgMatches) -> Result<(), ()> {
    let before_name = args.value_of("before").unwrap();
    let after_name = args.value_of("after").unwrap();
    let encoding = Encoding::get_encoding("437").map_err(|e| {
        eprintln!("Could not load encoding {:?}", e);
        ()
    })?;
    let bytes_before = ByteGrid::load(Path::new(before_name), &encoding).map_err(|e| {
        eprintln!("Could not load map {}: {:?}", before_name, e);
        ()
    })?;
    let bytes_after = ByteGrid::load(Path::new(after_name), &encoding).map_err(|e| {
        eprintln!("Could not load map {}: {:?}", after_name, e);
        ()
    })?;
    let diff = bytes_before.diff(&bytes_after).serialize();
    if let Some(path) = args.value_of("output") {
        File::create(Path::new(path))
            .and_then(|mut out| out.write(&diff))
            .map_err(|e| {
                eprint!("Output error {}", e);
                ()
            })?;
    } else {
        std::io::stdout().write(&diff).map_err(|_| ())?;
    }
    Ok(())
}

fn run_patch(args: &ArgMatches) -> Result<(), ()> {
    let before_name = args.value_of("data").unwrap();
    let patch = args.value_of("patch").unwrap();
    let output = args.value_of("output");
    let encoding = Encoding::get_encoding("437").map_err(|e| {
        eprintln!("Could not load encoding {:?}", e);
        ()
    })?;
    let mut bytes_before = ByteGrid::load(Path::new(before_name), &encoding).map_err(|e| {
        eprintln!("Could not load map {}: {:?}", before_name, e);
        ()
    })?;
    let patch = std::fs::read(patch)
        .map_err(|e| {
            eprintln!("Could not read patch: {}", e);
            ()
        })
        .and_then(|data| ByteGridDiff::deserialize(&data))
        .map_err(|_| {
            eprintln!("Could not decode patch");
            ()
        })?;
    bytes_before.patch(&patch);
    let write_result = if let Some(output_path) = output {
        File::create(Path::new(&output_path))
            .and_then(|mut f| bytes_before.save(&mut f, &encoding))
    } else {
        bytes_before.save(&mut std::io::stdout(), &encoding)
    };
    write_result.map_err(|e| {
        eprintln!("Write error: {}", e);
        return ()
    })?;
    Ok(())
}

fn main() {
    let matches = App::new("ethdec")
        .version(crate_version!())
        .author("Kārlis Seņko <karlis3p70l1ij@gmail.com>")
        .about("Binary bit flip game heavily based on \"Rogue Bit\"")
        .arg(Arg::with_name("encoding").takes_value(true))
        .subcommand(
            clap::SubCommand::with_name("diff")
                .about("Diff two images")
                .arg(Arg::with_name("before"))
                .arg(Arg::with_name("after"))
                .arg(Arg::with_name("output").short("o").takes_value(true)),
        )
        .subcommand(
            clap::SubCommand::with_name("patch")
                .about("Diff two images")
                .arg(Arg::with_name("data"))
                .arg(Arg::with_name("patch"))
                .arg(Arg::with_name("output").short("o").takes_value(true)),
        )
        .get_matches();

    let result = match matches.subcommand() {
        ("diff", Some(m)) => run_diff(m),
        ("patch", Some(m)) => run_patch(m),
        _ => Err(()),
    };
    ::std::process::exit(match result {
        Ok(_) => 0,
        Err(_) => 1,
    });
}
