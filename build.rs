use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");

    let commit_sha = Command::new("git")
        .args(["rev-parse", "--short=12", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|sha| sha.trim().to_string())
        .filter(|sha| !sha.is_empty())
        .or_else(|| {
            std::env::var("GITHUB_SHA")
                .ok()
                .map(|sha| sha.chars().take(12).collect::<String>())
                .filter(|sha| !sha.is_empty())
        })
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=GIT_COMMIT_SHA={}", commit_sha);

    let now = chrono::Utc::now();
    println!("cargo:rustc-env=BUILD_TIME={}", now.format("%d%m%Y%H%M%S"));
}
