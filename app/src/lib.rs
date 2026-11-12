//! The window of Herb's Contacts Optimizer. `ui` draws it; `main.rs` only opens it.

pub mod accounts;
pub mod osaccounts;
pub mod bulk;
pub mod editor;
pub mod theme;
pub mod toolbox;
pub mod ui;

pub const APP_NAME: &str = "Herb's Contacts Optimizer";
pub const VERSION: &str = env!("HCO_VERSION");
pub const TAGLINE: &str = "Clean up your contacts.";
pub const AUTHOR: &str = "dunkelherzmann";
pub const PROJECT_URL: &str = "https://github.com/dunkelherzmann/herbs-contacts-optimizer";
/// Leaves OpenGL out of the graphics choices on Windows, where Direct3D 12 (with its software device) and
/// Vulkan cover every PC: some OpenGL drivers put up a "LoadLibrary failed with error 126" box when they
/// are merely asked whether they work, and the program waits behind it. `WGPU_BACKEND` still overrides.
pub fn graphics(setup: &mut eframe::egui_wgpu::WgpuSetup) {
    use eframe::wgpu::Backends;
    if let (true, eframe::egui_wgpu::WgpuSetup::CreateNew(new)) = (cfg!(windows) && Backends::from_env().is_none(), setup) {
        new.instance_descriptor.backends.remove(Backends::GL);
    }
}

pub const ICON_PNG: &[u8] = include_bytes!("../assets/icon-256.png");
