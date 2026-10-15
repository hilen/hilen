# Builds the metal and rough fixture from scratch and saves it as the
# Blender file given after `--`, then `export_glb.py` exports it:
#
#   blender --background --python deps/models/make_metal_rough.py -- assets/models/metal_rough.blend
#   blender --background --python deps/models/export_glb.py -- assets/models/metal_rough.blend
#
# One upright quad, twice as wide as tall, facing the front, the engine
# culls back faces. Its material has a metallic and a roughness factor of
# 1 and takes both from one texture, glTF reads metallic from blue and
# roughness from green, and its occlusion from red of the same texture.
# The left half is a smooth metal, the right half a rough dielectric, the
# lower half is occluded and the upper half is not. A loader that drops
# the texture draws the whole quad as a rough metal. The origin is at
# the middle of the quad, nothing else is in the file.

import bpy, sys

dst = sys.argv[sys.argv.index("--") + 1]

TEXELS = 64
HALF_WIDTH = 2.0
HALF_HEIGHT = 1.0
# Linear light, what the exporter writes as the base color factor.
BASE = (0.9, 0.35, 0.08, 1.0)
METAL_ROUGHNESS = 0.15
OCCLUDED = 0.25


def orm_image():
    """Red is the occlusion, green the roughness, blue the metallic."""
    image = bpy.data.images.new("MetalRough", TEXELS, TEXELS, alpha=False)
    image.colorspace_settings.name = "Non-Color"
    pixels = []
    # Blender rows run from the bottom up, the same way as the uv.
    for y in range(TEXELS):
        for x in range(TEXELS):
            metal = x < TEXELS // 2
            occlusion = OCCLUDED if y < TEXELS // 2 else 1.0
            roughness = METAL_ROUGHNESS if metal else 1.0
            pixels.extend((occlusion, roughness, 1.0 if metal else 0.0, 1.0))
    image.pixels = pixels
    image.pack()
    return image


def occlusion_output():
    """The node group the glTF exporter reads the occlusion from."""
    group = bpy.data.node_groups.new("glTF Material Output", "ShaderNodeTree")
    group.interface.new_socket("Occlusion", in_out="INPUT", socket_type="NodeSocketFloat")
    return group


def material(image):
    mat = bpy.data.materials.new("MetalRough")
    mat.use_nodes = True
    nodes, links = mat.node_tree.nodes, mat.node_tree.links
    bsdf = nodes["Principled BSDF"]
    bsdf.inputs["Base Color"].default_value = BASE
    texture = nodes.new("ShaderNodeTexImage")
    texture.image = image
    separate = nodes.new("ShaderNodeSeparateColor")
    links.new(texture.outputs["Color"], separate.inputs["Color"])
    links.new(separate.outputs["Green"], bsdf.inputs["Roughness"])
    links.new(separate.outputs["Blue"], bsdf.inputs["Metallic"])
    output = nodes.new("ShaderNodeGroup")
    output.node_tree = occlusion_output()
    links.new(separate.outputs["Red"], output.inputs["Occlusion"])
    return mat


bpy.ops.wm.read_factory_settings(use_empty=True)

mesh = bpy.data.meshes.new("Quad")
# Blender is z up and its front is minus y, so the quad faces minus y.
corners = [
    (-HALF_WIDTH, 0.0, -HALF_HEIGHT),
    (HALF_WIDTH, 0.0, -HALF_HEIGHT),
    (HALF_WIDTH, 0.0, HALF_HEIGHT),
    (-HALF_WIDTH, 0.0, HALF_HEIGHT),
]
mesh.from_pydata(corners, [], [(0, 1, 2, 3)])
uv = mesh.uv_layers.new(name="UVMap")
for loop, point in zip(uv.data, [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]):
    loop.uv = point
mesh.materials.append(material(orm_image()))
obj = bpy.data.objects.new("Quad", mesh)
bpy.context.scene.collection.objects.link(obj)

bpy.ops.wm.save_as_mainfile(filepath=dst)
print("SAVED", dst)
