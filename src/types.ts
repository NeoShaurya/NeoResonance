export interface SmtcState {
  active: boolean;
  title: string;
  artist: string;
  album: string;
  app_id: string;
  playback_status: string; // "Playing" | "Paused" | "Stopped" | "Idle"
  position_ms: number;
  duration_ms: number;
  last_updated_unix_ms: number;
  album_art?: string | null;
}

export interface AudioTelemetry {
  bass_energy: number;   // 0.0 .. 1.0
  mid_energy: number;    // 0.0 .. 1.0
  waveform: number[];    // 32 points normalized -1.0 .. 1.0
  is_silent: boolean;
}

export interface WordItem {
  text: string;
  start_ms: number;
  end_ms: number;
}

export interface LyricLine {
  start_ms: number;
  end_ms: number;
  text: string;
  words: WordItem[];
}

export interface TrackLyrics {
  track_key: string;
  is_synced: boolean;
  lines: LyricLine[];
}

export interface WidgetConfig {
  x: number | null;
  y: number | null;
  locked: boolean;
  show_aura: boolean;
  show_lyrics: boolean;
}

export interface AuraPalette {
  primary: string;
  secondary: string;
  accent: string;
}
