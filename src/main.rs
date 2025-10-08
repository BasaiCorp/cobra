use clap::{Parser, Subcommand};
use cobra::{Result, CobraError};
use colored::Colorize;
use std::time::Instant;

#[derive(Parser)]
#[command(name = "cobra")]
#[command(about = "⚡ Ultra-fast Python package manager - 20x faster than pip", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new cobra.toml configuration
    Init {
        #[arg(short, long, default_value = ".")]
        path: String,
    },
    
    /// Install packages from cobra.toml
    Install {
        #[arg(short, long)]
        no_cache: bool,
    },
    
    /// Add a package to cobra.toml
    Add {
        packages: Vec<String>,
    },
    
    /// Remove a package from cobra.toml
    Remove {
        packages: Vec<String>,
    },
    
    /// Update all packages
    Update {
        #[arg(short, long)]
        package: Option<String>,
    },
    
    /// List installed packages
    List,
    
    /// Show detailed package information
    Show {
        package: String,
    },
    
    /// Search PyPI for packages
    Search {
        query: String,
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();
    
    let start = Instant::now();
    let cli = Cli::parse();
    
    let result = match cli.command {
        Commands::Init { path } => {
            cobra::cli::init::execute(&path).await
        }
        Commands::Install { no_cache } => {
            cobra::cli::install::execute(no_cache).await
        }
        Commands::Add { packages } => {
            cobra::cli::add::execute(packages).await
        }
        Commands::Remove { packages } => {
            cobra::cli::remove::execute(packages).await
        }
        Commands::Update { package } => {
            cobra::cli::update::execute(package).await
        }
        Commands::List => {
            cobra::cli::list::execute().await
        }
        Commands::Show { package } => {
            cobra::cli::show::execute(package).await
        }
        Commands::Search { query, limit } => {
            cobra::cli::search::execute(query, Some(limit)).await
        }
    };
    
    match result {
        Ok(_) => {
            let elapsed = start.elapsed();
            println!(
                "\n{} Completed in {:.2}s",
                "✓".green().bold(),
                elapsed.as_secs_f64()
            );
        }
        Err(e) => {
            eprintln!("{} {}", "✗".red().bold(), e);
            std::process::exit(1);
        }
    }
}
