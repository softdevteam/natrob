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
pub trait T { }

struct S1;
impl T for S1 { }

struct S2 {
    x: usize
}
impl T for S2 { }

fn main() {
    let s1 = NarrowT::new(S1);
    let s2 = NarrowT::new(S2 { x: 2 });

    assert!(s1.downcast::<S1>().is_some());
    assert!(s1.downcast::<S2>().is_none());
    assert_eq!(s2.downcast::<S2>().unwrap().x, 2);
}
