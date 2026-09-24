use nokhwa::*;

use eframe::{self, egui::{Vec2, accesskit::Size}};
use view_app::ViewApp;

mod view_app;

fn main() {
    let backend = native_api_backend().unwrap();
    let devices = query(backend).unwrap();
    println!("There are {} available cameras.", devices.len());
    for device in devices {
        println!("{device}");
    }
    
    let mut options = eframe::NativeOptions::default();
    options.viewport = eframe::egui::ViewportBuilder::default().with_always_on_top().with_resizable(false).with_inner_size(eframe::egui::Vec2::new(300f32, 250f32));
    let app = Box::new(ViewApp::default());
    eframe::run_native("Racoon Camera", options, Box::new(|_cc| Ok(app))).unwrap();
}
