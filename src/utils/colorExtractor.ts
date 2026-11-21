import { AuraPalette } from "../types";

const DEFAULT_PALETTE: AuraPalette = {
  primary: "rgba(139, 92, 246, 0.65)",   // Purple
  secondary: "rgba(14, 165, 233, 0.65)", // Cyan / Sky Blue
  accent: "rgba(236, 72, 153, 0.45)",    // Pink / Magenta
};

export function extractPaletteFromImage(
  dataUrl: string | null | undefined,
  callback: (palette: AuraPalette) => void
): void {
  if (!dataUrl) {
    callback(DEFAULT_PALETTE);
    return;
  }

  const img = new Image();
  img.crossOrigin = "anonymous";
  img.onload = () => {
    try {
      const canvas = document.createElement("canvas");
      const ctx = canvas.getContext("2d", { willReadFrequently: true });
      if (!ctx) {
        callback(DEFAULT_PALETTE);
        return;
      }

      // Sample a small 32x32 image to quickly find vibrant colors
      canvas.width = 32;
      canvas.height = 32;
      ctx.drawImage(img, 0, 0, 32, 32);
      const imgData = ctx.getImageData(0, 0, 32, 32).data;

      const colors: { r: number; g: number; b: number; sat: number; val: number }[] = [];

      for (let i = 0; i < imgData.length; i += 16) {
        const r = imgData[i];
        const g = imgData[i + 1];
        const b = imgData[i + 2];

        // Convert to HSV to evaluate vibrance
        const max = Math.max(r, g, b);
        const min = Math.min(r, g, b);
        const delta = max - min;
        const sat = max === 0 ? 0 : delta / max;
        const val = max / 255;

        // Skip pure blacks, pure whites, and muddy greys
        if (sat > 0.25 && val > 0.25 && val < 0.95) {
          colors.push({ r, g, b, sat, val });
        }
      }

      if (colors.length < 2) {
        callback(DEFAULT_PALETTE);
        return;
      }

      // Sort by saturation and brightness
      colors.sort((a, b) => b.sat * b.val - a.sat * a.val);

      const c1 = colors[0];
      const c2 = colors[Math.min(Math.floor(colors.length * 0.4), colors.length - 1)];
      const c3 = colors[Math.min(Math.floor(colors.length * 0.8), colors.length - 1)];

      callback({
        primary: `rgba(${c1.r}, ${c1.g}, ${c1.b}, 0.65)`,
        secondary: `rgba(${c2.r}, ${c2.g}, ${c2.b}, 0.65)`,
        accent: `rgba(${c3.r}, ${c3.g}, ${c3.b}, 0.45)`,
      });
    } catch {
      callback(DEFAULT_PALETTE);
    }
  };

  img.onerror = () => {
    callback(DEFAULT_PALETTE);
  };

  img.src = dataUrl;
}
