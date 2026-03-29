import { invoke } from "@tauri-apps/api/core";
import { AddonConfig, CatalogResponse, InstalledAddon, MetaItem, Stream, StreamResponse } from "../types";

export interface HomeCatalog {
  movies: MetaItem[];
  series: MetaItem[];
}

export async function fetchHomeCatalogs(): Promise<HomeCatalog> {
  const config: AddonConfig = await invoke("get_installed_addons");

  if (!config.addons || config.addons.length === 0) {
    throw new Error("No addons installed");
  }

  const mainAddon = config.addons[0];

  const movieCatalog = mainAddon.manifest.catalogs.find(c => c.type === "movie");
  const seriesCatalog = mainAddon.manifest.catalogs.find(c => c.type === "series");

  const fetchCat = async (catalog?: { type: string; id: string }) => {
    if (!catalog) return [];
    try {
      const response: CatalogResponse = await invoke("fetch_catalog_from_addon", {
        manifestUrl: mainAddon.transport_url,
        itemType: catalog.type,
        catalogId: catalog.id,
      });
      return (response.metas || []).map((meta) => ({
        ...meta,
        released: meta.released ? new Date(meta.released) : null,
        videos: meta.videos.map((video) => ({
          ...video,
          released: video.released ? new Date(video.released) : null,
        }))
      }));
    } catch (err) {
      return [];
    }
  };

  const [movies, series] = await Promise.all([
    fetchCat(movieCatalog),
    fetchCat(seriesCatalog)
  ]);

  return { movies, series };
};

export async function fetchStreams(itemType: string, id: string): Promise<Stream[]> {
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

  const fetchPromises = streamAddons.map(async (addon: InstalledAddon): Promise<Stream[]> => {
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

export async function playStream(stream: Stream) {
  try {
    const url: string = await invoke("play_stream_command", {
      stream: {
        url: stream.url,
        infoHash: stream.infoHash,
        fileIdx: stream.fileIdx
      }
    });
    return url;
  } catch (err) {
    console.error("Failed to start stream:", err);
    throw err;
  }
};
