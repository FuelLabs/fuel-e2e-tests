use std::env;

mod utils;

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    println!("cargo:rerun-if-changed=build.rs");
}
