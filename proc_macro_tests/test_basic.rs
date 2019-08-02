// Compiler:
//   status: success
//   stderr:
//   stdout:
//
// Run-time:
//   status: success
//   stderr:
//   stdout:

#![feature(alloc_layout_extra)]
#![feature(coerce_unsized)]

use std::mem::size_of;

extern crate natrob;
use natrob::narrowable;

#[narrowable(NarrowT)]
pub trait T { fn f(&self) -> usize; }

struct S1;
impl T for S1 {
    fn f(&self) -> usize {
        1
    }
}

struct S2;
impl T for S2 {
    fn f(&self) -> usize {
        2
    }
}

struct S3 {
    x: usize
}
impl T for S3 {
    fn f(&self) -> usize {
        self.x
    }
}

fn main() {
    assert_eq!(size_of::<NarrowT>(), size_of::<usize>());
    assert_eq!(size_of::<&NarrowT>(), size_of::<usize>());
    assert_eq!(size_of::<&dyn T>(), 2 * size_of::<usize>());

    let s1 = NarrowT::new(S1);
    let s2 = NarrowT::new(S2);
    let s3 = NarrowT::new(S3 { x: 3 });

    let s1_t: &dyn T = &*s1;
    let s2_t: &dyn T = &*s2;
    let s3_t: &dyn T = &*s3;
    assert_eq!(s1_t.f(), 1);
    assert_eq!(s2_t.f(), 2);
    assert_eq!(s3_t.f(), 3);
}
