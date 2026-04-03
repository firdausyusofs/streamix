import { useCallback, useEffect, useRef, useState } from "react";
import { MetaItem } from "../types";
import { MetaCard } from "../components/MetaCard";
import { AppHeader } from "../components/AppHeader";
import { fetchHomeCatalogs, HomeCatalog } from "../api/stremio";
import { useNavigate } from "react-router";
import { AlertTriangle, ChevronRight, Flame, Tv } from "lucide-react";

const MIN_CARD_WIDTH = 220;
const GRID_GAP = 24;

function useCardsPerRow(): [React.RefCallback<HTMLDivElement>, number] {
  const [count, setCount] = useState(9);
  const observerRef = useRef<ResizeObserver | null>(null);

  const ref = useCallback((el: HTMLDivElement | null) => {
    observerRef.current?.disconnect();
    observerRef.current = null;
    if (!el) return;
    const observer = new ResizeObserver(([entry]) => {
      const width = entry.contentRect.width;
      const fit = Math.floor((width + GRID_GAP) / (MIN_CARD_WIDTH + GRID_GAP));
      setCount(Math.max(1, fit));
    });
    observer.observe(el);
    observerRef.current = observer;
  }, []);

  return [ref, count];
}

export function Catalog() {
  const navigate = useNavigate();
  const [catalogs, setCatalogs] = useState<HomeCatalog>({ movies: [], series: [] });
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);
  const [moviesGridRef, moviesPerRow] = useCardsPerRow();
  const [seriesGridRef, seriesPerRow] = useCardsPerRow();
  const [query, setQuery] = useState("");

  const loadCatalogs = useCallback(() => {
    setLoading(true);
    setError(null);
    fetchHomeCatalogs()
      .then(setCatalogs)
      .catch(err => setError(err.message))
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => { loadCatalogs(); }, [loadCatalogs]);

  const handleMovieClick = (meta: MetaItem) => {
    navigate(`/meta/${meta.id}`, { state: { meta } });
  };

  if (loading) return (
    <div className="status-screen">
      <div className="status-card">
        <div className="spinner" />
        <h3>Loading your catalog</h3>
        <p>Fetching addons and titles…</p>
      </div>
    </div>
  );

  if (error) return (
    <div className="status-screen">
      <div className="status-card error-card">
        <AlertTriangle className="status-icon" size={28} />
        <h3>Couldn't load catalog</h3>
        <p>{error}</p>
        <button className="btn-retry" onClick={loadCatalogs}>Try Again</button>
      </div>
    </div>
  );

return (
    <div className="page-content">
      <AppHeader
        query={query}
        onQueryChange={setQuery}
        searchPlaceholder="Search movies & series…"
      />

      {/* TOP MOVIES SECTION */}
      {catalogs.movies.length > 0 && (
        <section className="catalog-section">
          <div className="section-header">
            <h2><Flame size={20} className="section-icon" /> Top Movies</h2>
            <button className="see-all-btn" onClick={() => navigate("/explore?type=movie")}>
              See All <ChevronRight size={14} />
            </button>
          </div>
          <div className="meta-grid" ref={moviesGridRef}>
            {catalogs.movies
              .filter(m => m.name.toLowerCase().includes(query.toLowerCase()))
              .slice(0, moviesPerRow)
              .map((movie) => (
                <MetaCard key={movie.id} meta={movie} onClick={handleMovieClick} />
              ))}
          </div>
        </section>
      )}

      {/* TOP SERIES SECTION */}
      {catalogs.series.length > 0 && (
        <section className="catalog-section" style={{ marginTop: "40px" }}>
          <div className="section-header">
            <h2><Tv size={20} className="section-icon" /> Top Series</h2>
            <button className="see-all-btn" onClick={() => navigate("/explore?type=series")}>
              See All <ChevronRight size={14} />
            </button>
          </div>
          <div className="meta-grid" ref={seriesGridRef}>
            {catalogs.series
              .filter(s => s.name.toLowerCase().includes(query.toLowerCase()))
              .slice(0, seriesPerRow)
              .map((series) => (
                <MetaCard key={series.id} meta={series} onClick={handleMovieClick} />
              ))}
          </div>
        </section>
      )}
    </div>
  );
};
