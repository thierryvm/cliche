//! Selection geometry: CSS pixels in, physical pixels out.
//!
//! # Why this is a module of its own, and not three lines in an event handler
//!
//! The selection rectangle is drawn in the WEBVIEW, whose coordinates are CSS
//! pixels. The image it will cut is what `xcap` grabbed, in PHYSICAL pixels.
//! The two are related by the window's scale factor - and on the machine this
//! was written on that factor is **1.00**, so the two coincide and every
//! mistake below is invisible here. It would appear on somebody else's 125 %
//! laptop, in a screenshot that is off by a few pixels, which is the kind of
//! defect that gets blamed on the screen rather than on the code.
//!
//! That is the whole reason the conversion is a pure function with its own
//! tests at 1.0, 1.25, 1.5 and 2.0. A multiplication buried in a pointer
//! handler cannot be run at 1.25 without a 1.25 screen; this one can, and is.
//!
//! # The rounding rule - decided HERE, once
//!
//! A CSS rectangle at 125 % lands on quarter pixels: 10 CSS px starting at 0.5
//! covers physical 0.625 .. 13.125. Something has to say which physical pixels
//! are in. Three rules were on the table, and they give three different images:
//!
//! | rule | what it does |
//! | --- | --- |
//! | truncate both | biases the whole rectangle up and left by up to a pixel |
//! | `floor` origin, `ceil` size ("outward") | always includes MORE than was drawn |
//! | round each EDGE to the nearest pixel | **chosen** |
//!
//! **Chosen: round the four EDGES to the nearest physical pixel, then DERIVE
//! the width and height as the difference between the rounded edges.**
//!
//! Two decisions in one sentence, and both are load-bearing:
//!
//! 1. **Nearest, not outward.** The veil draws its stroke ON the boundary the
//!    user is aiming at, so the cut should land where they aimed: error at most
//!    half a physical pixel, and in no particular direction. Outward rounding
//!    is never wrong by more than a pixel either - but it is wrong the SAME WAY
//!    every single time, so every capture taken at 125 % carries a sliver of
//!    whatever the user deliberately excluded. On a screenshot cropped to a
//!    window edge, that sliver is a visible line.
//!
//! 2. **Edges, not origin-and-size.** Rounding the origin and the size as two
//!    independent quantities lets their errors ADD: the far edge can then land
//!    a full pixel from where it was drawn, and unpredictably, because it
//!    depends on two roundings instead of one. Deriving the size from two
//!    rounded edges bounds every edge at half a pixel, full stop.
//!
//! The price of decision 2, said here rather than discovered later: **a given
//! CSS width does not always yield the same physical width.** At 125 %, 10 CSS
//! px starting at 0.0 is 13 physical px; the same 10 CSS px starting at 0.5 is
//! 12. That is not a defect of the rule - it is what mapping a fractional
//! rectangle onto a pixel grid costs, and every rule pays it somewhere. This
//! one pays it in the size rather than in the position of the edges, which is
//! the right way round for a tool whose user is aiming at an edge.
//! `the_size_is_derived_from_the_rounded_edges...` pins both numbers.
//!
//! Ties - an edge landing exactly on `.5` - go AWAY FROM ZERO, which for the
//! non-negative coordinates this module accepts means "up". That is
//! `f64::round`'s documented behaviour and it is NOT banker's rounding: 4.5
//! becomes 5, not 4. There is a test for that too, because the two disagree and
//! a reader is entitled to know which one is in force.
//!
//! # Nothing here may panic
//!
//! This runs inside a webview IPC handler. Every conversion out of `f64` is
//! range-checked before the cast, because Rust's float-to-integer casts
//! SATURATE: `f64::NAN as u32` is `0` and `1e30 as u32` is `u32::MAX`, both
//! without a word. A selection that silently became the top-left corner is
//! exactly the failure this module exists to refuse.

