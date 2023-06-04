use anyhow::anyhow;
use clap::Parser;
use human_bytes::human_bytes;
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command()]
/// Music library duplicate finder, using fingerprinting
struct Args {
    #[arg(short, long)]
    /// The directory to search for duplicates in
    cwd: Option<String>,

    #[arg(short, long)]
    /// Delete the duplicates from the filesystem
    delete: bool,

    #[arg(short, long)]
    /// Show detailed output
    verbose: bool,

    #[arg(short, long)]
    /// Don't show file paths
    quiet: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    simple_logger::SimpleLogger::new().env().init()?;

    if args.verbose {
        log::set_max_level(log::LevelFilter::Debug);
    } else {
        log::set_max_level(log::LevelFilter::Info);
    }

    if let Some(cwd) = args.cwd {
        let path = std::path::Path::new(&cwd);
        if !path.exists() || !path.is_dir() {
            return Err(anyhow!("{} is not a valid directory", cwd));
        }

        std::env::set_current_dir(path)?;
    }

    //let mut duplicates = HashMap::new();

    let files = WalkDir::new(".")
        .follow_links(true)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| {
            let ext = e.path().extension().unwrap_or_default();
            e.file_type().is_file() && (ext == "mp3" || ext == "flac")
        })
        .collect::<Vec<_>>();

    let mut processor = music_deduper::Processor::new(files)?;
    let to_delete = processor.process()?;

    let to_delete_size = to_delete
        .iter()
        .map(|f| f.metadata().unwrap().len())
        .sum::<u64>();

    if args.delete {
        for file in &to_delete {
            if !args.quiet {
                println!("{}", file.path().display());
            }
            std::fs::remove_file(file.path())?;
        }
        log::info!(
            "{} duplicates deleted with size: {}",
            to_delete.len(),
            human_bytes(to_delete_size as f64)
        );
        return Ok(());
    } else if !args.quiet {
        for file in &to_delete {
            println!("{}", file.path().display());
        }
    }
    log::info!(
        "{} duplicates found with size: {}",
        to_delete.len(),
        human_bytes(to_delete_size as f64)
    );

    Ok(())
}
