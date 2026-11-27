#!/bin/sh
exec qemu-system-x86_64 -no-reboot -d int,reset,cpu_reset -D "${R9_QEMU_LOG:-/tmp/qemu-diag.log}" "$@"
