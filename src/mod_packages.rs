//! Versioned, optional Wasm mods. No filesystem, network or credential imports are
//! exposed to guest code; service integration stays behind the native host API.
use crate::addons::Id;
use anyhow::{Context, Result, ensure};
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

pub const MAX_PACKAGE: usize = 8 * 1024 * 1024;
const MAX_WASM: usize = 1024 * 1024;
const MAX_COMMANDS: usize = 4096;
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CatalogEntry {
    pub id: Id,
    pub slug: String,
    pub bytes: usize,
    pub sha256: String,
}
pub fn catalog() -> Vec<CatalogEntry> {
    serde_json::from_str(include_str!("../mods/catalog.json")).expect("validated bundled catalogue")
}
pub fn slug(id: Id) -> &'static str {
    match id {
        Id::Nyan => "nyan",
        Id::WaveLink => "wavelink",
        Id::JuiceVault => "juicevault",
        Id::Oneko => "oneko",
        Id::CatJam => "catjam",
        Id::YouTube => "youtube",
        Id::Vivy => "vivy",
        Id::Backmusic => "backmusic",
        Id::TrackPeek => "trackpeek",
        Id::SoundVisualizer => "soundvisualizer",
    }
}
fn identity(id: Id) -> i32 {
    match id {
        Id::Nyan => 0,
        Id::WaveLink => 1,
        Id::JuiceVault => 2,
        Id::Oneko => 3,
        Id::CatJam => 4,
        Id::YouTube => 5,
        Id::Vivy => 6,
        Id::Backmusic => 7,
        Id::TrackPeek => 8,
        Id::SoundVisualizer => 9,
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Package {
    schema: u32,
    id: Id,
    wasm: String,
    sprites: Vec<SpriteData>,
    credits: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SpriteData {
    files: Vec<String>,
    columns: u16,
    rows: u16,
    frames: u16,
}
pub struct Sprite {
    pub files: Vec<Arc<[u8]>>,
    pub columns: u16,
    pub rows: u16,
    pub frames: u16,
}
#[derive(Clone, Debug)]
pub struct Draw {
    pub kind: i32,
    pub values: [f32; 8],
}
struct State {
    input: Vec<f32>,
    output: Vec<Draw>,
    limits: wasmi::StoreLimits,
    overflow: bool,
}
struct Instance {
    store: wasmi::Store<State>,
    render: wasmi::TypedFunc<(), ()>,
    failed: bool,
}
pub struct Guest {
    inner: Mutex<Instance>,
    pub sprites: Vec<Sprite>,
    pub capabilities: u32,
}
impl std::fmt::Debug for Guest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Guest")
            .field("capabilities", &self.capabilities)
            .finish_non_exhaustive()
    }
}
impl Guest {
    pub fn render(&self, input: Vec<f32>) -> Result<Vec<Draw>> {
        let mut vm = self
            .inner
            .lock()
            .map_err(|_| anyhow::anyhow!("Mod runtime unavailable"))?;
        ensure!(!vm.failed, "Mod stopped after a runtime error");
        vm.store.data_mut().input = input;
        vm.store.data_mut().output.clear();
        vm.store.data_mut().overflow = false;
        vm.store.set_fuel(2_000_000)?;
        let render = vm.render;
        if let Err(error) = render.call(&mut vm.store, ()) {
            vm.failed = true;
            log::warn!("An optional mod stopped after a runtime error: {error}");
            return Err(error.into());
        }
        if vm.store.data().overflow {
            vm.failed = true;
            anyhow::bail!("Mod exceeded the draw limit");
        }
        Ok(std::mem::take(&mut vm.store.data_mut().output))
    }
}
pub fn decode(id: Id, bytes: &[u8]) -> Result<Arc<Guest>> {
    let expected = catalog()
        .into_iter()
        .find(|v| v.id == id)
        .context("Unknown add-on")?;
    ensure!(
        bytes.len() <= MAX_PACKAGE && bytes.len() == expected.bytes,
        "Mod download size mismatch"
    );
    ensure!(
        format!("{:x}", Sha256::digest(bytes)) == expected.sha256,
        "Mod download checksum mismatch"
    );
    let package: Package = serde_json::from_slice(bytes).context("Invalid mod package")?;
    ensure!(
        package.schema == 1 && package.id == id,
        "Incompatible mod package"
    );
    ensure!(
        package.credits.len() <= 128_000 && !package.credits.is_empty(),
        "Missing mod attribution"
    );
    ensure!(package.wasm.len() <= MAX_WASM * 2, "Mod code is too large");
    let wasm = base64::engine::general_purpose::STANDARD.decode(package.wasm)?;
    ensure!(wasm.len() <= MAX_WASM, "Mod code is too large");
    ensure!(package.sprites.len() <= 3, "Too many sprite sets");
    let mut sprites = Vec::new();
    for set in package.sprites {
        ensure!(
            !set.files.is_empty()
                && set.files.len() <= 6
                && set.columns > 0
                && set.rows > 0
                && set.frames > 0
                && u32::from(set.frames)
                    <= u32::from(set.columns) * u32::from(set.rows) * set.files.len() as u32,
            "Invalid sprite layout"
        );
        let mut files = Vec::new();
        for encoded in set.files {
            let bytes = base64::engine::general_purpose::STANDARD.decode(encoded)?;
            ensure!(bytes.len() <= 4_000_000, "Sprite is too large");
            let reader = image::ImageReader::with_format(
                std::io::Cursor::new(&bytes),
                image::ImageFormat::Png,
            );
            let (width, height) = reader.into_dimensions()?;
            ensure!(
                width > 0
                    && height > 0
                    && width <= 4096
                    && height <= 4096
                    && width % u32::from(set.columns) == 0
                    && height % u32::from(set.rows) == 0,
                "Invalid sprite dimensions"
            );
            files.push(Arc::<[u8]>::from(bytes));
        }
        sprites.push(Sprite {
            files,
            columns: set.columns,
            rows: set.rows,
            frames: set.frames,
        });
    }
    let mut config = wasmi::Config::default();
    config.consume_fuel(true);
    let engine = wasmi::Engine::new(&config);
    let module = wasmi::Module::new(&engine, &wasm)?;
    let limits = wasmi::StoreLimitsBuilder::new()
        .memory_size(16 * 1024 * 1024)
        .instances(1)
        .tables(4)
        .table_elements(4096)
        .build();
    let mut store = wasmi::Store::new(
        &engine,
        State {
            input: Vec::new(),
            output: Vec::new(),
            limits,
            overflow: false,
        },
    );
    store.limiter(|s| &mut s.limits);
    store.set_fuel(2_000_000)?;
    let mut linker = wasmi::Linker::<State>::new(&engine);
    linker.func_wrap(
        "vibefy",
        "number",
        |caller: wasmi::Caller<'_, State>, index: i32| -> f32 {
            caller
                .data()
                .input
                .get(index as usize)
                .copied()
                .filter(|v| v.is_finite())
                .unwrap_or(0.)
        },
    )?;
    linker.func_wrap(
        "vibefy",
        "emit",
        |mut caller: wasmi::Caller<'_, State>,
         kind: i32,
         a: f32,
         b: f32,
         c: f32,
         d: f32,
         e: f32,
         f: f32,
         g: f32,
         h: f32| {
            let values = [a, b, c, d, e, f, g, h];
            let state = caller.data_mut();
            if state.output.len() >= MAX_COMMANDS
                || !(0..=8).contains(&kind)
                || values
                    .iter()
                    .any(|v| !v.is_finite() || v.abs() > 1_000_000.)
            {
                state.overflow = true;
                return;
            }
            state.output.push(Draw { kind, values });
        },
    )?;
    let instance = linker.instantiate_and_start(&mut store, &module)?;
    ensure!(
        instance
            .get_typed_func::<(), i32>(&store, "abi")?
            .call(&mut store, ())?
            == 1,
        "Unsupported mod API"
    );
    ensure!(
        instance
            .get_typed_func::<(), i32>(&store, "identity")?
            .call(&mut store, ())?
            == identity(id),
        "Mod identity mismatch"
    );
    let capabilities = instance
        .get_typed_func::<(), i32>(&store, "capabilities")?
        .call(&mut store, ())? as u32;
    let allowed = match id {
        Id::WaveLink => 1,
        Id::JuiceVault => 2,
        Id::YouTube => 4,
        _ => 0,
    };
    ensure!(capabilities == allowed, "Unexpected mod permissions");
    let render = instance.get_typed_func::<(), ()>(&store, "render")?;
    Ok(Arc::new(Guest {
        inner: Mutex::new(Instance {
            store,
            render,
            failed: false,
        }),
        sprites,
        capabilities,
    }))
}
pub fn package_path(root: &std::path::Path, id: Id) -> std::path::PathBuf {
    root.join("mods").join(format!("{}-v1.vmod", slug(id)))
}
pub fn load(root: &std::path::Path, id: Id) -> Result<Arc<Guest>> {
    use std::io::Read;
    let file =
        std::fs::File::open(package_path(root, id)).context("Install this add-on from Mods")?;
    let mut bytes = Vec::new();
    file.take(MAX_PACKAGE as u64 + 1).read_to_end(&mut bytes)?;
    decode(id, &bytes)
}
pub async fn install(root: std::path::PathBuf, id: Id) -> Result<()> {
    let expected = catalog()
        .into_iter()
        .find(|v| v.id == id)
        .context("Unknown mod")?;
    let url = format!(
        "https://github.com/Spinti1/Vibefy/releases/download/mods-v1/{}.vmod",
        slug(id)
    );
    let client = reqwest::Client::builder()
        .no_proxy()
        .https_only(true)
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(90))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            let url = attempt.url();
            if attempt.previous().len() > 5
                || url.scheme() != "https"
                || !matches!(
                    url.host_str(),
                    Some(
                        "github.com"
                            | "release-assets.githubusercontent.com"
                            | "objects.githubusercontent.com"
                    )
                )
                || !url.username().is_empty()
                || url.password().is_some()
                || url.port().is_some()
            {
                attempt.error("Untrusted mod download redirect")
            } else {
                attempt.follow()
            }
        }))
        .build()?;
    let mut response = client.get(url).send().await?.error_for_status()?;
    if let Some(length) = response.content_length() {
        ensure!(
            length == expected.bytes as u64,
            "Mod download size mismatch"
        );
    }
    let mut bytes = Vec::with_capacity(expected.bytes);
    while let Some(chunk) = response.chunk().await? {
        ensure!(
            bytes.len() + chunk.len() <= expected.bytes.min(MAX_PACKAGE),
            "Mod download exceeded its declared size"
        );
        bytes.extend_from_slice(&chunk);
    }
    tokio::task::spawn_blocking(move || -> Result<()> {
        decode(id, &bytes)?;
        let path = package_path(&root, id);
        let parent = path.parent().context("Invalid mod directory")?;
        std::fs::create_dir_all(parent)?;
        ensure!(
            !std::fs::symlink_metadata(parent)?.file_type().is_symlink(),
            "Mod directory is a symbolic link"
        );
        let temp = path.with_extension(format!("{}.tmp", rand::random::<u64>()));
        use std::io::Write;
        let result = (|| -> Result<()> {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            drop(file);
            // Replace only a corrupt package; a verified installed version is reusable.
            if path.exists() {
                if load(&root, id).is_ok() {
                    std::fs::remove_file(&temp)?;
                } else {
                    let backup = path.with_extension("corrupt");
                    if backup.exists() {
                        std::fs::remove_file(&backup)?;
                    }
                    std::fs::rename(&path, &backup)?;
                    if let Err(error) = std::fs::rename(&temp, &path) {
                        let _ = std::fs::rename(&backup, &path);
                        return Err(error.into());
                    }
                    let _ = std::fs::remove_file(backup);
                }
            } else {
                std::fs::rename(&temp, &path)?;
            }
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temp);
        }
        result
    })
    .await??;
    Ok(())
}
pub fn load_all(root: &std::path::Path, ids: Vec<Id>) -> BTreeMap<Id, Result<Arc<Guest>, String>> {
    ids.into_iter()
        .map(|id| (id, load(root, id).map_err(|e| e.to_string())))
        .collect()
}

