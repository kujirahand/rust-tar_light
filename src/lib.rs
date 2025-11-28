//! Simple tar archive reader and writer library
//!
//! # Usage
//!
//! ## Packing files into a TAR archive
//!
//! ```rust
//! use tar_light::pack;
//!
//! let files = vec!["testdata/file1.txt", "testdata/file2.txt"];
//! 
//! pack("archive.tar", &files);
//! // Creates archive.tar containing file1.txt and file2.txt
//! pack("archive.tar.gz", &files);
//! // Creates archive.tar.gz that is gzip-compressed
//! ```
//!
//! ## Unpacking files from a TAR archive
//!
//! ```rust
//! use tar_light::unpack;
//!
//! unpack("testdata/simple.tar", "output_directory");
//! // Extracts all files from simple.tar to output_directory/
//! unpack("testdata/simple.tar.gz", "output_directory");
//! // Extracts all files from simple.tar.gz that is gzip-compressed
//! ```
//!
//! ## Listing files in a TAR archive header
//!
//! ```rust
//! use tar_light::list;
//!
//! match list("archive.tar") {
//!     Ok(headers) => {
//!         println!("Files in archive:");
//!         for header in headers {
//!             println!("  {} ({} bytes)", header.name, header.size);
//!         }
//!     }
//!     Err(e) => eprintln!("Error: {}", e),
//! }
//! ```
//!
//! ## Listing files in a TAR entry
//!
//! ```rust
//! use tar_light::list_entry;
//!
//! match list_entry("archive.tar") {
//!     Ok(entries) => {
//!         println!("Files in archive:");
//!         for entry in entries {
//!             println!("  {} ({} bytes)", entry.header.name, entry.header.size);
//!         }
//!     }
//!     Err(e) => eprintln!("Error: {}", e),
//! }
//! ```
//!
//! ## Advanced usage with low-level API
//!
//! ```rust
//! use tar_light::{read_tar, write_tar, Tar, TarEntry, TarHeader};
//! use std::fs;
//!
//! // Read tar archives
//! let bin_bytes = fs::read("testdata/simple.tar").unwrap();
//! let entries = read_tar(&bin_bytes);
//!
//! // List entries
//! for entry in &entries {
//!     println!("{}: {} bytes", entry.header.name, entry.header.size);
//! }
//! 
//! // Write entries
//! let tar_bytes = write_tar(&entries);
//! fs::write("archive.tar", tar_bytes).unwrap();
//! 
//! // Create tar archive from scratch
//! let mut tar = Tar::new();
//! tar.add_str_entry("file1.txt", "Hello, World!");
//! tar.add_str_entry("file2.txt", "This is a test.");
//! let tar_bytes = tar.to_bytes();
//! fs::write("archive.tar", tar_bytes).unwrap();
//! ```

pub mod tar;

use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::io::{Write, Read};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::{self, BufRead};

#[cfg(unix)]
use std::ffi::CStr;

pub use tar::{read_tar, write_tar, Tar, TarEntry, TarHeader};

// ----------------------------------------------------------------
// Helper functions for gzip compression/decompression
// ----------------------------------------------------------------

#[cfg(unix)]
/// Get username from uid using libc
fn get_username_from_uid(uid: u32) -> Option<String> {
    unsafe {
        let passwd = libc::getpwuid(uid);
        if passwd.is_null() {
            return None;
        }
        let name_ptr = (*passwd).pw_name;
        if name_ptr.is_null() {
            return None;
        }
        CStr::from_ptr(name_ptr)
            .to_str()
            .ok()
            .map(|s| s.to_string())
    }
}

#[cfg(unix)]
/// Get group name from gid using libc
fn get_groupname_from_gid(gid: u32) -> Option<String> {
    unsafe {
        let group = libc::getgrgid(gid);
        if group.is_null() {
            return None;
        }
        let name_ptr = (*group).gr_name;
        if name_ptr.is_null() {
            return None;
        }
        CStr::from_ptr(name_ptr)
            .to_str()
            .ok()
            .map(|s| s.to_string())
    }
}

