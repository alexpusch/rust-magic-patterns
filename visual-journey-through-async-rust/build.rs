use std::env;

fn main() {
    let python_version = String::from_utf8(std::fs::read("./.python-version").unwrap()).unwrap().trim().to_string();
    let home_dir = env::var("HOME").unwrap();
    
    // Set the library path
    println!(
        "cargo:rustc-env=LD_LIBRARY_PATH={home_dir}/.rye/py/cpython@{python_version}/lib:$LD_LIBRARY_PATH",
        
    );

    // Set Rust flags
    println!(
        "cargo:rustc-flags=-L {home_dir}/.rye/py/cpython@{python_version}/lib",
    );

    // Set PYO3 Python version for cross-compilation
    println!("cargo:rustc-env=PYO3_CROSS_PYTHON_VERSION={python_version}");
}
