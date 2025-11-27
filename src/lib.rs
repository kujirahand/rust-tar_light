//! Simple tar archive reader and writer library
 
/// Tar header representation
#[derive(Debug)]
pub struct TarHeader {
    pub name: String,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub size: u64,
    pub mtime: u64,
    pub checksum: u32,
    pub typeflag: u8,
    pub linkname: String,
    pub magic: String,
    pub version: String,
    pub uname: String,
    pub gname: String,
    pub devmajor: u32,
    pub devminor: u32,
    pub prefix: String,
}

impl TarHeader {
    /// Create a new TarHeader with minimal required fields
    /// Other fields are filled with default values
    pub fn new(name: String, mode: u32, size: u64) -> Self {
        Self {
            name,
            mode,
            size,
            uid: 0,
            gid: 0,
            mtime: 0,
            checksum: 0,
            typeflag: b'0', // Regular file
            linkname: String::new(),
            magic: "ustar".to_string(),
            version: "00".to_string(),
            uname: String::new(),
            gname: String::new(),
            devmajor: 0,
            devminor: 0,
            prefix: String::new(),
        }
    }

    /// Parse a TarHeader from a 512-byte slice
    pub fn from_bytes(data: &[u8]) -> Self {
        parse_tar_header(data)
    }

    /// Convert the TarHeader to a 512-byte array
    pub fn to_bytes(&self) -> [u8; 512] {
        create_tar_header(self)
    }

    /// Verify the checksum of the header
    /// Returns true if the checksum is valid
    pub fn verify_checksum(&self, data: &[u8]) -> bool {
        if data.len() < 512 {
            return false;
        }
        let mut sum: u32 = 0;
        for (i, &b) in data.iter().take(512).enumerate() {
            if (148..156).contains(&i) {
                sum += b' ' as u32;
            } else {
                sum += b as u32;
            }
        }
        println!("Calculated checksum: {}, Header checksum: {}", sum, self.checksum);
        sum == self.checksum
    }
}

/// Tar entry representation
#[derive(Debug)]
pub struct TarEntry {
    pub header: TarHeader,
    pub data: Vec<u8>,
    pub header_bytes: [u8; 512],
}

/// Reads a tar archive from a byte slice and returns a vector of TarEntry
pub fn read_tar(data: &[u8]) -> Vec<TarEntry> {
    let mut entries = Vec::new();
    let mut offset = 0;
    while offset + 512 <= data.len() {
        // read 512-byte header
        let header_data = &data[offset..offset + 512];
        
        // Check if this is an empty block (end of archive)
        if is_empty_block(header_data) {
            break;
        }
        
        let header = parse_tar_header(header_data);
        
        // read file data
        let size = header.size as usize;
        let data_start = offset + 512;
        let data_end = data_start + size;
        
        if data_end > data.len() {
            break; // Corrupted archive
        }
        
        let entry_data = data[data_start..data_end].to_vec();
        
        // Copy header bytes
        let mut header_bytes = [0u8; 512];
        header_bytes.copy_from_slice(header_data);
        
        // Only add regular files (typeflag '0' or 0)
        if header.typeflag == b'0' || header.typeflag == 0 {
            entries.push(TarEntry { header, data: entry_data, header_bytes });
        }
        
        // Move to next entry (align to 512-byte boundary)
        let padding = if size % 512 == 0 { 0 } else { 512 - (size % 512) };
        offset = data_end + padding;
    }
    entries
}

/// Check if a block is empty (all zeros)
fn is_empty_block(data: &[u8]) -> bool {
    data.iter().all(|&b| b == 0)
}

use std::ops::Range;

fn read_tar_str(data: &[u8], range: Range<usize>) -> String {
    String::from_utf8_lossy(&data[range])
        .trim_end_matches('\0')
        .trim()
        .to_string()
}

fn read_tar_u32(data: &[u8], range: Range<usize>) -> u32 {
    let s = read_tar_str(data, range);
    if s.is_empty() {
        return 0;
    }
    u32::from_str_radix(&s, 8).unwrap_or(0)
}

fn read_tar_u64(data: &[u8], range: Range<usize>) -> u64 {
    let s = read_tar_str(data, range);
    if s.is_empty() {
        return 0;
    }
    u64::from_str_radix(&s, 8).unwrap_or(0)
}

fn read_tar_checksum(data: &[u8], range: Range<usize>) -> u32 {
    // checksum is stored as octal string
    // e.g., "0000644\0 "=(str + null + space)
    println!("Reading checksum from bytes: {:?}", &data[range.clone()]);
    let s = read_tar_str(&data, range)
        .trim()
        .trim_end_matches('\0')
        .trim()
        .to_string();
    if s.is_empty() {
        return 0;
    }
    u32::from_str_radix(&s, 8).unwrap_or(0)
}