#[cfg(not(unix))]
/// Stub for non-Unix platforms
fn get_username_from_uid(_uid: u32) -> Option<String> {
    None
}

#[cfg(not(unix))]
/// Stub for non-Unix platforms
fn get_groupname_from_gid(_gid: u32) -> Option<String> {
    None
}

// ----------------------------------------------------------------
// Helper functions for gzip compression/decompression
// ----------------------------------------------------------------
/// Checks if filename indicates gzip compression
fn is_gzipped(filename: &str) -> bool {
    filename.ends_with(".tar.gz") || filename.ends_with(".tgz")
}

/// Decompresses gzipped data if the filename suggests it's compressed
/// Returns the raw data unchanged if not gzipped
fn ungzip(filename: &str, data: Vec<u8>) -> Result<Vec<u8>, std::io::Error> {
    if is_gzipped(filename) {
        let mut decoder = GzDecoder::new(&data[..]);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        Ok(decompressed)
    } else {
        Ok(data)
    }
}

/// Compresses data with gzip if the filename suggests it should be compressed
/// Returns the raw data unchanged if not a gzip filename
fn gzip(filename: &str, data: Vec<u8>) -> Result<Vec<u8>, std::io::Error> {
    if is_gzipped(filename) {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&data)?;
        encoder.finish()
    } else {
        Ok(data)
    }
}

// ----------------------------------------------------------------
// Helper functions for recursive directory packing
// ----------------------------------------------------------------
/// Adds a single file to entries
fn add_file_to_entries(file_path: &Path, base_path: &Path, entries: &mut Vec<TarEntry>) {
    let data = match fs::read(file_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error reading {}: {}", file_path.display(), e);
            return;
        }
    };
    
    // Calculate relative path from base_path
    let relative_path = file_path.strip_prefix(base_path)
        .unwrap_or(file_path)
        .to_string_lossy()
        .to_string();

    let mut header = TarHeader::new(
        relative_path,
        0o644,
        data.len() as u64       
    );
    // get file metadata
    match fs::metadata(file_path) {
        Ok(m) => {
            header.mode = m.mode() as u32;
            header.mtime = m.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                .duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap_or_default().as_secs();
            header.gid = m.gid();
            header.uid = m.uid();
            // Set uname and gname from uid/gid
            if let Some(uname) = get_username_from_uid(m.uid()) {
                header.uname = uname;
            }
            if let Some(gname) = get_groupname_from_gid(m.gid()) {
                header.gname = gname;
            }
        },
        Err(e) => {
            eprintln!("Error getting metadata for {}: {}", file_path.display(), e);
            return;
        }
    };    let header_bytes = header.to_bytes();
    
    entries.push(TarEntry {
        header,
        data,
        header_bytes,
    });
}

/// Recursively collects all files from a directory
fn collect_files_from_dir(dir_path: &Path, base_path: &Path, entries: &mut Vec<TarEntry>) {
    let read_dir = match fs::read_dir(dir_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error reading directory {}: {}", dir_path.display(), e);
            return;
        }
    };
    
    for entry_result in read_dir {
        let entry = match entry_result {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Error reading directory entry: {}", e);
                continue;
            }
        };
        
        let path = entry.path();
        
        if path.is_dir() {
            // Recursively process subdirectory
            collect_files_from_dir(&path, base_path, entries);
        } else if path.is_file() {
            // Add file to entries
            add_file_to_entries(&path, base_path, entries);
        }
    }
}

