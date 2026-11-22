import React, { useEffect, useRef } from "react";

interface StringBaselineWaveProps {
  waveform: number[]; // 32 points normalized -1.0 .. 1.0
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

      // Spring physics smoothing between current and target wave points
      for (let i = 0; i < 32; i++) {
        current[i] += ((target[i] || 0) - current[i]) * 0.22;
      }

      // Draw 1px continuous physics Bezier string
      ctx.beginPath();

      const numPoints = current.length;
      const stepX = width / (numPoints - 1);
      const amp = height * 0.42;

      // Start point
      const p0x = 0;
      const p0y = centerY + current[0] * amp;
      ctx.moveTo(p0x, p0y);

      // Smooth cubic spline / bezier interpolation across points
      for (let i = 0; i < numPoints - 1; i++) {
        const x0 = i > 0 ? (i - 1) * stepX : 0;
        const y0 = centerY + current[Math.max(0, i - 1)] * amp;

        const x1 = i * stepX;
        const y1 = centerY + current[i] * amp;

        const x2 = (i + 1) * stepX;
        const y2 = centerY + current[i + 1] * amp;

        const x3 = i + 2 < numPoints ? (i + 2) * stepX : x2;
        const y3 = centerY + current[Math.min(numPoints - 1, i + 2)] * amp;

        // Catmull-Rom to Cubic Bezier control point conversion
        const cp1x = x1 + (x2 - x0) / 6;
        const cp1y = y1 + (y2 - y0) / 6;

        const cp2x = x2 - (x3 - x1) / 6;
        const cp2y = y2 - (y3 - y1) / 6;

        ctx.bezierCurveTo(cp1x, cp1y, cp2x, cp2y, x2, y2);
      }

      // Stroke style: ultra-fine 1px glowing line with subtle cyan/white gradient
      const gradient = ctx.createLinearGradient(0, 0, width, 0);
      gradient.addColorStop(0, "rgba(255, 255, 255, 0.4)");
      gradient.addColorStop(0.3, "rgba(168, 85, 247, 0.85)"); // Subtle violet
      gradient.addColorStop(0.7, "rgba(56, 189, 248, 0.85)");  // Electric cyan
      gradient.addColorStop(1, "rgba(255, 255, 255, 0.4)");

      ctx.strokeStyle = gradient;
      ctx.lineWidth = 1.0;
      ctx.shadowColor = "rgba(56, 189, 248, 0.6)";
      ctx.shadowBlur = 6;
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
    <div className="w-full h-5 px-1 flex items-center justify-center pointer-events-none select-none">
      <canvas
        ref={canvasRef}
        width={660}
        height={20}
        className="w-full h-full block"
      />
    </div>
  );
};
