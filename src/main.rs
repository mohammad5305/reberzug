extern crate reberzug;
use clap::{Parser, ValueEnum};
use image::imageops::FilterType;
use reberzug::display::x11::display_image;
use std::path::PathBuf;
use x11rb::connection::Connection;

#[derive(Parser, Debug)]
#[command(name="reberzug", author, about="ueberzug replacment written in rust for showing images on terminal using child window", long_about = None)]
struct Args {
    image: PathBuf,

    #[arg(short, default_value_t = 0)]
    x: u32,

    #[arg(short, default_value_t = 0)]
    y: u32,

    #[arg(short = 'W', long)]
    width: u32,

    #[arg(short = 'H', long)]
    height: u32,

    #[arg(short, long, value_enum, default_value_t=ArgFilterType::Triangle, help= "Which filter to use for scaling")]
    filter: ArgFilterType,
}

// TODO: ugly enum arg
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum ArgFilterType {
    Nearest,
    Triangle,
    CatmullRom,
    Gaussian,
    Lanczos3,
}

impl From<ArgFilterType> for FilterType {
    fn from(filter_type: ArgFilterType) -> Self {
        match filter_type {
            ArgFilterType::Nearest => FilterType::Nearest,
            ArgFilterType::Triangle => FilterType::Triangle,
            ArgFilterType::Gaussian => FilterType::Gaussian,
            ArgFilterType::CatmullRom => FilterType::CatmullRom,
            ArgFilterType::Lanczos3 => FilterType::Lanczos3,
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
        args.width,
        args.height,
        args.filter.into(),
    )
    .expect("Failed to display image");
    loop {
        let event = conn.wait_for_event().unwrap();
        println!("{event:?}");
    }
}
