import { useEffect, useState, useRef } from "react";
import { Channel, invoke } from "@tauri-apps/api/core";
import { AudioTelemetry } from "../types";

const INITIAL_TELEMETRY: AudioTelemetry = {
  bass_energy: 0,
  mid_energy: 0,
  waveform: new Array(32).fill(0),
  is_silent: true,
};

export function useAudio() {
  const [telemetry, setTelemetry] = useState<AudioTelemetry>(INITIAL_TELEMETRY);
  const streamStarted = useRef(false);

  useEffect(() => {
    if (streamStarted.current) return;
    streamStarted.current = true;

    const channel = new Channel<AudioTelemetry>();
    channel.onmessage = (message) => {
      setTelemetry(message);
    };

    invoke("start_audio_stream", { channel }).catch((err) => {
      console.warn("Failed to start audio stream:", err);
    });
  }, []);

  return telemetry;
}
