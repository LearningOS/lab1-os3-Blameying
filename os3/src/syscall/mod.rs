#[macro_use]
mod console;

const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;

pub fn syscall(id: usize, args: [usize; 3]) -> isize {
    match id {
        SYSCALL_WRITE => println!("syscall write was called"),
        SYSCALL_EXIT => println!("syscall exit was called"),
        _ => panic!("unsupported syscall id {}", syscall_id),
    };
    0
}