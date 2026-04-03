use std::io::{IsTerminal, Read};

use clap::{Parser, Subcommand};

use notch::client::{self, parse_page_id, Client};
use notch::markdown::format_output;

const FETCH_AFTER_HELP: &str = "\
Examples:
  notch fetch https://notion.so/My-Page-abc123
  notch fetch abc123def456...
  echo \"page-id\" | notch fetch
  notch fetch -
";

const SEARCH_AFTER_HELP: &str = "\
Examples:
  notch search \"keyword\"
";

const QUERY_AFTER_HELP: &str = "\
Examples:
  notch query https://notion.so/database-id
  notch query abc123def456...
  echo \"database-id\" | notch query
  notch query -
";

#[derive(Parser)]
#[command(name = "notch", about = "Notion Page to Markdown CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Fetch a Notion page as Markdown
    #[command(after_help = FETCH_AFTER_HELP)]
    Fetch {
        /// Page ID or Notion URL. Reads piped stdin when omitted, or any stdin with `-`.
        page_id_or_url: Option<String>,
    },
    /// Search Notion pages by title
    #[command(after_help = SEARCH_AFTER_HELP)]
    Search {
        /// Search query
        query: String,
    },
    /// Query a Notion database
    #[command(after_help = QUERY_AFTER_HELP)]
    Query {
        /// Database ID or Notion URL. Reads piped stdin when omitted, or any stdin with `-`.
        database_id_or_url: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    let cli = Cli::parse();

    if let Err(e) = run(cli).await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<(), client::NotchError> {
    let client = Client::new()?;

    match cli.command {
        Commands::Fetch { page_id_or_url } => {
            let stdin = std::io::stdin();
            let page_id_or_url =
                resolve_resource_input(page_id_or_url, stdin.lock(), stdin.is_terminal())?;
            let page_id = parse_page_id(&page_id_or_url)?;
            let (md_resp, meta) = tokio::try_join!(
                client.fetch_markdown(&page_id),
                client.fetch_metadata(&page_id),
            )?;

            if !md_resp.unknown_block_ids.is_empty() {
                eprintln!(
                    "Warning: {} block(s) could not be converted to Markdown",
                    md_resp.unknown_block_ids.len()
                );
            }

            let title = meta.properties.title_text();
            let result = format_output(&title, &md_resp.markdown, md_resp.truncated);

            for warning in &result.warnings {
                eprintln!("{warning}");
            }

            print!("{}", result.stdout);
        }
        Commands::Search { query } => {
            let resp = client.search(&query).await?;

            if resp.results.is_empty() {
                eprintln!("No pages found for: {query}");
                return Ok(());
            }

            for page in &resp.results {
                let title = page.properties.title_text();
                let title = if title.is_empty() {
                    "(Untitled)"
                } else {
                    &title
                };
                println!("{}\t{}\t{}", page.id, title, page.last_edited_time);
            }
        }
        Commands::Query { database_id_or_url } => {
            let stdin = std::io::stdin();
            let database_id_or_url =
                resolve_resource_input(database_id_or_url, stdin.lock(), stdin.is_terminal())?;
            let db_id = parse_page_id(&database_id_or_url)?;
            let db = client.retrieve_database(&db_id).await?;

            if db.data_sources.is_empty() {
                return Err(client::NotchError::NoDataSources);
            }

            let ds_id = &db.data_sources[0].id;
            let resp = client.query_data_source(ds_id).await?;

            if resp.results.is_empty() {
                eprintln!("No rows found for database: {db_id}");
                return Ok(());
            }

            if resp.has_more {
                eprintln!(
                    "Warning: Results truncated. {} rows returned, more available",
                    resp.results.len()
                );
            }

            let columns = resp.results[0].properties.sorted_names();
            println!("id\t{}", columns.join("\t"));

            for row in &resp.results {
                let values: Vec<String> = columns
                    .iter()
                    .map(|col| row.properties.property_text(col))
                    .collect();
                println!("{}\t{}", row.id, values.join("\t"));
            }
        }
    }

    Ok(())
}

fn resolve_resource_input(
    value: Option<String>,
    mut stdin: impl Read,
    stdin_is_terminal: bool,
) -> Result<String, client::NotchError> {
    match value {
        Some(value) if value != "-" => Ok(value),
        Some(_) => read_stdin_value(&mut stdin),
        None if stdin_is_terminal => Err(client::NotchError::InvalidInput(
            "Missing ID/URL argument. Pipe one via stdin or pass `-` to read stdin interactively"
                .to_string(),
        )),
        None => read_stdin_value(&mut stdin),
    }
}

fn read_stdin_value(mut stdin: impl Read) -> Result<String, client::NotchError> {
    let mut buffer = String::new();
    stdin.read_to_string(&mut buffer)?;

    let trimmed = buffer.trim();
    if trimmed.is_empty() {
        return Err(client::NotchError::InvalidInput(
            "No input provided. Pass an ID/URL argument or pipe one via stdin".to_string(),
        ));
    }

    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    use std::io::Cursor;

    fn subcommand_help(name: &str) -> String {
        let mut command = Cli::command();
        command
            .find_subcommand_mut(name)
            .unwrap()
            .render_help()
            .to_string()
    }

    fn assert_help_contains_examples(name: &str, snippets: &[&str]) {
        let help = subcommand_help(name);

        assert!(
            help.contains("Examples:"),
            "subcommand '{name}' should include Examples"
        );

        for snippet in snippets {
            assert!(
                help.contains(snippet),
                "subcommand '{name}' should include example '{snippet}'"
            );
        }
    }

    fn parse_fetch(args: &[&str]) -> Option<String> {
        match Cli::try_parse_from(args).unwrap().command {
            Commands::Fetch { page_id_or_url } => page_id_or_url,
            _ => panic!("expected Fetch"),
        }
    }

    fn parse_query(args: &[&str]) -> Option<String> {
        match Cli::try_parse_from(args).unwrap().command {
            Commands::Query { database_id_or_url } => database_id_or_url,
            _ => panic!("expected Query"),
        }
    }

    #[test]
    fn subcommand_help_includes_examples() {
        for (name, snippets) in [
            (
                "fetch",
                &[
                    "notch fetch https://notion.so/My-Page-abc123",
                    "echo \"page-id\" | notch fetch",
                ][..],
            ),
            ("search", &["notch search \"keyword\""][..]),
            ("query", &["notch query https://notion.so/database-id"][..]),
        ] {
            assert_help_contains_examples(name, snippets);
        }
    }

    #[test]
    fn resolve_resource_input_handles_positional_and_stdin_cases() {
        for (input, stdin, stdin_is_terminal, expected) in [
            (Some("abc123"), "", false, "abc123"),
            (Some("-"), "abc123\n", true, "abc123"),
            (None, "abc123\n", false, "abc123"),
        ] {
            let value = resolve_resource_input(
                input.map(str::to_string),
                Cursor::new(stdin.as_bytes()),
                stdin_is_terminal,
            )
            .unwrap();
            assert_eq!(value, expected);
        }
    }

    #[test]
    fn resolve_resource_input_rejects_missing_argument_on_tty() {
        let err = resolve_resource_input(None, Cursor::new(Vec::<u8>::new()), true).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Missing ID/URL argument. Pipe one via stdin or pass `-` to read stdin interactively"
        );
    }

    #[test]
    fn resolve_resource_input_rejects_empty_stdin_with_dash() {
        let err =
            resolve_resource_input(Some("-".to_string()), Cursor::new(Vec::<u8>::new()), true)
                .unwrap_err();
        assert_eq!(
            err.to_string(),
            "No input provided. Pass an ID/URL argument or pipe one via stdin"
        );
    }

    #[test]
    fn resolve_resource_input_rejects_empty_piped_stdin() {
        let err = resolve_resource_input(None, Cursor::new(Vec::<u8>::new()), false).unwrap_err();
        assert_eq!(
            err.to_string(),
            "No input provided. Pass an ID/URL argument or pipe one via stdin"
        );
    }

    #[test]
    fn cli_parses_optional_stdin_inputs() {
        assert_eq!(parse_fetch(&["notch", "fetch", "-"]).as_deref(), Some("-"));
        assert_eq!(parse_fetch(&["notch", "fetch"]), None);
        assert_eq!(parse_query(&["notch", "query", "-"]).as_deref(), Some("-"));
        assert_eq!(parse_query(&["notch", "query"]), None);
    }

    #[test]
    fn all_subcommands_have_examples_in_after_help() {
        let command = Cli::command();

        for sub in command.get_subcommands() {
            let after_help = sub
                .get_after_help()
                .map(|help| help.to_string())
                .unwrap_or_default();

            assert!(
                after_help.contains("Examples"),
                "subcommand '{}' should have Examples in after_help",
                sub.get_name()
            );
        }
    }
}
