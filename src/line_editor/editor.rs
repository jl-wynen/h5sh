use super::completion;
use super::parse::{Argument, Expression, Parser, StringExpression};
use super::text_index::TextIndex;
use crate::h5::{self, CacheValue, H5Error, H5File, H5FileCache, H5Object, H5Path};

use crossterm::{
    ExecutableCommand,
    style::{Attribute, Color, Print, PrintStyledContent, Stylize},
};
use log::{error, info};
use rustyline::{
    CompletionType, Context, Helper, Hinter, Validator,
    completion::Completer,
    config::{BellStyle, Config, EditMode},
    error::ReadlineError,
    highlight::{CmdKind, Highlighter},
    history::DefaultHistory,
};
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashSet;
use std::ops::DerefMut;

type UnderlyingEditor<'f> = rustyline::Editor<Hinter<'f>, DefaultHistory>;

pub struct LineEditor<'f> {
    editor: UnderlyingEditor<'f>,
}

impl<'f> LineEditor<'f> {
    pub fn new(commands: HashSet<String>, file: &'f H5File) -> rustyline::Result<Self> {
        let mut editor = UnderlyingEditor::with_config(configuration()?)?;

        let hinter = match Hinter::new(commands, file) {
            Ok(hinter) => hinter,
            Err(err) => {
                error!("Failed to create hinter: {err}");
                return Err(ReadlineError::Interrupted);
            }
        };
        editor.set_helper(Some(hinter));

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

    pub fn set_working_group(&mut self, group: H5Path) {
        if let Some(helper) = self.editor.helper_mut() {
            helper.working_group = group;
        }
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
struct Hinter<'f> {
    commands: HashSet<String>,
    file: &'f H5File,
    file_cache: RefCell<H5FileCache>,
    working_group: H5Path,
}

impl<'f> Hinter<'f> {
    fn new(commands: HashSet<String>, file: &'f H5File) -> h5::Result<Self> {
        Ok(Self {
            commands,
            file,
            file_cache: H5FileCache::with_root(file)?.into(),
            working_group: H5Path::root(),
        })
    }
}

impl<'f> Completer for Hinter<'f> {
    type Candidate = completion::Candidate;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        let expression = Parser::new(line).parse();

        let child_loader = move |parent: &CacheValue| match self
            .file
            .load(*parent.location_token())?
        {
            H5Object::Group(group) => Ok(self.file.load_children(group)?.filter_map(|object| {
                Some((
                    object.path().clone(),
                    CacheValue::from_h5object(&object).ok()?,
                    matches!(object, H5Object::Group(_)),
                ))
            })),
            H5Object::Dataset(_) => Err(H5Error::Other("Not a group".into())),
        };

        let mut file_cache = self.file_cache.borrow_mut();

        completion::complete(
            &expression,
            line,
            pos,
            &self.commands,
            file_cache.deref_mut(),
            &self.working_group,
            child_loader,
        )
    }
}

impl<'f> Highlighter for Hinter<'f> {
    fn highlight<'l>(&self, line: &'l str, _: usize) -> Cow<'l, str> {
        let expression = Parser::new(line).parse();

        if let Ok(highlighted) = InputHighlighter::new(&self.commands).highlight(&expression, line)
        {
            Cow::Owned(highlighted)
        } else {
            Cow::Borrowed(line)
        }
    }

    fn highlight_char(&self, line: &str, pos: usize, kind: CmdKind) -> bool {
        // This could be optimized further to not highlight if pos is in a plain
        // argument or some other string that does not get highlighted. But that would
        // require parsing the line here or keeping a persistent AST.
        let line_modified = !matches!(kind, CmdKind::MoveCursor);
        let inserted_whitespace =
            !line.is_empty() && pos > 0 && line.as_bytes()[pos - 1].is_ascii_whitespace();
        line_modified && !inserted_whitespace
    }
}

struct InputHighlighter<'a> {
    buffer: Vec<u8>,
    pos: TextIndex,
    commands: &'a HashSet<String>,
}

impl<'a> InputHighlighter<'a> {
    fn new(commands: &'a HashSet<String>) -> Self {
        Self {
            buffer: Vec::default(),
            pos: TextIndex::default(),
            commands,
        }
    }

    fn highlight(mut self, expression: &Expression, src: &str) -> std::io::Result<String> {
        // Allocated enough space for most cases
        self.buffer.reserve((2 * src.len()).max(16));
        self.highlight_expression(expression, src)?;
        self.unstyled_to(src.len().into(), src)?;
        Ok(String::from_utf8(self.buffer).unwrap_or_else(|_| src.to_string()))
    }

    fn highlight_expression(&mut self, expr: &Expression, src: &str) -> std::io::Result<()> {
        match expr {
            Expression::Call(call) => {
                let function_color = if self.commands.contains(&src[call.function.range]) {
                    Some(Color::White)
                } else {
                    Some(Color::Red)
                };
                self.highlight_string(&call.function, function_color, Some(Attribute::Bold), src)?;
                for arg in &call.arguments {
                    self.highlight_argument(arg, src)?;
                }
            }
            Expression::String(string) => {
                self.highlight_string(string, None, None, src)?;
            }
            Expression::Noop => {}
        }
        Ok(())
    }

    fn highlight_argument(&mut self, arg: &Argument, src: &str) -> std::io::Result<()> {
        match arg {
            Argument::Plain(string) => {
                self.highlight_string(string, None, None, src)?;
            }
            Argument::Long(string) => {
                self.highlight_string(string, Some(Color::Yellow), None, src)?;
            }
            Argument::Short(string) => {
                self.highlight_string(string, Some(Color::Yellow), None, src)?;
            }
        }
        Ok(())
    }

    fn highlight_string(
        &mut self,
        string: &StringExpression,
        foreground: Option<Color>,
        attribute: Option<Attribute>,
        src: &str,
    ) -> std::io::Result<()> {
        self.unstyled_to(string.range.start(), src)?;
        let mut styled = src[string.range].stylize();
        if let Some(foreground) = foreground {
            styled = styled.with(foreground);
        }
        if let Some(attribute) = attribute {
            styled = styled.attribute(attribute);
        }
        self.buffer.execute(PrintStyledContent(styled))?;
        self.pos = string.range.end();
        Ok(())
    }

    fn unstyled_to(&mut self, end: TextIndex, src: &str) -> std::io::Result<()> {
        if self.pos < end {
            self.buffer
                .execute(Print(&src[self.pos.as_index()..end.as_index()]))?;
            self.pos = end;
        }
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
        .build())
}

fn history_path() -> std::path::PathBuf {
    dirs::cache_dir().unwrap().join("h5sh").join("history.txt")
}
