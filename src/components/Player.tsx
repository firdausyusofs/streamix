import { useEffect, useRef } from "react";

interface PlayerProps {
  streamUrl: string;
  title: string;
  onClose: () => void;
}

export function Player({ streamUrl, title, onClose }: PlayerProps) {
  const videoRef = useRef<HTMLVideoElement>(null);

  // Allow the user to press 'Escape' to close the player
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  return (
    <div
      className="player-overlay"
      style={{
        position: "fixed",
        inset: 0, // Shorthand for top:0, left:0, right:0, bottom:0
        backgroundColor: "#000",
        zIndex: 9999,
        display: "flex",
        flexDirection: "column"
      }}
    >
      {/* Top Bar (Title and Close Button) */}
      <div style={{
        position: "absolute",
        top: 0,
        left: 0,
        right: 0,
        padding: "24px 40px",
        background: "linear-gradient(to bottom, rgba(0,0,0,0.9) 0%, rgba(0,0,0,0) 100%)",
        display: "flex",
        justifyContent: "space-between",
        alignItems: "center",
        zIndex: 10,
        pointerEvents: "none" // Lets clicks pass through to the video timeline if needed
      }}>
        <h2 style={{
          color: "white",
          margin: 0,
          fontSize: "1.25rem",
          fontWeight: 600,
          textShadow: "0 2px 4px rgba(0,0,0,0.8)",
          pointerEvents: "auto"
        }}>
          {title}
        </h2>

        <button
          onClick={onClose}
          style={{
            pointerEvents: "auto", // Re-enables clicking for just the button
            background: "rgba(255,255,255,0.1)",
            border: "1px solid rgba(255,255,255,0.2)",
            color: "white",
            padding: "8px 16px",
            borderRadius: "8px",
            cursor: "pointer",
            backdropFilter: "blur(8px)",
            fontSize: "1rem",
            fontWeight: "bold",
            transition: "background 0.2s ease"
          }}
          onMouseEnter={(e) => e.currentTarget.style.background = "rgba(255,255,255,0.2)"}
          onMouseLeave={(e) => e.currentTarget.style.background = "rgba(255,255,255,0.1)"}
        >
          ✕ Close
        </button>
      </div>

      {/* Native HTML5 Video Player
        Your Rust axum server will stream the torrent bytes directly into this src!
      */}
      <video
        ref={videoRef}
        src={streamUrl}
        controls
        autoPlay
        style={{
          width: "100%",
          height: "100%",
          outline: "none",
          backgroundColor: "#000"
        }}
      />
    </div>
  );
}
