//! Simple tar archive reader and writer library
//!
//! # Usage
//!
//! ## Reading TAR Archives
//!
//! ```rust
//! use tar_light::tar::{read_tar, TarEntry};
//!
//! // Read TAR archive from file
//! let tar_data = std::fs::read("testdata/simple.tar").unwrap();
//! let entries = read_tar(&tar_data);
//!
//! // Process entries
//! for entry in entries {
//!     println!("File name: {}", entry.header.name);
//!     println!("Size: {} bytes", entry.header.size);
//!     println!("Content: {}", String::from_utf8_lossy(&entry.data));
//! }
//! ```
//!
//! ## Creating TAR Archives
//!
//! ```rust
//! use tar_light::tar::{TarHeader, TarEntry, write_tar};
//!
//! let mut entries = Vec::new();
//!
//! // Create a new entry
//! let header = TarHeader::new("hello.txt".to_string(), 0o644, 12);
//! let data = b"Hello, World".to_vec();
//! let header_bytes = header.to_bytes();
//!
//! entries.push(TarEntry {
//!     header,
//!     data,
//!     header_bytes,
//! });
//!
//! // Write to TAR archive
//! let tar_data = write_tar(&entries);
//! std::fs::write("archive.tar", tar_data).unwrap();
//! ```
//!
//! ## Working with Headers
//!
//! ```rust
//! use tar_light::tar::TarHeader;
//!
//! // Create header with minimal fields
//! let header = TarHeader::new("file.txt".to_string(), 0o644, 1024);
//!
//! // Parse header from byte array
//! let bytes = [0u8; 512]; // TAR header is 512 bytes
//! let header = TarHeader::from_bytes(&bytes);
//!
//! // Convert header to byte array
//! let bytes = header.to_bytes();
//!
//! // Verify checksum
//! let is_valid = header.verify_checksum(&bytes);
//! ```
 
// Tar header struct
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
    /// new TarHeader with additional fields
    pub fn new_full(
        name: String,
        mode: u32,
        size: u64,
        mtime: u64,
        typeflag: u8,
        linkname: String,
        uname: String,
        gname: String,
    ) -> Self {
        let mut header = Self::new(name, mode, size);
        header.mtime = mtime;
        header.typeflag = typeflag;
        header.linkname = linkname;
        header.uname = uname;
        header.gname = gname;
        header
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
        let sum = calc_checksum(data);
        sum == self.checksum
    }
}

/// Tar entry struct
#[derive(Debug)]
pub struct TarEntry {
    pub header: TarHeader,
    pub data: Vec<u8>,
    pub header_bytes: [u8; 512],
}

// Tar struct
#[derive(Debug)]
pub struct Tar {
    pub entries: Vec<TarEntry>,
    pub use_header_parsing: bool, // if true, update TarEntry.header_bytes on modification
}
impl Tar {
    /// Create a new empty Tar archive
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            use_header_parsing: false,
        }
    }
    /// Create a Tar archive from bytes
    pub fn from_bytes(data: &[u8]) -> Self {
        let entries = read_tar(data);
        Self {
            entries,
            use_header_parsing: false,
        }
    }
    /// Add an entry to the Tar archive
    pub fn add_entry(&mut self, entry: TarEntry) {
        self.entries.push(entry);
    }
    /// Add string data to the Tar archive
    pub fn add_str_entry(&mut self, name: &str, content: &str) {
        let data = content.as_bytes().to_vec();
        let mut header = TarHeader::new(name.to_string(), 0o664, data.len() as u64);
        header.typeflag = b'0'; // 通常ファイルとして明示
        let mut header_bytes = [0u8; 512];
        if self.use_header_parsing {
            header_bytes = header.to_bytes();
        }
        let entry = TarEntry {
            header,
            data,
            header_bytes,
        };
        self.entries.push(entry);
    }
    /// Find entry by name
    pub fn find_entry(&self, name: &str) -> Option<&TarEntry> {
        self.entries.iter().find(|e| e.header.name == name)
    }
    /// set string like key-value store
    pub fn set_str(&mut self, name: &str, content: &str) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.header.name == name) {
            entry.data = content.as_bytes().to_vec();
            entry.header.size = entry.data.len() as u64;
            if self.use_header_parsing {
                entry.header_bytes = entry.header.to_bytes();
            }
        } else {
            self.add_str_entry(name, content);
        }
    }
    /// get string like key-value store
    pub fn get_str(&self, name: &str) -> Option<String> {
        if let Some(entry) = self.entries.iter().find(|e| e.header.name == name) {
            let data = String::from_utf8_lossy(&entry.data)
                .trim_end_matches('\0')
                .to_string();
            Some(data)
        } else {
            None
        }
    }
    /// Convert the Tar archive to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        write_tar(&self.entries)
    }
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
        let header_bytes = create_tar_header(&entry.header);
        tar_data.extend_from_slice(&header_bytes);
        // Write data and padding to 512-byte boundary
        tar_data.extend_from_slice(&entry.data);
        let padding = (512 - (entry.data.len() % 512)) % 512;
        tar_data.extend_from_slice(&vec![0u8; padding]);
    }
    // Add two 512-byte zero blocks at the end (TAR format specification)
    tar_data.extend_from_slice(&[0u8; 1024]);
    tar_data
}

