import { useEffect, useState } from "react";
import { MetaPreview } from "../types";
import { MovieCard } from "../components/MovieCard";
import { fetchHomeCatalogs, HomeCatalog } from "../api/stremio";
import { useNavigate } from "react-router";

export function Catalog() {
  const navigate = useNavigate();
  const [catalogs, setCatalogs] = useState<HomeCatalog>({ movies: [], series: [] });
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);

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
          <div className="movie-grid">
            {/* ONLY slice the first 9 movies! */}
            {catalogs.movies.slice(0, 9).map((movie) => (
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
          <div className="movie-grid">
            {/* ONLY slice the first 9 series! */}
            {catalogs.series.slice(0, 9).map((series) => (
              <MovieCard key={series.id} movie={series} onClick={handleMovieClick} />
            ))}
          </div>
        </section>
      )}
    </div>
  );
};
