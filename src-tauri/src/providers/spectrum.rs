//! Visualizador: captura lo que suena en la salida de audio por defecto (WASAPI en modo
//! loopback, no el micrófono) y lo reduce a unas cuantas barras con una FFT.
//!
//! Solo corre mientras hay una canción reproduciéndose y el usuario no lo apagó: capturar
//! audio todo el tiempo se come la batería de una laptop.

use std::f32::consts::PI;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use rustfft::num_complex::Complex;
use rustfft::{Fft, FftPlanner};
use tauri::{AppHandle, Emitter, Manager};
use windows::Win32::Media::Audio::{
    AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK,
    IAudioCaptureClient, IAudioClient, IMMDeviceEnumerator, MMDeviceEnumerator, WAVEFORMATEX,
    WAVEFORMATEXTENSIBLE, eConsole, eRender,
};
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree,
};
use windows::core::GUID;

use crate::settings::SettingsStore;

pub const SPECTRUM_EVENT: &str = "island:spectrum";
pub const BANDS: usize = 16;

const FFT_SIZE: usize = 2048;
const MIN_HZ: f32 = 50.0;
const MAX_HZ: f32 = 14_000.0;
/// Rango de dB que se mapea a 0–1. Por debajo es silencio; por encima, barra llena.
const FLOOR_DB: f32 = -70.0;
const CEIL_DB: f32 = -12.0;
/// La música tiene menos energía en agudos (~-3 dB por octava): sin compensar, las
/// barras de la derecha casi nunca se mueven.
const TILT_DB_PER_OCTAVE: f32 = 3.0;
/// Suavizado: suben rápido (se siente el golpe) y bajan lento (no parpadean).
const ATTACK: f32 = 0.65;
const RELEASE: f32 = 0.18;

const FRAME: Duration = Duration::from_millis(33);
const IDLE_POLL: Duration = Duration::from_millis(250);
/// Cada cuánto se revisa si cambió la salida de audio (p. ej. se conectaron audífonos).
const DEVICE_CHECK: Duration = Duration::from_secs(2);

// mmreg.h: dos constantes no justifican compilar dos features más del crate `windows`.
const WAVE_FORMAT_IEEE_FLOAT: u16 = 0x0003;
const WAVE_FORMAT_EXTENSIBLE: u16 = 0xFFFE;
const KSDATAFORMAT_SUBTYPE_IEEE_FLOAT: GUID =
    GUID::from_u128(0x00000003_0000_0010_8000_00aa00389b71);

pub struct Analyzer {
    fft: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    /// `2 / Σ ventana`: deja un seno de amplitud 1 en 0 dBFS.
    norm: f32,
    ring: Vec<f32>,
    write: usize,
    edges: [usize; BANDS + 1],
    tilt_db: [f32; BANDS],
    levels: [f32; BANDS],
    scratch: Vec<Complex<f32>>,
}

impl Analyzer {
    pub fn new(sample_rate: u32) -> Self {
        let window: Vec<f32> = (0..FFT_SIZE)
            .map(|i| 0.5 - 0.5 * (2.0 * PI * i as f32 / FFT_SIZE as f32).cos())
            .collect();
        let norm = 2.0 / window.iter().sum::<f32>();
        let edges = band_edges(sample_rate);
        let bin_hz = sample_rate as f32 / FFT_SIZE as f32;
        let tilt_db = std::array::from_fn(|b| {
            let center_hz = (edges[b] + edges[b + 1]) as f32 / 2.0 * bin_hz;
            TILT_DB_PER_OCTAVE * (center_hz / 1000.0).log2()
        });
        Self {
            fft: FftPlanner::new().plan_fft_forward(FFT_SIZE),
            window,
            norm,
            ring: vec![0.0; FFT_SIZE],
            write: 0,
            edges,
            tilt_db,
            levels: [0.0; BANDS],
            scratch: vec![Complex::default(); FFT_SIZE],
        }
    }

    pub fn push(&mut self, sample: f32) {
        self.ring[self.write] = sample;
        self.write = (self.write + 1) % FFT_SIZE;
    }

