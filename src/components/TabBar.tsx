import { useLocation, useNavigate } from "react-router";
import { Home, Compass, Puzzle } from "lucide-react";

const tabs = [
  { label: "Home", path: "/", icon: Home },
  { label: "Explore", path: "/explore", icon: Compass },
  { label: "Addons", path: "/addons", icon: Puzzle },
];

export function TabBar() {
  const navigate = useNavigate();
  const location = useLocation();

  return (
    <nav className="tab-bar">
      {tabs.map(({ label, path, icon: Icon }) => {
        const active = location.pathname === path;
        return (
          <button
            key={path}
            className={`tab-item ${active ? "tab-active" : ""}`}
            onClick={() => navigate(path)}
          >
            <Icon size={22} />
            <span>{label}</span>
          </button>
        );
      })}
    </nav>
  );
}
