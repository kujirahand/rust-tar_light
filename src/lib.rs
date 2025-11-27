//! Simple tar archive reader and writer library
//!
//! # Usage
//!
//! ## Packing files into a TAR archive
//!
//! ```rust
//! use tar_light::pack;
//!
//! let file1 = "file1.txt".to_string();
//! let file2 = "file2.txt".to_string();
//! let files = vec![&file1, &file2];
//! 
//! pack("archive.tar", &files);
//! // Creates archive.tar containing file1.txt and file2.txt
//! ```
//!
//! ## Unpacking files from a TAR archive
//!
//! ```rust
//! use tar_light::unpack;
//!
//! unpack("archive.tar", "output_directory");
//! // Extracts all files from archive.tar to output_directory/
//! ```
//!
//! ## Listing files in a TAR archive
//!
//! ```rust
//! use tar_light::list;
//!
//! match list("archive.tar") {
//!     Ok(filenames) => {
//!         println!("Files in archive:");
//!         for name in filenames {
//!             println!("  {}", name);
//!         }
//!     }
//!     Err(e) => eprintln!("Error: {}", e),
//! }
//! ```
//!
//! ## Advanced usage with low-level API
//!
//! ```rust
//! use tar_light::{read_tar, write_tar, TarEntry, TarHeader};
//! use std::fs;
//!
//! // Reading TAR archives
//! let tar_data = fs::read("archive.tar").unwrap();
//! let entries = read_tar(&tar_data);
//!
//! for entry in entries {
//!     println!("{}: {} bytes", entry.header.name, entry.header.size);
//! }
//!
//! // Creating TAR archives
//! let mut entries = Vec::new();
//! let header = TarHeader::new("hello.txt".to_string(), 0o644, 12);
//! let data = b"Hello, World".to_vec();
//! let header_bytes = header.to_bytes();
//!
//! entries.push(TarEntry { header, data, header_bytes });
//! let tar_data = write_tar(&entries);
//! fs::write("new_archive.tar", tar_data).unwrap();
//! ```

pub mod tar;

use std::fs;
use std::path::Path;
use std::io::Write;

pub use tar::{read_tar, write_tar, TarEntry, TarHeader};

// ----------------------------------------------------------------
// simple methods for reading and writing tar archives
// ----------------------------------------------------------------
/// Packs files into a tar archive
pub fn pack(tarfile: &str, files: &[&String]) {
    let mut entries = Vec::new();
    
    for file_path in files {
        let path = Path::new(file_path);
        if !path.exists() {
            eprintln!("Warning: File not found: {}", file_path);
            continue;
        }
        
        let data = match fs::read(path) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Error reading {}: {}", file_path, e);
                continue;
            }
        };
        
        let filename = path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        
        let header = TarHeader::new(filename, 0o644, data.len() as u64);
        let header_bytes = header.to_bytes();
        
        entries.push(TarEntry {
            header,
            data,
            header_bytes,
        });
    }
    
    let tar_data = write_tar(&entries);
    
    match fs::write(tarfile, &tar_data) {
        Ok(_) => println!("Created tar archive: {}", tarfile),
        Err(e) => {
            eprintln!("Error writing tar file: {}", e);
            std::process::exit(1);
        }
    }
}

/// Unpacks files from a tar archive
pub fn unpack(tarfile: &str, output_dir: &str) {
    let tar_data = match fs::read(tarfile) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error reading tar file: {}", e);
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
        
        match fs::File::create(&file_path) {
            Ok(mut file) => {
                if let Err(e) = file.write_all(&entry.data) {
                    eprintln!("Error writing {}: {}", entry.header.name, e);
                } else {
                    println!("Extracted: {}", entry.header.name);
                }
            }
            Err(e) => {
                eprintln!("Error creating {}: {}", entry.header.name, e);
            }
        }
    }
    
    println!("Extraction complete to: {}", output_dir);
}

