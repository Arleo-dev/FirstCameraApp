use std::process::{Child, Stdio};

use eframe::{
    self,
    egui::{self},
};
pub struct App {
    text: String,
    view_proc: Option<Child>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            text: Default::default(),
            view_proc: Default::default(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let label = egui::Label::new(&self.text);
            if !self.text.is_empty() {
                ui.add(label);
            }
            if ui.add_enabled(self.view_proc.is_none(),egui::Button::new("Click me")).clicked() {
                self.text = "PIPISISI".to_string();
                self.view_proc = Some(
                    std::process::Command::new(std::env::current_exe().unwrap())
                        .stdin(Stdio::piped())
                        .arg("text")
                        .spawn()
                        .unwrap(),
                );
                
            }
        });
    }
}
