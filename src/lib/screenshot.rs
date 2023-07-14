use image::RgbaImage;
use serde_derive::{Deserialize, Serialize};
use std::io::Cursor;

use crate::config::Config;

pub fn as_png(
    data: Vec<u8>,
    width: u16,
    height: u16,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let image: RgbaImage =
        RgbaImage::from_raw(width as u32, height as u32, data).expect("Invalid image dimensions");

    let mut data = Cursor::new(Vec::new());

    image::DynamicImage::ImageRgba8(image).write_to(&mut data, image::ImageOutputFormat::Png)?;

    Ok(data.into_inner())
}

pub fn as_jpeg(
    data: Vec<u8>,
    width: u16,
    height: u16,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let image: RgbaImage =
        RgbaImage::from_raw(width as u32, height as u32, data).expect("Invalid image dimensions");

    let mut data = Cursor::new(Vec::new());

    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut data, image::ImageOutputFormat::Jpeg(100))?;

    Ok(data.into_inner())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ScreenshotType {
    Region(RegionSelection), // x, y, width, height

    #[cfg(target_os = "linux")]
    Window(x11rb::protocol::xproto::Window),
    #[cfg(target_os = "windows")]
    Window(winapi::shared::windef::HWND),

    Screen,
}

impl Config {
    // returns a Vec<u8> of the image data in PNG format
    pub fn screenshot(&self, typ: ScreenshotType) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        match self.screenshotter {
            None => match typ {
                ScreenshotType::Region(selection) => {
                    let data = capture_region(&selection)?;

                    Ok(as_png(data, selection.w, selection.h)?)
                }
                ScreenshotType::Window(window) => {
                    let (data, (w, h)) = capture_window(window)?;

                    Ok(as_png(data, w, h)?)
                }
                ScreenshotType::Screen => {
                    let (data, (w, h)) = capture_screen()?;

                    Ok(as_png(data, w, h)?)
                }
            },
            Some(ref s) => s.capture(),
        }
    }
}

pub mod custom {
    use serde_derive::{Deserialize, Serialize};

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct Screenshotter {
        pub path: String,
        pub args: Vec<String>,
    }

    impl Screenshotter {
        pub fn capture(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
            let output = std::process::Command::new(&self.path)
                .args(&self.args)
                .output()?;

            Ok(output.stdout)
        }
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use std::ffi::CString;
    use std::sync::{Arc, RwLock};

    use gdk::CursorType;
    use gdk_pixbuf::PixbufLoader;
    use serde_derive::{Deserialize, Serialize};
    use x11rb::connection::Connection;
    use x11rb::errors::ReplyOrIdError;
    use x11rb::protocol::xproto::{self, ConnectionExt, InternAtomReply, Window, GetGeometryReply};
    use x11rb::rust_connection::RustConnection;

    // Function to get the root window of the X11 display
    fn get_root_window(connection: &RustConnection) -> Result<Window, ReplyOrIdError> {
        let setup = connection.setup();
        Ok(setup.roots[0].root)
    }

    // Function to get the windows present on the screen
    pub fn get_windows() -> Result<Vec<Window>, ReplyOrIdError> {
        let (connection, _screen_num) =
            RustConnection::connect(None).expect("Failed to connect to X server");
        let root_window = get_root_window(&connection)?;

        let cookie = connection.query_tree(root_window)?;
        let reply = cookie.reply()?;

        Ok(reply.children)
    }

    struct DragData {
        start_pos: (i16, i16),
        end_pos: (i16, i16),
        dragging: bool,
        success: bool,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct RegionSelection {
        pub x: i16,
        pub y: i16,
        pub w: u16,
        pub h: u16,

        #[serde(skip_serializing, skip_deserializing)]
        pub pixbuf: Option<gdk_pixbuf::Pixbuf>
    }

    // we never access pixbuf from another thread, so it's safe.
    unsafe impl Send for RegionSelection {}

    impl From<GetGeometryReply> for RegionSelection {
        fn from(reply: GetGeometryReply) -> Self {
            Self {
                x: reply.x,
                y: reply.y,
                w: reply.width,
                h: reply.height,
                pixbuf: None
            }
        }
    }

