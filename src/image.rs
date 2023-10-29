extern crate image;
extern crate x11rb;

use image::{imageops::FilterType, io::Reader, DynamicImage, GenericImageView};
use std::error::Error;
use x11rb::image::{BitsPerPixel, ColorComponent, Image, ImageOrder, PixelLayout, ScanlinePad};
use x11rb::{
    connection::Connection, protocol::xproto::*, rust_connection::RustConnection,
    wrapper::ConnectionExt as _,
};

fn check_visual(screen: &Screen, id: Visualid) -> PixelLayout {
    // Find the information about the visual and at the same time check its depth.
    let visual_info = screen
        .allowed_depths
        .iter()
        .filter_map(|depth| {
            let info = depth.visuals.iter().find(|depth| depth.visual_id == id);
            info.map(|info| (depth.depth, info))
        })
        .next();
    let (depth, visual_type) = match visual_info {
        Some(info) => info,
        None => {
            eprintln!("Did not find the root visual's description?!");
            std::process::exit(1);
        }
    };
    // Check that the pixels have red/green/blue components that we can set directly.
    match visual_type.class {
        VisualClass::TRUE_COLOR | VisualClass::DIRECT_COLOR => {}
        _ => {
            eprintln!(
                "The root visual is not true / direct color, but {:?}",
                visual_type,
            );
            std::process::exit(1);
        }
    }
    let result = PixelLayout::from_visual_type(*visual_type)
        .expect("The server sent a malformed visual type");
    assert_eq!(result.depth(), depth);
    result
}

fn create_image(
    conn: &RustConnection,
    img: DynamicImage,
    width: u32,
    height: u32,
    pixel_layout: PixelLayout,
) -> Result<Image, Box<dyn Error>> {
    let x11_image = Image::new(
        width as u16,
        height as u16,
        ScanlinePad::Pad8,
        24,
        BitsPerPixel::B24,
        ImageOrder::MsbFirst,
        img.into_rgb8().into_raw().into(),
    )?;

    let image_layout = PixelLayout::new(
        ColorComponent::new(8, 16)?,
        ColorComponent::new(8, 8)?,
        ColorComponent::new(8, 0)?,
    );
    let x11_image = x11_image.reencode(image_layout, pixel_layout, conn.setup())?;

    Ok(x11_image.into_owned())
}

fn set_title(conn: &RustConnection, title: &str) {
    conn.change_property8(
        PropMode::REPLACE,
        win,
        AtomEnum::WM_NAME,
        AtomEnum::STRING,
        title.as_bytes(),
    );
}
pub fn display_image(
    image_path: &str,
    x: u16,
    y: u16,
    width: u32,
    height: u32,
) -> Result<(), Box<dyn Error>> {
    let image = Reader::open(image_path)?.decode()?;
    // let (width, height) = calculate_image_size(width, height);
    let image = image.resize(width, height, FilterType::Nearest);
    let (width, height) = image.dimensions();

    let (conn, screen_num) = x11rb::connect(None)?;
    let screen = &conn.setup().roots[screen_num];

    let depth = screen.root_depth;
    let root = screen.root;

    let pixmap = conn.generate_id()?;
    conn.create_pixmap(depth, pixmap, root, width as u16, height as u16)?;

    let gc = conn.generate_id()?;
    create_gc(&conn, gc, pixmap, &CreateGCAux::new())?;

    let pixel_layout = check_visual(screen, screen.root_visual);
    let x11_image = create_image(&conn, image, width, height, pixel_layout);
    x11_image?.put(&conn, pixmap, gc, 0, 0)?;

    let win = conn.generate_id()?;
    let window_aux = CreateWindowAux::new()
        .border_pixel(screen.black_pixel)
        .background_pixmap(pixmap);

    set_title(&conn, "reberzug");
    conn.create_window(
        depth,
        win,
        // root,
        31457292,
        x.try_into()?,
        y.try_into()?,
        width.try_into()?,
        height.try_into()?,
        0,
        WindowClass::COPY_FROM_PARENT,
        screen.root_visual,
        &window_aux,
    )?;

    conn.map_window(win)?;
    conn.flush()?;
}
