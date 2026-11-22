import React from "react";
import { LyricsProgress } from "../hooks/useLyrics";

interface LyricsDisplayProps {
  progress: LyricsProgress;
  trackTitle: string;
  artist: string;
  visible: boolean;
}

export const LyricsDisplay: React.FC<LyricsDisplayProps> = React.memo(({
  progress,
  trackTitle,
  artist,
  visible,
}) => {
  if (!visible) return null;

  const { currentLine, upcomingLine, activeWordIndex } = progress;

  return (
    <div className="flex flex-col justify-center items-start w-full min-h-[140px] px-2 py-4 select-none">
      {currentLine ? (
        <div className="flex flex-col space-y-1.5 transition-all duration-300 ease-out w-full">
          {/* Active Focused Line (Karaoke Glow) */}
          <div className="text-[21px] md:text-[23px] font-bold tracking-tight text-white/90 leading-tight flex flex-wrap items-baseline gap-x-2 gap-y-1">
            {currentLine.words && currentLine.words.length > 0 ? (
              currentLine.words.map((word, idx) => {
                const isPast = idx < activeWordIndex;
                const isActive = idx === activeWordIndex;
                const isFuture = idx > activeWordIndex;

                let wordClass = "text-white/45 transition-all duration-150";

                if (isActive) {
                  wordClass =
                    "text-white font-extrabold scale-[1.025] origin-bottom inline-block transition-transform duration-100 drop-shadow-[0_0_18px_rgba(255,255,255,0.85)] drop-shadow-[0_0_10px_rgba(135,206,250,0.6)]";
                } else if (isPast) {
                  wordClass = "text-white/45";
                } else if (isFuture) {
                  wordClass = "text-white/60";
                }

                return (
                  <span key={idx} className={wordClass}>
                    {word.text}
                  </span>
                );
              })
            ) : (
              <span className="text-white drop-shadow-[0_0_16px_rgba(255,255,255,0.7)]">
                {currentLine.text}
              </span>
            )}
          </div>

          {/* Upcoming Preview Line */}
          {upcomingLine && (
            <div className="text-[13.5px] font-medium text-white/30 tracking-tight transition-all duration-500 ease-out truncate max-w-full">
              {upcomingLine.text}
            </div>
          )}
        </div>
      ) : upcomingLine ? (
        /* Intro before lyrics begin */
        <div className="flex flex-col space-y-1.5 transition-all duration-300 ease-out">
          <div className="text-[20px] font-bold text-white/50 tracking-tight animate-pulse">
            •••
          </div>
          <div className="text-[13.5px] font-medium text-white/35 tracking-tight">
            {upcomingLine.text}
          </div>
        </div>
      ) : (
        /* Idle / No lyrics state */
        <div className="flex flex-col space-y-1 text-white/40">
          <div className="text-[18px] font-medium tracking-tight text-white/60">
            {trackTitle && trackTitle !== "No Media Playing"
              ? trackTitle
              : "No Media Playing"}
          </div>
          <div className="text-[12px] text-white/30">
            {artist ? `Listening on Windows • ${artist}` : "Ready for audio playback"}
          </div>
        </div>
      )}
    </div>
  );
});
