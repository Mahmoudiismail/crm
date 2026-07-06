fn main() {
    let mut cmd = tokio::process::Command::new("cmd");
    #[cfg(target_os = "windows")]
    {
        cmd.creation_flags(0x08000000);
    }
}
