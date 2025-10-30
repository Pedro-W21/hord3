use std::{f32::consts::{PI, SQRT_2}, ops::{Add, AddAssign, Div}};

use to_from_bytes_derive::{ToBytes, FromBytes};

use crate::{defaults::default_rendering::vectorinator::meshes::Rectangle, horde::geometry::{Intersection, line::Line3D, plane::{EquationPlane, LinePlaneIntersection, VectorPlane}, vec3d::{Coord, Vec3D}}};

use super::{rotation::Rotation, vec3d::Vec3Df};






#[derive(Clone, ToBytes, FromBytes, Debug)]
pub struct FixedPointCloud<const N:usize> {
    points:[Vec3Df ; N],
}

impl<const N:usize> Add<Vec3Df> for FixedPointCloud<N> {
    type Output = Self;
    fn add(self, rhs: Vec3Df) -> Self::Output {
        Self {points:self.points.map(|point| {point + rhs})}
    }
}

impl<const N:usize> AddAssign<Vec3Df> for FixedPointCloud<N> {
    fn add_assign(&mut self, rhs: Vec3Df) {
        *self = self.clone() + rhs;
    }
}

impl<const N:usize> Div<f32> for &FixedPointCloud<N> {
    type Output = FixedPointCloud<N>;
    fn div(self, rhs: f32) -> Self::Output {
        FixedPointCloud {points:self.points.map(|point| {point / rhs})}
    }
}

impl<const N:usize> Div<f32> for FixedPointCloud<N> {
    type Output = Self;
    fn div(self, rhs: f32) -> Self::Output {
        Self {points:self.points.map(|point| {point / rhs})}
    }
}

impl<const N:usize> Add<Vec3Df> for &FixedPointCloud<N> {
    type Output = FixedPointCloud<N>;
    fn add(self, rhs: Vec3Df) -> Self::Output {
        FixedPointCloud {points:self.points.map(|point| {point + rhs})}
    }
}

pub type FixedConvexFace<const N:usize> = FixedPointCloud<N>;

impl<const N:usize> FixedConvexFace<N> {
    pub fn new(points:[Vec3Df ; N]) -> Self {
        Self { points }
    }
    pub fn get_lines(&self) -> [Line3D ; N] {
        let mut lines = [Line3D::new(Vec3Df::zero(), Vec3Df::zero()) ; N];
        for i in 0..(N-1) {
            lines[i] = Line3D::new(self.points[i], self.points[i+1] - self.points[i]);
        }
        lines[N-1] = Line3D::new(self.points[N-1], self.points[0] - self.points[N-1]);
        lines
    }
    pub fn barycenter(&self) -> Vec3Df {
        let mut total = Vec3Df::zero();
        for i in 0..N {
            total += self.points[i]; 
        }
        
        total/(N as f32)
    }
    pub fn get_triangles(&self, reverse:bool) -> Vec<Triangle> {
        let mut tris = Vec::new();
        self.add_triangles(reverse, &mut tris);
        tris
    }
    pub fn get_points(&self) -> [Vec3Df ; N] {
        self.points.clone()
    }