    /// Nivel de cada banda de 0 a 255, ya suavizado.
    pub fn bands(&mut self) -> [u8; BANDS] {
        for (i, slot) in self.scratch.iter_mut().enumerate() {
            let sample = self.ring[(self.write + i) % FFT_SIZE];
            *slot = Complex::new(sample * self.window[i], 0.0);
        }
        self.fft.process(&mut self.scratch);

        std::array::from_fn(|b| {
            let peak = self.scratch[self.edges[b]..self.edges[b + 1]]
                .iter()
                .map(|c| c.norm())
                .fold(0.0, f32::max)
                * self.norm;
            let db = 20.0 * peak.max(1e-9).log10() + self.tilt_db[b];
            let target = ((db - FLOOR_DB) / (CEIL_DB - FLOOR_DB)).clamp(0.0, 1.0);
            let level = &mut self.levels[b];
            let speed = if target > *level { ATTACK } else { RELEASE };
            *level += (target - *level) * speed;
            (*level * 255.0).round() as u8
        })
    }
}

/// Bins de la FFT donde empieza cada banda (y donde termina la última), repartidos en
/// escala logarítmica, que es como el oído percibe la frecuencia.
fn band_edges(sample_rate: u32) -> [usize; BANDS + 1] {
    let bin_hz = sample_rate as f32 / FFT_SIZE as f32;
    let nyquist_bin = FFT_SIZE / 2;
    let mut edges: [usize; BANDS + 1] = std::array::from_fn(|i| {
        let hz = MIN_HZ * (MAX_HZ / MIN_HZ).powf(i as f32 / BANDS as f32);
        ((hz / bin_hz).round() as usize).clamp(1, nyquist_bin)
    });
    // En graves varias bandas caen en el mismo bin: cada una necesita al menos uno.
    for i in 1..=BANDS {
        if edges[i] <= edges[i - 1] {
            edges[i] = edges[i - 1] + 1;
        }
    }
    edges
}

pub fn spawn(app: AppHandle, playing: Arc<AtomicBool>) -> std::io::Result<()> {
    thread::Builder::new()
        .name("spectrum".into())
        .spawn(move || {
            // SAFETY: inicializa COM en este hilo, requisito de las APIs de Core Audio.
            if unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_err() {
                return;
            }
            loop {
                if should_capture(&app, &playing) {
                    if let Err(err) = capture(&app, &playing) {
                        eprintln!("visualizador: {err}");
                        thread::sleep(Duration::from_secs(1));
                    }
                } else {
                    thread::sleep(IDLE_POLL);
                }
            }
        })?;
    Ok(())
}

fn should_capture(app: &AppHandle, playing: &AtomicBool) -> bool {
    playing.load(Ordering::Relaxed) && app.state::<SettingsStore>().get().visualizer
}

/// Captura hasta que deja de sonar música, el usuario apaga el visualizador o cambia
/// la salida de audio por defecto (en ese caso el bucle de arriba vuelve a empezar).
fn capture(app: &AppHandle, playing: &AtomicBool) -> windows::core::Result<()> {
    // SAFETY: llamadas COM documentadas; los punteros devueltos por la API se liberan
    // con `CoTaskMemFree` como pide su contrato y no se usan después.
    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
        let initial_device = device_id(&device)?;
        let client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;

        let format = client.GetMixFormat()?;
        let channels = (*format).nChannels.max(1) as usize;
        let sample_rate = (*format).nSamplesPerSec;
        let is_float = is_float_format(format);
        let bits = (*format).wBitsPerSample;
        // 100 ms de búfer en unidades de 100 ns: holgado para un ciclo de 33 ms.
        let init = client.Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            AUDCLNT_STREAMFLAGS_LOOPBACK,
            1_000_000,
            0,
            format,
            None,
        );
        CoTaskMemFree(Some(format as *const _));
        init?;
        if !is_float && bits != 16 {
            // El formato de mezcla compartido es float32 desde Vista; 16 bits es el único
            // otro caso realista. Cualquier otro se ignora en vez de dibujar basura.
            return Ok(());
        }

        let capture: IAudioCaptureClient = client.GetService()?;
        client.Start()?;
        let mut analyzer = Analyzer::new(sample_rate);
        let mut last_device_check = Instant::now();

        while should_capture(app, playing) {
            thread::sleep(FRAME);
            loop {
                let frames_available = capture.GetNextPacketSize()?;
                if frames_available == 0 {
                    break;
                }
                let mut data = std::ptr::null_mut();
                let mut frames = 0u32;
                let mut flags = 0u32;
                capture.GetBuffer(&mut data, &mut frames, &mut flags, None, None)?;
                let silent = flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0;
                let samples = frames as usize * channels;
                for frame in 0..frames as usize {
                    let mono = if silent {
                        0.0
                    } else {
                        let at = frame * channels;
                        (0..channels)
                            .map(|c| read_sample(data, at + c, is_float, samples))
                            .sum::<f32>()
                            / channels as f32
                    };
                    analyzer.push(mono);
                }
                capture.ReleaseBuffer(frames)?;
            }
            let _ = app.emit(SPECTRUM_EVENT, analyzer.bands());

            if last_device_check.elapsed() >= DEVICE_CHECK {
                last_device_check = Instant::now();
                let current = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
                if device_id(&current)? != initial_device {
                    break;
                }
            }
        }
        client.Stop()?;
        // Que las barras bajen a cero en vez de congelarse en el último cuadro.
        let _ = app.emit(SPECTRUM_EVENT, [0u8; BANDS]);
        Ok(())
    }
}

