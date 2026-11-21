use rustfft::{num_complex::Complex, FftPlanner};
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::ipc::Channel;
use windows::Win32::Media::Audio::{
    eConsole, eRender, IAudioCaptureClient, IAudioClient, IMMDeviceEnumerator, MMDeviceEnumerator,
    AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK,
    WAVEFORMATEXTENSIBLE,
};

const WAVE_FORMAT_IEEE_FLOAT: u16 = 3;
const WAVE_FORMAT_EXTENSIBLE: u16 = 0xFFFE;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED,
};

const FFT_SIZE: usize = 1024;
const WAVE_POINTS: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioTelemetry {
    pub bass_energy: f32,       // 0.0 .. 1.0 (sub-bass beat reactivity)
    pub mid_energy: f32,        // 0.0 .. 1.0 (mid harmonic warmth)
    pub waveform: Vec<f32>,     // 32 points normalized -1.0 .. 1.0 for Bezier wave
    pub is_silent: bool,
}

static AUDIO_RUNNING: AtomicBool = AtomicBool::new(false);

#[tauri::command]
pub fn start_audio_stream(channel: Channel<AudioTelemetry>) -> Result<(), String> {
    if AUDIO_RUNNING.swap(true, Ordering::SeqCst) {
        // Stream already running
        return Ok(());
    }

    std::thread::Builder::new()
        .name("wasapi-loopback-thread".into())
        .spawn(move || {
            run_wasapi_capture(channel);
            AUDIO_RUNNING.store(false, Ordering::SeqCst);
        })
        .map_err(|e| e.to_string())?;

    Ok(())
}