    pub fn  add_triangles(&self, reverse:bool, tris:&mut Vec<Triangle>) {
        if N == 3 {
            if reverse {
                tris.push(Triangle::new([self.points[2], self.points[1], self.points[0]]));
            }
            else {
                tris.push(Triangle::new([self.points[0], self.points[1], self.points[2]]));
            }
            
        }
        else if N == 4 {
            if reverse {
                tris.push(Triangle::new([self.points[2], self.points[1], self.points[0]]));
                tris.push(Triangle::new([self.points[0], self.points[3], self.points[2]]));
            }
            else {
                tris.push(Triangle::new([self.points[0], self.points[1], self.points[2]]));
                tris.push(Triangle::new([self.points[2], self.points[3], self.points[0]]));
            }
        }
        else {
            let bary = self.barycenter();
            if reverse {
                for i in (0..N).rev() {
                    tris.push(Triangle::new([bary, self.points[i], self.points[if i >= 1 {i - 1} else {N - 1}]]));
                }
            }
            else {
                for i in 0..N {
                    tris.push(Triangle::new([self.points[i], self.points[(i + 1) % N], bary]));
                }
            }
        }
    }
    pub fn sides() -> usize {
        N
    }
    pub fn add_pos(self, pos:Vec3Df) -> Self {
        Self {points:self.points.map(|point| {point + pos})}
    }
    pub fn get_quick_normal(&self) -> Vec3Df {
        (self.points[N - 1] - self.points[0]).cross(&(self.points[1] - self.points[0]))
    }
    #[inline(always)]
    pub fn get_point(&self, at:usize) -> Vec3Df {
        self.points[at]
    }
    pub fn is_point_in_inf_cylinder(&self, point:&Vec3Df) -> bool {
        for i in 0..N {
            if self.vector_starting_from(i).dot(&(*point - self.get_point(i))) < -0.01 {
                return false
            }
        }
        true
    }
    pub fn vector_starting_from(&self, index:usize) -> Vec3Df {
        self.points[if index < N - 1 {index + 1} else {0}] - self.points[index]
    }
    pub fn directors(&self) -> (Vec3Df, Vec3Df) {
        (self.points[N - 1] - self.points[0], self.points[1] - self.points[0])
    }
    pub fn quick_size(&self) -> f32 {
        self.barycenter().dist(&self.get_point(0))
    }
    pub fn rotate_around_origin(&self, rotation:&Rotation) -> Self {
        Self { points: rotation.rotate_array(&self.points) }
    }
    pub fn rotate_around_barycenter(&self, rotation:&Rotation) -> Self {
        let bary = self.barycenter();
        let m_bary = self + (-bary);
        Self { points: rotation.rotate_array(&m_bary.points) } + bary
    } 
}

#[derive(Clone, Debug)]
pub struct FixedRegularFace<const N:usize> {
    face:FixedConvexFace<N>,
    radius:f32,
}

impl<const N:usize> FixedRegularFace<N> {
    pub fn new(radius:f32) -> Self {
        let mut points = [Vec3Df::zero() ; N];
        let mut i = 0;
        let angle = 2.0*PI/(N as f32);
        while i < N {
            points[i] = Vec3Df::new(radius * (angle * i as f32).cos(), radius * (angle * i as f32).sin(), 0.0);
            i += 1;
        } 
        Self { face:FixedConvexFace::new(points), radius }

    }
    pub fn radius(&self) -> f32 {
        self.radius
    }
    pub fn from_points(points:[Vec3Df ; N]) -> Self {
        let mut face = Self { face:FixedConvexFace::new(points), radius: 0.0 };
        face.radius = points[0].dist(&face.barycenter());
        face
    }
    pub fn rotate_around_barycenter(&self, rotation:&Rotation) -> Self {
        Self { face: self.face.rotate_around_barycenter(rotation), radius:self.radius }
    }
    pub fn rotate_around_origin(&self, rotation:&Rotation) -> Self {
        Self { face: self.face.rotate_around_origin(rotation), radius:self.radius }
    }
    pub fn new_complete(points:[Vec3Df ; N], radius:f32) -> Self {
        Self { face:FixedConvexFace::new(points), radius }
    }
    pub fn barycenter(&self) -> Vec3Df {
        self.face.barycenter()
    }
    pub fn get_triangles(&self, reverse:bool) -> Vec<Triangle> {
        self.face.get_triangles(reverse)
    }
    pub fn add_triangles(&self, reverse:bool, tris:&mut Vec<Triangle>) {
        self.face.add_triangles(reverse, tris);
    }
    pub fn sides() -> usize {
        N
    }
    pub fn add_pos(self, pos:Vec3Df) -> Self {
        Self {face:self.face.add_pos(pos), radius:self.radius}
    }
    pub fn get_quick_normal(&self) -> Vec3Df {
        self.face.get_quick_normal()
    }
    pub fn point(&self, at:usize) -> Vec3Df {
        self.face.points[at]
    }
    pub fn points(&self) -> [Vec3Df ; N] {
        self.face.points.clone()
    }
}

impl<const N:usize> Add<Vec3Df> for FixedRegularFace<N> {
    type Output = Self;
    fn add(self, rhs: Vec3Df) -> Self::Output {
        self.add_pos(rhs)
    }
}

pub type Triangle = FixedConvexFace<3>;

