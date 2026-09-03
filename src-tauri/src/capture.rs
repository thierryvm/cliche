//! Screen capture, and PNG encoding, measured SEPARATELY.
//!
//! The separation is the point of this module, not a tidiness preference.
//! Step 1d has to choose how the frozen image reaches the webview, and one of
//! the candidate routes is a data URL - which means a PNG. If encoding costs
//! 40 ms out of a 150 ms budget, that route is dead before anyone tries it. So
//! `capture` and `encode_png` are two functions, marked with two labels, and
//! the report shows two lines.
//!
//! **Nothing here may panic on a real path.** This code ends up inside the
//! global shortcut handler; a panic there takes the application down. Hence
//! `Result<_, String>` everywhere, no `unwrap`, no `expect`, and the buffer
//! invariant enforced in a constructor - see [`Frame`] for the one panic this
//! module is actually defending against.
//!
//! # What xcap does under us, on Windows
//!
//! Read from the vendored source of `xcap` 0.9.8 on 3 September 2026, not
//! assumed:
//!
//! - The crate's `wgc` feature is NOT a default feature and `Cargo.toml` does
//!   not ask for it, so the **GDI** path is compiled:
//!   `GetWindowDC(GetDesktopWindow())` -> `BitBlt` -> `GetDIBits`.
//! - `Monitor::width()/height()` come from `EnumDisplaySettingsW` -
//!   `dmPelsWidth`/`dmPelsHeight` - i.e. the mode's real pixel count.
//! - `Monitor::x()/y()` come from `DEVMODEW.dmPosition`, the monitor's origin
//!   in the desktop arrangement.
//! - The GDI path does not draw the mouse cursor, which is what a screenshot
//!   tool wants.
//!
//! # Two lists of "the screens" now exist - NOT reconciled here
//!
//! `displays.rs` enumerates monitors through Tauri (`available_monitors`,
//! `Monitor::position/size/scale_factor`); this module enumerates them through
//! `xcap`. Pixel-exact capture in lot 1 depends on those two agreeing about
//! origin, axis direction, and physical-vs-logical pixels. Reconciling them
//! blind, today, would be worse than not doing it: the ignored test
//! `probe_...` below prints xcap's view in EXACTLY the line format
//! `displays::summarize` uses, so the two can be held side by side by eye.
//! Deciding which one is authoritative is 1d/1e.
//!
//! One difference is already visible in the source and has to be flagged:
//!
//! - Tauri's `Monitor::position()` is documented as physical pixels and comes
//!   from the window manager's virtual-desktop rectangle; xcap's `x()/y()`
//!   come from `dmPosition`. These are not guaranteed to be the same number on
//!   a mixed-DPI setup.
//! - **DPI awareness differs between the app and `cargo test`.** The Tauri
//!   binary ships a manifest that makes the process DPI aware; a test binary
//!   built by cargo carries no such manifest. A DPI-unaware process gets a
//!   virtualised desktop DC, while `dmPelsWidth` is never virtualised - so on a
//!   display scaled above 100 %, the ignored tests below can return a frame of
//!   the right SIZE with wrong CONTENT (stretched, or padded with black).
//!   That is why those tests print the scale factor and shout when it is not
//!   1.0. This paragraph is read from the xcap source and Windows' documented
//!   behaviour; it has NOT been measured on this machine.

use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{ExtendedColorType, ImageEncoder, RgbaImage};
use xcap::Monitor;

use crate::timing::Timings;

/// Timing label for the grab itself.
///
/// A named constant rather than a literal at the call site: 1d asserts on
/// these, and a report line whose label drifted would be a step that silently
/// changed meaning.
pub const MARK_CAPTURE: &str = "capture";

/// Timing label for the PNG encode.
pub const MARK_ENCODE_PNG: &str = "encode_png";

/// Bytes per pixel in RGBA8. Named because it appears in a length invariant
/// that a panic depends on.
const BYTES_PER_PIXEL: usize = 4;

/// One captured frame: RGBA8 bytes plus the dimensions they belong to.
///
/// **The fields are private on purpose.** `ImageEncoder::write_image` is
/// documented to PANIC when `width * height * bytes_per_pixel != buf.len()`.
/// Making the three values inseparable, and checking them once in
/// [`Frame::new`], is what turns that panic into a `Result` at the only place
/// a mismatch can be introduced. Public fields would let a caller build an
/// inconsistent `Frame` and blow up inside the shortcut handler instead.
///
/// Layout: row-major, top-left origin, no padding between rows - that is what
/// `RgbaImage` guarantees and what `write_image` expects.
///
/// Not `Clone`, deliberately: 1920x1080x4 is 8.29 MB and an accidental
/// `.clone()` on this path is exactly the copy this lot is trying not to make.
#[derive(PartialEq, Eq)]
pub struct Frame {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

/// Prints the shape, never the 8 MB of bytes. A `{:?}` in a log line must not
/// dump a whole screenshot into the terminal.
impl std::fmt::Debug for Frame {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Frame")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("bytes", &self.pixels.len())
            .finish()
    }
}

