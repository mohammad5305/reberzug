extern crate reberzug;
extern crate fast_image_resize;

use clap::{Parser, ValueEnum};
use reberzug::display::x11::display_image;
use std::{path::PathBuf, num::NonZeroU32};
use x11rb::connection::Connection;
use fast_image_resize::{ResizeAlg, FilterType};

#[derive(Parser, Debug)]
#[command(name="reberzug", author, about="ueberzug replacment written in rust for showing images on terminal using child window", long_about = None)]
struct Args {
    image: PathBuf,

    #[arg(short, default_value_t = 0)]
    x: u32,

    #[arg(short, default_value_t = 0)]
    y: u32,

    #[arg(short = 'W', long)]
    width: NonZeroU32,

    #[arg(short = 'H', long)]
    height: NonZeroU32,

    #[arg(short, long, value_enum, default_value_t=ArgFilterType::Bilinear, help= "Which algorithm to use for resizing")]
    resize_alg: ArgFilterType,
}

// TODO: ugly enum arg
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum ArgFilterType {
    Nearest,
    Box,
    Bilinear,
    Hamming,
    CatmullRom,
    Mitchell,
    Lanczos3,
}

impl From<ArgFilterType> for ResizeAlg {
    fn from(filter_type: ArgFilterType) -> Self {
        match filter_type {
            ArgFilterType::Nearest => ResizeAlg::Nearest,
            ArgFilterType::CatmullRom => ResizeAlg::Convolution(FilterType::CatmullRom),
            ArgFilterType::Lanczos3 => ResizeAlg::Convolution(FilterType::Lanczos3),
            ArgFilterType::Bilinear => ResizeAlg::Convolution(FilterType::Bilinear),
            ArgFilterType::Box => ResizeAlg::Convolution(FilterType::Box),
            ArgFilterType::Hamming => ResizeAlg::Convolution(FilterType::Hamming),
            ArgFilterType::Mitchell => ResizeAlg::Convolution(FilterType::Mitchell),
        }
    }
}

fn main() {
    let args = Args::parse();
    // TODO: process exit or panic?
    let conn = display_image(
        args.image,
        args.x as u16,
        args.y as u16,
        args.width.into(),
        args.height.into(),
        args.resize_alg.into(),
    )
    .expect("Failed to display image");
    loop {
        let event = conn.wait_for_event().unwrap();
        println!("{event:?}");
    }
}
