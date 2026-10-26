fn main() {
    for path in ["build.rs", "Cargo.toml", "Cargo.lock", "src", "data"] {
        println!("cargo:rerun-if-changed={path}");
    }
    println!(
        "cargo:rustc-env=BCFUNL_BUILD_DATE={}",
        chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ")
    );
}
