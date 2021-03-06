use std::any::Any;
use std::process;

use crate::as_any::AsAny;
use crate::error::Error;
use crate::row::Row;
use crate::table::Table;

/// This function is just a proxy that creates a `Command` or returns an `Error`.
/// The way it decides if it will return a `MetaCommand` or a `Statement` is
/// by looking on the `String` `input` if it starts with a dot (`.`).
fn build_command(input: &str) -> Result<Box<dyn Command>, Error> {
    if input.chars().next() == Some('.') {
        MetaCommand::from_str(&input.trim())
    } else {
        Statement::from_str(&input.trim())
    }
}

/// Receives a table and a string, and tries to build the
/// command and execute it right away
pub fn run_command(table: &mut Table, command: String) -> Result<String, Error> {
    let command_result = build_command(&command);

    try_execute_command(command_result, table)
}

fn try_execute_command(
    command_result: Result<Box<dyn Command>, Error>,
    table: &mut Table,
) -> Result<String, Error> {
    command_result?.execute(table)
}

/// Creates an `Error` with the default `"not implemented"` message.
fn build_not_implemented_error(input: &str) -> Error {
    let message = format!("Unrecognized keyword at start of '{}'", input);
    Error::UnrecognizedStatement(message)
}

/// The interface that every `Command` asks for is just an `execute` method, which
/// executes the specific logic for the `Command`.
pub trait Command: AsAny {
    fn execute(&self, table: &mut Table) -> Result<String, Error>;
}

/// `MetaCommand` is the `enum` that contains all meta commands for `scoolite`.
/// An example of meta command is `.exit`, it does not belong to the `SQL` specification
/// however it is used to close the program/REPL.
#[derive(Debug, PartialEq)]
enum MetaCommand {
    Exit,
}

impl MetaCommand {
    /// Tries to parse an `&str` `input` into a `Box<Command>`, if
    /// it fails it returns a `"not implemented error"` `Error`.
    ///
    /// All of the possibilities are just the variants on the `enum`.
    fn from_str(input: &str) -> Result<Box<dyn Command>, Error> {
        match input {
            ".exit" => Ok(Box::new(MetaCommand::Exit)),
            _ => Err(build_not_implemented_error(input)),
        }
    }
}

impl Command for MetaCommand {
    /// Executes an different logic for each variant of the `enum`.
    fn execute(&self, _table: &mut Table) -> Result<String, Error> {
        match *self {
            MetaCommand::Exit => process::exit(0),
        }
    }
}

impl AsAny for MetaCommand {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// `Statement` is the `enum` that contains all of the statements for `scoolite`.
/// An example of a statement is `insert`, it does belong to the `SQL` specification
/// and it is used to add a row to a table.
#[derive(Debug, PartialEq)]
enum Statement {
    Insert(String),
    Select,
}

impl Statement {
    /// Tries to parse an `&str` `input` into a `Box<Command>`, if
    /// it fails it returns a `"not implemented error"` `Error`.
    ///
    /// All of the possibilities are just the variants on the `enum`.
    fn from_str(input: &str) -> Result<Box<dyn Command>, Error> {
        let input = input.to_string();

        if input.starts_with("insert") {
            Ok(Box::new(Statement::Insert(input)))
        } else if input.starts_with("select") {
            Ok(Box::new(Statement::Select))
        } else {
            Err(build_not_implemented_error(&input))
        }
    }

    /// Creates a new `Row` based of an `input` `&str` and inserts it
    /// inside of a `table`.
    /// This is what get's called when something like
    /// `Statement::Insert("insert 1 john john@mailbox.com").execute()` happens.
    fn insert(&self, input: &str, table: &mut Table) -> Result<String, Error> {
        let row = Row::from_str(&input)?;

        table.add_row(row);

        Ok("".to_string())
    }

    /// Returns all `Row`s inside of a table as String.
    /// This is what get's called when something like
    /// `Statement::Select.execute()` happens.
    fn select(&self, table: &Table) -> Result<String, Error> {
        let rows = table.list_rows();

        Ok(rows.iter().map(|r| format!("{}\n", r)).collect())
    }
}

impl Command for Statement {
    /// Executes an different logic for each variant of the `enum`.
    /// If it succeeds, it will return the String of the command executed
    /// concatenated with `Executed.\n`.
    fn execute(&self, table: &mut Table) -> Result<String, Error> {
        let result = match self {
            Statement::Insert(input) => self.insert(&input, table),
            Statement::Select => self.select(table),
        };

        if result.is_ok() {
            return result.map(|s| format!("{}Executed.\n", s));
        }

        result
    }
}

impl AsAny for Statement {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod test {
    use crate::command::{build_command, run_command, MetaCommand, Statement};
    use crate::error::Error;
    use crate::table::Table;

    #[test]
    fn build_command_meta_command() {
        let input = ".exit".to_string();

        let command = build_command(&input).unwrap();

        // stupid necessary casting, because command is a Command trait object
        let command = command.as_any().downcast_ref::<MetaCommand>().unwrap();

        assert_eq!(*command, MetaCommand::Exit);
    }

    #[test]
    fn build_command_statement() {
        let input = "insert a b c".to_string();

        let command = build_command(&input).unwrap();

        // stupid necessary casting, because command is a Command trait object
        let command = command.as_any().downcast_ref::<Statement>().unwrap();

        assert_eq!(*command, Statement::Insert(input));
    }

    #[test]
    fn statement_from_str_insert() {
        let input = "insert a b c";

        let insert_statement = Statement::from_str(input).unwrap();

        let insert_statement = insert_statement
            .as_any()
            .downcast_ref::<Statement>()
            .unwrap();

        assert_eq!(*insert_statement, Statement::Insert(input.to_string()));
    }

    #[test]
    fn statement_from_str_select() {
        let input = "select";

        let select_statement = Statement::from_str(input).unwrap();

        let select_statement = select_statement
            .as_any()
            .downcast_ref::<Statement>()
            .unwrap();

        assert_eq!(*select_statement, Statement::Select);
    }

    #[test]
    fn statement_from_str_not_implemented_error() {
        let input = "unexistent statement";

        let unimplemented_error = Statement::from_str(input).err().unwrap();

        let expected_error_message =
            "Unrecognized keyword at start of \'unexistent statement\'".to_string();

        assert_eq!(
            unimplemented_error,
            Error::UnrecognizedStatement(expected_error_message)
        );
    }

    #[test]
    fn run_command_insert_with_select_success() {
        let mut table = Table::new();

        let output = run_command(
            &mut table,
            "insert 1 otaviopace otavio@gmail.com".to_string(),
        )
        .unwrap();

        assert_eq!(output, "Executed.\n");

        let output = run_command(&mut table, "select".to_string()).unwrap();

        assert_eq!(output, "(1, otaviopace, otavio@gmail.com)\nExecuted.\n");
    }

    #[test]
    fn run_command_insert_syntax_error() {
        let mut table = Table::new();

        let error = run_command(
            &mut table,
            "insert text_id otaviopace otavio@gmail.com".to_string(),
        )
        .unwrap_err();

        assert_eq!(
            error,
            Error::SyntaxError("Syntax error. Failed to parse 'id' of input".to_string())
        );
    }

    #[test]
    fn run_command_insert_negative_id() {
        let mut table = Table::new();

        let error = run_command(
            &mut table,
            "insert -1 otaviopace otavio@gmail.com".to_string(),
        )
        .unwrap_err();

        assert_eq!(
            error,
            Error::SyntaxError("Syntax error. Failed to parse 'id' of input".to_string())
        );
    }
}
