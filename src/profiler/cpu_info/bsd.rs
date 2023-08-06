// Copyright (c) 2022, BlockProject 3D
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

use crate::profiler::cpu_info::CpuInfo;
use std::ffi::{c_char, c_int, c_void, CStr};

extern "C" {
    fn sysctlbyname(
        name: *const c_char,
        oldp: *mut c_void,
        oldlenp: *mut usize,
        newp: *mut c_void,
        newlen: usize,
    ) -> c_int;
}

#[cfg(target_vendor = "apple")]
pub fn read_cpu_info() -> Option<CpuInfo> {
    const MACHDEP_CPU_CORE_COUNT: *const c_char =
        unsafe { CStr::from_bytes_with_nul_unchecked(b"machdep.cpu.core_count\0").as_ptr() };
    const MACHDEP_CPU_BRAND_STRING: *const c_char =
        unsafe { CStr::from_bytes_with_nul_unchecked(b"machdep.cpu.brand_string\0").as_ptr() };
    read_cpu_info_bsd(MACHDEP_CPU_CORE_COUNT, MACHDEP_CPU_BRAND_STRING)
}

#[cfg(not(target_vendor = "apple"))]
pub fn read_cpu_info() -> Option<CpuInfo> {
    const HW_NCPU: *const c_char =
        unsafe { CStr::from_bytes_with_nul_unchecked(b"hw.ncpu\0").as_ptr() };
    const HW_MODEL: *const c_char =
        unsafe { CStr::from_bytes_with_nul_unchecked(b"hw.model\0").as_ptr() };
    read_cpu_info_bsd(HW_NCPU, HW_MODEL)
}

fn read_cpu_info_bsd(
    name_core_count: *const c_char,
    name_brand_string: *const c_char,
) -> Option<CpuInfo> {
    let mut core_count: i32 = 0;
    unsafe {
        let mut size = std::mem::size_of::<i32>();
        let res = sysctlbyname(
            name_core_count,
            &mut core_count as *mut i32 as _,
            &mut size,
            std::ptr::null_mut(),
            0,
        );
        if res != 0 {
            return None;
        }
        let res = sysctlbyname(
            name_brand_string,
            std::ptr::null_mut(),
            &mut size,
            std::ptr::null_mut(),
            0,
        );
        if res != 0 {
            return None;
        }
        let mut buffer = vec![0u8; size + 1];
        let res = sysctlbyname(
            name_brand_string,
            buffer.as_mut_ptr() as _,
            &mut size,
            std::ptr::null_mut(),
            0,
        );
        if res != 0 {
            return None;
        }
        Some(CpuInfo {
            core_count: core_count as _,
            name: CStr::from_ptr(buffer.as_ptr() as _)
                .to_string_lossy()
                .into(),
        })
    }
}
