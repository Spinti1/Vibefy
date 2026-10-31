//! Opt-in Track Peek: one cancellable native preview, no queue/history changes.
use crate::player::{EngineConfig, LocalTrack, MatchCache};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
    time::{Duration, Instant},
};

#[derive(Clone, Debug)]
pub enum Status {
    Loading,
    Playing,
    Finished,
    Failed(String),
}
pub struct Session {
    pub track: LocalTrack,
    pub cancelled: AtomicBool,
    pub volume: AtomicU32,
    pub status: Mutex<Status>,
}
impl Session {
    pub fn new(track: LocalTrack, volume: u16) -> Arc<Self> {
        Arc::new(Self {
            track,
            cancelled: AtomicBool::new(false),
            volume: AtomicU32::new(u32::from(volume)),
            status: Mutex::new(Status::Loading),
        })
    }
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
    pub fn status(&self) -> Status {
        self.status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
    fn set(&self, status: Status) {
        *self.status.lock().unwrap_or_else(|e| e.into_inner()) = status;
    }
}
pub struct Active {
    pub session: Arc<Session>,
    pub resume: Option<(String, u64)>,
}
impl Drop for Active {
    fn drop(&mut self) {
        self.session.cancel();
    }
}

/// Resolution stays on the existing runtime. Decode/output only starts on demand.
pub async fn run(
    session: Arc<Session>,
    config: EngineConfig,
    cache: Option<MatchCache>,
    wake: crate::backend::Waker,
) {
    let result: anyhow::Result<()> = async {
        let cache=cache.ok_or_else(||anyhow::anyhow!("Connect local playback before previewing audio."))?;
        let binary=crate::player::find_yt_dlp_binary().ok_or_else(||anyhow::anyhow!("The audio helper is unavailable."))?;
        let bytes=tokio::select!{
            result=tokio::time::timeout(Duration::from_secs(25),crate::player::resolve_track_audio(&binary,&cache,&session.track,&config.proxy))=>result.map_err(|_|anyhow::anyhow!("Preview took too long. Try again later."))??,
            _=async{while !session.cancelled.load(Ordering::Relaxed){tokio::time::sleep(Duration::from_millis(40)).await;}}=>return Ok(()),
        };
        if session.cancelled.load(Ordering::Relaxed){return Ok(());}
        let active=Arc::clone(&session);let waker=wake.clone();
        tokio::task::spawn_blocking(move||play(&active,&config,bytes,&waker)).await??;
        Ok(())
    }.await;
    if !session.cancelled.load(Ordering::Relaxed) {
        session.set(match result {
            Ok(()) => Status::Finished,
            Err(e) => Status::Failed(e.to_string()),
        });
        wake.wake();
    }
}
struct Volume(Arc<Session>);
impl crate::sink::VolumeGetter for Volume {
    fn attenuation_factor(&self) -> f64 {
        f64::from(self.0.volume.load(Ordering::Relaxed)) / 65535.0
    }
}

fn play(
    session: &Arc<Session>,
    config: &EngineConfig,
    bytes: Vec<u8>,
    wake: &crate::backend::Waker,
) -> anyhow::Result<()> {
    use crate::sink::{AudioControl, RodioSink, Sink};
    let mut sink = RodioSink::new(
        config.audio_device.clone(),
        Arc::new(|e| log::warn!("Track Peek output: {e}")),
        Box::new(Volume(Arc::clone(session))),
        config.buffer_ms,
        AudioControl::new(config.buffer_ms),
    );
    let result = decode(session, Arc::clone(&config.eq), &mut sink, bytes, wake);
    let until = Instant::now() + Duration::from_secs(2);
    while sink.queued_ms() > 0
        && !session.cancelled.load(Ordering::Relaxed)
        && Instant::now() < until
    {
        std::thread::sleep(Duration::from_millis(10));
    }
    sink.stop()?;
    result.map(|_| ())
}

fn decode(
    session: &Arc<Session>,
    shared_eq: crate::eq::SharedEq,
    sink: &mut dyn crate::sink::Sink,
    bytes: Vec<u8>,
    wake: &crate::backend::Waker,
) -> anyhow::Result<usize> {
    use crate::sink::{AudioPacket, Converter, SAMPLE_RATE};
    use symphonia::core::{
        audio::SampleBuffer,
        codecs::{CODEC_TYPE_NULL, DecoderOptions},
        formats::FormatOptions,
        io::MediaSourceStream,
        meta::MetadataOptions,
        probe::Hint,
    };
    let stream = MediaSourceStream::new(Box::new(std::io::Cursor::new(bytes)), Default::default());
    let mut format = symphonia::default::get_probe()
        .format(
            &Hint::new(),
            stream,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )?
        .format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| anyhow::anyhow!("No preview audio found."))?;
    let id = track.id;
    let rate = track.codec_params.sample_rate.unwrap_or(SAMPLE_RATE);
    let mut decoder =
        symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default())?;
    let mut resampler = crate::resample::Resampler::new(rate, SAMPLE_RATE, 2);
    let mut eq = crate::eq::Processor::new(shared_eq);
    let mut converter = Converter::new(None);
    let mut sent = 0usize;
    let max = SAMPLE_RATE as usize * 30;
    let mut started = false;
    let mut sample_buf: Option<SampleBuffer<f32>> = None;
    while sent < max && !session.cancelled.load(Ordering::Relaxed) {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(e) => return Err(e.into()),
        };
        if packet.track_id() != id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(e) => return Err(e.into()),
        };
        let channels = decoded.spec().channels.count();
        anyhow::ensure!(channels > 0, "Invalid preview channels");
        if sample_buf
            .as_ref()
            .is_some_and(|buf| buf.capacity() < decoded.capacity() * channels)
        {
            sample_buf = None;
        }
        let buf = sample_buf.get_or_insert_with(|| {
            SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec())
        });
        buf.copy_interleaved_ref(decoded);
        let stereo: Vec<f32> = buf
            .samples()
            .chunks_exact(channels)
            .flat_map(|frame| [frame[0], frame[if channels > 1 { 1 } else { 0 }]])
            .collect();
        let samples = match resampler.as_mut() {
            Some(r) => std::borrow::Cow::Owned(r.process(&stereo)),
            None => std::borrow::Cow::Borrowed(stereo.as_slice()),
        };
        let count = (max - sent).min(samples.len() / 2);
        let mut audio: Vec<f64> = samples[..count * 2].iter().map(|s| f64::from(*s)).collect();
        eq.process(&mut audio);
        if !started {
            sink.start()?;
            session.set(Status::Playing);
            wake.wake();
            started = true;
        }
        sink.write(AudioPacket::Samples(audio), &mut converter)?;
        sent += count;
    }
    anyhow::ensure!(
        started || session.cancelled.load(Ordering::Relaxed),
        "No decodable preview audio found."
    );
    Ok(sent)
}

/// Exercise the production decoder without opening an output device.
#[cfg(feature = "demo")]
pub fn demo_decode(bytes: Vec<u8>, cancel_after_first_packet: bool) -> anyhow::Result<usize> {
    struct Silent {
        session: Arc<Session>,
        cancel: bool,
    }
    impl crate::sink::Sink for Silent {
        fn start(&mut self) -> crate::sink::SinkResult {
            Ok(())
        }
        fn stop(&mut self) -> crate::sink::SinkResult {
            Ok(())
        }
        fn write(
            &mut self,
            _: crate::sink::AudioPacket,
            _: &mut crate::sink::Converter,
        ) -> crate::sink::SinkResult {
            if self.cancel {
                self.session.cancel();
            }
            Ok(())
        }
    }
    let session = Session::new(LocalTrack::default(), 30000);
    let mut sink = Silent {
        session: Arc::clone(&session),
        cancel: cancel_after_first_packet,
    };
    decode(
        &session,
        crate::eq::shared(),
        &mut sink,
        bytes,
        &crate::backend::Waker::default(),
    )
}
