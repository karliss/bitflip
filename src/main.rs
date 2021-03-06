//TODO: enable dead_code check
#![allow(dead_code)]
#[macro_use]
extern crate clap;
#[macro_use]
extern crate serde_derive;

use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use clap::{App, Arg, ArgMatches};

use crate::bytegrid::{ByteGrid, ByteGridDiff};
use crate::encoding::Encoding;
use crate::game_ui::*;
use tgame::ui::*;

mod bytegrid;
mod encoding;
mod game_ui;
mod gameplay;
mod resource;
mod serde_rbbin;

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
        File::create(Path::new(&output_path)).and_then(|mut f| bytes_before.save(&mut f, &encoding))
    } else {
        bytes_before.save(&mut std::io::stdout(), &encoding)
    };
    write_result.map_err(|e| {
        eprintln!("Write error: {}", e);
        return ();
    })?;
    Ok(())
}

fn run_game(_args: &ArgMatches) -> Result<(), ()> {
    let mut stdout = std::io::stdout();
    {
        let mut context = UiContext::create(&stdout).ok_or_else(|| {
            eprintln!("failed to initialize terminal");
        })?;
        let mut menu = GameUi::new(&mut context);
        context.run(&mut menu)
    }
    .map_err(|e| {
        eprintln!("Error {:?}", e);
        ()
    })?;

    write!(
        stdout,
        "{}{}",
        ::termion::style::Reset,
        ::termion::cursor::Show
    )
    .map_err(|_| ())?;
    Ok(())
}

fn run_single_level(args: &ArgMatches) -> Result<(), ()> {
    let mut stdout = std::io::stdout();
    {
        let game_data = crate::gameplay::GamePlayState::load_from_path(Path::new(
            args.value_of(&"path").unwrap().into(),
        ))
        .map_err(|e| {
            eprintln!("{} ", e);
        })?; //TODO: error handling
        let mut context = UiContext::create(&stdout).ok_or(())?;

        let mut ui = GamePlayUI::new(&mut context);
        ui.set_state(game_data);
        context.run(&mut ui).map_err(|_| ())?;
    }

    write!(
        stdout,
        "{}{}",
        ::termion::style::Reset,
        ::termion::cursor::Show
    )
    .map_err(|_| ())?;
    Ok(())
}

fn dump_rbsave(args: &ArgMatches) -> Result<(), ()> {
    let path_str = args.value_of("path").unwrap();
    let path = Path::new(path_str);
    if !path.is_file() {
        eprintln!("File {} does not exist", path_str);
        return Err(());
    }
    let handle_io_error = |e| {
        eprintln!("{}", e);
    };
    let mut f = File::open(path).map_err(handle_io_error)?;
    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer).map_err(handle_io_error)?;
    let v: serde_json::Value =
        serde_rbbin::from_bytes(&buffer).map_err(|e| eprintln!("Failed to parse file {}", e))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&v).map_err(|e| {
            eprintln!("Json printing error {:?}", e);
        })?
    );
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
        .subcommand(
            clap::SubCommand::with_name("play")
                .about("Play single level")
                .arg(Arg::with_name("path")),
        )
        .subcommand(
            clap::SubCommand::with_name("dump_rbsave")
                .about("Read RB save file and print it as text")
                .arg(Arg::with_name("path")),
        )
        .get_matches();

    let result = match matches.subcommand() {
        ("diff", Some(m)) => run_diff(m),
        ("patch", Some(m)) => run_patch(m),
        ("play", Some(m)) => run_single_level(m),
        ("dump_rbsave", Some(m)) => dump_rbsave(m),
        _ => run_game(&matches),
    };
    ::std::process::exit(match result {
        Ok(_) => 0,
        Err(_) => 1,
    });
}