/// A selection rectangle as the webview measured it: CSS pixels, floating
/// point, origin at the top-left of the veil document.
///
/// Stored as four EDGES rather than an origin and a size, because the edges are
/// what the rounding rule operates on. Keeping the other representation around
/// would invite somebody to round the size on its own, which is the thing the
/// module header rules out.
///
/// The fields are private and the only constructor normalises, so a rectangle
/// with its right edge left of its left edge cannot exist.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CssRect {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}

impl CssRect {
    /// Builds a rectangle from the two corners of a drag: where the pointer
    /// went down, and where it is now.
    ///
    /// **The normalisation is here, in Rust, on purpose.** Dragging right to
    /// left or bottom to top is an ordinary gesture, and deciding which corner
    /// is which is geometry - geometry that lives in a pointer handler is
    /// geometry nobody can test. The page draws its rectangle from the same two
    /// corners with the same rule, so what is shown and what is cut come from
    /// one decision, not two.
    ///
    /// Refuses anything that is not a coordinate. These four numbers arrive
    /// from JavaScript over IPC, where `NaN` is one bad expression away, and a
    /// `NaN` that got through would end up as a silent `0` at the cast - a
    /// selection that quietly became the top-left corner of the screen.
    /// Negative values are refused rather than clamped for the same reason:
    /// the veil document starts at 0, so a negative coordinate means the page
    /// sent something it never measured, and repairing that quietly is how a
    /// wrong rectangle becomes a wrong screenshot nobody notices. The page
    /// clamps to its own viewport before sending - it is the only side that
    /// knows how big the viewport is.
    pub fn from_corners(
        anchor_x: f64,
        anchor_y: f64,
        pointer_x: f64,
        pointer_y: f64,
    ) -> Result<Self, String> {
        for (name, value) in [
            ("anchor x", anchor_x),
            ("anchor y", anchor_y),
            ("pointer x", pointer_x),
            ("pointer y", pointer_y),
        ] {
            if !value.is_finite() {
                return Err(format!(
                    "the selection's {name} is {value}, which is not a coordinate"
                ));
            }
            if value < 0.0 {
                return Err(format!(
                    "the selection's {name} is {value}; the veil document starts at 0, so a \
                     negative coordinate is a value the page never measured"
                ));
            }
        }

        Ok(Self {
            left: anchor_x.min(pointer_x),
            top: anchor_y.min(pointer_y),
            right: anchor_x.max(pointer_x),
            bottom: anchor_y.max(pointer_y),
        })
    }
}

/// A rectangle in the capture's own coordinates: physical pixels, integers,
/// origin at the top-left of the frame.
///
/// **Cannot be empty.** [`PhysicalRect::new`] refuses a zero width or height,
/// for the same reason `capture::Frame::new` refuses a zero dimension: it makes
/// the invalid state unrepresentable, so `capture::crop` has no empty case to
/// forget. A selection that rounds to nothing in an axis is a click, not a
/// drag, and it has no image to deliver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicalRect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

impl PhysicalRect {
    /// Builds a rectangle, refusing one with no area.
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Result<Self, String> {
        if width == 0 || height == 0 {
            return Err(format!(
                "a selection of {width}x{height} physical px has no pixels; at this scale the \
                 drag was too short to cover a whole pixel in one of its axes"
            ));
        }

        Ok(Self {
            x,
            y,
            width,
            height,
        })
    }

    pub fn x(&self) -> u32 {
        self.x
    }

    pub fn y(&self) -> u32 {
        self.y
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }
}

