fn main() {
    let build_time = chrono::Utc::now();
    println!(
        "cargo:rustc-env=BUILD_TIME={}",
        build_time.format("%d%m%Y%H%M%S")
    );
}
