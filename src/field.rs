// Copyright (c) 2024, BlockProject 3D
//
// All rights reserved.
//
// Redistribution and use in source and binary forms, with or without modification,
// are permitted provided that the following conditions are met:
//
//     * Redistributions of source code must retain the above copyright notice,
//       this list of conditions and the following disclaimer.
//     * Redistributions in binary form must reproduce the above copyright notice,
//       this list of conditions and the following disclaimer in the documentation
//       and/or other materials provided with the distribution.
//     * Neither the name of BlockProject 3D nor the names of its contributors
//       may be used to endorse or promote products derived from this software
//       without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
// "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
// LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
// A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT OWNER OR
// CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
// EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
// PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
// PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF
// LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
// NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
// SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

use std::fmt::Debug;

pub trait Visitor {
    fn visit_int(&mut self, name: &str, value: i64) {
        self.visit_debug(name, value);
    }

    fn visit_uint(&mut self, name: &str, value: u64) {
        self.visit_debug(name, value);
    }

    fn visit_float(&mut self, name: &str, value: f32) {
        self.visit_debug(name, value);
    }

    fn visit_double(&mut self, name: &str, value: f64) {
        self.visit_debug(name, value);
    }

    fn visit_string(&mut self, name: &str, value: &str) {
        self.visit_debug(name, value);
    }

    fn visit_debug<T: Debug>(&mut self, name: &str, debug: T);
}

pub trait FieldSet {
    fn record<V: Visitor>(self, visitor: &mut V);
}

impl FieldSet for () {
    fn record<V: Visitor>(self, _: &mut V) {
    }
}

macro_rules! impl_tuple_fieldset {
    ($(($($id: tt: $name: ident),*)),*) => {
        $(
            impl<$($name: FieldSet),*> FieldSet for ($($name),*) {
                fn record<V: Visitor>(self, visitor: &mut V) {
                    $(
                        self.$id.record(visitor);
                    )*
                }
            }
        )*
    };
}

impl_tuple_fieldset!{
    (0: T, 1: T1),
    (0: T, 1: T1, 2: T2),
    (0: T, 1: T1, 2: T2, 3: T3),
    (0: T, 1: T1, 2: T2, 3: T3, 4: T4),
    (0: T, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5),
    (0: T, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5, 6: T6),
    (0: T, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5, 6: T6, 7: T7),
    (0: T, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5, 6: T6, 7: T7, 8: T8),
    (0: T, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5, 6: T6, 7: T7, 8: T8, 9: T9)
}

macro_rules! impl_fieldset {
    // Would've preferred expr, but turns out expr is useless in macros, so let's not use it.
    ($($t: ty => $func: ident),*) => {
        $(
            impl FieldSet for (&str, $t) {
                fn record<V: Visitor>(self, visitor: &mut V) {
                    let (name, value) = self;
                    visitor.$func(name, value as _);
                }
            }
        )*
    };
}

impl_fieldset! {
    u8 => visit_uint,
    u16 => visit_uint,
    u32 => visit_uint,
    u64 => visit_uint,
    i8 => visit_int,
    i16 => visit_int,
    i32 => visit_int,
    i64 => visit_int,
    f32 => visit_float,
    f64 => visit_double,
    &str => visit_string
}

pub struct D<T>(pub T);

impl<T: Debug> FieldSet for (&str, D<T>) {
    fn record<V: Visitor>(self, visitor: &mut V) {
        visitor.visit_debug(self.0, self.1.0);
    }
}

#[macro_export]
macro_rules! field {
    ($name: ident) => {(stringify!($name), $name)};
    (?$name: ident) => {(stringify!($name), $crate::field::D($name))};
    ($name: ident = $value: expr) => {(stringify!($name), $value)};
    ($name: ident = ?$value: expr) => {(stringify!($name), $crate::field::D($value))};
}

#[macro_export]
macro_rules! fields {
    ($({$($field: tt)*})*) => {
        ($(
            field!($($field)*),
        )*)
    };
}
