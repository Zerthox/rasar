use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use serde_json::Value;

fn read_header(buffer: &[u8]) -> usize {
	let mut result = 0;
	for i in 0..4 {
		result += (buffer[i] as usize) << i * 8;
	}
	result
}

pub fn list(file: &str) -> Result<(), std::io::Error> {
    let file = File::open(file)?;
    let mut reader = BufReader::new(file);

    // read header bytes
    let mut header_buffer = vec![0u8; 16];
    reader.read_exact(&mut header_buffer)?;

    // grab json size
    let json_size = read_header(&header_buffer[12..]);

    // read json bytes
    let mut json_buffer = vec![0u8; json_size];
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