import { useCallback, useEffect, useRef, useState } from "react";
import Hls from "hls.js";

interface SubtitleTrack { id: number; name: string; lang?: string; }
interface AudioTrack    { id: number; name: string; lang?: string; }

interface PlayerProps {
  streamUrl: string;
  title: string;
  onClose: () => void;
  /** Known duration in seconds (e.g. parsed from meta.runtime). Takes priority over HLS detection. */
  duration?: number;
}

function fmt(sec: number): string {
  if (!isFinite(sec) || isNaN(sec) || sec < 0) return "0:00";
  const h = Math.floor(sec / 3600);
  const m = Math.floor((sec % 3600) / 60);
  const s = Math.floor(sec % 60);
  if (h > 0) return `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
  return `${m}:${String(s).padStart(2, "0")}`;
}

export function Player({ streamUrl, title, onClose, duration: propDuration }: PlayerProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const videoRef     = useRef<HTMLVideoElement>(null);
  const hlsRef       = useRef<Hls | null>(null);
  const seekBarRef   = useRef<HTMLDivElement>(null);
  const hideTimer    = useRef<ReturnType<typeof setTimeout> | null>(null);

  const [playing,     setPlaying]     = useState(false);
  const [currentTime, setCurrentTime] = useState(0);
  const [duration,    setDuration]    = useState(propDuration ?? 0);
  const [buffered,    setBuffered]    = useState(0);
  const [volume,      setVolume]      = useState(1);
  const [muted,       setMuted]       = useState(false);
  const [visible,     setVisible]     = useState(true);
  const [fullscreen,  setFullscreen]  = useState(false);
  const [subMenu,     setSubMenu]     = useState(false);
  const [audioMenu,   setAudioMenu]   = useState(false);
  const [subtitles,   _setSubtitles]  = useState<SubtitleTrack[]>([]);
  const [activeSub,   setActiveSub]   = useState(-1);
  const [audios,      _setAudios]     = useState<AudioTrack[]>([]);
  const [activeAudio, setActiveAudio] = useState(0);  const resetHide = useCallback(() => {
    setVisible(true);
    if (hideTimer.current) clearTimeout(hideTimer.current);
    hideTimer.current = setTimeout(() => setVisible(false), 3000);
  }, []);

  const toggleFullscreen = useCallback(() => {
    if (!containerRef.current) return;
    if (!document.fullscreenElement) containerRef.current.requestFullscreen();
    else document.exitFullscreen();
  }, []);

  useEffect(() => {
    const video = videoRef.current;
    if (!video) return;

    const onTime = () => {
      setCurrentTime(video.currentTime);
      if (video.buffered.length > 0)
        setBuffered(video.buffered.end(video.buffered.length - 1));
    };
    const onPlay   = () => setPlaying(true);
    const onPause  = () => setPlaying(false);
    const onVol    = () => { setVolume(video.volume); setMuted(video.muted); };
    const onDur    = () => {
      if (isFinite(video.duration) && video.duration > 0 && !propDuration)
        setDuration(video.duration);
    };

    video.addEventListener("timeupdate",     onTime);
    video.addEventListener("play",           onPlay);
    video.addEventListener("pause",          onPause);
    video.addEventListener("volumechange",   onVol);
    video.addEventListener("durationchange", onDur);

    video.src = streamUrl;

    video.addEventListener("loadedmetadata", () => {
      video.play().catch(e => console.warn("Autoplay blocked:", e));
    });

    // let hls: Hls | null = null;

    // if (Hls.isSupported()) {
    //   hls = new Hls({ maxBufferLength: 30, startLevel: -1, startPosition: 0 });
    //   hlsRef.current = hls;
    //   hls.loadSource(streamUrl);
    //   hls.attachMedia(video);

    //   hls.on(Hls.Events.MANIFEST_PARSED, () => {
    //     video.play().catch(e => console.warn("Autoplay blocked:", e));
    //   });

    //   // On-the-fly HLS transcoding reports Infinity for duration.
    //   // LEVEL_LOADED gives us totalduration by summing all manifest segments.
    //   hls.on(Hls.Events.LEVEL_LOADED, (_, data) => {
    //     const total = data.details.totalduration;
    //     if (total > 0 && isFinite(total) && !propDuration)
    //       setDuration(prev => (prev > 0 ? prev : total));
    //   });

    //   hls.on(Hls.Events.SUBTITLE_TRACKS_UPDATED, (_, data) => {
    //     setSubtitles(data.subtitleTracks.map((t, i) => ({
    //       id: i, name: t.name || t.lang || `Subtitle ${i + 1}`, lang: t.lang,
    //     })));
    //   });
    //   hls.on(Hls.Events.SUBTITLE_TRACK_SWITCH, (_, data) => setActiveSub(data.id));

    //   hls.on(Hls.Events.AUDIO_TRACKS_UPDATED, (_, data) => {
    //     setAudios(data.audioTracks.map((t, i) => ({
    //       id: i, name: t.name || t.lang || `Audio ${i + 1}`, lang: t.lang,
    //     })));
    //   });
    //   hls.on(Hls.Events.AUDIO_TRACK_SWITCHED, (_, data) => setActiveAudio(data.id));

    //   hls.on(Hls.Events.ERROR, (_, data) => {
    //     if (data.fatal && data.type === Hls.ErrorTypes.MEDIA_ERROR)
    //       hls!.recoverMediaError();
    //   });
    // } else if (video.canPlayType("application/vnd.apple.mpegurl")) {
    //   video.src = streamUrl;
    //   video.addEventListener("loadedmetadata", () => video.play());
    // }

    const onKey = (e: KeyboardEvent) => {
      const tag = (e.target as HTMLElement).tagName;
      if (tag === "INPUT" || tag === "TEXTAREA") return;
      switch (e.key) {
        case "Escape":      onClose(); return;
        case " ": case "k": e.preventDefault(); video.paused ? video.play() : video.pause(); break;
        case "f": case "F": e.preventDefault(); toggleFullscreen(); break;
        case "ArrowRight":  e.preventDefault(); video.currentTime = Math.min(video.currentTime + 10, video.duration || 0); break;
        case "ArrowLeft":   e.preventDefault(); video.currentTime = Math.max(video.currentTime - 10, 0); break;
        case "ArrowUp":     e.preventDefault(); video.volume = Math.min(video.volume + 0.1, 1); break;
        case "ArrowDown":   e.preventDefault(); video.volume = Math.max(video.volume - 0.1, 0); break;
        case "m": case "M": video.muted = !video.muted; break;
      }
      resetHide();
    };
    const onFsChange = () => setFullscreen(!!document.fullscreenElement);

    window.addEventListener("keydown", onKey);
    document.addEventListener("fullscreenchange", onFsChange);
    resetHide();

    return () => {
      window.removeEventListener("keydown", onKey);
      document.removeEventListener("fullscreenchange", onFsChange);
      video.removeEventListener("timeupdate", onTime);
      video.removeEventListener("play", onPlay);
      video.removeEventListener("pause", onPause);
      video.removeEventListener("volumechange", onVol);
      video.removeEventListener("durationchange", onDur);
      if (hideTimer.current) clearTimeout(hideTimer.current);

      // Properly abort the HTTP stream connection so the server-side FFmpeg
      // process receives the closed-pipe signal immediately.  Without this,
      // React StrictMode's double-invoke causes the browser to open a second
      // connection to the same URL while the first FFmpeg process is still
      // running, resulting in duplicate "Broken pipe" errors in the console.
      video.pause();
      video.removeAttribute("src");
      video.load();
    };
  }, [streamUrl, onClose, toggleFullscreen, resetHide, propDuration]);

  const togglePlay = useCallback(() => {
    const v = videoRef.current;
    if (!v) return;
    v.paused ? v.play() : v.pause();
  }, []);

  const handleSeek = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    const v   = videoRef.current;
    const bar = seekBarRef.current;
    if (!v || !bar || duration <= 0) return;
    const rect = bar.getBoundingClientRect();
    v.currentTime = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width)) * duration;
  }, [duration]);

  const handleVolume = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const v = videoRef.current;
    if (!v) return;
    const val = Number(e.target.value);
    v.volume = val;
    v.muted  = val === 0;
  }, []);

  const toggleMute = useCallback(() => {
    const v = videoRef.current;
    if (v) v.muted = !v.muted;
  }, []);

  const selectSub = useCallback((id: number) => {
    if (hlsRef.current) hlsRef.current.subtitleTrack = id;
    setActiveSub(id);
    setSubMenu(false);
  }, []);

  const selectAudio = useCallback((id: number) => {
    if (hlsRef.current) hlsRef.current.audioTrack = id;
    setActiveAudio(id);
    setAudioMenu(false);
  }, []);

  const progressPct = duration > 0 ? (currentTime / duration) * 100 : 0;
  const bufferedPct = duration > 0 ? (buffered  / duration) * 100 : 0;
  const effectiveVol = muted ? 0 : volume;

  return (
    <div
      ref={containerRef}
      className="player-container"
      onMouseMove={resetHide}
      onClick={() => { setSubMenu(false); setAudioMenu(false); }}
      style={{ cursor: visible ? "default" : "none" }}
    >
      <video ref={videoRef} className="player-video" onClick={togglePlay} />

      {/* ── Top bar ─────────────────────────────────── */}
      <div className={`player-top${visible ? " player-ui-on" : ""}`}>
        <button className="player-back" onClick={onClose}>
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
            <div className="player-seek-buf"  style={{ width: `${bufferedPct}%` }} />
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
                {effectiveVol === 0 ? (
                  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
                    <polygon points="11,5 6,9 2,9 2,15 6,15 11,19" fill="currentColor" stroke="none"/>
                    <line x1="23" y1="9" x2="17" y2="15"/><line x1="17" y1="9" x2="23" y2="15"/>
                  </svg>
                ) : effectiveVol < 0.5 ? (
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
                value={effectiveVol}
                style={{ "--vol": `${effectiveVol * 100}%` } as React.CSSProperties}
                onChange={handleVolume}
                onClick={e => e.stopPropagation()}
              />
            </div>

            <span className="player-time">
              {fmt(currentTime)}
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
                        {t.name}{t.lang ? ` (${t.lang})` : ""}
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
                        {t.name}{t.lang ? ` (${t.lang})` : ""}
                      </button>
                    ))}
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
