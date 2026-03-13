use clap::{Parser, Subcommand};

use notch::client::{self, parse_page_id, Client};
use notch::markdown::format_output;

#[derive(Parser)]
#[command(name = "notch", about = "Notion Page to Markdown CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Fetch a Notion page as Markdown
    Fetch {
        /// Page ID or Notion URL
        page_id_or_url: String,
    },
    /// Search Notion pages by title
    Search {
        /// Search query
        query: String,
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
    }

    Ok(())
}
