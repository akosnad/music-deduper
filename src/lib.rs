use blake3::Hash;
use itertools::Itertools;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use walkdir::DirEntry;

pub struct Processor {
    files: Vec<DirEntry>,
}

impl Processor {
    pub fn new(files: Vec<DirEntry>) -> anyhow::Result<Self> {
        ffmpeg_next::init()?;
        Ok(Self { files })
    }

    pub fn process(&mut self) -> anyhow::Result<Vec<&DirEntry>> {
        let results = self.generate_hashes();

        log::debug!("Finding duplicates...");
        let dupes = results
            .iter()
            .duplicates_by(|(_, h1)| h1.as_bytes())
            .collect_vec();
        log::debug!("Found {} duplicates", dupes.len());

        /*
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
        */

        log::debug!("Reading duplicate files metadata...");
        let old_dupes = dupes
            .iter()
            .flat_map(|(_, h1)| {
                results
                    .iter()
                    .filter(|(_, h)| h.as_bytes() == h1.as_bytes())
                    .sorted_by(|(f1, _), (f2, _)| {
                        let f1_meta = f1.metadata().unwrap();
                        let f2_meta = f2.metadata().unwrap();

                        f1_meta
                            .modified()
                            .unwrap()
                            .cmp(&f2_meta.modified().unwrap())
                    })
                    .rev()
                    .skip(1)
            })
            .map(|(f, _)| *f)
            .collect_vec();

        log::debug!(
            "Found {} duplicates that can be deleted safely",
            old_dupes.len()
        );

        Ok(old_dupes)
    }

    fn generate_hashes(&self) -> Vec<(&DirEntry, Hash)> {
        log::debug!("Generating hashes for {} files", self.files.len());
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
