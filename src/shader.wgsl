struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) face_index: u32,
};

struct Point {
    e012: f32,
    e013: f32,
    e023: f32,
    e123: f32,
}

fn vec3_to_point(v: vec3<f32>) -> Point {
    var result: Point;
    result.e012 = v.z;
    result.e013 = -v.y;
    result.e023 = v.x;
    result.e123 = 1.0;
    return result;
}

fn point_to_vec3(p: Point) -> vec3<f32> {
    return vec3<f32>(
        p.e023 / p.e123,
        -p.e013 / p.e123,
        p.e012 / p.e123,
    );
}

struct Motor {
    s: f32,
    e12: f32,
    e13: f32,
    e23: f32,
    e01: f32,
    e02: f32,
    e03: f32,
    e0123: f32,
}

fn transform_point(point: Point, motor: Motor) -> Point {
    let a = motor.s;
    let b = motor.e12;
    let c = motor.e13;
    let d = motor.e23;
    let e = motor.e01;
    let f = motor.e02;
    let g = motor.e03;
    let h = motor.e0123;
    let i = point.e012;
    let j = point.e013;
    let k = point.e023;
    let l = point.e123;

    var result: Point;
    result.e012 = -2.0 * a * d * j + -2.0 * a * g * l + 1.0 * a * a * i + 2.0 * a * c * k + -1.0 * d * d * i + -2.0 * d * f * l + 2.0 * b * d * k + -2.0 * b * h * l + -2.0 * c * e * l + 1.0 * b * b * i + 2.0 * b * c * j + -1.0 * c * c * i;
    result.e013 = -2.0 * a * b * k + -1.0 * b * b * j + 2.0 * b * c * i + 2.0 * b * e * l + 1.0 * a * a * j + 2.0 * a * d * i + 2.0 * a * f * l + -2.0 * c * h * l + -2.0 * d * g * l + -1.0 * d * d * j + 2.0 * c * d * k + 1.0 * c * c * j;
    result.e023 = -2.0 * a * c * i + -2.0 * a * e * l + 1.0 * a * a * k + 2.0 * a * b * j + -1.0 * c * c * k + 2.0 * c * d * j + 2.0 * c * g * l + -2.0 * d * h * l + 2.0 * b * f * l + -1.0 * b * b * k + 2.0 * b * d * i + 1.0 * d * d * k;
    result.e123 = a * a * l + b * b * l + c * c * l + d * d * l;
    return result;
}

fn inverse_motor(motor: Motor) -> Motor {
    var result = motor;
    result.e12 = -motor.e12;
    result.e13 = -motor.e13;
    result.e23 = -motor.e23;
    result.e01 = -motor.e01;
    result.e02 = -motor.e02;
    result.e03 = -motor.e03;
    return result;
}

struct Camera {
    transform: Motor,
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

    let inverse_camera_transform = inverse_motor(camera.transform);
    let position = point_to_vec3(transform_point(vec3_to_point(faces.vertices[input.vertex_index % 6u] + faces.faces[output.face_index].position), inverse_camera_transform));

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
