import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

interface MpvState {
  playing: boolean;
  time_pos: number;
  duration: number;
  volume: number;
  muted: boolean;
  idle: boolean;
  paused_for_cache: boolean;
  title: string;
}

interface Track {
  type: string;
  id: number;
  title: string;
  lang: string;
  selected: boolean;
}

interface TorrentStats {
  downloadSpeed: number;
  uploadSpeed:   number;
  peers:         number;
  downloaded:    number;
  uploaded:      number;
}

function fmtBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1073741824) return `${(bytes / 1048576).toFixed(1)} MB`;
  return `${(bytes / 1073741824).toFixed(2)} GB`;
}

function fmtSpeed(bps: number): string {
  return `${fmtBytes(bps)}/s`;
}

interface PlayerProps {
  streamUrl: string | null;
  logo?: string;
  poster?: string;
  title: string;
  infoHash?: string;
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

export function Player({ streamUrl, logo, poster, title, infoHash, onClose }: PlayerProps) {
  const containerRef   = useRef<HTMLDivElement>(null);
  const seekBarRef     = useRef<HTMLDivElement>(null);
  const hideTimer      = useRef<ReturnType<typeof setTimeout> | null>(null);
  const firstFrameRef  = useRef(false);

  const [playing,     setPlaying]     = useState(false);
  const [currentTime, setCurrentTime] = useState(0);
  const [duration,    setDuration]    = useState(0);
  const [volume,      setVolume]      = useState(100);
  const [muted,       setMuted]       = useState(false);

  const currentTimeRef = useRef(0);
  const durationRef    = useRef(0);
  const volumeRef      = useRef(100);
  const mutedRef       = useRef(false);
  const [buffering,   setBuffering]   = useState(true);
  const [visible,     setVisible]     = useState(true);
  const [fullscreen,  setFullscreen]  = useState(false);
  const [subMenu,     setSubMenu]     = useState(false);
  const [audioMenu,   setAudioMenu]   = useState(false);
  const [subtitles,   setSubtitles]   = useState<Track[]>([]);
  const [activeSub,   setActiveSub]   = useState(-1);
  const [audios,      setAudios]      = useState<Track[]>([]);
  const [activeAudio, setActiveAudio] = useState(0);

  const [logoFailed,   setLogoFailed]   = useState(false);
  const [posterFailed, setPosterFailed] = useState(false);

  const [showStats,    setShowStats]    = useState(false);
  const [torrentStats, setTorrentStats] = useState<TorrentStats | null>(null);
  const statsIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const resetHide = useCallback(() => {
    setVisible(true);
    if (hideTimer.current) clearTimeout(hideTimer.current);
    hideTimer.current = setTimeout(() => setVisible(false), 3000);
  }, []);

  const toggleFullscreen = useCallback(async () => {
    const win = getCurrentWindow();
    const isFs = await win.isFullscreen();
    await win.setFullscreen(!isFs);
    setFullscreen(!isFs);
  }, []);

  // ── Add mpv-active synchronously before the first paint ─
  useLayoutEffect(() => {
    document.body.classList.add("mpv-active");
    invoke("set_window_background", { transparent: true });
    return () => {
      document.body.classList.remove("mpv-active");
      document.body.classList.remove("mpv-ready");
      firstFrameRef.current = false;
      invoke("set_window_background", { transparent: false });
    };
  }, []);

  // ── Start mpv playback & subscribe to state events ───────
  useEffect(() => {
    if (!streamUrl) return; // wait until URL is resolved

    let unlistenState: UnlistenFn | null = null;
    let unlistenEnd: UnlistenFn | null = null;
    let unlistenFirstFrame: UnlistenFn | null = null;
    let unlistenFileLoaded: UnlistenFn | null = null;
    let mounted = true;

    (async () => {
      // Register listeners before invoking mpv_play to avoid race conditions

      // Clear buffering overlay and make webview transparent once first GL frame is on screen
      unlistenFirstFrame = await listen("mpv-first-frame", () => {
        if (mounted) {
          firstFrameRef.current = true;
          setBuffering(false);
          document.body.classList.add("mpv-ready");
        }
      });

      // Listen for mpv state updates
      unlistenState = await listen<MpvState>("mpv-state", (event) => {
        if (!mounted) return;
        const s = event.payload;
        setPlaying(s.playing);
        currentTimeRef.current = s.time_pos;
        setCurrentTime(s.time_pos);
        if (s.duration > 0 && isFinite(s.duration)) {
          durationRef.current = s.duration;
          setDuration(s.duration);
        }
        volumeRef.current = s.volume;
        setVolume(s.volume);
        mutedRef.current = s.muted;
        setMuted(s.muted);
        setBuffering(firstFrameRef.current ? s.paused_for_cache : true);
        if (firstFrameRef.current) {
          if (s.paused_for_cache) document.body.classList.remove("mpv-ready");
          else document.body.classList.add("mpv-ready");
        }
      });

      // Listen for end-of-file
      unlistenEnd = await listen<string>("mpv-end-file", (event) => {
        if (!mounted) return;
        if (event.payload === "eof") {
          if (infoHash) invoke("stop_stream_command", { infoHash });
          onClose();
        }
      });

      // Tell mpv to load this URL
      await invoke("mpv_play", { url: streamUrl });

      // Fetch available tracks when mpv signals the file is fully loaded
      const fetchTracks = async () => {
        if (!mounted) return;
        try {
          const tracksJson: string = await invoke("mpv_get_tracks");
          const tracks: Track[] = JSON.parse(tracksJson);
          setSubtitles(tracks.filter(t => t.type === "sub"));
          setAudios(tracks.filter(t => t.type === "audio"));
          const activeSt = tracks.find(t => t.type === "sub" && t.selected);
          const activeAt = tracks.find(t => t.type === "audio" && t.selected);
          if (activeSt) setActiveSub(activeSt.id);
          if (activeAt) setActiveAudio(activeAt.id);
        } catch { /* tracks not available yet */ }
      };
      unlistenFileLoaded = await listen("mpv-file-loaded", fetchTracks);
    })();

    // Keyboard shortcuts
    const onKey = (e: KeyboardEvent) => {
      const tag = (e.target as HTMLElement).tagName;
      if (tag === "INPUT" || tag === "TEXTAREA") return;
      switch (e.key) {
        case "Escape":      getCurrentWindow().setFullscreen(false).catch(() => {}); return;
        case " ": case "k": e.preventDefault(); invoke("mpv_toggle_pause"); break;
        case "f": case "F": e.preventDefault(); toggleFullscreen(); break;
        case "ArrowRight":  e.preventDefault(); invoke("mpv_seek", { seconds: Math.min(currentTimeRef.current + 10, durationRef.current || Infinity) }); break;
        case "ArrowLeft":   e.preventDefault(); invoke("mpv_seek", { seconds: Math.max(currentTimeRef.current - 10, 0) }); break;
        case "ArrowUp":     e.preventDefault(); invoke("mpv_set_volume", { volume: Math.min(volumeRef.current + 5, 150) }); break;
        case "ArrowDown":   e.preventDefault(); invoke("mpv_set_volume", { volume: Math.max(volumeRef.current - 5, 0) }); break;
        case "m": case "M": invoke("mpv_set_mute", { muted: !mutedRef.current }); break;
      }
      resetHide();
    };

    // Sync fullscreen state when native window exits fullscreen (Esc / OS green button)
    let unlistenResize: (() => void) | null = null;
    getCurrentWindow().onResized(async () => {
      const isFs = await getCurrentWindow().isFullscreen();
      setFullscreen(isFs);
      // Notify the GL context of the new dimensions so mpv renders at full size
      invoke("mpv_update_context").catch(() => {});
    }).then(fn => { unlistenResize = fn; });

    window.addEventListener("keydown", onKey);
    resetHide();

    return () => {
      mounted = false;
      window.removeEventListener("keydown", onKey);
      unlistenResize?.();
      if (hideTimer.current) clearTimeout(hideTimer.current);
      unlistenState?.();
      unlistenEnd?.();
      unlistenFirstFrame?.();
      unlistenFileLoaded?.();
      invoke("mpv_stop");
    };
  }, [streamUrl]);

  // ── Handlers ─────────────────────────────────────────────

  const togglePlay = useCallback(() => {
    invoke("mpv_toggle_pause");
  }, []);

  const handleSeek = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    const bar = seekBarRef.current;
    if (!bar || duration <= 0) return;
    const rect = bar.getBoundingClientRect();
    const pos = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width)) * duration;
    invoke("mpv_seek", { seconds: pos });
  }, [duration]);

  const handleVolume = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const val = Number(e.target.value);
    invoke("mpv_set_volume", { volume: val });
    if (val === 0) invoke("mpv_set_mute", { muted: true });
    else if (muted) invoke("mpv_set_mute", { muted: false });
  }, [muted]);

  const toggleMute = useCallback(() => {
    invoke("mpv_set_mute", { muted: !muted });
  }, [muted]);

  const selectSub = useCallback((id: number) => {
    invoke("mpv_set_track", { trackType: "sub", id });
    setActiveSub(id);
    setSubMenu(false);
  }, []);

  const selectAudio = useCallback((id: number) => {
    invoke("mpv_set_track", { trackType: "audio", id });
    setActiveAudio(id);
    setAudioMenu(false);
  }, []);

  const fetchTorrentStats = useCallback(async () => {
    if (!infoHash || !streamUrl) return;
    try {
      const { origin } = new URL(streamUrl);
      const res = await fetch(`${origin}/${infoHash}/stats`);
      if (!res.ok) return;
      const data: TorrentStats = await res.json();
      setTorrentStats({
        downloadSpeed: data.downloadSpeed,
        uploadSpeed:   data.uploadSpeed,
        peers:         data.peers,
        downloaded:    data.downloaded,
        uploaded:      data.uploaded,
      });
    } catch { /* ignore */ }
  }, [infoHash, streamUrl]);

  const handleStatsEnter = useCallback(() => {
    setShowStats(true);
    fetchTorrentStats();
    statsIntervalRef.current = setInterval(fetchTorrentStats, 2000);
  }, [fetchTorrentStats]);

  const handleStatsLeave = useCallback(() => {
    setShowStats(false);
    if (statsIntervalRef.current) {
      clearInterval(statsIntervalRef.current);
      statsIntervalRef.current = null;
    }
  }, []);

  useEffect(() => () => {
    if (statsIntervalRef.current) clearInterval(statsIntervalRef.current);
  }, []);

  const handleClose = useCallback(() => {
    getCurrentWindow().setFullscreen(false).catch(() => {});
    invoke("mpv_stop");
    if (infoHash) invoke("stop_stream_command", { infoHash });
    onClose();
  }, [onClose, infoHash]);

  const progressPct = duration > 0 ? (currentTime / duration) * 100 : 0;
  const effectiveVol = muted ? 0 : volume;
  const volNorm = effectiveVol / 100; // normalise 0-100 to 0-1 for display

  return (
    <div
      ref={containerRef}
      className="player-container"
      onMouseMove={resetHide}
      onClick={() => { setSubMenu(false); setAudioMenu(false); }}
      onContextMenu={e => e.preventDefault()}
      style={{ cursor: visible ? "default" : "none" }}
    >
      {/* Transparent click target — mpv renders natively behind this */}
      <div className="player-video" onClick={togglePlay} />

      {/* ── Buffering overlay ──────────────────────────── */}
      {buffering && (
        <div className="player-buffering">
          {logo && !logoFailed ? (
            <img src={logo} alt={title} className="player-buffering-logo" onError={() => setLogoFailed(true)} />
          ) : poster && !posterFailed ? (
            <img src={poster} alt={title} className="player-buffering-poster" onError={() => setPosterFailed(true)} />
          ) : (
            <span className="player-buffering-title">{title}</span>
          )}
        </div>
      )}

      {/* ── Top bar ─────────────────────────────────── */}
      <div className={`player-top${visible ? " player-ui-on" : ""}`}>
        <button className="player-back" onClick={handleClose}>
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
            <polyline points="15 18 9 12 15 6"/>
          </svg>
          Back
        </button>
        <span className="player-heading">{title}</span>
      </div>

      {/* ── Bottom controls ──────────────────────────── */}
      <div className={`player-bottom${visible ? " player-ui-on" : ""}`}>

        {/* Seek bar */}
        <div ref={seekBarRef} className="player-seek" onClick={handleSeek}>
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
            <button className="player-icon-btn" onClick={togglePlay} title={playing ? "Pause (Space)" : "Play (Space)"}>
              {playing ? (
                <svg width="22" height="22" viewBox="0 0 24 24" fill="currentColor">
                  <rect x="6" y="4" width="4" height="16" rx="1"/>
                  <rect x="14" y="4" width="4" height="16" rx="1"/>
                </svg>
              ) : (
                <svg width="22" height="22" viewBox="0 0 24 24" fill="currentColor">
                  <polygon points="5,3 19,12 5,21"/>
                </svg>
              )}
            </button>

            <div className="player-vol-wrap">
              <button className="player-icon-btn" onClick={toggleMute} title="Toggle mute (M)">
                {volNorm === 0 ? (
                  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
                    <polygon points="11,5 6,9 2,9 2,15 6,15 11,19" fill="currentColor" stroke="none"/>
                    <line x1="23" y1="9" x2="17" y2="15"/><line x1="17" y1="9" x2="23" y2="15"/>
                  </svg>
                ) : volNorm < 0.5 ? (
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
                type="range" min={0} max={100} step={1}
                value={effectiveVol}
                style={{ "--vol": `${volNorm * 100}%` } as React.CSSProperties}
                onChange={handleVolume}
                onClick={e => e.stopPropagation()}
              />
            </div>

            <span className="player-time">
              {duration > 0 ? fmt(currentTime) : "--:--"}
              <span className="player-time-sep"> / </span>
              {duration > 0 ? fmt(duration) : "--:--"}
            </span>
          </div>

          {/* Right cluster */}
          <div className="player-cluster">

            {/* Subtitle picker */}
            {subtitles.length > 0 && (
              <div className="player-menu-anchor">
                <button
                  className={`player-pill-btn${activeSub >= 0 ? " active" : ""}`}
                  onClick={e => { e.stopPropagation(); setSubMenu(v => !v); setAudioMenu(false); }}
                  title="Subtitles"
                >
                  CC
                </button>
                {subMenu && (
                  <div className="player-popup" onClick={e => e.stopPropagation()}>
                    <p className="player-popup-head">Subtitles</p>
                    <button className={`player-popup-item${activeSub === -1 ? " active" : ""}`} onClick={() => selectSub(-1)}>Off</button>
                    {subtitles.map(t => (
                      <button key={t.id} className={`player-popup-item${activeSub === t.id ? " active" : ""}`} onClick={() => selectSub(t.id)}>
                        {t.title || t.lang || `Subtitle ${t.id}`}
                      </button>
                    ))}
                  </div>
                )}
              </div>
            )}

            {/* Audio track picker */}
            {audios.length > 1 && (
              <div className="player-menu-anchor">
                <button
                  className="player-pill-btn"
                  onClick={e => { e.stopPropagation(); setAudioMenu(v => !v); setSubMenu(false); }}
                  title="Audio track"
                >
                  Audio
                </button>
                {audioMenu && (
                  <div className="player-popup" onClick={e => e.stopPropagation()}>
                    <p className="player-popup-head">Audio Track</p>
                    {audios.map(t => (
                      <button key={t.id} className={`player-popup-item${activeAudio === t.id ? " active" : ""}`} onClick={() => selectAudio(t.id)}>
                        {t.title || t.lang || `Audio ${t.id}`}
                      </button>
                    ))}
                  </div>
                )}
              </div>
            )}

            {/* Torrent stats */}
            {infoHash && (
              <div
                className="player-menu-anchor"
                onMouseEnter={handleStatsEnter}
                onMouseLeave={handleStatsLeave}
              >
                <button className="player-icon-btn" title="Stream stats">
                  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                    <line x1="18" y1="20" x2="18" y2="10"/><line x1="12" y1="20" x2="12" y2="4"/><line x1="6" y1="20" x2="6" y2="14"/>
                  </svg>
                </button>
                {showStats && (
                  <div className="player-popup player-stats-popup" onClick={e => e.stopPropagation()}>
                    <p className="player-popup-head">Stream Stats</p>
                    <div className="player-stats-grid">
                      <span className="player-stats-label">↓ Download</span>
                      <span className="player-stats-value">{torrentStats ? fmtSpeed(torrentStats.downloadSpeed) : "—"}</span>
                      <span className="player-stats-label">↑ Upload</span>
                      <span className="player-stats-value">{torrentStats ? fmtSpeed(torrentStats.uploadSpeed) : "—"}</span>
                      <span className="player-stats-label">Peers</span>
                      <span className="player-stats-value">{torrentStats ? torrentStats.peers : "—"}</span>
                      <span className="player-stats-label">Downloaded</span>
                      <span className="player-stats-value">{torrentStats ? fmtBytes(torrentStats.downloaded) : "—"}</span>
                      <span className="player-stats-label">Uploaded</span>
                      <span className="player-stats-value">{torrentStats ? fmtBytes(torrentStats.uploaded) : "—"}</span>
                    </div>
                  </div>
                )}
              </div>
            )}

            {/* Fullscreen */}
            <button className="player-icon-btn" onClick={toggleFullscreen} title="Fullscreen (F)">
              {fullscreen ? (
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
                  <polyline points="8,3 3,3 3,8"/><polyline points="21,8 21,3 16,3"/>
                  <polyline points="3,16 3,21 8,21"/><polyline points="16,21 21,21 21,16"/>
                </svg>
              ) : (
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
                  <polyline points="15,3 21,3 21,9"/><polyline points="9,21 3,21 3,15"/>
                  <line x1="21" y1="3" x2="14" y2="10"/><line x1="3" y1="21" x2="10" y2="14"/>
                </svg>
              )}
            </button>
          </div>

        </div>
      </div>
    </div>
  );
}
