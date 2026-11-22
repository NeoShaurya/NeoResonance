import React, { useEffect, useRef } from "react";

interface StringBaselineWaveProps {
  waveform: number[]; // 32 frequency band energies (0.0 .. 1.0)
  isSilent: boolean;
  visible: boolean;
}

export const StringBaselineWave: React.FC<StringBaselineWaveProps> = ({
  waveform,
  isSilent,
  visible,
}) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const currentPointsRef = useRef<number[]>(new Array(32).fill(0));
  const timeRef = useRef<number>(0);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !visible) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    let animId: number;

    const render = () => {
      const width = canvas.width;
      const height = canvas.height;
      const centerY = height / 2;

      ctx.clearRect(0, 0, width, height);

      const target = isSilent ? new Array(32).fill(0) : waveform;
      const current = currentPointsRef.current;

      // Responsive spring physics smoothing
      for (let i = 0; i < 32; i++) {
        const tVal = target[i] || 0;
        current[i] += (tVal - current[i]) * 0.32;
      }

      timeRef.current += 0.048;
      const time = timeRef.current;
      const numPoints = current.length;
      const stepX = width / (numPoints - 1);
      const amp = 14.5; // Pronounced oscillation amplitude

      // Helper function to draw a smooth Catmull-Rom Bezier wave
      const drawWavePath = (
        waveFn: (idx: number, energy: number, taper: number) => number
      ) => {
        ctx.beginPath();
        const p0y = centerY + waveFn(0, current[0], 0);
        ctx.moveTo(0, p0y);

        for (let i = 0; i < numPoints - 1; i++) {
          const taper0 = Math.sin((Math.max(0, i - 1) / (numPoints - 1)) * Math.PI);
          const taper1 = Math.sin((i / (numPoints - 1)) * Math.PI);
          const taper2 = Math.sin(((i + 1) / (numPoints - 1)) * Math.PI);
          const taper3 = Math.sin((Math.min(numPoints - 1, i + 2) / (numPoints - 1)) * Math.PI);

          const x0 = i > 0 ? (i - 1) * stepX : 0;
          const y0 = centerY + waveFn(Math.max(0, i - 1), current[Math.max(0, i - 1)], taper0);

          const x1 = i * stepX;
          const y1 = centerY + waveFn(i, current[i], taper1);

          const x2 = (i + 1) * stepX;
          const y2 = centerY + waveFn(i + 1, current[i + 1], taper2);

          const x3 = i + 2 < numPoints ? (i + 2) * stepX : x2;
          const y3 = centerY + waveFn(Math.min(numPoints - 1, i + 2), current[Math.min(numPoints - 1, i + 2)], taper3);

          const cp1x = x1 + (x2 - x0) / 6;
          const cp1y = y1 + (y2 - y0) / 6;
          const cp2x = x2 - (x3 - x1) / 6;
          const cp2y = y2 - (y3 - y1) / 6;

          ctx.bezierCurveTo(cp1x, cp1y, cp2x, cp2y, x2, y2);
        }
      };

      // 1. Ghost Harmonic String (Violet depth layer)
      drawWavePath((i, energy, taper) => {
        const w1 = Math.sin(time * 3.2 - i * 0.48) * (energy * 0.75);
        const w2 = Math.cos(time * 5.0 + i * 0.35) * (energy * 0.4);
        return (w1 + w2) * (amp * 0.7) * taper;
      });
      ctx.strokeStyle = "rgba(168, 85, 247, 0.38)";
      ctx.lineWidth = 1.0;
      ctx.shadowColor = "rgba(168, 85, 247, 0.45)";
      ctx.shadowBlur = 5;
      ctx.stroke();

      // 2. Primary 1px Resonant String Wave (Vibrant Cyan-Violet-White)
      drawWavePath((i, energy, taper) => {
        const w1 = Math.sin(time * 3.8 + i * 0.44) * energy;
        const w2 = Math.sin(time * 5.6 - i * 0.62) * (energy * 0.55);
        const w3 = Math.cos(time * 2.1 + i * 0.28) * (energy * 0.35);
        return (w1 + w2 + w3) * amp * taper;
      });

      const gradient = ctx.createLinearGradient(0, 0, width, 0);
      gradient.addColorStop(0, "rgba(255, 255, 255, 0.35)");
      gradient.addColorStop(0.25, "rgba(168, 85, 247, 0.9)");  // Violet
      gradient.addColorStop(0.65, "rgba(56, 189, 248, 0.95)");  // Electric cyan
      gradient.addColorStop(0.85, "rgba(255, 255, 255, 0.85)"); // White shimmer
      gradient.addColorStop(1, "rgba(255, 255, 255, 0.35)");

      ctx.strokeStyle = gradient;
      ctx.lineWidth = 1.2;
      ctx.shadowColor = "rgba(56, 189, 248, 0.85)";
      ctx.shadowBlur = 8;
      ctx.stroke();

      animId = requestAnimationFrame(render);
    };

    render();

    return () => {
      cancelAnimationFrame(animId);
    };
  }, [waveform, isSilent, visible]);

  if (!visible) return null;

  return (
    <div className="relative w-full h-5 px-1 flex items-center justify-center pointer-events-none select-none overflow-visible">
      <canvas
        ref={canvasRef}
        width={660}
        height={36}
        className="w-full h-[36px] absolute pointer-events-none"
      />
    </div>
  );
};
