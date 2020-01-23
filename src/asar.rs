use std::{
    error::Error,
    env,
    fs,
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
    io,
    io::{SeekFrom, prelude::*}
};
// use glob::glob;
use serde_json::{json, Value};

const MAX_SIZE: u64 = 4294967295;

fn align_size(value: usize) -> usize {
    value + (4 - (value % 4)) % 4
}

fn read_u32(buffer: &[u8]) -> u32 {
	let mut result = 0;
	for i in 0..4 {
		result += (buffer[i] as u32) << i * 8;
	}
	result
}

fn write_u32(buffer: &mut [u8], value: u32) {
    for i in 0..4 {
        buffer[i] = (value >> i * 8) as u8;
    }
}


fn read_header(reader: &mut File) -> Result<(u32, Value), Box<dyn Error>> {
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

fn iterate_entries(json: &Value, mut callback: impl FnMut(&Value, &str)) {
    iterate_entries_err(json, |current, path| {
        callback(current, path);
        Ok(())
    }).expect("Error iterating entries");
}

fn iterate_entries_err(json: &Value, mut callback: impl FnMut(&Value, &str) -> Result<(), Box<dyn Error>>) -> Result<(), Box<dyn Error>> {
    fn helper(current: &Value, path: String, callback: &mut impl FnMut(&Value, &str) -> Result<(), Box<dyn Error>>) -> Result<(), Box<dyn Error>> {
        callback(current, &path)?;
        if current["files"] != Value::Null {
            for (key, val) in current["files"].as_object().unwrap() {
                helper(&val, String::from(&path) + "\\" + key, callback)?;
            }
        }
        Ok(())
    }
    for (key, val) in json["files"].as_object().unwrap() {
        helper(val, String::from(key), &mut callback)?;
    }
    Ok(())
}

pub fn list(file: &str) -> Result<(), Box<dyn Error>> {
    let mut file = File::open(file)?;

    // read header
    let (_, json) = read_header(&mut file)?;

    // list files
    iterate_entries(&json, |_, path| println!("\\{}", path));

    Ok(())
}

pub fn pack(path: &str, dest: &str) -> Result<(), Box<dyn Error>> {
    let mut header_json = json!({
        "files": {}
    });
    let dir = env::current_dir()?.join(path);
    if dir.exists() {
        fn walk_dir(dir: impl AsRef<Path>, json: &mut Value, mut offset: &mut u64) -> Result<Vec<PathBuf>, Box<dyn Error>> {
            let mut files = vec![];
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let name = entry.file_name().into_string().expect("Error converting OS path to string");
                let meta = entry.metadata()?;
                if meta.is_dir() {
                    json[&name] = json!({
                        "files": {}
                    });
                    files.append(&mut walk_dir(entry.path(), &mut json[&name]["files"], &mut offset)?);
                }
                else {
                    let size = meta.len();
                    if size > MAX_SIZE {
                        panic!("File {} ({} GB) is above the maximum possible size of {} GB", name, size as f64 / 1e9, MAX_SIZE as f64 / 1e9);
                    }
                    json[&name] = json!({
                        "offset": offset.to_string(),
                        "size": size
                    });
                    *offset += size;
                    files.push(entry.path());
                }
            }
            Ok(files)
        }
        let files = walk_dir(dir, &mut header_json["files"], &mut 0)?;

        // create json buffer
        let json = serde_json::to_vec(&header_json)?;

        // compute & write sizes
        let size = align_size(json.len());
        let mut header = vec![0u8; 16];
        header[0] = 4;
        write_u32(&mut header[4..8], 8 + size as u32);
        write_u32(&mut header[8..12], 4 + size as u32);
        write_u32(&mut header[12..16], json.len() as u32);
        fs::write(dest, &header)?;

        // append json
        let mut archive = OpenOptions::new()
            .write(true)
            .append(true)
            .open(dest)?;
        archive.write_all(&json)?;

        // copy file contents
        for filename in files {
            io::copy(&mut File::open(filename)?, &mut archive)?;
        }
    }
    else {
        // TODO: allow globs
        // if let Ok(entries) = glob(path) {
        //     for entry in entries {
        //         dbg!(entry?);
        //     }
        // }
        panic!("{} is not a valid directory", path);
    }

	Ok(())
}

pub fn extract(file: &str, dest: &str) -> Result<(), Box<dyn Error>> {
    let mut file = File::open(file)?;

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
        }
        else {
            let dir = dest.join(path);
            if !dir.exists() {
                fs::create_dir(dir)?;
            }
        }
        Ok(())
    })?;

    Ok(())
}