impl Triangle {
    pub fn get_plane(&self) -> VectorPlane {
        VectorPlane::new(self.points[1] - self.points[0], self.points[2] - self.points[0], self.points[0])
    }
    pub fn get_raster_rectangle(&self) -> Rectangle<i32> {
        Rectangle::new(
            self.points[0].x.min(self.points[1].x).min(self.points[2].x) as i32,
            self.points[0].y.min(self.points[1].y).min(self.points[2].y) as i32,
            self.points[0].x.max(self.points[1].x).max(self.points[2].x) as i32,
            self.points[0].y.max(self.points[1].y).max(self.points[2].y) as i32
        )
    }
}

impl Intersection<Line3D> for Triangle {
    type IntersectionType = LinePlaneIntersection;
    fn intersect_with(&self, target:&Line3D) -> LinePlaneIntersection {
        let vector_plane = self.get_plane();
        let plane = vector_plane.to_equation_plane();
        match plane.intersect_with(target) {
            LinePlaneIntersection::Nothing => LinePlaneIntersection::Nothing,
            LinePlaneIntersection::Line => LinePlaneIntersection::Line,
            LinePlaneIntersection::Point(coef) => {
                let point = target.get_point_at(coef);
                let (dx, dy) = vector_plane.directors();
                let orig = self.points[0];
                let plane_point = (point - orig);
                let (mut v1,mut v2) = (plane_point.component_div(&dx), plane_point.component_div(&dy));
                v1.zero_out_nans();
                v2.zero_out_nans();
                let mut a = Coord::X;
                let mut b = Coord::Y;
                let mut other_member = 1.0;
                'coord_search : for first_co in Coord::ALL_COORDS {
                    if v1.co(first_co) != 0.0 {
                        let others = first_co.get_others();
                        for other in others {
                            other_member = v2.co(other) - v2.co(first_co) * v1.co(other);
                            if other_member != 0.0 {
                                a = first_co;
                                b = other;
                                break 'coord_search; 
                            }
                        }
                    }
                }
                let inv_v1a = 1.0/v1.co(a);
                let k = (plane_point.co(b) - plane_point.co(a) * v1.co(b)*inv_v1a)/other_member;
                let i = (plane_point.co(a) - k * v2.co(a))*inv_v1a;
                // Not sure about this condition
                if k >= 0.0 && i >= 0.0 && k.powi(2) + i.powi(2) <= 1.0 {
                    LinePlaneIntersection::Point(coef)
                }
                else {

                    LinePlaneIntersection::Nothing
                }
            }
        }
    }
}

impl Intersection<Triangle> for Triangle {
    type IntersectionType = bool;
    fn intersect_with(&self, target:&Triangle) -> Self::IntersectionType {
        // big optimizations : use iterators for the lines, and reuse the plane for each triangle computed internally
        let other_lines = target.get_lines();
        for line in other_lines {
            if self.intersect_with(&line).is_something() {
                return true
            }
        }
        let my_lines = self.get_lines();
        for line in my_lines {
            if target.intersect_with(&line).is_something() {
                return true
            }
        }
        false
    }
}

impl Intersection<Sphere> for Triangle {
    type IntersectionType = bool;
    fn intersect_with(&self, target:&Sphere) -> Self::IntersectionType {
        let eq_plane = self.get_plane().to_equation_plane();
        let s_d = eq_plane.signed_distance(&target.origin);
        if s_d.abs() <= target.radius {
            let normal = eq_plane.get_normal();
            let line = Line3D::new(target.origin, normal.normalise() * target.radius);
            if self.intersect_with(&line).is_something() {
                return true
            }
            else {
                let mini_line = Line3D::new(target.origin, Vec3Df::all_ones() * target.radius * 0.01);
                let my_lines = self.get_lines();
                for my_line in my_lines {
                    if my_line.calc_shortest_distance_between_director_segments(&mini_line) < target.radius {
                        return true
                    }
                }
                false
            }

        }
        else {
            false
        }
    }
}

pub type EquiTriangle = FixedRegularFace<3>;

pub type Quad = FixedConvexFace<4>;

pub type Square = FixedRegularFace<4>;

impl Square {
    pub fn new_square(side:f32) -> Self {
        Self::new(side/SQRT_2)
    }
}