// ----------------------------------------------------------------
// simple methods for reading and writing tar archives
// ----------------------------------------------------------------
/// Packs files into a tar archive (supports .tar and .tar.gz)
pub fn pack(tarfile: &str, files: &[&str]) {
    let mut entries = Vec::new();
    
    for file_path in files {
        let path = Path::new(file_path);
        if !path.exists() {
            eprintln!("Warning: File not found: {}", file_path);
            continue;
        }
        
        // Check if it's a directory
        if path.is_dir() {
            // Recursively add all files in the directory
            collect_files_from_dir(path, path, &mut entries);
        } else {
            // Add single file - use parent directory as base to preserve filename
            let base = path.parent().unwrap_or_else(|| Path::new(""));
            add_file_to_entries(path, base, &mut entries);
        }
    }
    
    let tar_data = write_tar(&entries);
    
    // Compress if needed
    let result = gzip(tarfile, tar_data)
        .and_then(|data| fs::write(tarfile, data));
    
    match result {
        Ok(_) => println!("Created tar archive: {}", tarfile),
        Err(e) => {
            eprintln!("Error writing tar file: {}", e);
            std::process::exit(1);
        }
    }
}

/// Unpacks files from a tar archive (supports .tar and .tar.gz)
pub fn unpack(tarfile: &str, output_dir: &str) {
    unpack_with_options(tarfile, output_dir, false, true);
}

/// Unpacks a tar archive with options
/// 
/// # Arguments
/// * `tarfile` - Path to the tar archive
/// * `output_dir` - Output directory
/// * `overwrite` - If true, overwrite existing files without prompting
///                 If false, skip existing files
/// * `use_prompt` - If true, prompt user for each existing file
pub fn unpack_with_options(tarfile: &str, output_dir: &str, overwrite: bool, use_prompt: bool) {
    let mut overwrite = overwrite;
    // Read file
    let file_data = match fs::read(tarfile) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error reading tar file: {}", e);
            std::process::exit(1);
        }
    };
    
    // Decompress if gzipped
    let tar_data = match ungzip(tarfile, file_data) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Error decompressing gzip: {}", e);
            std::process::exit(1);
        }
    };
    
    let entries = read_tar(&tar_data);
    
    let output_path = Path::new(output_dir);
    if !output_path.exists() {
        if let Err(e) = fs::create_dir_all(output_path) {
            eprintln!("Error creating output directory: {}", e);
            std::process::exit(1);
        }
    }
    
    for entry in entries {
        let file_path = output_path.join(&entry.header.name);
        let mut flag_overwrite = false;
        // Check if file exists and overwrite is false
        if file_path.exists() {
            if !overwrite {
                if use_prompt {
                    // ask to user
                    println!("❓File '{}' already exists. Overwrite? ([Y]es/[N]o/[A]ll): ", entry.header.name);
                    let stdin = io::stdin();
                    let mut line = String::new();
                    stdin.lock().read_line(&mut line).unwrap_or(0);
                    let answer = line.trim().to_lowercase();
                    
                    if answer == "a" || answer == "all" {
                        // Overwrite this and all subsequent files
                        println!("⚡ Overwriting all files...");
                        overwrite = true;
                    } else if answer == "y" || answer == "yes" {
                    } else {
                        println!("- Skipping: {}", entry.header.name);
                        continue;
                    }
                } else {
                    println!("- Skipping: {}", entry.header.name);
                    continue;
                }
            }
            flag_overwrite = true;
        }
        
        // Create parent directories if they don't exist
        if let Some(parent) = file_path.parent() {
            if !parent.exists() {
                if let Err(e) = fs::create_dir_all(parent) {
                    eprintln!("❌ Error creating directory {}: {}", parent.display(), e);
                    continue;
                }
            }
        }
        
        match fs::File::create(&file_path) {
            Ok(mut file) => {
                if let Err(e) = file.write_all(&entry.data) {
                    eprintln!("❌ Error writing {}: {}", entry.header.name, e);
                } else {
                    let overwrite_msg = if flag_overwrite { " (overwritten)" } else { "" };
                    println!("- Extracted: {}{}", entry.header.name, overwrite_msg);
                }
            }
            Err(e) => {
                eprintln!("❌ Error creating {}: {}", entry.header.name, e);
            }
        }
    }
    
    println!("Extraction complete to: {}", output_dir);
}

/// Lists TarHeader in a tar archive (supports .tar and .tar.gz)
pub fn list(tarfile: &str) -> Result<Vec<TarHeader>, std::io::Error> {
    let file_data = fs::read(tarfile)?;
    
    // Decompress if gzipped
    let tar_data = ungzip(tarfile, file_data)?;
    
    let entries = read_tar(&tar_data);
    let headers: Vec<TarHeader> = entries.into_iter().map(|e| e.header).collect();
    Ok(headers)
}

