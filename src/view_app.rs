
use eframe::{
    self,
    egui::{self, vec2, widgets, Painter, Pos2, Rect, Shape, Vec2},
    emath::Rot2,
    epaint::image,
    glow::Buffer,
};
use nokhwa::{
    self,
    pixel_format::RgbFormat,
    utils::{RequestedFormat, RequestedFormatType},
    Camera,
};

pub struct ViewApp {
    camera: nokhwa::Camera,
    data: Vec<u8>,
    rotate: f32,
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
        frame
            .decode_image_to_buffer::<RgbFormat>(&mut self.data)
            .unwrap();
        eframe::egui::ColorImage::from_rgb(
            [
                frame.resolution().width() as usize,
                frame.resolution().height() as usize,
            ],
            &self.data,
        )
    }  

    fn rotate_point_around_center(point: Pos2, center: Pos2, angle: f32) -> Pos2 {
        let sin = angle.sin();
        let cos = angle.cos();
        
        // Translate point to origin
        let translated_x = point.x - center.x;
        let translated_y = point.y - center.y;
    
        // Rotate point
        let rotated_x = translated_x * cos - translated_y * sin;
        let rotated_y = translated_x * sin + translated_y * cos;
    
        // Translate point back
        Pos2 {
            x: rotated_x + center.x,
            y: rotated_y + center.y,
        }
    }
    
    fn get_sized_rect(rect: Rect, size: f32) -> Rect {
        let center = rect.center();
        
        // Get the four corners of the rectangle
        let top_left = rect.min;
        let top_right = Pos2::new(rect.max.x, rect.min.y);
        let bottom_left = Pos2::new(rect.min.x, rect.max.y);
        let bottom_right = rect.max;
    
        // Rotate the corners around the center
        let top_left_rotated = Self::rotate_point_around_center(top_left, center, size);
        let top_right_rotated = Self::rotate_point_around_center(top_right, center, size);
        let bottom_left_rotated = Self::rotate_point_around_center(bottom_left, center, size);
        let bottom_right_rotated = Self::rotate_point_around_center(bottom_right, center, size);
    
        // Find the minimum and maximum x and y coordinates
        let min_x = top_left_rotated.x.min(top_right_rotated.x).min(bottom_left_rotated.x).min(bottom_right_rotated.x);
        let max_x = top_left_rotated.x.max(top_right_rotated.x).max(bottom_left_rotated.x).max(bottom_right_rotated.x);
        let min_y = top_left_rotated.y.min(top_right_rotated.y).min(bottom_left_rotated.y).min(bottom_right_rotated.y);
        let max_y = top_left_rotated.y.max(top_right_rotated.y).max(bottom_left_rotated.y).max(bottom_right_rotated.y);
    
        // Return the axis-aligned bounding box
        Rect::from_min_max(Pos2::new(min_x, min_y), Pos2::new(max_x, max_y))
    }

}

impl eframe::App for ViewApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            let img = self.get_camera_image();

            let tex = ui
                .ctx()
                .load_texture("frame", img, eframe::egui::TextureOptions::LINEAR);
            let rect = eframe::egui::Rect::from_center_size(
                eframe::egui::Pos2::new(350., 350.),
                eframe::egui::Vec2::new(400., 400.),
            );
            let rot = ViewApp::get_sized_rect(rect, self.rotate);

            eframe::egui::Image::new(&tex)
                .rounding(1000.)
                .paint_at(ui, rot);
            self.rotate += 0.1;
        });
    }
}
