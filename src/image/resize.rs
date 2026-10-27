use clap::ValueEnum;
use image::{DynamicImage, imageops::FilterType};
use std::str::FromStr;

#[derive(Default, Clone, PartialEq, Eq, ValueEnum)]
pub enum ResizeType {
    #[default]
    Resize,
    Exact,
    Fill,
}

pub struct Dimension {
    pub w: u32,
    pub h: u32,
}

impl FromStr for Dimension {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> std::prelude::v1::Result<Self, Self::Err> {
        let (w, h) = s
            .split_once('x')
            .ok_or_else(|| anyhow::anyhow!("invalid dimensions: expected WIDTHxHEIGHT"))?;

        Ok(Dimension {
            w: u32::from_str(w)?,
            h: u32::from_str(h)?,
        })
    }
}

pub fn resize(img: &mut DynamicImage, dimension: Dimension, resize_type: ResizeType) {
    match resize_type {
        ResizeType::Resize => *img = img.resize(dimension.w, dimension.h, FilterType::Nearest),
        ResizeType::Exact => *img = img.resize_exact(dimension.w, dimension.h, FilterType::Nearest),
        ResizeType::Fill => {
            *img = img.resize_to_fill(dimension.w, dimension.h, FilterType::Lanczos3)
        }
    }
}
