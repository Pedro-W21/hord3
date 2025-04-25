use std::f32::consts::PI;

use crate::horde::{geometry::{rotation::Rotation, shapes_3d::{Cylinder, CylinderTriangles, FixedRegularFace, Quad, Sphere, Triangle}, vec3d::{Vec3D, Vec3Df}}, utils::bitfield::BitField};

use super::meshes::{MeshLOD, MeshTriangles, TrianglePoint};

pub fn add_tri_to(mesh:&mut MeshLOD, tri:&Triangle, texture:u32, collux:&[(u8,u8,u8) ; 3], uv:&[(f32,f32) ; 3], flags:u32, points_added:Option<[usize ; 3]>) {
    match points_added {
        Some(points) => {
            mesh.triangles.add_triangle(
                TrianglePoint::new(points[0], uv[0].0, uv[0].1, collux[0].0, collux[0].1, collux[0].2), 
                TrianglePoint::new(points[1], uv[1].0, uv[1].1, collux[1].0, collux[1].1, collux[1].2),
                TrianglePoint::new(points[2], uv[2].0, uv[2].1, collux[2].0, collux[2].1, collux[2].2),
                texture, flags);
        },
        None => {
            mesh.add_points(&tri.get_points());
            mesh.triangles.add_triangle(
                TrianglePoint::new(mesh.x.len() - 3, uv[0].0, uv[0].1, collux[0].0, collux[0].1, collux[0].2), 
                TrianglePoint::new(mesh.x.len() - 2, uv[1].0, uv[1].1, collux[1].0, collux[1].1, collux[1].2),
                TrianglePoint::new(mesh.x.len() - 1, uv[2].0, uv[2].1, collux[2].0, collux[2].1, collux[2].2),
                texture, flags);
        }
    }
}

pub fn vec_to_complex(tris:&Vec<Triangle>, textures:&Vec<u32>, collluxes:&Vec<[(u8,u8,u8) ; 3]>, uvs:&Vec<[(f32,f32) ; 3]>, fields:&Vec<u32>) -> MeshLOD {
    let mut out = MeshLOD::new(Vec::new(), Vec::new(), Vec::new(), MeshTriangles::with_capacity(10));
    tris.iter().zip(textures.iter().zip(collluxes.iter().zip(uvs.iter().zip(fields.iter()))))
    .for_each(|(tri, (texture, (collux, (uv, field))))| {add_tri_to(&mut out, tri, *texture, collux, uv, *field, None)});
    out
}

pub fn vec_to_complex_rand(tris:&Vec<Triangle>, texture_range:(u32,u32), bitflags:u32) -> MeshLOD {
    let mut out = MeshLOD::new(Vec::new(), Vec::new(), Vec::new(), MeshTriangles::with_capacity(10));
    if texture_range.0 != texture_range.1 {
        tris.iter().for_each(|tri| {add_tri_to(&mut out,tri, fastrand::u32(texture_range.0..texture_range.1), &[(255,255,255), (255,255,255), (255,255,255)], &[(1.0, 1.0), (1.0, 0.0), (0.0, 1.0)], bitflags, None)});
    }
    else {
        tris.iter().for_each(|tri| {add_tri_to(&mut out,tri, texture_range.0, &[(255,255,255), (255,255,255), (255,255,255)], &[(1.0, 1.0), (1.0, 0.0), (0.0, 1.0)], bitflags, None)});
    }
    out
}

pub fn cylinder_to_render_comp<const N:usize>(cylinder:&Cylinder<N>, side_texture:u32, top_texture:u32, bottom_texture:u32, reverse:bool, collux_bot:&Vec<(u8,u8,u8)>, collux_top:&Vec<(u8,u8,u8)>, top_field:u32, bot_field:u32, side_field:u32, bottom:bool, top:bool ) -> MeshLOD {
    let mut lod_out = MeshLOD::new(Vec::new(), Vec::new(), Vec::new(), MeshTriangles::with_capacity(10));
    let tris = cylinder.get_triangles(reverse);
    if !reverse {
        for (i, (tri1, tri2), ) in tris.get_sides().iter().enumerate() {
            add_tri_to(&mut lod_out, tri1, side_texture, &[collux_top[(i + 1) % N], collux_bot[(i + 1) % N], collux_bot[i]], &[(1.0,0.0) , (1.0, 1.0) ,(0.0,1.0)], side_field, None);
            add_tri_to(&mut lod_out, tri2, side_texture, &[collux_top[(i + 1) % N], collux_bot[(i + 1) % N], collux_bot[i]], &[(1.0,0.0) , (1.0, 1.0) ,(0.0,1.0)], side_field, None);
        }
    }
    else {
        for (i, (tri1, tri2), ) in tris.get_sides().iter().enumerate() {
            add_tri_to(&mut lod_out, tri1, side_texture, &[collux_bot[i], collux_bot[(i + 1) % N], collux_top[(i + 1) % N]], &[(1.0,0.0) , (1.0, 1.0) ,(0.0,1.0)], side_field, None);
            add_tri_to(&mut lod_out, tri2, side_texture, &[collux_top[(i + 1) % N], collux_top[i], collux_bot[i]], &[(1.0,0.0) , (1.0, 1.0) ,(0.0,1.0)], side_field, None);
        }
    }
    
    if bottom {
        let other = texture_collux_on_regular_face(&cylinder.get_base(), bottom_texture, collux_bot, reverse, bot_field);
        let pre_len = lod_out.x.len();
        for i in 0..other.x.len() {
            lod_out.add_point(Vec3D::new(other.x[i], other.y[i], other.z[i]));
        }
        for i in 0..other.triangles.len() {
            let (t1, t2, t3, _, _) = other.triangles.get_triangle_points_shifted_indices(i, pre_len);
            lod_out.triangles.add_triangle(
                t1,
                t2,
                t3,
                bottom_texture, bot_field);
        }
    }
    
    if top {
        let other = texture_collux_on_regular_face(&cylinder.get_top(), top_texture, collux_top, !reverse, top_field);
        let pre_len = lod_out.x.len();
        for i in 0..other.x.len() {
            lod_out.add_point(Vec3D::new(other.x[i], other.y[i], other.z[i]));
        }
        for i in 0..other.triangles.len() {
            let (t1, t2, t3, _, _) = other.triangles.get_triangle_points_shifted_indices(i, pre_len);
            lod_out.triangles.add_triangle(
                t1,
                t2,
                t3,
                top_texture, top_field);
        }
    }
    

    lod_out
}

