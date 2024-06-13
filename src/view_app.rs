use std::*;
use eframe::{
    self,
    egui::{Color32, Pos2, Rect},
};

use nokhwa::{
    self,
    pixel_format::RgbFormat,
    utils::{RequestedFormat, RequestedFormatType},
    Camera,
};
use rand::{random, Rng};

pub struct ViewApp {
    camera: nokhwa::Camera,
    mycamera: virtualcam_rs::Camera,
    data: Vec<u8>,
    rotate: f32,
    r: u8,
    g: u8,
    b: u8,
}

impl Default for ViewApp {
    fn default() -> Self {
        Self {
            camera: Camera::new(
                nokhwa::utils::CameraIndex::Index(0),
                RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate),
            )
            .unwrap(),
            data: Vec::new(),
            rotate: 0.,
            mycamera: virtualcam_rs::Camera::new(1280, 720, "Unity Video Capture").unwrap(),
            r: 0,
            g: 0,
            b: 0,
        }
    }
}

impl ViewApp {
    fn get_camera_image(&mut self) -> eframe::egui::ColorImage {
        let frame = self.camera.frame().unwrap();
        let size = frame.resolution().width() * frame.resolution().height() * 3;
        if self.data.is_empty() {
            self.data.reserve_exact(size as usize);
            for _ in 0..size {
                self.data.push(0);
            }
        }
        frame.decode_image_to_buffer::<RgbFormat>(&mut self.data).unwrap();

        eframe::egui::ColorImage::from_rgb(
            [
                frame.resolution().width() as usize,
                frame.resolution().height() as usize,
            ],
            &self.data,
        )
    }
}

impl eframe::App for ViewApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            let img = self.get_camera_image();
            let slr = eframe::egui::Slider::new(&mut self.r, 0..=255)
                .text("r")
                .text_color(Color32::RED);
            let slg = eframe::egui::Slider::new(&mut self.g, 0..=255)
                .text("g")
                .text_color(Color32::GREEN);
            let slb = eframe::egui::Slider::new(&mut self.b, 0..=255)
                .text("b")
                .text_color(Color32::BLUE);
            ui.add(slr);
            ui.add(slg);
            ui.add(slb);
            let mut pixels = Vec::new();
            for pixel in img.pixels.clone() {
                    pixels.push(pixel.a());
                    pixels.push(pixel.b().max(self.b));
                    pixels.push(pixel.g().max(self.g));
                    pixels.push(pixel.r().max(self.r));
            }
            pixels.reverse();

            let res = self.mycamera.send(pixels.clone());
            match res {
                Ok(_) => print!(""),
                Err(_e) => println!("nok"),
            }
            // let tex = ui
            //     .ctx()
            //     .load_texture("frame", img, eframe::egui::TextureOptions::LINEAR);
            // let rect = eframe::egui::Rect::from_center_size(
            //     eframe::egui::Pos2::new(350., 350.),
            //     eframe::egui::Vec2::new(400., 400.),
            // );
            // // rect.rotate_bb(Rot2::from_angle(0.));
            // // let rot = ViewApp::get_sized_rect(rect, self.rotate);
            // eframe::egui::Image::new(&tex)
            //     //.rotate(self.rotate, eframe::egui::Vec2::splat(0.5))
            //     .paint_at(ui, rect);
            // self.rotate += 0.07;
        });
    }
}
