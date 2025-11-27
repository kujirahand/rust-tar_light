use tar_light::{pack, unpack, list};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        print_usage();
        std::process::exit(1);
    }
    
    let command = &args[1];
    
    match command.as_str() {
        "pack" => {
            if args.len() < 4 {
                eprintln!("Error: pack requires at least tarfile and one input file");
                print_usage();
                std::process::exit(1);
            }
            let tarfile = &args[2];
            let files: Vec<&str> = args[3..].iter().map(|s| s.as_str()).collect();
            pack(tarfile, &files);
        }
        "unpack" => {
            if args.len() < 4 {
                eprintln!("Error: unpack requires tarfile and output directory");
                print_usage();
                std::process::exit(1);
            }
            let tarfile = &args[2];
            let output_dir = &args[3];
            unpack(tarfile, output_dir);
        }
        "list" => {
            if args.len() < 3 {
                eprintln!("Error: list requires tarfile");
                print_usage();
                std::process::exit(1);
            }
            let tarfile = &args[2];
            match list(tarfile) {
                Ok(headers) => {
                    println!("Files in {}:", tarfile);
                    println!("{:>10}  {}", "Size", "Name");
                    println!("{}", "-".repeat(50));
                    for header in &headers {
                        println!("{:>10}  {}", header.size, header.name);
                    }
                    println!("\nTotal: {} file(s)", headers.len());
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        _ => {
            eprintln!("Error: Unknown command '{}'", command);
            print_usage();
            std::process::exit(1);
        }
    }
}

fn print_usage() {
    eprintln!("Usage:");
    eprintln!("  pack <tarfile> <file1> <file2> ... - Create tar archive");
    eprintln!("  unpack <tarfile> <directory>      - Extract tar archive");
    eprintln!("  list <tarfile>                     - List files in tar archive");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    #[test]
    fn test_pack_command() {
        // Create test files
        let test_file1 = "test_main_file1.txt";
        let test_file2 = "test_main_file2.txt";
        let test_tar = "test_main_pack.tar";
        
        fs::write(test_file1, "Test content 1").unwrap();
        fs::write(test_file2, "Test content 2").unwrap();
        
        // Execute pack function
        let files = vec![test_file1, test_file2];
        pack(test_tar, &files);
        
        // Verify tar file was created
        assert!(Path::new(test_tar).exists());
        
        // Cleanup
        fs::remove_file(test_file1).unwrap();
        fs::remove_file(test_file2).unwrap();
        fs::remove_file(test_tar).unwrap();
    }

    #[test]
    fn test_unpack_command() {
        // Create test file and tar archive
        let test_file = "test_main_unpack_file.txt";
        let test_content = "Main unpack test";
        let test_tar = "test_main_unpack.tar";
        let output_dir = "test_main_unpack_output";
        
        fs::write(test_file, test_content).unwrap();
        
        // Create tar archive
        let files = vec![test_file];
        pack(test_tar, &files);
        
        // Execute unpack function
        unpack(test_tar, output_dir);
        
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
    fn test_list_command() {
        // Create test files and tar archive
        let test_file1 = "test_main_list_file1.txt";
        let test_file2 = "test_main_list_file2.txt";
        let test_tar = "test_main_list.tar";
        
        fs::write(test_file1, "List test 1").unwrap();
        fs::write(test_file2, "List test 2").unwrap();
        
        // Create tar archive
        let files = vec![test_file1, test_file2];
        pack(test_tar, &files);
        
        // Execute list function
        let headers = list(test_tar).unwrap();
        
        // Verify results
        assert_eq!(headers.len(), 2);
        assert_eq!(headers[0].name, test_file1);
        assert_eq!(headers[0].size, 11);
        assert_eq!(headers[1].name, test_file2);
        assert_eq!(headers[1].size, 11);
        
        // Cleanup
        fs::remove_file(test_file1).unwrap();
        fs::remove_file(test_file2).unwrap();
        fs::remove_file(test_tar).unwrap();
    }

    #[test]
    fn test_simple() {
        let files = vec![
            "src/main.rs",
            "src/lib.rs",
            "src/tar.rs",
            "Cargo.toml",
        ];
        pack("a.tar.gz", &files);
        let headers = list("a.tar.gz").unwrap();
        assert!(headers.len() == 4);
        // cleanup
        fs::remove_file("a.tar.gz").unwrap();
    }

}