impl Frame {
    /// Builds a frame, refusing anything whose length contradicts its
    /// dimensions.
    ///
    /// Zero is refused too: PNG forbids a zero dimension, and a zero-sized
    /// capture means the grab failed without saying so - which is the failure
    /// mode worth catching early, not passing on.
    pub fn new(width: u32, height: u32, pixels: Vec<u8>) -> Result<Self, String> {
        if width == 0 || height == 0 {
            return Err(format!(
                "a capture of {width}x{height} has no pixels; the grab failed without reporting it"
            ));
        }

        // Checked, not `as usize` arithmetic: on a 32-bit target a large
        // enough frame would wrap and the length check would then PASS on a
        // buffer that is far too short.
        let expected = (width as usize)
            .checked_mul(height as usize)
            .and_then(|pixel_count| pixel_count.checked_mul(BYTES_PER_PIXEL))
            .ok_or_else(|| format!("a capture of {width}x{height} does not fit in memory"))?;

        if pixels.len() != expected {
            return Err(format!(
                "capture buffer is {} byte(s) but {width}x{height} RGBA needs {expected}",
                pixels.len()
            ));
        }

        Ok(Self {
            width,
            height,
            pixels,
        })
    }

    /// Takes ownership of an `xcap`/`image` buffer without copying it.
    ///
    /// `into_raw` MOVES the `Vec` out of the `ImageBuffer`; it does not
    /// duplicate it. That is the whole reason this function exists rather than
    /// `to_vec()` at a call site.
    fn from_rgba_image(image: RgbaImage) -> Result<Self, String> {
        let (width, height) = image.dimensions();
        Self::new(width, height, image.into_raw())
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Borrows the RGBA bytes. Borrow, so that reading them costs nothing.
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Hands the buffer over, still without copying it. For a caller that has
    /// to move the frame somewhere else and does not need it here any more.
    pub fn into_pixels(self) -> Vec<u8> {
        self.pixels
    }
}

/// A capture and its PNG, kept together because the interesting question -
/// "how much does the PNG cost, in time and in bytes, next to the raw frame?" -
/// needs both.
pub struct Shot {
    frame: Frame,
    png: Vec<u8>,
}

/// Same reason as [`Frame`]'s: a derived `Debug` would print the entire PNG
/// into whatever log the `{:?}` landed in.
impl std::fmt::Debug for Shot {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Shot")
            .field("frame", &self.frame)
            .field("png_bytes", &self.png.len())
            .finish()
    }
}

impl Shot {
    pub fn frame(&self) -> &Frame {
        &self.frame
    }

    pub fn png(&self) -> &[u8] {
        &self.png
    }

    pub fn into_parts(self) -> (Frame, Vec<u8>) {
        (self.frame, self.png)
    }
}

/// Finds the monitor Windows calls primary.
///
/// Scans for the one that says so rather than taking the first of the list:
/// `Monitor::all()` is in enumeration order, which is not "primary first", and
/// capturing the wrong screen silently is the one outcome worth failing over.
/// If no monitor claims to be primary, that is an error - inventing a default
/// here would hide a real problem behind a plausible screenshot.
pub fn primary_monitor() -> Result<Monitor, String> {
    let monitors =
        Monitor::all().map_err(|error| format!("could not enumerate monitors: {error}"))?;

    if monitors.is_empty() {
        return Err("no monitor at all was enumerated".to_owned());
    }

    // Kept so that "none is primary" and "we could not ask" are told apart in
    // the message. They have different fixes.
    let mut first_error: Option<String> = None;

    for monitor in monitors {
        match monitor.is_primary() {
            Ok(true) => return Ok(monitor),
            Ok(false) => {}
            Err(error) => {
                if first_error.is_none() {
                    first_error = Some(error.to_string());
                }
            }
        }
    }

    Err(match first_error {
        Some(reason) => format!("could not tell which monitor is primary: {reason}"),
        None => "no monitor reports itself as the primary one".to_owned(),
    })
}

/// Grabs the whole primary screen.
///
/// **Copies of the 8.29 MB buffer on this path: ONE heap allocation, and it is
/// made inside xcap.** Traced through the vendored source:
///
/// 1. `to_rgba_image` allocates `vec![0u8; w * h * 4]` and `GetDIBits` fills
///    it. That is the buffer. (The `BitBlt` before it copies the screen into a
///    GDI bitmap, in driver/kernel memory - not a Rust heap copy, and not
///    something this crate can avoid on the GDI path.)
/// 2. `bgra_to_rgba` takes that `Vec` BY VALUE and swaps bytes in place -
///    same allocation, no copy.
/// 3. `RgbaImage::from_raw` moves it in - no copy.
/// 4. [`Frame::from_rgba_image`] calls `into_raw`, which moves it out again -
///    no copy.
///
/// So the bytes are written once and never duplicated before they reach the
/// caller. [`encode_png`] then reads them as a slice, also without copying.
pub fn capture_primary() -> Result<Frame, String> {
    let monitor = primary_monitor()?;

    let image = monitor
        .capture_image()
        .map_err(|error| format!("could not capture the primary screen: {error}"))?;

    Frame::from_rgba_image(image)
}

