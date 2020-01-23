use std::error::Error;
use std::env;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::io::SeekFrom;
use std::io::prelude::*;
use serde_json::Value;

fn read_u32(buffer: &[u8]) -> u32 {
	let mut result = 0;
	for i in 0..4 {
		result += (buffer[i] as u32) << i * 8;
	}
	result
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

pub fn pack(file: &str) -> Result<(), Box<dyn Error>> {
	println!("Packing {}", file);
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