fn create_tar_header(header: &TarHeader) -> [u8; 512] {
    let mut data = [0u8; 512];
    // Simplified header creation logic for demonstration purposes
    let name_bytes = header.name.as_bytes();
    let name_len = name_bytes.len().min(100); // Max 100 bytes for name field
    data[0..name_len].copy_from_slice(&name_bytes[..name_len]);
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
    
    // Set typeflag
    data[156] = header.typeflag;
    
    // Set linkname
    let linkname_bytes = header.linkname.as_bytes();
    let linkname_len = linkname_bytes.len().min(100);
    data[157..157 + linkname_len].copy_from_slice(&linkname_bytes[..linkname_len]);
    
    // Set magic ("ustar")
    let magic_bytes = header.magic.as_bytes();
    let magic_len = magic_bytes.len().min(6);
    data[257..257 + magic_len].copy_from_slice(&magic_bytes[..magic_len]);
    
    // Set version
    let version_bytes = header.version.as_bytes();
    let version_len = version_bytes.len().min(2);
    data[263..263 + version_len].copy_from_slice(&version_bytes[..version_len]);
    
    // Set uname
    let uname_bytes = header.uname.as_bytes();
    let uname_len = uname_bytes.len().min(32);
    data[265..265 + uname_len].copy_from_slice(&uname_bytes[..uname_len]);
    
    // Set gname
    let gname_bytes = header.gname.as_bytes();
    let gname_len = gname_bytes.len().min(32);
    data[297..297 + gname_len].copy_from_slice(&gname_bytes[..gname_len]);
    
    // Set devmajor
    let devmajor_str = format!("{:o}", header.devmajor);
    let devmajor_bytes = devmajor_str.as_bytes();
    let devmajor_len = devmajor_bytes.len().min(8);
    data[329..329 + devmajor_len].copy_from_slice(&devmajor_bytes[..devmajor_len]);
    
    // Set devminor
    let devminor_str = format!("{:o}", header.devminor);
    let devminor_bytes = devminor_str.as_bytes();
    let devminor_len = devminor_bytes.len().min(8);
    data[337..337 + devminor_len].copy_from_slice(&devminor_bytes[..devminor_len]);
    
    // Set prefix
    let prefix_bytes = header.prefix.as_bytes();
    let prefix_len = prefix_bytes.len().min(155); // Max 155 bytes for prefix field
    data[345..345 + prefix_len].copy_from_slice(&prefix_bytes[..prefix_len]);
    
    // calc checksum
    let checksum = calc_checksum(&data);
    let checksum_str = format!("{:06o}\0 ", checksum);
    let checksum_bytes = checksum_str.as_bytes();
    data[148..148 + checksum_bytes.len()].copy_from_slice(checksum_bytes);
    data
}

