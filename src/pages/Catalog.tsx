import { useEffect, useState } from "react";
import { MetaPreview } from "../types";
import { MovieCard } from "../components/MovieCard";
import { fetchDynamicCatalog } from "../api/stremio";

export function Catalog() {
  const [movies, setMovies] = useState<MetaPreview[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetchDynamicCatalog()
      .then(setMovies)
      .catch(err => setError(err.message))
      .finally(() => setLoading(false));
  }, []);

  if (loading) return <div className="status-screen">Loading Addons & Movies...</div>;
  if (error) return <div className="status-screen error">Error: {error}</div>;

  return (
    <div className="page-content">
      <header>
        <h1>🍿 Streamix Catalog</h1>
      </header>
      <div className="movie-grid">
        {movies.map(movie => (
          <MovieCard key={movie.id} movie={movie} onClick={(m) => console.log("Clicked movie:", m)} />
        ))}
      </div>
    </div>
  );
};
