# Pin ledger: port::rfork and sys/include/rfork.h (plan 02-01)

PIN rfork-values RED
error[E0432]: unresolved import `port::rfork` (nix develop -c cargo test -p port --test rfork_test; port/src/rfork.rs absent)
PENDING rfork-patch sha256 998ba6a0eee57eaed83790575869f47a7ec4ca39fc866783260505a59bd2c786
PENDING rfork-tests test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s | rfork-header: sys/include/rfork.h matches port/src/rfork.rs
PIN rfork-values GREEN-1 test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s | rfork-header: sys/include/rfork.h matches port/src/rfork.rs
PIN rfork-values GREEN-2 test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s | rfork-header: sys/include/rfork.h matches port/src/rfork.rs
PIN rfork-values ADVANCED 01922de
APPROVED rfork-patch sha256 998ba6a0eee57eaed83790575869f47a7ec4ca39fc866783260505a59bd2c786

# Pin ledger: rfork masks and the single definition (plan 02-04)

PIN rfork-masks RED
error[E0432]: unresolved import `super::RFORK_SUPPORTED` at aarch64/src/syscall.rs:2810 (no `RFORK_SUPPORTED` in `syscall`); x86_64 mask test is not compiled on this Mac (D-24)
PIN rfork-single-definition RED
x86_64/src/proc.rs:171: defines an rfork flag constant; the one definition is port::rfork | aarch64/src/syscall.rs:92: defines an rfork flag constant; the one definition is port::rfork (plus 9 more in each file; aarch64/src/proc.rs:2536 reaches a flag through crate::syscall; both syscall.rs masks are function-local)
