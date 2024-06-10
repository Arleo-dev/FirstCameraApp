use std::env::Args;
use nokhwa::*;

use app::App;
use eframe;
use view_app::ViewApp;

mod view_app;
mod app;

fn main() {

    let backend = native_api_backend().unwrap();
            let devices = query(backend).unwrap();
            println!("There are {} available cameras.", devices.len());
            for device in devices {
                println!("{device}");
            }
    let options = eframe::NativeOptions::default();
    let app= get_current_app(std::env::args().len());
    let param = app;
    eframe::run_native(
        "Racoon Camera",
        options,
        Box::new(|_cc| param),
    )
    .unwrap();
}

fn get_current_app(args: usize) -> Box<dyn eframe::App> {
    if args == 2 {
        Box::new(App::default())
    } else {
        Box::new(ViewApp::default())
    }
}
