extern crate clap;
extern crate serde_json;

use clap::{crate_description, crate_version, App, AppSettings, Arg, SubCommand};
use rasar;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
	let args = App::new("Rasar")
		.version(crate_version!())
		.about(crate_description!())
		.settings(&[
			AppSettings::ArgRequiredElseHelp,
			AppSettings::VersionlessSubcommands,
		])
		.subcommand(
			SubCommand::with_name("list")
				.visible_alias("l")
				.about("List all files included in an asar archive")
				.arg(
					Arg::with_name("ARCHIVE")
						.required(true)
						.help("Target asar archive"),
				),
		)
		.subcommand(
			SubCommand::with_name("pack")
				.visible_alias("p")
				.about("Pack a directory into an asar archive")
				.arg(
					Arg::with_name("DIR")
						.required(true)
						.help("Target directory"),
				)
				.arg(
					Arg::with_name("DEST")
						.required(true)
						.help("Asar archive file destination"),
				),
		)
		.subcommand(
			SubCommand::with_name("extract")
				.visible_alias("e")
				.about("Extract all files from an asar archive")
				.arg(
					Arg::with_name("ARCHIVE")
						.required(true)
						.help("Target asar archive"),
				)
				.arg(
					Arg::with_name("DEST")
						.required(true)
						.help("Destination folder"),
				),
		)
		.subcommand(
			SubCommand::with_name("extract-file")
				.visible_alias("ef")
				.about("Extract a single files from an asar archive")
				.arg(
					Arg::with_name("ARCHIVE")
						.required(true)
						.help("Target asar archive"),
				)
				.arg(
					Arg::with_name("DEST")
						.required(true)
						.help("File destination"),
				),
		)
		.get_matches();

	match args.subcommand() {
		("list", Some(cmd)) => {
			for entry in rasar::list(cmd.value_of("ARCHIVE").unwrap())? {
				println!(
					"{}",
					entry.to_str().expect("Error converting OS path to string")
				);
			}
		}
		("pack", Some(cmd)) => {
			rasar::pack(cmd.value_of("DIR").unwrap(), cmd.value_of("DEST").unwrap())?
		}
		("extract", Some(cmd)) => rasar::extract(
			cmd.value_of("ARCHIVE").unwrap(),
			cmd.value_of("DEST").unwrap(),
		)?,
		("extract-file", Some(cmd)) => rasar::extract_file(
			cmd.value_of("ARCHIVE").unwrap(),
			cmd.value_of("DEST").unwrap(),
		)?,
		_ => unreachable!(),
	}

	Ok(())
}
