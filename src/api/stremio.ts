import { invoke } from "@tauri-apps/api/core";
import { AddonConfig, CatalogResponse, MetaPreview } from "../types";

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
