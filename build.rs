//! Build script for Music Minder
//!
//! Embeds the application icon and version info into the Windows executable.

fn main() {
    // Only run on Windows builds
    #[cfg(target_os = "windows")]
    {
        windows_build();
    }
}

#[cfg(target_os = "windows")]
fn windows_build() {
    // Embed icon and version info into the Windows executable
    let mut res = winresource::WindowsResource::new();

    // Set the application icon
    res.set_icon("assets/icon.ico");

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
