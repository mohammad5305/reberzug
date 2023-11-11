extern crate anyhow;
extern crate image;
extern crate x11rb;

use anyhow::{Context, Result};
use image::{io::Reader, DynamicImage};
use std::{
    borrow::Cow,
    num::NonZeroU32,
    path::PathBuf,
};
use x11rb::{
    connection::Connection,
    image::{BitsPerPixel, ColorComponent, Image, ImageOrder, PixelLayout, ScanlinePad},
    protocol::xproto::*,
    rust_connection::{ParseError::InvalidValue, RustConnection},
    wrapper::ConnectionExt as _,
};

use fast_image_resize as fr;

use sysinfo::{Pid, PidExt, ProcessExt, ProcessRefreshKind, RefreshKind, System, SystemExt};

use std::env::var;

fn get_ppid(pid: u32) -> Option<Vec<u32>> {
    let process_tree = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    );
    let mut parents: Vec<u32> = Vec::with_capacity(20);
    let mut current_pid = pid;
    while current_pid != 1 {
        let current_process = process_tree
            .process(Pid::from_u32(current_pid))
            .expect("failed");

        current_pid = current_process.parent().expect("failed").as_u32();
        parents.push(current_pid);
    }
    Some(parents)
}

fn get_parent_winid(_conn: &RustConnection, _root: u32) -> Result<u32> {
    Ok(var("WINDOWID")
        .context("Failed to get parent Window id")?
        .parse::<u32>()?)

    // TODO: getting window id with pid is tricky find a way to handle it
    // below isn't a correct and working example and sometimes needs to be plused by 12 or 5 idk why :)
    //
    // let windows = query_tree(conn, root).expect("failed to get qureytree").reply().expect("No cookie reply");
    // let ppids: Vec<u32> = get_ppid(process::id()).expect("no ppid found");
    // for window in windows.children {
    //     let spec = ClientIdSpec {
    //         client: window,
    //         mask: ClientIdMask::from(2 as u32),
    //     };
    //     if let Ok(window) = query_client_ids(conn, &[spec]) {
    //         for id in window.reply().expect("Failed to get reply").ids {
    //             if let Some(value) = id.value.get(0) {
    //                 dbg!(&id);
    //                 if ppids.contains(value) {
    //                     dbg!(id.spec.client + 12);
    //                     return id.spec.client;
    //                 }
    //             }
    //         }
    //     }
    // }
}

fn check_visual(screen: &Screen, id: Visualid) -> Option<PixelLayout> {
    // TODO: refactor this
    // Find the information about the visual and at the same time check its depth.
    let (depth, visual_type) = screen
        .allowed_depths
        .iter()
        .filter_map(|depth| {
            let info = depth.visuals.iter().find(|depth| depth.visual_id == id);
            info.map(|info| (depth.depth, info))
        })
        .next()?;
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
    Some(result)
}

fn create_image<'a>(
    conn: &'a RustConnection,
    img: &'a [u8],
    width: u32,
    height: u32,
    pixel_layout: PixelLayout,
) -> Result<Image<'a>> {
    let x11_image = Image::new(
        width as u16,
        height as u16,
        ScanlinePad::Pad8,
        24,
        BitsPerPixel::B24,
        ImageOrder::MsbFirst,
        Cow::Borrowed(img),
    )?;

    let image_layout = PixelLayout::new(
        ColorComponent::new(8, 16)?,
        ColorComponent::new(8, 8)?,
        ColorComponent::new(8, 0)?,
    );
    let x11_image = x11_image.reencode(image_layout, pixel_layout, conn.setup())?;

    Ok(x11_image.into_owned())
}

fn set_title(conn: &RustConnection, win_id: u32, title: &str) {
    let _ = conn.change_property8(
        PropMode::REPLACE,
        win_id,
        AtomEnum::WM_NAME,
        AtomEnum::STRING,
        title.as_bytes(),
    );
}

fn resize_image(
    img: DynamicImage,
    width: NonZeroU32,
    height: NonZeroU32,
    algorithm: fr::ResizeAlg,
) -> Result<Vec<u8>> {
    let src_image = fr::Image::from_vec_u8(
        NonZeroU32::new(img.width()).unwrap(),
        NonZeroU32::new(img.height()).unwrap(),
        img.to_rgb8().into_raw(),
        fr::PixelType::U8x3,
    )?;

    let mut dst_image = fr::Image::new(width, height, src_image.pixel_type());

    let mut resizer = fr::Resizer::new(algorithm);
    resizer.resize(&src_image.view(), &mut dst_image.view_mut())?;

    Ok(dst_image.into_vec())
}
pub fn display_image(
    image_path: PathBuf,
    x: u16,
    y: u16,
    width: u32,
    height: u32,
    resize_alg: fr::ResizeAlg,
) -> Result<RustConnection> {
    let image = Reader::open(image_path)
        .context("Invalid Image format")?
        .decode()?;

    let (conn, screen_num) = x11rb::connect(None).context("Failed to connect to x11")?;
    let screen = &conn.setup().roots[screen_num];

    let depth = screen.root_depth;
    let root = screen.root;

    let pixmap = conn.generate_id()?;
    conn.create_pixmap(depth, pixmap, root, width as u16, height as u16)?;

    let gc = conn.generate_id()?;
    create_gc(&conn, gc, pixmap, &CreateGCAux::new())?;

    let pixel_layout = check_visual(screen, screen.root_visual).ok_or(InvalidValue)?;

    let resized_image = resize_image(
        image,
        NonZeroU32::new(width).unwrap(),
        NonZeroU32::new(height).unwrap(),
        resize_alg,
    )?;
    let x11_image = create_image(&conn, resized_image.as_slice(), width, height, pixel_layout);
    x11_image?.put(&conn, pixmap, gc, 0, 0)?;

    let win = conn.generate_id()?;
    let window_aux = CreateWindowAux::new()
        .border_pixel(screen.black_pixel)
        .background_pixmap(pixmap);

    let parent_id = get_parent_winid(&conn, root)?;

    conn.create_window(
        depth,
        win,
        parent_id,
        x.try_into()?,
        y.try_into()?,
        width.try_into()?,
        height.try_into()?,
        0,
        WindowClass::COPY_FROM_PARENT,
        screen.root_visual,
        &window_aux,
    )?;

    set_title(&conn, win, "reberzug");
    conn.map_window(win)?;
    conn.flush()?;
    Ok(conn)
}