/// Turns a CSS rectangle into the physical pixels it covers.
///
/// `scale` is the webview's CSS-pixel-to-physical-pixel ratio - 1.0 at 100 %,
/// 1.25 at 125 %. Read it from the window that produced the coordinates, not
/// from a monitor: on a mixed-DPI desktop the two can differ.
///
/// The rounding rule, and the reasoning behind it, are in the module header.
/// Read them before changing a single `round` below: the three candidate rules
/// all "work", and they deliver three different images.
pub fn to_physical(rect: CssRect, scale: f64) -> Result<PhysicalRect, String> {
    if !scale.is_finite() || scale <= 0.0 {
        return Err(format!(
            "a scale factor of {scale} is not a ratio; CSS pixels cannot be turned into physical \
             ones without a positive, finite one"
        ));
    }

    let left = round_edge(rect.left, scale, "left")?;
    let top = round_edge(rect.top, scale, "top")?;
    let right = round_edge(rect.right, scale, "right")?;
    let bottom = round_edge(rect.bottom, scale, "bottom")?;

    // DERIVED from the rounded edges, never rounded on its own - that is
    // decision 2 of the module header, and it is this subtraction.
    //
    // `checked_sub` rather than `-`: `CssRect` is normalised and both
    // multiplication by a positive scale and `round` are monotonic, so the far
    // edge cannot land before the near one. That is a chain of three
    // arguments, and a chain of three arguments is not a reason to let a
    // webview thread panic on an underflow.
    let width = right
        .checked_sub(left)
        .ok_or_else(|| format!("the right edge rounded to {right}, before the left edge {left}"))?;
    let height = bottom
        .checked_sub(top)
        .ok_or_else(|| format!("the bottom edge rounded to {bottom}, above the top edge {top}"))?;

    PhysicalRect::new(left, top, width, height)
}

