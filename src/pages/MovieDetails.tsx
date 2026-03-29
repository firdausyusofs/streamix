import { useLocation, useNavigate } from "react-router-dom";
import { MetaPreview, Stream } from "../types";
import { useEffect, useMemo, useState } from "react";
import { fetchStreams } from "../api/stremio";

export function MovieDetails() {
  const location = useLocation();
  const navigate = useNavigate();

  const movie = location.state?.movie as MetaPreview;

  const [streams, setStreams] = useState<Stream[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);

  const [selectedAddon, setSelectedAddon] = useState<string>("All");

  useEffect(() => {
    if (!movie) return;

    setLoading(true);
    setStreams([]);
    setError(null);
    setSelectedAddon("All");

    fetchStreams(movie.type, movie.id)
      .then(res => setStreams(res || []))
      .catch(err => setError(err.message))
      .finally(() => setLoading(false));
  }, [movie]);

  const uniqueAddons = useMemo(() => {
    const names = streams.map(s => s.addonName).filter(Boolean) as string[];
    return ["All", ...Array.from(new Set(names))];
  }, [streams]);

  const filteredStreams = useMemo(() => {
    if (selectedAddon === "All") return streams;
    return streams.filter(s => s.addonName === selectedAddon);
  }, [streams, selectedAddon]);

  if (!movie) return <div className="status-screen">No movie found. Please go back and select a movie.</div>;

  return (
    <div className="details-page">

      <div
        className="hero-banner"
        style={{ backgroundImage: `url(${movie.background || movie.poster})` }}
      >
        <div className="hero-gradient"></div>
      </div>

      <div className="details-content">

        <button
          onClick={() => navigate(-1)}
          style={{
            background: "rgba(0,0,0,0.5)", border: "1px solid rgba(255,255,255,0.1)",
            color: "white", padding: "8px 16px", borderRadius: "8px",
            cursor: "pointer", width: "fit-content", backdropFilter: "blur(8px)"
          }}
        >
          ← Back
        </button>

        <div className="details-header-grid">
          <img src={movie.poster} alt={movie.name} className="details-poster" />
          <div className="details-info">
            {movie.logo ? (
              <img src={movie.logo} alt={movie.name} className="movie-logo" />
            ) : (
              <h1 className="movie-title-text">{movie.name}</h1>
            )}

            <div className="metadata-row">
              <span>{movie.releaseInfo}</span>
              <span className="metadata-dot">●</span>
              <span>{movie.runtime || "N/A"}</span>
              {movie.genres && movie.genres.length > 0 && (
                <>
                  <span className="metadata-dot">●</span>
                  <span>{movie.genres.slice(0, 3).join(", ")}</span>
                </>
              )}
            </div>

            <p className="details-description">{movie.description}</p>

            {movie.cast && movie.cast.length > 0 && (
              <p style={{ color: "#94a3b8", marginTop: "16px", textShadow: "0 2px 4px rgba(0,0,0,0.8)" }}>
                <strong>Starring:</strong> {movie.cast.slice(0, 4).join(", ")}
              </p>
            )}
          </div>
        </div>

        <div className="streams-section" style={{ marginTop: "24px", position: "relative", zIndex: 10 }}>
          <h2 style={{ borderBottom: "1px solid rgba(255,255,255,0.1)", paddingBottom: "12px", marginBottom: "20px" }}>
            Streams {filteredStreams.length > 0 && <span style={{ color: "#3b82f6" }}>({filteredStreams.length})</span>}
          </h2>

          {loading && <p style={{ color: "#94a3b8", fontSize: "1.1rem" }}>Aggregating streams from your addons...</p>}
          {error && <p className="error">{error}</p>}

          {!loading && uniqueAddons.length > 1 && (
            <div style={{ display: "flex", gap: "8px", marginBottom: "20px", flexWrap: "wrap" }}>
              {uniqueAddons.map(addon => (
                <button
                  key={addon}
                  onClick={() => setSelectedAddon(addon)}
                  style={{
                    padding: "6px 16px",
                    borderRadius: "20px",
                    fontSize: "0.9rem",
                    fontWeight: "600",
                    cursor: "pointer",
                    transition: "all 0.2s ease",
                    border: selectedAddon === addon ? "1px solid #3b82f6" : "1px solid rgba(255,255,255,0.1)",
                    background: selectedAddon === addon ? "rgba(59, 130, 246, 0.2)" : "rgba(30, 41, 59, 0.5)",
                    color: selectedAddon === addon ? "#60a5fa" : "#cbd5e1",
                    backdropFilter: "blur(8px)"
                  }}
                  onMouseEnter={(e) => {
                    if (selectedAddon !== addon) e.currentTarget.style.background = "rgba(255,255,255,0.1)";
                  }}
                  onMouseLeave={(e) => {
                    if (selectedAddon !== addon) e.currentTarget.style.background = "rgba(30, 41, 59, 0.5)";
                  }}
                >
                  {addon}
                </button>
              ))}
            </div>
          )}

          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(300px, 1fr))", gap: "16px" }}>
            {filteredStreams.length === 0 && !loading && !error && <p>No streams found for this content.</p>}

            {filteredStreams.map((stream, idx) => (
              <div
                key={idx}
                onClick={() => alert(`Starting torrent: ${stream.infoHash}`)}
                style={{
                  background: "rgba(30, 41, 59, 0.7)", backdropFilter: "blur(12px)",
                  padding: "16px", borderRadius: "12px", cursor: "pointer",
                  border: "1px solid rgba(255,255,255,0.05)",
                  transition: "transform 0.15s ease, background 0.15s ease",
                  display: "flex", flexDirection: "column", gap: "8px",
                  // 1. Ensure the parent container doesn't accidentally stretch
                  overflow: "hidden"
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.transform = "scale(1.02)";
                  e.currentTarget.style.background = "rgba(51, 65, 85, 0.9)";
                  e.currentTarget.style.borderColor = "rgba(255,255,255,0.2)";
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.transform = "scale(1)";
                  e.currentTarget.style.background = "rgba(30, 41, 59, 0.7)";
                  e.currentTarget.style.borderColor = "rgba(255,255,255,0.05)";
                }}
              >
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: "8px" }}>
                  <h4 style={{ margin: 0, color: "white", fontSize: "1.05rem", wordBreak: "break-word" }}>
                    {stream.name || "Torrent"}
                  </h4>
                  <span style={{ fontSize: "0.75rem", color: "#e2e8f0", background: "#3b82f6", padding: "2px 8px", borderRadius: "12px", fontWeight: "bold", whiteSpace: "nowrap" }}>
                    {stream.addonName}
                  </span>
                </div>

                {/* 2. THE FIX: Force the long torrent strings to break! */}
                <span style={{
                  fontSize: "0.85rem",
                  color: "#94a3b8",
                  whiteSpace: "pre-wrap",
                  lineHeight: "1.4",
                  wordBreak: "break-word",
                  overflowWrap: "anywhere"
                }}>
                  {stream.title}
                </span>

              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
};
