use anyhow::Result;
use clap::ValueEnum;
use image::{
    DynamicImage, ExtendedColorType, ImageEncoder, Rgb, RgbImage,
    codecs::{
        avif::AvifEncoder,
        jpeg::JpegEncoder,
        png::{CompressionType, FilterType, PngEncoder},
    },
};
use std::io::Write;

#[derive(Clone, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Jpeg,
    Png,
    Webp,
    Avif,
}

impl OutputFormat {
    pub fn encoder(&self, quality: u8, bg: Option<[u8; 3]>) -> Box<dyn Encoder> {
        match self {
            Self::Jpeg => Box::new(Jpeg { quality, bg }),
            Self::Png => Box::new(Png { quality }),
            Self::Webp => Box::new(Webp { quality }),
            Self::Avif => Box::new(Avif { quality }),
        }
    }
}

pub trait Encoder {
    fn encode(&self, img: &DynamicImage, out: &mut dyn Write) -> Result<()>;
    fn extension(&self) -> &'static str;
}

fn add_background(img: &DynamicImage, bg: Option<[u8; 3]>) -> RgbImage {
    let rgba = img.to_rgba8();
    let mut output = RgbImage::new(rgba.width(), rgba.height());

    for (x, y, p) in rgba.enumerate_pixels() {
        let a = p[3] as u32;
        let mix = |fg: u8, bg: u8| ((fg as u32 * a + bg as u32 * (255 - a) + 127) / 255) as u8;

        let bg = bg.unwrap_or([255, 255, 255]);

        output.put_pixel(
            x,
            y,
            Rgb([mix(p[0], bg[0]), mix(p[1], bg[1]), mix(p[2], bg[2])]),
        );
    }

    output
}

pub struct Jpeg {
    pub quality: u8,
    pub bg: Option<[u8; 3]>,
}

impl Encoder for Jpeg {
    fn encode(&self, img: &DynamicImage, out: &mut dyn Write) -> Result<()> {
        let out_img = add_background(img, self.bg);

        JpegEncoder::new_with_quality(out, self.quality).write_image(
            &out_img,
            out_img.width(),
            out_img.height(),
            ExtendedColorType::Rgb8,
        )?;
        Ok(())
    }

    fn extension(&self) -> &'static str {
        "jpg"
    }
}

pub struct Png {
    pub quality: u8,
}

impl Encoder for Png {
    fn encode(&self, img: &DynamicImage, out: &mut dyn Write) -> Result<()> {
        let rgba = img.to_rgba8();
        let q = (self.quality as f32 / 10.0).clamp(0.0, 10.0).round() as u8;

        let mut compression = CompressionType::Default;

        if q < 1 {
            compression = CompressionType::Fast;
        } else if (1..9).contains(&q) {
            compression = CompressionType::Level(q)
        } else if q > 9 {
            compression = CompressionType::Best;
        }

        PngEncoder::new_with_quality(out, compression, FilterType::NoFilter).write_image(
            rgba.as_raw(),
            rgba.width(),
            rgba.height(),
            ExtendedColorType::Rgba8,
        )?;

        Ok(())
    }

    fn extension(&self) -> &'static str {
        "png"
    }
}

pub struct Webp {
    pub quality: u8,
}

impl Encoder for Webp {
    fn encode(&self, img: &DynamicImage, out: &mut dyn Write) -> Result<()> {
        let rgba = img.to_rgba8();

        let mem = webp::Encoder::from_rgba(&rgba, rgba.width(), rgba.height())
            .encode(self.quality as f32);

        out.write_all(&mem)?;

        Ok(())
    }

    fn extension(&self) -> &'static str {
        "webp"
    }
}

pub struct Avif {
    pub quality: u8,
}

impl Encoder for Avif {
    fn encode(&self, img: &DynamicImage, out: &mut dyn Write) -> Result<()> {
        let rgba = img.to_rgba8();
        AvifEncoder::new_with_speed_quality(out, 7, self.quality).write_image(
            rgba.as_raw(),
            rgba.width(),
            rgba.height(),
            ExtendedColorType::Rgba8,
        )?;
        Ok(())
    }

    fn extension(&self) -> &'static str {
        "avif"
    }
}
