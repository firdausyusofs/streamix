import { useState } from "react";
import { Film } from "lucide-react";
import { MetaItem } from "../types";

interface MetaCardProps {
  meta: MetaItem;
  onClick: (meta: MetaItem) => void;
};

export function MetaCard({ meta, onClick }: MetaCardProps) {
  const [imgFailed, setImgFailed] = useState(false);
  const showPlaceholder = !meta.poster || imgFailed;

  return (
    <div className="meta-card" onClick={() => onClick(meta)}>
      <div className="poster-wrapper">
        {!showPlaceholder && (
          <img
            src={meta.poster!}
            alt={meta.name}
            loading="lazy"
            onError={() => setImgFailed(true)}
          />
        )}
        {showPlaceholder && (
          <div className="poster-placeholder">
            <Film size={48} className="poster-placeholder-icon" />
          </div>
        )}
      </div>
      <div className="meta-info">
        <h3 title={meta.name}>{meta.name}</h3>
      </div>
    </div>
  );
};