fn run_wasapi_capture(channel: Channel<AudioTelemetry>) {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }

    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(FFT_SIZE);

    // Precalculate Hann window
    let mut hann = vec![0.0f32; FFT_SIZE];
    for i in 0..FFT_SIZE {
        hann[i] = 0.5 * (1.0 - (2.0 * PI * i as f32 / FFT_SIZE as f32).cos());
    }

    let mut ring_buffer = vec![0.0f32; FFT_SIZE];
    let mut ring_idx = 0;

    let mut smooth_bass = 0.0f32;
    let mut smooth_mid = 0.0f32;
    let mut smooth_wave = vec![0.0f32; WAVE_POINTS];
    let mut silence_counter = 0;

    'outer: loop {
        let mut _audio_client: Option<IAudioClient> = None;
        let mut capture_client: Option<IAudioCaptureClient> = None;
        let mut sample_rate = 48000u32;
        let mut channels = 2u16;
        let mut is_float = true;

        unsafe {
            let enumerator_res: Result<IMMDeviceEnumerator, _> =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL);
            if let Ok(enumerator) = enumerator_res {
                if let Ok(device) = enumerator.GetDefaultAudioEndpoint(eRender, eConsole) {
                    if let Ok(client) = device.Activate::<IAudioClient>(CLSCTX_ALL, None) {
                        if let Ok(format_ptr) = client.GetMixFormat() {
                            let fmt = &*format_ptr;
                            sample_rate = fmt.nSamplesPerSec;
                            channels = fmt.nChannels;

                            if fmt.wFormatTag == WAVE_FORMAT_EXTENSIBLE as u16 {
                                let ext = &*(format_ptr as *const WAVEFORMATEXTENSIBLE);
                                let sub_format = std::ptr::addr_of!(ext.SubFormat).read_unaligned();
                                is_float = sub_format == windows::core::GUID::from_u128(0x00000003_0000_0010_8000_00aa00389b71);
                            } else {
                                is_float = fmt.wFormatTag == WAVE_FORMAT_IEEE_FLOAT as u16;
                            }

                            let init_res = client.Initialize(
                                AUDCLNT_SHAREMODE_SHARED,
                                AUDCLNT_STREAMFLAGS_LOOPBACK,
                                10_000_000,
                                0,
                                format_ptr,
                                None,
                            );

                            if init_res.is_ok() {
                                if let Ok(cap) = client.GetService::<IAudioCaptureClient>() {
                                    let _ = client.Start();
                                    _audio_client = Some(client);
                                    capture_client = Some(cap);
                                }
                            }
                        }
                    }
                }
            }
        }

        if capture_client.is_none() {
            // Default audio device not available, emit silence and sleep
            let zero_wave = vec![0.0f32; WAVE_POINTS];
            let _ = channel.send(AudioTelemetry {
                bass_energy: 0.0,
                mid_energy: 0.0,
                waveform: zero_wave,
                is_silent: true,
            });
            std::thread::sleep(Duration::from_millis(500));
            continue;
        }

        let capture = capture_client.unwrap();

        // Capture loop
        loop {
            let packet_size = match unsafe { capture.GetNextPacketSize() } {
                Ok(size) => size,
                Err(_) => {
                    // Audio device likely changed or disconnected
                    std::thread::sleep(Duration::from_millis(100));
                    break;
                }
            };

            if packet_size == 0 {
                // Throttle when silent/idle (~60fps wait)
                std::thread::sleep(Duration::from_millis(16));
            } else {
                let mut buffer_ptr = std::ptr::null_mut();
                let mut num_frames = 0u32;
                let mut flags = 0u32;

                let get_buf_res = unsafe {
                    capture.GetBuffer(
                        &mut buffer_ptr,
                        &mut num_frames,
                        &mut flags,
                        None,
                        None,
                    )
                };

                if get_buf_res.is_ok() && num_frames > 0 && !buffer_ptr.is_null() {
                    let is_silent_flag = (flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32) != 0;

                    if !is_silent_flag {
                        let ch_count = channels as usize;
                        let frame_count = num_frames as usize;

                        if is_float {
                            let slice = unsafe {
                                std::slice::from_raw_parts(buffer_ptr as *const f32, frame_count * ch_count)
                            };
                            for f in 0..frame_count {
                                let mut sum = 0.0f32;
                                for c in 0..ch_count {
                                    sum += slice[f * ch_count + c];
                                }
                                ring_buffer[ring_idx] = sum / (ch_count as f32);
                                ring_idx = (ring_idx + 1) % FFT_SIZE;
                            }
                        } else {
                            // 16-bit PCM
                            let slice = unsafe {
                                std::slice::from_raw_parts(buffer_ptr as *const i16, frame_count * ch_count)
                            };
                            for f in 0..frame_count {
                                let mut sum = 0.0f32;
                                for c in 0..ch_count {
                                    sum += (slice[f * ch_count + c] as f32) / 32768.0;
                                }
                                ring_buffer[ring_idx] = sum / (ch_count as f32);
                                ring_idx = (ring_idx + 1) % FFT_SIZE;
                            }
                        }
                    }

                    unsafe {
                        let _ = capture.ReleaseBuffer(num_frames);
                    }
                }
            }

            // Perform FFT and calculate telemetry
            let mut fft_buffer: Vec<Complex<f32>> = (0..FFT_SIZE)
                .map(|i| {
                    let sample = ring_buffer[(ring_idx + i) % FFT_SIZE];
                    Complex {
                        re: sample * hann[i],
                        im: 0.0,
                    }
                })
                .collect();

            fft.process(&mut fft_buffer);

            // Compute magnitudes
            let bin_hz = sample_rate as f32 / FFT_SIZE as f32;
            let mut raw_bass = 0.0f32;
            let mut bass_count = 0;
            let mut raw_mid = 0.0f32;
            let mut mid_count = 0;
            let mut rms_sum = 0.0f32;

            for i in 1..(FFT_SIZE / 2) {
                let freq = i as f32 * bin_hz;
                let mag = (fft_buffer[i].re.powi(2) + fft_buffer[i].im.powi(2)).sqrt() / (FFT_SIZE as f32 / 2.0);
                rms_sum += mag * mag;

                if freq >= 20.0 && freq <= 180.0 {
                    raw_bass += mag;
                    bass_count += 1;
                } else if freq > 180.0 && freq <= 2500.0 {
                    raw_mid += mag;
                    mid_count += 1;
                }
            }

            if bass_count > 0 {
                raw_bass /= bass_count as f32;
            }
            if mid_count > 0 {
                raw_mid /= mid_count as f32;
            }

            let total_rms = (rms_sum / (FFT_SIZE as f32 / 2.0)).sqrt();
            let is_silent = total_rms < 0.001;

            if is_silent {
                silence_counter += 1;
            } else {
                silence_counter = 0;
            }

            // Bass & Mid normalization with fast attack, smooth decay
            let target_bass = (raw_bass * 14.0).clamp(0.0, 1.0);
            let target_mid = (raw_mid * 18.0).clamp(0.0, 1.0);

            if target_bass > smooth_bass {
                smooth_bass = smooth_bass * 0.4 + target_bass * 0.6; // Quick hit
            } else {
                smooth_bass = smooth_bass * 0.88 + target_bass * 0.12; // Natural decay
            }

            if target_mid > smooth_mid {
                smooth_mid = smooth_mid * 0.5 + target_mid * 0.5;
            } else {
                smooth_mid = smooth_mid * 0.90 + target_mid * 0.10;
            }

            // 32-point physics string waveform
            let step = FFT_SIZE / WAVE_POINTS;
            for p in 0..WAVE_POINTS {
                let sample = ring_buffer[(ring_idx + p * step) % FFT_SIZE];
                let target_val = if silence_counter > 15 {
                    0.0
                } else {
                    sample.clamp(-1.0, 1.0)
                };
                // Damped spring physics smoothing
                smooth_wave[p] = smooth_wave[p] * 0.75 + target_val * 0.25;
            }

            // Emit telemetry
            let telemetry = AudioTelemetry {
                bass_energy: if silence_counter > 20 { 0.0 } else { smooth_bass },
                mid_energy: if silence_counter > 20 { 0.0 } else { smooth_mid },
                waveform: smooth_wave.clone(),
                is_silent: silence_counter > 20,
            };

            if channel.send(telemetry).is_err() {
                // Channel disconnected by frontend
                break 'outer;
            }

            // If prolonged silence, throttle loop to 50ms to save CPU
            if silence_counter > 40 {
                std::thread::sleep(Duration::from_millis(50));
            } else {
                std::thread::sleep(Duration::from_millis(16)); // ~60 FPS
            }
        }
    }
}
