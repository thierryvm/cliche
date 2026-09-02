//! Display enumeration.
//!
//! Everything here works in PHYSICAL pixels. Screenshot geometry has to: a
//! rectangle expressed in logical pixels is wrong by the scale factor on any
//! screen that is not at 100 %, and that error is invisible on a 100 % machine.

use serde::Serialize;
use tauri::window::Monitor;
use tauri::AppHandle;

/// One display, in physical pixels.
///
/// Serialised in camelCase to match `DisplayInfo` in `src/displays.ts`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayInfo {
    /// OS name, e.g. `\\.\DISPLAY1`. Empty when Windows reports no name.
    pub name: String,
    /// X of the top-left corner in the virtual desktop. Negative on screens
    /// placed to the left of the primary one.
    pub x: i32,
    /// Y of the top-left corner in the virtual desktop.
    pub y: i32,
    /// Width in physical pixels.
    pub width: u32,
    /// Height in physical pixels.
    pub height: u32,
    /// Logical-to-physical ratio: 1.0 at 100 %, 1.5 at 150 %.
    pub scale_factor: f64,
}

impl DisplayInfo {
    fn from_monitor(monitor: &Monitor) -> Self {
        let position = monitor.position();
        let size = monitor.size();

        Self {
            // `name()` yields None when the monitor has just been unplugged.
            // An empty string is honest here; inventing "Display 1" would not be.
            name: monitor.name().cloned().unwrap_or_default(),
            x: position.x,
            y: position.y,
            width: size.width,
            height: size.height,
            scale_factor: monitor.scale_factor(),
        }
    }
}

/// Reads the monitor list. Pure plumbing, no side effect, so both the command
/// and the startup hook can share it.
pub fn collect_displays(app: &AppHandle) -> Result<Vec<DisplayInfo>, String> {
    let monitors = app
        .available_monitors()
        .map_err(|error| format!("could not enumerate monitors: {error}"))?;

    Ok(monitors.iter().map(DisplayInfo::from_monitor).collect())
}

/// Renders the display list as human-readable lines.
///
/// Kept separate from printing so it can be unit-tested without a running
/// application: constructing a `Monitor` requires a live event loop, a
/// `DisplayInfo` does not.
///
/// ASCII only on purpose: the Windows console is not reliably UTF-8, and a
/// mangled diagnostic line is worse than a plain one.
pub fn summarize(displays: &[DisplayInfo]) -> Vec<String> {
    let mut lines = Vec::with_capacity(displays.len() + 1);
    lines.push(format!("{} display(s) detected", displays.len()));

    for (index, display) in displays.iter().enumerate() {
        let name = if display.name.is_empty() {
            "(unnamed)"
        } else {
            display.name.as_str()
        };

        lines.push(format!(
            "  #{rank} {name} - {width}x{height} physical px at ({x}, {y}), scale {scale:.2}",
            rank = index + 1,
            name = name,
            width = display.width,
            height = display.height,
            x = display.x,
            y = display.y,
            scale = display.scale_factor,
        ));
    }

    lines
}

/// Writes the display list to stdout, which `pnpm tauri dev` shows.
pub fn print_displays(origin: &str, displays: &[DisplayInfo]) {
    for line in summarize(displays) {
        println!("[cliche] {origin}: {line}");
    }
}

/// Logs and returns every display. Exposed to the frontend.
#[tauri::command]
pub fn describe_displays(app: AppHandle) -> Result<Vec<DisplayInfo>, String> {
    let displays = collect_displays(&app)?;
    print_displays("describe_displays", &displays);
    Ok(displays)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn display(name: &str, width: u32, height: u32, scale: f64) -> DisplayInfo {
        DisplayInfo {
            name: name.to_owned(),
            x: 0,
            y: 0,
            width,
            height,
            scale_factor: scale,
        }
    }

    #[test]
    fn first_line_counts_the_displays() {
        let displays = vec![
            display("\\\\.\\DISPLAY1", 1920, 1080, 1.0),
            display("\\\\.\\DISPLAY2", 2560, 1440, 1.5),
        ];

        let lines = summarize(&displays);

        assert_eq!(lines[0], "2 display(s) detected");
        assert_eq!(lines.len(), 3, "one header line plus one line per display");
    }

    #[test]
    fn no_display_is_reported_as_zero_not_as_an_empty_output() {
        let lines = summarize(&[]);

        assert_eq!(lines, vec!["0 display(s) detected".to_owned()]);
    }

    #[test]
    fn a_display_line_carries_size_position_and_scale() {
        let displays = vec![DisplayInfo {
            name: "\\\\.\\DISPLAY1".to_owned(),
            x: -1920,
            y: 120,
            width: 1920,
            height: 1080,
            scale_factor: 1.0,
        }];

        let lines = summarize(&displays);

        assert_eq!(
            lines[1],
            "  #1 \\\\.\\DISPLAY1 - 1920x1080 physical px at (-1920, 120), scale 1.00"
        );
    }

    #[test]
    fn fractional_scale_is_not_rounded_away() {
        let displays = vec![display("\\\\.\\DISPLAY1", 3840, 2160, 1.25)];

        let lines = summarize(&displays);

        assert!(
            lines[1].ends_with("scale 1.25"),
            "125 % scaling must survive formatting, got: {}",
            lines[1]
        );
    }

    #[test]
    fn an_unnamed_display_is_labelled_rather_than_left_blank() {
        let displays = vec![display("", 1920, 1080, 1.0)];

        let lines = summarize(&displays);

        assert!(
            lines[1].contains("(unnamed)"),
            "an empty name must not produce a dangling dash, got: {}",
            lines[1]
        );
    }
}
