use image::ImageBuffer;
use plox::{Cubic, Point, Spline};

fn lerp(p1: Point, p2: Point, t: f32) -> Point {
    let lerp_x = (1.0 - t) * p1.x + t * p2.x;
    let lerp_y = (1.0 - t) * p1.y + t * p2.y;
    Point {
        x: lerp_x,
        y: lerp_y,
    }
}

fn raster<C>(img: &mut ImageBuffer<image::Rgb<u8>, C>, spline: &Spline)
where
    C: std::ops::DerefMut + std::ops::Deref<Target = [<image::Rgb<u8> as image::Pixel>::Subpixel]>,
{
    let n = 1000;

    for Cubic(p1, p2, p3, p4) in spline.strokes() {
        for i in 0..n {
            let t = (i as f32) / (n as f32);
            // De Casteljau's Algorithm
            let q1 = lerp(*p1, *p2, t);
            let q2 = lerp(*p2, *p3, t);
            let q3 = lerp(*p3, *p4, t);

            let q4 = lerp(q1, q2, t);
            let q5 = lerp(q2, q3, t);

            let q = lerp(q4, q5, t);

            *img.get_pixel_mut((200.0 + q.x) as u32, (250.0 + q.y) as u32) =
                image::Rgb([255 as u8, 255, 255]);
        }
    }
}

fn main() {
    let spline = plox::load();

    let mut img = ImageBuffer::new(1000, 1000);

    raster(&mut img, &spline);

    image::imageops::flip_vertical_in_place(&mut img);
    img.save("test.png").expect("failed to save test image");
}
