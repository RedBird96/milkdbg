pushd ./examples/readfile
rustc main.rs --target=i686-pc-windows-msvc
popd
# set-item env:/RUST_LOG "trace"
cargo run -- --path ./examples/readfile/debug.js
# rm env:/RUST_LOG