    // Function to capture a region of the screen specified by coordinates (x, y, width, height)
    pub fn capture_region(
        selection: &RegionSelection
    ) -> Result<Vec<u8>, ReplyOrIdError> {

        // if we already have a pixbuf, likely from freeze, we should use that
        if let Some(pixbuf) = &selection.pixbuf {
            let region_pixbuf = pixbuf.new_subpixbuf(
                selection.x as i32,
                selection.y as i32,
                selection.w as i32,
                selection.h as i32
            );

            // Get the image dimensions
            let width = region_pixbuf.width();
            let height = region_pixbuf.height();

            // Determine the color components and depth
            let n_channels = region_pixbuf.n_channels() as usize;
            let row_stride = region_pixbuf.rowstride() as usize;

            // Get the pixel data
            let pixels = unsafe { region_pixbuf.pixels() };

            // Convert the pixel data to a raw image buffer
            let mut raw_image = vec![0u8; width as usize * height as usize * n_channels];
            for y in 0..height as usize {
                let src_offset = y * row_stride;
                let dst_offset = y * width as usize * n_channels;
                let row_data = &pixels[src_offset..src_offset + width as usize * n_channels];
                raw_image[dst_offset..dst_offset + width as usize * n_channels].copy_from_slice(row_data);
            }

            return Ok(raw_image);
        }

        let (connection, _screen_num) =
            RustConnection::connect(None).expect("Failed to connect to X server");
        let root_window = get_root_window(&connection)?;

        let get_image_cookie = connection.get_image(
            xproto::ImageFormat::Z_PIXMAP,
            root_window,
            selection.x,
            selection.y,
            selection.w,
            selection.h,
            std::u32::MAX,
        )?;

        let reply = get_image_cookie.reply()?;

        let image_data = reply.data.to_vec();

        // Convert byte order from BGR to RGB
        let mut rgb_data = Vec::with_capacity(image_data.len());

        for pixel in image_data.chunks_exact(4) {
            let b = pixel[0];
            let g = pixel[1];
            let r = pixel[2];
            let a = pixel[3];
            rgb_data.extend_from_slice(&[r, g, b, a]);
        }

        Ok(rgb_data)
    }

    // this function should dim the screen, then give the user a drag cursor to select a region
    pub fn select_region(freeze: bool) -> Result<RegionSelection, Box<dyn std::error::Error>> {
        use gdk::Cursor;
        use gtk::prelude::*;
        use gtk::{Window, WindowPosition, WindowType};

        let (connection, _screen_num) =
            RustConnection::connect(None).expect("Failed to connect to X server");
        let root_window = get_root_window(&connection)?;

        let get_geometry_cookie = connection.get_geometry(root_window)?;
        let get_geometry_reply = get_geometry_cookie.reply()?;

        gtk::init()?;

        // Create a GTK window with transparent background
        let window = Arc::new(Window::new(WindowType::Toplevel));
        window.set_type_hint(gdk::WindowTypeHint::Dock);
        window.set_decorated(false);
        window.set_skip_taskbar_hint(true);
        window.set_skip_pager_hint(true);
        window.set_position(WindowPosition::CenterAlways);
        window.set_app_paintable(true);

        let screen =
        gtk::prelude::GtkWindowExt::screen(window.as_ref()).ok_or("Failed to get screen")?;
        let visual = screen.rgba_visual().ok_or("Failed to get RGBA visual")?;
        window.set_visual(Some(&visual));

        // Set up a drawing area for region selection
        let drawing_area = gtk::DrawingArea::new();
        drawing_area.set_size_request(
            get_geometry_reply.width as i32,
            get_geometry_reply.height as i32,
        );
        window.add(&drawing_area);

        let area_clone = drawing_area.clone();

        let drag_data = Arc::new(RwLock::new(DragData {
            start_pos: (0, 0),
            end_pos: (0, 0),
            dragging: false,
            success: false,
        }));

        let mut pixbuf: Arc<Option<gdk_pixbuf::Pixbuf>> = Arc::new(None);

        if freeze {
            let (data, (w, h)) =  capture_screen()?;

            let data = crate::screenshot::as_png(data, w, h)?;

            let loader = PixbufLoader::new();
            loader.write(&data)?;
            loader.close()?;

            pixbuf = Arc::new(loader.pixbuf());
        }

        let drag_data_ref = Arc::clone(&drag_data);
        let pixbuf_ref = Arc::clone(&pixbuf);
        drawing_area.connect_draw(move |_, cr| {
            // if freeze, draw the frozen screen
            if freeze {
                cr.set_source_pixbuf(&pixbuf_ref.as_ref().clone().unwrap(), 0.0, 0.0);
                if let Err(e) = cr.paint() {
                    println!("Error painting freeze screen: {}", e);
                }
            }

            // Create a transparent overlay
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.5);
            if let Err(e) = cr.paint() {
                println!("Error painting overlay: {}", e);
            }

            let drag_data = drag_data_ref.write().unwrap();

            // Draw the selection rectangle
            if drag_data.dragging {
                let (x, y, width, height) = (
                    i16::min(drag_data.start_pos.0, drag_data.end_pos.0),
                    i16::min(drag_data.start_pos.1, drag_data.end_pos.1),
                    (drag_data.start_pos.0 - drag_data.end_pos.0).abs() as u16,
                    (drag_data.start_pos.1 - drag_data.end_pos.1).abs() as u16,
                );
                cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
                cr.rectangle(x as f64, y as f64, width as f64, height as f64);
                if let Err(e) = cr.stroke() {
                    println!("Error drawing rectangle: {}", e);
                }
            }

            Inhibit(false)
        });

