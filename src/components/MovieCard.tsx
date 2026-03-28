import { MetaPreview } from "../types";

interface MovieCardProps {
  movie: MetaPreview;
  onClick: (movie: MetaPreview) => void;
};

export function MovieCard({ movie, onClick }: MovieCardProps) {
return (
    <div className="movie-card" onClick={() => onClick(movie)}>
      <div className="poster-wrapper">
        {movie.poster ? (
            <img src={movie.poster} alt={movie.name} loading="lazy" />
        ) : (
            <div className="poster-placeholder">{movie.name}</div>
        )}
      </div>
      <div className="movie-info">
        <h3 title={movie.name}>{movie.name}</h3>
        <span>
          {movie.year} {movie.genre?.length ? `• ${movie.genre.slice(0, 2).join(", ")}` : ""}
        </span>
      </div>
    </div>
  );
};
