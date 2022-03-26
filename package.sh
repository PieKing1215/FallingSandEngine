if cargo build --release ; then
    rm -rf package/
    mkdir package
    mkdir package/gamedir
    cp target/release/fs_main.exe package/
    cp -r gamedir/assets/ package/gamedir/assets/
    cargo lichking bundle --file package/dep_licenses.txt || true
fi