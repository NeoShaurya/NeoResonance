import React from "react";
import { Lock, Unlock, Sparkles } from "lucide-react";

interface WidgetFooterProps {
  locked: boolean;
  onToggleLock: () => void;
}

export const WidgetFooter: React.FC<WidgetFooterProps> = ({
  locked,
  onToggleLock,
}) => {
  return (
    <div className="flex items-center justify-between w-full select-none pt-0.5 px-1">
      {/* Bottom Left Lock State */}
      <button
        onClick={onToggleLock}
        className="flex items-center space-x-1.5 opacity-40 hover:opacity-80 transition-opacity duration-150 cursor-pointer group"
        title={locked ? "Click to Unlock and Drag" : "Click to Lock Position"}
      >
        {locked ? (
          <Lock className="w-2.5 h-2.5 text-white" />
        ) : (
          <Unlock className="w-2.5 h-2.5 text-cyan-300 animate-pulse" />
        )}
        <span className="text-[9.5px] font-medium tracking-wider text-white uppercase">
          {locked ? "Locked" : "Unlocked — Drag to Move"}
        </span>
      </button>

      {/* Subtle Reference Sparkle (Bottom Right) */}
      <div className="opacity-25 hover:opacity-60 transition-opacity duration-200">
        <Sparkles className="w-3 h-3 text-white fill-white/20" />
      </div>
    </div>
  );
};
