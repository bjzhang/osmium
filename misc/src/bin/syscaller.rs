#![no_main]
#![no_std]
#![feature(asm)]

#[macro_use]
extern crate misc;

use core::str;
use misc::syscall;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("Please enter a byte");

    let mut buf = [0u8; 1];
    syscall::sys_read(&mut buf, 1);
    println!("Good job > {}", str::from_utf8(&buf).unwrap());

    println!("My proc id is {:x}", syscall::sys_get_proc_id());

    match syscall::sys_fork() {
        syscall::ForkResult::Parent(id) => {
            println!("I am a parent of {:x}", id);
        },
        syscall::ForkResult::Fail => {
            println!("fork failed");
        },
        syscall::ForkResult::Child => {
            println!("I'm a child!! ogya-");
            syscall::sys_execve("/bin/nop", 8, &[], &[]);
        }
    }

    println!("Yield!");
    syscall::sys_yield();
    println!("Goodbye");
    syscall::sys_exit(0)
}