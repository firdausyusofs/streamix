import { Puzzle } from "lucide-react";
import { AppHeader } from "../components/AppHeader";

export function Addons() {
  return (
    <div className="page-content">
      <AppHeader />
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
