use eframe::{self, egui::Color32};
use fast_image_resize::images::Image;
use image::{self, ImageBuffer, Pixel, Rgba};
use imageproc::geometric_transformations::Border;
use imageproc::{drawing::draw_hollow_rect_mut, rect::Rect};

use nokhwa::{
    self,
    pixel_format::{RgbAFormat, RgbFormat},
    utils::{RequestedFormat, RequestedFormatType},
    Camera,
};
use onnxruntime::ndarray::*;
use onnxruntime::{
    environment::Environment, session::Session, GraphOptimizationLevel, LoggingLevel,
};
use std::cell::RefCell;
use std::sync::OnceLock;
use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

const MAX_ZOOM_FACTOR: f32 = 10f32;
const CONFIDENCE_THRESHOLD: f32 = 0.55;
static ONNX_ENV: OnceLock<Environment> = OnceLock::new();

fn get_onnx_env() -> &'static Environment {
    ONNX_ENV.get_or_init(|| {
        Environment::builder()
            .with_name("face_detection")
            .with_log_level(LoggingLevel::Warning)
            .build()
            .expect("Failed to create ONNX environment")
    })
}

pub struct ViewApp {
    camera: nokhwa::Camera,
    virtual_camera: virtualcam_rs::Camera,
    rotate: f32,
    zoom_factor: f32,
    rotate_delta: f32,
    rgb: image::Rgb<u8>,
    disco_rgb: Receiver<image::Rgb<u8>>,
    current_disco_rgb: image::Rgb<u8>,
    is_racoon: bool,
    is_disco: bool,
    is_zoom: bool,
    timeout: u64,
    timeout_sender: Sender<u64>,
    ort_session: Session<'static>,
    previous_score: f32,
    previous_box: (u32, u32, u32, u32),
    frame_count: u32,
}

impl Default for ViewApp {
    fn default() -> Self {
        let (data, receive) = mpsc::channel();
        let disco_rgb = receive;
        let timeout = 300;
        let (timeout_sender, receive) = mpsc::channel();
        thread::spawn(move || {
            let mut timeout = timeout;
            loop {
                if let Ok(time) = receive.try_recv() {
                    timeout = time;
                }
                let mut rgb = image::Rgb([0, 0, 0]);
                rgb.0[0] = rand::random_range(10..=200);
                rgb.0[1] = rand::random_range(10..=200);
                rgb.0[2] = rand::random_range(10..=200);
                let _ = data.send(rgb);
                thread::sleep(std::time::Duration::from_millis(timeout));
            }
        });
        let camera = Camera::new(
            nokhwa::utils::CameraIndex::Index(0),
            RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate),
        )
        .unwrap();

        let path = std::env::current_dir().unwrap();
        let path = format!("{}/resources/version-RFB-640.onnx", path.display());

        let env = get_onnx_env();

        let ort_session = env
            .new_session_builder()
            .expect("Failed to create ONNX session builder")
            .with_optimization_level(GraphOptimizationLevel::Extended)
            .expect("Failed to set optimization level")
            .with_number_threads(4)
            .expect("Failed to set threads count")
            .with_model_from_file(path)
            .expect("Failed to load ONNX model");

        Self {
            virtual_camera: virtualcam_rs::Camera::new(
                camera.resolution().width() as i32,
                camera.resolution().height() as i32,
                "Unity Video Capture",
            )
            .unwrap(),
            camera: camera,
            rgb: image::Rgb([0, 0, 0]),
            disco_rgb,
            current_disco_rgb: image::Rgb([0, 0, 0]),
            rotate: 0.0,
            zoom_factor: 2f32,
            rotate_delta: 0.0,
            is_racoon: false,
            is_disco: false,
            is_zoom: false,
            timeout,
            timeout_sender,
            ort_session,
            previous_score: 0.0,
            previous_box: (0, 0, 0, 0),
            frame_count: 0,
        }
    }
}

