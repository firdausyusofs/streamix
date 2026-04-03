import { Clapperboard, Search, User, X } from "lucide-react";

interface AppHeaderProps {
  query?: string;
  onQueryChange?: (q: string) => void;
  searchPlaceholder?: string;
}

export function AppHeader({ query = "", onQueryChange, searchPlaceholder = "Search…" }: AppHeaderProps) {
  return (
    <header className="app-header">
      {/* Left — branding */}
      <div className="header-brand">
        <Clapperboard size={24} className="brand-icon" />
        <span className="header-brand-name">Stream<span className="brand-accent">ix</span></span>
      </div>

      {/* Center — search */}
      <div className="header-search">
        <Search size={15} className="header-search-icon" />
        <input
          className="header-search-input"
          type="text"
          placeholder={searchPlaceholder}
          value={query}
          onChange={e => onQueryChange?.(e.target.value)}
        />
        {query && (
          <button className="header-search-clear" onClick={() => onQueryChange?.("")}>
            <X size={13} />
          </button>
        )}
      </div>

      {/* Right — profile (placeholder) */}
      <button className="header-profile-btn" title="Profile">
        <User size={18} />
      </button>
    </header>
  );
}
