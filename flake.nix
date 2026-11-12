{
  description = "r9 dev environment — pinned rust toolchain, qemu, cargo xtask";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  inputs.rust-overlay.url = "github:oxalica/rust-overlay";
  inputs.rust-overlay.inputs.nixpkgs.follows = "nixpkgs";

  outputs = { self, nixpkgs, rust-overlay }:
    let
      # Intel-Mac tuple deliberately omitted — out of project scope (REQUIREMENTS.md)
      systems = [ "aarch64-darwin" "aarch64-linux" "x86_64-linux" ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
      pkgsFor = system: import nixpkgs {
        inherit system;
        overlays = [ rust-overlay.overlays.default ];
      };
      # rust-toolchain.toml is the single source of truth for channel,
      # components and targets (D-09) — never restate pins here; r9's
      # "bump rust" commits must not require dual-pin edits.
      toolchainFor = system:
        (pkgsFor system).rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      # llvm-tools lands in the toolchain's rustlib dir; xtask's objcopy()
      # already honors $OBJCOPY first (xtask/src/main.rs:246-271).
      # NB: nixpkgs' bintools wrapper (present even under mkShellNoCC —
      # stdenv keeps ar/strip for fixup) unconditionally exports
      # OBJCOPY=objcopy, a bare-name placeholder, BEFORE the shellHook runs.
      # Treat only absolute-path values as genuine user overrides.
      objcopyHook = toolchain: ''
        case "''${OBJCOPY:-}" in
          /*) : ;;  # user override (absolute path) — keep
          *) export OBJCOPY="$(ls ${toolchain}/lib/rustlib/*/bin/llvm-objcopy 2>/dev/null | head -1)" ;;
        esac
      '';
      # Same default for apps. Pure-bash glob loop: writeShellApplication's
      # checkPhase runs shellcheck, which rejects `ls … | head -1` (SC2012).
      objcopyExport = toolchain: ''
        _r9_objcopy=""
        for _r9_c in ${toolchain}/lib/rustlib/*/bin/llvm-objcopy; do
          if [ -e "$_r9_c" ]; then _r9_objcopy="$_r9_c"; break; fi
        done
        export OBJCOPY="''${OBJCOPY:-$_r9_objcopy}"
      '';
      # 14-07 (D-05): the parameterized ISO builder — ONE implementation
      # of the whole reproducibility machinery, shared UN-FORKED by r9's
      # own packages.boot-iso FOD (kernel-only, its wired hash riding the
      # same bytes) and by Harvey's compose app (kernel + Harvey-baked
      # bootfs module, driven at runtime through the exported
      # packages.iso-builder script).  Two consumers, one source of
      # truth: the FOD and the flake-lib derivation BOTH invoke the same
      # r9-iso-builder script below — no duplicated wrapper logic.
      isoBuilderFor = pkgs:
        let
          # The committed loader-config TEMPLATE as a store file — the
          # same source xtask's build_x86_64_iso emits from (14-05 D-03):
          # kernel-only emission drops the @R9-MODULE* lines and stays
          # byte-identical to Phase 13's staged config; module-bearing
          # emission expands the placeholder to one module2 line per
          # staged module (the xtask emit_grub_cfg convention).
          grubCfg = pkgs.writeText "r9-grub.cfg" (builtins.readFile ./x86_64/lib/grub.cfg);
          # The argv-driven builder:
          #   r9-iso-builder KERNEL-ELF OUTDIR [MODULE-PATH MODULE-NAME ...]
          # stages the pinned-date xorriso pipeline (grub-mkrescue) and
          # writes OUTDIR/boot.iso.  Modules stage VERBATIM at
          # /boot/<MODULE-NAME> — a bootfs.paq is a paqfs archive, never
          # stripped (the kernel stage is strip-ALL, not the xtask dev
          # copy's strip-debug: rustc's per-invocation-random
          # .llvm.<u64> symbols make any symtab-carrying stage break ISO
          # byte-identity — the measured 13-06 result; the dev copy keeps
          # symbols for debugging, the distributable/composed ISO cannot.
          # The xtask dev ISO also runs grub-mkrescue without the
          # pinned-date xorriso wrapper — its per-invocation copy is
          # evidence-fresh, never hashed.  Those two deltas are the ONLY
          # intentional differences from the xtask staging; the template,
          # the strip direction, and the module staging match it).
          iso-builder = pkgs.writeShellApplication {
            name = "r9-iso-builder";
            # gawk: the module-bearing emission's awk program (kernel-only
            # builds never reach it, but Harvey's compose always stages
            # the bootfs module — the script must run standalone for BOTH
            # consumers).
            runtimeInputs = [ pkgs.grub2 pkgs.xorriso pkgs.binutils pkgs.gnused pkgs.coreutils pkgs.gawk ];
            text = ''
              # r9-iso-builder — the D-05 parameterized ISO builder (the
              # 13-06 boot-iso FOD body factored out verbatim, argv-driven).
              # One implementation behind three doors: r9's kernel-only
              # FOD, r9.lib.bootIso derivations, and Harvey's compose app.
              usage() {
                echo "usage: r9-iso-builder KERNEL-ELF OUTDIR [MODULE-PATH MODULE-NAME ...]" >&2
                echo "  KERNEL-ELF  the r9 x86_64 kernel ELF (strip-ALL staged at /boot/kernel)" >&2
                echo "  OUTDIR      receives boot.iso" >&2
                echo "  MODULE pairs  stage each module verbatim at /boot/<NAME> and emit" >&2
                echo "               its grub.cfg module2 line (zero modules = kernel-only" >&2
                echo "               ISO, the Phase-13-identical loader config)" >&2
                exit 2
              }
              err() { echo "r9-iso-builder: $*" >&2; exit 1; }

              [ "$#" -ge 2 ] || usage
              KERNEL=$1; OUTDIR=$2; shift 2
              [ -f "$KERNEL" ] || err "kernel $KERNEL not found"
              modpaths=(); modnames=()
              while [ "$#" -gt 0 ]; do
                [ "$#" -ge 2 ] || usage
                [ -f "$1" ] || err "module $1 not found"
                modpaths+=("$1"); modnames+=("$2"); shift 2
              done
              mkdir -p "$OUTDIR"

              # Reproducibility pins (13-06, measured on grub 2.14 +
              # xorriso 1.5.8.pl02): grub-mkrescue injects a CURRENT-TIME
              # --modification-date into its xorriso argv and has no
              # SOURCE_DATE_EPOCH support, and the staged/grub file mtimes
              # ride through into Rock Ridge TF records — the wrapper
              # below pins the modification date, overrides every file
              # date (--set_all_file_dates) and pins the volume id.
              export SOURCE_DATE_EPOCH="''${SOURCE_DATE_EPOCH:-1735689600}"
              export R9_ISO_DATE="''${R9_ISO_DATE:-2025010100000000}"
              export R9_ISO_VOLID="''${R9_ISO_VOLID:-R9BOOT}"

              WORK=$(mktemp -d "''${TMPDIR:-/tmp}/r9-iso-builder.XXXXXX")
              trap 'chmod -R u+w "$WORK" 2>/dev/null || true; rm -rf "$WORK"' EXIT

              xorrisoPinned="$WORK/xorriso-pinned"
              cat > "$xorrisoPinned" <<'WRAP'
#!/bin/sh
a1=$1; shift
a2=$1; shift
n=$#
i=0
while [ "$i" -lt "$n" ]; do
  x=$1
  shift
  case "$x" in
    --modification-date=*) x="--modification-date=$R9_ISO_DATE" ;;
  esac
  set -- "$@" "$x"
  i=$((i+1))
done
exec xorriso "$a1" "$a2" \
  "--modification-date=$R9_ISO_DATE" \
  "--set_all_file_dates" "$R9_ISO_DATE" \
  "-volid" "$R9_ISO_VOLID" \
  "$@"
WRAP
              chmod +x "$xorrisoPinned"

              # The grub module tree rides the ISO verbatim, and nixpkgs'
              # modinfo.sh carries a store-path bash shebang — a
              # fixed-output derivation must not reference store paths,
              # so graft from a copy with the shebang scrubbed.
              moddir="$WORK/grub-i386-pc"
              cp -r "${pkgs.grub2}/lib/grub/i386-pc" "$moddir"
              chmod -R u+w "$moddir"
              sed -i "s|^#![^ ]*bin/bash|#!/bin/sh|" "$moddir/modinfo.sh"

              stage="$WORK/stage"
              mkdir -p "$stage/boot/grub"

              # The staged kernel is a FULLY stripped copy — strip-all,
              # not the xtask dev-copy's strip-debug: rustc emits
              # per-invocation-random `.llvm.<u64>` suffixes on ~200
              # anonymous-namespace symbols (measured: two fresh sandbox
              # builds from identical source produce kernels differing
              # ONLY there), so any symtab-carrying stage breaks ISO
              # byte-identity.  GRUB's mb2 loader reads only PT_LOAD
              # segments (all intact under strip-all; the mb2 header
              # rides .text.boot).  The sed then zeroes the store-hash
              # runs inside build-std's panic-location strings
              # (/nix/store/<hash>-rust-default/... rides the kernel's
              # .rodata): a fixed-output derivation must not reference
              # store paths, and nix's scanner keys on the hash — the
              # length-preserving zero-zap keeps the strings readable
              # while breaking both the full-path and the hash match.
              objcopy --strip-all "$KERNEL" "$stage/boot/kernel"
              sed -E "s|/nix/store/[a-z0-9]{32}-|/nix/store/00000000000000000000000000000000-|g" -i "$stage/boot/kernel"

              # D-03 module staging: each module lands VERBATIM at
              # /boot/<NAME> (never stripped — paqfs archives), BEFORE
              # the loader config is emitted, so the emitted module2
              # lines never reference an absent file (GRUB fails a
              # menuentry whose module is missing).
              for i in "''${!modpaths[@]}"; do
                cp "''${modpaths[$i]}" "$stage/boot/''${modnames[$i]}"
              done

              # The loader config, emitted from the committed template
              # (14-05 D-03).  Kernel-only: the same sed rule the 13-06
              # FOD ran — the staged bytes stay byte-identical to Phase
              # 13's 13-line config (the awk emitter was cmp-proven
              # against this sed output on the committed template during
              # 14-07's re-proof).  Module-bearing: the placeholder line
              # expands IN PLACE to one module2 line per staged module —
              # indentation kept, payload re-keyed to the staged name,
              # every @R9-MODULE-DOC@ documentation line dropped (the
              # xtask emit_grub_cfg convention).
              if [ "''${#modnames[@]}" -eq 0 ]; then
                sed -e '/^# @R9-MODULE-DOC@/d' -e '/^[[:space:]]*# @R9-MODULE@ /d' \
                  "${grubCfg}" > "$stage/boot/grub/grub.cfg"
              else
                printf '%s\n' "''${modnames[@]}" | awk '
                  NR == FNR { names[++n] = $1; next }
                  /^[[:space:]]*# @R9-MODULE@ / {
                    match($0, /^[[:space:]]*/); indent = substr($0, 1, RLENGTH)
                    for (i = 1; i <= n; i++) print indent "module2 /boot/" names[i]
                    next
                  }
                  /^# @R9-MODULE-DOC@/ { next }
                  { print }
                ' - "${grubCfg}" > "$stage/boot/grub/grub.cfg"
              fi

              grub-mkrescue -d "$moddir" -o "$WORK/boot.iso" "$stage" --xorriso "$xorrisoPinned"
              cp "$WORK/boot.iso" "$OUTDIR/boot.iso"
              echo "r9-iso-builder: $OUTDIR/boot.iso $(sha256sum "$OUTDIR/boot.iso" | cut -d" " -f1) (modules: ''${modnames[*]:-none})"
            '';
          };
        in
        {
          inherit iso-builder;
          # The derivation former — pure-eval consumers (r9.lib.bootIso)
          # and r9's own packages.boot-iso both call this with the SAME
          # script; outputHash is injectable so the FOD wiring and the
          # hash-free byte-identity proof builds share one path (the
          # 13-06 discipline: wire a hash only after two independent
          # derivations agree).
          bootIso = { kernel-elf, modules ? [ ], outputHash ? null }:
            let
              moduleArgs = nixpkgs.lib.concatMapStringsSep " "
                (m: nixpkgs.lib.escapeShellArg m.path + " " + nixpkgs.lib.escapeShellArg m.name)
                modules;
            in
            pkgs.stdenv.mkDerivation ({
              pname = "r9-boot-iso";
              version = self.shortRev or "wip";
              # no src: every input rides the iso-builder script's
              # closure (the kernel via the kernel-elf argument's string
              # context, the template and the toolchain via the script)
              dontUnpack = true;
              buildPhase = ''
                runHook preBuild
                ${iso-builder}/bin/r9-iso-builder ${nixpkgs.lib.escapeShellArg kernel-elf} "$NIX_BUILD_TOP/iso" ${moduleArgs}
                runHook postBuild
              '';
              installPhase = ''
                runHook preInstall
                mkdir -p "$out"
                cp "$NIX_BUILD_TOP/iso/boot.iso" "$out/boot.iso"
                runHook postInstall
              '';
            } // nixpkgs.lib.optionalAttrs (outputHash != null) {
              outputHashAlgo = "sha256";
              outputHashMode = "recursive";
              inherit outputHash;
            });
        };
    in
    {
      # D-05's pure-eval surface: r9.lib.bootIso { system, kernel-elf,
      # modules, outputHash } → the ISO derivation built by the same
      # r9-iso-builder script r9's own boot-iso FOD uses.  (lib values
      # are functions, so `nix eval .#lib --json` cannot serialize them
      # — eval the function itself: nix eval .#lib.bootIso --apply
      # 'f: builtins.functionArgs f'.)
      lib = {
        bootIso = { system, kernel-elf, modules ? [ ], outputHash ? null }:
          (isoBuilderFor (pkgsFor system)).bootIso { inherit kernel-elf modules outputHash; };
      };

      # D-11 (05-10): the ONLY artifact Harvey's flake input consumes — a
      # derivation that builds the shell-capable aarch64 kernel ELF
      # (`cargo xtask build --arch aarch64 --config virtshell`, debug profile:
      # the gate default the apps and boot-test harness use) and installs it
      # plus the committed aarch64/lib/virt-gicv2.dtb.  The toolchain comes
      # from rust-toolchain.toml via toolchainFor (D-09's single-source rule —
      # never a restated pin); cargo deps are vendored from the committed
      # Cargo.lock via importCargoLock so the sandboxed build never touches
      # the network (T-05-19).  Harvey bakes its own initrd and launches qemu
      # itself — it consumes these as store paths, never by running xtask.
      packages = forAllSystems (system:
        let
          pkgs = pkgsFor system;
          toolchain = toolchainFor system;
          # `-Z build-std=core,alloc` resolves the rust sysroot graph from
          # the toolchain's rust-src component; that graph has its own
          # crates.io members (rustc-literal-escaper et al., pinned by
          # rust-src's library Cargo.lock), so the vendored set must cover
          # BOTH lockfiles or the sandboxed sysroot build dies offline
          # (the two sets are disjoint crate-name dirs — symlinkJoin merges).
          # The name is load-bearing: importCargoLock's outputs ship a
          # .cargo/config.toml whose vendored-sources directory is the
          # RELATIVE "cargo-vendor-dir", and cargoSetupHook prefers that
          # shipped config — the joined output must keep the name so the
          # hook's copy lands where the config points.
          cargoDeps = pkgs.symlinkJoin {
            name = "cargo-vendor-dir";
            paths = [
              (pkgs.rustPlatform.importCargoLock { lockFile = ./Cargo.lock; })
              (pkgs.rustPlatform.importCargoLock {
                lockFile = "${toolchain}/lib/rustlib/src/rust/library/Cargo.lock";
              })
            ];
          };
          # X64-05 (13-06): the x86_64 sibling of kernel-virt, built the
          # same way (same cargoDeps symlinkJoin, same toolchain/
          # cargoSetupHook inputs) — the Phase-14 compose contract:
          # r9.packages.<system>.kernel-x64.  14-07 re-keyed the build
          # config to q35shell: the compose consumes this kernel to boot
          # Harvey's bootfs chain (exec + bootchain devs required — the
          # plain q35 profile carries no user mode), and the shipped
          # artifact is the gated artifact (q35shell is the 14-06
          # full-chain gate, green twice consecutively).
          # 17-02 re-keyed it to q35churnshell — q35shell VERBATIM plus
          # the churn dev key — so the composed venue's kernel carries
          # the per-exit churngate heap marker (the composed churn
          # gate's memory-side probe, same cfg gate as the r9-side
          # tier): the boot chain, every device, and every existing
          # composed gate's semantics are unchanged (the churn DRIVER
          # never runs under dev_bootchain); q35shell.toml itself stays
          # byte-stable (Pitfall 7).
          kernel-x64 = pkgs.stdenv.mkDerivation {
            pname = "r9-kernel-x64";
            version = self.shortRev or "wip";
            src = ./.;
            nativeBuildInputs = [ toolchain pkgs.rustPlatform.cargoSetupHook ];
            inherit cargoDeps;
            buildPhase = ''
              runHook preBuild
              ${objcopyExport toolchain}
              cargo xtask build --arch x86-64 --config q35churnshell
              runHook postBuild
            '';
            installPhase = ''
              runHook preInstall
              mkdir -p "$out"
              cp target/x86_64-unknown-none-elf/debug/x86_64 "$out/kernel-x64.elf"
              runHook postInstall
            '';
          };
          # 16-01 (D-05): the exported netdev emitter — a HOST-STD bin
          # wrapping the ONE shared builder (xtask's
          # config::netdev_args) with the committed config_q35shell.toml
          # embedded at build time.  Harvey's flake consumes
          # packages.<system>.netdev-args at the 16-04 pin advance
          # (boot-r9-x64.sh substitutes this bin's stdout for its
          # hardcoded -netdev/-device lines).  Mirrors kernel-x64's
          # skeleton (cargoSetupHook + the same cargoDeps symlinkJoin)
          # minus the objcopy/sysroot machinery — plain cargo builds
          # this host bin; reshaping the export later re-touches both
          # lockfiles and both call sites (the CONTEXT D-05 costly
          # rating).
          netdev-args = pkgs.stdenv.mkDerivation {
            pname = "r9-netdev-args";
            version = self.shortRev or "wip";
            src = ./.;
            nativeBuildInputs = [ toolchain pkgs.rustPlatform.cargoSetupHook ];
            inherit cargoDeps;
            buildPhase = ''
              runHook preBuild
              cargo build --release --bin r9-netdev-args
              runHook postBuild
            '';
            installPhase = ''
              runHook preInstall
              mkdir -p "$out/bin"
              cp target/release/r9-netdev-args "$out/bin/r9-netdev-args"
              runHook postInstall
            '';
          };
          builder = isoBuilderFor pkgs;
        in
        {
          kernel-virt = pkgs.stdenv.mkDerivation {
            pname = "r9-kernel-virt";
            version = self.shortRev or "wip";
            src = ./.;
            nativeBuildInputs = [ toolchain pkgs.rustPlatform.cargoSetupHook ];
            inherit cargoDeps;
            buildPhase = ''
              runHook preBuild
              ${objcopyExport toolchain}
              cargo xtask build --arch aarch64 --config virtshell
              runHook postBuild
            '';
            installPhase = ''
              runHook preInstall
              mkdir -p "$out/aarch64/lib"
              # The store-path contract Harvey's compose app (05-11) reads:
              # the built kernel ELF, and the committed DTBs at their
              # repo-relative paths (gicv2 = the TCG reference shape;
              # gicv3 = the hvf/TCG gic-version=3 shape — the kernel
              # dispatches on the DTB's compatible string, so one ELF
              # serves both).
              cp target/aarch64-unknown-none-elf/debug/aarch64 "$out/kernel-virt.elf"
              cp aarch64/lib/virt-gicv2.dtb "$out/aarch64/lib/virt-gicv2.dtb"
              cp aarch64/lib/virt-gicv3.dtb "$out/aarch64/lib/virt-gicv3.dtb"
              runHook postInstall
            '';
          };
        }
        # The x86_64 deliverables: the kernel package and the netdev
        # emitter on every linux host (the rustc target spec is
        # cross-platform; the emitter is host-std); the BIOS ISO
        # machinery only where grub2 ships the i386-pc modules it embeds
        # — the q35 channel's loaders and gates are x86_64-native
        # (X64-05).  14-07 (D-05): iso-builder (the argv-driven script
        # Harvey's compose app consumes) exports beside boot-iso (the
        # kernel-only FOD, re-expressed through the same builder).
        # 16-01 (D-05): netdev-args exports beside kernel-x64 — its own
        # gate is this same isLinux optionalAttrs (r9's own package
        # surface; the frozen-list anti-pattern applies to Harvey's
        # consumer side, not here).
        // nixpkgs.lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
          inherit kernel-x64 netdev-args;
        }
        // nixpkgs.lib.optionalAttrs (system == "x86_64-linux") {
          inherit (builder) iso-builder;
          boot-iso = builder.bootIso {
            kernel-elf = "${kernel-x64}/kernel-x64.elf";
            # Fixed-output.  13-06 wired the first hash only after a
            # two-derivation byte-identity proof; every Phase-14 kernel
            # commit since 14-01 changed the kernel ELF bytes the FOD
            # stages, so the 13-06 hash went stale THEN (proven
            # pre-existing at abee15c — 14-05's deferred-items note).
            # 14-07 re-wired it after the refactor's own byte-preservation
            # proof plus fresh independent-derivation proofs (same
            # discipline, new kernel bytes): the old 13-06 FOD body and
            # this builder, run over FOUR independent fresh kernel builds
            # (two per body), produced the IDENTICAL ISO — file-level
            # sha256 cdcfa00835c0ee37b0b7cbc2064ed8511db632408699faaad8814b49a885118a,
            # NAR hash (what this attr checks)
            # 944aad869ee1d0974037cad4208b48fb753af84401fad12a2340e3aba994a99b
            # (sha256-lEqthp7h…, wired at fe39d83).
            # The phase's own post-fe39d83 kernel commits (8a338a6 console
            # RX, e5def72 SWAPGS cmpw, e417660 user SSE + eager stack —
            # the phase-close pin) re-staled that hash: the drift class is
            # standing (ANY kernel-byte commit re-stales this FOD; the pin
            # discipline requires a re-wire at close).  The 14-VERIFICATION
            # gap fix re-wired it at e417660 with the same two-derivation
            # proof: two independent fresh-kernel builds (version-override
            # clones) through the non-FOD bootIso realization produced the
            # byte-identical ISO — file-level sha256
            # bd0281bbc00f96f0b7a615bdf23e17b0c69cc00059733fc64abb6304b74804e3,
            # NAR hash (what this attr checks)
            # 9706c34aa63160d1bc648863903b338879448c53372493846229e5def6ca59ac
            # — and `nix build .#boot-iso` + `--rebuild` re-verify at the
            # wired value.  The 14 code-review fixes (CR-01's shared
            # seed_tos_va exec pre-check, WR-01's live ax@0 assert, WR-02's
            # l.S tag-walk hardening — kernel bytes again) re-staled it
            # once more; re-wired at 86a8677 with the same two-derivation
            # proof: two independent fresh-kernel builds (version-override
            # clones reviewfix-proof-a/-b) through the non-FOD bootIso
            # realization produced the byte-identical ISO — file-level
            # sha256
            # 2be9a177cea8c6e59b080b6bf45e67bc85dd2506f3d651d16c6c797960889be5,
            # NAR hash (what this attr checks)
            # 3cb7f2637bf8f10aeea2760af9327b27e0a2ea855a085f3cc24ce1145d925735
            # (sha256-PLfyY3v…), in three-way agreement with the FOD's own
            # measured got-value.  The IN fixes (IN-01's park-print
            # removal from reschedule's hot path, IN-02's dead
            # touser/UTZERO deletion — kernel bytes again) re-staled it
            # once more; re-wired at 2679bb7 with the same
            # two-derivation proof: two independent fresh-kernel builds
            # (version-override clones reviewfix2-proof-a/-b) through
            # the non-FOD bootIso realization produced the
            # byte-identical ISO — file-level sha256
            # b0435dd294c6face846032c3535ea389a2296146fdbefed095278d9b62044a73,
            # NAR hash (what this attr checks)
            # ada738310c47b9236f48f1c691129e4d2faa66095acc140ba2def8d1d8ee7ade
            # (sha256-rac4MQx…), in three-way agreement with the FOD's own
            # measured got-value.  The 15-03 services fixes re-staled it
            # once more and were re-wired at 3362cf7 (NCK9JST5… above);
            # the 15-08 DESK-03 module-stash raise (8->64 MiB — kernel
            # bytes again) re-staled it in turn; re-wired at the fix
            # commit with the same two-derivation proof: two independent
            # fresh-kernel builds (version-override clones
            # desk03-proof-a/-b) through the non-FOD bootIo realization
            # produced the byte-identical ISO — file-level sha256
            # 1a175703d2e6492f935dc23029a91a97c8f0b33eab4af69e0a841955917a37b2,
            # NAR hash (what this attr checks)
            # ab9e19a76fcb1120ee4b08c8c14e0451e45ad85c0182f68a05c7422af98949be
            # (sha256-q54Zp2…), in three-way agreement with the FOD's own
            # measured got-value.  The 17-01 WR-03 OOM-release fixes
            # (pml4_map_user's walk-failure unwind, map_user_page's leaf
            # free, rfork_copy's copy-arm free, new_user_proc's
            # kstack-table prefix — kernel bytes again) re-staled it in
            # turn; re-wired with the same two-derivation proof: two
            # independent fresh-kernel builds (version-override clones
            # wr03fix-proof-a/-b) through the non-FOD bootIo realization
            # produced the byte-identical ISO — file-level sha256
            # 1cd4ef4036cfba865d54b9117529b1d3848a90c7852f58b0b9ca53d005d9405b,
            # NAR hash (what this attr checks)
            # 2bad0ee078c4d2cd1eb2e08dab4c11dfe41007c06b169516fc6bf7c95023da15
            # (sha256-K60O4HjE…), in three-way agreement with the FOD's own
            # measured got-value.
            # The 17-02 churn commits (the churn profile + the two
            # root-caused kernel fixes — kernel bytes, plus kernel-x64
            # re-pointed at q35churnshell) re-staled it once more;
            # re-wired at 17-02's close with the same two-derivation
            # proof: two independent fresh-kernel builds (kernel A plain
            # + kernel B under a never-before-used churnproof-b version
            # override) through the non-FOD lib.bootIso realization
            # produced the byte-identical ISO — file-level sha256
            # 16bef2bffc095dfbd3c2577d6006ac252bf154cb661cdef554cb9963a8d73340,
            # NAR hash (what this attr checks)
            # c0b656a0678278c3107c078744b93f8427d02a8fac9d8ae7b66e89b72744c912
            # (sha256-wLZWhm…), in three-way agreement with the FOD's own
            # measured got-value.
            # The 17-02 sweep fix (slash/dot plain-drop — kernel bytes)
            # re-staled it within the same plan; re-wired with the same
            # two-derivation proof (kernel A plain + kernel B under a
            # churnproof-c version override): byte-identical ISOs, file
            # sha256
            # fc5cde5559cb1728433e7392aea0480400978ca7d07a09b44c3e760dabd4accf,
            # NAR hash (what this attr checks)
            # f05ec06a8a736f6c10474277d80a8176da7ac424fdcf2a08d8eb38393ba4391
            # (sha256-bw7Ank…).  nix build + --rebuild green at the wired
            # value.
            # The 17-03 WR-05/WR-06 hardening fixes (the WaitSet resize
            # to MAX_WAITERS=512 + the loud full arm across netif/devip,
            # and the used-ring unposted-slot liveness reject — port
            # bytes, kernel bytes with them) re-staled it once more;
            # re-wired with the same two-derivation proof: two
            # independent fresh-kernel builds (kernel A plain + kernel B
            # under a never-before-used wr05wr06-proof-b version
            # override) through the non-FOD lib.bootIso realization
            # produced the byte-identical ISO — file-level sha256
            # 8f3f6e2aa172bd6ce4741dc80da036bb87d0ebbe97b8c32ec977fde683856bda,
            # NAR hash (what this attr checks)
            # d0a19c8335db1408535d4a216f810d15f44d19e8ba21f8909e9cb96eb29d6b22
            # (sha256-0KGcgz…), in three-way agreement with the FOD's
            # own measured got-value.
            outputHash = "sha256-0KGcgzXbFAhTXUohb4ENFfRNGei6IfiQnpy5brKdayI=";
          };
        });

      devShells = forAllSystems (system:
        let
          pkgs = pkgsFor system;
          toolchain = toolchainFor system;
        in
        {
          default = pkgs.mkShellNoCC {
            # grub2 + xorriso (13-01): grub-mkrescue for the x86_64 mb2
            # boot channel — permanently in the devshell so `nix develop`
            # builds the q35 ISO without a wrapping `nix shell`.
            packages = [ toolchain pkgs.qemu pkgs.dtc pkgs.grub2 pkgs.xorriso ];
            shellHook = ''
              ${objcopyHook toolchain}
              echo "r9 devshell: $(rustc --version)"
              echo "$(qemu-system-aarch64 --version | head -1)"
              echo "apps: nix run .#build | nix run .#boot-virt | nix run .#default"
            '';
          };
        });

      apps = forAllSystems (system:
        let
          pkgs = pkgsFor system;
          toolchain = toolchainFor system;
          # writeShellApplication embeds runtimeInputs on PATH, so the app
          # runs bare (no devshell) per D-08's zero-host-setup intent.
          r9-build = pkgs.writeShellApplication {
            name = "r9-build";
            runtimeInputs = [ toolchain pkgs.qemu ];
            text = ''
              ${objcopyExport toolchain}
              exec cargo xtask build --arch aarch64 "$@"
            '';
          };
          # D-11 acceptance harness: boot virt on a serial socket and assert
          # banner -> >=3 ticks -> typed-char echo (SC#3/SC#4), writing the
          # captured log to target/boot-test-virt.log.
          r9-boot-virt = pkgs.writeShellApplication {
            name = "r9-boot-virt";
            runtimeInputs = [ toolchain pkgs.qemu ];
            text = ''
              ${objcopyExport toolchain}
              exec cargo xtask boot-test --arch aarch64 --config virt "$@"
            '';
          };
        in
        {
          build = { type = "app"; program = "${r9-build}/bin/r9-build"; };
          boot-virt = { type = "app"; program = "${r9-boot-virt}/bin/r9-boot-virt"; };
          default = { type = "app"; program = "${r9-build}/bin/r9-build"; };
        });
    };
}
