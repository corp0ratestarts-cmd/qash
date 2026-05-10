#![no_std]
#![forbid(unsafe_code)]

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { core::hint::spin_loop(); }
}

fn main() -> ! {
    loop { core::hint::spin_loop(); }
}
