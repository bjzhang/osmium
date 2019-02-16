#![no_main]
#![no_std]
#![feature(asm)]

#[macro_use]
extern crate misc;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("{}", "Hello Syscall");

    loop {}
}

use core::panic::PanicInfo;
#[panic_handler]
#[no_mangle]
pub fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

#[no_mangle]
pub extern "C" fn abort() -> ! {
    loop {}
}
