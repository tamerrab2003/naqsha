// Windows: put the app icon and version details into Naqsha.exe.
fn main() {
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("windows/naqsha.ico");
        res.set("ProductName", "Naqsha");
        res.set("FileDescription", "Naqsha");
        res.set("CompanyName", "Nixpx LLC");
        res.set("LegalCopyright", "© 2026 Nixpx LLC. MIT License.");
        if let Err(e) = res.compile() { println!("cargo:warning=could not embed the Windows icon: {e}"); }
    }
    println!("cargo:rerun-if-changed=windows/naqsha.ico");

    // when this build was made: a lifetime Pro licence covers versions built within its year of
    // updates (reproducible builds can pin it with SOURCE_DATE_EPOCH)
    let when = std::env::var("SOURCE_DATE_EPOCH").ok().and_then(|s| s.parse::<u64>().ok()).unwrap_or_else(|| {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
    });
    println!("cargo:rustc-env=NAQSHA_BUILD_UNIX={when}");
    println!("cargo:rerun-if-env-changed=SOURCE_DATE_EPOCH");
}
