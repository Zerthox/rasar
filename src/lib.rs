mod error;
mod util;

pub use error::Error;
use glob::glob;
use serde_json::{json, Value};
use std::{
	env, fs,
	fs::{File, OpenOptions},
	io,
	io::{prelude::*, SeekFrom},
	path::{Component, Path, PathBuf},
};
use util::{align_size, read_u32, write_u32};

/// Maximum possible file size for files in asar archives.
const MAX_SIZE: u64 = std::u32::MAX as u64;

/// Read the header of an asar archive and extract the header size & json.
///
/// This may return an `io::Error` if there is an error reading the file.
fn read_header(reader: &mut File) -> Result<(u32, Value), io::Error> {
	// read header bytes
	let mut header_buffer = vec![0u8; 16];
	reader.read_exact(&mut header_buffer)?;

	// grab sizes
	let header_size = read_u32(&header_buffer[4..8]);
	let json_size = read_u32(&header_buffer[12..]);

	// read json bytes
	let mut json_buffer = vec![0u8; json_size as usize];
	reader.read_exact(&mut json_buffer)?;

	// parse json
	let json: Value = serde_json::from_slice(&json_buffer)?;

	Ok((header_size + 8, json))
}

/// Iterate over all entries in an asar archive.
fn iterate_entries(json: &Value, mut callback: impl FnMut(&Value, &PathBuf)) {
	iterate_entries_err(json, |current, path| {
		callback(current, path);
		Ok(())
	})
	.expect("Unexpected error while iterating archive entries");
}

/// Iterate over all entries in an asar archive while forwarding errors from the passed closure.
fn iterate_entries_err(
	json: &Value,
	mut callback: impl FnMut(&Value, &PathBuf) -> Result<(), Error>,
) -> Result<(), Error> {
	fn helper(
		current: &Value,
		path: PathBuf,
		callback: &mut impl FnMut(&Value, &PathBuf) -> Result<(), Error>,
	) -> Result<(), Error> {
		callback(current, &path)?;
		if current["files"] != Value::Null {
			for (key, val) in current["files"].as_object().unwrap() {
				helper(&val, path.join(key), callback)?;
			}
		}
		Ok(())
	}
	for (key, val) in json["files"].as_object().unwrap() {
		helper(val, PathBuf::new().join(key), &mut callback)?;
	}
	Ok(())
}

/// Get a list of all files in an asar archive.
///
/// # Examples
///
/// ```no_run
/// let file_entries = rasar::list("myarchive.asar").expect("Something went wrong");
/// ```
pub fn list(archive: &str) -> Result<Vec<PathBuf>, io::Error> {
	let mut file = File::open(archive)?;

	// read header
	let (_, json) = read_header(&mut file)?;

	// list files
	let mut files = vec![];
	iterate_entries(&json, |_, path| files.push(path.clone()));

	Ok(files)
}

