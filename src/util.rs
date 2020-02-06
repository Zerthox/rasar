pub fn align_size(size: usize) -> usize {
	size + (4 - (size % 4)) % 4
}

pub fn read_u32(buffer: &[u8]) -> u32 {
	buffer
		.iter()
		.take(4)
		.enumerate()
		.fold(0, |result, (i, byte)| result + ((*byte as u32) << (i * 8)))
}

pub fn write_u32(buffer: &mut [u8], value: u32) {
	for (i, byte) in buffer.iter_mut().take(4).enumerate() {
		*byte = (value >> (i * 8)) as u8;
	}
}