/// Rounds one edge to the nearest physical pixel. THE rounding rule, in one
/// place, applied identically to all four edges.
///
/// `f64::round` ties away from zero; the coordinates reaching here are
/// non-negative, so that is "half up".
fn round_edge(css: f64, scale: f64, name: &str) -> Result<u32, String> {
    let physical = (css * scale).round();

    // Checked BEFORE the cast, never after: `as u32` on a float saturates
    // instead of failing, so an absurd edge would arrive as a plausible
    // `u32::MAX` and be diagnosed as a rectangle "outside the capture" three
    // functions away from the number that caused it.
    if !physical.is_finite() || physical < 0.0 || physical > f64::from(u32::MAX) {
        return Err(format!(
            "the {name} edge is {css} CSS px, which at scale {scale} lands at {physical} physical \
             px - not a pixel index"
        ));
    }

    Ok(physical as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rectangle every scale test below is run on: 100 x 50 CSS px at
    /// (10, 20). Deliberately not square and not at the origin, so a swapped
    /// axis or a dropped offset fails here rather than on screen.
    fn reference_rect() -> CssRect {
        CssRect::from_corners(10.0, 20.0, 110.0, 70.0).expect("finite, non-negative corners")
    }

    #[test]
    fn at_scale_one_the_css_rectangle_is_its_own_physical_rectangle() {
        // The case this machine runs, and therefore the case that proves
        // nothing about the other three.
        let physical = to_physical(reference_rect(), 1.0).expect("scale 1.0 is a ratio");

        assert_eq!(
            (
                physical.x(),
                physical.y(),
                physical.width(),
                physical.height()
            ),
            (10, 20, 100, 50)
        );
    }

    #[test]
    fn a_fractional_scale_is_applied_to_every_edge_with_figures_computed_by_hand() {
        // Every number below is worked out on paper from the rule in the module
        // header, and every product is exact in binary floating point (the CSS
        // values are whole and the scales are 5/4, 3/2 and 2), so nothing here
        // depends on which side of an ulp a rounding lands.
        //
        // scale 1.25  left  10 x 1.25 =  12.5  -> 13     top  20 x 1.25 =  25    -> 25
        //             right 110 x 1.25 = 137.5 -> 138    bottom 70 x 1.25 = 87.5 -> 88
        //             width 138 - 13 = 125               height 88 - 25 = 63
        //
        // The height is the interesting one: 50 CSS px at 1.25 is 62.5 physical
        // px. Truncating gives 62 and loses a row; this rule gives 63.
        //
        // scale 1.5   15, 30, 165, 105  -> 150 x 75
        // scale 2.0   20, 40, 220, 140  -> 200 x 100
        let cases = [
            (1.25_f64, (13_u32, 25_u32, 125_u32, 63_u32)),
            (1.5, (15, 30, 150, 75)),
            (2.0, (20, 40, 200, 100)),
        ];

        for (scale, expected) in cases {
            let physical = to_physical(reference_rect(), scale).expect("a positive finite scale");

            assert_eq!(
                (
                    physical.x(),
                    physical.y(),
                    physical.width(),
                    physical.height()
                ),
                expected,
                "at scale {scale}"
            );
        }
    }

    #[test]
    fn the_size_is_derived_from_the_rounded_edges_not_rounded_on_its_own() {
        // THE consequence of decision 2, pinned with both numbers.
        //
        // The same 10 CSS px of width, at 125 %, twice:
        //   from 0.0 to 10.0   ->  0.000 -> 0   and  12.500 -> 13   =  13 px
        //   from 0.5 to 10.5   ->  0.625 -> 1   and  13.125 -> 13   =  12 px
        //
        // A rule that rounded the SIZE would answer 13 in both cases and put
        // the right edge somewhere the user did not draw it. Every value here
        // is a multiple of 1/8, so all of it is exact in binary.
        let flush = CssRect::from_corners(0.0, 0.0, 10.0, 4.0).expect("valid corners");
        let offset = CssRect::from_corners(0.5, 0.0, 10.5, 4.0).expect("valid corners");

        let flush = to_physical(flush, 1.25).expect("1.25 is a ratio");
        let offset = to_physical(offset, 1.25).expect("1.25 is a ratio");

        assert_eq!((flush.x(), flush.width()), (0, 13));
        assert_eq!(
            (offset.x(), offset.width()),
            (1, 12),
            "the same CSS width may cover a different number of physical pixels depending on \
             where it starts; that is the price of bounding every EDGE at half a pixel"
        );
    }

    #[test]
    fn a_tie_rounds_away_from_zero_and_not_to_the_even_neighbour() {
        // 3 CSS px at 1.5 is exactly 4.5 physical px. `f64::round` gives 5;
        // banker's rounding - the rule several other languages use by default -
        // gives 4. A reader is entitled to know which one is in force, so it is
        // asserted rather than described.
        let rect = CssRect::from_corners(3.0, 3.0, 5.0, 5.0).expect("valid corners");

        let physical = to_physical(rect, 1.5).expect("1.5 is a ratio");

        assert_eq!(
            physical.x(),
            5,
            "4.5 must round to 5; banker's rounding would answer 4 here"
        );
        assert_eq!(physical.y(), 5);
        // 5 x 1.5 = 7.5 -> 8, so 8 - 5 = 3.
        assert_eq!((physical.width(), physical.height()), (3, 3));
    }

    #[test]
    fn the_whole_viewport_maps_onto_the_whole_capture_with_nothing_left_over() {
        // The case a user hits by dragging corner to corner: at 125 %, a
        // 1536 x 864 CSS viewport is exactly the 1920 x 1080 capture. An
        // off-by-one anywhere in the rule shows up here as a missing row or a
        // rectangle one pixel past the end of the image.
        let rect = CssRect::from_corners(0.0, 0.0, 1536.0, 864.0).expect("valid corners");

        let physical = to_physical(rect, 1.25).expect("1.25 is a ratio");

        assert_eq!(
            (
                physical.x(),
                physical.y(),
                physical.width(),
                physical.height()
            ),
            (0, 0, 1920, 1080)
        );
    }

    #[test]
    fn a_drag_in_any_direction_describes_the_same_rectangle() {
        // Right-to-left and bottom-to-top are ordinary gestures. All four
        // orders must give one rectangle, or the cut depends on which way the
        // hand moved.
        let reference = reference_rect();

        assert_eq!(
            CssRect::from_corners(110.0, 70.0, 10.0, 20.0).expect("valid"),
            reference,
            "dragging up and left"
        );
        assert_eq!(
            CssRect::from_corners(110.0, 20.0, 10.0, 70.0).expect("valid"),
            reference,
            "dragging down and left"
        );
        assert_eq!(
            CssRect::from_corners(10.0, 70.0, 110.0, 20.0).expect("valid"),
            reference,
            "dragging up and right"
        );
    }

    #[test]
    fn a_coordinate_that_is_not_a_number_is_refused_instead_of_becoming_zero() {
        // `f64::NAN as u32` is 0 in Rust, silently. Without this refusal a
        // single bad expression in the page would produce a selection anchored
        // at the top-left corner of the screen and no error anywhere.
        for (label, corners) in [
            ("NaN anchor x", (f64::NAN, 0.0, 10.0, 10.0)),
            ("infinite pointer x", (0.0, 0.0, f64::INFINITY, 10.0)),
            (
                "negative infinite anchor y",
                (0.0, f64::NEG_INFINITY, 10.0, 10.0),
            ),
        ] {
            let (ax, ay, bx, by) = corners;
            assert!(
                CssRect::from_corners(ax, ay, bx, by).is_err(),
                "{label} must be refused"
            );
        }

        let message =
            CssRect::from_corners(f64::NAN, 0.0, 10.0, 10.0).expect_err("NaN is not a coordinate");
        assert!(
            message.contains("anchor x"),
            "the message must name which coordinate is wrong: {message}"
        );
    }

    #[test]
    fn a_negative_coordinate_is_refused_rather_than_quietly_clamped() {
        let message = CssRect::from_corners(-1.0, 0.0, 10.0, 10.0)
            .expect_err("the veil document has no negative coordinates");

        assert!(message.contains("anchor x"), "{message}");
        assert!(
            CssRect::from_corners(0.0, 0.0, 10.0, 10.0).is_ok(),
            "zero is a legal coordinate, or the check is refusing the top-left corner"
        );
    }

    #[test]
    fn a_scale_that_is_not_a_ratio_is_refused() {
        // A zero scale would collapse every selection to a point; a negative
        // one would put the rectangle off the top-left of the image. Both are
        // reachable if a platform ever reports a scale factor it could not
        // read.
        for scale in [0.0, -1.5, f64::NAN, f64::INFINITY] {
            assert!(
                to_physical(reference_rect(), scale).is_err(),
                "a scale of {scale} must be refused"
            );
        }
    }

    #[test]
    fn a_selection_that_rounds_to_nothing_is_refused_rather_than_cut_to_nothing() {
        // 0.2 CSS px at 100 % rounds to the same physical pixel at both edges,
        // so the width is 0. That is a click, not a drag, and there is no image
        // to deliver.
        let sliver = CssRect::from_corners(10.0, 10.0, 10.2, 12.0).expect("valid corners");

        let message = to_physical(sliver, 1.0).expect_err("a zero-width rectangle has no pixels");

        assert!(
            message.contains("0x2") || message.contains("has no pixels"),
            "the message must say the selection was empty: {message}"
        );
    }

    #[test]
    fn an_edge_beyond_the_pixel_grid_is_refused_rather_than_saturated() {
        // `1e10 as u32` is `u32::MAX` in Rust, silently. Refused here, where
        // the offending scale can still be named, rather than three functions
        // later as "a rectangle outside the capture".
        let rect = CssRect::from_corners(0.0, 0.0, 1.0, 1.0).expect("valid corners");

        let message = to_physical(rect, 1e10).expect_err("that edge is not a pixel index");

        assert!(
            message.contains("not a pixel index"),
            "unexpected message: {message}"
        );
    }

    #[test]
    fn a_rectangle_with_no_area_cannot_be_constructed_at_all() {
        // The empty case is closed HERE, at construction, so `capture::crop`
        // has none to forget. Same reasoning as `capture::Frame::new`.
        assert!(PhysicalRect::new(0, 0, 0, 10).is_err());
        assert!(PhysicalRect::new(0, 0, 10, 0).is_err());
        assert!(PhysicalRect::new(0, 0, 0, 0).is_err());
        assert!(
            PhysicalRect::new(0, 0, 1, 1).is_ok(),
            "a single pixel is a legal selection, or the check is refusing everything"
        );
    }
}
