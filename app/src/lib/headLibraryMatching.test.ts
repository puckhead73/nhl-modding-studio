import { expect, it } from "vitest";
import { foldHeadName, headPart, previewColorMaps } from "./headLibraryMatching";
it("matches accented and unaccented hockey names", () => {
  expect(foldHeadName("Jaromír Jágr")).toBe(foldHeadName("Jaromir Jagr"));
  expect(foldHeadName("HÅKAN ŁÓDŹ ØST Ægir")).toBe("hakan lodz ost aegir");
});
it("uses Ignite face coefficients when CM is absent, but prefers a real color map", () => {
  const coeff={name:"head_nik_antropov_coeff_nhl16.dds"};
  const hair={name:"hair_nik_antropov_cm_nhl16.dds"};
  const normal={name:"head_nik_antropov_nm_nhl16.dds"};
  expect(previewColorMaps([hair, coeff, normal],"nhl16")).toEqual([hair,coeff]);
  const color={name:"head_nik_antropov_cm_nhl19.dds"};
  expect(previewColorMaps([coeff,color],"nhl19")).toEqual([color]);
  expect(previewColorMaps([{name:"head_common_dual_coeff_hash.dds"}],"nhl27")).toEqual([]);
});
it("does not interpret a surname containing eye as an eye texture", () => {
  expect(headPart("meyer_freddy_04_01_1981_head_freddy_meyer_cm_nhl16.dds")).toBe("head");
  expect(headPart("geo_reyeShape")).toBe("eye");
});
