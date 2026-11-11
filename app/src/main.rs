#![cfg_attr(windows, windows_subsystem = "windows")]

use herbs_contacts_optimizer::{ui::AppState, APP_NAME, ICON_PNG};

struct Window(AppState);

impl eframe::App for Window {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.0.draw(ui);
    }
}

/// `--self-check`: proves the built program starts and its core works, without opening a window.
fn self_check() -> bool {
    let card = "BEGIN:VCARD\r\nVERSION:3.0\r\nFN:Self Check\r\nTEL:600 111 222\r\nEND:VCARD\r\n";
    let read = hco_core::read(card.as_bytes());
    let ok = matches!(&read, Ok((hco_core::Format::VCard3, contacts)) if contacts.len() == 1 && contacts[0].name() == "Self Check")
        && hco_core::Format::WRITABLE.iter().all(|format| read.as_ref().is_ok_and(|(_, contacts)| format.write(contacts).is_some_and(|bytes| hco_core::read(&bytes).is_ok_and(|(found, back)| found == *format && back.len() == 1))))
        && hco_core::countries::COUNTRIES.len() > 200;
    println!("{APP_NAME} {} self-check: {}", herbs_contacts_optimizer::VERSION, if ok { "passed" } else { "FAILED" });
    ok
}

fn main() -> eframe::Result {
    if std::env::args().any(|arg| arg == "--self-check") {
        std::process::exit(if self_check() { 0 } else { 1 });
    }
    let mut viewport = egui::ViewportBuilder::default()
        .with_title(APP_NAME)
        .with_app_id("org.herbscontactsoptimizer.app")
        .with_inner_size([1100.0, 780.0])
        .with_min_inner_size([760.0, 440.0]);
    if let Ok(icon) = eframe::icon_data::from_png_bytes(ICON_PNG) {
        viewport = viewport.with_icon(icon);
    }
    // Every system draws its own title bar, so the toolbar below it is the same on all three.
    let options = eframe::NativeOptions { viewport, ..Default::default() };
    eframe::run_native(
        APP_NAME,
        options,
        Box::new(|cc| {
            let mut state = AppState::new(&cc.egui_ctx);
            state.load_settings();
            for path in std::env::args_os().skip(1).filter(|arg| !arg.to_string_lossy().starts_with("--")) {
                state.open_path(path.into());
            }
            Ok(Box::new(Window(state)))
        }),
    )
}
