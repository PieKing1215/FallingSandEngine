if cargo build --profile release-lto ; then
    rm -rf package/
    mkdir package
    mkdir package/gamedir
    cp target/release-lto/fs_main.exe package/
    cp -r gamedir/assets/ package/gamedir/assets/
    cargo lichking bundle --file package/dep_licenses.txt || true
fi