impl Quad {
    pub fn new_square(side:f32) -> Self {
        FixedRegularFace::new(side/SQRT_2).face
    }
    fn get_rect_triangles(&self, reverse:bool) -> (Triangle, Triangle) {
        if reverse {
            (Triangle::new([self.points[2], self.points[1], self.points[0]]),
            Triangle::new([self.points[0], self.points[3], self.points[2]]))
        }
        else {
            (Triangle::new([self.points[0], self.points[1], self.points[2]]),
            Triangle::new([self.points[2], self.points[3], self.points[0]]))
        }
    }
    pub fn get_both_sides(&self) -> [Triangle ; 4] {
        let reverse = self.get_rect_triangles(true);
        let normal = self.get_rect_triangles(false);
        //dbg!(reverse.clone());
        //dbg!(normal.clone());
        [reverse.0, reverse.1, normal.0, normal.1] 
    }
}

pub struct Cylinder<const N:usize> {
    base:FixedRegularFace<N>,
    height:f32,
}

impl<const N:usize> Add<Vec3Df> for Cylinder<N> {
    type Output = Cylinder<N>;
    fn add(self, rhs: Vec3Df) -> Self::Output {
        Self {base:self.base + rhs, height:self.height}
    }
}

impl<const N:usize> Cylinder<N> {
    pub fn get_triangles(&self, reverse:bool) -> CylinderTriangles {
        let top = self.get_top();
        let mut tris = CylinderTriangles {top:top.get_triangles(!reverse), bot:self.base.get_triangles(reverse), sides:self.get_side_triangles(!reverse, &top) };

        tris
    }
    pub fn get_top(&self) -> FixedRegularFace<N> {
        self.base.clone() + -(self.base.get_quick_normal().normalise() * self.height)
    }
    pub fn get_base(&self) -> FixedRegularFace<N> {
        self.base.clone()
    }
    pub fn get_side_triangles(&self, reverse:bool, top:&FixedRegularFace<N>) -> Vec<(Triangle, Triangle)> {
        let mut sides = Vec::with_capacity(N);
        for i in 0..N - 1 {
            sides.push(Quad::new([self.base.face.get_point(i), self.base.face.get_point(i + 1), top.face.get_point(i + 1), top.face.get_point(i)]).get_rect_triangles(reverse));
        }
        sides.push(Quad::new([self.base.face.get_point(N - 1), self.base.face.get_point(0), top.face.get_point(0), top.face.get_point(N - 1)]).get_rect_triangles(reverse));
        sides
    }
    pub fn new(base:FixedRegularFace<N>, height:f32) -> Self {
        Self { base, height }
    }
    pub fn rotate_around_base_center(&self, rotation:&Rotation) -> Self {
        let bary = self.base.barycenter();
        //dbg!(bary);
        //dbg!(self.base.face.points);
        //panic!("A");
        Self {
            base:FixedRegularFace::new_complete(rotation.rotate_array(&(&self.base.face + (-bary)).points) , self.base.radius) + bary,
            height:self.height
        }
        
    }
    pub fn rotate_around_barycenter(&self, rotation:&Rotation) -> Self {
        let bary = self.base.barycenter() - (self.base.get_quick_normal().normalise() * self.height/2.0);
        //dbg!(bary);
        //panic!("A");
        Self {
            base:FixedRegularFace::new_complete(rotation.rotate_array(&(&self.base.face + (-bary)).points) , self.base.radius) + bary,
            height:self.height
        }
    }
}

pub struct CylinderTriangles {
    top:Vec<Triangle>,
    bot:Vec<Triangle>,
    sides:Vec<(Triangle, Triangle)>
}

impl CylinderTriangles {
    pub fn get_all_tris_raw(&self) -> Vec<Triangle> {
        let mut tris = Vec::with_capacity(self.top.len() * 2 + self.sides.len() * 2);
        self.add_tris_raw(&mut tris);
        tris
    }
    pub fn add_tris_raw(&self, tris:&mut Vec<Triangle>) {
        for tri in &self.top {
            tris.push(tri.clone());
        }
        for tri in &self.bot {
            tris.push(tri.clone());
        }
        for (tri1, tri2) in &self.sides {
            tris.push(tri1.clone());
            tris.push(tri2.clone());
        }
    }
    pub fn get_sides(&self) -> &Vec<(Triangle,Triangle)> {
        &self.sides
    }
    pub fn get_top(&self) -> &Vec<Triangle> {
        &self.top
    }
    pub fn get_bot(&self) -> &Vec<Triangle> {
        &self.bot
    }
    
}

