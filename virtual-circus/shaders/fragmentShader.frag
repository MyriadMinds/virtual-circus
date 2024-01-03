#version 460
// #extension GL_EXT_debug_printf : enable

layout(location = 0) in float light_intensity;
layout(location = 1) in vec4 frag_color;
layout(location = 2) in vec2 frag_texcoord;

layout(set = 1, binding = 0) uniform MaterialData 
{
    vec4 base_color_factor;             // 0 - 15
    vec3 emissive_factor;               // 16 - 31
    vec2 metallic_roughness_factor;     // 32 - 39
    float normals_scale_factor;         // 40 - 43
    float occlusion_strength_factor;    // 44 - 47
    float alpha_cutoff;                 // 48 - 51
    uint flags;                         // 52 - 55
} material;

layout(set = 1, binding = 1) uniform sampler2D tex_sampler;
layout(set = 1, binding = 2) uniform sampler2D metallic_roughness_sampler;
layout(set = 1, binding = 3) uniform sampler2D normals_sampler;
layout(set = 1, binding = 4) uniform sampler2D occlusion_sampler;
layout(set = 1, binding = 5) uniform sampler2D emissive_sampler;

layout(location = 0) out vec4 outColor;

void main() {
    vec4 tex_color = texture(tex_sampler, frag_texcoord);
    // debugPrintfEXT("alpha_cutoff: %f, tex_alpha: %f \n", material.alpha_cutoff, tex_color.w);
    if(tex_color.w < 0.5) discard;
    outColor = frag_color * material.base_color_factor * tex_color * light_intensity;
}