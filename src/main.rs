#[macro_use]
extern crate clap;
extern crate serde_json;

mod asar;

use clap::AppSettings;

fn main() -> Result<(), std::io::Error> {
    let args = clap_app!(Rasar =>
        (version: "0.1.0")
        (about: "Pack & extract asar archives in Rust")
        (setting: AppSettings::ArgRequiredElseHelp)
        (@subcommand list =>
            (about: "List all files included in an asar archive")
            (@arg FILE: +required "Target asar archive file")
        )
        (@subcommand pack =>
            (about: "Pack a directory into an asar archive")
            (@arg DIR: +required "Target directory")
        )
        (@subcommand extract =>
            (about: "Extract all files from an asar archive")
            (@arg FILE: +required "Target asar archive file")
        )
    ).get_matches();


    match args.subcommand() {
        ("list", Some(cmd)) => asar::list(cmd.value_of("FILE").unwrap())?,
        ("pack", Some(cmd)) => asar::pack(cmd.value_of("FILE").unwrap())?,
        ("extract", Some(cmd)) => asar::extract(cmd.value_of("FILE").unwrap())?,
        _ => unreachable!()
    }

    Ok(())
}