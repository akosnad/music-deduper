use blake3::Hash;
use itertools::Itertools;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use std::path::{Path, PathBuf};
use walkdir::DirEntry;

pub struct Processor {
    parent: PathBuf,
    files: Vec<DirEntry>,
}

impl Processor {
    pub fn new(parent: PathBuf, files: Vec<DirEntry>) -> anyhow::Result<Self> {
        ffmpeg_next::init()?;
        Ok(Self { parent, files })
    }

    pub fn process(&mut self) -> anyhow::Result<()> {
        let results = self.generate_hashes();

        let dupes = results
            .iter()
            .duplicates_by(|(_, h1)| h1.as_bytes())
            .collect_vec();

        for dupe in &dupes {
            results
                .iter()
                .filter(|(p, h)| {
                    h.as_bytes() == dupe.1.as_bytes() && p.path().to_str() != dupe.0.path().to_str()
                })
                .for_each(|(p, _)| {
                    println!(
                        "{} is a duplicate of {}",
                        p.path().display(),
                        dupe.0.path().display()
                    );
                });
        }

        Ok(())
    }

    fn generate_hashes(&self) -> Vec<(&DirEntry, Hash)> {
        self.files
            .par_iter()
            .map(|f| -> anyhow::Result<(&DirEntry, Hash)> {
                let mut ictx = ffmpeg_next::format::input(&f.path())?;
                let input = ictx
                    .streams()
                    .best(ffmpeg_next::media::Type::Audio)
                    .ok_or(ffmpeg_next::Error::StreamNotFound)?;
                let audio_stream_index = input.index();

                let mut hasher = blake3::Hasher::new();

                ictx.packets().for_each(|(stream, packet)| {
                    if stream.index() == audio_stream_index {
                        packet.data().map(|data| hasher.update(data));
                    }
                });

                Ok((f, hasher.finalize()))
            })
            .filter_map(Result::ok)
            .collect()
    }
}
