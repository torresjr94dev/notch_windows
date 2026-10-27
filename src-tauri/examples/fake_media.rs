//! Fixture de desarrollo: una "canción" en silencio registrada en los controles de medios
//! de Windows, para probar Now Playing (sesión, carátula, glow y controles) sin poner
//! música real.
//!
//! ```text
//! cargo run --manifest-path src-tauri/Cargo.toml --example fake_media
//! ```
//!
//! La isla debe mostrar "Canción de prueba"; sus botones controlan este reproductor.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::{env, fs, thread, time::Duration};

use windows::Media::Core::MediaSource;
use windows::Media::MediaPlaybackType;
use windows::Media::Playback::{MediaPlaybackItem, MediaPlayer};
use windows::Storage::StorageFile;
use windows::Storage::Streams::RandomAccessStreamReference;
use windows::core::HSTRING;

const SAMPLE_RATE: u32 = 8_000;

fn main() -> Result<(), Box<dyn Error>> {
    let dir = env::temp_dir().join("notch-fake-media");
    fs::create_dir_all(&dir)?;
    let wav = dir.join("silencio.wav");
    fs::write(&wav, silent_wav(90))?;
    // `StorageFile` rechaza rutas con `/`: cada segmento por separado.
    let cover = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("icons")
        .join("128x128.png");

    let item =
        MediaPlaybackItem::Create(&MediaSource::CreateFromStorageFile(&storage_file(&wav)?)?)?;
    // MediaPlayer se registra solo en los controles de medios del sistema y publica los
    // metadatos del *item*: los del `DisplayUpdater` del SMTC los sobrescribe.
    let display = item.GetDisplayProperties()?;
    display.SetType(MediaPlaybackType::Music)?;
    let music = display.MusicProperties()?;
    music.SetTitle(&HSTRING::from("Canción de prueba"))?;
    music.SetArtist(&HSTRING::from("Notch Windows"))?;
    music.SetAlbumTitle(&HSTRING::from("Fixtures"))?;
    display.SetThumbnail(&RandomAccessStreamReference::CreateFromFile(
        &storage_file(&cover)?,
    )?)?;
    item.ApplyDisplayProperties(&display)?;

    let player = MediaPlayer::new()?;
    player.SetSource(&item)?;
    player.SetIsLoopingEnabled(true)?;
    player.Play()?;
    println!("Reproduciendo en silencio. Ctrl+C para salir.");
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}

fn storage_file(path: &Path) -> windows::core::Result<StorageFile> {
    StorageFile::GetFileFromPathAsync(&HSTRING::from(path.as_os_str()))?.join()
}

/// WAV PCM de 16 bits, mono, todo en cero.
fn silent_wav(seconds: u32) -> Vec<u8> {
    let data_len = SAMPLE_RATE * seconds * 2;
    let mut wav = Vec::with_capacity(44 + data_len as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes()); // tamaño del bloque fmt
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&1u16.to_le_bytes()); // mono
    wav.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    wav.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes()); // bytes por segundo
    wav.extend_from_slice(&2u16.to_le_bytes()); // bytes por muestra
    wav.extend_from_slice(&16u16.to_le_bytes()); // bits por muestra
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    wav.resize(44 + data_len as usize, 0);
    wav
}
