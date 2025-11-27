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
//! ## Listing files in a TAR archive
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
use std::io::{Write, Read};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;

pub use tar::{read_tar, write_tar, TarEntry, TarHeader};

// ----------------------------------------------------------------
// simple methods for reading and writing tar archives
// ----------------------------------------------------------------
/// Packs files into a tar archive (supports .tar and .tar.gz)
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
    
    // Check if output should be gzipped
    let is_gzipped = tarfile.ends_with(".tar.gz") || tarfile.ends_with(".tgz");
    
    let result = if is_gzipped {
        // Compress with gzip
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&tar_data)
            .and_then(|_| encoder.finish())
            .and_then(|compressed| fs::write(tarfile, compressed))
    } else {
        // Write as plain tar
        fs::write(tarfile, &tar_data)
    };
    
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
    // Read file
    let file_data = match fs::read(tarfile) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error reading tar file: {}", e);
            std::process::exit(1);
        }
    };
    
    // Check if input is gzipped
    let is_gzipped = tarfile.ends_with(".tar.gz") || tarfile.ends_with(".tgz");
    
    let tar_data = if is_gzipped {
        // Decompress with gzip
        let mut decoder = GzDecoder::new(&file_data[..]);
        let mut decompressed = Vec::new();
        match decoder.read_to_end(&mut decompressed) {
            Ok(_) => decompressed,
            Err(e) => {
                eprintln!("Error decompressing gzip: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        file_data
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

/// Lists TarHeader in a tar archive (supports .tar and .tar.gz)
pub fn list(tarfile: &str) -> Result<Vec<TarHeader>, std::io::Error> {
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
    let headers: Vec<TarHeader> = entries.into_iter().map(|e| e.header).collect();
    Ok(headers)
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
        let headers = list(test_tar).unwrap();
        
        // 結果を確認
        assert_eq!(headers.len(), 2);
        assert_eq!(headers[0].name, test_file1);
        assert_eq!(headers[0].size, 9);
        assert_eq!(headers[1].name, test_file2);
        assert_eq!(headers[1].size, 16);
        
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

    #[test]
    fn test_tar_gz() {
        // テスト用のファイルを作成
        let test_file1 = "test_gz_file1.txt";
        let test_file2 = "test_gz_file2.txt";
        let test_tar_gz = "test_pack.tar.gz";
        let output_dir = "test_gz_output";
        
        fs::write(test_file1, "GZ test content 1").unwrap();
        fs::write(test_file2, "GZ test content 2 longer").unwrap();
        
        // pack関数を実行（.tar.gz形式）
        let file1 = test_file1.to_string();
        let file2 = test_file2.to_string();
        let files = vec![&file1, &file2];
        pack(test_tar_gz, &files);
        
        // .tar.gzファイルが作成されたことを確認
        assert!(Path::new(test_tar_gz).exists());
        
        // list関数でファイル一覧を取得（.tar.gzから）
        let headers = list(test_tar_gz).unwrap();
        assert_eq!(headers.len(), 2);
        assert_eq!(headers[0].name, test_file1);
        assert_eq!(headers[0].size, 17);
        assert_eq!(headers[1].name, test_file2);
        assert_eq!(headers[1].size, 24);
        
        // unpack関数を実行（.tar.gzから展開）
        unpack(test_tar_gz, output_dir);
        
        // ファイルが展開されたことを確認
        let extracted_file1 = Path::new(output_dir).join(test_file1);
        let extracted_file2 = Path::new(output_dir).join(test_file2);
        assert!(extracted_file1.exists());
        assert!(extracted_file2.exists());
        
        // ファイル内容を確認
        let content1 = fs::read_to_string(&extracted_file1).unwrap();
        let content2 = fs::read_to_string(&extracted_file2).unwrap();
        assert_eq!(content1, "GZ test content 1");
        assert_eq!(content2, "GZ test content 2 longer");
        
        // クリーンアップ
        fs::remove_file(test_file1).unwrap();
        fs::remove_file(test_file2).unwrap();
        fs::remove_file(test_tar_gz).unwrap();
        fs::remove_dir_all(output_dir).unwrap();
    }
}
