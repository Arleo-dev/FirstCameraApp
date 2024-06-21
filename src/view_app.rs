use eframe::{
    self,
    egui::{Color32, Image, Pos2, Rect, Rgba, Vec2},
};
use std::f32::consts::PI;

use nokhwa::{
    self,
    pixel_format::{RgbAFormat, RgbFormat},
    utils::{RequestedFormat, RequestedFormatType},
    Camera,
};
use rand::{random, Rng};
use image;

pub struct ViewApp {
    camera: nokhwa::Camera,
    my_camera: virtualcam_rs::Camera,
    rotate:f32,
    rotateD:f32,
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
            my_camera: virtualcam_rs::Camera::new(1280, 720, "Unity Video Capture").unwrap(),
            r: 0,
            g: 0,
            b: 0,
            rotate: 0.0,
            rotateD: 0.0,
        }
    }
}

impl ViewApp {
    fn set_camera_image(&mut self) {
        let frame = self.camera.frame().unwrap();

        let image = frame.decode_image::<RgbAFormat>().unwrap();
        let radius: i32 = image.height() as i32 / 2;
        let (cx, cy) = (image.width() as i32 / 2, image.height() as i32 / 2);

        let mut image = imageproc::geometric_transformations::rotate_about_center(
            &image, 
            self.rotate,  
            imageproc::geometric_transformations::Interpolation::Nearest, 
            image::Rgba([0, 0, 0, 255]));

        for x in 0..image.width() {
            for y in 0..image.height() {
                let dx = x as i32 - cx;
                let dy = y as i32 - cy;
                if dx * dx + dy * dy >= radius * radius {
                    image.put_pixel(x, y, image::Rgba([0, 0, 0, 255]));
                }
            }
        }
        let mut pixels = Vec::new();
        for pixel in image.pixels().clone() {
            pixels.push(pixel.0[3]);
            pixels.push(pixel.0[2].max(self.b));
            pixels.push(pixel.0[1].max(self.g));
            pixels.push(pixel.0[0].max(self.r));
        }
        pixels.reverse();
        let _ = self.my_camera.send(pixels);
        self.rotate += self.rotateD;
    }
}

impl eframe::App for ViewApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            self.set_camera_image();
            let slr = eframe::egui::Slider::new(&mut self.r, 0..=255)
                .text("r")
                .text_color(Color32::RED);
            let slg = eframe::egui::Slider::new(&mut self.g, 0..=255)
                .text("g")
                .text_color(Color32::GREEN);
            let slb = eframe::egui::Slider::new(&mut self.b, 0..=255)
                .text("b")
                .text_color(Color32::BLUE);
            let slspeed = eframe::egui::Slider::new(&mut self.rotateD, -1.0..=1.0)
                .text("speed");
            ui.add(slr);
            ui.add(slg);
            ui.add(slb);
            ui.add(slspeed);
        });
    }
}
