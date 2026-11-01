use crate::deviceutil::map_device_register;
use crate::io::{read_reg, write_reg};
use crate::registers::{UART0_DR, UART0_FR};
use crate::vm;
use port::Result;
use port::devcons::Uart;
use port::fdt::DeviceTree;
use port::mem::{PhysRange, VirtRange};

#[cfg(not(test))]
use port::println;

/// PL011 on qemu's virt machine.  Unlike the raspi variant
/// (platform/raspi3b/uartpl011.rs) there is no GPIO/mailbox init -- qemu
/// pre-configures the UART (phase-03 research Pattern 3), so this driver is
/// DTB-discovered, map_device_register-mapped, polled-TX only.  RX IRQ init
/// arrives in 03-05.
pub struct Pl011Uart {
    pl011_virtrange: VirtRange,
}

impl Pl011Uart {
    pub fn new(dt: &DeviceTree) -> Result<Pl011Uart> {
        let pl011_physrange = Self::find_pl011_physrange(dt)?;
        let pl011_virtrange =
            match map_device_register("pl011", pl011_physrange, vm::PageSize::Page4K) {
                Ok(pl011_virtrange) => pl011_virtrange,
                Err(msg) => {
                    println!("can't map pl011 {:?}", msg);
                    return Err("can't create pl011");
                }
            };

        Ok(Pl011Uart { pl011_virtrange })
    }

    fn find_pl011_physrange(dt: &DeviceTree) -> Result<PhysRange> {
        dt.find_compatible("arm,pl011")
            .next()
            .and_then(|uart| dt.property_translated_reg_iter(uart).next())
            .and_then(|reg| reg.regblock())
            .map(|reg| PhysRange::from(&reg))
            .ok_or("can't find pl011")
    }
}

impl Uart for Pl011Uart {
    fn putb(&self, b: u8) {
        // Wait for UART to become ready to transmit.
        while read_reg(&self.pl011_virtrange, UART0_FR) & (1 << 5) != 0 {}
        write_reg(&self.pl011_virtrange, UART0_DR, b as u32);
    }
}