/// Encodes a frame as PNG, in memory.
///
/// Compression and filter are pinned EXPLICITLY rather than left to
/// `PngEncoder::new`'s defaults. `image` documents that the exact output of
/// these hints is not covered by its SemVer guarantee, and the number this
/// function is measured for is meaningless if a dependency bump can quietly
/// change what is being measured. `Fast` is the right choice on a latency
/// budget: this PNG exists to be shown, not archived.
///
/// The `write_image` call below panics on a length mismatch. It cannot be
/// reached with a mismatch, because [`Frame`] cannot be constructed with one -
/// that is what the private fields buy.
pub fn encode_png(frame: &Frame) -> Result<Vec<u8>, String> {
    // No `with_capacity`: the compressed size is not knowable in advance, and
    // a guess that is too small costs a reallocation while a guess that is too
    // big wastes megabytes.
    let mut png = Vec::new();

    PngEncoder::new_with_quality(&mut png, CompressionType::Fast, FilterType::Adaptive)
        .write_image(
            frame.pixels(),
            frame.width(),
            frame.height(),
            ExtendedColorType::Rgba8,
        )
        .map_err(|error| format!("could not encode the capture as PNG: {error}"))?;

    Ok(png)
}

/// Size of the BMP header this module writes: `BITMAPFILEHEADER` (14) +
/// `BITMAPINFOHEADER` (40) + three 32-bit channel masks (12).
///
/// The masks live in the colour-table area, right after the info header, which
/// is why `bfOffBits` is 66 and not 54.
const BMP_HEADER_BYTES: usize = 14 + 40 + 12;

