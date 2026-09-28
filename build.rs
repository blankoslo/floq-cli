use std::env;

// ref: https://doc.rust-lang.org/cargo/reference/build-scripts.html
fn main() {
    let compile_profile = env::var("PROFILE").unwrap();

    if compile_profile != "release" {
        println!("cargo:rustc-env=FLOQ_DOMAIN=https://test.floq.no");
        println!("cargo:rustc-env=FLOQ_API_DOMAIN=https://api-test.floq.no");
    }
}
