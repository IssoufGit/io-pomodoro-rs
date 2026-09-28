// App icon: the Noto Color Emoji ⏳ (assets/pomodoro.png, license in
// assets/NOTO-EMOJI-LICENSE), embedded in the binary so the window, the macOS
// menu bar item and the Linux top bar indicator all show the same hourglass
// as the dock.

use eframe::egui;

pub const ICON_PNG: &[u8] = include_bytes!("../assets/pomodoro.png");

pub fn make_icon() -> egui::IconData {
    decode(ICON_PNG).unwrap_or_default()
}

fn decode(bytes: &[u8]) -> Option<egui::IconData> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    // Expand palette/low-bit-depth images and strip 16-bit so we always get
    // 8-bit samples.
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    buf.truncate(info.buffer_size());
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => buf.chunks(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
        png::ColorType::GrayscaleAlpha => buf.chunks(2).flat_map(|p| [p[0], p[0], p[0], p[1]]).collect(),
        png::ColorType::Grayscale => buf.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return None, // EXPAND converts these to RGB(A)
    };
    Some(egui::IconData { rgba, width: info.width, height: info.height })
}
