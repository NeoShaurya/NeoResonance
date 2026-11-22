import React, { useRef } from "react";
import { Play, Pause, SkipBack, SkipForward, Disc } from "lucide-react";
import { SmtcState } from "../types";

interface MediaHeaderProps {
  smtc: SmtcState;
  isPlaying: boolean;
  onPlayPause: () => void;
  onNext: () => void;
  onPrev: () => void;
  onSeek: (positionMs: number) => void;
  currentTimeMs: number;
}

function formatTime(ms: number): string {
  if (!ms || ms < 0 || isNaN(ms)) return "0:00";
  const totalSeconds = Math.floor(ms / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes}:${seconds.toString().padStart(2, "0")}`;
}

export const MediaHeader: React.FC<MediaHeaderProps> = React.memo(({
  smtc,
  isPlaying,
  onPlayPause,
  onNext,
  onPrev,
  onSeek,
  currentTimeMs,
}) => {
  const progressBarRef = useRef<HTMLDivElement>(null);

  const durationMs = smtc.duration_ms || 1;
  const progressRatio = Math.min(Math.max(currentTimeMs / durationMs, 0), 1);

  const handleSeekClick = (e: React.MouseEvent<HTMLDivElement>) => {
    if (!progressBarRef.current || durationMs <= 1) return;
    const rect = progressBarRef.current.getBoundingClientRect();
    const clickX = e.clientX - rect.left;
    const ratio = Math.min(Math.max(clickX / rect.width, 0), 1);
    onSeek(ratio * durationMs);
  };

  return (
    <div className="flex items-start justify-between w-full select-none pt-0 px-0.5">
      {/* Media Identity Hub (Left) */}
      <div className="flex items-center space-x-2.5 max-w-[55%]">
        {/* Album Artwork */}
        <div className="relative w-10 h-10 rounded-[10px] overflow-hidden flex-shrink-0 shadow-[0_4px_12px_rgba(0,0,0,0.5)] border border-white/15 bg-black/40 group">
          {smtc.album_art ? (
            <img
              src={smtc.album_art}
              alt={smtc.title || "Album Art"}
              className="w-full h-full object-cover transition-transform duration-500 group-hover:scale-105"
            />
          ) : (
            <div className="w-full h-full flex items-center justify-center bg-gradient-to-br from-purple-900/60 to-blue-950/60">
              <Disc className="w-4.5 h-4.5 text-white/50 animate-[spin_6s_linear_infinite]" />
            </div>
          )}
        </div>

        {/* Text Stack: Title & Artist */}
        <div className="flex flex-col min-w-0 pr-2">
          <div className="overflow-hidden whitespace-nowrap">
            <span
              className="text-[12.5px] font-semibold text-white tracking-tight inline-block truncate max-w-[195px] drop-shadow-sm"
              title={smtc.title}
            >
              {smtc.title || "No Media Playing"}
            </span>
          </div>
          <span
            className="text-[10.5px] font-medium text-purple-200/70 truncate max-w-[195px] tracking-tight -mt-0.5"
            title={smtc.artist || smtc.album}
          >
            {smtc.artist || (smtc.active ? "Unknown Artist" : "Idle")}
          </span>
        </div>
      </div>

      {/* Control Pod & Seeker (Right) */}
      <div className="flex flex-col items-end space-y-1 pt-0">
        {/* Micro Playback Controls */}
        <div className="flex items-center space-x-2 pr-0.5">
          <button
            onClick={onPrev}
            className="p-1 text-white/65 hover:text-white transition-colors duration-150 active:scale-90"
            title="Previous"
          >
            <SkipBack className="w-3.5 h-3.5 fill-current" />
          </button>

          <button
            onClick={onPlayPause}
            className="w-7 h-7 rounded-full flex items-center justify-center bg-white/15 hover:bg-white/25 border border-white/30 text-white shadow-[0_0_10px_rgba(255,255,255,0.35)] transition-all duration-200 active:scale-90"
            title={isPlaying ? "Pause" : "Play"}
          >
            {isPlaying ? (
              <Pause className="w-3 h-3 fill-current" />
            ) : (
              <Play className="w-3 h-3 fill-current ml-0.5" />
            )}
          </button>

          <button
            onClick={onNext}
            className="p-1 text-white/65 hover:text-white transition-colors duration-150 active:scale-90"
            title="Next"
          >
            <SkipForward className="w-3.5 h-3.5 fill-current" />
          </button>
        </div>

        {/* Ultra-Thin 2px Seeker Bar with Timestamps */}
        <div className="flex flex-col items-end w-36 space-y-0.5">
          <div
            ref={progressBarRef}
            onClick={handleSeekClick}
            className="w-full h-3 flex items-center cursor-pointer group py-1"
          >
            <div className="w-full h-[2px] group-hover:h-[3.5px] bg-white/20 rounded-full overflow-hidden transition-all duration-150 relative">
              <div
                className="h-full bg-white rounded-full transition-all duration-75 relative shadow-[0_0_8px_rgba(255,255,255,0.8)]"
                style={{ width: `${progressRatio * 100}%` }}
              />
            </div>
          </div>

          {/* Sub-label Timestamps: Elapsed & Remaining */}
          <div className="flex items-center justify-between w-full text-[9.5px] font-mono text-white/55 tracking-tight px-0.5 -mt-1">
            <span>{formatTime(currentTimeMs)}</span>
            <span>{formatTime(durationMs)}</span>
          </div>
        </div>
      </div>
    </div>
  );
});