/// Lists TarEntry in a tar archive (supports .tar and .tar.gz)
pub fn list_entry(tarfile: &str) -> Result<Vec<TarEntry>, std::io::Error> {
    let file_data = fs::read(tarfile)?;
    
    // Check if input is gzipped
    let is_gzipped = tarfile.ends_with(".tar.gz") || tarfile.ends_with(".tgz");
    
    let tar_data = if is_gzipped {
        // Decompress with gzip
        let mut decoder = GzDecoder::new(&file_data[..]);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        decompressed
    } else {
        file_data
    };
    
    let entries = read_tar(&tar_data);
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    #[test]
    fn test_pack() {
        // Create test files
        let test_file1 = "test_file1.txt";
        let test_file2 = "test_file2.txt";
        let test_tar = "test_pack.tar";
        
        fs::write(test_file1, "Hello, World!").unwrap();
        fs::write(test_file2, "Test content 2").unwrap();
        
        // Execute pack function
        let files = vec![test_file1, test_file2];
        pack(test_tar, &files);
        
        // Verify tar file was created
        assert!(Path::new(test_tar).exists());
        
        // Verify tar file contents
        let tar_data = fs::read(test_tar).unwrap();
        let entries = read_tar(&tar_data);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].header.name, test_file1);
        assert_eq!(entries[1].header.name, test_file2);
        
        // Cleanup
        fs::remove_file(test_file1).unwrap();
        fs::remove_file(test_file2).unwrap();
        fs::remove_file(test_tar).unwrap();
    }

    #[test]
    fn test_unpack() {
        // Create test file and tar archive
        let test_file = "test_unpack_file.txt";
        let test_content = "Unpack test content";
        let test_tar = "test_unpack.tar";
        let output_dir = "test_unpack_output";
        
        fs::write(test_file, test_content).unwrap();
        
        // Create tar archive
        let files = vec![test_file];
        pack(test_tar, &files);
        
        // Execute unpack function
        unpack_with_options(test_tar, output_dir, false, false);
        
        // Verify file was extracted
        let extracted_file = Path::new(output_dir).join(test_file);
        assert!(extracted_file.exists());
        
        // Verify file content
        let content = fs::read_to_string(&extracted_file).unwrap();
        assert_eq!(content, test_content);
        
        // Cleanup
        fs::remove_file(test_file).unwrap();
        fs::remove_file(test_tar).unwrap();
        fs::remove_dir_all(output_dir).unwrap();
    }

    #[test]
    fn test_list() {
        // Create test files and tar archive
        let test_file1 = "test_list_file1.txt";
        let test_file2 = "test_list_file2.txt";
        let test_tar = "test_list.tar";
        
        fs::write(test_file1, "Content 1").unwrap();
        fs::write(test_file2, "Content 2 longer").unwrap();
        
        // Create tar archive
        let files = vec![test_file1, test_file2];
        pack(test_tar, &files);
        
        // Execute list function
        let headers = list(test_tar).unwrap();
        
        // Verify results
        assert_eq!(headers.len(), 2);
        assert_eq!(headers[0].name, test_file1);
        assert_eq!(headers[0].size, 9);
        assert_eq!(headers[1].name, test_file2);
        assert_eq!(headers[1].size, 16);
        
        // Verify tar file contents directly
        let tar_data = fs::read(test_tar).unwrap();
        let entries = read_tar(&tar_data);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].header.name, test_file1);
        assert_eq!(entries[0].header.size, 9);
        assert_eq!(entries[1].header.name, test_file2);
        assert_eq!(entries[1].header.size, 16);
        
        // Cleanup
        fs::remove_file(test_file1).unwrap();
        fs::remove_file(test_file2).unwrap();
        fs::remove_file(test_tar).unwrap();
    }

    #[test]
    fn test_tar_gz() {
        // Create test files
        let test_file1 = "test_gz_file1.txt";
        let test_file2 = "test_gz_file2.txt";
        let test_tar_gz = "test_pack.tar.gz";
        let output_dir = "test_gz_output";
        
        fs::write(test_file1, "GZ test content 1").unwrap();
        fs::write(test_file2, "GZ test content 2 longer").unwrap();
        
        // Execute pack function (.tar.gz format)
        let files = vec![test_file1, test_file2];
        pack(test_tar_gz, &files);
        
        // Verify .tar.gz file was created
        assert!(Path::new(test_tar_gz).exists());
        
        // Get file list with list function (from .tar.gz)
        let headers = list(test_tar_gz).unwrap();
        assert_eq!(headers.len(), 2);
        assert_eq!(headers[0].name, test_file1);
        assert_eq!(headers[0].size, 17);
        assert_eq!(headers[1].name, test_file2);
        assert_eq!(headers[1].size, 24);
        
        // Execute unpack function (extract from .tar.gz)
        unpack_with_options(test_tar_gz, output_dir, false, false);
        
        // Verify files were extracted
        let extracted_file1 = Path::new(output_dir).join(test_file1);
        let extracted_file2 = Path::new(output_dir).join(test_file2);
        assert!(extracted_file1.exists());
        assert!(extracted_file2.exists());
        
        // Verify file contents
        let content1 = fs::read_to_string(&extracted_file1).unwrap();
        let content2 = fs::read_to_string(&extracted_file2).unwrap();
        assert_eq!(content1, "GZ test content 1");
        assert_eq!(content2, "GZ test content 2 longer");
        
        // Cleanup
        fs::remove_file(test_file1).unwrap();
        fs::remove_file(test_file2).unwrap();
        fs::remove_file(test_tar_gz).unwrap();
        fs::remove_dir_all(output_dir).unwrap();
    }

    #[test]
    fn test_pack_directory() {
        // Create test directory structure
        let test_dir = "test_pack_dir";
        let test_tar = "test_pack_dir.tar";
        
        fs::create_dir_all(format!("{}/subdir", test_dir)).unwrap();
        fs::write(format!("{}/file1.txt", test_dir), "File 1 content").unwrap();
        fs::write(format!("{}/file2.txt", test_dir), "File 2 content").unwrap();
        fs::write(format!("{}/subdir/file3.txt", test_dir), "File 3 in subdir").unwrap();
        
        // Pack directory
        let files = vec![test_dir];
        pack(test_tar, &files);
        
        // Verify tar file was created
        assert!(Path::new(test_tar).exists());
        
        // Verify tar file contents
        let tar_data = fs::read(test_tar).unwrap();
        let entries = read_tar(&tar_data);
        assert_eq!(entries.len(), 3);
        
        // Verify file names (should be stored as relative paths)
        let names: Vec<String> = entries.iter().map(|e| e.header.name.clone()).collect();
        assert!(names.contains(&"file1.txt".to_string()));
        assert!(names.contains(&"file2.txt".to_string()));
        assert!(names.contains(&"subdir/file3.txt".to_string()));
        
        // Cleanup
        fs::remove_dir_all(test_dir).unwrap();
        fs::remove_file(test_tar).unwrap();
    }

    #[test]
    fn test_pack_and_unpack_directory() {
        // Create test directory structure
        let test_dir = "test_dir_full";
        let test_tar = "test_dir_full.tar";
        let output_dir = "test_dir_full_output";
        
        fs::create_dir_all(format!("{}/a/b/c", test_dir)).unwrap();
        fs::write(format!("{}/root.txt", test_dir), "Root file").unwrap();
        fs::write(format!("{}/a/file_a.txt", test_dir), "File in a").unwrap();
        fs::write(format!("{}/a/b/file_b.txt", test_dir), "File in b").unwrap();
        fs::write(format!("{}/a/b/c/file_c.txt", test_dir), "File in c").unwrap();
        
        // Pack directory
        let files = vec![test_dir];
        pack(test_tar, &files);
        
        // unpack
        unpack_with_options(test_tar, output_dir, false, false);
        
        // Verify all files were extracted
        assert!(Path::new(output_dir).join("root.txt").exists());
        assert!(Path::new(output_dir).join("a/file_a.txt").exists());
        assert!(Path::new(output_dir).join("a/b/file_b.txt").exists());
        assert!(Path::new(output_dir).join("a/b/c/file_c.txt").exists());
        
        // Verify file content
        let content = fs::read_to_string(Path::new(output_dir).join("a/b/c/file_c.txt")).unwrap();
        assert_eq!(content, "File in c");
        
        // Cleanup
        fs::remove_dir_all(test_dir).unwrap();
        fs::remove_file(test_tar).unwrap();
        fs::remove_dir_all(output_dir).unwrap();
    }

    #[test]
    fn test_pack_mixed_files_and_directories() {
        // Create test file and directory
        let test_file = "test_mixed_file.txt";
        let test_dir = "test_mixed_dir";
        let test_tar = "test_mixed.tar";
        
        fs::write(test_file, "Single file").unwrap();
        fs::create_dir_all(format!("{}/subdir", test_dir)).unwrap();
        fs::write(format!("{}/dir_file.txt", test_dir), "File in dir").unwrap();
        fs::write(format!("{}/subdir/sub_file.txt", test_dir), "File in subdir").unwrap();
        
        // Pack mixed files and directories
        let files = vec![test_file, test_dir];
        pack(test_tar, &files);
        
        // Verify tar file contents
        let tar_data = fs::read(test_tar).unwrap();
        let entries = read_tar(&tar_data);
        assert_eq!(entries.len(), 3);
        
        // Verify file names
        let names: Vec<String> = entries.iter().map(|e| e.header.name.clone()).collect();
        assert!(names.contains(&test_file.to_string()));
        assert!(names.contains(&"dir_file.txt".to_string()));
        assert!(names.contains(&"subdir/sub_file.txt".to_string()));
        
        // Cleanup
        fs::remove_file(test_file).unwrap();
        fs::remove_dir_all(test_dir).unwrap();
        fs::remove_file(test_tar).unwrap();
    }

    #[test]
    fn test_pack_directory_gzipped() {
        // Create test directory structure
        let test_dir = "test_pack_dir_gz";
        let test_tar_gz = "test_pack_dir.tar.gz";
        let output_dir = "test_pack_dir_gz_output";
        
        fs::create_dir_all(format!("{}/nested/deep", test_dir)).unwrap();
        fs::write(format!("{}/file1.txt", test_dir), "First file").unwrap();
        fs::write(format!("{}/nested/file2.txt", test_dir), "Second file").unwrap();
        fs::write(format!("{}/nested/deep/file3.txt", test_dir), "Third file").unwrap();
        
        // Pack directory (gzip compressed)
        let files = vec![test_dir];
        pack(test_tar_gz, &files);
        
        // Verify .tar.gz file was created
        assert!(Path::new(test_tar_gz).exists());
        
        // Verify contents with list
        let headers = list(test_tar_gz).unwrap();
        assert_eq!(headers.len(), 3);
        
        // Verify by unpacking
        unpack_with_options(test_tar_gz, output_dir, false, false);
        assert!(Path::new(output_dir).join("file1.txt").exists());
        assert!(Path::new(output_dir).join("nested/file2.txt").exists());
        assert!(Path::new(output_dir).join("nested/deep/file3.txt").exists());
        
        // Verify file content
        let content = fs::read_to_string(Path::new(output_dir).join("nested/deep/file3.txt")).unwrap();
        assert_eq!(content, "Third file");
        
        // Cleanup
        fs::remove_dir_all(test_dir).unwrap();
        fs::remove_file(test_tar_gz).unwrap();
        fs::remove_dir_all(output_dir).unwrap();
    }

    #[test]
    fn security_test_unpack_path_traversal() {
        // Test that unpacking with path traversal attempts is handled
        // Note: Current implementation is VULNERABLE - this test documents the risk
        
        use crate::tar::{TarEntry, TarHeader};
        
        let test_tar = "test_security_traversal.tar";
        let output_dir = "test_security_output";
        
        // Create malicious tar with path traversal
        let mut entries = Vec::new();
        
        // Attempt to write outside output directory
        let malicious_paths = vec![
            "../outside.txt",
            "../../etc/outside2.txt",
            "subdir/../../../outside3.txt",
        ];
        
        for malicious_path in malicious_paths {
            let header = TarHeader::new(malicious_path.to_string(), 0o644, 9);
            let data = b"malicious".to_vec();
            let header_bytes = header.to_bytes();
            entries.push(TarEntry { header, data, header_bytes });
        }
        
        let tar_data = write_tar(&entries);
        fs::write(test_tar, tar_data).unwrap();
        
        // This WILL create files outside the intended directory (VULNERABILITY)
        // In production, unpack should sanitize paths
        unpack_with_options(test_tar, output_dir, false, false);
        
        // Cleanup
        fs::remove_file(test_tar).unwrap();
        if Path::new(output_dir).exists() {
            fs::remove_dir_all(output_dir).ok();
        }
        // Also cleanup any files created outside (if they exist)
        fs::remove_file("outside.txt").ok();
        fs::remove_file("../outside.txt").ok();
        fs::remove_file("outside2.txt").ok();
        fs::remove_file("outside3.txt").ok();
    }

    #[test]
    fn security_test_unpack_absolute_path() {
        // Test handling of absolute paths in tar archives
        // Note: Current implementation is VULNERABLE
        
        use crate::tar::{TarEntry, TarHeader};
        
        let test_tar = "test_security_absolute.tar";
        let output_dir = "test_security_abs_output";
        
        // Create tar with absolute path (should be rejected or sanitized)
        let header = TarHeader::new("/tmp/absolute_file.txt".to_string(), 0o644, 8);
        let data = b"absolute".to_vec();
        let header_bytes = header.to_bytes();
        let entry = TarEntry { header, data, header_bytes };
        
        let tar_data = write_tar(&[entry]);
        fs::write(test_tar, tar_data).unwrap();
        
        // This may write to /tmp/absolute_file.txt (VULNERABILITY)
        unpack_with_options(test_tar, output_dir, false, false);
        
        // Cleanup
        fs::remove_file(test_tar).unwrap();
        if Path::new(output_dir).exists() {
            fs::remove_dir_all(output_dir).ok();
        }
        // Cleanup absolute path file if created
        fs::remove_file("/tmp/absolute_file.txt").ok();
    }

    #[test]
    fn security_test_unpack_large_file_size() {
        // Test handling of files with unrealistic size declarations
        
        use crate::tar::{TarEntry, TarHeader};
        
        let test_tar = "test_security_large.tar";
        let output_dir = "test_security_large_output";
        
        // Create tar with exaggerated size but small actual data
        let header = TarHeader::new("fake_large.txt".to_string(), 0o644, 5);
        let data = b"small".to_vec();
        let header_bytes = header.to_bytes();
        let entry = TarEntry { header, data, header_bytes };
        
        let tar_data = write_tar(&[entry]);
        fs::write(test_tar, tar_data).unwrap();
        
        // Should handle gracefully
        unpack_with_options(test_tar, output_dir, false, false);
        
        // Verify file was created with actual (small) size
        let extracted_file = Path::new(output_dir).join("fake_large.txt");
        if extracted_file.exists() {
            let content = fs::read(&extracted_file).unwrap();
            assert_eq!(content.len(), 5);
        }
        
        // Cleanup
        fs::remove_file(test_tar).unwrap();
        if Path::new(output_dir).exists() {
            fs::remove_dir_all(output_dir).unwrap();
        }
    }

    #[test]
    fn security_test_unpack_empty_filename() {
        // Test handling of entries with empty filenames
        
        use crate::tar::{TarEntry, TarHeader};
        
        let test_tar = "test_security_empty_name.tar";
        let output_dir = "test_security_empty_output";
        
        // Create tar with empty filename
        let header = TarHeader::new("".to_string(), 0o644, 4);
        let data = b"data".to_vec();
        let header_bytes = header.to_bytes();
        let entry = TarEntry { header, data, header_bytes };
        
        let tar_data = write_tar(&[entry]);
        fs::write(test_tar, tar_data).unwrap();
        
        // Should handle gracefully (may skip or error)
        unpack_with_options(test_tar, output_dir, false, false);
        
        // Cleanup
        fs::remove_file(test_tar).unwrap();
        if Path::new(output_dir).exists() {
            fs::remove_dir_all(output_dir).ok();
        }
    }

    #[test]
    fn security_test_unpack_special_characters() {
        // Test handling of filenames with special characters
        
        use crate::tar::{TarEntry, TarHeader};
        
        let test_tar = "test_security_special.tar";
        let output_dir = "test_security_special_output";
        
        // Create tar with special characters in filename
        let special_names = vec![
            "file\0with\0nulls.txt",
            "file\nwith\nnewlines.txt",
            "file;with;semicolons.txt",
            "file|with|pipes.txt",
        ];
        
        let mut entries = Vec::new();
        for name in special_names {
            let header = TarHeader::new(name.to_string(), 0o644, 7);
            let data = b"special".to_vec();
            let header_bytes = header.to_bytes();
            entries.push(TarEntry { header, data, header_bytes });
        }
        
        let tar_data = write_tar(&entries);
        fs::write(test_tar, tar_data).unwrap();
        
        // Should handle gracefully
        unpack_with_options(test_tar, output_dir, false, false);
        
        // Cleanup
        fs::remove_file(test_tar).unwrap();
        if Path::new(output_dir).exists() {
            fs::remove_dir_all(output_dir).ok();
        }
    }

    #[test]
    fn security_test_pack_symlink_handling() {
        // Test packing a directory that contains symlinks
        // Should verify symlinks are handled appropriately
        
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            
            let test_dir = "test_security_symlink_dir";
            let test_tar = "test_security_symlink.tar";
            
            // Create directory with a symlink
            fs::create_dir_all(test_dir).unwrap();
            fs::write(format!("{}/target.txt", test_dir), "target content").unwrap();
            
            // Create symlink
            let symlink_path = format!("{}/link.txt", test_dir);
            let target_path = format!("{}/target.txt", test_dir);
            symlink(&target_path, &symlink_path).ok(); // May fail on some systems
            
            // Pack directory
            let files = vec![test_dir];
            pack(test_tar, &files);
            
            // Verify tar was created
            assert!(Path::new(test_tar).exists());
            
            // Cleanup
            fs::remove_file(&symlink_path).ok();
            fs::remove_dir_all(test_dir).unwrap();
            fs::remove_file(test_tar).unwrap();
        }
    }

    #[test]
    fn security_test_unpack_overwrites_existing() {
        // Test that unpacking overwrites existing files
        // This could be a security concern if not properly documented
        
        use crate::tar::{TarEntry, TarHeader};
        
        let test_tar = "test_security_overwrite.tar";
        let output_dir = "test_security_overwrite_output";
        
        fs::create_dir_all(output_dir).unwrap();
        
        // Create existing file with sensitive content
        let sensitive_file = Path::new(output_dir).join("important.txt");
        fs::write(&sensitive_file, "SENSITIVE DATA").unwrap();
        
        // Create tar that will overwrite it
        let header = TarHeader::new("important.txt".to_string(), 0o644, 9);
        let data = b"overwrite".to_vec();
        let header_bytes = header.to_bytes();
        let entry = TarEntry { header, data, header_bytes };
        
        let tar_data = write_tar(&[entry]);
        fs::write(test_tar, tar_data).unwrap();
        
        // Unpack will overwrite existing file
        unpack_with_options(test_tar, output_dir, true, false);
        
        // Verify file was overwritten
        let content = fs::read_to_string(&sensitive_file).unwrap();
        assert_eq!(content, "overwrite");
        
        // Cleanup
        fs::remove_file(test_tar).unwrap();
        fs::remove_dir_all(output_dir).unwrap();
    }
}
