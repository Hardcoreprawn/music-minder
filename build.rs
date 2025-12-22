//! Build script for Music Minder
//!
//! Embeds the application icon and version info into the Windows executable.

fn main() {
    // Only run when targeting Windows
    // Note: In build scripts, we check CARGO_CFG_TARGET_OS env var, not #[cfg]
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "windows" {
        windows_build();
    }
}

fn windows_build() {
    // Get the project root directory (where Cargo.toml is)
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let icon_path = std::path::Path::new(&manifest_dir)
        .join("assets")
        .join("icon.ico");

    // Embed icon and version info into the Windows executable
    let mut res = winresource::WindowsResource::new();

    // Set the application icon (use absolute path)
    res.set_icon(icon_path.to_str().expect("Invalid icon path"));

    // Set version info from Cargo.toml
    res.set("ProductName", "Music Minder");
    res.set("FileDescription", "Music Library Manager");
    res.set("CompanyName", "Hardcoreprawn");
    res.set("LegalCopyright", "MIT License");

    // Compile the resources
    if let Err(e) = res.compile() {
        eprintln!("Failed to compile Windows resources: {}", e);
        // Don't fail the build - icon is optional
    }
}
