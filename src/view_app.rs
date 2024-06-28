use eframe::{
    self,
    egui::{Color32, Image, Pos2, Rect, Rgba, Vec2},
};
use imageproc::drawing::Canvas;
use std::{
    sync::{Arc, Mutex},
    thread,
};

use image::{self, GenericImageView, ImageBuffer, Pixel};
use nokhwa::{
    self,
    pixel_format::{RgbAFormat, RgbFormat},
    utils::{RequestedFormat, RequestedFormatType},
    Camera,
};
use rand::{random, Rng};

pub struct ViewApp {
    camera: nokhwa::Camera,
    my_camera: virtualcam_rs::Camera,
    rotate: f32,
    rotate_delta: f32,
    rgb: image::Rgb<u8>,
    disco_rgb: Arc<Mutex<image::Rgb<u8>>>,
    is_racoon: bool,
    is_disco: bool,
    is_start: bool,
    timeout: u64,
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
            rgb: image::Rgb([0, 0, 0]),
            disco_rgb: Mutex::new(image::Rgb([0, 0, 0])).into(),
            rotate: 0.0,
            rotate_delta: 0.0,
            is_racoon: false,
            is_disco: false,
            is_start: true,
            timeout: 0,
        }
    }
}

impl ViewApp {
    fn update_disco_value(&mut self) {
        let data = Arc::new(Mutex::new(image::Rgb([0, 0, 0])));
        self.disco_rgb = data.clone();
        thread::spawn(move || loop {
            let mut rgb = data.lock().unwrap();
            rgb.0[0] = rand::thread_rng().gen_range(10..=200);
            rgb.0[1] = rand::thread_rng().gen_range(10..=200);
            rgb.0[2] = rand::thread_rng().gen_range(10..=200);
            drop(rgb);
            thread::sleep(std::time::Duration::from_millis(300));
        });
    }

    fn get_color_effected_pixel(&mut self, pixel: &image::Rgba<u8>) -> image::Rgba<u8> {
        let rgb = if self.is_disco {
            self.disco_rgb.lock().unwrap().clone()
        } else {
            self.rgb
        };
        let r = pixel.0[0].max(rgb.0[0]);
        let g = pixel.0[1].max(rgb.0[1]);
        let b = pixel.0[2].max(rgb.0[2]);
        image::Rgba([r, g, b, pixel.0[3]])
    }

    fn get_racoon_style_image(
        &mut self,
        image: ImageBuffer<image::Rgba<u8>, Vec<u8>>,
    ) -> ImageBuffer<image::Rgba<u8>, Vec<u8>> {
        let radius: i32 = image.height() as i32 / 2;
        let (cx, cy) = (image.width() as i32 / 2, image.height() as i32 / 2);
        let mut image = imageproc::geometric_transformations::rotate_about_center(
            &image,
            self.rotate,
            imageproc::geometric_transformations::Interpolation::Nearest,
            image::Rgba([0, 0, 0, 255]),
        );

        for x in 0..image.width() {
            for y in 0..image.height() {
                let dx = x as i32 - cx;
                let dy = y as i32 - cy;
                if dx * dx + dy * dy >= radius * radius {
                    image.put_pixel(x, y, image::Rgba([0, 0, 0, 255]));
                } else {
                    let pixel = self.get_color_effected_pixel(image.get_pixel(x, y));
                    image.put_pixel(x, y, pixel);
                }
            }
        }
        self.rotate += self.rotate_delta;
        image
    }

    fn get_color_effected_image(
        &mut self,
        mut image: ImageBuffer<image::Rgba<u8>, Vec<u8>>,
    ) -> ImageBuffer<image::Rgba<u8>, Vec<u8>> {
        for x in 0..image.width() {
            for y in 0..image.height() {
                let pixel = self.get_color_effected_pixel(image.get_pixel(x, y));
                image.put_pixel(x, y, pixel);
            }
        }
        image
    }

    fn set_camera_image(&mut self) {
        let frame = self.camera.frame().unwrap();

        let mut image: ImageBuffer<image::Rgba<u8>, Vec<u8>> =
            frame.decode_image::<RgbAFormat>().unwrap();

        image = if self.is_racoon {
            self.get_racoon_style_image(image)
        } else {
            self.get_color_effected_image(image)
        };

        let pixels = self.get_pixels_from_img(image);

        let _ = self.my_camera.send(pixels);
    }

    fn get_pixels_from_img(&mut self, img: ImageBuffer<image::Rgba<u8>, Vec<u8>>) -> Vec<u8> {
        let mut pixels = Vec::new();
        for pixel in img.pixels().clone() {
            let p = *pixel;
            pixels.push(p.0[3]);
            pixels.push(p.0[2]);
            pixels.push(p.0[1]);
            pixels.push(p.0[0]);
        }
        pixels.reverse();
        return pixels;
    }
}

impl eframe::App for ViewApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();
        if self.is_start {
            self.update_disco_value();
            self.is_start = false;
        }
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            self.set_camera_image();
            let mut r = self.rgb.channels_mut()[0];
            let mut g = self.rgb.channels_mut()[1];
            let mut b = self.rgb.channels_mut()[2];
            let slr = eframe::egui::Slider::new(&mut r, 0..=255)
                .text("r")
                .text_color(Color32::RED);
            let slg = eframe::egui::Slider::new(&mut g, 0..=255)
                .text("g")
                .text_color(Color32::GREEN);
            let slb = eframe::egui::Slider::new(&mut b, 0..=255)
                .text("b")
                .text_color(Color32::BLUE);
            let racoon_cb = eframe::egui::Checkbox::new(&mut self.is_racoon, "On Racoon");
            let sl_speed =
                eframe::egui::Slider::new(&mut self.rotate_delta, -1.0..=1.0).text("speed");
            let disco_cb = eframe::egui::Checkbox::new(&mut self.is_disco, "On Disco");
            let sl_update_speed =
                eframe::egui::Slider::new(&mut self.timeout, 10..=500).text("Update Speed");
            ui.add(slr);
            ui.add(slg);
            ui.add(slb);
            ui.add(racoon_cb);
            if self.is_racoon {
                ui.add(sl_speed);
            }
            ui.add(disco_cb);
            if self.is_disco {
                ui.add(sl_update_speed);
            }
            self.rgb.channels_mut()[0] = r;
            self.rgb.channels_mut()[1] = g;
            self.rgb.channels_mut()[2] = b;
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        let path = std::env::current_dir().unwrap();
        let path = format!("{}/resources/on_exit_img.jpg", path.display());
        let img = image::open(path).unwrap().into_rgba8();
        let pixels = self.get_pixels_from_img(img);
        let _ = self.my_camera.send(pixels);
    }
}
