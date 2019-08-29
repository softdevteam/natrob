// Copyright (c) 2019 King's College London created by the Software Development Team
// <http://soft-dev.org/>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0>, or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, or the UPL-1.0 license <http://opensource.org/licenses/UPL>
// at your option. This file may not be copied, modified, or distributed except according to those
// terms.

#![recursion_limit = "256"]

extern crate proc_macro;

use crate::proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, AttributeArgs, ItemTrait, NestedMeta};

#[proc_macro_attribute]
pub fn narrowable(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);
    let input = parse_macro_input!(input as ItemTrait);
    if args.len() != 1 {
        panic!("Need precisely one argument to 'narrowable'");
    }
    let struct_id = match &args[0] {
        NestedMeta::Meta(m) => m.name(),
        NestedMeta::Literal(_) => panic!("Literals not valid attributes to 'narrowable'")
    };
    let trait_id = &input.ident;
    let expanded = quote! {
        /// A narrow pointer to #trait_id.
        #[repr(C)]
        struct #struct_id {
            // A pointer to an object; immediately preceding that object is a usized pointer to the
            // object's vtable. In other words, on a 64 bit machine the layout is (in bytes):
            //   -8..-1: vtable
            //   0..: object
            // Note that:
            //   1) Depending on the alignment of `object`, the allocated block of memory might
            //      start *before* -8 bytes. To calculate the beginning of the block of memory you
            //      need to know the alignment of both the vtable pointer and `object` (see
            //      `Drop::drop` below).
            //   2) If `object` is zero-sized the pointer might be to the very end of the block, so
            //      you mustn't blindly load bytes from this pointer.
            // The reason for this complex dance is that we're trying to optimise the common case
            // of converting this thin pointer into a fat pointer. However, we can only know
            // `object`'s alignment by looking it up in the vtable: if the user doesn't then call
            // anything in the vtable, we've loaded the vtable's cache line for no good reason.
            // Using the layout above, we can avoid doing this load entirely except in the less
            // common case of dropping the pointer.
            objptr: *mut u8
        }

        impl #struct_id {
            /// Create a new narrow pointer to #trait_id.
            pub fn new<U>(v: U) -> Self
            where
                *const U: ::std::ops::CoerceUnsized<*const (dyn #trait_id + 'static)>,
                U: #trait_id + 'static
            {
                let (layout, uoff) = ::std::alloc::Layout::new::<usize>().extend(
                    ::std::alloc::Layout::new::<U>()).unwrap();
                // In order for our storage scheme to work, it's necessary that `uoff -
                // sizeof::<usize>()` gives a valid alignment for a `usize`. There are only two
                // cases we need to consider here:
                //   1) `object`'s alignment is smaller than or equal to `usize`. If so, no padding
                //      will be added, at which point by definition `uoff - sizeof::<usize>()` will
                //      be exactly equivalent to the start point of the layout.
                //   2) `object`'s alignment is bigger than `usize`. Since alignment must be a
                //      power of two, that means that we must by definition be adding at least one
                //      exact multiple of `usize` bytes of padding.
                // The assert below is thus paranoia writ large: it could only trigger if `Layout`
                // started adding amounts of padding that directly contradict the documentation.
                debug_assert_eq!(uoff % ::std::mem::align_of::<usize>(), 0);

                let objptr = unsafe {
                    let baseptr = ::std::alloc::alloc(layout);
                    let objptr = baseptr.add(uoff);
                    let vtableptr = objptr.sub(::std::mem::size_of::<usize>());
                    let t: &dyn #trait_id = &v;
                    let vtable = ::std::mem::transmute::<*const dyn #trait_id, (usize, usize)>(t).1;
                    ::std::ptr::write(vtableptr as *mut usize, vtable);
                    if ::std::mem::size_of::<U>() != 0 {
                        objptr.copy_from_nonoverlapping(&v as *const U as *const u8,
                            ::std::mem::size_of::<U>());
                    }
                    objptr
                };
                ::std::mem::forget(v);

                #struct_id {
                    objptr
                }
            }

            /// Try casting this narrow trait object to a concrete struct type `U`, returning
            /// `Some(...)` if this narrow trait object has stored an object of type `U` or `None`
            /// otherwise.
            pub fn downcast<U: #trait_id>(&self) -> Option<&U> {
                let t_vtable = {
                    let t: &dyn #trait_id = unsafe { &*(0 as *const U) };
                    unsafe { ::std::mem::transmute::<&dyn #trait_id, (usize, usize)>(t) }.1
                };

                let vtable = unsafe {
                    let vtableptr = self.objptr.sub(::std::mem::size_of::<usize>());
                    ::std::ptr::read(vtableptr as *mut usize)
                };

                if t_vtable == vtable {
                    Some(unsafe { &*(self.objptr as *const U) })
                } else {
                    None
                }
            }
        }

        impl ::std::ops::Deref for #struct_id {
            type Target = dyn #trait_id;

            fn deref(&self) -> &(dyn #trait_id + 'static) {
                unsafe {
                    let vtableptr = self.objptr.sub(::std::mem::size_of::<usize>());
                    let vtable = ::std::ptr::read(vtableptr as *mut usize);
                    ::std::mem::transmute::<(*const _, usize), &dyn #trait_id>(
                        (self.objptr, vtable))
                }
            }
        }

        impl ::std::ops::DerefMut for #struct_id {
            fn deref_mut(&mut self) -> &mut (dyn #trait_id + 'static) {
                unsafe {
                    let vtableptr = self.objptr.sub(::std::mem::size_of::<usize>());
                    let vtable = ::std::ptr::read(vtableptr as *mut usize);
                    ::std::mem::transmute::<(*const _, usize), &mut dyn #trait_id>(
                        (self.objptr, vtable))
                }
            }
        }

        impl ::std::ops::Drop for #struct_id {
            fn drop(&mut self) {
                let fatptr = unsafe {
                    let vtableptr = self.objptr.sub(::std::mem::size_of::<usize>());
                    let vtable = ::std::ptr::read(vtableptr as *mut usize);
                    ::std::mem::transmute::<(*const _, usize), &mut dyn #trait_id>(
                        (self.objptr, vtable))
                };

                // Call `drop` on the trait object before deallocating memory.
                unsafe { ::std::ptr::drop_in_place(fatptr as *mut dyn #trait_id) };

                let align = ::std::mem::align_of_val(fatptr);
                let size = ::std::mem::size_of_val(fatptr);
                unsafe {
                    let (layout, uoff) = ::std::alloc::Layout::new::<usize>().extend(
                        ::std::alloc::Layout::from_size_align_unchecked(size, align)).unwrap();
                    let baseptr = self.objptr.sub(uoff);
                    ::std::alloc::dealloc(baseptr, layout);
                }
            }
        }

        #input
    };

    TokenStream::from(expanded)
}

