import { useCallback, useEffect, useRef, useState } from "react";
import { MetaPreview } from "../types";
import { MovieCard } from "../components/MovieCard";
import { fetchHomeCatalogs, HomeCatalog } from "../api/stremio";
import { useNavigate } from "react-router";

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

  useEffect(() => {
    fetchHomeCatalogs()
      .then(setCatalogs)
      .catch(err => setError(err.message))
      .finally(() => setLoading(false));
  }, []);

  const handleMovieClick = (movie: MetaPreview) => {
    navigate(`/movie/${movie.id}`, { state: { movie } });
  };

  const handleSeeAll = (type: "movies" | "series") => {
    alert(`See all ${type} - Not implemented yet!`);
  };

  if (loading) return <div className="status-screen">Loading Addons & Movies...</div>;
  if (error) return <div className="status-screen error">Error: {error}</div>;

return (
    <div className="page-content">
      <header className="app-header">
        <div className="brand-container">
          <h1>🍿 Stream<span className="brand-accent">ix</span></h1>
        </div>
      </header>

      {/* TOP MOVIES SECTION */}
      {catalogs.movies.length > 0 && (
        <section className="catalog-section">
          <div className="section-header">
            <h2>🔥 Top Movies</h2>
            <button className="see-all-btn" onClick={() => handleSeeAll("movies")}>
              See All
            </button>
          </div>
          <div className="movie-grid" ref={moviesGridRef}>
            {catalogs.movies.slice(0, moviesPerRow).map((movie) => (
              <MovieCard key={movie.id} movie={movie} onClick={handleMovieClick} />
            ))}
          </div>
        </section>
      )}

      {/* TOP SERIES SECTION */}
      {catalogs.series.length > 0 && (
        <section className="catalog-section" style={{ marginTop: "40px" }}>
          <div className="section-header">
            <h2>📺 Top Series</h2>
            <button className="see-all-btn" onClick={() => handleSeeAll("series")}>
              See All
            </button>
          </div>
          <div className="movie-grid" ref={seriesGridRef}>
            {catalogs.series.slice(0, seriesPerRow).map((series) => (
              <MovieCard key={series.id} movie={series} onClick={handleMovieClick} />
            ))}
          </div>
        </section>
      )}
    </div>
  );
};
