use crossterm::{
    ExecutableCommand,
    style::{Color, Print, ResetColor, SetForegroundColor},
};
use log::info;
use rustyline::{
    CompletionType, Context, Helper, Hinter, Validator,
    completion::Completer,
    config::{BellStyle, Config, EditMode},
    error::ReadlineError,
    highlight::{CmdKind, Highlighter},
    history::DefaultHistory,
};
use std::borrow::Cow;

use super::parse::{Argument, Expression, Parser, StringExpression};
use super::text_index::TextIndex;

type UnderlyingEditor = rustyline::Editor<Hinter, DefaultHistory>;

pub struct LineEditor {
    editor: UnderlyingEditor,
}

impl LineEditor {
    pub fn new() -> rustyline::Result<Self> {
        let mut editor = UnderlyingEditor::with_config(configuration()?)?;
        editor.set_helper(Some(Hinter));
        if editor.load_history(&history_path()).is_err() {
            info!("No previous history.");
        }
        Ok(Self { editor })
    }

    pub fn poll(&mut self) -> Poll {
        let line = self.editor.readline("|> ");
        match line {
            Ok(line) => {
                if line.is_empty() {
                    Poll::Skip
                } else {
                    self.add_history_entry(line.as_str());
                    Poll::Cmd(line)
                }
            }
            Err(ReadlineError::Interrupted) => Poll::Skip,
            Err(ReadlineError::Eof) => Poll::Exit,
            Err(err) => Poll::Error(err.to_string()),
        }
    }

    pub fn save_history(&mut self) -> rustyline::Result<()> {
        let path = history_path();
        // The history is never in the root dir.
        let parent = path.parent().unwrap();
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
        self.editor.save_history(&path)
    }

    fn add_history_entry<S: AsRef<str> + Into<String>>(&mut self, entry: S) {
        let _ = self.editor.add_history_entry(entry);
    }
}

#[derive(Debug)]
pub enum Poll {
    Cmd(String),
    Error(String),
    Skip,
    Exit,
}

#[derive(Helper, Hinter, Validator)]
struct Hinter;

impl Completer for Hinter {
    type Candidate = String;

    fn complete(
        &self,
        _line: &str,
        _pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        Ok((0, vec!["/entry".to_string()]))
    }
}

impl Highlighter for Hinter {
    fn highlight<'l>(&self, line: &'l str, _: usize) -> Cow<'l, str> {
        let Ok(expression) = Parser::new(line).parse() else {
            return Cow::Borrowed(line);
        };

        if let Ok(highlighted) = InputHighlighter::new().highlight(&expression, line) {
            Cow::Owned(highlighted)
        } else {
            Cow::Borrowed(line)
        }
    }

    fn highlight_char(&self, _line: &str, _pos: usize, _kind: CmdKind) -> bool {
        true // TODO optimise
    }
}

struct InputHighlighter {
    buffer: Vec<u8>,
    pos: TextIndex,
}

impl InputHighlighter {
    fn new() -> Self {
        Self {
            buffer: Vec::default(),
            pos: TextIndex::default(),
        }
    }

    fn highlight(mut self, expression: &Expression, src: &str) -> std::io::Result<String> {
        self.buffer.reserve(2 * src.len());
        self.highlight_expression(expression, src)?;
        Ok(String::from_utf8(self.buffer).unwrap_or_else(|_| src.to_string()))
    }

    fn highlight_expression(&mut self, expr: &Expression, src: &str) -> std::io::Result<()> {
        match expr {
            Expression::Call(call) => {
                self.highlight_string(&call.function, src)?;
                for arg in &call.arguments {
                    self.highlight_argument(arg, src)?;
                }
            }
            Expression::String(string) => {
                self.highlight_string(string, src)?;
            }
            Expression::Noop => {}
        }
        Ok(())
    }

    fn highlight_argument(&mut self, arg: &Argument, src: &str) -> std::io::Result<()> {
        match arg {
            Argument::Plain(string) => {
                self.highlight_string(string, src)?;
            }
            Argument::Long(string) => {
                self.highlight_string(string, src)?;
            }
            Argument::Short(string) => {
                self.highlight_string(string, src)?;
            }
        }
        Ok(())
    }

    fn highlight_string(&mut self, string: &StringExpression, src: &str) -> std::io::Result<()> {
        if self.pos < string.range.start() {
            self.buffer.execute(Print(
                " ".repeat((string.range.start() - self.pos).as_index()),
            ))?;
        }
        self.buffer.execute(Print(&src[string.range]))?;
        self.pos = string.range.end();
        Ok(())
    }
}

fn configuration() -> rustyline::Result<Config> {
    Ok(Config::builder()
        .history_ignore_dups(true)?
        .history_ignore_space(true)
        .completion_type(CompletionType::List)
        .edit_mode(EditMode::Emacs)
        .bell_style(BellStyle::None)
        // .color_mode() // TODO read NO_COLOR?
        .build())
}

fn history_path() -> std::path::PathBuf {
    dirs::cache_dir().unwrap().join("h5sh").join("history.txt")
}
