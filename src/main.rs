use nokhwa::*;

use eframe;
use view_app::ViewApp;

mod view_app;

fn main() {

    let backend = native_api_backend().unwrap();
            let devices = query(backend).unwrap();
            println!("There are {} available cameras.", devices.len());
            for device in devices {
                println!("{device}");
            }
    let options = eframe::NativeOptions::default();
    let app= Box::new(ViewApp::default());
    eframe::run_native(
        "Racoon Camera",
        options,
        Box::new(|_cc| app),
    )
    .unwrap();
}
