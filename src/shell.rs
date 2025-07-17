use std::rc::Rc;

use crate::cmd::{self, Command, CommandError, CommandOutcome};
use crate::h5::{H5File, H5Path};
use crate::line_editor::LineEditor;
use crate::line_editor::parse::{Argument, Expression, Parser};
use crate::output::Printer;

pub struct Shell {
    working_group: H5Path,
    printer: Printer,
    commands: cmd::Commands,
}

impl Shell {
    pub fn new() -> Self {
        Self {
            working_group: H5Path::root(),
            printer: Printer::new(),
            commands: cmd::Commands::new(),
        }
    }

    pub fn printer(&self) -> &Printer {
        &self.printer
    }

    pub fn commands(&self) -> &cmd::Commands {
        &self.commands
    }

    fn get_command(&self, name: &str) -> Option<Rc<dyn Command>> {
        self.commands.get_command(name)
    }

    pub fn get_working_group(&self) -> &H5Path {
        &self.working_group
    }

    pub fn set_working_group(&mut self, path: H5Path) {
        self.working_group = path;
    }

    pub fn resolve_path(&self, path: &H5Path) -> H5Path {
        self.working_group.join(path).resolve()
    }

    pub fn start_editor<'f>(&self, file: &'f H5File) -> rustyline::Result<LineEditor<'f>> {
        LineEditor::new(self.commands.keys().cloned().collect(), file)
    }

    pub fn parse_and_execute_input(&mut self, input: &str, h5file: &H5File) -> CommandOutcome {
        let (cmd, args) = parse_and_resolve_input(input, &self.commands);
        let Some(cmd) = self.get_command(&cmd) else {
            self.printer()
                .print_shell_error(format!("Unknown command: {cmd}"));
            return CommandOutcome::KeepRunning;
        };
        match self.parse_and_run_command(cmd, &args, h5file) {
            Ok(outcome) => outcome,
            Err(err) => {
                self.printer().print_cmd_error(&err);
                match err {
                    CommandError::Critical(_) => CommandOutcome::ExitFailure,
                    _ => CommandOutcome::KeepRunning,
                }
            }
        }
    }

    fn parse_and_run_command(
        &mut self,
        cmd: Rc<dyn Command>,
        args: &[String],
        h5file: &H5File,
    ) -> cmd::CmdResult {
        match cmd
            .arg_parser()
            .no_binary_name(true)
            .try_get_matches_from(args)
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

fn parse_and_resolve_input(src: &str, commands: &cmd::Commands) -> (String, Vec<String>) {
    let expression = Parser::new(src).parse();

    match expression {
        Expression::Call(call) => {
            let function = call.function.get_content(src);
            match commands.get_alias(function) {
                Some(alias) => parse_and_resolve_input(
                    &format!("{alias} {}", call.get_args_str(src)),
                    commands,
                ),
                None => (function.to_string(), collect_args(&call.arguments, src)),
            }
        }
        Expression::String(string) => (string.get_content(src).to_string(), Vec::new()),
        Expression::Noop => (String::new(), Vec::new()),
    }
}

fn collect_args(arguments: &[Argument], src: &str) -> Vec<String> {
    arguments
        .iter()
        .map(|arg| arg.get_content(src).to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn parse_and_resolve_input_empty() {
        let input = "";
        let commands = cmd::Commands::new();

        let (cmd, args) = parse_and_resolve_input(input, &commands);
        assert_eq!(cmd, "");
        assert_eq!(args, Vec::<&str>::new());
    }

    #[test]
    fn parse_and_resolve_input_only_command() {
        let input = "help";
        let commands = cmd::Commands::new();

        let (cmd, args) = parse_and_resolve_input(input, &commands);
        assert_eq!(cmd, "help");
        assert_eq!(args, Vec::<&str>::new());
    }

    #[test]
    fn parse_and_resolve_input_command_with_pos_arg() {
        let input = "cd some/where";
        let commands = cmd::Commands::new();

        let (cmd, args) = parse_and_resolve_input(input, &commands);
        assert_eq!(cmd, "cd");
        assert_eq!(args, vec!["some/where"]);
    }

    #[test]
    fn parse_and_resolve_input_command_with_mixed_arg() {
        let input = "ls -l path";
        let commands = cmd::Commands::new();

        let (cmd, args) = parse_and_resolve_input(input, &commands);
        assert_eq!(cmd, "ls");
        assert_eq!(args, vec!["-l", "path"]);
    }

    #[test]
    fn parse_and_resolve_input_only_alias() {
        let input = "l";
        let mut commands = cmd::Commands::new();
        commands.add_alias("l", "ls -l");

        let (cmd, args) = parse_and_resolve_input(input, &commands);
        assert_eq!(cmd, "ls");
        assert_eq!(args, vec!["-l"]);
    }

    #[test]
    fn parse_and_resolve_input_alias_with_pos_arg() {
        let input = "l path";
        let mut commands = cmd::Commands::new();
        commands.add_alias("l", "ls -l");

        let (cmd, args) = parse_and_resolve_input(input, &commands);
        assert_eq!(cmd, "ls");
        assert_eq!(args, vec!["-l", "path"]);
    }

    #[test]
    fn parse_and_resolve_input_alias_with_mixed_arg() {
        let input = "l --name path";
        let mut commands = cmd::Commands::new();
        commands.add_alias("l", "ls -l");

        let (cmd, args) = parse_and_resolve_input(input, &commands);
        assert_eq!(cmd, "ls");
        assert_eq!(args, vec!["-l", "--name", "path"]);
    }

    #[test]
    fn parse_and_resolve_input_recursive_alias_with_mixed_arg() {
        let input = "dir group/inner --name";
        let mut commands = cmd::Commands::new();
        commands.add_alias("dir", "l --type");
        commands.add_alias("l", "ls -l");

        let (cmd, args) = parse_and_resolve_input(input, &commands);
        assert_eq!(cmd, "ls");
        assert_eq!(args, vec!["-l", "--type", "group/inner", "--name"]);
    }
}