/// Lists files in a tar archive
pub fn list(tarfile: &str) -> Result<Vec<String>, std::io::Error> {
    let tar_data = fs::read(tarfile)?;
    
    let entries = read_tar(&tar_data);
    
    println!("Files in {}:", tarfile);
    println!("{:>10}  {}", "Size", "Name");
    println!("{}", "-".repeat(50));
    
    let mut filenames = Vec::new();
    for entry in entries {
        println!("{:>10}  {}", entry.header.size, entry.header.name);
        filenames.push(entry.header.name);
    }
    
    println!("\nTotal: {} file(s)", filenames.len());
    
    Ok(filenames)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    #[test]
    fn test_pack() {
        // テスト用のファイルを作成
        let test_file1 = "test_file1.txt";
        let test_file2 = "test_file2.txt";
        let test_tar = "test_pack.tar";
        
        fs::write(test_file1, "Hello, World!").unwrap();
        fs::write(test_file2, "Test content 2").unwrap();
        
        // pack関数を実行
        let file1 = test_file1.to_string();
        let file2 = test_file2.to_string();
        let files = vec![&file1, &file2];
        pack(test_tar, &files);
        
        // tarファイルが作成されたことを確認
        assert!(Path::new(test_tar).exists());
        
        // tarファイルの内容を確認
        let tar_data = fs::read(test_tar).unwrap();
        let entries = read_tar(&tar_data);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].header.name, test_file1);
        assert_eq!(entries[1].header.name, test_file2);
        
        // クリーンアップ
        fs::remove_file(test_file1).unwrap();
        fs::remove_file(test_file2).unwrap();
        fs::remove_file(test_tar).unwrap();
    }

    #[test]
    fn test_unpack() {
        // テスト用のファイルとtarアーカイブを作成
        let test_file = "test_unpack_file.txt";
        let test_content = "Unpack test content";
        let test_tar = "test_unpack.tar";
        let output_dir = "test_unpack_output";
        
        fs::write(test_file, test_content).unwrap();
        
        // tarアーカイブを作成
        let file = test_file.to_string();
        let files = vec![&file];
        pack(test_tar, &files);
        
        // unpack関数を実行
        unpack(test_tar, output_dir);
        
        // ファイルが展開されたことを確認
        let extracted_file = Path::new(output_dir).join(test_file);
        assert!(extracted_file.exists());
        
        // ファイル内容を確認
        let content = fs::read_to_string(&extracted_file).unwrap();
        assert_eq!(content, test_content);
        
        // クリーンアップ
        fs::remove_file(test_file).unwrap();
        fs::remove_file(test_tar).unwrap();
        fs::remove_dir_all(output_dir).unwrap();
    }

    #[test]
    fn test_list() {
        // テスト用のファイルとtarアーカイブを作成
        let test_file1 = "test_list_file1.txt";
        let test_file2 = "test_list_file2.txt";
        let test_tar = "test_list.tar";
        
        fs::write(test_file1, "Content 1").unwrap();
        fs::write(test_file2, "Content 2 longer").unwrap();
        
        // tarアーカイブを作成
        let file1 = test_file1.to_string();
        let file2 = test_file2.to_string();
        let files = vec![&file1, &file2];
        pack(test_tar, &files);
        
        // list関数を実行
        let result = list(test_tar).unwrap();
        
        // 結果を確認
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], test_file1);
        assert_eq!(result[1], test_file2);
        
        // tarファイルの内容を直接確認
        let tar_data = fs::read(test_tar).unwrap();
        let entries = read_tar(&tar_data);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].header.name, test_file1);
        assert_eq!(entries[0].header.size, 9);
        assert_eq!(entries[1].header.name, test_file2);
        assert_eq!(entries[1].header.size, 16);
        
        // クリーンアップ
        fs::remove_file(test_file1).unwrap();
        fs::remove_file(test_file2).unwrap();
        fs::remove_file(test_tar).unwrap();
    }
}
