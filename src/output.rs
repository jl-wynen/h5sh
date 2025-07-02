use crossterm::{
    queue,
    style::{Color, Print, ResetColor, SetForegroundColor},
};
use lscolors::{Indicator, LsColors};
use std::fmt::Display;
use std::io::{Write, stderr, stdout};
use term_grid::{Direction, Filling, Grid, GridOptions};

use crate::cmd::CommandError;

pub struct Printer {
    style: Style,
}

impl Printer {
    pub fn new() -> Self {
        Self {
            style: Style::new(),
        }
    }

    pub fn println<T: Display>(&self, line: T) {
        println!("{line}");
    }

    pub fn print_grid<T: AsRef<str>>(&self, cells: Vec<T>) {
        let grid = Grid::new(
            cells,
            GridOptions {
                filling: Filling::Spaces(2),
                direction: Direction::TopToBottom,
                width: terminal_width(),
            },
        );
        let _ = stdout().write_all(grid.to_string().as_bytes());
    }

    pub fn print_cmd_error(&self, error: &CommandError) {
        let mut stderr = stderr();
        match error {
            CommandError::Error(message) => {
                let _ = queue!(
                    stderr,
                    SetForegroundColor(Color::DarkRed),
                    Print("Error: "),
                    Print(message),
                    ResetColor,
                    Print("\n"),
                );
            }
            CommandError::NoMessage => {}
            CommandError::Critical(message) => {
                let _ = queue!(
                    stderr,
                    SetForegroundColor(Color::Red),
                    Print("CRITICAL ERROR: "),
                    SetForegroundColor(Color::DarkRed),
                    Print(message),
                    ResetColor,
                    Print("\n"),
                );
            }
            CommandError::Exit => {}
        }
        let _ = stderr.flush();
    }

    pub fn print_shell_error<M: Display>(&self, message: M) {
        let mut stderr = stderr();
        let _ = queue!(
            stderr,
            SetForegroundColor(Color::DarkRed),
            Print(message),
            ResetColor,
            Print("\n"),
        );
        let _ = stderr.flush();
    }

    pub fn apply_style_dataset(&self, value: &str) -> String {
        self.style.apply_dataset(value)
    }

    pub fn apply_style_group(&self, value: &str) -> String {
        self.style.apply_group(value)
    }

    pub fn format_human_size(&self, size: u64, short: bool) -> String {
        let units = if short {
            &BYTE_UNITS_SHORT
        } else {
            &BYTE_UNITS_LONG
        };
        let mut size = size;
        for unit in units.iter() {
            if size < 1024 {
                return format!("{size}{unit}");
            }
            size /= 1024;
        }
        format!("{size}{}", units[units.len() - 1])
    }
}

const BYTE_UNITS_SHORT: [&str; 5] = ["B ", "Ki", "Mi", "Gi", "Ti"];
const BYTE_UNITS_LONG: [&str; 5] = ["B  ", "KiB", "MiB", "GiB", "TiB"];

struct Style {
    // Use nu_ansi_term::Style for best compatibility with lscolors.
    // (lscolors depends on an older version of crossterm.)
    // We apply styles before passing values to crossterm anyway,
    // so we don't need crossterm compatibility.
    dataset: nu_ansi_term::Style,
    group: nu_ansi_term::Style,
}

impl Style {
    fn new() -> Self {
        let ls_colors = LsColors::from_env().unwrap_or_default();
        Self {
            dataset: nu_ansi_term_style_for_indicator(&ls_colors, Indicator::RegularFile),
            group: nu_ansi_term_style_for_indicator(&ls_colors, Indicator::Directory),
        }
    }

    fn apply_dataset(&self, value: &str) -> String {
        self.dataset.paint(value).to_string()
    }

    fn apply_group(&self, value: &str) -> String {
        self.group.paint(value).to_string()
    }
}

fn nu_ansi_term_style_for_indicator(
    ls_colors: &LsColors,
    indicator: Indicator,
) -> nu_ansi_term::Style {
    ls_colors
        .style_for_indicator(indicator)
        .map(lscolors::Style::to_nu_ansi_term_style)
        .unwrap_or_default()
}

fn terminal_width() -> usize {
    crossterm::terminal::window_size().map_or(96, |size| size.columns as usize)
}
