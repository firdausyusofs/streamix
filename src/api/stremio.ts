import { invoke } from "@tauri-apps/api/core";
import { AddonConfig, CatalogResponse, InstalledAddon, MetaPreview, Stream, StreamResponse } from "../types";

export async function fetchDynamicCatalog(): Promise<MetaPreview[]> {
  const config: AddonConfig = await invoke("get_installed_addons");

  if (!config.addons || config.addons.length === 0) {
    throw new Error("No addons installed");
  }

  const mainAddon = config.addons[0];
  const movieCatalog = mainAddon.manifest.catalogs.find(c => c.type === "movie");

  if (!movieCatalog) {
    throw new Error(`Addon ${mainAddon.manifest.name} does not have a movie catalog`);
  }

  const response: CatalogResponse = await invoke("fetch_catalog_from_addon", {
    manifestUrl: mainAddon.transport_url,
    itemType: movieCatalog.type,
    catalogId: movieCatalog.id,
  });

  return response.metas;
};

export async function fetchMovieStreams(itemType: string, id: string): Promise<Stream[]> {
  const config: AddonConfig = await invoke("get_installed_addons");

  const streamAddons = config.addons.filter((a: InstalledAddon) => {
    return a.manifest.resources.some((r) => {
      if (typeof r === "string") {
        return r === "stream" && a.manifest.types.includes(itemType);
      }

      if (typeof r === "object" && r.name === "stream") {
        if (r.types && !r.types.includes(itemType)) {
          return false;
        }
        return true;
      }

      return false;
    });
  });

  if (!streamAddons || streamAddons.length === 0) {
    throw new Error("No addon with stream resource found");
  }

  const fetchPromises = streamAddons.map(async (addon: InstalledAddon) => {
    try {
      const response: StreamResponse = await invoke("fetch_streams_from_addon", {
        manifestUrl: addon.transport_url,
        itemType,
        id: id,
      });

      return (response.streams || []).map((stream) => ({
        ...stream,
        addonName: addon.manifest.name,
      }));
    } catch (err) {
      return [];
    }
  });

  const results = await Promise.all(fetchPromises);

  const allStreams = results.flat();

  return allStreams;
};
