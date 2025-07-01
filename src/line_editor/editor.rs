use log::info;
use rustyline::{
    CompletionType, Context, Helper, Highlighter, Hinter, Validator,
    completion::Completer,
    config::{BellStyle, Config, EditMode},
    error::ReadlineError,
    history::DefaultHistory,
};

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

#[derive(Helper, Highlighter, Hinter, Validator)]
struct Hinter;

impl Completer for Hinter {
    type Candidate = String;

    fn complete(
        &self,
        _line: &str,
        _pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<String>)> {
        Ok((0, vec![]))
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
