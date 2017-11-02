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

//! Isosurface definitions for use in multiple examples

use isosurface;

/// The distance-field equation for a torus
fn torus(x : f32, y : f32, z : f32) -> f32 {
    const R1 : f32 = 1.0 / 4.0;
    const R2 : f32 = 1.0 / 10.0;
    let q_x = ((x*x + y*y).sqrt()).abs() - R1;
    let len = (q_x*q_x + z*z).sqrt();
    len - R2
}

pub struct Torus {}

impl isosurface::source::Source for Torus {
    fn sample(&self, x : f32, y : f32, z : f32) -> f32 {
        torus(x - 0.5, y - 0.5, z - 0.5)
    }
}