pub type Cube = Cylinder<4>;

impl Cube {
    pub fn new_cube(side:f32) -> Self {
        Cylinder::new(Square::new((SQRT_2/2.0) * side), side)
    }
}

impl Triangle {
    pub fn reverse(self, reverse:bool) -> Self {
        if reverse {
            Triangle::new([self.points[2], self.points[1], self.points[0]])
        }
        else {
            self.clone()
        }
    }
}

pub struct Sphere {
    radius:f32,
    origin:Vec3Df,
}

impl Sphere {
    pub fn new(origin:Vec3Df, radius:f32) -> Self {
        Self { radius, origin }
    }
    pub fn add_triangles<const PRECISION:usize>(&self,reverse:bool,tris:&mut Vec<Triangle>) {
        let angle = PI/(PRECISION as f32);
        let mut current_face = Vec::new();
        let mut next_face = get_regular_points(3, 0.0) + Vec3Df::new(0.0, self.radius, 0.0);

        for i in 0..PRECISION/2 {
            current_face = next_face.clone();
            let ang = (i + 1) as f32 * angle;
            next_face = get_regular_points((3 * 2_usize.pow((i + 1) as u32)), self.radius * ang.sin()) + Vec3Df::new(0.0, self.radius * ang.cos(), 0.0);
            let limit = 3 * 2_usize.pow(i as u32);
            for j in 0..limit {

                let start_ind = 2 * j;
                let end_ind = 2 * ((j + 1) % limit);
                tris.push(Triangle::new([next_face[start_ind], current_face[j], next_face[start_ind + 1]]).reverse(reverse));
                tris.push(Triangle::new([current_face[j], current_face[(j + 1) % limit], next_face[start_ind + 1]]).reverse(reverse));
                tris.push(Triangle::new([next_face[start_ind + 1], current_face[(j + 1) % limit], next_face[end_ind]]).reverse(reverse));
            }
        }
        for i in PRECISION/2..PRECISION {
            current_face = next_face.clone();
            let ang = (i + 1) as f32 * angle;
            next_face = get_regular_points((3 * 2_usize.pow((PRECISION - (i + 1)) as u32)), (self.radius * ang.sin()).abs()) + Vec3Df::new(0.0, self.radius * ang.cos(), 0.0);
            let limit = 3 * 2_usize.pow((PRECISION - (i + 1)) as u32);
            for j in 0..limit {
                // lol mdr ptdr prout
                let start_ind = 2 * j;
                let end_ind = 2 * ((j + 1) % limit);
                tris.push(Triangle::new([current_face[start_ind], next_face[j], current_face[start_ind + 1]]).reverse(!reverse));
                tris.push(Triangle::new([next_face[j], next_face[(j + 1) % limit], current_face[start_ind + 1]]).reverse(!reverse));
                tris.push(Triangle::new([current_face[start_ind + 1], next_face[(j + 1) % limit], current_face[end_ind]]).reverse(!reverse));
            }
        }
    }
    pub fn get_triangles<const PRECISION:usize>(&self,reverse:bool) -> Vec<Triangle> {
        
        let mut tris = Vec::new();

        //(EquiTriangle::new(self.radius * angle.sin()) + Vec3Df::new(0.0, self.radius * angle.cos(), 0.0)).add_triangles(!reverse, &mut tris);
        self.add_triangles::<PRECISION>(reverse, &mut tris);
        for tri in &mut tris {
            *tri += self.origin;
        }
        //(EquiTriangle::new(self.radius * angle.sin()) + Vec3Df::new(0.0, -self.radius * angle.cos(), 0.0)).add_triangles(reverse, &mut tris);
        //panic!("zazou");
        tris
    }
}

type PointVec = Vec<Vec3Df>;

pub fn get_regular_points(number:usize, radius:f32) -> PointVec {
    let mut points = Vec::with_capacity(number);
    let mut i = 0;
    let angle = 2.0*PI/(number as f32);
    while i < number {
        points.push(Vec3Df::new(radius * (angle * i as f32).cos(), 0.0, radius * (angle * i as f32).sin()));
        i += 1;
    } 
    points
}

impl Add<Vec3Df> for PointVec {
    type Output = PointVec;
    fn add(self, rhs: Vec3Df) -> Self::Output {
        self.iter().map(|point| {*point + rhs}).collect()
    }
}