impl ViewApp {
    fn get_zoomed_face(
        &mut self,
        mut frame: ImageBuffer<image::Rgba<u8>, Vec<u8>>,
    ) -> ImageBuffer<image::Rgba<u8>, Vec<u8>> {
        let orig_w = frame.width();
        let orig_h = frame.height();
        let model_h = 480;
        let model_w = 640;
        if self.frame_count % 5 == 0 {
            let resized_frame = get_resized_image(&frame, model_w, model_h);
            let mut tensor = Array4::<f32>::zeros((1, 3, model_h as usize, model_w as usize));
            for (x, y, pixel) in resized_frame.enumerate_pixels() {
                tensor[[0, 0, y as usize, x as usize]] = pixel[0] as f32 / 255.0;
                tensor[[0, 1, y as usize, x as usize]] = pixel[1] as f32 / 255.0;
                tensor[[0, 2, y as usize, x as usize]] = pixel[2] as f32 / 255.0;
            }

            let (scores_array, boxes_array): (ArrayD<f32>, ArrayD<f32>) = {
                let outputs = match self.ort_session.run(vec![tensor]) {
                    Ok(o) => o,
                    Err(e) => {
                        eprintln!("❌ ONNX inference failed: {:?}", e);
                        return frame;
                    }
                };
                (outputs[0].to_owned(), outputs[1].to_owned())
            };

            let scores_slice = scores_array.index_axis(Axis(0), 0);

            let (i, score) = scores_slice
                .axis_iter(Axis(0))
                .enumerate()
                .max_by(|(_, x), (_, y)| {
                    x[1].partial_cmp(&y[1]).unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap();

            let face_score = score[1];
            if face_score > CONFIDENCE_THRESHOLD {
                let boxes_batch = boxes_array.index_axis(Axis(0), 0);
                let box_coords = boxes_batch.index_axis(Axis(0), i);
                self.previous_score = face_score;
                let x_min_norm = box_coords[0];
                let y_min_norm = box_coords[1];
                let x_max_norm = box_coords[2];
                let y_max_norm = box_coords[3];

                let (x, y, w, h) = get_box_size_with_scale(
                    self.zoom_factor,
                    (x_min_norm, y_min_norm, x_max_norm, y_max_norm),
                    orig_w,
                    orig_h,
                );

                let a = ((x as i32 - self.previous_box.0 as i32).abs()) as u32;
                let b = ((y as i32 - self.previous_box.1 as i32).abs()) as u32;

                if self.previous_box == (0, 0, 0, 0)
                    || a > self.previous_box.2 / 4
                    || b > self.previous_box.3 / 4
                    || (w < self.previous_box.2 && h < self.previous_box.3)
                    || (w > self.previous_box.2 && h > self.previous_box.3)
                {
                    self.previous_box = (x, y, w, h);
                }
            }
        }

        let x = self.previous_box.0;
        let y = self.previous_box.1;
        let w = self.previous_box.2;
        let h = self.previous_box.3;

        // FOR DEBUG
        // let rect = Rect::at(x as i32, y as i32).of_size(w, h);
        // draw_hollow_rect_mut(&mut frame, rect, Rgba([0, 255, 100, 255]));

        let cropped_face = image::imageops::crop_imm(&frame, x, y, w, h).to_image();
        frame = get_resized_image(&cropped_face, orig_w, orig_h);

        frame
    }

    fn get_color_effected_pixel(&mut self, pixel: &image::Rgba<u8>) -> image::Rgba<u8> {
        let rgb = if self.is_disco {
            if let Ok(rgb) = self.disco_rgb.try_recv() {
                self.current_disco_rgb = rgb;
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
            Border::Constant(image::Rgba([0, 0, 0, 255])),
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
        match self.camera.frame() {
            Ok(frame) => {
                let mut image: ImageBuffer<image::Rgba<u8>, Vec<u8>> =
                    frame.decode_image::<RgbAFormat>().unwrap();

                if self.is_zoom {
                    image = self.get_zoomed_face(image);
                };

                image = if self.is_racoon {
                    self.get_racoon_style_image(image)
                } else {
                    self.get_color_effected_image(image)
                };

                let pixels = self.get_pixels_from_img(image);

                let _ = self.virtual_camera.send(pixels);
            }
            Err(e) => {
                eprintln!("Dropped a frame or MSMF backend lagged: {:?}", e);
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
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
    fn ui(&mut self, ui: &mut eframe::egui::Ui, frame: &mut eframe::Frame) {
        ui.request_repaint();

        eframe::egui::CentralPanel::default().show(ui, |ui: &mut eframe::egui::Ui| {
            self.frame_count += 1;
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
            let disco_cb = eframe::egui::Checkbox::new(&mut self.is_disco, "On Disco");
            let zoom_cb = eframe::egui::Checkbox::new(&mut self.is_zoom, "On Zoom");
            ui.add(slr);
            ui.add(slg);
            ui.add(slb);
            ui.add(racoon_cb);
            if self.is_racoon {
                let sl_speed =
                    eframe::egui::Slider::new(&mut self.rotate_delta, -1.0..=1.0).text("speed");
                ui.add(sl_speed);
            }else {
                self.rotate = 0f32;
                self.rotate_delta = 0f32;
            }

            ui.add(zoom_cb);
            if self.is_zoom {
                let sl_zoom = eframe::egui::Slider::new(&mut self.zoom_factor, 1f32..=MAX_ZOOM_FACTOR)
                .text("zoom");
                ui.add(sl_zoom);
            }

            ui.add(disco_cb);
            if self.is_disco {
                
            
            let sl_update_speed =
                eframe::egui::Slider::new(&mut self.timeout, 10..=500).text("Update Speed");
                if ui.add(sl_update_speed).changed() {
                    let _ = self.timeout_sender.send(self.timeout);
                }
            }

            self.rgb.channels_mut()[0] = r;
            self.rgb.channels_mut()[1] = g;
            self.rgb.channels_mut()[2] = b;
        });
    }

    fn on_exit(&mut self) {
        let path = std::env::current_dir().unwrap();
        let path = format!("{}/resources/on_exit_img.jpg", path.display());
        let mut img = image::open(path).unwrap().into_rgba8();
        let res = self.camera.resolution();
        img = get_resized_image(&img, res.width(), res.height());
        let pixels = self.get_pixels_from_img(img);
        let _ = self.virtual_camera.send(pixels);
    }
}

fn get_box_size_with_scale(
    zoom_factor: f32,
    box_size: (f32, f32, f32, f32),
    orig_w: u32,
    orig_h: u32,
) -> (u32, u32, u32, u32) {
    let mut x = box_size.0 * orig_w as f32;
    let mut y = box_size.1 * orig_h as f32;
    let mut w = (box_size.2 - box_size.0) * orig_w as f32;
    let mut h = (box_size.3 - box_size.1) * orig_h as f32;

    let cx = x as f32 + w as f32 / 2.0;
    let cy = y as f32 + h as f32 / 2.0;

    w = orig_w as f32 / zoom_factor;
    h = orig_h as f32 / zoom_factor;

    x = (cx - w / 2.0).round();
    y = (cy - h / 2.0).round();

    let x = (x.round() as u32).max(0) as u32;
    let y = (y.round() as u32).max(0) as u32;
    let w = (w.round() as u32).min(orig_w - x);
    let h = (h.round() as u32).min(orig_h - y);

    (x, y, w, h)
}

fn get_resized_image(
    image: &ImageBuffer<image::Rgba<u8>, Vec<u8>>,
    resize_w: u32,
    resize_h: u32,
) -> ImageBuffer<image::Rgba<u8>, Vec<u8>> {
    let orig_w = image.width();
    let orig_h = image.height();
    let mut resizer = fast_image_resize::Resizer::new();
    let image_raw = image.as_raw();
    let image = Image::from_vec_u8(
        orig_w,
        orig_h,
        image_raw.clone(),
        fast_image_resize::PixelType::U8x4,
    )
    .expect("Failed to create crop view");

    let mut resized = Image::new(resize_w, resize_h, fast_image_resize::PixelType::U8x4);
    resizer.resize(&image, &mut resized, None).unwrap();

    let resized_frame: ImageBuffer<image::Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_vec(resize_w, resize_h, resized.into_vec()).unwrap();
    resized_frame
}
