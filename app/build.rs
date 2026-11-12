//! Hands the app its version: `version.txt` when a build script wrote one (the Flatpak and Windows
//! builds, which have no `sh` or no git history), else VERSION plus the commits since it changed
//! (scripts/version.sh). On Windows it also puts the icon into the program file.
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=../VERSION");
    println!("cargo:rerun-if-changed=../version.txt");
    println!("cargo:rerun-if-changed=../.git/logs/HEAD");
    let written = std::fs::read_to_string("../version.txt").ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
    let counted = || {
        Command::new("sh")
            .arg("../scripts/version.sh")
            .output()
            .ok()
            .filter(|out| out.status.success())
            .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
            .filter(|v| !v.is_empty())
    };
    let version = written.or_else(counted).unwrap_or_else(|| format!("{}.0", std::fs::read_to_string("../VERSION").unwrap_or_default().trim()));
    println!("cargo:rustc-env=HCO_VERSION={version}");

    #[cfg(target_os = "macos")]
    {
        println!("cargo:rerun-if-changed=macos/contacts.m");
        cc::Build::new().file("macos/contacts.m").flag("-fobjc-arc").compile("hco_contacts");
        println!("cargo:rustc-link-lib=framework=Contacts");
        println!("cargo:rustc-link-lib=framework=Foundation");
    }

    #[cfg(windows)]
    {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("assets/icon.ico");
        // The curved apostrophe: winresource writes the straight one with a backslash before it.
        resource.set("ProductName", "Herb\u{2019}s Contacts Optimizer");
        resource.set("FileDescription", "Herb\u{2019}s Contacts Optimizer");
        resource.set("LegalCopyright", "Herbert Cisneros");
        resource.compile().expect("the Windows icon resource");
    }
}