fn parse_tar_header(data: &[u8]) -> TarHeader {
    // Simplified parsing logic for demonstration purposes
    TarHeader {
        name: read_tar_str(data, 0..100),
        mode: read_tar_u32(data, 100..108),
        uid: read_tar_u32(data, 108..116),
        gid: read_tar_u32(data, 116..124),
        size: read_tar_u64(data, 124..136),
        mtime: read_tar_u64(data, 136..148),
        checksum: read_tar_checksum(data, 148..156),
        typeflag: data[156],
        linkname: read_tar_str(data, 157..257),
        magic: read_tar_str(data, 257..263),
        version: read_tar_str(data, 263..265),
        uname: read_tar_str(data, 265..297),
        gname: read_tar_str(data, 297..329),
        devmajor: read_tar_u32(data, 329..337),
        devminor: read_tar_u32(data, 337..345),
        prefix: read_tar_str(data, 345..500),
    }
}

/// Writes a vector of TarEntry to a tar archive in a byte vector
pub fn write_tar(entries: &[TarEntry]) -> Vec<u8> {
    let mut tar_data = Vec::new();
    for entry in entries {
        // Use header_bytes if available, otherwise create from header
        tar_data.extend_from_slice(&entry.header_bytes);
        tar_data.extend_from_slice(&entry.data);
        let padding = (512 - (entry.data.len() % 512)) % 512;
        tar_data.extend_from_slice(&vec![0u8; padding]);
    }
    tar_data
}

fn create_tar_header(header: &TarHeader) -> [u8; 512] {
    let mut data = [0u8; 512];
    // Simplified header creation logic for demonstration purposes
    let name_bytes = header.name.as_bytes();
    data[0..name_bytes.len()].copy_from_slice(name_bytes);
    let mode_str = format!("{:o}", header.mode);
    let mode_bytes = mode_str.as_bytes();
    data[100..100 + mode_bytes.len()].copy_from_slice(mode_bytes);
    let uid_str = format!("{:o}", header.uid);
    let uid_bytes = uid_str.as_bytes();
    data[108..108 + uid_bytes.len()].copy_from_slice(uid_bytes);
    let gid_str = format!("{:o}", header.gid);
    let gid_bytes = gid_str.as_bytes();
    data[116..116 + gid_bytes.len()].copy_from_slice(gid_bytes);
    let size_str = format!("{:o}", header.size);
    let size_bytes = size_str.as_bytes();
    data[124..124 + size_bytes.len()].copy_from_slice(size_bytes);
    let mtime_str = format!("{:o}", header.mtime);
    let mtime_bytes = mtime_str.as_bytes();
    data[136..136 + mtime_bytes.len()].copy_from_slice(mtime_bytes);
    // calc checksum
    let checksum = calc_tar_checksum(&data);
    let checksum_str = format!("{:06o}\0 ", checksum);
    let checksum_bytes = checksum_str.as_bytes();
    data[148..148 + checksum_bytes.len()].copy_from_slice(checksum_bytes);
    data
}

fn calc_tar_checksum(header: &[u8; 512]) -> u32 {
    let mut sum: u32 = 0;
    for (i, &b) in header.iter().enumerate() {
        if (148..156).contains(&i) {
            sum += b' ' as u32;
        } else {
            sum += b as u32;
        }
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_test() {
        let data = include_bytes!("../testdata/test.tar");
        let entries = read_tar(data);
        let test_entry = entries.iter().find(|e| e.header.name == "test.txt").unwrap();
        let calculated_checksum = test_entry.header.verify_checksum(&test_entry.header_bytes);
        assert!(calculated_checksum, "Checksum verification failed");
    }

    #[test]
    fn read_test() {
        let data = include_bytes!("../testdata/test.tar");
        let entries = read_tar(data);
        for e in &entries {
            println!("{:?}", e);
        }
        // find `test.txt` from entries
        let test_entry = entries.iter().find(|e| e.header.name == "test.txt");
        assert!(test_entry.is_some());
        let test_entry = test_entry.unwrap();
        assert_eq!(test_entry.header.name, "test.txt");
        assert_eq!(test_entry.header.size, 33);
        assert_eq!(String::from_utf8_lossy(&test_entry.data), "0123456789ABCDEF__This is a pen.\n");
    }

    #[test]
    fn write_test() {
        let mut entries: Vec<TarEntry> = Vec::new();
        // Test TarHeader::new
        let header = TarHeader::new("hoge.txt".to_string(), 0o644, 12);
        assert_eq!(header.name, "hoge.txt");
        assert_eq!(header.mode, 0o644);
        assert_eq!(header.size, 12);
        
        // Create a test entry
        let data = b"Hello, World".to_vec();
        let header_bytes = header.to_bytes();
        entries.push(TarEntry { header, data, header_bytes });
        
        // Write tar and verify it can be read back
        let tar_data = write_tar(&entries);
        let read_entries = read_tar(&tar_data);
        
        assert_eq!(read_entries.len(), 1);
        assert_eq!(read_entries[0].header.name, "hoge.txt");
        assert_eq!(read_entries[0].header.size, 12);
        assert_eq!(read_entries[0].data, b"Hello, World");
    }
}
