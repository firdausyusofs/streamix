import { MetaItem } from "../types";

interface MetaCardProps {
  meta: MetaItem;
  onClick: (meta: MetaItem) => void;
};

export function MetaCard({ meta, onClick }: MetaCardProps) {
return (
    <div className="meta-card" onClick={() => onClick(meta)}>
      <div className="poster-wrapper">
        {meta.poster ? (
            <img src={meta.poster} alt={meta.name} loading="lazy" />
        ) : (
            <div className="poster-placeholder">{meta.name}</div>
        )}
      </div>
      <div className="meta-info">
        <h3 title={meta.name}>{meta.name}</h3>
        {/* <span>
          {meta.releaseInfo} {meta.genres?.length ? `• ${meta.genres.slice(0, 2).join(", ")}` : ""}
        </span> */}
      </div>
    </div>
  );
};