/// Wraps a frame in a BMP header, in memory, WITHOUT touching the pixels.
///
/// # Why BMP, and why hand-written - the arithmetic
///
/// Measured on this machine on 3 September 2026: capture 24.2 ms, PNG encode
/// **69.6 ms**, against a 150 ms budget. The PNG spends 46 % of the budget
/// compressing an image that never leaves the machine and is thrown away a
/// second later. Every byte of that work is pure loss on this path.
///
/// A BMP is a header and the pixels. This function allocates once and does a
/// single `memcpy`; for a 1920x1080 frame that is 8.29 MB of memory bandwidth,
/// which costs single-digit milliseconds, against 69.6 ms. That is the entire
/// justification, and it is a comparison of two measured/derived numbers, not a
/// preference.
///
/// # Why not `image`'s `BmpEncoder`
///
/// It would mean enabling the crate's `bmp` feature, and - more importantly -
/// trusting its header choices sight unseen. The two choices below are exactly
/// what makes this function a `memcpy` instead of a per-pixel loop, and neither
/// is guaranteed by a general-purpose encoder:
///
/// - **Top-down rows** (`biHeight` negative). BMP is bottom-up by default; a
///   bottom-up writer has to walk the rows backwards, which is a per-row copy
///   instead of one.
/// - **`BI_BITFIELDS` with the masks describing R,G,B in that byte order.** BMP
///   is conventionally BGRA; declaring the masks lets the decoder read OUR RGBA
///   bytes as they already are. Without it, every pixel needs a channel swap -
///   a byte loop over 8.29 MB, compiled WITHOUT optimisation in this crate (see
///   the `[profile.dev.package."*"]` note in `Cargo.toml`, which optimises
///   dependencies only). That loop is precisely the kind of cost that made the
///   first dev-build measurement of `capture` read 140.8 ms.
///
/// # Alpha
///
/// Only three masks are declared, so there is no alpha channel: the fourth byte
/// of every pixel is padding the decoder must ignore, and the image is opaque.
/// This is deliberate. `GetDIBits` on a `BI_RGB` desktop DC leaves the high
/// byte undefined, so an image that HONOURED that byte could come out fully
/// transparent - a black veil, or no veil at all, for a reason that would look
/// like a rendering bug.
///
/// # What is NOT proven here
///
/// That WebView2's decoder accepts this file. The unit tests below check the
/// header against the documented layout and round-trip it through `image`'s own
/// BMP decoder (a dev-dependency), which is an INDEPENDENT reader - but it is
/// not Chromium's. Only running the app settles that.
pub fn encode_bmp(frame: &Frame) -> Result<Vec<u8>, String> {
    let pixels = frame.pixels();

    // Every conversion is checked. A `as i32` on a width above 2^31 would wrap
    // to a negative number, and a negative `biWidth` is not "a big image", it is
    // a malformed file - the sort of thing that produces a blank veil and no
    // error message at all.
    let width = i32::try_from(frame.width())
        .map_err(|_| format!("a width of {} does not fit a BMP header", frame.width()))?;
    let height = i32::try_from(frame.height())
        .map_err(|_| format!("a height of {} does not fit a BMP header", frame.height()))?;
    let payload_bytes = u32::try_from(pixels.len()).map_err(|_| {
        format!(
            "a payload of {} byte(s) does not fit a BMP header",
            pixels.len()
        )
    })?;
    let file_bytes = u32::try_from(BMP_HEADER_BYTES)
        .ok()
        .and_then(|header| header.checked_add(payload_bytes))
        .ok_or_else(|| "the BMP file size does not fit in 32 bits".to_owned())?;

    // Rows are 32 bits per pixel, so the stride is always a multiple of 4 and
    // BMP's row padding rule never applies. That is the second reason this can
    // be one copy: there is nothing to insert between rows.
    let mut bmp = Vec::with_capacity(BMP_HEADER_BYTES + pixels.len());

    // --- BITMAPFILEHEADER (14 bytes) ---
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&file_bytes.to_le_bytes());
    bmp.extend_from_slice(&0u16.to_le_bytes()); // bfReserved1
    bmp.extend_from_slice(&0u16.to_le_bytes()); // bfReserved2
    bmp.extend_from_slice(&(BMP_HEADER_BYTES as u32).to_le_bytes()); // bfOffBits

    // --- BITMAPINFOHEADER (40 bytes) ---
    bmp.extend_from_slice(&40u32.to_le_bytes()); // biSize
    bmp.extend_from_slice(&width.to_le_bytes()); // biWidth
                                                 // NEGATIVE height: top-down rows, first row in the file is the top row of
                                                 // the screen. `wrapping_neg` and not `-`: `i32::MIN` has no positive
                                                 // counterpart and would panic in debug. It is unreachable - a frame that
                                                 // tall cannot be allocated - but nothing on this path may panic.
    bmp.extend_from_slice(&height.wrapping_neg().to_le_bytes()); // biHeight
    bmp.extend_from_slice(&1u16.to_le_bytes()); // biPlanes
    bmp.extend_from_slice(&32u16.to_le_bytes()); // biBitCount
    bmp.extend_from_slice(&3u32.to_le_bytes()); // biCompression = BI_BITFIELDS
    bmp.extend_from_slice(&payload_bytes.to_le_bytes()); // biSizeImage
    bmp.extend_from_slice(&0i32.to_le_bytes()); // biXPelsPerMeter
    bmp.extend_from_slice(&0i32.to_le_bytes()); // biYPelsPerMeter
    bmp.extend_from_slice(&0u32.to_le_bytes()); // biClrUsed
    bmp.extend_from_slice(&0u32.to_le_bytes()); // biClrImportant

    // --- Channel masks (12 bytes) ---
    // A pixel is four bytes R,G,B,A in memory; read as a little-endian u32 that
    // is `R | G<<8 | B<<16 | A<<24`. Hence these three masks, which say "our
    // bytes are already in the right order, do not swap anything".
    bmp.extend_from_slice(&0x0000_00FFu32.to_le_bytes()); // red
    bmp.extend_from_slice(&0x0000_FF00u32.to_le_bytes()); // green
    bmp.extend_from_slice(&0x00FF_0000u32.to_le_bytes()); // blue
                                                          // No alpha mask on purpose - see the doc comment.

    // The one bulk operation of this function. `extend_from_slice` on a `Vec<u8>`
    // bottoms out in `copy_from_slice`, i.e. `memcpy` from the standard library,
    // which ships precompiled and optimised. A hand-written `for` loop here
    // would NOT be optimised in this crate.
    bmp.extend_from_slice(pixels);

    Ok(bmp)
}

