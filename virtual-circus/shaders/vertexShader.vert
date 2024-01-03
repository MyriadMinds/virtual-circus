#version 460
// #extension GL_EXT_debug_printf : enable

layout(location = 0) in vec3 pos;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec4 tangent;
layout(location = 3) in vec2 texcoord;
layout(location = 4) in vec2 matcoord;
layout(location = 5) in vec2 normcoord;
layout(location = 6) in vec2 occlusioncoord;
layout(location = 7) in vec2 emissivecoord;
layout(location = 8) in vec4 color;

layout(set = 0, binding = 0) uniform UniformBufferObject 
{
    mat4 model;
    mat4 view;
    mat4 proj;
} ubo;

layout( push_constant ) uniform constants
{
	float time;
    mat4 model_matrix;
} push_constants;

layout(location = 0) out float light_intensity;
layout(location = 1) out vec4 frag_color;
layout(location = 2) out vec2 frag_texcoord;

vec4 quaternionFromEuler(vec3 euler)
{
    vec3 c = cos(euler * 0.5);
    vec3 s = sin(euler * 0.5);
    
    float cx = c.x;
    float cy = c.y;
    float cz = c.z;
    float sx = s.x;
    float sy = s.y;
    float sz = s.z;
    
    return vec4(
        sx * cy * cz - cx * sy * sz,
        cx * sy * cz + sx * cy * sz,
        cx * cy * sz - sx * sy * cz,
        cx * cy * cz + sx * sy * sz
    );
}

mat4 matrixFromQuaternion(vec4 quat)
{
    float x = quat.x;
    float y = quat.y;
    float z = quat.z;
    float w = quat.w;

    float x2 = x + x;
    float y2 = y + y;
    float z2 = z + z;
    float xx = x * x2;
    float xy = x * y2;
    float xz = x * z2;
    float yy = y * y2;
    float yz = y * z2;
    float zz = z * z2;
    float wx = w * x2;
    float wy = w * y2;
    float wz = w * z2;

    vec4 x_axis = vec4(1.0 - (yy + zz), xy + wz, xz - wy, 0.0);
    vec4 y_axis = vec4(xy - wz, 1.0 - (xx + zz), yz + wx, 0.0);
    vec4 z_axis = vec4(xz + wy, yz - wx, 1.0 - (xx + yy), 0.0);
    vec4 w_row = vec4(0.0, 0.0, 0.0, 1.0);
    
    return mat4(x_axis, y_axis, z_axis, w_row);
}

void main() {
    // debugPrintfEXT("texcoord: %v2f\n", texcoord);

    vec3 euler = vec3(1.570796, 0.0, push_constants.time / 1000);
    vec4 quaternion = quaternionFromEuler(euler);
    mat4 rotation = matrixFromQuaternion(quaternion);
    mat4 model_location = ubo.view * ubo.model * rotation;

    vec3 calcNormal = mat3(model_location) * normal;
	vec3 lightDirection = normalize(mat3(ubo.view) * vec3(1.0));
	float lightIntensity = clamp(dot(lightDirection, calcNormal), 0, 1);

    gl_Position = ubo.proj * model_location  * vec4(pos, 1.0);
    light_intensity = lightIntensity;
    frag_color = color;
    frag_texcoord = texcoord;
}