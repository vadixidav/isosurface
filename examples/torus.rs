// Copyright 2017 Tristam MacDonald
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#[macro_use]
extern crate glium;
extern crate cgmath;
extern crate num;
extern crate isosurface;

use glium::glutin;
use glium::Surface;
use glium::index::PrimitiveType;
use glutin::{GlProfile, GlRequest, Api, Event, WindowEvent, ControlFlow};
use cgmath::{Vector3, vec3, Matrix4, Point3};
use num::range_step;
use std::slice;
use isosurface::marching_cubes;

#[derive(Copy, Clone)]
#[repr(C)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
}

implement_vertex!(Vertex, position, normal);

/// This is used to reinterpret slices of floats as slices of repr(C) structs, without any
/// copying. It is optimal, but it is also punching holes in the type system. I hope that Rust
/// provides safe functionality to handle this in the future. In the meantime, reproduce
/// this workaround at your own risk.
fn reinterpret_cast_slice<S, T>(input : &[S], length : usize) -> &[T] {
    unsafe {
        slice::from_raw_parts(input.as_ptr() as *const T, length)
    }
}

/// The distance-field equation for a torus
fn torus(x : f32, y : f32, z : f32) -> f32 {
    const R1 : f32 = 1.0 / 4.0;
    const R2 : f32 = 1.0 / 10.0;
    let q_x = ((x*x + y*y).sqrt()).abs() - R1;
    let len = (q_x*q_x + z*z).sqrt();
    len - R2
}

struct Torus {}

impl marching_cubes::Source for Torus {
    fn sample(&self, x : f32, y : f32, z : f32) -> f32 {
        torus(x - 0.5, y - 0.5, z - 0.5)
    }
}

/// Takes an array of vertices, and indices defining the faces of a triangle mesh.
/// Outputs a welded array of vertices + normals matching the indices.
fn build_smooth_normals(vertices : &[Vector3<f32>], indices : &[u32], output : &mut Vec<Vector3<f32>>) {
    for &v in vertices.iter() {
        output.push(v);
        output.push(vec3(0.0, 0.0, 0.0));
    }

    for i in range_step(0, indices.len(), 3) {
        let v0 : Vector3<f32> = vertices[indices[i] as usize];
        let v1 : Vector3<f32> = vertices[indices[i+1] as usize];
        let v2 : Vector3<f32> = vertices[indices[i+2] as usize];

        let n = (v1 - v0).cross(v2 - v0);

        for j in 0..3 {
            output[(indices[i+j]*2 + 1) as usize] = output[(indices[i+j]*2 + 1) as usize] + n;
        }
    }
}

fn main() {
    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new()
        .with_title("torus")
        .with_dimensions(1024, 768);
    let context = glutin::ContextBuilder::new()
        .with_vsync(true)
        .with_gl_profile(GlProfile::Core)
        .with_gl(GlRequest::Specific(Api::OpenGl, (3, 3)))
        .with_depth_buffer(24);
    let display = glium::Display::new(window, context, &events_loop)
        .expect("failed to create display");

    let torus = Torus{};

    let mut vertices = vec![];
    let mut indices = vec![];
    let mut marching_cubes = marching_cubes::MarchingCubes::new(256);

    marching_cubes.extract(&torus, &mut vertices, &mut indices);

    let mut vertices_with_normals = vec![];

    build_smooth_normals(reinterpret_cast_slice(&vertices, vertices.len()/3), &indices, &mut vertices_with_normals);

    let vertex_buffer: glium::VertexBuffer<Vertex> = {
        glium::VertexBuffer::new(
            &display,
            reinterpret_cast_slice(&vertices_with_normals, vertices.len()/3)
        ).expect("failed to create vertex buffer")
    };

    let index_buffer: glium::IndexBuffer<u32> =
        glium::IndexBuffer::new(&display, PrimitiveType::TrianglesList, &indices)
            .expect("failed to create index buffer");

    let program = program!(&display,
            330 => {
                vertex: "#version 330
                    uniform mat4 model_view_projection;

                    layout(location=0) in vec3 position;
                    layout(location=1) in vec3 normal;

                    out vec3 vNormal;

                    void main() {
                        gl_Position = model_view_projection * vec4(position, 1.0);
                        vNormal = normal;
                    }
                ",
                fragment: "#version 330
                    in vec3 vNormal;

                    layout(location=0) out vec4 color;

                    void main() {
                        float NdotL = dot(normalize(vNormal), vec3(0,-1,0))*0.75 + 0.25;
                        color = vec4(vec3(0.667, 0.459, 0.224) * NdotL, 1.0);
                    }
                "
            },
        ).expect("failed to compile shaders");

    let projection = cgmath::perspective(cgmath::Deg(45.0), 1024.0/768.0, 0.01, 1000.0);
    let view = Matrix4::look_at(Point3::new(-0.25, -0.25, -0.25), Point3::new(0.0, 0.0, 0.0), vec3(0.0, 1.0, 0.0));

    events_loop.run_forever(|event| {
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::Closed => return ControlFlow::Break,
                _ => (),
            },
            _ => (),
        }

        let mut surface = display.draw();
        surface.clear_color_and_depth((0.153, 0.337, 0.42, 0.0), 1.0);

        let uniforms = uniform! {
            model_view_projection: Into::<[[f32; 4]; 4]>::into(projection * view),
        };

        let params = glium::DrawParameters {
            depth: glium::Depth {
                test: glium::DepthTest::IfLess,
                write: true,
                ..Default::default()
            },
            backface_culling: glium::draw_parameters::BackfaceCullingMode::CullCounterClockwise,
            ..Default::default()
        };

        surface.draw(
            &vertex_buffer,
            &index_buffer,
            &program,
            &uniforms,
            &params,
        ).expect("failed to draw to surface");

        surface.finish().expect("failed to finish rendering frame");

        ControlFlow::Continue
    });

}