#[macro_use]
extern crate clap;
extern crate serde_json;

mod asar;

use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use clap::AppSettings;
use serde_json::Value;

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
        ("list", Some(cmd)) => {
            list(cmd.value_of("FILE").unwrap())?;
        }
        ("pack", Some(_pack)) => {
            println!("Packing archive!");
        }
        ("extract", Some(_extract)) => {
            println!("Extracting archive!");
        }
        _ => unreachable!()
    }

    Ok(())
}

fn list(file: &str) -> Result<(), std::io::Error> {
    let file = File::open(file)?;
    let mut reader = BufReader::new(file);

    // read header bytes
    let mut header_buffer = vec![0u8; 16];
    reader.read_exact(&mut header_buffer)?;

    // grab json size
    let json_size = asar::read_header(&header_buffer[12..]);

    // read json bytes
    let mut json_buffer = vec![0u8; json_size as usize];
    reader.read_exact(&mut json_buffer)?;

    // parse json
    let json: Value = serde_json::from_slice(&json_buffer)?;

    // recursively list files
    fn recursive_list(current: &Value, path: &str) {
        println!("{}", path);
        if current["files"] != Value::Null {
            for (key, val) in current["files"].as_object().unwrap() {
                recursive_list(&val, &(String::from(path) + "\\" + key));
            }
        }
    }
    recursive_list(&json, "");

    Ok(())
}