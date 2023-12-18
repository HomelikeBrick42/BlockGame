struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) face_index: u32,
};

struct Camera {
    position: vec3<f32>,
    aspect: f32,
    near_clip: f32,
    far_clip: f32,
}

@group(0)
@binding(0)
var<uniform> camera: Camera;

struct Face {
    position: vec3<f32>,
    normal: vec3<f32>,
    color: vec3<f32>,
}

struct Faces {
    vertices: array<vec3<f32>, 6>,
    faces: array<Face>,
}

@group(1)
@binding(0)
var<storage, read> faces: Faces;

@vertex
fn vertex(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.face_index = input.vertex_index / 6u;

    let position = faces.vertices[input.vertex_index % 6u] + faces.faces[output.face_index].position - camera.position;

    output.clip_position = vec4<f32>(
        position.z / camera.aspect,
        position.y,
        -position.x * -(camera.far_clip + camera.near_clip) / (camera.far_clip - camera.near_clip) - (2.0 * camera.far_clip * camera.near_clip) / (camera.far_clip - camera.near_clip),
        position.x,
    );

    return output;
}

@fragment
fn pixel(input: VertexOutput) -> @location(0) vec4<f32> {
    let light_dir = vec3<f32>(0.3, -0.6, 0.2);
    let light = dot(light_dir, -faces.faces[input.face_index].normal) * 0.5 + 0.5;
    return vec4<f32>(faces.faces[input.face_index].color * light, 1.0);
}