        let drag_data_ref = Arc::clone(&drag_data);
        let window_ref = window.clone();
        drawing_area.connect_button_press_event(move |_, event| {
            let mut drag_data = drag_data_ref.write().unwrap();

            match event.button() {
                1 => {
                    drag_data.dragging = true;
                    drag_data.start_pos = (event.position().0 as i16, event.position().1 as i16);
                }
                3 => {
                    window_ref.hide();
                    gtk::main_quit();
                }
                _ => {}
            }

            Inhibit(false)
        });

        let window_ref = window.clone();
        drawing_area.connect_key_press_event(move |_, event| {
            if event.keyval() == gdk::keys::constants::Escape {
                window_ref.hide();
                gtk::main_quit();
            }

            Inhibit(false)
        });

        let drag_data_ref = Arc::clone(&drag_data);
        let window_ref = window.clone();
        drawing_area.connect_button_release_event(move |_, event| {
            let mut drag_data = drag_data_ref.write().unwrap();

            if event.button() == 1 && drag_data.dragging {
                // Left button released, stop dragging
                drag_data.dragging = false;
                drag_data.end_pos = (event.position().0 as i16, event.position().1 as i16);
                drag_data.success = true;
                window_ref.hide();
                gtk::main_quit();
            }
            Inhibit(false)
        });

        let drag_data_ref = Arc::clone(&drag_data);
        drawing_area.connect_motion_notify_event(move |_, event| {
            let mut drag_data = drag_data_ref.write().unwrap();

            if drag_data.dragging {
                // Update the selection as the user drags
                drag_data.end_pos = (event.position().0 as i16, event.position().1 as i16);
                area_clone.queue_draw();
            }
            Inhibit(false)
        });

        drawing_area.add_events(
            gdk::EventMask::BUTTON_PRESS_MASK
                | gdk::EventMask::BUTTON_RELEASE_MASK
                | gdk::EventMask::POINTER_MOTION_MASK
                | gdk::EventMask::KEY_PRESS_MASK,
        );

        // Set up a transparent cursor for the drawing area
        let display = gdk::Display::default().ok_or("Failed to get default display")?;
        let cursor = Cursor::for_display(&display, CursorType::Cross);
        drawing_area.realize();
        drawing_area
            .window()
            .ok_or("Failed to get drawing area window")?
            .set_cursor(cursor.as_ref());

        window.show_all();

        gtk::main();

        // Calculate the coordinates and dimensions of the selected region
        let drag_data = drag_data.read().unwrap();
        if !drag_data.success {
            return Err("User cancelled region selection".into());
        }

        let (x, y, w, h) = (
            i16::min(drag_data.start_pos.0, drag_data.end_pos.0),
            i16::min(drag_data.start_pos.1, drag_data.end_pos.1),
            (drag_data.start_pos.0 - drag_data.end_pos.0).abs() as u16,
            (drag_data.start_pos.1 - drag_data.end_pos.1).abs() as u16,
        );