/// Captures and encodes, timestamping each step on an already-open run.
///
/// The run is NOT opened or closed here: the caller owns it, because the run
/// starts at the shortcut press, well before this function is entered. A step
/// that fails records no mark, so a failed run reaches `finish_run` with
/// nothing in it and is discarded rather than averaged in as a fast one.
pub fn capture_and_encode(timings: &Timings) -> Result<Shot, String> {
    let frame = capture_primary()?;
    timings.mark(MARK_CAPTURE);

    let png = encode_png(&frame)?;
    timings.mark(MARK_ENCODE_PNG);

    Ok(Shot { frame, png })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::ImageFormat;

    /// The 8 bytes every PNG file starts with (PNG spec, section 5.2).
    const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

    /// 3 wide, 2 high - deliberately not square, so a width/height swap fails
    /// here instead of on screen. Channel values all differ inside each pixel,
    /// so a BGRA/RGBA mix-up fails. Alpha ranges from 0 to 255, so an encoder
    /// that drops or premultiplies alpha fails.
    fn known_frame() -> Frame {
        let pixels = vec![
            1, 2, 3, 4, //
            5, 6, 7, 8, //
            9, 10, 11, 12, //
            250, 200, 150, 100, //
            0, 0, 0, 0, //
            255, 255, 255, 255, //
        ];

        Frame::new(3, 2, pixels).expect("the hand-written frame must be consistent")
    }

    // ---------------------------------------------------------------------
    // No screen required: these run in CI.
    // ---------------------------------------------------------------------

    #[test]
    fn a_frame_refuses_a_buffer_whose_length_contradicts_its_dimensions() {
        // One byte short of 2x2 RGBA. This is the exact condition that makes
        // `write_image` panic, so it has to be caught here.
        let too_short = Frame::new(2, 2, vec![0u8; 15]);
        let too_long = Frame::new(2, 2, vec![0u8; 17]);

        assert!(too_short.is_err(), "a short buffer would panic the encoder");
        assert!(too_long.is_err());
        assert!(
            Frame::new(2, 2, vec![0u8; 16]).is_ok(),
            "the exact length must be accepted, or the check is just refusing everything"
        );
    }

    #[test]
    fn a_frame_error_says_both_lengths_so_it_can_be_acted_on() {
        let message = Frame::new(2, 2, vec![0u8; 15]).expect_err("15 != 16");

        assert!(
            message.contains("15") && message.contains("16"),
            "a mismatch message that names neither length is unusable: {message}"
        );
    }

    #[test]
    fn a_zero_sized_capture_is_an_error_not_an_empty_image() {
        assert!(Frame::new(0, 1080, Vec::new()).is_err());
        assert!(Frame::new(1920, 0, Vec::new()).is_err());
        assert!(
            Frame::new(0, 0, Vec::new()).is_err(),
            "PNG forbids a zero dimension, and a zero-sized grab is a failure, not a result"
        );
    }

    #[test]
    fn a_frame_hands_its_buffer_over_without_altering_it() {
        let frame = known_frame();
        let borrowed = frame.pixels().to_vec();

        assert_eq!(frame.into_pixels(), borrowed);
    }

    #[test]
    fn a_debug_line_shows_the_shape_and_not_the_pixels() {
        // A `{:?}` on a real capture would otherwise print 8 MB into a log.
        let rendered = format!("{:?}", known_frame());

        assert!(rendered.contains("width: 3"), "unexpected: {rendered}");
        assert!(rendered.contains("bytes: 24"), "unexpected: {rendered}");
        assert!(
            !rendered.contains("250"),
            "the pixel values must not be in the debug output: {rendered}"
        );
    }

    #[test]
    fn an_encoded_frame_really_is_a_png_file() {
        let png = encode_png(&known_frame()).expect("encoding a 3x2 frame must succeed");

        assert!(
            png.starts_with(&PNG_SIGNATURE),
            "the encoder produced {} byte(s) that do not begin with the PNG signature",
            png.len()
        );
    }

    #[test]
    fn a_png_round_trip_returns_the_same_dimensions_and_the_same_pixels() {
        // The test that proves the encoder does not lie: encode a buffer known
        // by hand, decode it back, compare byte for byte.
        let frame = known_frame();
        let expected = frame.pixels().to_vec();

        let png = encode_png(&frame).expect("encoding must succeed");
        let decoded = image::load_from_memory_with_format(&png, ImageFormat::Png)
            .expect("what we just encoded must decode")
            .into_rgba8();

        assert_eq!(decoded.dimensions(), (3, 2), "3 wide and 2 high, not 2x3");
        assert_eq!(
            decoded.into_raw(),
            expected,
            "every channel of every pixel must survive the round trip"
        );
    }

    #[test]
    fn the_decoder_rejects_nonsense_so_the_round_trip_is_not_vacuous() {
        // Without this, "it decoded" could just mean "the decoder accepts
        // anything", and the test above would prove nothing.
        assert!(image::load_from_memory_with_format(b"not a png", ImageFormat::Png).is_err());
        assert!(image::load_from_memory_with_format(b"", ImageFormat::Png).is_err());

        // A truncated PNG: valid signature, nothing behind it.
        assert!(image::load_from_memory_with_format(&PNG_SIGNATURE, ImageFormat::Png).is_err());
    }

    #[test]
    fn a_single_pixel_frame_encodes_and_comes_back() {
        // The smallest thing PNG allows. Guards the edge of the dimension
        // check as much as the encoder.
        let frame = Frame::new(1, 1, vec![7, 8, 9, 10]).expect("1x1 RGBA is 4 bytes");

        let png = encode_png(&frame).expect("a 1x1 PNG is legal");
        let decoded = image::load_from_memory_with_format(&png, ImageFormat::Png)
            .expect("must decode")
            .into_rgba8();

        assert_eq!(decoded.dimensions(), (1, 1));
        assert_eq!(decoded.into_raw(), vec![7, 8, 9, 10]);
    }

    #[test]
    fn a_frame_adopts_an_image_buffer_without_changing_its_bytes() {
        // Covers the seam between the `image` type xcap returns and our own,
        // which is the only place a transposition could creep in.
        let mut buffer = RgbaImage::new(3, 2);
        buffer.put_pixel(2, 1, image::Rgba([11u8, 22, 33, 44]));

        let frame = Frame::from_rgba_image(buffer).expect("a fresh RgbaImage is consistent");

        assert_eq!((frame.width(), frame.height()), (3, 2));
        // Last pixel of the buffer: row 1, column 2, row-major.
        assert_eq!(&frame.pixels()[20..24], &[11, 22, 33, 44]);
    }

    // ---------------------------------------------------------------------
    // BMP: the transport format 1d measures. See `encode_bmp` for the
    // arithmetic that chose it over PNG.
    // ---------------------------------------------------------------------

    /// Reads a little-endian `i32` at `offset`. `try_into` rather than slicing
    /// into an array by index so a wrong offset fails the test instead of
    /// panicking in a way that hides which field is wrong.
    fn le_i32(bytes: &[u8], offset: usize) -> i32 {
        let field: [u8; 4] = bytes
            .get(offset..offset + 4)
            .and_then(|slice| slice.try_into().ok())
            .unwrap_or_else(|| {
                panic!(
                    "no 4 bytes at offset {offset} of a {}-byte file",
                    bytes.len()
                )
            });
        i32::from_le_bytes(field)
    }

    fn le_u32(bytes: &[u8], offset: usize) -> u32 {
        le_i32(bytes, offset) as u32
    }

    fn le_u16(bytes: &[u8], offset: usize) -> u16 {
        let field: [u8; 2] = bytes
            .get(offset..offset + 2)
            .and_then(|slice| slice.try_into().ok())
            .unwrap_or_else(|| panic!("no 2 bytes at offset {offset}"));
        u16::from_le_bytes(field)
    }

    #[test]
    fn a_bmp_is_a_66_byte_header_followed_by_the_pixels_unchanged() {
        // The claim the whole transport choice rests on: encoding is a header
        // plus a copy. If a single pixel byte differed, the "no per-pixel work"
        // argument would be false.
        let frame = known_frame();
        let expected = frame.pixels().to_vec();

        let bmp = encode_bmp(&frame).expect("a 3x2 frame must encode");

        assert_eq!(bmp.len(), BMP_HEADER_BYTES + expected.len());
        assert_eq!(&bmp[..2], b"BM", "every BMP starts with 'BM'");
        assert_eq!(&bmp[BMP_HEADER_BYTES..], &expected[..]);
    }

    #[test]
    fn the_bmp_header_declares_top_down_rows_and_bitfields_in_rgb_byte_order() {
        // Every field checked against the documented BITMAPFILEHEADER /
        // BITMAPINFOHEADER layout. These four are the ones that make the copy
        // legal; getting any of them wrong yields a file that decodes to
        // something upside down, blue-tinted, or transparent.
        let bmp = encode_bmp(&known_frame()).expect("must encode");

        assert_eq!(
            le_u32(&bmp, 2),
            bmp.len() as u32,
            "bfSize is the whole file"
        );
        assert_eq!(
            le_u32(&bmp, 10),
            66,
            "bfOffBits must clear header AND masks"
        );
        assert_eq!(le_u32(&bmp, 14), 40, "biSize: BITMAPINFOHEADER");
        assert_eq!(le_i32(&bmp, 18), 3, "biWidth");
        assert_eq!(
            le_i32(&bmp, 22),
            -2,
            "biHeight must be NEGATIVE: a positive height means bottom-up, and \
             the veil would show the screen upside down"
        );
        assert_eq!(le_u16(&bmp, 26), 1, "biPlanes");
        assert_eq!(le_u16(&bmp, 28), 32, "biBitCount");
        assert_eq!(le_u32(&bmp, 30), 3, "biCompression must be BI_BITFIELDS");
        assert_eq!(le_u32(&bmp, 34), 24, "biSizeImage: 3x2x4");

        assert_eq!(le_u32(&bmp, 54), 0x0000_00FF, "red is the FIRST byte");
        assert_eq!(le_u32(&bmp, 58), 0x0000_FF00, "green is the second");
        assert_eq!(le_u32(&bmp, 62), 0x00FF_0000, "blue is the third");
    }

    #[test]
    fn a_bmp_round_trips_through_an_independent_decoder() {
        // The strongest check available without a browser: `image`'s own BMP
        // decoder - a reader that knows nothing about `encode_bmp` - is handed
        // the file and must return the pixels we put in, right way up.
        //
        // `image`'s `bmp` feature is a DEV-dependency only (see Cargo.toml), so
        // no BMP decoder ships in the application.
        //
        // If this test ever fails, do NOT relax it: the most likely cause is a
        // header WebView2 would reject too.
        let frame = known_frame();
        let expected = frame.pixels().to_vec();

        let bmp = encode_bmp(&frame).expect("must encode");
        let decoded = image::load_from_memory_with_format(&bmp, ImageFormat::Bmp)
            .expect("what we just encoded must decode")
            .into_rgba8();

        assert_eq!(decoded.dimensions(), (3, 2), "3 wide and 2 high, not 2x3");

        // Alpha is deliberately NOT carried (no alpha mask), so the decoder is
        // entitled to return 255 everywhere. Compare the three colour channels
        // only - and say so, rather than quietly comparing a subset.
        let actual = decoded.into_raw();
        for (index, (got, want)) in actual
            .chunks_exact(4)
            .zip(expected.chunks_exact(4))
            .enumerate()
        {
            assert_eq!(
                &got[..3],
                &want[..3],
                "pixel {index}: R,G,B must survive the round trip (alpha is not carried, by design)"
            );
        }
        assert_eq!(actual.len(), expected.len());
    }

    #[test]
    fn the_bmp_decoder_rejects_nonsense_so_the_round_trip_is_not_vacuous() {
        assert!(image::load_from_memory_with_format(b"not a bmp", ImageFormat::Bmp).is_err());
        assert!(image::load_from_memory_with_format(b"BM", ImageFormat::Bmp).is_err());
    }

    #[test]
    fn a_bmp_costs_the_raw_size_and_a_png_does_not_which_is_the_whole_trade() {
        // Pins the shape of the trade-off 1d measures, in a form that fails if
        // someone later makes `encode_bmp` compress: BMP is header + raw, PNG
        // is smaller and pays for it in time (69.6 ms median, measured
        // 3 September 2026).
        let frame = known_frame();

        let bmp = encode_bmp(&frame).expect("must encode");

        assert_eq!(
            bmp.len() - BMP_HEADER_BYTES,
            frame.pixels().len(),
            "a BMP that is not exactly the raw pixels means work is being done per pixel"
        );
    }

    #[test]
    fn the_two_timing_labels_are_distinct_and_stable() {
        // `Timings::mark` drops a duplicate label and counts it as ignored, so
        // two identical labels would silently lose the second measurement -
        // which is the whole point of this module.
        assert_ne!(MARK_CAPTURE, MARK_ENCODE_PNG);
        assert_eq!(MARK_CAPTURE, "capture");
        assert_eq!(MARK_ENCODE_PNG, "encode_png");
    }

    // ---------------------------------------------------------------------
    // A real screen required. Ignored by default: a CI runner is headless,
    // and a test that cannot run there must not be counted as passing there.
    //
    //   cargo test --manifest-path src-tauri/Cargo.toml -- --ignored --nocapture
    //
    // NOT TESTABLE AT ALL, and deliberately left untested rather than faked:
    //   - that the pixels are what was actually on screen. There is no
    //     reference to compare against inside a test; that is precisely why
    //     one PNG is written to disk for a human to look at.
    //   - the xcap / displays.rs comparison, automatically: `displays.rs`
    //     needs a live Tauri event loop, which a unit test has no way to
    //     start. The probe below prints one half; the app's `[cliche] startup:`
    //     lines print the other.
    // ---------------------------------------------------------------------

    /// How many captures the proof run makes. 20 and not 10, for the reason
    /// `timing.rs` documents: nearest-rank p95 on 10 samples collapses onto
    /// the maximum and stops being a percentile.
    const PROOF_RUNS: usize = 20;

    /// Renders a value that may not be readable, without hiding the reason.
    fn shown<T: std::fmt::Display, E: std::fmt::Display>(value: Result<T, E>) -> String {
        match value {
            Ok(value) => value.to_string(),
            Err(error) => format!("<unavailable: {error}>"),
        }
    }

    #[test]
    #[ignore = "needs a real screen; a CI runner is headless"]
    fn the_captured_frame_has_the_size_xcap_announces_for_that_monitor() {
        // No 1920 or 1080 anywhere: hard-coded dimensions would pass on this
        // machine and fail on every other one, including the CI runner.
        let monitor = primary_monitor().expect("a primary monitor must be found");
        let announced_width = monitor.width().expect("xcap must report a width");
        let announced_height = monitor.height().expect("xcap must report a height");

        let frame = capture_primary().expect("capturing the primary screen must succeed");

        assert_eq!(
            (frame.width(), frame.height()),
            (announced_width, announced_height),
            "the grab must cover exactly the monitor xcap described, no more and no less"
        );
        assert_eq!(
            frame.pixels().len(),
            announced_width as usize * announced_height as usize * BYTES_PER_PIXEL,
            "the buffer length must match the announced size"
        );
    }

    #[test]
    #[ignore = "needs a real screen; a CI runner is headless"]
    fn twenty_measured_captures_and_one_png_on_disk() {
        let timings = Timings::new();
        let mut last: Option<Shot> = None;

        for run in 1..=PROOF_RUNS {
            timings.begin_run();
            match capture_and_encode(&timings) {
                Ok(shot) => {
                    timings.finish_run();
                    last = Some(shot);
                }
                Err(error) => {
                    timings.abandon_run();
                    panic!("capture run {run} of {PROOF_RUNS} failed: {error}");
                }
            }
        }

        let shot = last.expect("twenty successful runs must leave a shot behind");

        // `temp_dir()` and never the repository: a test that litters the
        // working tree is a test nobody runs twice.
        let path = std::env::temp_dir().join(format!("cliche-capture-{}.png", std::process::id()));
        std::fs::write(&path, shot.png())
            .unwrap_or_else(|error| panic!("could not write {}: {error}", path.display()));

        let raw_bytes = shot.frame().pixels().len();
        let png_bytes = shot.png().len();

        println!();
        println!("[cliche] capture proof");
        println!("[cliche]   PNG written to: {}", path.display());
        println!(
            "[cliche]   {}x{} - {raw_bytes} raw RGBA byte(s), {png_bytes} PNG byte(s) ({:.1} %)",
            shot.frame().width(),
            shot.frame().height(),
            100.0 * png_bytes as f64 / raw_bytes as f64,
        );
        println!(
            "[cliche]   as a base64 data URL that PNG would weigh about {} byte(s)",
            png_bytes.div_ceil(3) * 4
        );

        let scale = primary_monitor().and_then(|monitor| {
            monitor
                .scale_factor()
                .map_err(|error| format!("scale factor unavailable: {error}"))
        });
        match scale {
            Ok(factor) if (factor - 1.0).abs() > f32::EPSILON => println!(
                "[cliche]   WARNING: display scale is {factor:.2}. This test binary carries no \
                 DPI manifest, so the CONTENT of the PNG may be stretched or padded even though \
                 its size is right. Look at the file before trusting it."
            ),
            Ok(factor) => println!("[cliche]   display scale {factor:.2}"),
            Err(error) => println!("[cliche]   display scale unknown: {error}"),
        }

        for line in timings.report().lines() {
            println!("[cliche] {line}");
        }
        println!();

        // Read back from disk, not from memory: what is asserted is the file,
        // which is what will actually be looked at.
        let written = std::fs::read(&path).expect("the file just written must be readable");
        assert!(
            written.starts_with(&PNG_SIGNATURE),
            "the file on disk is not a PNG"
        );

        let decoded = image::load_from_memory_with_format(&written, ImageFormat::Png)
            .expect("the file on disk must decode as a PNG");
        assert_eq!(
            decoded.width(),
            shot.frame().width(),
            "the PNG on disk must carry the captured width"
        );
        assert_eq!(decoded.height(), shot.frame().height());

        let report = timings.report();
        assert_eq!(report.runs, PROOF_RUNS, "every run must have been filed");
        assert_eq!(
            report.ignored_marks, 0,
            "a dropped mark means a step was measured under the wrong name"
        );
        assert_eq!(
            report.steps.len(),
            2,
            "capture and encode must appear as two separate steps, or the whole point is lost"
        );
        assert_eq!(report.steps[0].label, MARK_CAPTURE);
        assert_eq!(report.steps[1].label, MARK_ENCODE_PNG);
        assert_eq!(report.steps[0].samples, PROOF_RUNS);
        assert_eq!(report.steps[1].samples, PROOF_RUNS);
    }

    #[test]
    #[ignore = "needs a real screen; a CI runner is headless"]
    fn probe_what_xcap_says_about_every_monitor() {
        // A PROBE, not a verdict. Its assertions are real but weak on purpose:
        // the value is the printed output, to be held line by line against the
        // `[cliche] startup:` lines `displays::summarize` prints from Tauri's
        // monitor list. The format below is deliberately identical to that one.
        // Reconciling the two coordinate systems is 1d/1e, not this lot.
        let monitors = Monitor::all().expect("xcap must be able to enumerate monitors");

        println!();
        println!("[cliche] xcap: {} display(s) detected", monitors.len());

        let mut primaries = 0;

        for (index, monitor) in monitors.iter().enumerate() {
            if monitor.is_primary().unwrap_or(false) {
                primaries += 1;
            }

            println!(
                "  #{rank} {name} - {width}x{height} physical px at ({x}, {y}), scale {scale}",
                rank = index + 1,
                name = shown(monitor.name()),
                width = shown(monitor.width()),
                height = shown(monitor.height()),
                x = shown(monitor.x()),
                y = shown(monitor.y()),
                scale = shown(monitor.scale_factor()),
            );
            println!(
                "       friendly {friendly} | rotation {rotation} | primary {primary} | \
                 builtin {builtin}",
                friendly = shown(monitor.friendly_name()),
                rotation = shown(monitor.rotation()),
                primary = shown(monitor.is_primary()),
                builtin = shown(monitor.is_builtin()),
            );
        }

        println!(
            "[cliche] xcap: compare the lines above with the `[cliche] startup:` lines the app \
             prints from displays.rs. Origin, sign and physical-vs-logical are NOT reconciled yet."
        );
        println!();

        assert!(!monitors.is_empty(), "a machine with a screen has monitors");
        assert_eq!(
            primaries, 1,
            "exactly one monitor must claim to be primary, or `primary_monitor` is picking blind"
        );
    }
}
