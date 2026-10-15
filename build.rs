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
}
