use anyhow::anyhow;
use clap::Parser;
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(author = "akosnad", about = "Music library duplicate finder")]
struct Args {
    #[arg(short, long)]
    cwd: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

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

    let mut processor = music_deduper::Processor::new(std::env::current_dir()?, files)?;
    processor.process()?;

    Ok(())
}
