// Read-only container command contract. All offsets here are bounded u32/usize
// values returned by the reader; resource IDs and hashes stay strings.
import { invoke } from "@tauri-apps/api/core";
export type FrostbiteLayer = { name: string; head: number | null; base: number | null; superbundles: string[]; initfs: string; chunkmanifest: boolean };
export type FrostbiteInstall = { root: string; layers: FrostbiteLayer[]; warnings: string[] };
export type FrostbitePage = { total: number; matched: number; toc_chunks: number; items: { ordinal: number; name: string; spans: number }[] };
export type FrostbiteAssets = { name: string; assets: { kind: string; name: string; size: number; sha1: string; resource_type: number | null; resource_id: string | null; resource_meta: string | null; logical_offset: number | null }[]; spans: { cas: number; offset: number; size: number }[] };
export type FrostbiteSelection = { root: string; layer: string; superbundle: string };
export type FrostbiteChunks = { matched: number; items: { ordinal: number; guid: string; span: { cas: number; offset: number; size: number } }[] };
export const listFrostbiteChunks = (selection: FrostbiteSelection, search: string, offset: number) => invoke<FrostbiteChunks>("frostbite_chunks", { ...selection, search, offset });
export const exportFrostbiteChunk = (selection: FrostbiteSelection, chunk: number, decoded: boolean, destination: string) => invoke<number>("frostbite_export_chunk", { ...selection, chunk, decoded, destination });
export const inspectFrostbite = (root: string) => invoke<FrostbiteInstall>("frostbite_inspect", { root });
export const listFrostbite = (selection: FrostbiteSelection, search: string, offset: number) => invoke<FrostbitePage>("frostbite_bundles", { ...selection, search, offset });
export const frostbiteAssets = (selection: FrostbiteSelection, bundle: number) => invoke<FrostbiteAssets>("frostbite_assets", { ...selection, bundle });
export const exportFrostbite = (selection: FrostbiteSelection, bundle: number, asset: number, decoded: boolean, destination: string) => invoke<number>("frostbite_export", { ...selection, bundle, asset, decoded, destination });
