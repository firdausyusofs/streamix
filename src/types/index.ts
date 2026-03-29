export type ResourceDescriptor = string | { name: string; types?: string[]; idPrefixes?: string[] };

export interface CatalogDescriptor {
  type: string;
  id: string;
  name: string;
};

export interface Manifest {
  id: string;
  name: string;
  version: string;
  logo: string;
  types: string[];
  resources: ResourceDescriptor[];
  catalogs: CatalogDescriptor[];
};

export interface InstalledAddon {
  transport_url: string;
  manifest: Manifest;
};

export interface AddonConfig {
  addons: InstalledAddon[];
};

export interface MetaPreview {
  id: string;
  name: string;
  description: string;
  type: string;
  releaseInfo: string | null;
  runtime: string;
  released: Date | string | null;
  cast: string[];
  genres: string[];
  poster: string;
  background: string;
  logo: string;
};

export interface Video {
  id: string,
  title: string,
  overview: string | null,
  released: Date | string | null,
  thumbnail: string | null,
  season?: number,
  episode?: number,
};

export interface MetaItem extends MetaPreview {
  videos: Video[];
};

export interface CatalogResponse {
  metas: MetaItem[];
};

export interface Stream {
  name?: string;
  title?: string;
  url?: string;
  infoHash?: string;
  fileIdx?: number;
  addonName?: string;
};

export interface StreamResponse {
  streams: Stream[];
}
