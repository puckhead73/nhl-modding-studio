import type { OutfitTextureDto } from "../bindings/OutfitTextureDto";
/** Select an explicit colour raster; never substitute a normal/specular map. */
export function uniformColorMap(textures: OutfitTextureDto[], piece: string) {
  return textures.find(t=>t.piece===piece&&!t.is_nameplate&&/color map$/i.test(t.label))
    ?? textures.find(t=>t.piece===piece&&!t.is_nameplate&&/_cm\.(?:rx2|rpsgl|gnf)$/i.test(t.file_name)&&!/(?:normal|spec|alpha|light) map$/i.test(t.label));
}
