use std::{
    fs::{self, File, OpenOptions},
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
    str::FromStr,
};

use image::ImageReader;
use rayon::prelude::*;

use crate::image::{
    codecs::OutputFormat,
    resize::{Dimension, ResizeType, resize},
};

use anyhow::{Result, anyhow};

pub struct Job {
    resize_type: ResizeType,
    size: Option<String>,
    output_format: OutputFormat,
    input: PathBuf,
    quality: u8,
    background: Option<[u8; 3]>,
}

impl Job {
    pub fn new(
        resize_type: ResizeType,
        size: Option<String>,
        output_format: OutputFormat,
        input: PathBuf,
        quality: u8,
        background: Option<[u8; 3]>,
    ) -> Self {
        Job {
            resize_type,
            size,
            output_format,
            input,
            quality,
            background,
        }
    }
}

pub fn process(jobs: Vec<Job>, outdir: &Path) -> Vec<(PathBuf, anyhow::Error)> {
    jobs.par_iter()
        .filter_map(|job| run(job, outdir).err().map(|e| (job.input.clone(), e)))
        .collect()
}

fn run(job: &Job, outdir: &Path) -> Result<()> {
    let stem = job
        .input
        .file_stem()
        .ok_or_else(|| anyhow!("invalid image path: {}", job.input.display()))?
        .to_string_lossy();

    let encoder = job.output_format.encoder(job.quality, job.background);

    let mut img = ImageReader::open(&job.input)?
        .with_guessed_format()?
        .decode()?;

    if let Some(size) = &job.size {
        let dimension = Dimension::from_str(size)?;

        resize(&mut img, dimension, job.resize_type.clone());
    };

    let base = if job.size.is_some() {
        format!("{stem}_{}", job.size.to_owned().unwrap_or(String::from("")))
    } else {
        stem.to_string()
    };

    let (file, output) = create_unique(outdir, &base, encoder.extension())?;

    let mut out = BufWriter::new(file);
    let res = encoder
        .encode(&img, &mut out)
        .and_then(|_| Ok(out.flush()?));

    if res.is_err() {
        drop(out);
        let _ = fs::remove_file(&output);
    }

    res
}

fn create_unique(dir: &Path, stem: &str, ext: &str) -> Result<(File, PathBuf)> {
    let mut n = 0u32;
    loop {
        let name = if n == 0 {
            format!("{stem}.{ext}")
        } else {
            format!("{stem}({n}).{ext}")
        };
        let path = dir.join(name);

        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((file, path)),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => n += 1,
            Err(e) => return Err(e.into()),
        }
    }
}
