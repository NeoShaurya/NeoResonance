import React, { useState, useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { useSmtc } from "./hooks/useSmtc";
import { useAudio } from "./hooks/useAudio";
import { useLyrics } from "./hooks/useLyrics";
import { useConfig } from "./hooks/useConfig";
import { AuraBackplate } from "./components/AuraBackplate";
import { MediaHeader } from "./components/MediaHeader";
import { LyricsDisplay } from "./components/LyricsDisplay";
import { StringBaselineWave } from "./components/StringBaselineWave";
import { WidgetFooter } from "./components/WidgetFooter";
import { extractPaletteFromImage } from "./utils/colorExtractor";
import { AuraPalette } from "./types";

export default function App() {
  const { state: smtc, isPlaying, playPause, next, prev, seek } = useSmtc();
  const audio = useAudio();
  const lyricsProgress = useLyrics(smtc);
  const { config, toggleLock } = useConfig();

  const [palette, setPalette] = useState<AuraPalette>({
    primary: "rgba(139, 92, 246, 0.65)",
    secondary: "rgba(14, 165, 233, 0.65)",
    accent: "rgba(236, 72, 153, 0.45)",
  });

  // Extract vibrant colors from album art
  useEffect(() => {
    extractPaletteFromImage(smtc.album_art, (newPalette) => {
      setPalette(newPalette);
    });
  }, [smtc.album_art]);

  // Handle drag window when unlocked
  const handleMouseDown = async (e: React.MouseEvent) => {
    // Only allow dragging if unlocked and not clicking interactive controls
    if (config.locked) return;
    const target = e.target as HTMLElement;
    if (
      target.closest("button") ||
      target.closest("input") ||
      target.closest(".cursor-pointer")
    ) {
      return;
    }

    try {
      // Direct native Rust dragging command
      await invoke("start_window_drag");
    } catch {
      try {
        const appWindow = getCurrentWindow();
        await appWindow.startDragging();
      } catch (err) {
        console.warn("Failed to drag window:", err);
      }
    }
  };

  // Right click toggles lock state
  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    toggleLock();
  };

  return (
    <div
      onContextMenu={handleContextMenu}
      className="w-screen h-screen flex items-center justify-center p-2 select-none overflow-hidden bg-transparent"
    >
      {/* The "Glass Ribbon" Capsule */}
      <div
        data-tauri-drag-region={!config.locked ? "" : undefined}
        onMouseDown={handleMouseDown}
        className={`relative w-full max-w-[690px] h-[280px] rounded-[24px] px-5 py-3.5 flex flex-col justify-between overflow-hidden transition-all duration-300 ${
          config.locked
            ? "border border-white/[0.13] shadow-[0_24px_50px_rgba(0,0,0,0.65),_inset_0_1px_0_0_rgba(255,255,255,0.18)]"
            : "border-2 border-dashed border-cyan-400/60 shadow-[0_0_35px_rgba(6,182,212,0.35)] cursor-move"
        }`}
        style={{
          background: "rgba(15, 15, 20, 0.42)",
          backdropFilter: "blur(28px) saturate(140%)",
          WebkitBackdropFilter: "blur(28px) saturate(140%)",
        }}
      >
        {/* Audio-Reactive Ambient Aura Backplate */}
        <AuraBackplate
          bassEnergy={audio.bass_energy}
          midEnergy={audio.mid_energy}
          palette={palette}
          visible={config.show_aura}
        />

        {/* Media Identity Hub (Top Row) */}
        <MediaHeader
          smtc={smtc}
          isPlaying={isPlaying}
          onPlayPause={playPause}
          onNext={next}
          onPrev={prev}
          onSeek={seek}
          currentTimeMs={lyricsProgress.currentTimeMs}
        />

        {/* Apple Music-Style Synced Lyrics (Center Stage) */}
        <LyricsDisplay
          progress={lyricsProgress}
          trackTitle={smtc.title}
          artist={smtc.artist}
          visible={config.show_lyrics}
        />

        {/* Bottom Section: The 1px Bezier String Wave & Widget Footer */}
        <div className="flex flex-col w-full space-y-0.5">
          <StringBaselineWave
            waveform={audio.waveform}
            isSilent={audio.is_silent || !isPlaying}
            visible={config.show_aura}
          />
          <WidgetFooter locked={config.locked} onToggleLock={toggleLock} />
        </div>
      </div>
    </div>
  );
}
