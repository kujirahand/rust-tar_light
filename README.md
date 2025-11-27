# tar_light for Rust

A simple tar archive reader and writer library in Rust.
Only packs and unpacks basic tar files without compression.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
tar_light = "0.1.0"
```

## Sample Usage

### Packing files into a TAR archive

```rust
use tar_light::pack;

let file1 = "file1.txt".to_string();
let file2 = "file2.txt".to_string();
let files = vec![&file1, &file2];

pack("archive.tar", &files);
// Creates archive.tar containing file1.txt and file2.txt
```

### Unpacking files from a TAR archive

```rust
use tar_light::unpack;

unpack("archive.tar", "output_directory");
// Extracts all files from archive.tar to output_directory/
```

### Listing files in a TAR archive

```rust
use tar_light::list;

match list("archive.tar") {
    Ok(headers) => {
        println!("Files in archive:");
        for header in headers {
            println!("  {} ({} bytes)", header.name, header.size);
        }
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

### Command line tool

```bash
# Pack files
cargo run -- pack archive.tar file1.txt file2.txt

# Unpack archive
cargo run -- unpack archive.tar output_dir

# List files in archive
cargo run -- list archive.tar
```

