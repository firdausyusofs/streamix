import { useEffect, useRef } from "react";
import Hls from "hls.js";

interface PlayerProps {
  streamUrl: string; // Now expects the .m3u8 URL!
  title: string;
  onClose: () => void;
}

export function Player({ streamUrl, title, onClose }: PlayerProps) {
  const videoRef = useRef<HTMLVideoElement>(null);

  useEffect(() => {
    const video = videoRef.current;
    if (!video) return;

    let hls: Hls;

    if (Hls.isSupported()) {
      hls = new Hls({
        // Tweak settings for faster startup
        maxBufferLength: 30,
        startLevel: -1,
        startPosition: 0
      });

      hls.loadSource(streamUrl);
      hls.attachMedia(video);

      hls.on(Hls.Events.MANIFEST_PARSED, () => {
        video.play().catch(e => console.warn("Autoplay blocked:", e));
      });

      hls.on(Hls.Events.ERROR, (event, data) => {
        if (data.fatal) {
          console.error("HLS Error:", data);
          // Try to recover from media errors
          if (data.type === Hls.ErrorTypes.MEDIA_ERROR) {
            hls.recoverMediaError();
          }
        }
      });
    } else if (video.canPlayType("application/vnd.apple.mpegurl")) {
      // Native Safari support
      video.src = streamUrl;
      video.addEventListener("loadedmetadata", () => {
        video.play();
      });
    }

    // Escape to close
    const handleKeyDown = (e: KeyboardEvent) => {
        if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", handleKeyDown);

    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      if (hls) hls.destroy();
    };
  }, [streamUrl, onClose]);

  return (
    <div className="player-overlay" style={{ position: "fixed", inset: 0, zIndex: 9999, backgroundColor: "black" }}>
      {/* Top Bar... */}
      <video ref={videoRef} controls style={{ width: "100%", height: "100%", outline: "none" }} />
    </div>
  );
}
