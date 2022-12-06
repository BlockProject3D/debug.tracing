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

//BSD*
// sysctl(hw.ncpu) = cpuCoreCount
// sysctl(hw.model) = cpuName

//Apple
// sysctl(machdep.cpu.core_count) = cpuCoreCount
// sysctl(machdep.cpu.brand_string) = cpuName

//Linux and Windows
//x86 & x86-64 -> cpuid instruction (rust-cpuid library / get_processor_brand_string() and max_cores_for_package().or(max_cores_for_cache()))
//other -> None

#[cfg(any(target_vendor = "apple", target_os = "freebsd", target_os = "netbsd", target_os = "openbsd"))]
mod bsd;

//if vendor != apple && os != bsd* && (arch == x86 || arch == x86_64)
#[cfg(all(not(any(target_vendor = "apple", target_os = "freebsd", target_os = "netbsd", target_os = "openbsd")), any(target_arch = "x86", target_arch = "x86_64")))]
mod x86_64;

#[cfg(any(target_vendor = "apple", target_os = "freebsd", target_os = "netbsd", target_os = "openbsd"))]
pub use bsd::read_cpu_info;

//if vendor != apple && os != bsd* && (arch == x86 || arch == x86_64)
#[cfg(all(not(any(target_vendor = "apple", target_os = "freebsd", target_os = "netbsd", target_os = "openbsd")), any(target_arch = "x86", target_arch = "x86_64")))]
pub use x86_64::read_cpu_info;

//if vendor != apple && os != bsd* && arch != x86 && arch != x86_64
#[cfg(all(not(any(target_vendor = "apple", target_os = "freebsd", target_os = "netbsd", target_os = "openbsd")), not(any(target_arch = "x86", target_arch = "x86_64"))))]
pub fn read_cpu_info() -> crate::profiler::network_types::Option<CpuInfo> {
    None
}
