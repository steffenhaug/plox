use image::ImageBuffer;
use plox::{spline, Cubic, Point, Spline};

fn raster<C>(img: &mut ImageBuffer<image::Rgb<u8>, C>, spline: &Spline)
where
    C: std::ops::DerefMut + std::ops::Deref<Target = [<image::Rgb<u8> as image::Pixel>::Subpixel]>,
{
    let n = 1000;
    let (w, _h) = img.dimensions();
    let (off_x, off_y) = (200.0, 250.0); // offset to render at

    for bezier in spline.strokes() {
        for i in 0..n {
            let t = (i as f32) / (n as f32);

            let q = bezier.at(t);
            *img.get_pixel_mut((off_x + q.x) as u32, (off_y + q.y) as u32) =
                image::Rgb([255 as u8, 255, 255]);
        }
    }

    let y = 200.0; // y value we are solving for
    for i in 0..w {
        *img.get_pixel_mut(i, (off_y + y) as u32) = image::Rgb([0 as u8, 255, 255]);
    }

    for bez @ Cubic(p0, p1, p2, p3) in spline.strokes() {
        let solns = spline::solve(p0.y - y, p1.y - y, p2.y - y, p3.y - y);
        if let Some(t) = solns.get(0) {
            if *t > 0.0 && *t < 1.0 {
                let Point { x, y } = bez.at(*t);
                println!(
                    "Found solution B({}) = ({}, {}). B = {:?} {:?} {:?} {:?}",
                    t, x, y, p0, p1, p2, p3,
                );
                *img.get_pixel_mut((x.round() + off_x) as u32, (y.round() + off_y) as u32) =
                    image::Rgb([255 as u8, 255, 0]);
            }
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
