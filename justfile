build:
    cargo build

build-release:
    cargo build --release

clean:
    rm *.tar
    rm *.tar.gz
    rm -f -r output_directory