        Ok(RegionSelection { x, y, w, h, pixbuf: Some(pixbuf.as_ref().clone().unwrap())})
    }

    pub fn get_active_window_id() -> Result<Option<Window>, ReplyOrIdError> {
        let (connection, _screen_num) =
            RustConnection::connect(None).expect("Failed to connect to X server");

        let get_input_focus_cookie = connection.get_input_focus()?;
        let reply = get_input_focus_cookie.reply()?;

        Ok(Some(reply.focus))
    }

    pub fn get_window_name(
        connection: &RustConnection,
        window: Window,
    ) -> Result<Option<String>, ReplyOrIdError> {
        let atom_name = CString::new("WM_NAME").expect("Failed to create CString");

        let intern_atom_cookie = connection.intern_atom(false, atom_name.as_bytes())?;
        let intern_atom_reply: InternAtomReply = intern_atom_cookie.reply()?;

        let cookie = connection.get_property(
            false,
            window,
            intern_atom_reply.atom,
            xproto::AtomEnum::STRING,
            0,
            std::u32::MAX,
        )?;
        let reply = cookie.reply()?;

        let property_data = reply.value.to_vec();
        let window_name = String::from_utf8_lossy(&property_data).into_owned();

        Ok(Some(window_name))
    }

    // Function to capture a specific window by its ID
    pub fn capture_window(window_id: Window) -> Result<(Vec<u8>, (u16, u16)), ReplyOrIdError> {
        let (connection, _screen_num) =
            RustConnection::connect(None).expect("Failed to connect to X server");

        let get_geometry_cookie = connection.get_geometry(window_id)?;
        let get_geometry_reply = get_geometry_cookie.reply()?;

        let capture = capture_region(&get_geometry_reply.into())?;

        Ok((
            capture,
            (get_geometry_reply.width, get_geometry_reply.height),
        ))
    }

    // Function to capture the whole screen
    pub fn capture_screen() -> Result<(Vec<u8>, (u16, u16)), ReplyOrIdError> {
        let (connection, _screen_num) =
            RustConnection::connect(None).expect("Failed to connect to X server");
        let root_window = get_root_window(&connection)?;

        let capture = capture_window(root_window)?;

        Ok(capture)
    }
}

#[cfg(target_os = "windows")]
mod windows {
    use std::io::{Error, ErrorKind};
    use std::os::windows::ffi::OsStringExt;
    use std::ptr::{null, null_mut};
    use winapi::shared::windef::HGDIOBJ;

    use winapi::shared::minwindef::{DWORD, LPARAM};
    use winapi::shared::windef::{HWND, RECT};
    use winapi::um::wingdi::{
        BitBlt, CreateDIBSection, DeleteObject, SelectObject, BITMAPINFO, BITMAPINFOHEADER,
        DIB_RGB_COLORS, RGBQUAD, SRCCOPY,
    };
    use winapi::um::winuser::{
        EnumWindows, FindWindowW, GetDC, GetWindow, GetWindowDC, GetWindowLongPtrW, GetWindowRect,
        IsWindowVisible, PrintWindow, ReleaseDC, PW_CLIENTONLY, PW_RENDERFULLCONTENT,
        WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    };
    use winapi::um::winuser::{GWL_STYLE, GW_OWNER};

    fn wide_string_to_string(wide_string: &[u16]) -> String {
        let os_string = std::ffi::OsString::from_wide(wide_string);
        os_string
            .into_string()
            .unwrap_or_else(|os_string| panic!("Failed to convert wide string: {:?}", os_string))
    }

    // Callback function for EnumWindows
    unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> i32 {
        let mut rect: RECT = std::mem::zeroed();
        let is_visible = IsWindowVisible(hwnd) != 0;
        let mut is_own_window = false;

        if is_visible {
            let owner = GetWindow(hwnd, GW_OWNER);
            is_own_window = owner.is_null();
        }

        if is_visible && is_own_window {
            let ex_style = GetWindowLongPtrW(hwnd, GWL_STYLE) as DWORD;
            if ex_style & WS_EX_TOOLWINDOW == 0
                && ex_style & WS_EX_APPWINDOW != 0
                && ex_style & WS_EX_NOACTIVATE != 0
            {
                if GetWindowRect(hwnd, &mut rect) != 0 {
                    let handle_vec: &mut Vec<HWND> = &mut *(lparam as *mut Vec<HWND>);
                    handle_vec.push(hwnd);
                }
            }
        }

        1 // Continue enumeration
    }

