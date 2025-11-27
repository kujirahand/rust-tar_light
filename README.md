# tar_light for Rust

A simple and lightweight tar archive reader and writer library in Rust.

## Features

- 📦 Pack and unpack TAR archives (`.tar`)
- 🗜️ Support for gzip compression (`.tar.gz`, `.tgz`)
- 📋 List files in archives
- 🚀 Simple and intuitive API
- 🔧 Command-line tool included
- ⚡ No external dependencies except `flate2` for gzip support

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
tar_light = "0.1"
```

Or use cargo:

```sh
cargo add tar_light
```

## Usage

### Packing files into a TAR archive

```rust
use tar_light::pack;

// Create plain TAR archive
let files = vec!["file1.txt",　"file2.txt", "dir/file3.txt"];
pack("archive.tar", &files);

// Create gzip-compressed TAR archive
pack("archive.tar.gz", &files);
```

### Unpacking files from a TAR archive

```rust
use tar_light::unpack;

// Extract plain TAR archive
unpack("archive.tar", "output_directory");

// Extract gzip-compressed TAR archive
unpack("archive.tar.gz", "output_directory");
```

### Listing files in a TAR archive

```rust
use tar_light::list;

// Works with both .tar and .tar.gz
match list("archive.tar.gz") {
    Ok(headers) => {
        println!("Files in archive:");
        for header in headers {
            println!("  {} ({} bytes)", header.name, header.size);
        }
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

### Advanced usage with low-level API

```rust
use tar_light::{read_tar, write_tar, TarEntry, TarHeader};
use std::fs;

// Reading TAR archives
let tar_data = fs::read("archive.tar").unwrap();
let entries = read_tar(&tar_data);

for entry in entries {
    println!("{}: {} bytes", entry.header.name, entry.header.size);
}

// Creating TAR archives
let mut entries = Vec::new();
let header = TarHeader::new("hello.txt".to_string(), 0o644, 12);
let data = b"Hello, World".to_vec();
let header_bytes = header.to_bytes();

entries.push(TarEntry { header, data, header_bytes });
let tar_data = write_tar(&entries);
fs::write("new_archive.tar", tar_data).unwrap();
```

## Command Line Tool

The library includes a command-line tool for basic tar operations:

```bash
# Pack files into TAR archive
cargo run -- pack archive.tar file1.txt file2.txt

# Pack files into gzip-compressed TAR archive
cargo run -- pack archive.tar.gz file1.txt file2.txt

# Unpack archive
cargo run -- unpack archive.tar output_dir

# Unpack gzip-compressed archive
cargo run -- unpack archive.tar.gz output_dir

# List files in archive
cargo run -- list archive.tar.gz
```

## Supported Formats

- `.tar` - Plain TAR archives
- `.tar.gz` - Gzip-compressed TAR archives
- `.tgz` - Gzip-compressed TAR archives (alternative extension)

The format is automatically detected based on the file extension.

## Easy Building with just

This project uses [just](https://github.com/casey/just) as a task runner, making it easy to build and test the project with simple commands. A `justfile` is provided for common tasks:

```sh
# Build the project
just build

# Build in release mode
just build-release

# Pack files into TAR archive
just pack archive.tar file1.txt file2.txt

# Unpack TAR archive
just unpack archive.tar output_dir

# List files in archive
just list archive.tar

# Clean up generated files
just clean
```

If you don't have `just` installed, you can install it with:

```sh
# macOS
brew install just

# Other platforms
cargo install just
```

## License

MIT

## Repository

- [github.com/kujirahand/rust-tar_light](https://github.com/kujirahand/rust-tar_light)
- [crates.io/crates/tar_light](https://crates.io/crates/tar_light)
- [docs.rs/tar_light](https://docs.rs/tar_light)
