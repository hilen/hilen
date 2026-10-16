// Writes assets/models/unlit_quad.glb: 1 red quad facing up whose material
// is marked KHR_materials_unlit, the fixture of the unlit material test.
// Run from the repo root: bun deps/models/make_unlit_quad.ts
import { Document, NodeIO } from "@gltf-transform/core@4.5.1";
import { KHRMaterialsUnlit } from "@gltf-transform/extensions@4.5.1";

const document = new Document();
const buffer = document.createBuffer();
const positions = document
  .createAccessor()
  .setType("VEC3")
  .setArray(new Float32Array([-1, 0, -1, -1, 0, 1, 1, 0, 1, 1, 0, -1]))
  .setBuffer(buffer);
const indices = document
  .createAccessor()
  .setType("SCALAR")
  .setArray(new Uint16Array([0, 1, 2, 0, 2, 3]))
  .setBuffer(buffer);
const unlit = document.createExtension(KHRMaterialsUnlit);
const material = document
  .createMaterial("unlit red")
  .setBaseColorFactor([1, 0, 0, 1])
  .setExtension("KHR_materials_unlit", unlit.createUnlit());
const primitive = document
  .createPrimitive()
  .setAttribute("POSITION", positions)
  .setIndices(indices)
  .setMaterial(material);
const mesh = document.createMesh("quad").addPrimitive(primitive);
const node = document.createNode("quad").setMesh(mesh);
document.createScene().addChild(node);

await new NodeIO().registerExtensions([KHRMaterialsUnlit]).write("assets/models/unlit_quad.glb", document);
console.log("wrote assets/models/unlit_quad.glb");