unsafe fn device_id(
    device: &windows::Win32::Media::Audio::IMMDevice,
) -> windows::core::Result<String> {
    // SAFETY: `GetId` devuelve un PWSTR que el llamador debe liberar.
    unsafe {
        let raw = device.GetId()?;
        let id = raw.to_string().unwrap_or_default();
        CoTaskMemFree(Some(raw.0 as *const _));
        Ok(id)
    }
}

unsafe fn is_float_format(format: *const WAVEFORMATEX) -> bool {
    // SAFETY: `format` viene de `GetMixFormat`; si la etiqueta es EXTENSIBLE, la
    // estructura real es `WAVEFORMATEXTENSIBLE`. Está empacada, de ahí el read_unaligned.
    unsafe {
        match (*format).wFormatTag {
            WAVE_FORMAT_IEEE_FLOAT => true,
            WAVE_FORMAT_EXTENSIBLE => {
                let ext = format as *const WAVEFORMATEXTENSIBLE;
                std::ptr::read_unaligned(std::ptr::addr_of!((*ext).SubFormat))
                    == KSDATAFORMAT_SUBTYPE_IEEE_FLOAT
            }
            _ => false,
        }
    }
}

unsafe fn read_sample(data: *const u8, index: usize, is_float: bool, len: usize) -> f32 {
    debug_assert!(index < len);
    // SAFETY: `index` está dentro del paquete que entregó `GetBuffer` (frames × canales).
    unsafe {
        if is_float {
            std::ptr::read_unaligned((data as *const f32).add(index))
        } else {
            std::ptr::read_unaligned((data as *const i16).add(index)) as f32 / i16::MAX as f32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 48_000;

    fn feed_sine(analyzer: &mut Analyzer, hz: f32, amplitude: f32) {
        for n in 0..FFT_SIZE {
            analyzer.push(amplitude * (2.0 * PI * hz * n as f32 / RATE as f32).sin());
        }
    }

    fn band_of(hz: f32) -> usize {
        let bin = (hz / (RATE as f32 / FFT_SIZE as f32)).round() as usize;
        let edges = band_edges(RATE);
        (0..BANDS)
            .find(|&b| bin >= edges[b] && bin < edges[b + 1])
            .unwrap()
    }

    #[test]
    fn edges_strictly_increase_within_nyquist() {
        for rate in [44_100, 48_000, 96_000] {
            let edges = band_edges(rate);
            assert!(edges.windows(2).all(|w| w[0] < w[1]), "{rate}: {edges:?}");
            assert!(edges[BANDS] <= FFT_SIZE / 2 + BANDS);
        }
    }

    #[test]
    fn silence_stays_at_zero() {
        let mut analyzer = Analyzer::new(RATE);
        for _ in 0..5 {
            assert_eq!(analyzer.bands(), [0; BANDS]);
        }
    }

    #[test]
    fn a_tone_lights_up_its_own_band() {
        let mut analyzer = Analyzer::new(RATE);
        feed_sine(&mut analyzer, 1_000.0, 0.5);
        let mut bands = [0; BANDS];
        for _ in 0..10 {
            bands = analyzer.bands();
        }
        let loudest = (0..BANDS).max_by_key(|&b| bands[b]).unwrap();
        assert_eq!(loudest, band_of(1_000.0), "{bands:?}");
        assert!(bands[loudest] > 200, "{bands:?}");
        assert!(bands[0] < 50 && bands[BANDS - 1] < 50, "{bands:?}");
    }

    #[test]
    fn levels_fall_slower_than_they_rise() {
        let mut analyzer = Analyzer::new(RATE);
        feed_sine(&mut analyzer, 1_000.0, 0.5);
        let band = band_of(1_000.0);
        let first = analyzer.bands()[band];
        for _ in 0..10 {
            analyzer.bands();
        }
        for _ in 0..FFT_SIZE {
            analyzer.push(0.0);
        }
        let after_silence = analyzer.bands()[band];
        assert!(first > 120, "sube rápido: {first}");
        assert!(after_silence > 150, "baja lento: {after_silence}");
    }
}
