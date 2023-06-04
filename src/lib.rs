use blake3::Hash;
use ffmpeg_next::frame::Audio;
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
        let hashes = self
            .files
            .par_iter()
            .map(|f| -> anyhow::Result<(&Path, Hash)> {
                let mut ictx = ffmpeg_next::format::input(&f.path())?;
                let input = ictx
                    .streams()
                    .best(ffmpeg_next::media::Type::Audio)
                    .ok_or(ffmpeg_next::Error::StreamNotFound)?;
                let audio_stream_index = input.index();

                let ctx_decoder = ffmpeg_next::codec::Context::from_parameters(input.parameters())?;
                let mut decoder = ctx_decoder.decoder().audio()?;

                let mut hasher = blake3::Hasher::new();

                let mut process_packets = |decoder: &mut ffmpeg_next::decoder::Audio| {
                    let mut decoded = Audio::empty();
                    while decoder.receive_frame(&mut decoded).is_ok() {
                        let data = decoded.data(0);
                        hasher.update(data);
                    }
                };

                for (stream, packet) in ictx.packets() {
                    if stream.index() == audio_stream_index {
                        decoder.send_packet(&packet)?;
                        process_packets(&mut decoder);
                    }
                }

                decoder.send_eof()?;
                decoder.flush();
                process_packets(&mut decoder);

                Ok((f.path(), hasher.finalize()))
            })
            .filter_map(|r| r.ok())
            .collect::<Vec<_>>();

        for (path, hash) in hashes {
            println!("{}: {}", path.display(), hash);
        }

        Ok(())
    }
}
