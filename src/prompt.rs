use anyhow::Result;
use crossterm::{
    ExecutableCommand,
    style::{Attributes, Color, Print, ResetColor, SetAttributes, SetForegroundColor},
};
use std::path::PathBuf;

use crate::h5::H5File;
use crate::shell::Shell;

pub struct Prompt {
    modules: Vec<Module>,
}

impl Prompt {
    pub fn new() -> Self {
        Self {
            modules: vec![
                Module::FileName {
                    style: Style {
                        color: Color::DarkYellow,
                        ..Default::default()
                    },
                },
                Module::WorkingGroup {
                    style: Style {
                        color: Color::Reset,
                        ..Default::default()
                    },
                },
                Module::Char {
                    c: String::from("$"),
                    style: Style {
                        color: Color::DarkRed,
                        padding_left: 1,
                        padding_right: 1,
                        ..Default::default()
                    },
                },
            ],
        }
    }

    pub fn render(&self, shell: &Shell, h5file: &H5File) -> String {
        self.render_modules(shell, h5file)
            .unwrap_or_else(|_| String::from("$"))
    }

    fn render_modules(&self, shell: &Shell, h5file: &H5File) -> Result<String> {
        let mut buffer: Vec<u8> = Vec::new();
        for module in &self.modules {
            module.render(&mut buffer, shell, h5file)?;
        }
        Ok(String::from_utf8(buffer)?)
    }
}

enum Module {
    FileName { style: Style },
    WorkingGroup { style: Style },
    Char { c: String, style: Style },
}

struct Style {
    color: Color,
    attributes: Attributes,
    padding_left: usize,
    padding_right: usize,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            color: Color::Grey,
            attributes: Attributes::none(),
            padding_left: 0,
            padding_right: 0,
        }
    }
}

impl Module {
    fn render<Out: ExecutableCommand>(
        &self,
        out: &mut Out,
        shell: &Shell,
        h5file: &H5File,
    ) -> Result<()> {
        match self {
            Self::FileName { style } => render_filename(out, h5file, style),
            Self::WorkingGroup { style } => render_working_group(out, shell, style),
            Self::Char { c, style } => render_char(out, c, style),
        }
    }
}

fn render_filename<Out: ExecutableCommand>(
    out: &mut Out,
    h5file: &H5File,
    style: &Style,
) -> Result<()> {
    let path = PathBuf::from(h5file.filename());
    let filename = path
        .file_name()
        .map_or_else(|| "", |s| s.to_str().unwrap_or(""));

    style.start(out)?;
    out.execute(Print(filename))?;
    style.end(out)?;
    Ok(())
}

fn render_working_group<Out: ExecutableCommand>(
    out: &mut Out,
    shell: &Shell,
    style: &Style,
) -> Result<()> {
    style.start(out)?;
    out.execute(Print(shell.get_working_group()))?;
    style.end(out)?;
    Ok(())
}

fn render_char<Out: ExecutableCommand>(out: &mut Out, c: &str, style: &Style) -> Result<()> {
    style.start(out)?;
    out.execute(Print(c))?;
    style.end(out)?;
    Ok(())
}

impl Style {
    fn start<Out: ExecutableCommand>(&self, out: &mut Out) -> Result<()> {
        if self.padding_left > 0 {
            out.execute(Print(" ".repeat(self.padding_left)))?;
        }
        out.execute(SetForegroundColor(self.color))?
            .execute(SetAttributes(self.attributes))?;
        Ok(())
    }

    fn end<Out: ExecutableCommand>(&self, out: &mut Out) -> Result<()> {
        out.execute(ResetColor)?
            .execute(SetAttributes(Attributes::none()))?;
        if self.padding_right > 0 {
            out.execute(Print(" ".repeat(self.padding_right)))?;
        }
        Ok(())
    }
}
