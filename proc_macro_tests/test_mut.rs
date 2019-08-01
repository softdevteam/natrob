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
pub trait T { fn f(&mut self, x: usize) -> usize; }

struct S1;
impl T for S1 {
    fn f(&mut self, _: usize) -> usize {
        1
    }
}

struct S2 {
    x: usize
}
impl T for S2 {
    fn f(&mut self, x: usize) -> usize {
        let old = self.x;
        self.x = x;
        old
    }
}

fn main() {
    let mut s1 = NarrowT::new(S1);
    let mut s2 = NarrowT::new(S2 { x: 2 });

    assert_eq!(s1.f(42), 1);
    assert_eq!(s2.f(3), 2);
    assert_eq!(s2.f(4), 3);
}
