// Code in this module should only be called before the MMU is enabled.

// pub for main9's pre-console panic path (initrd invalid-range print): on
// virt the identity map covers the PL011, so the early-uart writes also work
// in the post-MMU window before devcons::init registers the console UART.
pub mod util;
mod vminit;