/// Pack a directory into an asar archive.
///
/// # Examples
///
/// ```no_run
/// match rasar::pack("myfolder", "myarchive.asar") {
/// 	Ok(()) => println!("Success!"),
/// 	Err(err) => panic!("This should not have happened!")
/// }
/// ```
pub fn pack(path: &str, dest: &str) -> Result<(), Error> {
	let mut header_json = json!({
		"files": {}
	});
	let mut files = vec![];
	let dir = env::current_dir()?.join(path);
	if dir.exists() {
		fn walk_dir(
			dir: impl AsRef<Path>,
			json: &mut Value,
			mut offset: &mut u64,
		) -> Result<Vec<PathBuf>, Error> {
			let mut files = vec![];
			for entry in fs::read_dir(dir)? {
				let entry = entry?;
				let name = entry
					.file_name()
					.into_string()
					.expect("Error converting OS path to string");
				let meta = entry.metadata()?;
				if meta.is_file() {
					let size = meta.len();
					if size > MAX_SIZE {
						panic!(
							"File {} ({} GB) is above the maximum possible size of {} GB",
							name,
							size as f64 / 1e9,
							MAX_SIZE as f64 / 1e9
						);
					}
					json[&name] = json!({
						"offset": offset.to_string(),
						"size": size
					});
					*offset += size;
					files.push(entry.path());
				} else {
					json[&name] = json!({
						"files": {}
					});
					files.append(&mut walk_dir(
						entry.path(),
						&mut json[&name]["files"],
						&mut offset,
					)?);
				}
			}
			Ok(files)
		}
		files = walk_dir(dir, &mut header_json["files"], &mut 0)?;
	} else if let Ok(entries) = glob(path) {
		let mut offset = 0u64;
		for entry in entries {
			let entry = entry?;
			let mut current = &mut header_json["files"];
			let comps: Vec<&Path> = entry
				.components()
				.map(|comp| match comp {
					Component::Normal(name) => Path::new(name),
					_ => unreachable!(),
				})
				.collect();
			for comp in comps.iter().take(comps.len() - 1) {
				let name = comp
					.file_name()
					.unwrap()
					.to_str()
					.expect("Error converting OS path to string");
				current = &mut current[name]["files"];
			}
			let name = entry
				.file_name()
				.unwrap()
				.to_str()
				.expect("Error converting OS path to string");
			if entry.is_file() {
				let size = entry.metadata()?.len();
				if size > MAX_SIZE {
					panic!(
						"File {} ({} GB) is above the maximum possible size of {} GB",
						name,
						size as f64 / 1e9,
						MAX_SIZE as f64 / 1e9
					);
				}
				current[name] = json!({
					"offset": offset.to_string(),
					"size": size
				});
				offset += size;
				files.push(entry);
			} else {
				current[name] = json!({
					"files": {}
				});
			}
		}
	} else {
		panic!("{} is neither a valid directory nor glob", path);
	}

	// create header buffer with json
	let mut header = serde_json::to_vec(&header_json)?;

	// compute sizes
	let json_size = header.len();
	let size = align_size(json_size);

	// resize header
	header.resize(16 + size, 0);

	// copy json
	header.copy_within(0..json_size, 16);

	// write sizes into header
	write_u32(&mut header[0..4], 4);
	write_u32(&mut header[4..8], 8 + size as u32);
	write_u32(&mut header[8..12], 4 + size as u32);
	write_u32(&mut header[12..16], json_size as u32);

	// write header
	fs::write(dest, &header)?;

	// copy file contents
	let mut archive = OpenOptions::new().write(true).append(true).open(dest)?;
	for filename in files {
		io::copy(&mut File::open(filename)?, &mut archive)?;
	}

	Ok(())
}

/// Extract all files from an asar archive.
///
/// # Examples
///
/// ```no_run
/// match rasar::extract("myarchive.asar", "extracted") {
/// 	Ok(()) => println!("Success!"),
/// 	Err(err) => panic!("This should not have happened!")
/// }
/// ```
pub fn extract(archive: &str, dest: &str) -> Result<(), Error> {
	let mut file = File::open(archive)?;

	// read header
	let (header_size, json) = read_header(&mut file)?;

	// create destination folder
	let dest = env::current_dir()?.join(dest);
	if !dest.exists() {
		fs::create_dir(&dest)?;
	}

	// iterate over entries
	iterate_entries_err(&json, |val, path| {
		if val["offset"] != Value::Null {
			let offset = val["offset"].as_str().unwrap().parse::<u64>()?;
			let size = val["size"].as_u64().unwrap();
			file.seek(SeekFrom::Start(header_size as u64 + offset))?;
			let mut buffer = vec![0u8; size as usize];
			file.read_exact(&mut buffer)?;
			fs::write(dest.join(path), buffer)?;
		} else {
			let dir = dest.join(path);
			if !dir.exists() {
				fs::create_dir(dir)?;
			}
		}
		Ok(())
	})?;

	Ok(())
}

/// Extract a single file from an asar archive.
///
/// # Examples
///
/// ```no_run
/// match rasar::extract("myarchive.asar", "file.txt") {
/// 	Ok(()) => println!("Success!"),
/// 	Err(err) => panic!("This should not have happened!")
/// }
/// ```
pub fn extract_file(archive: &str, dest: &str) -> Result<(), Error> {
	let cwd = env::current_dir()?;
	let full_path = cwd.join(dest);
	let dest = cwd.join(Path::new(dest).file_name().unwrap());
	let mut file = File::open(archive)?;

	// read header
	let (header_size, json) = read_header(&mut file)?;

	// iterate over entries
	iterate_entries_err(&json, |val, path| {
		if cwd.join(path) == full_path {
			let offset = val["offset"].as_str().unwrap().parse::<u64>()?;
			let size = val["size"].as_u64().unwrap();
			file.seek(SeekFrom::Start(header_size as u64 + offset))?;
			let mut buffer = vec![0u8; size as usize];
			file.read_exact(&mut buffer)?;
			fs::write(&dest, buffer)?;
		}
		Ok(())
	})?;

	Ok(())
}
