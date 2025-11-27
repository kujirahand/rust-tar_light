# Justfile for tar_light (task runner: just)

default:
  @just --list

build:
    cargo build

build-release:
    cargo build --release

pack tarfile *source_files:
    cargo run -- pack {{tarfile}} {{source_files}}

unpack tarfile output_directory:
    cargo run -- unpack {{tarfile}} {{output_directory}}

list tarfile:
    cargo run -- list {{tarfile}}

clean:
    rm *.tar
    rm *.tar.gz
    rm -f -r output_directory
