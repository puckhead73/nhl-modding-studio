export function foldHeadName(name: string) {
  const replacements: Record<string,string> = { "ø":"o", "ł":"l", "ß":"ss", "æ":"ae", "œ":"oe", "đ":"d", "ð":"d", "þ":"th" };
  return name.toLowerCase().replace(/[øłßæœđðþ]/g, c => replacements[c]).normalize("NFKD").replace(/\p{M}/gu, "");
}

export function headPart(name: string) {
  // A player's surname (e.g. Meyer) is not an eye material semantic.
  if (/(?:^|_)head(?:shape|_|\.|$)/i.test(name)) return "head";
  return /beard|facial|moustache|goatee/i.test(name) ? "facial" : /hair|card/i.test(name) ? "hair" :
    /eyelash/i.test(name) ? "eyelash" : /eye/i.test(name) ? "eye" : "head";
}

export function previewColorMaps<T extends { name: string }>(textures: T[], game: string): T[] {
  const colors = textures.filter(t => /(?:_cm(?:_|\.))|(?:_color_)/i.test(t.name));
  // Ignite coefficient maps carry face RGB in titles without a separate CM.
  // Do not substitute Frostbite dual-coefficient/specular maps for face color.
  if (/^nhl(?:1[5-9]|2[01])$/i.test(game)) {
    for (const t of textures.filter(t => /_coeff(?:_|\.)/i.test(t.name))) {
      if (!colors.some(c => headPart(c.name) === headPart(t.name))) colors.push(t);
    }
  }
  return colors;
}
