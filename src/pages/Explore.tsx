import { useCallback, useEffect, useRef, useState } from "react";
import { MetaItem } from "../types";
import { MetaCard } from "../components/MetaCard";
import { AppHeader } from "../components/AppHeader";
import { fetchHomeCatalogs } from "../api/stremio";
import { useNavigate, useSearchParams } from "react-router";
import { AlertTriangle } from "lucide-react";

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

export function Explore() {
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();
  const type = (searchParams.get("type") as "movie" | "series") || "movie";

  const [catalogs, setCatalogs] = useState<{ movies: MetaItem[]; series: MetaItem[] }>({ movies: [], series: [] });
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [gridRef] = useCardsPerRow();
  const [query, setQuery] = useState("");

  const items = (type === "movie" ? catalogs.movies : catalogs.series)
    .filter(item => item.name.toLowerCase().includes(query.toLowerCase()));

  const loadCatalogs = useCallback(() => {
    setLoading(true);
    setError(null);
    fetchHomeCatalogs()
      .then(setCatalogs)
      .catch(err => setError(err.message))
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => { loadCatalogs(); }, [loadCatalogs]);

  const handleItemClick = (meta: MetaItem) => {
    navigate(`/meta/${meta.id}`, { state: { meta } });
  };

  if (loading) return (
    <div className="status-screen">
      <div className="status-card">
        <div className="spinner" />
        <h3>Loading catalog</h3>
        <p>Fetching titles…</p>
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
        onQueryChange={q => { setQuery(q); }}
        searchPlaceholder="Search movies & series…"
      />

      <div className="explore-toolbar">
        <select
          className="type-dropdown"
          value={type}
          onChange={e => setSearchParams({ type: e.target.value })}
        >
          <option value="movie">Movies</option>
          <option value="series">Series</option>
        </select>
      </div>

      <div className="meta-grid explore-grid" ref={gridRef}>
        {items.map((item) => (
          <MetaCard key={item.id} meta={item} onClick={handleItemClick} />
        ))}
      </div>
    </div>
  );
}