#[cfg(feature = "abgc")]
#[proc_macro_attribute]
pub fn narrowable_abgc(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);
    let input = parse_macro_input!(input as ItemTrait);
    if args.len() != 1 {
        panic!("Need precisely one argument to 'narrowable'");
    }
    let struct_id = match &args[0] {
        NestedMeta::Meta(m) => m.name(),
        NestedMeta::Literal(_) => panic!("Literals not valid attributes to 'narrowable'")
    };
    let trait_id = &input.ident;
    let expanded = quote! {
        /// A narrow pointer to #trait_id.
        pub struct #struct_id {
            // This struct points to a vtable pointer followed by an object. In other words, on a
            // 64 bit machine the layout is (in bytes):
            //   0..7: vtable
            //   8..: object
            // This is an inflexible layout, since we can only support structs whose alignment is
            // the same or less than a usize's. However, we're stuck for the time being, since abgc
            // can't handle interior pointers.
            vtable: *mut u8
        }

        impl #struct_id {
            /// Create a new narrow pointer to #trait_id.
            pub fn new<U>(v: U) -> ::abgc::Gc<Self>
            where
                *const U: ::std::ops::CoerceUnsized<*const (dyn #trait_id + 'static)>,
                U: #trait_id + 'static
            {
                let (layout, uoff) = ::std::alloc::Layout::new::<usize>().extend(
                    ::std::alloc::Layout::new::<U>()).unwrap();
                // Check that we've not been given an object whose alignment exceeds that of a
                // size: we can't handle such cases until abgc can store interior pointers.
                debug_assert_eq!(uoff, ::std::mem::size_of::<usize>());

                let baseptr = ::abgc::Gc::<#struct_id>::alloc_blank(layout);
                unsafe {
                    let objptr = (baseptr as *mut u8).add(uoff);
                    let t: &dyn #trait_id = &v;
                    let vtable = ::std::mem::transmute::<*const dyn #trait_id, (usize, usize)>(t).1;
                    ::std::ptr::write(baseptr as *mut usize, vtable);
                    if ::std::mem::size_of::<U>() != 0 {
                        objptr.copy_from_nonoverlapping(&v as *const U as *const u8,
                            ::std::mem::size_of::<U>());
                    }
                }
                ::std::mem::forget(v);

                unsafe { ::abgc::Gc::from_raw(baseptr) }
            }

            // In the future, this function could be made safe if:
            //   1) `downcast` returns `Option<Recoverable<&U>>` where `Recoverable` is a simple
            //      wrapper around a reference.
            //   2) A new `deref_recoverable` function returns objects of type
            //      `Recoverable<dyn #trait_id>`.
            //   3) `recover` then only takes in objects of type `Recoverable<dyn #trait_id>`.
            //   4) Rust allows unsized rvalues (RFC 1909) *and* when the
            //      `receiver_is_dispatchable` function in `object_safety.rs` in rustc is
            //      updated to allow unsized rvalues.
            pub unsafe fn recover(o: &dyn Obj) -> ::abgc::Gc<#struct_id> {
                let objptr = o as *const _;
                let baseptr = (objptr as *const usize).sub(1);
                Gc::recover(baseptr as *const u8 as *const #struct_id)
            }

            /// Try casting this narrow trait object to a concrete struct type `U`, returning
            /// `Some(...)` if this narrow trait object has stored an object of type `U` or `None`
            /// otherwise.
            pub fn downcast<U: #trait_id>(&self) -> Option<&U> {
                let t_vtable = {
                    let t: &dyn #trait_id = unsafe { &*(0 as *const U) };
                    unsafe { ::std::mem::transmute::<&dyn #trait_id, (usize, usize)>(t) }.1
                };

                let vtable = unsafe {
                    ::std::ptr::read(self as *const _ as *const usize)
                };

                if t_vtable == vtable {
                    let objptr = unsafe { (self as *const _ as *const usize).add(1) };
                    Some(unsafe { &*(objptr as *const U) })
                } else {
                    None
                }
            }
        }

        impl ::std::ops::Deref for #struct_id {
            type Target = dyn #trait_id;

            fn deref(&self) -> &(dyn #trait_id + 'static) {
                unsafe {
                    let vtable = ::std::ptr::read(self as *const _ as *const usize as *mut usize);
                    let objptr = (self as *const _ as *const usize).add(1);
                    ::std::mem::transmute::<(*const _, usize), &dyn #trait_id>(
                        (objptr, vtable))
                }
            }
        }

        impl ::std::ops::Drop for #struct_id {
            fn drop(&mut self) {
                let fatptr = unsafe {
                    let vtable = ::std::ptr::read(self as *const _ as *const usize as *mut usize);
                    let objptr = (self as *const _ as *const usize).add(1);
                    ::std::mem::transmute::<(*const _, usize), &mut dyn #trait_id>(
                        (objptr, vtable))
                };

                // Call `drop` on the trait object before deallocating memory.
                unsafe { ::std::ptr::drop_in_place(fatptr as *mut dyn #trait_id) };
            }
        }

        impl ::abgc::GcLayout for #struct_id {
            fn layout(&self) -> ::std::alloc::Layout {
                let fatptr = unsafe {
                    let vtable = ::std::ptr::read(self as *const _ as *const usize as *mut usize);
                    let objptr = (self as *const _ as *const usize).add(1);
                    ::std::mem::transmute::<(*const _, usize), &mut dyn #trait_id>(
                        (objptr, vtable))
                };

                let align = ::std::mem::align_of_val(fatptr);
                let size = ::std::mem::size_of_val(fatptr);
                let obj_layout = unsafe { ::std::alloc::Layout::from_size_align_unchecked(size, align) };
                ::std::alloc::Layout::new::<usize>().extend(obj_layout).unwrap().0
            }
        }

        #input
    };

    TokenStream::from(expanded)
}