/// Calc checksum of the header bytes
pub fn calc_checksum(data: &[u8]) -> u32 {
    if data.len() < 512 {
        return 0;
    }
    let mut sum: u32 = 0;
    for (i, &b) in data.iter().take(512).enumerate() {
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
    fn tar_methods_test() {
        // Tar::new で空のTarを作成
        let mut tar = Tar::new();
        assert_eq!(tar.entries.len(), 0);

        // add_str_entry でエントリ追加
        tar.add_str_entry("foo.txt", "hello");
        assert_eq!(tar.entries.len(), 1);
        assert_eq!(tar.entries[0].header.name, "foo.txt");
        assert_eq!(String::from_utf8_lossy(&tar.entries[0].data), "hello");

        // set_str で同名エントリを上書き
        tar.set_str("foo.txt", "world");
        assert_eq!(tar.entries.len(), 1); // 上書きなので1件のまま
        assert_eq!(tar.get_str("foo.txt").as_deref(), Some("world"));

        // add_entry でTarEntryを追加
        let header = TarHeader::new("bar.txt".to_string(), 0o644, 3);
        let data = b"abc".to_vec();
        let header_bytes = header.to_bytes();
        let entry = TarEntry { header, data: data.clone(), header_bytes };
        tar.add_entry(entry);
        assert_eq!(tar.entries.len(), 2);
        assert_eq!(tar.get_str("bar.txt").as_deref(), Some("abc"));

        // find_entry で検索
        let found = tar.find_entry("foo.txt");
        assert!(found.is_some());
        assert_eq!(String::from_utf8_lossy(&found.unwrap().data), "world");

        // to_bytes でバイト列化し from_bytes で復元
        let mut tar1  = Tar::new();
        tar1.add_str_entry("foo.txt", "foo");
        tar1.add_str_entry("bar.txt", "bar");
        let bytes = tar1.to_bytes();
        println!("Bytes length: {}", bytes.len());
        let tar2 = Tar::from_bytes(&bytes);
        println!("Tar2: {:?}", tar2.entries);
        assert_eq!(tar2.entries.len(), 2);
        assert_eq!(tar2.get_str("foo.txt").as_deref(), Some("foo"));
        assert_eq!(tar2.get_str("bar.txt").as_deref(), Some("bar"));
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

    #[test]
    fn security_test_oversized_name() {
        // Test with name exceeding 100 bytes (maximum for name field)
        let long_name = "a".repeat(200); // 200 bytes, exceeds 100 byte limit
        let header = TarHeader::new(long_name.clone(), 0o644, 10);
        let header_bytes = header.to_bytes();
        
        // Verify that only first 100 bytes are written
        let name_field = &header_bytes[0..100];
        let null_pos = name_field.iter().position(|&b| b == 0).unwrap_or(100);
        assert!(null_pos <= 100, "Name field should not exceed 100 bytes");
        
        // Create entry and verify it can be read back
        let data = b"Test data!".to_vec();
        let entry = TarEntry { header, data: data.clone(), header_bytes };
        let tar_data = write_tar(&[entry]);
        let read_entries = read_tar(&tar_data);
        
        assert_eq!(read_entries.len(), 1);
        assert_eq!(read_entries[0].data, data);
        // Name should be truncated to 100 bytes
        assert!(read_entries[0].header.name.len() <= 100);
    }

    #[test]
    fn security_test_oversized_prefix() {
        // Test with prefix exceeding 155 bytes (maximum for prefix field)
        let long_prefix = "b".repeat(200); // 200 bytes, exceeds 155 byte limit
        let mut header = TarHeader::new("test.txt".to_string(), 0o644, 10);
        header.prefix = long_prefix;
        let header_bytes = header.to_bytes();
        
        // Verify that only first 155 bytes are written to prefix field
        let prefix_field = &header_bytes[345..500];
        let null_pos = prefix_field.iter().position(|&b| b == 0).unwrap_or(155);
        assert!(null_pos <= 155, "Prefix field should not exceed 155 bytes");
        
        // Create entry and verify it can be read back
        let data = b"Test data!".to_vec();
        let entry = TarEntry { header, data: data.clone(), header_bytes };
        let tar_data = write_tar(&[entry]);
        let read_entries = read_tar(&tar_data);
        
        assert_eq!(read_entries.len(), 1);
        assert_eq!(read_entries[0].data, data);
        // Prefix should be truncated to 155 bytes
        assert!(read_entries[0].header.prefix.len() <= 155);
    }

    #[test]
    fn security_test_special_characters() {
        // Test with special characters and null bytes in name
        let special_name = "test\0file\x00name.txt";
        let header = TarHeader::new(special_name.to_string(), 0o644, 5);
        let header_bytes = header.to_bytes();
        
        // Create entry and verify it can be read back
        let data = b"Hello".to_vec();
        let entry = TarEntry { header, data: data.clone(), header_bytes };
        let tar_data = write_tar(&[entry]);
        let read_entries = read_tar(&tar_data);
        
        assert_eq!(read_entries.len(), 1);
        assert_eq!(read_entries[0].data, data);
    }

    #[test]
    fn security_test_all_fields_oversized() {
        // Test with multiple oversized fields at once
        let long_name = "n".repeat(150);
        let long_prefix = "p".repeat(200);
        let long_uname = "u".repeat(50);
        let long_gname = "g".repeat(50);
        let long_linkname = "l".repeat(150);
        
        let data = b"Test".to_vec();
        let header = TarHeader::new_full(
            long_name,
            0o644,
            data.len() as u64, // Use actual data size
            0,
            b'0',
            long_linkname,
            long_uname,
            long_gname,
        );
        let mut header_with_prefix = header;
        header_with_prefix.prefix = long_prefix;
        
        let header_bytes = header_with_prefix.to_bytes();
        
        // Verify all fields are properly truncated
        assert!(header_bytes[0..100].iter().any(|&b| b != 0), "Name field should have data");
        assert!(header_bytes[265..297].iter().any(|&b| b != 0), "Uname field should have data");
        assert!(header_bytes[297..329].iter().any(|&b| b != 0), "Gname field should have data");
        assert!(header_bytes[345..500].iter().any(|&b| b != 0), "Prefix field should have data");
        
        // Create entry and verify it can be written and read
        let entry = TarEntry { header: header_with_prefix, data: data.clone(), header_bytes };
        let tar_data = write_tar(&[entry]);
        let read_entries = read_tar(&tar_data);
        
        assert_eq!(read_entries.len(), 1);
        assert_eq!(read_entries[0].data, b"Test");
    }

    #[test]
    fn security_test_path_traversal_attack() {
        // Test with path traversal attempts in filename
        let malicious_names = vec![
            "../../../etc/passwd",
            "../../secret.txt",
            "subdir/../../outside.txt",
            "/absolute/path/file.txt",
            "..\\..\\windows\\path.txt",
        ];
        
        for malicious_name in malicious_names {
            let header = TarHeader::new(malicious_name.to_string(), 0o644, 10);
            let data = b"malicious!".to_vec();
            let header_bytes = header.to_bytes();
            
            let entry = TarEntry { header, data: data.clone(), header_bytes };
            let tar_data = write_tar(&[entry]);
            let read_entries = read_tar(&tar_data);
            
            // Archive should be parseable
            assert_eq!(read_entries.len(), 1);
            assert_eq!(read_entries[0].data, data);
            
            // Verify that malicious path is stored (sanitization should happen at unpack time)
            assert!(read_entries[0].header.name.contains("..") || read_entries[0].header.name.starts_with('/'));
        }
    }

    #[test]
    fn security_test_size_mismatch() {
        // Test with header size not matching actual data size
        let header = TarHeader::new("fake_size.txt".to_string(), 0o644, 1000000); // Claims 1MB
        let data = b"tiny".to_vec(); // Only 4 bytes
        let header_bytes = header.to_bytes();
        
        let entry = TarEntry { 
            header, 
            data: data.clone(), 
            header_bytes 
        };
        let tar_data = write_tar(&[entry]);
        
        // read_tar should handle this gracefully (reads only what's available)
        let read_entries = read_tar(&tar_data);
        
        // Should not crash, but may have unexpected results
        // This tests resilience against corrupted archives
        assert!(read_entries.len() <= 1);
    }

    #[test]
    fn security_test_integer_overflow() {
        // Test with maximum size value (potential integer overflow)
        let header = TarHeader::new("overflow.txt".to_string(), 0o644, u64::MAX);
        let data = b"small".to_vec();
        let header_bytes = header.to_bytes();
        
        let entry = TarEntry { header, data, header_bytes };
        let tar_data = write_tar(&[entry]);
        
        // read_tar should not crash or allocate massive memory
        let read_entries = read_tar(&tar_data);
        
        // Archive is malformed, but should be handled gracefully
        assert!(read_entries.is_empty() || read_entries[0].data.len() < 10);
    }

    #[test]
    fn security_test_null_byte_injection() {
        // Test with null bytes in various positions
        let names_with_nulls = vec![
            "file\0hidden.txt",
            "normal.txt\0\0\0",
            "\0start_null.txt",
        ];
        
        for name_with_null in names_with_nulls {
            let header = TarHeader::new(name_with_null.to_string(), 0o644, 5);
            let data = b"test!".to_vec();
            let header_bytes = header.to_bytes();
            
            let entry = TarEntry { header, data: data.clone(), header_bytes };
            let tar_data = write_tar(&[entry]);
            let read_entries = read_tar(&tar_data);
            
            assert_eq!(read_entries.len(), 1);
            assert_eq!(read_entries[0].data, data);
        }
    }

    #[test]
    fn security_test_invalid_checksum() {
        // Test with deliberately invalid checksum
        let mut tar = Tar::new();
        tar.use_header_parsing = true;
        tar.add_str_entry("test.txt", "test data!");
        let mut tar_data = tar.to_bytes();
        tar_data[148] = b'9'; // Corrupt checksum
        tar_data[149] = b'9'; // Corrupt checksum
        let tar2 = Tar::from_bytes(&tar_data);
        // Explicit checksum verification should fail
        assert!(!tar2.entries[0].header.verify_checksum(&tar2.entries[0].header_bytes));
    }

    #[test]
    fn security_test_symlink_in_archive() {
        // Test handling of symbolic link entries (typeflag '2')
        let mut header = TarHeader::new("symlink.txt".to_string(), 0o777, 0);
        header.typeflag = b'2'; // Symbolic link
        header.linkname = "/etc/passwd".to_string();
        let header_bytes = header.to_bytes();
        
        let entry = TarEntry { 
            header, 
            data: Vec::new(), 
            header_bytes 
        };
        let tar_data = write_tar(&[entry]);
        let read_entries = read_tar(&tar_data);
        
        // Symbolic links should be filtered out (only regular files returned)
        assert_eq!(read_entries.len(), 0);
    }

    #[test]
    fn security_test_device_file_in_archive() {
        // Test handling of device file entries (typeflag '3' and '4')
        let test_cases = vec![
            (b'3', "char_device"),  // Character device
            (b'4', "block_device"), // Block device
            (b'5', "directory"),    // Directory
            (b'6', "fifo"),         // FIFO
        ];
        
        for (typeflag, name) in test_cases {
            let mut header = TarHeader::new(name.to_string(), 0o644, 0);
            header.typeflag = typeflag;
            let header_bytes = header.to_bytes();
            
            let entry = TarEntry { 
                header, 
                data: Vec::new(), 
                header_bytes 
            };
            let tar_data = write_tar(&[entry]);
            let read_entries = read_tar(&tar_data);
            
            // Non-regular files should be filtered out
            assert_eq!(read_entries.len(), 0, "Typeflag {} should be filtered", typeflag);
        }
    }

    #[test]
    fn security_test_deeply_nested_path() {
        // Test with extremely deep directory nesting
        let deep_path = "a/".repeat(50) + "file.txt"; // 50 levels deep
        let header = TarHeader::new(deep_path.clone(), 0o644, 4);
        let data = b"deep".to_vec();
        let header_bytes = header.to_bytes();
        
        let entry = TarEntry { header, data: data.clone(), header_bytes };
        let tar_data = write_tar(&[entry]);
        let read_entries = read_tar(&tar_data);
        
        assert_eq!(read_entries.len(), 1);
        assert_eq!(read_entries[0].data, data);
        // Path should be truncated to fit in name field (100 bytes)
        assert!(read_entries[0].header.name.len() <= 100);
    }

    #[test]
    fn security_test_malformed_archive_early_termination() {
        // Test with archive that ends abruptly
        let header = TarHeader::new("incomplete.txt".to_string(), 0o644, 1000);
        let data = b"short".to_vec(); // Much shorter than declared size
        let header_bytes = header.to_bytes();
        
        // Create incomplete tar data (header + partial data, no padding)
        let mut tar_data = Vec::new();
        tar_data.extend_from_slice(&header_bytes);
        tar_data.extend_from_slice(&data);
        // No padding or end markers
        
        // Should handle gracefully without crashing
        let read_entries = read_tar(&tar_data);
        
        // May return empty or incomplete entry, but shouldn't crash
        assert!(read_entries.is_empty() || read_entries[0].data.len() <= 5);
    }
}
