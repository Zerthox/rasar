pub fn read_header(buffer: &[u8]) -> usize {
	let mut result = 0;
	for i in 0..4 {
		result += (buffer[i] as usize) << i * 8;
	}
	result
}