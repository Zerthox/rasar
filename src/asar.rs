use std::error::Error;
use std::env;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use serde_json::Value;

fn read_u32(buffer: &[u8]) -> u32 {
	let mut result = 0;
	for i in 0..4 {
		result += (buffer[i] as u32) << i * 8;
	}
	result
}

fn read_header(reader: &mut BufReader<File>) -> Result<(u32, Value), Box<dyn Error>> {
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
    let file = File::open(file)?;
    let mut reader = BufReader::new(file);

    // read header
    let (_, json) = read_header(&mut reader)?;

    // list files
    iterate_entries(&json, |_, path| println!("\\{}", path));

    Ok(())
}

pub fn pack(file: &str) -> Result<(), std::io::Error> {
	println!("Packing {}", file);
	// let file = File::open(file)?;
	// let mut reader = BufReader::new(file);

	Ok(())
}

pub fn extract(file: &str) -> Result<(), std::io::Error> {
	println!("Extracting {}", file);
    // let file = File::open(file)?;
    // let mut reader = BufReader::new(file);

    Ok(())
}