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

detail tarfile:
    cargo run -- list_detail {{tarfile}}

clean:
    rm *.tar
    rm *.tar.gz
    rm -f -r output_directory

test:
    cargo test
    just pack src.tar.gz src
    just detail src.tar.gz
