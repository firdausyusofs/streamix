import { Puzzle } from "lucide-react";

export function Addons() {
  return (
    <div className="page-content">
      <header className="app-header">
        <div className="brand-container">
          <h2 className="page-heading"><Puzzle size={22} className="brand-icon" /> Addons</h2>
        </div>
      </header>
      <div className="addons-placeholder">
        <div className="addons-placeholder-icon">
          <Puzzle size={56} />
        </div>
        <h3>Addon Management</h3>
        <p>Manage your Stremio add-ons here. Coming soon.</p>
      </div>
    </div>
  );
}
