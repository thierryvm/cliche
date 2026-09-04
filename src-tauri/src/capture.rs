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

use crate::geometry::PhysicalRect;
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

/// Cuts a rectangle out of a frame. Bytes and a rectangle in, bytes out.
///
/// Pure, and deliberately ignorant: it knows nothing about the webview, about
/// the scale factor, or about Tauri. The rectangle it is handed is ALREADY in
/// physical pixels - turning a CSS rectangle into one is `geometry::to_physical`
/// and is tested there, at scales this machine does not have.
///
/// # The four cases, handled on purpose rather than by accident
///
/// - **Empty rectangle.** Impossible to hand over: `PhysicalRect::new` refuses
///   a zero width or height, exactly as [`Frame::new`] refuses a zero
///   dimension. The empty case is closed at construction, so there is no branch
///   here to forget - and a test in `geometry` holds that door shut.
/// - **Rectangle leaving the image.** An ERROR, never a clamp. A rectangle that
///   does not fit means the coordinates that produced it are wrong, and the
///   likeliest cause is the one this machine cannot see: a scale factor applied
///   wrongly. Clamping would return a plausible image of the wrong region and
///   hide that for ever. The message names both rectangles so the disagreement
///   can be read off it.
/// - **A single pixel.** Legal. Tested.
/// - **The whole image.** Legal, and the result must equal the input byte for
///   byte. Tested, because a stride mistake is invisible at that size otherwise.
///
/// # Cost
///
/// One allocation of the cut, and one `copy_from_slice` per row. The source is
/// borrowed, so the frame stays available for a second cut. Nothing here is on
/// the 150 ms path: a selection happens after the veil is painted.
pub fn crop(frame: &Frame, rect: PhysicalRect) -> Result<Frame, String> {
    // In `u64`, so the sum of two `u32` cannot wrap and turn an out-of-bounds
    // rectangle into an in-bounds one.
    let right = u64::from(rect.x()) + u64::from(rect.width());
    let bottom = u64::from(rect.y()) + u64::from(rect.height());

    if right > u64::from(frame.width()) || bottom > u64::from(frame.height()) {
        return Err(format!(
            "a selection of {width}x{height} at ({x}, {y}) reaches ({right}, {bottom}), which is \
             outside a {frame_width}x{frame_height} capture",
            width = rect.width(),
            height = rect.height(),
            x = rect.x(),
            y = rect.y(),
            frame_width = frame.width(),
            frame_height = frame.height(),
        ));
    }

    // `Frame::new` already proved that `width * height * BYTES_PER_PIXEL` fits
    // in a `usize`, and height is at least 1, so a single row does too.
    let source_stride = frame.width() as usize * BYTES_PER_PIXEL;

    // A `Frame` cannot exist with a zero width, so this is at least 4. But
    // `chunks_exact(0)` PANICS, and this module's rule is that nothing on a
    // real path may panic - not even through an invariant that currently holds.
    if source_stride == 0 {
        return Err("a capture with no width cannot be cropped".to_owned());
    }

    let row_bytes = rect.width() as usize * BYTES_PER_PIXEL;
    // Bounded by `source_stride` through the check above, so neither the offset
    // nor its end can wrap.
    let row_start = rect.x() as usize * BYTES_PER_PIXEL;
    let row_end = row_start + row_bytes;

    let mut pixels = Vec::with_capacity(row_bytes * rect.height() as usize);

    // `chunks_exact` yields exactly `frame.height()` full rows and no
    // remainder: `Frame::new` guarantees the buffer is exactly
    // `width * height * BYTES_PER_PIXEL` bytes.
    for row in frame
        .pixels()
        .chunks_exact(source_stride)
        .skip(rect.y() as usize)
        .take(rect.height() as usize)
    {
        // `get`, not `row[a..b]`: the bounds check above makes this range
        // valid, and a slicing mistake must still not panic a webview thread.
        let slice = row.get(row_start..row_end).ok_or_else(|| {
            format!(
                "a row of {} byte(s) has no bytes {row_start}..{row_end}",
                row.len()
            )
        })?;
        pixels.extend_from_slice(slice);
    }

    // The last backstop. Should fewer rows have been copied than were asked
    // for, the length contradicts the dimensions and this refuses the frame
    // rather than returning a short image that would encode as garbage.
    Frame::new(rect.width(), rect.height(), pixels)
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
    // CROP. The test that matters in this module is
    // `a_crop_is_exact_to_the_byte`; everything around it exists so that it
    // cannot pass for the wrong reason.
    // ---------------------------------------------------------------------

    /// A synthetic test image whose every byte is known - NOT a screenshot -
    /// and asymmetric in BOTH axes.
    ///
    /// The asymmetry is the whole point, not a flourish. On an image that
    /// repeats along x, a crop shifted by one column produces the bytes of the
    /// correct crop and the test passes on a broken implementation. Here:
    ///
    /// - **R carries the column**: `10 * (column + 1)`.
    /// - **G carries the row**: `100 + 10 * row`.
    /// - **B carries a serial number**, row-major from 1, so no two pixels of
    ///   the whole image are alike.
    /// - **A is `255 - serial`**, so alpha is carried too and a channel that
    ///   gets dropped, premultiplied or reordered shows up.
    ///
    /// Consequences, which are what make the exactness test able to fail:
    /// a shift of one COLUMN moves R by 10 and B by 1; a shift of one ROW moves
    /// G by 10 and B by the width; a transposition swaps R and G; a stride
    /// taken from the rectangle instead of the frame lands on a different
    /// serial from the second row onwards. None of the four can pass.
    fn mire(width: u32, height: u32) -> Frame {
        // The channel arithmetic above is `u8`, and a bigger mire would wrap -
        // silently in release, in a panic in debug. Refused with a reason
        // rather than left as a trap for whoever wants a larger fixture.
        assert!(
            width <= 20 && height <= 12,
            "the mire's channel arithmetic only holds up to 20 x 12"
        );

        let mut pixels = Vec::with_capacity(width as usize * height as usize * BYTES_PER_PIXEL);

        for row in 0..height {
            for column in 0..width {
                let serial = (row * width + column + 1) as u8;
                pixels.push(10 * (column as u8 + 1));
                pixels.push(100 + 10 * row as u8);
                pixels.push(serial);
                pixels.push(255 - serial);
            }
        }

        Frame::new(width, height, pixels).expect("the mire must match its own dimensions")
    }

    /// Shorthand for the tests below. A rectangle that cannot exist fails the
    /// test where it is written rather than three lines later.
    fn rect(x: u32, y: u32, width: u32, height: u32) -> PhysicalRect {
        PhysicalRect::new(x, y, width, height).expect("the test rectangle must have an area")
    }

    #[test]
    fn the_mire_is_the_image_this_module_thinks_it_is() {
        // Without this, a bug in `mire` would make every comparison below
        // vacuous: the expected bytes would be wrong in exactly the same way as
        // the actual ones. Spot values computed by hand from the four rules.
        let frame = mire(5, 4);

        assert_eq!(frame.pixels().len(), 5 * 4 * 4);
        // (0, 0): first column, first row, serial 1.
        assert_eq!(&frame.pixels()[0..4], &[10, 100, 1, 254]);
        // (1, 1): serial 1 * 5 + 1 + 1 = 7, so byte 6 * 4 = 24.
        assert_eq!(&frame.pixels()[24..28], &[20, 110, 7, 248]);
        // (4, 2): serial 2 * 5 + 4 + 1 = 15, so byte 14 * 4 = 56.
        assert_eq!(&frame.pixels()[56..60], &[50, 120, 15, 240]);
        // (4, 3): the last pixel, serial 20, so byte 19 * 4 = 76.
        assert_eq!(&frame.pixels()[76..80], &[50, 130, 20, 235]);
    }

    #[test]
    fn a_crop_is_exact_to_the_byte() {
        // THE test of this lot. A region of a known image, cut out and compared
        // byte for byte against a table written by hand - columns 1 to 3 of
        // rows 1 and 2 of `mire(5, 4)`.
        //
        // Serial numbers, from the mire's rule (row * 5 + column + 1):
        //   row 1: columns 1, 2, 3 ->  7,  8,  9
        //   row 2: columns 1, 2, 3 -> 12, 13, 14
        let frame = mire(5, 4);

        let cut = crop(&frame, rect(1, 1, 3, 2)).expect("3x2 at (1, 1) fits inside 5x4");

        #[rustfmt::skip]
        let expected: Vec<u8> = vec![
            20, 110,  7, 248,    30, 110,  8, 247,    40, 110,  9, 246,
            20, 120, 12, 243,    30, 120, 13, 242,    40, 120, 14, 241,
        ];

        assert_eq!(
            (cut.width(), cut.height()),
            (3, 2),
            "3 wide and 2 high, not 2x3"
        );
        assert_eq!(
            cut.pixels(),
            &expected[..],
            "the cut must be exact to the byte"
        );
    }

    #[test]
    fn shifting_that_crop_by_one_pixel_changes_the_bytes_in_every_direction() {
        // The test above is only worth something if it can FAIL. This one
        // proves it can: the same rectangle moved by a single pixel, in each of
        // the four directions, must produce different bytes. If the mire ever
        // stops being asymmetric enough, this test says so instead of leaving
        // the exactness test quietly toothless.
        let frame = mire(5, 4);
        let reference = crop(&frame, rect(1, 1, 3, 2))
            .expect("the reference rectangle fits")
            .into_pixels();

        for (direction, shifted) in [
            ("one column left", rect(0, 1, 3, 2)),
            ("one column right", rect(2, 1, 3, 2)),
            ("one row up", rect(1, 0, 3, 2)),
            ("one row down", rect(1, 2, 3, 2)),
        ] {
            let moved = crop(&frame, shifted)
                .expect("every shifted rectangle still fits inside 5x4")
                .into_pixels();

            assert_ne!(
                moved, reference,
                "moving the rectangle {direction} produced the same bytes; an off-by-one in \
                 `crop` would go undetected"
            );
        }
    }

    #[test]
    fn a_crop_of_the_whole_image_returns_the_image_unchanged() {
        // Catches a stride computed from the rectangle instead of the frame,
        // and any silent transposition, at the one size where both are easiest
        // to get accidentally right.
        let frame = mire(5, 4);
        let expected = frame.pixels().to_vec();

        let cut = crop(&frame, rect(0, 0, 5, 4)).expect("the whole image is a legal selection");

        assert_eq!((cut.width(), cut.height()), (5, 4));
        assert_eq!(cut.into_pixels(), expected);
    }

    #[test]
    fn a_single_pixel_crop_returns_that_one_pixel() {
        // The bottom-right pixel: the one an off-by-one at the far edge misses.
        let frame = mire(5, 4);

        let cut = crop(&frame, rect(4, 3, 1, 1)).expect("the last pixel is inside the image");

        assert_eq!((cut.width(), cut.height()), (1, 1));
        assert_eq!(cut.pixels(), &[50, 130, 20, 235]);
    }

    #[test]
    fn a_rectangle_that_leaves_the_image_is_refused_and_not_clamped() {
        // Clamping would return a plausible image of the wrong region - which
        // is precisely how a scale-factor mistake would survive unnoticed on a
        // 125 % screen.
        let frame = mire(5, 4);

        assert!(
            crop(&frame, rect(3, 0, 3, 1)).is_err(),
            "one column too far"
        );
        assert!(crop(&frame, rect(0, 3, 1, 2)).is_err(), "one row too far");
        assert!(
            crop(&frame, rect(5, 0, 1, 1)).is_err(),
            "origin past the right edge"
        );
        assert!(
            crop(&frame, rect(4, 3, 1, 1)).is_ok(),
            "the last pixel must still be reachable, or the check is off by one itself"
        );

        let message = crop(&frame, rect(3, 0, 3, 1)).expect_err("3 + 3 > 5");
        assert!(
            message.contains("5x4") && message.contains("3x1"),
            "the message must name both rectangles to be actionable: {message}"
        );
    }

    #[test]
    fn a_crop_can_be_encoded_like_any_other_frame() {
        // The cut is a `Frame` and nothing about it is special: it must survive
        // the same round trip as a capture, or the selection would produce an
        // image the rest of the pipeline cannot carry.
        let frame = mire(5, 4);
        let cut = crop(&frame, rect(1, 1, 3, 2)).expect("must cut");
        let expected = cut.pixels().to_vec();

        let png = encode_png(&cut).expect("a 3x2 frame encodes");
        let decoded = image::load_from_memory_with_format(&png, ImageFormat::Png)
            .expect("what we just encoded must decode")
            .into_rgba8();

        assert_eq!(decoded.dimensions(), (3, 2));
        assert_eq!(decoded.into_raw(), expected);
    }

    // ---------------------------------------------------------------------
    // RESIZING, AND THE EXACTNESS THAT MUST SURVIVE IT.
    //
    // Required in these words on 4 September 2026: "la decoupe au pixel doit
    // rester exacte apres un redimensionnement : le test au pixel doit couvrir
    // ce cas."
    //
    // THE PROPERTY. A selection is carried as its TWO ABSOLUTE CORNERS, in CSS
    // pixels, and never as a size that each gesture adds to. It follows that a
    // rectangle reached by N resizes cuts the same bytes as the same rectangle
    // drawn in one go - and that is what `a_sequence_of_resizes_...` asserts,
    // against a table written by hand so that "identical" cannot mean
    // "identically wrong".
    //
    // WHAT THESE TESTS DO NOT PROVE. The gesture itself lives in
    // `src/veil/zones.ts` and nothing here executes that file. `Drag` below is
    // a MODEL of its anchor rule, exercised against the real Rust pipeline the
    // gesture ends in (`CssRect::from_corners` -> `geometry::to_physical` ->
    // `crop`). It pins the property and it pins the Rust half.
    //
    // Since 4 September 2026 the TypeScript half is no longer held to it by
    // reading alone: `src/veil/zones.test.ts` states the same anchor rule under
    // Vitest, on the SAME start rectangle and the same target points as
    // `every_grip_anchors_on_the_side_it_is_not_moving` below. Two statements
    // in two languages, neither derived from the other - which is what makes a
    // future disagreement between them worth reading.
    // ---------------------------------------------------------------------

    use crate::geometry::{to_physical, CssRect};

    /// The eight resize grips, named as `hitTest` in `src/veil/zones.ts` names
    /// its zones.
    #[derive(Clone, Copy, Debug)]
    enum Grip {
        Nw,
        N,
        Ne,
        W,
        E,
        Sw,
        S,
        Se,
    }

    /// A selection as the veil holds it: the two corners the pointer reported.
    /// Absolute, in CSS pixels. No size, no delta, no accumulation.
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Drag {
        anchor: (f64, f64),
        pointer: (f64, f64),
    }

    impl Drag {
        fn new(anchor: (f64, f64), pointer: (f64, f64)) -> Self {
            Self { anchor, pointer }
        }

        fn left(self) -> f64 {
            self.anchor.0.min(self.pointer.0)
        }

        fn right(self) -> f64 {
            self.anchor.0.max(self.pointer.0)
        }

        fn top(self) -> f64 {
            self.anchor.1.min(self.pointer.1)
        }

        fn bottom(self) -> f64 {
            self.anchor.1.max(self.pointer.1)
        }

        /// Grabbing a grip is a DRAW whose anchor is the opposite corner - the
        /// rule `main.ts` implements, and the reason resizing needs no inversion
        /// branch of its own: `from_corners` already normalises.
        ///
        /// A side grip moves one edge and keeps the other axis, which it does by
        /// pinning the pointer's other coordinate to the edge that is not
        /// moving. So one of `to`'s two coordinates is deliberately ignored, and
        /// the tests below pass an absurd value in it to prove it is.
        fn grab(self, grip: Grip, to: (f64, f64)) -> Self {
            match grip {
                Grip::Nw => Self::new((self.right(), self.bottom()), to),
                Grip::Ne => Self::new((self.left(), self.bottom()), to),
                Grip::Sw => Self::new((self.right(), self.top()), to),
                Grip::Se => Self::new((self.left(), self.top()), to),
                Grip::N => Self::new((self.left(), self.bottom()), (self.right(), to.1)),
                Grip::S => Self::new((self.left(), self.top()), (self.right(), to.1)),
                Grip::W => Self::new((self.right(), self.top()), (to.0, self.bottom())),
                Grip::E => Self::new((self.left(), self.top()), (to.0, self.bottom())),
            }
        }

        /// The whole real path from two CSS corners to the bytes that would
        /// reach the clipboard.
        fn cut(self, frame: &Frame, scale: f64) -> Frame {
            let css =
                CssRect::from_corners(self.anchor.0, self.anchor.1, self.pointer.0, self.pointer.1)
                    .expect("every corner in these tests is a finite, non-negative coordinate");
            let rect = to_physical(css, scale).expect("the rectangle has an area at this scale");
            crop(frame, rect).expect("the rectangle is inside the mire")
        }
    }

    /// The same gesture written the WRONG way: the selection carried as whole
    /// CSS pixels that each move adds a rounded delta to. It exists so that the
    /// exactness test cannot pass vacuously - see the test that uses it.
    #[derive(Clone, Copy, Debug)]
    struct Drift {
        left: f64,
        top: f64,
        right: f64,
        bottom: f64,
        last: (f64, f64),
    }

    impl Drift {
        fn new(anchor: (f64, f64), pointer: (f64, f64)) -> Self {
            Self {
                left: anchor.0.min(pointer.0).round(),
                top: anchor.1.min(pointer.1).round(),
                right: anchor.0.max(pointer.0).round(),
                bottom: anchor.1.max(pointer.1).round(),
                last: pointer,
            }
        }

        /// Only the south-east corner, which is all the test below needs.
        fn drag_se(mut self, to: (f64, f64)) -> Self {
            self.right += (to.0 - self.last.0).round();
            self.bottom += (to.1 - self.last.1).round();
            self.last = to;
            self
        }

        fn cut(self, frame: &Frame, scale: f64) -> Frame {
            Drag::new((self.left, self.top), (self.right, self.bottom)).cut(frame, scale)
        }
    }

    #[test]
    fn every_grip_anchors_on_the_side_it_is_not_moving() {
        // The rule the whole resize gesture rests on. Got wrong, a corner grip
        // moves the rectangle instead of resizing it, and a side grip drags the
        // other axis with it.
        let start = Drag::new((10.0, 20.0), (30.0, 40.0));

        // Corner grips: the opposite corner stays put, both axes follow.
        assert_eq!(start.grab(Grip::Se, (35.0, 45.0)).anchor, (10.0, 20.0));
        assert_eq!(start.grab(Grip::Nw, (5.0, 15.0)).anchor, (30.0, 40.0));
        assert_eq!(start.grab(Grip::Ne, (35.0, 15.0)).anchor, (10.0, 40.0));
        assert_eq!(start.grab(Grip::Sw, (5.0, 45.0)).anchor, (30.0, 20.0));

        // Side grips: one edge moves, the other axis is untouched. The absurd
        // coordinate proves the ignored one really is ignored.
        let north = start.grab(Grip::N, (999.0, 12.0));
        assert_eq!((north.left(), north.right()), (10.0, 30.0));
        assert_eq!((north.top(), north.bottom()), (12.0, 40.0));

        let south = start.grab(Grip::S, (999.0, 50.0));
        assert_eq!((south.left(), south.right()), (10.0, 30.0));
        assert_eq!((south.top(), south.bottom()), (20.0, 50.0));

        let west = start.grab(Grip::W, (4.0, 999.0));
        assert_eq!((west.left(), west.right()), (4.0, 30.0));
        assert_eq!((west.top(), west.bottom()), (20.0, 40.0));

        let east = start.grab(Grip::E, (44.0, 999.0));
        assert_eq!((east.left(), east.right()), (10.0, 44.0));
        assert_eq!((east.top(), east.bottom()), (20.0, 40.0));
    }

    #[test]
    fn a_sequence_of_resizes_cuts_the_same_bytes_as_the_rectangle_drawn_directly() {
        // THE test Thierry asked for. Six gestures - four grips, then two that
        // drag a corner PAST the opposite one so the rectangle inverts twice -
        // and the cut must be the one a single drag on the final corners makes.
        //
        // Scale 1.25, which this machine does not have: the interesting
        // coordinates are the ones that do not land on a physical pixel.
        let frame = mire(20, 12);
        let scale = 1.25;

        let resized = Drag::new((2.0, 1.0), (4.0, 3.0))
            .grab(Grip::Se, (7.6, 5.2))
            .grab(Grip::Nw, (3.2, 1.6))
            .grab(Grip::E, (9.6, 999.0))
            .grab(Grip::N, (999.0, 0.8))
            // Past the opposite corner, in both axes at once: the rectangle
            // turns inside out and no branch anywhere handles it.
            .grab(Grip::Nw, (12.0, 6.4))
            .grab(Grip::Sw, (10.4, 4.0));

        // Hand-computed from the six gestures above: left 10.4, top 4.0,
        // right 12.0, bottom 5.2. Written with the corners SWAPPED against the
        // sequence's, so `from_corners` normalisation is exercised as well.
        let expected_corners = Drag::new((10.4, 4.0), (12.0, 5.2));
        assert_eq!(
            (
                resized.left(),
                resized.top(),
                resized.right(),
                resized.bottom()
            ),
            (10.4, 4.0, 12.0, 5.2),
            "the six gestures do not end where the hand computation says they do"
        );

        let direct = expected_corners.cut(&frame, scale);

        // At 1.25 those corners land on physical (13, 5) to (15, 7): a 2 x 2
        // cut whose bottom edge, 6.5, is the one a rounding rule can move.
        // Serial numbers from the mire's rule (row * 20 + column + 1):
        //   row 5: columns 13, 14 -> 114, 115
        //   row 6: columns 13, 14 -> 134, 135
        #[rustfmt::skip]
        let by_hand: Vec<u8> = vec![
            140, 150, 114, 141,   150, 150, 115, 140,
            140, 160, 134, 121,   150, 160, 135, 120,
        ];
        assert_eq!(
            (direct.width(), direct.height()),
            (2, 2),
            "the hand computation says 2 x 2 physical px"
        );
        assert_eq!(
            direct.pixels(),
            &by_hand[..],
            "the rectangle drawn directly must be the bytes computed by hand, or `identical` \
             below would only mean `identically wrong`"
        );

        assert_eq!(
            resized.cut(&frame, scale).pixels(),
            direct.pixels(),
            "six resizes ending on that rectangle must cut it byte for byte"
        );
    }

    #[test]
    fn carrying_the_selection_as_a_size_instead_of_two_corners_loses_pixels() {
        // Why the test above is not vacuous. The same three moves, applied as
        // rounded deltas to a stored size - the implementation the property
        // forbids - and the bytes differ. Each move is 0.4 px, which rounds to
        // nothing three times in a row while the corners say 1.2.
        let frame = mire(20, 12);
        let scale = 1.0;

        let exact = Drag::new((2.0, 1.0), (4.0, 3.0))
            .grab(Grip::Se, (4.4, 3.4))
            .grab(Grip::Se, (4.8, 3.8))
            .grab(Grip::Se, (5.2, 4.2));

        let drifted = Drift::new((2.0, 1.0), (4.0, 3.0))
            .drag_se((4.4, 3.4))
            .drag_se((4.8, 3.8))
            .drag_se((5.2, 4.2));

        let exact_cut = exact.cut(&frame, scale);
        let drifted_cut = drifted.cut(&frame, scale);

        assert_eq!(
            (exact_cut.width(), exact_cut.height()),
            (3, 3),
            "two absolute corners say 2..5.2 and 1..4.2, which is 3 x 3 physical px"
        );
        assert_eq!(
            (drifted_cut.width(), drifted_cut.height()),
            (2, 2),
            "an accumulated size swallows three 0.4 px moves"
        );
        assert_ne!(
            drifted_cut.pixels(),
            exact_cut.pixels(),
            "if these were equal, the exactness test above could not fail either"
        );
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
