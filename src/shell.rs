use crate::cmd::{self, Command, CommandError, CommandOutcome};
use crate::h5::{H5File, H5Path};
use crate::line_editor::LineEditor;
use crate::output::Printer;
use std::rc::Rc;

pub struct Shell {
    working_dir: H5Path,
    printer: Printer,
    commands: cmd::CommandMap,
}

impl Shell {
    pub fn new() -> Self {
        Self {
            working_dir: H5Path::root(),
            printer: Printer::new(),
            commands: cmd::commands(),
        }
    }

    pub fn printer(&self) -> &Printer {
        &self.printer
    }

    pub fn commands(&self) -> &cmd::CommandMap {
        &self.commands
    }

    pub fn get_command(&self, name: &str) -> Option<Rc<dyn Command>> {
        self.commands.get(name).cloned()
    }

    pub fn get_working_dir(&self) -> &H5Path {
        &self.working_dir
    }

    pub fn set_working_dir(&mut self, path: H5Path) {
        self.working_dir = path;
    }

    pub fn resolve_path(&self, path: &H5Path) -> H5Path {
        self.working_dir.join(path).resolve()
    }

    pub fn start_editor<'f>(&self, file: &'f H5File) -> rustyline::Result<LineEditor<'f>> {
        LineEditor::new(self.commands.keys().cloned().collect(), file)
    }

    pub fn parse_and_execute_input(&mut self, input: &str, h5file: &H5File) -> CommandOutcome {
        let (cmd, args) = split_cmd(input);
        let Some(cmd) = self.get_command(cmd) else {
            self.printer()
                .print_shell_error(format!("Unknown command: {cmd}"));
            return CommandOutcome::KeepRunning;
        };
        match self.parse_and_run_command(cmd, args, h5file) {
            Ok(outcome) => outcome,
            Err(err) => {
                self.printer().print_cmd_error(&err);
                match err {
                    CommandError::Critical(_) => CommandOutcome::ExitFailure,
                    CommandError::Exit => CommandOutcome::ExitSuccess,
                    _ => CommandOutcome::KeepRunning,
                }
            }
        }
    }

    fn parse_and_run_command(
        &mut self,
        cmd: Rc<dyn Command>,
        args: &str,
        h5file: &H5File,
    ) -> cmd::CmdResult {
        match cmd
            .arg_parser()
            .no_binary_name(true)
            .try_get_matches_from(split_args(args))
        {
            Ok(matches) => cmd.run(matches, self, h5file),
            Err(err) => match err.kind() {
                clap::error::ErrorKind::DisplayHelp => {
                    self.printer().println(cmd.arg_parser().render_help());
                    Ok(CommandOutcome::KeepRunning)
                }
                _ => {
                    self.printer().println(err.render());
                    Err(CommandError::NoMessage)
                }
            },
        }
    }
}

fn split_cmd(input: &str) -> (&str, &str) {
    input.trim_start().split_once(' ').unwrap_or((input, ""))
}

fn split_args(args: &str) -> impl Iterator<Item = &str> {
    // TODO properly handle quotes and escapes
    //   this should be handled by / in cooperation with the editor for highlighting
    args.split(' ').filter(|s| !s.is_empty())
}
