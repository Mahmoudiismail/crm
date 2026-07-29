fn main() {
    let now = chrono::Utc::now();
    println!("cargo:rustc-env=BUILD_TIME={}", now.format("%d%m%Y%H%M%S"));
}
