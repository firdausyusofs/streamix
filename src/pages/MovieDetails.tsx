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
        <div className="hero-gradient" />
      </div>

      <div className="details-content">

        <button className="btn-back" onClick={() => navigate(-1)}>
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
              <p className="details-starring">
                <strong>Starring:</strong> {movie.cast.slice(0, 4).join(", ")}
              </p>
            )}
          </div>
        </div>

        <div className="streams-section">
          <div className="streams-header">
            <h2>Streams</h2>
            {filteredStreams.length > 0 && (
              <span className="streams-count">{filteredStreams.length}</span>
            )}
          </div>

          {loading && <p className="streams-loading">Aggregating streams from your addons…</p>}
          {error && <p className="error">{error}</p>}

          {!loading && uniqueAddons.length > 1 && (
            <div className="addon-filters">
              {uniqueAddons.map(addon => (
                <button
                  key={addon}
                  className={`addon-pill${selectedAddon === addon ? " active" : ""}`}
                  onClick={() => setSelectedAddon(addon)}
                >
                  {addon}
                </button>
              ))}
            </div>
          )}

          <div className="streams-grid">
            {filteredStreams.length === 0 && !loading && !error && (
              <p className="streams-empty">No streams found for this content.</p>
            )}
            {filteredStreams.map((stream, idx) => (
              <div
                key={idx}
                className="stream-card"
                onClick={() => alert(`Starting torrent: ${stream.infoHash}`)}
              >
                <div className="stream-card-header">
                  <h4 className="stream-name">{stream.name || "Torrent"}</h4>
                  <span className="stream-badge">{stream.addonName}</span>
                </div>
                <span className="stream-title">{stream.title}</span>
              </div>
            ))}
          </div>
        </div>

      </div>
    </div>
  );
};
