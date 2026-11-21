import { useEffect, useState, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { LyricLine, TrackLyrics, SmtcState } from "../types";

export interface LyricsProgress {
  currentLine: LyricLine | null;
  upcomingLine: LyricLine | null;
  activeWordIndex: number; // Index of word being sung in currentLine
  interpolatedWordFraction: number; // 0.0 .. 1.0 progress through current word
  isSynced: boolean;
  hasLyrics: boolean;
  currentTimeMs: number;
}

export function useLyrics(smtc: SmtcState) {
  const [lyrics, setLyrics] = useState<TrackLyrics | null>(null);
  const [progress, setProgress] = useState<LyricsProgress>({
    currentLine: null,
    upcomingLine: null,
    activeWordIndex: -1,
    interpolatedWordFraction: 0,
    isSynced: false,
    hasLyrics: false,
    currentTimeMs: 0,
  });

  const lastTrackKey = useRef("");

  // Fetch lyrics when track changes
  useEffect(() => {
    if (!smtc.title || smtc.title === "No Media Playing") {
      setLyrics(null);
      lastTrackKey.current = "";
      return;
    }

    const trackKey = `${smtc.artist.trim().toLowerCase()} - ${smtc.title.trim().toLowerCase()}`;
    if (trackKey === lastTrackKey.current) return;
    lastTrackKey.current = trackKey;

    const durationSec = Math.floor(smtc.duration_ms / 1000);

    invoke<TrackLyrics>("get_lyrics", {
      artist: smtc.artist,
      title: smtc.title,
      album: smtc.album || null,
      durationSec: durationSec > 0 ? durationSec : null,
    })
      .then((res) => {
        setLyrics(res);
      })
      .catch((err) => {
        console.warn("Failed to get lyrics:", err);
        setLyrics(null);
      });
  }, [smtc.title, smtc.artist, smtc.album, smtc.duration_ms]);

  // High-precision rAF interpolation loop
  useEffect(() => {
    let animId: number;

    const tick = () => {
      const now = Date.now();
      let currentMs = smtc.position_ms;
      if (smtc.playback_status === "Playing") {
        const elapsed = now - smtc.last_updated_unix_ms;
        currentMs = Math.min(smtc.position_ms + elapsed, smtc.duration_ms || Infinity);
      }

      if (!lyrics || !lyrics.lines || lyrics.lines.length === 0) {
        setProgress({
          currentLine: null,
          upcomingLine: null,
          activeWordIndex: -1,
          interpolatedWordFraction: 0,
          isSynced: false,
          hasLyrics: false,
          currentTimeMs: currentMs,
        });
        if (smtc.playback_status === "Playing") {
          animId = requestAnimationFrame(tick);
        }
        return;
      }

      const lines = lyrics.lines;
      let currentIndex = -1;

      // Find active line
      for (let i = 0; i < lines.length; i++) {
        if (currentMs >= lines[i].start_ms && currentMs < lines[i].end_ms) {
          currentIndex = i;
          break;
        }
      }

      // If before first line or between lines
      if (currentIndex === -1) {
        if (currentMs < lines[0].start_ms) {
          // Intro before first line
          setProgress({
            currentLine: null,
            upcomingLine: lines[0],
            activeWordIndex: -1,
            interpolatedWordFraction: 0,
            isSynced: lyrics.is_synced,
            hasLyrics: true,
            currentTimeMs: currentMs,
          });
        } else {
          // Find the last passed line or upcoming line
          let lastPassed = 0;
          for (let i = 0; i < lines.length; i++) {
            if (lines[i].start_ms <= currentMs) {
              lastPassed = i;
            } else {
              break;
            }
          }
          const cur = lines[lastPassed];
          const up = lastPassed + 1 < lines.length ? lines[lastPassed + 1] : null;
          setProgress({
            currentLine: cur,
            upcomingLine: up,
            activeWordIndex: cur.words.length - 1,
            interpolatedWordFraction: 1.0,
            isSynced: lyrics.is_synced,
            hasLyrics: true,
            currentTimeMs: currentMs,
          });
        }
      } else {
        const curLine = lines[currentIndex];
        const upLine = currentIndex + 1 < lines.length ? lines[currentIndex + 1] : null;

        // Word-level highlight evaluation
        let wordIdx = -1;
        let wordFrac = 0;

        if (curLine.words && curLine.words.length > 0) {
          for (let w = 0; w < curLine.words.length; w++) {
            const word = curLine.words[w];
            if (currentMs >= word.start_ms && currentMs < word.end_ms) {
              wordIdx = w;
              const dur = Math.max(word.end_ms - word.start_ms, 1);
              wordFrac = Math.min(Math.max((currentMs - word.start_ms) / dur, 0), 1);
              break;
            } else if (currentMs >= word.end_ms) {
              wordIdx = w;
              wordFrac = 1.0;
            }
          }
        }

        setProgress({
          currentLine: curLine,
          upcomingLine: upLine,
          activeWordIndex: wordIdx,
          interpolatedWordFraction: wordFrac,
          isSynced: lyrics.is_synced,
          hasLyrics: true,
          currentTimeMs: currentMs,
        });
      }

      if (smtc.playback_status === "Playing") {
        animId = requestAnimationFrame(tick);
      }
    };

    tick();

    return () => {
      if (animId) cancelAnimationFrame(animId);
    };
  }, [lyrics, smtc.position_ms, smtc.playback_status, smtc.last_updated_unix_ms, smtc.duration_ms]);

  return progress;
}
