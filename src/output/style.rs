use crossterm::style::{Attribute, Attributes, Color, Colors, SetAttributes, SetColors};
use lscolors::{Indicator, LsColors};
use std::fmt::Write;

#[derive(Debug)]
pub struct Style {
    pub dataset: Item,
    pub group: Item,
    pub attribute: Item,
}

#[derive(Debug)]
pub struct Item {
    colors: Colors,
    attributes: Attributes,
}

impl Style {
    pub fn new() -> Self {
        let ls_colors = LsColors::from_env().unwrap_or_default();

        Self {
            dataset: Item::from_lscolors(&ls_colors, Indicator::RegularFile),
            group: Item::from_lscolors(&ls_colors, Indicator::Directory),
            attribute: Item {
                colors: Colors {
                    foreground: Some(Color::DarkCyan),
                    background: None,
                },
                attributes: Attributes::default(),
            },
        }
    }
}

impl Item {
    fn from_lscolors(ls_colors: &LsColors, indicator: Indicator) -> Self {
        let Some(style) = ls_colors.style_for_indicator(indicator) else {
            return Self::default();
        };
        Self {
            colors: Colors {
                foreground: style.foreground.map(convert_ls_color),
                background: style.background.map(convert_ls_color),
            },
            attributes: convert_ls_attributes(style.font_style),
        }
    }
}

impl Default for Item {
    fn default() -> Self {
        Self {
            colors: Colors {
                foreground: None,
                background: None,
            },
            attributes: Attributes::default(),
        }
    }
}

impl crossterm::Command for Item {
    fn write_ansi(&self, f: &mut impl Write) -> std::fmt::Result {
        SetColors(self.colors).write_ansi(f)?;
        SetAttributes(self.attributes).write_ansi(f)?;
        Ok(())
    }

    #[cfg(windows)]
    fn execute_winapi(&self) -> std::io::Result<()> {
        SetColors(self.colors).execute_winapi()?;
        SetAttributes(self.attributes).execute_winapi()?;
        Ok(())
    }

    #[cfg(windows)]
    fn is_ansi_code_supported(&self) -> bool {
        SetColors(self.colors).is_ansi_code_supported()
            && SetAttributes(self.attributes).is_ansi_code_supported()
    }
}

// lscolors depends on an older version of crossterm, so we cannot use its builtin
// conversions. These are copies of the conversion functions in lscolors.
fn convert_ls_color(ls_color: lscolors::Color) -> Color {
    match ls_color {
        lscolors::Color::RGB(r, g, b) => Color::Rgb { r, g, b },
        lscolors::Color::Fixed(n) => Color::AnsiValue(n),
        lscolors::Color::Black => Color::Black,
        lscolors::Color::Red => Color::DarkRed,
        lscolors::Color::Green => Color::DarkGreen,
        lscolors::Color::Yellow => Color::DarkYellow,
        lscolors::Color::Blue => Color::DarkBlue,
        lscolors::Color::Magenta => Color::DarkMagenta,
        lscolors::Color::Cyan => Color::DarkCyan,
        lscolors::Color::White => Color::Grey,
        lscolors::Color::BrightBlack => Color::DarkGrey,
        lscolors::Color::BrightRed => Color::Red,
        lscolors::Color::BrightGreen => Color::Green,
        lscolors::Color::BrightYellow => Color::Yellow,
        lscolors::Color::BrightBlue => Color::Blue,
        lscolors::Color::BrightMagenta => Color::Magenta,
        lscolors::Color::BrightCyan => Color::Cyan,
        lscolors::Color::BrightWhite => Color::White,
    }
}

fn convert_ls_attributes(ls_style: lscolors::FontStyle) -> Attributes {
    let mut attributes = Attributes::default();
    if ls_style.bold {
        attributes.set(Attribute::Bold);
    }
    if ls_style.dimmed {
        attributes.set(Attribute::Dim);
    }
    if ls_style.italic {
        attributes.set(Attribute::Italic);
    }
    if ls_style.underline {
        attributes.set(Attribute::Underlined);
    }
    if ls_style.slow_blink {
        attributes.set(Attribute::SlowBlink);
    }
    if ls_style.rapid_blink {
        attributes.set(Attribute::RapidBlink);
    }
    if ls_style.reverse {
        attributes.set(Attribute::Reverse);
    }
    if ls_style.hidden {
        attributes.set(Attribute::Hidden);
    }
    if ls_style.strikethrough {
        attributes.set(Attribute::CrossedOut);
    }
    attributes
}
