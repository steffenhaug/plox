use image::ImageBuffer;
use plox::{Point, Spline};

#[macro_use]
extern crate glium;

/// Software rasterization of a spline to an image. Mostly for debugging purposes.
fn raster<C>(img: &mut ImageBuffer<image::Rgb<u8>, C>, spline: Spline)
where
    C: std::ops::DerefMut + std::ops::Deref<Target = [<image::Rgb<u8> as image::Pixel>::Subpixel]>,
{
    let spline = spline.scale(0.3);
    let n = 500;
    let (w, h) = img.dimensions();
    let (off_x, off_y) = (10.0, 75.0); // offset to render at


    for y in (0..h).step_by(2) {
        for x in (0..w).step_by(2) {
            let p = Point {
                x: x as f32,
                y: y as f32 - off_y,
            };
            let win = spline.winding_number(p);
            if win != 0 {
                *img.get_pixel_mut((off_x + x as f32) as u32, (y as f32) as u32) =
                    image::Rgb::<u8>([200, 200, 200]);
            }
        }
    }

    for bezier in spline.strokes() {
        for i in 0..n {
            let t = (i as f32) / (n as f32);
            let q = bezier.at(t);
            let d = bezier.dy().at(t);

            *img.get_pixel_mut((off_x + q.x) as u32, (off_y + q.y) as u32) = if d > 0.0 {
                image::Rgb::<u8>([150, 150, 150])
            } else {
                image::Rgb::<u8>([50, 50, 50])
            }
        }
    }
}

fn main() {
    let spline = plox::load();

    let mut img = ImageBuffer::from_pixel(1000, 300, image::Rgb::<u8>([255, 255, 255]));

    raster(&mut img, spline);

    image::imageops::flip_vertical_in_place(&mut img);
    img.save("test.png").expect("failed to save test image");
}
