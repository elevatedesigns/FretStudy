//! On Windows, embeds the app icon and version info in FretStudy.exe.
fn main() {
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../assets/fretstudy.ico");
        res.set("ProductName", "FretStudy");
        res.set("FileDescription", "FretStudy guitar lessons");
        if let Err(e) = res.compile() {
            println!("cargo:warning=couldn't embed the Windows icon: {e}");
        }
    }
    println!("cargo:rerun-if-changed=../../assets/fretstudy.ico");
}
