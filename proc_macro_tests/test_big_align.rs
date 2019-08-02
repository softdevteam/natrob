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

#[repr(align(1024))]
struct S2 {
    x: usize
}
impl T for S2 {
    fn f(&self) -> usize {
        self.x
    }
}

fn main() {
    let s1 = NarrowT::new(S1);
    let s2 = NarrowT::new(S2 { x: 2 });

    let s1_t: &dyn T = &*s1;
    let s2_t: &dyn T = &*s2;
    assert_eq!(s1_t.f(), 1);
    assert_eq!(s2_t.f(), 2);
}
