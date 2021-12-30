/*! Conversion facilities */

pub fn to_i32_arr(a: [u32; 3]) -> [i32; 3] {
    [a[0] as i32, a[1] as i32, a[2] as i32]
}


pub fn to_u32_arr(a: [i32; 3]) -> [u32; 3] {
    [a[0] as u32, a[1] as u32, a[2] as u32]
}
