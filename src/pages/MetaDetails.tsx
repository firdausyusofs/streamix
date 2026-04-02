import { useLocation, useNavigate } from "react-router-dom";
import { MetaItem, Stream, Video } from "../types";
import { useEffect, useMemo, useState } from "react";
import { fetchStreams, playStreamForMpv } from "../api/stremio";
import { Player } from "../components/Player";

/** Parse "2h 30min", "120 min", "1 hr 45 m" → seconds */
function parseRuntime(runtime?: string | null): number {
  if (!runtime) return 0;
  const h = runtime.match(/(\d+)\s*h/i)?.[1];
  const m = runtime.match(/(\d+)\s*m/i)?.[1];
  return (h ? +h * 3600 : 0) + (m ? +m * 60 : 0);
}

export function MetaDetails() {
  const location = useLocation();
  const navigate = useNavigate();

  const meta = location.state?.meta as MetaItem;

  const [streams, setStreams] = useState<Stream[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);

  const [selectedSeason, setSelectedSeason] = useState<number | null>(null);
  const [selectedEpisode, setSelectedEpisode] = useState<Video | null>(null);

  const [selectedAddon, setSelectedAddon] = useState<string>("All");

  const [playerSession, setPlayerSession] = useState<{
    url: string | null;
    logo?: string;
    poster?: string;
    title: string;
  } | null>(null);
  const [streamError, setStreamError] = useState<string | null>(null);

  useEffect(() => {
    if (!streamError) return;
    const t = setTimeout(() => setStreamError(null), 5000);
    return () => clearTimeout(t);
  }, [streamError]);

  const availableSeasons = useMemo(() => {
    if (meta.type !== "series" || !meta.videos) return [];
    const seasons = meta.videos.map(v => v.season).filter(s => s != null) as number[];
    return Array.from(new Set(seasons)).sort((a, b) => a - b);
  }, [meta]);

  useEffect(() => {
    if (availableSeasons.length > 0) {
      setSelectedSeason(availableSeasons[0]);
    }
  }, [availableSeasons]);

  const seasonEpisodes = useMemo(() => {
    if (!meta.videos) return [];
    return meta.videos.filter(v => v.season === selectedSeason).sort((a, b) => ((a.episode || 0) - (b.episode || 0)));
  }, [meta, selectedSeason]);

  useEffect(() => {
    if (!meta) return;

    const idToFetch = meta.type == "series" ? selectedEpisode?.id : meta.id;

    if (!idToFetch) {
      setStreams([]);
      return;
    }

    setLoading(true);
    setStreams([]);
    setError(null);
    setSelectedAddon("All");

    fetchStreams(meta.type, idToFetch)
      .then(res => setStreams(res || []))
      .catch(err => setError(err.message))
      .finally(() => setLoading(false));
  }, [meta, selectedEpisode]);

  const uniqueAddons = useMemo(() => {
    const names = streams.map(s => s.addonName).filter(Boolean) as string[];
    return ["All", ...Array.from(new Set(names))];
  }, [streams]);

  const filteredStreams = useMemo(() => {
    if (selectedAddon === "All") return streams;
    return streams.filter(s => s.addonName === selectedAddon);
  }, [streams, selectedAddon]);

  const handleStreamClick = async (stream: Stream) => {
    setPlayerSession({
      url: null,
      logo: meta?.logo || undefined,
      poster: meta?.poster || undefined,
      title: meta?.name || "Playing Video",
    });
    try {
      const url = await playStreamForMpv(stream);
      setPlayerSession(prev => prev ? { ...prev, url } : null);
    } catch (err: any) {
      setPlayerSession(null);
      setStreamError(err.message || "Failed to start stream.");
    }
  };

  if (!meta) return <div className="status-screen">No movie found. Please go back and select a movie.</div>;

  return (
    <div className="details-page">

      {playerSession && (
        <Player
          streamUrl={playerSession.url}
          logo={playerSession.logo}
          poster={playerSession.poster}
          title={playerSession.title}
          onClose={() => setPlayerSession(null)}
          duration={parseRuntime(meta?.runtime)}
        />
      )}

      {streamError && (
        <div className="stream-error-toast">
          <span>⚠</span>
          <span>{streamError}</span>
          <button className="toast-close" onClick={() => setStreamError(null)}>✕</button>
        </div>
      )}

      <div
        className="hero-banner"
        style={{ backgroundImage: `url(${meta.background || meta.poster})` }}
      >
        <div className="hero-gradient" />
      </div>

      <div className="details-content">

        <button className="btn-back" onClick={() => navigate(-1)}>
          ← Back
        </button>

        <div className="details-header-grid">
          <img src={meta.poster} alt={meta.name} className="details-poster" />
          <div className="details-info">
            {meta.logo ? (
              <img src={meta.logo} alt={meta.name} className="meta-logo" />
            ) : (
              <h1 className="meta-title-text">{meta.name}</h1>
            )}

            <div className="metadata-row">
              <span>{meta.releaseInfo}</span>
              <span className="metadata-dot">●</span>
              <span>{meta.runtime || "N/A"}</span>
              {meta.genres && meta.genres.length > 0 && (
                <>
                  <span className="metadata-dot">●</span>
                  <span>{meta.genres.slice(0, 3).join(", ")}</span>
                </>
              )}
            </div>

            <p className="details-description">{meta.description}</p>

            {meta.cast && meta.cast.length > 0 && (
              <p className="details-starring">
                <strong>Starring:</strong> {meta.cast.slice(0, 4).join(", ")}
              </p>
            )}
          </div>
        </div>

        {meta.type === "series" && availableSeasons.length > 0 && (
          <div className="episodes-section" style={{ marginTop: "40px" }}>
            <h2>Episodes</h2>

            {/* Season Selector Pills */}
            <div className="addon-filters" style={{ marginBottom: "20px" }}>
              {availableSeasons.map(season => (
                <button
                  key={season}
                  className={`addon-pill ${selectedSeason === season ? "active" : ""}`}
                  onClick={() => {
                    setSelectedSeason(season);
                    setSelectedEpisode(null); // Reset streams when changing season
                  }}
                >
                  Season {season}
                </button>
              ))}
            </div>

            {/* Episode List */}
            <div className="streams-grid" style={{ gridTemplateColumns: "repeat(auto-fill, minmax(250px, 1fr))" }}>
              {seasonEpisodes.map((ep) => (
                <div
                  key={ep.id}
                  className="stream-card"
                  style={{
                    border: selectedEpisode?.id === ep.id ? "2px solid #3b82f6" : "1px solid rgba(255,255,255,0.05)",
                    opacity: selectedEpisode && selectedEpisode.id !== ep.id ? 0.6 : 1
                  }}
                  onClick={() => setSelectedEpisode(ep)}
                >
                  {/* Episode Thumbnail */}
                  <div style={{ aspectRatio: "16/9", background: "#0f172a", borderRadius: "6px", marginBottom: "12px", overflow: "hidden" }}>
                    {ep.thumbnail ? (
                      <img src={ep.thumbnail} alt={ep.title} style={{ width: "100%", height: "100%", objectFit: "cover" }} />
                    ) : (
                      <div style={{ display: "flex", alignItems: "center", justifyContent: "center", height: "100%", color: "#64748b" }}>No Image</div>
                    )}
                  </div>
                  <div className="stream-card-header">
                    <h4 className="stream-name" style={{ fontSize: "1rem" }}>
                      {ep.title || `Episode ${ep.episode || "N/A"}`}
                    </h4>
                  </div>
                  <span className="stream-title" style={{ color: "#e2e8f0" }}>{ep.released ? new Date(ep.released).toLocaleDateString('en-GB', {
                    day: '2-digit',
                    month: 'short',
                    year: 'numeric'
                  }) : "N/A"}</span>
                </div>
              ))}
            </div>
          </div>
        )}

        {(meta.type === "movie" || selectedEpisode) && (
          <div className="streams-section">
            <div className="streams-header">
              <h2>Streams</h2>
              {filteredStreams.length > 0 && (
                <span className="streams-count">{filteredStreams.length}</span>
              )}
            </div>

            {loading && (
              <div className="streams-grid">
                {Array.from({ length: 4 }).map((_, i) => (
                  <div key={i} className="skeleton-card">
                    <div className="skeleton-line short" />
                    <div className="skeleton-line medium" />
                    <div className="skeleton-line long" />
                  </div>
                ))}
              </div>
            )}

            {!loading && error && (
              <div className="error-banner">
                <span className="error-banner-icon">⚠</span>
                <span>{error}</span>
              </div>
            )}

            {!loading && !error && uniqueAddons.length > 1 && (
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
              {!loading && filteredStreams.length === 0 && !error && (
                <p className="streams-empty">No streams found for this content.</p>
              )}
              {!loading && filteredStreams.map((stream, idx) => (
                <div
                  key={idx}
                  className="stream-card"
                  onClick={() => handleStreamClick(stream)}
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
        )}

      </div>
    </div>
  );
};