#[cfg(test)]
mod release_update_tests {
    use super::*;
    fn input(style: u8) -> Vec<f32> {
        let mut v = vec![0.; 3584];
        v[2] = 1200.;
        v[3] = 760.;
        v[4] = 12.;
        v[6] = 1.;
        v[12] = 1.;
        v[13] = style as f32;
        v[18] = 128.;
        v[19] = 24.;
        v[26] = 30.;
        for (i, b) in v[64..192].iter_mut().enumerate() {
            *b = (i as f32 * 0.1).sin().abs();
        }
        for (i, w) in v[3328..].iter_mut().enumerate() {
            *w = (i as f32 * 0.06).sin() * 0.5;
        }
        v
    }
    fn package(id: Id) -> Arc<Guest> {
        decode(
            id,
            &std::fs::read(format!(
                "{}/mods/packages/{}.vmod",
                env!("CARGO_MANIFEST_DIR"),
                slug(id)
            ))
            .unwrap(),
        )
        .unwrap()
    }
    #[test]
    fn all_soundvisualizer_modes_and_existing_visuals_stay_within_the_sandbox_budget() {
        for id in [Id::SoundVisualizer, Id::Vivy, Id::Backmusic] {
            let guest = package(id);
            for style in 0..if id == Id::SoundVisualizer { 12 } else { 2 } {
                for (width, height) in [(1200., 760.), (320., 240.)] {
                    let mut v = input(style);
                    v[2] = width;
                    v[3] = height;
                    let commands = guest
                        .render(v)
                        .unwrap_or_else(|e| panic!("{id:?} style {style}: {e}"));
                    assert!(!commands.is_empty() && commands.len() <= MAX_COMMANDS);
                    assert!(commands.iter().flat_map(|c| c.values).all(f32::is_finite));
                }
            }
        }
    }
    #[test]
    fn waveform_modes_use_pcm_and_paused_modes_do_not_request_animation_frames() {
        for style in [3, 4] {
            let guest = package(Id::SoundVisualizer);
            let mut quiet = input(style);
            quiet[3328..].fill(0.);
            let a = guest.render(quiet.clone()).unwrap();
            let mut signal = quiet.clone();
            signal[3328..].fill(0.5);
            let b = guest.render(signal).unwrap();
            assert!(
                a.iter()
                    .zip(&b)
                    .any(|(a, b)| a.kind == 1 && a.values != b.values)
            );
            quiet[6] = 0.;
            assert!(!guest.render(quiet).unwrap().iter().any(|c| c.kind == 8));
        }
    }
}
