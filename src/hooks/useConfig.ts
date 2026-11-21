import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { WidgetConfig } from "../types";

const INITIAL_CONFIG: WidgetConfig = {
  x: null,
  y: null,
  locked: true,
  show_aura: true,
  show_lyrics: true,
};

export function useConfig() {
  const [config, setConfig] = useState<WidgetConfig>(INITIAL_CONFIG);

  useEffect(() => {
    invoke<WidgetConfig>("get_widget_config")
      .then((cfg) => {
        if (cfg) setConfig(cfg);
      })
      .catch((e) => console.warn("Failed to get widget config:", e));

    const unlistenPromise = listen<WidgetConfig>("config_changed", (event) => {
      setConfig(event.payload);
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  const toggleLock = useCallback(async () => {
    try {
      const newLocked = await invoke<boolean>("toggle_widget_locked");
      setConfig((prev) => ({ ...prev, locked: newLocked }));
    } catch (e) {
      console.warn("toggle_widget_locked failed:", e);
    }
  }, []);

  const toggleAura = useCallback(async () => {
    try {
      const newShow = await invoke<boolean>("toggle_widget_aura");
      setConfig((prev) => ({ ...prev, show_aura: newShow }));
    } catch (e) {
      console.warn("toggle_widget_aura failed:", e);
    }
  }, []);

  const toggleLyrics = useCallback(async () => {
    try {
      const newShow = await invoke<boolean>("toggle_widget_lyrics");
      setConfig((prev) => ({ ...prev, show_lyrics: newShow }));
    } catch (e) {
      console.warn("toggle_widget_lyrics failed:", e);
    }
  }, []);

  const savePosition = useCallback(async (x: number, y: number) => {
    try {
      await invoke("save_widget_position", { x: Math.floor(x), y: Math.floor(y) });
      setConfig((prev) => ({ ...prev, x, y }));
    } catch (e) {
      console.warn("save_widget_position failed:", e);
    }
  }, []);

  return {
    config,
    toggleLock,
    toggleAura,
    toggleLyrics,
    savePosition,
  };
}
