fn main() {
    println!("cargo:rerun-if-changed=Cargo.toml");

    // Capture the local compilation time with timezone abbreviation (e.g. 2026-09-04 08:59:00 EDT)
    let date_str = std::process::Command::new("date")
        .args(["+%Y-%m-%d %H:%M:%S %Z"])
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                String::from_utf8(out.stdout).ok().map(|s| s.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=BUILD_TIME={date_str}");
}
