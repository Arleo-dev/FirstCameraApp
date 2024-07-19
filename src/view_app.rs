use eframe::{
    self,
    egui::{Color32, Image, Pos2, Rect, Rgba, Vec2},
};
use imageproc::drawing::Canvas;
use std::{
    f64::NAN,
    ptr::null,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
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
    virtual_camera: virtualcam_rs::Camera,
    rotate: f32,
    rotate_delta: f32,
    rgb: image::Rgb<u8>,
    disco_rgb: Receiver<image::Rgb<u8>>,
    current_disco_rgb: image::Rgb<u8>,
    is_racoon: bool,
    is_disco: bool,
    timeout: u64,
    timeout_sender: Sender<u64>,
    image_pixels: Vec<u8>,
}

impl Default for ViewApp {
    fn default() -> Self {
        let (data, receive) = mpsc::channel();
        let disco_rgb = receive;
        let timeout = 300;
        let (timeout_sender, receive) = mpsc::channel();
        thread::spawn(move || {
            let mut timeout = timeout;
            let mut rng_thread = rand::thread_rng();
            loop {
                if let Ok(time) = receive.try_recv() {
                    timeout = time;
                }
                    let mut rgb = image::Rgb([0, 0, 0]);
                    rgb.0[0] = rng_thread.gen_range(10..=200);
                    rgb.0[1] = rng_thread.gen_range(10..=200);
                    rgb.0[2] = rng_thread.gen_range(10..=200);
                    let _ = data.send(rgb);
                thread::sleep(std::time::Duration::from_millis(timeout));
            }
        });
        let camera = Camera::new(
            nokhwa::utils::CameraIndex::Index(0),
            RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate),
        ).unwrap();
        
        Self {
            virtual_camera: virtualcam_rs::Camera::new(camera.resolution().width() as i32, camera.resolution().height() as i32, "Unity Video Capture").unwrap(),
            camera: camera,
            rgb: image::Rgb([0, 0, 0]),
            disco_rgb,
            current_disco_rgb: image::Rgb([0,0,0]),
            rotate: 0.0,
            rotate_delta: 0.0,
            is_racoon: false,
            is_disco: false,
            timeout,
            timeout_sender,
            image_pixels: Vec::new(),
        }
    }
}

impl ViewApp {
    fn get_color_effected_pixel(&mut self, pixel: &image::Rgba<u8>) -> image::Rgba<u8> {
        let rgb = if self.is_disco {
            if let Ok(rgb) = self.disco_rgb.try_recv() {
                self.current_disco_rgb= rgb;
            }
            self.current_disco_rgb
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

        let _ = self.virtual_camera.send(pixels);
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
        pixels
    }
}

impl eframe::App for ViewApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();
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
                if ui.add(sl_update_speed).changed() {
                    let _ = self.timeout_sender.send(self.timeout);
                }
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
        let _ = self.virtual_camera.send(pixels);
    }
}
