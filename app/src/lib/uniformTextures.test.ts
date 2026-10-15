import { expect,it } from "vitest";
import { uniformColorMap } from "./uniformTextures";
import type { OutfitTextureDto } from "../bindings/OutfitTextureDto";
const texture=(file_name:string,label:string,index=0):OutfitTextureDto=>({file_name,label,piece:"jersey",texture_index:index,rel_path:"rendering/jersey/jersey_2/"+file_name,is_nameplate:false});
it("chooses the selected kit colour map even when normal sorts first",()=>{
 const normal=texture("jersey_0_2_0_0_nm.rx2","Jersey · normal map");const color=texture("jersey_0_2_0_cm.rx2","Jersey · color map");
 expect(uniformColorMap([normal,color],"jersey")).toBe(color);
});
it("selects the colour raster within a multi-texture bundle",()=>{
 const normal=texture("texlib_0_2_0.rx2","Jersey · normal map",0);const color=texture("texlib_0_2_0.rx2","Jersey · color map",2);
 expect(uniformColorMap([normal,color],"jersey")?.texture_index).toBe(2);
});
it("does not substitute normal/spec maps when colour is absent",()=>{
 expect(uniformColorMap([texture("jersey_0_2_0_nm.rx2","Jersey · normal map")],"jersey")).toBeUndefined();
});
