import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { SmtcState } from "../types";

const INITIAL_STATE: SmtcState = {
  active: false,
  title: "No Media Playing",
  artist: "Idle",
  album: "",
  app_id: "",
  playback_status: "Idle",
  position_ms: 0,
  duration_ms: 0,
  last_updated_unix_ms: Date.now(),
  album_art: null,
};

export function useSmtc() {
  const [state, setState] = useState<SmtcState>(INITIAL_STATE);

  useEffect(() => {
    // Initial fetch
    invoke<SmtcState>("get_smtc_state")
      .then((data) => {
        if (data) setState(data);
      })
      .catch((err) => console.warn("Failed to get smtc state:", err));

    // Listen for updates from backend
    const unlistenPromise = listen<SmtcState>("smtc_state", (event) => {
      setState(event.payload);
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  const playPause = useCallback(async () => {
    try {
      await invoke("media_play_pause");
    } catch (e) {
      console.warn("media_play_pause failed:", e);
    }
  }, []);

  const next = useCallback(async () => {
    try {
      await invoke("media_next");
    } catch (e) {
      console.warn("media_next failed:", e);
    }
  }, []);

  const prev = useCallback(async () => {
    try {
      await invoke("media_prev");
    } catch (e) {
      console.warn("media_prev failed:", e);
    }
  }, []);

  const seek = useCallback(async (positionMs: number) => {
    try {
      await invoke("media_seek", { positionMs: Math.floor(positionMs) });
      setState((prev) => ({
        ...prev,
        position_ms: positionMs,
        last_updated_unix_ms: Date.now(),
      }));
    } catch (e) {
      console.warn("media_seek failed:", e);
    }
  }, []);

  return {
    state,
    playPause,
    next,
    prev,
    seek,
    isPlaying: state.playback_status === "Playing",
  };
}
