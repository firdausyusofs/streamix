import { useCallback, useEffect, useRef, useState } from "react";
import { AlertTriangle, Link, Puzzle, Trash2 } from "lucide-react";
import { AppHeader } from "../components/AppHeader";
import { getInstalledAddons, installAddon, removeAddon } from "../api/stremio";
import { InstalledAddon } from "../types";

function AddonCard({
  addon,
  onRemove,
  removing,
}: {
  addon: InstalledAddon;
  onRemove: () => void;
  removing: boolean;
}) {
  const { manifest } = addon;
  const [imgError, setImgError] = useState(false);

  return (
    <div className={`addon-card${removing ? " addon-card--removing" : ""}`}>
      <div className="addon-card-logo">
        {manifest.logo && !imgError ? (
          <img
            src={manifest.logo}
            alt={manifest.name}
            onError={() => setImgError(true)}
          />
        ) : (
          <Puzzle size={28} />
        )}
      </div>

      <div className="addon-card-info">
        <div className="addon-card-name">{manifest.name}</div>
        <div className="addon-card-version">v{manifest.version}</div>
        {manifest.description && (
          <div className="addon-card-description">{manifest.description}</div>
        )}
        <div className="addon-resource-badges">
          {(manifest.types || []).map((t) => (
            <span key={t} className="addon-resource-badge badge-type">
              {t}
            </span>
          ))}
        </div>
      </div>

      <button
        className="addon-remove-btn"
        title="Remove addon"
        onClick={onRemove}
        disabled={removing}
      >
        {removing ? <div className="spinner spinner--sm" /> : <Trash2 size={16} />}
      </button>
    </div>
  );
}

export function Addons() {
  const [addons, setAddons] = useState<InstalledAddon[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [query, setQuery] = useState("");

  const [urlInput, setUrlInput] = useState("");
  const [installing, setInstalling] = useState(false);
  const [installError, setInstallError] = useState<string | null>(null);

  const [removingUrl, setRemovingUrl] = useState<string | null>(null);
  const [removeError, setRemoveError] = useState<string | null>(null);
  const removeErrorTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const loadAddons = useCallback(() => {
    setLoading(true);
    setLoadError(null);
    getInstalledAddons()
      .then(setAddons)
      .catch((err) => setLoadError(String(err)))
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => { loadAddons(); }, [loadAddons]);

  const handleInstall = async () => {
    const url = urlInput.trim();
    if (!url) return;
    setInstalling(true);
    setInstallError(null);
    try {
      const addon = await installAddon(url);
      setAddons((prev) => [...prev, addon]);
      setUrlInput("");
    } catch (err) {
      setInstallError(String(err));
    } finally {
      setInstalling(false);
    }
  };

  const handleRemove = async (transportUrl: string) => {
    setRemovingUrl(transportUrl);
    setRemoveError(null);
    try {
      await removeAddon(transportUrl);
      setAddons((prev) => prev.filter((a) => a.transport_url !== transportUrl));
    } catch (err) {
      setRemoveError(String(err));
      if (removeErrorTimer.current) clearTimeout(removeErrorTimer.current);
      removeErrorTimer.current = setTimeout(() => setRemoveError(null), 5000);
    } finally {
      setRemovingUrl(null);
    }
  };

  const filtered = addons.filter((a) =>
    a.manifest.name.toLowerCase().includes(query.toLowerCase())
  );

  return (
    <div className="page-content">
      <AppHeader
        query={query}
        onQueryChange={setQuery}
        searchPlaceholder="Search installed addons…"
      />

      {/* ── Add Addon ── */}
      <section className="addons-install-section">
        <h2 className="page-heading">
          <Link size={20} className="section-icon" />
          Add Addon
        </h2>
        <p className="addons-install-hint">
          Paste a Stremio addon manifest URL to install it.
        </p>
        <div className="addon-url-form">
          <input
            className="addon-url-input"
            type="url"
            placeholder="https://example.com/manifest.json"
            value={urlInput}
            onChange={(e) => setUrlInput(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleInstall()}
            disabled={installing}
          />
          <button
            className="btn-install"
            onClick={handleInstall}
            disabled={installing || !urlInput.trim()}
          >
            {installing ? <div className="spinner spinner--sm" /> : "Install"}
          </button>
        </div>
        {installError && (
          <div className="error-banner" style={{ marginTop: 12 }}>
            <AlertTriangle size={16} className="error-banner-icon" />
            {installError}
          </div>
        )}
      </section>

      {/* ── Installed Addons ── */}
      <section className="addons-installed-section">
        <div className="section-header" style={{ marginBottom: 20 }}>
          <h2>
            <Puzzle size={18} className="section-icon" />
            Installed Addons
            {addons.length > 0 && (
              <span className="streams-count" style={{ marginLeft: 10 }}>
                {addons.length}
              </span>
            )}
          </h2>
        </div>

        {removeError && (
          <div className="error-banner" style={{ marginBottom: 16 }}>
            <AlertTriangle size={16} className="error-banner-icon" />
            {removeError}
          </div>
        )}

        {loading ? (
          <div className="addons-status">
            <div className="spinner" />
            <span>Loading addons…</span>
          </div>
        ) : loadError ? (
          <div className="status-card error-card" style={{ marginTop: 0 }}>
            <AlertTriangle className="status-icon" size={28} />
            <h3>Couldn't load addons</h3>
            <p>{loadError}</p>
            <button className="btn-retry" onClick={loadAddons}>Try Again</button>
          </div>
        ) : filtered.length === 0 ? (
          <div className="addons-empty">
            <div className="addons-placeholder-icon">
              <Puzzle size={32} />
            </div>
            <p>{query ? `No addons match "${query}"` : "No addons installed yet."}</p>
          </div>
        ) : (
          <div className="addons-grid">
            {filtered.map((addon) => (
              <AddonCard
                key={addon.transport_url}
                addon={addon}
                removing={removingUrl === addon.transport_url}
                onRemove={() => handleRemove(addon.transport_url)}
              />
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
