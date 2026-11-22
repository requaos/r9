# Phase 7 API Coverage Declaration

No external API integration: kernel-driver phase; device peer is qemu's emulated virtio-net hardware (MMIO/DMA), not an external API/SDK/service.

Detail: DTB-driven PCI/ECAM discovery, a from-scratch virtio transport + virtio-net driver (D-01: no external crate), and devether `#l` in the r9 kernel. The only host coupling is qemu command-line flags (`-netdev user`, `-device virtio-net-pci-non-transitional`, `-object filter-dump`) and a hand-rolled pcap parser in xtask (zero new crate dependencies, per D-01/D-06 and the xtask std-only precedent). The api-coverage detector confirms: `detected: false`.