    // Helper function to retrieve the windows
    fn get_windows() -> Vec<HWND> {
        let mut handles: Vec<HWND> = Vec::new();
        unsafe {
            EnumWindows(
                Some(enum_windows_proc),
                &mut handles as *mut Vec<HWND> as LPARAM,
            );
        }
        handles
    }

    // Function to capture the screen region
    pub fn capture_region(x: i32, y: i32, width: u32, height: u32) -> Result<Vec<u8>, Error> {
        let hwnd =
            unsafe { FindWindowW(null(), wide_string_to_string(&[0]).as_ptr() as *const u16) };

        if hwnd.is_null() {
            return Err(Error::new(
                ErrorKind::NotFound,
                "Failed to find a valid window handle",
            ));
        }

        let rect = RECT {
            left: x,
            top: y,
            right: x + (width as i32),
            bottom: y + (height as i32),
        };

        capture(hwnd, rect)
    }

    // Function to capture a specific window
    pub fn capture_window(hwnd: HWND) -> Result<Vec<u8>, Error> {
        let windows = get_windows();
        if windows.is_empty() {
            return Err(Error::new(ErrorKind::NotFound, "No windows found"));
        }

        let mut rect: RECT = unsafe { std::mem::zeroed() };
        if unsafe { GetWindowRect(hwnd, &mut rect) } != 0 {
            if rect.right - rect.left > 0 && rect.bottom - rect.top > 0 {
                return capture(hwnd, rect);
            }
        }

        Err(Error::new(ErrorKind::NotFound, "No visible windows found"))
    }

    // Function to capture the entire screen
    pub fn capture_screen() -> Result<Vec<u8>, Error> {
        let rect = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };

        capture(null_mut(), rect)
    }

    // Helper function to capture the screenshot based on the window handle
    fn capture(hwnd: HWND, rect: RECT) -> Result<Vec<u8>, Error> {
        let hdc = unsafe { GetWindowDC(hwnd) };
        if hdc.is_null() {
            return Err(Error::last_os_error());
        }

        let mut width = rect.right - rect.left;
        let mut height = rect.bottom - rect.top;

        // Adjust the width and height for negative dimensions
        if width < 0 {
            width *= -1;
        }
        if height < 0 {
            height *= -1;
        }

        let bmp_info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as DWORD,
                biWidth: width,
                biHeight: height,
                biPlanes: 1,
                biBitCount: 24,
                biCompression: 0,
                biSizeImage: (width * height * 3) as DWORD,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [RGBQUAD {
                rgbBlue: 0,
                rgbGreen: 0,
                rgbRed: 0,
                rgbReserved: 0,
            }; 1],
        };

        let mut pixels: Vec<u8> = vec![0; (width * height * 3) as usize];
        let mut bmp_info_copy = bmp_info;

        unsafe {
            let hbitmap = CreateDIBSection(
                hdc,
                &mut bmp_info_copy,
                DIB_RGB_COLORS,
                (&mut pixels[..]).as_mut_ptr() as *mut *mut winapi::ctypes::c_void,
                null_mut(),
                0,
            );

            if hbitmap.is_null() {
                return Err(Error::last_os_error());
            }

            let old_object = SelectObject(hdc, hbitmap as *mut std::ffi::c_void as HGDIOBJ);

            // Copy the screen content to the bitmap
            if hwnd.is_null() {
                BitBlt(
                    hdc,
                    0,
                    0,
                    width,
                    height,
                    GetDC(null_mut()),
                    rect.left,
                    rect.top,
                    SRCCOPY,
                );
            } else {
                PrintWindow(hwnd, hdc, PW_CLIENTONLY | PW_RENDERFULLCONTENT);
            }

            // Convert the pixels Vec<u8> to a new Vec<u8> to return
            let image_bytes: Vec<u8> = pixels.clone();

            // Clean up resources
            SelectObject(hdc, old_object);
            DeleteObject(hbitmap as HGDIOBJ);
            ReleaseDC(null_mut(), hdc);

            Ok(image_bytes)
        }
    }
}

#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(target_os = "windows")]
pub use windows::*;