pub fn uv_on_regular_face<const N:usize>(face:&FixedRegularFace<N>, reverse:bool) -> [(f32, f32) ; N] {
    if reverse {
        let ez_face = FixedRegularFace::<N>::new(face.radius()).rotate_around_barycenter(&&Rotation::new_from_euler(PI/4.0, 0.0, 0.0));
        let c2 = Vec3Df::new(face.radius(), face.radius(), 0.0);

        let mut coords = [(0.0, 0.0) ; N];

        let db_r = face.radius() * 2.0;
    
        for i in 0..N {
            coords[N - i - 1].0 = (c2.x + ez_face.point(i).x)/db_r;
            coords[N - i - 1].1 = (c2.y + ez_face.point(i).y)/db_r;
        }

        //dbg!(coords);
        //dbg!(ez_face.points());
        //panic!("");

        coords
    }
    else {
        let ez_face = FixedRegularFace::<N>::new(face.radius()).rotate_around_barycenter(&Rotation::new_from_euler(PI/4.0, 0.0, 0.0));
        let c2 = Vec3Df::new(face.radius(), face.radius(), 0.0);

        let mut coords = [(0.0, 0.0) ; N];

        let db_r = face.radius() * 2.0;
    
        for i in 0..N {
            coords[i].0 = (c2.x + ez_face.point(i).x)/db_r;
            coords[i].1 = (c2.y + ez_face.point(i).y)/db_r;
        }

        coords
    }
    
}

pub fn texture_collux_on_regular_face<const N:usize>(face:&FixedRegularFace<N>, texture_id:u32, collux:&Vec<(u8,u8,u8)>, reverse:bool, field:u32) -> MeshLOD {
    let mut out = MeshLOD::new(Vec::new(), Vec::new(), Vec::new(), MeshTriangles::with_capacity(10));
    if N == 3 {

        let uvs = uv_on_regular_face(face, reverse);
        add_tri_to(&mut out, &face.get_triangles(reverse)[0], texture_id, &[collux[0], collux[1], collux[2]], &[uvs[0], uvs[1], uvs[2]], field, None);
        

        out
    }
    else if N == 4 {

        let uvs = uv_on_regular_face(face, reverse);
        add_tri_to(&mut out, &face.get_triangles(reverse)[0], texture_id, &[collux[0], collux[1], collux[2]], &[uvs[0], uvs[1], uvs[2]], field, None);
        add_tri_to(&mut out, &face.get_triangles(reverse)[1], texture_id, &[collux[2], collux[3], collux[0]], &[uvs[2], uvs[3], uvs[0]], field, None);
        
        out
    }
    else {

        let (mut r, mut g, mut b) = (0,0,0);
        collux.iter().for_each(|col| {r += col.0 as usize; g += col.1 as usize; b += col.2 as usize;});
        r /= N;
        b /= N;
        g /= N;
        let uvs = uv_on_regular_face(face, reverse);
        if reverse {
            for (i,tri) in face.get_triangles(reverse).iter().enumerate() {
                add_tri_to(&mut out, &tri, texture_id, &[collux[i], collux[(i + 1) % N], (r as u8,g as u8,b as u8) ], &[(0.5,0.5),uvs[i],uvs[(i + 1) % N]], field, None);
            }
        }
        else {
            for (i,tri) in face.get_triangles(reverse).iter().enumerate() {
                add_tri_to(&mut out, &tri, texture_id, &[(r as u8,g as u8,b as u8), collux[(i + 1) % N], collux[i]], &[ uvs[(i + 1) % N], uvs[i], (0.5,0.5)], field, None);
            }
        }
        
        out
    }
}