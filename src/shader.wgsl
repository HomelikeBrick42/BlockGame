struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

struct Camera {
    position: vec3<f32>,
    aspect: f32,
}

@group(0)
@binding(0)
var<uniform> camera: Camera;

struct Vertices {
    vertices: array<vec3<f32>, 6>,
}

@group(1)
@binding(0)
var<uniform> vertices: Vertices;

struct Face {
    position: vec3<f32>,
}

struct Faces {
    faces: array<Face>,
}

@group(1)
@binding(1)
var<storage, read> faces: Faces;

@vertex
fn vertex(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;

    let position = vertices.vertices[input.vertex_index % 6u] + faces.faces[input.vertex_index / 6u].position - camera.position;

    output.clip_position = vec4<f32>(position.z / camera.aspect, position.y, position.x / 10.0, position.x);

    return output;
}

@fragment
fn pixel(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}
