import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef, useState } from "react";

interface MpvStatus {
  position: number;
  duration: number;
  paused: boolean;
}

interface MpvPlayerProps {
  title: string;
  onClose: () => void;
}

function fmt(sec: number): string {
  if (!isFinite(sec) || isNaN(sec) || sec < 0) return "0:00";
  const h = Math.floor(sec / 3600);
  const m = Math.floor((sec % 3600) / 60);
  const s = Math.floor(sec % 60);
  if (h > 0) return `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
  return `${m}:${String(s).padStart(2, "0")}`;
}

export function MpvPlayer({ title, onClose }: MpvPlayerProps) {
  const seekBarRef  = useRef<HTMLDivElement>(null);
  const hideTimer   = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pollRef     = useRef<ReturnType<typeof setInterval> | null>(null);

  const [status,   setStatus]   = useState<MpvStatus>({ position: 0, duration: 0, paused: true });
  const [volume,   setVolume]   = useState(1);
  const [visible,  setVisible]  = useState(true);

  // Stop MPV when the HUD is closed
  useEffect(() => {
    return () => {
      invoke("mpv_stop").catch(console.warn);
    };
  }, []);

  // Poll MPV status every second
  useEffect(() => {
    const poll = async () => {
      try {
        const s = await invoke<MpvStatus>("mpv_get_status");
        setStatus(s);
      } catch {/* MPV not ready yet */}
    };
    poll();
    pollRef.current = setInterval(poll, 1000);
    return () => { if (pollRef.current) clearInterval(pollRef.current); };
  }, []);

  const resetHide = useCallback(() => {
    setVisible(true);
    if (hideTimer.current) clearTimeout(hideTimer.current);
    hideTimer.current = setTimeout(() => setVisible(false), 3000);
  }, []);

  useEffect(() => {
    resetHide();
    return () => { if (hideTimer.current) clearTimeout(hideTimer.current); };
  }, [resetHide]);

  const togglePlay = useCallback(() => {
    const next = !status.paused;
    invoke("mpv_set_pause", { paused: next }).catch(console.warn);
    setStatus(s => ({ ...s, paused: next }));
  }, [status.paused]);

  const handleSeekClick = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    const bar = seekBarRef.current;
    if (!bar || status.duration <= 0) return;
    const rect = bar.getBoundingClientRect();
    const t = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width)) * status.duration;
    invoke("mpv_seek", { seconds: t }).catch(console.warn);
    setStatus(s => ({ ...s, position: t }));
  }, [status.duration]);

  const handleVolumeChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const v = Number(e.target.value);
    setVolume(v);
    invoke("mpv_set_volume", { volume: v }).catch(console.warn);
  }, []);

  const handleClose = useCallback(() => {
    invoke("mpv_stop").catch(console.warn);
    onClose();
  }, [onClose]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const tag = (e.target as HTMLElement).tagName;
      if (tag === "INPUT" || tag === "TEXTAREA") return;
      switch (e.key) {
        case "Escape":
          handleClose(); return;
        case " ": case "k":
          e.preventDefault(); togglePlay(); break;
        case "ArrowRight":
          e.preventDefault();
          invoke("mpv_seek", { seconds: Math.min(status.position + 10, status.duration) }).catch(console.warn);
          break;
        case "ArrowLeft":
          e.preventDefault();
          invoke("mpv_seek", { seconds: Math.max(status.position - 10, 0) }).catch(console.warn);
          break;
        case "ArrowUp":
          e.preventDefault(); {
            const v = Math.min(volume + 0.1, 1);
            setVolume(v);
            invoke("mpv_set_volume", { volume: v }).catch(console.warn);
          } break;
        case "ArrowDown":
          e.preventDefault(); {
            const v = Math.max(volume - 0.1, 0);
            setVolume(v);
            invoke("mpv_set_volume", { volume: v }).catch(console.warn);
          } break;
      }
      resetHide();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [handleClose, togglePlay, resetHide, status.position, status.duration, volume]);

  const progressPct = status.duration > 0 ? (status.position / status.duration) * 100 : 0;

  return (
    <div
      className="player-container"
      style={{ background: "transparent", cursor: visible ? "default" : "none" }}
      onMouseMove={resetHide}
    >
      {/* No <video> element — MPV renders into the native child window below */}

      {/* ── Top bar ── */}
      <div className={`player-top${visible ? " player-ui-on" : ""}`}>
        <button className="player-back" onClick={handleClose}>
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
            <polyline points="15 18 9 12 15 6"/>
          </svg>
          Back
        </button>
        <span className="player-heading">{title}</span>
      </div>

      {/* ── Bottom controls ── */}
      <div className={`player-bottom${visible ? " player-ui-on" : ""}`}>

        {/* Seek bar */}
        <div ref={seekBarRef} className="player-seek" onClick={handleSeekClick}>
          <div className="player-seek-track">
            <div className="player-seek-prog" style={{ width: `${progressPct}%` }}>
              <div className="player-seek-knob" />
            </div>
          </div>
        </div>

        {/* Controls row */}
        <div className="player-row">

          {/* Left cluster */}
          <div className="player-cluster">
            <button className="player-icon-btn" onClick={togglePlay} title={status.paused ? "Play (Space)" : "Pause (Space)"}>
              {status.paused ? (
                <svg width="22" height="22" viewBox="0 0 24 24" fill="currentColor">
                  <polygon points="5,3 19,12 5,21"/>
                </svg>
              ) : (
                <svg width="22" height="22" viewBox="0 0 24 24" fill="currentColor">
                  <rect x="6" y="4" width="4" height="16" rx="1"/>
                  <rect x="14" y="4" width="4" height="16" rx="1"/>
                </svg>
              )}
            </button>

            <div className="player-vol-wrap">
              <button className="player-icon-btn" title="Volume">
                {volume === 0 ? (
                  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
                    <polygon points="11,5 6,9 2,9 2,15 6,15 11,19" fill="currentColor" stroke="none"/>
                    <line x1="23" y1="9" x2="17" y2="15"/><line x1="17" y1="9" x2="23" y2="15"/>
                  </svg>
                ) : volume < 0.5 ? (
                  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
                    <polygon points="11,5 6,9 2,9 2,15 6,15 11,19" fill="currentColor" stroke="none"/>
                    <path d="M15.54 8.46a5 5 0 0 1 0 7.07"/>
                  </svg>
                ) : (
                  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
                    <polygon points="11,5 6,9 2,9 2,15 6,15 11,19" fill="currentColor" stroke="none"/>
                    <path d="M15.54 8.46a5 5 0 0 1 0 7.07"/>
                    <path d="M19.07 4.93a10 10 0 0 1 0 14.14"/>
                  </svg>
                )}
              </button>
              <input
                className="player-vol-slider"
                type="range" min={0} max={1} step={0.02}
                value={volume}
                style={{ "--vol": `${volume * 100}%` } as React.CSSProperties}
                onChange={handleVolumeChange}
                onClick={e => e.stopPropagation()}
              />
            </div>

            <span className="player-time">
              {fmt(status.position)}
              <span className="player-time-sep"> / </span>
              {status.duration > 0 ? fmt(status.duration) : "--:--"}
            </span>
          </div>

          {/* Right cluster (placeholder for future subtitle/audio track support) */}
          <div className="player-cluster" />

        </div>
      </div>
    </div>
  );
}
