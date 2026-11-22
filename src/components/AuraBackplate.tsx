import React from "react";
import { AuraPalette } from "../types";

interface AuraBackplateProps {
  bassEnergy: number; // 0.0 .. 1.0
  midEnergy: number;  // 0.0 .. 1.0
  palette: AuraPalette;
  visible: boolean;
}

export const AuraBackplate: React.FC<AuraBackplateProps> = ({
  bassEnergy,
  midEnergy,
  palette,
  visible,
}) => {
  if (!visible) return null;

  // Reactivity calculations for organic breathing
  const bassScale = 1.0 + bassEnergy * 0.45;
  const bassOpacity = 0.4 + bassEnergy * 0.6;
  const midScale = 1.0 + midEnergy * 0.35;
  const midOpacity = 0.4 + midEnergy * 0.55;

  return (
    <div className="absolute inset-0 overflow-hidden pointer-events-none rounded-[24px] -z-10">
      {/* Dynamic Purple/Violet Bass Cloud (Left/Center) */}
      <div
        className="absolute w-[330px] h-[240px] -left-10 -bottom-12 rounded-full blur-[55px] transition-transform duration-100 ease-out will-change-transform"
        style={{
          background: palette.primary,
          transform: `scale(${bassScale}) translate(${bassEnergy * 8}px, ${-bassEnergy * 6}px)`,
          opacity: bassOpacity,
        }}
      />

      {/* Dynamic Electric Cyan/Blue Mid Cloud (Right/Center) */}
      <div
        className="absolute w-[340px] h-[250px] right-2 top-4 rounded-full blur-[60px] transition-transform duration-100 ease-out will-change-transform"
        style={{
          background: palette.secondary,
          transform: `scale(${midScale}) translate(${-midEnergy * 6}px, ${midEnergy * 8}px)`,
          opacity: midOpacity,
        }}
      />

      {/* Ambient Accent Magenta/Teal Cloud (Center Stage) */}
      <div
        className="absolute w-[250px] h-[200px] left-1/3 top-1/4 rounded-full blur-[65px] transition-opacity duration-200 ease-out will-change-transform"
        style={{
          background: palette.accent,
          transform: `scale(${1.0 + (bassEnergy + midEnergy) * 0.15})`,
          opacity: 0.35 + (bassEnergy + midEnergy) * 0.25,
        }}
      />

      {/* Deep Dark Base Tint for Crisp Text Legibility */}
      <div className="absolute inset-0 bg-[#0d0e17]/45 backdrop-blur-[30px]" />
    </div>
  );
};
