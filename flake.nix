{
  description = "r9 dev environment — pinned rust toolchain, qemu, cargo xtask";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  inputs.rust-overlay.url = "github:oxalica/rust-overlay";
  inputs.rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  # 01-10 (D-09, D-23, D-06): Harvey as a normal flake input (never the
  # spike flakes' non-flake input form), pinned by flake.lock to a pushed
  # commit. The url carries no ref so a pin bump is a one-commit, lock-only
  # change (`nix flake lock --override-input harvey github:requaos/harvey/<sha>`).
  # The nixpkgs follow is the A13 probe: it keeps one nixpkgs in the lock
  # graph and is confirmed by the harvey-harness / harvey-drawterm checks
  # below. NB: no follow is set on Harvey's own `r9` input — Harvey's
  # outputs function takes `r9`, so an emptied follow would break its
  # evaluation (01-RESEARCH.md Pitfall 8); the nested harvey/r9 lock node
  # (an older r9 revision) is expected until the D-10 flip's pin bump.
  inputs.harvey.url = "github:requaos/harvey";
  inputs.harvey.inputs.nixpkgs.follows = "nixpkgs";

  outputs = { self, nixpkgs, rust-overlay, harvey }:
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
      # 01-18 (D-07/D-09/HARN-08): the moved gate-app wrapper, copied
      # from Harvey bd7a698's flake (L26-39): a writeShellApplication
      # carries its runtimeInputs on PATH (the app runs bare, no
      # devshell) and prepends the plan9port bin LAST per BUILD_ON_UNIX.
      mkScriptApp = pkgs: name: file: extraHeader: runtimeInputs:
        pkgs.writeShellApplication {
          inherit name runtimeInputs;
          text = ''
            set -euo pipefail
            export PLAN9="${pkgs.plan9port}/plan9"
            # plan9 bin LAST per BUILD_ON_UNIX — plan9port ships a 9p
            # `mount` which must not shadow util-linux mount in the
            # namespace build.
            export PATH="$PATH:$PLAN9/bin"
            ${extraHeader}
            ${builtins.readFile file}
          '';
        };
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
          #   r9-iso-builder KERNEL-ELF OUTDIR [--append=CMDLINE] [MODULE-PATH MODULE-NAME ...]
          # stages the pinned-date xorriso pipeline (grub-mkrescue) and
          # writes OUTDIR/boot.iso.  --append=CMDLINE substitutes for
          # the template's @R9-APPEND@ token on the multiboot2 line
          # (19-01, CPU-02: the mb2 tag-1 kernel cmdline the kernel
          # parses into the ROOT env seed's bootconf pairs; absent =
          # the token strips and the emission stays byte-identical to
          # the no-append shape — the wired FOD hash rides on those
          # bytes).  Modules stage VERBATIM at
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
                echo "usage: r9-iso-builder KERNEL-ELF OUTDIR [--append=CMDLINE] [MODULE-PATH MODULE-NAME ...]" >&2
                echo "  KERNEL-ELF  the r9 x86_64 kernel ELF (strip-ALL staged at /boot/kernel)" >&2
                echo "  OUTDIR      receives boot.iso" >&2
                echo "  --append=CMDLINE  substitute CMDLINE for the template's @R9-APPEND@" >&2
                echo "               token on the multiboot2 line (the mb2 tag-1 kernel cmdline;" >&2
                echo "               absent = token stripped, the no-append emission stays" >&2
                echo "               byte-identical)" >&2
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
              append=""
              while [ "$#" -gt 0 ]; do
                case "$1" in
                  --append=*)
                    append="''${1#--append=}"
                    [ -n "$append" ] || err "--append= requires a non-empty cmdline"
                    shift ;;
                  *)
                    [ "$#" -ge 2 ] || usage
                    [ -f "$1" ] || err "module $1 not found"
                    modpaths+=("$1"); modnames+=("$2"); shift 2 ;;
                esac
              done
              # Compose-time append validation (19-01, the seam's teeth):
              # the kernel rejects an overlong bootconf LOUDLY at first
              # boot — the builder refuses EARLIER, never staging an ISO
              # the boot would refuse.  Non-printable bytes would corrupt
              # the loader line or the NUL-terminated tag string.
              _alen=$(printf '%s' "$append" | wc -c | tr -d ' ')
              [ "$_alen" -le 256 ] \
                || err "--append= is $_alen bytes; the kernel cmdline cap is 256 (mb2 MAX_CMDLINE)"
              if [ -n "$append" ] && [ -z "$(printf '%s' "$append" | tr -d '[:space:]')" ]; then
                err "--append= carries no bootconf content (whitespace only)"
              fi
              case "$append" in
                *[![:print:]]*) err "--append= carries non-printable bytes" ;;
              esac
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
              # (14-05 D-03; 19-01 adds the append token).  Kernel-only
              # AND append-less: the same sed rule the 13-06 FOD ran,
              # extended only to drop the @R9-APPEND-DOC@ documentation
              # lines and strip the @R9-APPEND@ token — the staged bytes
              # stay byte-identical to Phase 13's 13-line config (the
              # awk emitter was cmp-proven against this sed output on
              # the committed template during 14-07's re-proof).
              # Module-bearing and/or appended: the awk emitter — the
              # placeholder line expands IN PLACE to one module2 line
              # per staged module (indentation kept, payload re-keyed to
              # the staged name), the append token substitutes through
              # index/substr with the replacement in ENVIRON (raw bytes:
              # no escape reprocessing, no regex & semantics), and every
              # @R9-MODULE-DOC@/@R9-APPEND-DOC@ documentation line drops
              # (the xtask emit_grub_cfg convention).
              if [ "''${#modnames[@]}" -eq 0 ] && [ -z "$append" ]; then
                sed -e '/^# @R9-MODULE-DOC@/d' -e '/^[[:space:]]*# @R9-MODULE@ /d' \
                    -e '/^# @R9-APPEND-DOC@/d' -e 's/@R9-APPEND@//' \
                  "${grubCfg}" > "$stage/boot/grub/grub.cfg"
              else
                if [ -n "$append" ]; then
                  export R9_APPEND_SUBST=" $append"
                else
                  export R9_APPEND_SUBST=""
                fi
                {
                  if [ "''${#modnames[@]}" -gt 0 ]; then
                    printf '%s\n' "''${modnames[@]}"
                  fi
                } | awk '
                  NR == FNR { names[++n] = $1; next }
                  /^[[:space:]]*# @R9-MODULE@ / {
                    match($0, /^[[:space:]]*/); indent = substr($0, 1, RLENGTH)
                    for (i = 1; i <= n; i++) print indent "module2 /boot/" names[i]
                    next
                  }
                  /^# @R9-MODULE-DOC@/ { next }
                  /^# @R9-APPEND-DOC@/ { next }
                  {
                    line = $0
                    tok = "@R9-APPEND@"
                    i = index(line, tok)
                    if (i > 0)
                      line = substr(line, 1, i-1) ENVIRON["R9_APPEND_SUBST"] substr(line, i+length(tok))
                    print line
                  }
                ' - "${grubCfg}" > "$stage/boot/grub/grub.cfg"
              fi

              grub-mkrescue -d "$moddir" -o "$WORK/boot.iso" "$stage" --xorriso "$xorrisoPinned"
              cp "$WORK/boot.iso" "$OUTDIR/boot.iso"
              echo "r9-iso-builder: $OUTDIR/boot.iso $(sha256sum "$OUTDIR/boot.iso" | cut -d" " -f1) (modules: ''${modnames[*]:-none}''${append:+ append=$append})"
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
          # 01-13 (HARN-07, D-03, D-24): the Harvey supply.  A Linux tree
          # fetches the source on the builder ITSELF through a fixed-output
          # derivation keyed on the locked rev + narHash, so the source never
          # round-trips through the Mac's case-insensitive APFS store (S2's
          # `s2-casecheck-fod` shape).  A session-only `--override-input
          # harvey path:...` has no rev and uses its outPath instead (D-06).
          harveyTreeSrc =
            if harvey ? rev && harvey ? narHash then
              pkgs.fetchFromGitHub {
                owner = "requaos";
                repo = "harvey";
                rev = harvey.rev;
                hash = harvey.narHash;
              }
            else
              harvey.outPath;
          # The one build environment every tree derivation rides: Harvey's
          # host toolchain plus the proot venue (D-25; no unshare).  Its
          # `mkTreeDerivation` takes { name, arch, guestScript, installScript }.
          harveyEnv = import ./nix/harvey/build-env.nix {
            inherit pkgs;
            harveySrc = harveyTreeSrc;
          };
          harveyTreeArm64 = import ./nix/harvey/tree-arm64.nix {
            inherit pkgs;
            harveySrc = harveyTreeSrc;
            rev = harvey.rev or "unknown";
            narHash = harvey.narHash or "unknown";
          };
          # 01-14 (HARN-07, D-19, D-25): the full amd64 tree — every tier,
          # the net section, the stage-build programs and the source-side
          # fixtures the assembler and the moved gates read.
          harveyTreeAmd64 = import ./nix/harvey/tree-amd64.nix {
            inherit pkgs;
            harveySrc = harveyTreeSrc;
            rev = harvey.rev or "unknown";
            narHash = harvey.narHash or "unknown";
          };
          # 01-15 (D-29, HARN-03): the reference bakes — Harvey's own
          # mkimage.sh run ONCE per architecture on a writable copy of the
          # built tree, recording the image contracts' mkpaqfs inputs as
          # member lists.  Plain stdenv venue (mkimage is a host shell
          # bake; no Plan 9 namespace needed).  KERNDATE is the flake
          # input's commit time (the pin's %ct); a rev-less session
          # override falls back to bd7a698's commit time so a path input
          # still bakes deterministically.  01-17 adds the arm64 sibling
          # (one bootfs contract) beside the amd64 one.
          harveyReference = import ./nix/harvey/reference.nix {
            inherit pkgs;
            tree = harveyTreeAmd64;
            treeArm64 = harveyTreeArm64;
            hostbin = harveyEnv.hostbin;
            mkpaqfs = harveyMkpaqfs;
            rev = harvey.rev or "unknown";
            kerndate = harvey.lastModified or 1795382101;
          };
          harveyReferenceAmd64 = harveyReference.amd64;
          harveyReferenceArm64 = harveyReference.arm64;
          # 01-14 (D-16, T-01-48): the host p9sk1 domain generator r9's
          # assembler runs at bake time (R9_AUTHDOMAIN_GEN).  Tool only —
          # no domain is ever generated inside the derivation.
          #
          # Linux only: the vendored donor crypt.c declares
          # `encrypt(void*, void*, int)`, which collides with macOS
          # <unistd.h>'s `encrypt(char *, int)` under clang.  The plan's
          # documented contingency: the cpu/auth images then assemble on
          # Linux, and their Mac checks are parse/drift only.
          harveyAuthdomainGen = import ./nix/harvey/authdomain-gen.nix {
            inherit pkgs;
            harveySrc = harvey.outPath;
          };
          # The host mkpaqfs builds locally on every system from the flake
          # input path (it is a plan9port host tool, not a tree member).
          harveyMkpaqfs = import ./nix/harvey/mkpaqfs.nix {
            inherit pkgs;
            harveySrc = harvey.outPath;
          };
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
          # 20-15 (ARM-09, the owner's separate-profile decision at 20-12):
          # the devcap-carrying arm64 kernel — kernel-virt's derivation with
          # --config virtcpu (virtshell's dev set plus "cap").  The SAME
          # store-path contract (kernel-virt.elf + both DTBs at their
          # repo-relative paths), so Harvey's launchers consume it through
          # the existing R9_KERNEL_PKG seam (boot-r9-cpu, 20-16) while
          # kernel-virt keeps serving the smoke/net-smoke/tier1 gates.
          kernel-virt-cpu = pkgs.stdenv.mkDerivation {
            pname = "r9-kernel-virt-cpu";
            version = self.shortRev or "wip";
            src = ./.;
            nativeBuildInputs = [ toolchain pkgs.rustPlatform.cargoSetupHook ];
            inherit cargoDeps;
            buildPhase = ''
              runHook preBuild
              ${objcopyExport toolchain}
              cargo xtask build --arch aarch64 --config virtcpu
              runHook postBuild
            '';
            installPhase = ''
              runHook preInstall
              mkdir -p "$out/aarch64/lib"
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
          # 01-13 (HARN-07): the Harvey build supply.  `harvey-hostbin` is
          # the tree toolchain; `harvey-tree-arm64` the smoke slice of the
          # arm64 tree (01-17 extends the same machinery).
          harvey-hostbin = harveyEnv.hostbin;
          harvey-tree-arm64 = harveyTreeArm64;
          # 01-14 (HARN-07): the full amd64 tree.
          harvey-tree-amd64 = harveyTreeAmd64;
          # 01-15 (D-29): the amd64 reference bake (nine member lists).
          harvey-reference-amd64 = harveyReferenceAmd64;
          # 01-17 (D-29): the arm64 reference bake (bootfs.members).
          harvey-reference-arm64 = harveyReferenceArm64;
          # 01-14 (D-16): the host authdomain-gen (Linux only — the donor
          # crypt.c's encrypt() clashes with macOS's unistd.h declaration).
          harvey-authdomain-gen = harveyAuthdomainGen;
        }
        // {
          # The host mkpaqfs on all three systems (01-08's R9_MKPAQFS).
          harvey-mkpaqfs = harveyMkpaqfs;
        }
        // nixpkgs.lib.optionalAttrs (system == "x86_64-linux") {
          inherit (builder) iso-builder;
          boot-iso = builder.bootIso {
            kernel-elf = "${kernel-x64}/kernel-x64.elf";
            # Pure derivation (Phase 5 CORR-02, D-05): no fixed-output
            # outputHash — Nix computes the output identity strictly
            # from the pinned workspace inputs (kernel-x64, iso-builder,
            # grub.cfg).  Green CI on the commit is now the per-commit
            # x86_64 kernel byte-change check; the historical FOD hash
            # re-derivation log (13-06 through plan 04-15) lives in git
            # history.
          };
        });

      # 01-10 (D-09, A13): the A13 probe. With
      # `inputs.harvey.inputs.nixpkgs.follows = "nixpkgs"` above, Harvey's
      # packages build against r9's nixpkgs pin instead of Harvey's own.
      # harvey-harness must BUILD on the Mac (aarch64-darwin) and Harvey's
      # drawterm (a Linux-only package) must EVALUATE for both Linux
      # systems; checks.x86_64-linux.harvey-drawterm is also the stop-A
      # witness's a13 leg (witness/run-witness.sh step_a13). If the follow
      # breaks either, it is removed and A13 is recorded REFUTED in
      # 01-10-SUMMARY.md.
      checks = forAllSystems (system:
        let
          pkgs = pkgsFor system;
        in
        {
          harvey-harness = harvey.packages.${system}.harness;
        }
        // nixpkgs.lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
          harvey-drawterm = harvey.packages.${system}.drawterm;
        });

      devShells = forAllSystems (system:
        let
          pkgs = pkgsFor system;
          toolchain = toolchainFor system;
          # D-13 (RESEARCH C3, Pitfall 2): a second derivation for
          # `cargo xtask miri` only.  The extension list repeats
          # rust-toolchain.toml's four components plus `miri`, which the
          # "never restate pins" rule above would otherwise forbid: adding
          # `miri` to rust-toolchain.toml moves the toolchain store path that
          # is embedded over a thousand times in each kernel, so the pinned
          # kernel toolchain must not change.  The kernel package derivations
          # keep `toolchainFor`; this binding is never added to `packages`
          # (both toolchains provide `cargo` and `rustc`).
          toolchainMiri = toolchain.override {
            extensions = [ "rustfmt" "rust-src" "clippy" "llvm-tools" "miri" ];
          };
          # 01-13 (HARN-07, D-21): the lazy per-architecture Harvey tree.
          # On the Mac the tree is the aarch64-linux package realised by the
          # registered builder; R9_HARVEY_TREE_SYSTEM tells xtask's
          # realise-on-first-use which attribute to ask for (D-05).
          treeSystem =
            if pkgs.stdenv.hostPlatform.isLinux then system else "aarch64-linux";
          harveyTreeArm64 = self.packages.${treeSystem}.harvey-tree-arm64;
          # 01-14: the full amd64 tree, same lazy discipline (on the Mac it
          # is the aarch64-linux package realised by the registered builder).
          harveyTreeAmd64 = self.packages.${treeSystem}.harvey-tree-amd64;
          # 01-15 (D-29): the reference bake, same lazy discipline.
          harveyReferenceAmd64 = self.packages.${treeSystem}.harvey-reference-amd64;
          # 01-17 (D-29): the arm64 reference bake (bootfs.members).
          harveyReferenceArm64 = self.packages.${treeSystem}.harvey-reference-arm64;
          # A9: a context-discarded outPath is a plain string — entering the
          # shell sends no build, and the later `nix build` prints the same
          # path (S2's lazy-devshell proof).
          harveyTreeArm64Path =
            builtins.unsafeDiscardStringContext harveyTreeArm64.outPath;
          harveyTreeAmd64Path =
            builtins.unsafeDiscardStringContext harveyTreeAmd64.outPath;
          harveyReferenceAmd64Path =
            builtins.unsafeDiscardStringContext harveyReferenceAmd64.outPath;
          harveyReferenceArm64Path =
            builtins.unsafeDiscardStringContext harveyReferenceArm64.outPath;
          # mkpaqfs and authdomain-gen are the EAGER exports: small host
          # tools, not trees, and every image assembly/bake needs them.
          # authdomain-gen is Linux-only (the donor crypt.c/encrypt clash
          # with macOS unistd.h); on the Mac the cpu/auth bakes stay on the
          # Linux host per the plan's venue note.
          harveyMkpaqfs = self.packages.${system}.harvey-mkpaqfs;
          harveyAuthdomainGen =
            if pkgs.stdenv.hostPlatform.isLinux
            then self.packages.${system}.harvey-authdomain-gen
            else null;
        in
        {
          default = pkgs.mkShellNoCC {
            # grub2 + xorriso (13-01): grub-mkrescue for the x86_64 mb2
            # boot channel — permanently in the devshell so `nix develop`
            # builds the q35 ISO without a wrapping `nix shell`.  Linux-only
            # since 20-04: grub2 has no darwin hostPlatform.
            # 01-18: the moved gate tools — expect + Xvfb + xdotool are
            # the drawterm driver's feed/venue, and Harvey's drawterm is
            # the gate binary (D-09); `cargo xtask gate` finds them here.
            packages = [ toolchain pkgs.qemu pkgs.dtc ]
              ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux [
                pkgs.grub2
                pkgs.xorriso
                pkgs.expect
                pkgs.xorgserver
                pkgs.xdotool
                pkgs.procps
                pkgs.iproute2
                harvey.packages.${system}.drawterm
              ];
            shellHook = ''
              ${objcopyHook toolchain}
              # 01-13 (HARN-07, D-21): the Harvey supply.  Each export is
              # guarded so a value the user already set is kept (D-05), and
              # R9_HARVEY_DIR is deliberately NEVER exported — it is the
              # manual override only (D-01).
              if [ -z "''${R9_HARVEY_TREE_SYSTEM:-}" ]; then
                export R9_HARVEY_TREE_SYSTEM="${treeSystem}"
              fi
              if [ -z "''${R9_HARVEY_DIR_ARM64:-}" ]; then
                export R9_HARVEY_DIR_ARM64="${harveyTreeArm64Path}"
              fi
              if [ -z "''${R9_HARVEY_DIR_AMD64:-}" ]; then
                export R9_HARVEY_DIR_AMD64="${harveyTreeAmd64Path}"
              fi
              # 01-15 (D-29): the amd64 reference bake (nine member lists).
              # Lazy like the trees: entering the shell sends no build; the
              # first --compare-reference/--emit-manifest realises it.
              if [ -z "''${R9_HARVEY_REFERENCE_AMD64:-}" ]; then
                export R9_HARVEY_REFERENCE_AMD64="${harveyReferenceAmd64Path}"
              fi
              # 01-17 (D-29): the arm64 reference bake (bootfs.members).
              if [ -z "''${R9_HARVEY_REFERENCE_ARM64:-}" ]; then
                export R9_HARVEY_REFERENCE_ARM64="${harveyReferenceArm64Path}"
              fi
              if [ -z "''${R9_MKPAQFS:-}" ]; then
                export R9_MKPAQFS="${harveyMkpaqfs}/bin/mkpaqfs"
              fi
              # D-13: Miri's own toolchain (RESEARCH C3).  The eager form,
              # like R9_MKPAQFS; a value the user already set wins.
              if [ -z "''${R9_MIRI_TOOLCHAIN:-}" ]; then
                export R9_MIRI_TOOLCHAIN="${toolchainMiri}"
              fi
              # D-13: append the Miri toolchain's bin so `cargo miri` finds
              # `cargo-miri` even when cargo is invoked by absolute path (the
              # step itself prepends it).  Appending never shadows the pinned
              # toolchain's cargo or rustc.
              export PATH="$PATH:$R9_MIRI_TOOLCHAIN/bin"
              ${pkgs.lib.optionalString (harveyAuthdomainGen != null) ''
              if [ -z "''${R9_AUTHDOMAIN_GEN:-}" ]; then
                export R9_AUTHDOMAIN_GEN="${harveyAuthdomainGen}/bin/authdomain-gen"
              fi
              ''}
              # The banner goes to stderr: `nix develop -c '<cmd>'` must not
              # mingle hook output with the command's stdout (the plan's
              # lazy-path check compares that stdout with `nix eval`).
              echo "r9 devshell: $(rustc --version)" >&2
              echo "$(qemu-system-aarch64 --version | head -1)" >&2
              echo "apps: nix run .#build | nix run .#boot-virt | nix run .#default" >&2
              echo "harvey supply: R9_HARVEY_DIR_ARM64=$R9_HARVEY_DIR_ARM64 (lazy; realised on first use)" >&2
              echo "harvey supply: R9_HARVEY_DIR_AMD64=$R9_HARVEY_DIR_AMD64 (lazy; realised on first use)" >&2
              echo "harvey supply: R9_HARVEY_REFERENCE_AMD64=$R9_HARVEY_REFERENCE_AMD64 (lazy; realised on first use)" >&2
              echo "harvey supply: R9_HARVEY_REFERENCE_ARM64=$R9_HARVEY_REFERENCE_ARM64 (lazy; realised on first use)" >&2
              ${pkgs.lib.optionalString (harveyAuthdomainGen != null) ''
              echo "harvey supply: R9_AUTHDOMAIN_GEN=$R9_AUTHDOMAIN_GEN" >&2
              ''}
            '';
          };
        });

      apps = forAllSystems (system:
        let
          pkgs = pkgsFor system;
          toolchain = toolchainFor system;
          # 01-21 (D-05/D-07): the arm64 tree is a Linux package; on the
          # Mac it is realised by the registered builder — the same
          # treeSystem rule as the devshell.
          treeSystem =
            if pkgs.stdenv.hostPlatform.isLinux then system else "aarch64-linux";
          harveyTreeArm64 = self.packages.${treeSystem}.harvey-tree-arm64;
          # The moved aarch64 gate apps' runtime inputs (01-21): qemu,
          # dtc (fdtput), expect and git as Harvey's had, plus the Rust
          # toolchain for `cargo xtask image` and the text tools the
          # tier-1 marker computation uses.  No lima.
          bootR9RuntimeInputs = [
            toolchain
            pkgs.bash
            pkgs.coreutils
            pkgs.gawk
            pkgs.gnugrep
            pkgs.gnused
            pkgs.findutils
            pkgs.qemu
            pkgs.dtc
            pkgs.git
            pkgs.expect
          ];
          # The moved x86_64 composed gate apps' runtime inputs (01-23):
          # qemu and expect as Harvey's had, procps for the EXIT trap's
          # survivor sweep and iproute2 for the smoke gate's hostfwd
          # observation, plus the Rust toolchain for `cargo xtask image`
          # and the text tools the computed tier/services marker sets
          # use.  No dtc (nothing in the x64 pipeline touches a DTB);
          # git stays as Harvey's had it (inert — the store tree has no
          # .git).
          bootR9X64RuntimeInputs = [
            toolchain
            pkgs.bash
            pkgs.coreutils
            pkgs.gawk
            pkgs.gnugrep
            pkgs.gnused
            pkgs.findutils
            pkgs.qemu
            pkgs.expect
            pkgs.procps
            pkgs.iproute2
            pkgs.git
          ];
          # 01-24 (D-07/D-09/HARN-08): the moved net gates' two host
          # helpers, built from r9's own committed sources.
          # netDoctorPkg is 16-03's READ-ONLY host-state doctor
          # (diagnose + print, never fix; one implementation consumed
          # two ways — its own app and R9_NET_DOCTOR in the soak/stress
          # headers) and takes no r9 attrs, so its app gate below is its
          # own optionalAttrs.  hostEchoX64Pkg is 16-05's committed
          # host-cc echo peer — a stdenv build of nix/gates/host-echo.c
          # (no fetched binaries, T-01-79), run as the soak/stress
          # gate's own child via HOST_ECHO_X64, never a daemon.
          netDoctorPkg = mkScriptApp pkgs "r9-net-doctor" ./nix/gates/net-doctor.sh ""
            [ pkgs.bash pkgs.qemu pkgs.iproute2 pkgs.iptables pkgs.procps
              pkgs.coreutils pkgs.gnugrep pkgs.gnused ];
          hostEchoX64Pkg = pkgs.stdenv.mkDerivation {
            pname = "host-echo";
            version = "16.05";
            dontUnpack = true;
            buildPhase = ''
              cc -O2 -Wall -o host-echo ${./nix/gates/host-echo.c}
            '';
            installPhase = ''
              install -Dm755 host-echo $out/bin/host-echo
            '';
          };
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
        }
        # 01-21 (D-07/D-09/HARN-08): the moved aarch64 boot, smoke and
        # tier-1 gate apps, on all three systems.  Each header exports
        # r9's own supply: the kernel-virt package, the read-only arm64
        # tree, the host mkpaqfs, and the Rust toolchain on PATH (in the
        # runtimeInputs).  No hasAttr gating — these are r9's own
        # attributes; no lima.
        // {
          boot-r9 = {
            type = "app";
            program = "${mkScriptApp pkgs "boot-r9" ./nix/gates/boot-r9.sh ''
              export R9_KERNEL_PKG="${self.packages.${system}.kernel-virt}"
              export R9_HARVEY_DIR_ARM64="${harveyTreeArm64}"
              export R9_MKPAQFS="${self.packages.${system}.harvey-mkpaqfs}/bin/mkpaqfs"
            '' bootR9RuntimeInputs}/bin/boot-r9";
          };
          boot-r9-smoke = {
            type = "app";
            program = "${mkScriptApp pkgs "boot-r9-smoke" ./nix/gates/smoke-r9.sh ''
              export R9_KERNEL_PKG="${self.packages.${system}.kernel-virt}"
              export R9_HARVEY_DIR_ARM64="${harveyTreeArm64}"
              export R9_MKPAQFS="${self.packages.${system}.harvey-mkpaqfs}/bin/mkpaqfs"
            '' bootR9RuntimeInputs}/bin/boot-r9-smoke";
          };
          boot-r9-tier1 = {
            type = "app";
            program = "${mkScriptApp pkgs "boot-r9-tier1" ./nix/gates/tier1-r9.sh ''
              export R9_KERNEL_PKG="${self.packages.${system}.kernel-virt}"
              export R9_HARVEY_DIR_ARM64="${harveyTreeArm64}"
              export R9_MKPAQFS="${self.packages.${system}.harvey-mkpaqfs}/bin/mkpaqfs"
            '' bootR9RuntimeInputs}/bin/boot-r9-tier1";
          };
          # 01-30 (D-07/D-09/HARN-08): the last two moved aarch64 gate
          # apps, in the 01-21 shape.  boot-r9-net-smoke runs the
          # composed net smoke on the virtshell kernel; boot-r9-cpu runs
          # the cpu fixture gate on the devcap-carrying virtcpu kernel
          # (the same store-path contract: kernel-virt.elf + both DTBs).
          # No hasAttr gating — both packages are r9's own attributes;
          # no lima.
          boot-r9-net-smoke = {
            type = "app";
            program = "${mkScriptApp pkgs "boot-r9-net-smoke" ./nix/gates/smoke-r9-net.sh ''
              export R9_KERNEL_PKG="${self.packages.${system}.kernel-virt}"
              export R9_HARVEY_DIR_ARM64="${harveyTreeArm64}"
              export R9_MKPAQFS="${self.packages.${system}.harvey-mkpaqfs}/bin/mkpaqfs"
            '' bootR9RuntimeInputs}/bin/boot-r9-net-smoke";
          };
          boot-r9-cpu = {
            type = "app";
            program = "${mkScriptApp pkgs "boot-r9-cpu" ./nix/gates/cpu-r9.sh ''
              export R9_KERNEL_PKG="${self.packages.${system}.kernel-virt-cpu}"
              export R9_HARVEY_DIR_ARM64="${harveyTreeArm64}"
              export R9_MKPAQFS="${self.packages.${system}.harvey-mkpaqfs}/bin/mkpaqfs"
            '' bootR9RuntimeInputs}/bin/boot-r9-cpu";
          };
        }
        # 01-18 (D-07/D-09/HARN-08): the moved drawterm gate app.  The
        # header is Harvey's drawtermGateX64Pkg env contract re-keyed
        # onto r9's supply: the kernel package, r9's iso-builder and
        # netdev emitter, the read-only amd64 tree, the host mkpaqfs and
        # authdomain generator (the image bake), and Harvey's drawterm
        # package as the gate binary (D-09).  The runtimeInputs add the
        # rust toolchain for `cargo xtask image`, the X11 feed tools and
        # the host observation tools.  x86_64-linux only: the BIOS ISO
        # machinery is the q35 channel's venue (X64-05).
        // nixpkgs.lib.optionalAttrs (system == "x86_64-linux") {
          drawterm-gate-x64 = {
            type = "app";
            program = "${mkScriptApp pkgs "drawterm-gate-x64" ./nix/gates/drawterm-gate-x64.sh ''
              export R9_KERNEL_X64_PKG="${self.packages.${system}.kernel-x64}"
              export R9_ISO_BUILDER="${self.packages.${system}.iso-builder}/bin/r9-iso-builder"
              export R9_NETDEV_EMITTER="${self.packages.${system}.netdev-args}/bin/r9-netdev-args"
              export R9_HARVEY_DIR_AMD64="${self.packages.${system}.harvey-tree-amd64}"
              export R9_MKPAQFS="${self.packages.${system}.harvey-mkpaqfs}/bin/mkpaqfs"
              export R9_AUTHDOMAIN_GEN="${self.packages.${system}.harvey-authdomain-gen}/bin/authdomain-gen"
              export DRAWTERM_BIN="${harvey.packages.${system}.drawterm}/bin/drawterm"
            '' [
              toolchain
              pkgs.bash
              pkgs.coreutils
              pkgs.qemu
              pkgs.expect
              pkgs.xorgserver
              pkgs.xdotool
              pkgs.procps
              pkgs.iproute2
              pkgs.grub2
              pkgs.xorriso
            ]}/bin/drawterm-gate-x64";
          };
        }
        # 01-23 (D-07/D-09/HARN-08): the moved x86_64 composed gate apps
        # — smoke (the chain leg + the cpu-chain leg), tier-1, churn and
        # services.  Each header exports r9's own supply: the kernel-x64
        # package, r9's iso-builder and (where Harvey's did) the netdev
        # emitter, the read-only amd64 tree, the host mkpaqfs and
        # authdomain generator (the image bake).  The runtimeInputs add
        # the Rust toolchain for `cargo xtask image` and the host tools
        # the drivers use.  x86_64-linux only: the BIOS ISO machinery
        # (iso-builder) is the q35 channel's venue (X64-05).
        // nixpkgs.lib.optionalAttrs (system == "x86_64-linux") {
          boot-r9-smoke-x64 = {
            type = "app";
            program = "${mkScriptApp pkgs "boot-r9-smoke-x64" ./nix/gates/smoke-r9-x64.sh ''
              export R9_KERNEL_X64_PKG="${self.packages.${system}.kernel-x64}"
              export R9_ISO_BUILDER="${self.packages.${system}.iso-builder}/bin/r9-iso-builder"
              export R9_NETDEV_EMITTER="${self.packages.${system}.netdev-args}/bin/r9-netdev-args"
              export R9_HARVEY_DIR_AMD64="${self.packages.${system}.harvey-tree-amd64}"
              export R9_MKPAQFS="${self.packages.${system}.harvey-mkpaqfs}/bin/mkpaqfs"
              export R9_AUTHDOMAIN_GEN="${self.packages.${system}.harvey-authdomain-gen}/bin/authdomain-gen"
            '' bootR9X64RuntimeInputs}/bin/boot-r9-smoke-x64";
          };
          boot-r9-tier1-x64 = {
            type = "app";
            program = "${mkScriptApp pkgs "boot-r9-tier1-x64" ./nix/gates/tier1-r9-x64.sh ''
              export R9_KERNEL_X64_PKG="${self.packages.${system}.kernel-x64}"
              export R9_ISO_BUILDER="${self.packages.${system}.iso-builder}/bin/r9-iso-builder"
              export R9_HARVEY_DIR_AMD64="${self.packages.${system}.harvey-tree-amd64}"
              export R9_MKPAQFS="${self.packages.${system}.harvey-mkpaqfs}/bin/mkpaqfs"
              export R9_AUTHDOMAIN_GEN="${self.packages.${system}.harvey-authdomain-gen}/bin/authdomain-gen"
            '' bootR9X64RuntimeInputs}/bin/boot-r9-tier1-x64";
          };
          boot-r9-churn-x64 = {
            type = "app";
            program = "${mkScriptApp pkgs "boot-r9-churn-x64" ./nix/gates/churn-r9-x64.sh ''
              export R9_KERNEL_X64_PKG="${self.packages.${system}.kernel-x64}"
              export R9_ISO_BUILDER="${self.packages.${system}.iso-builder}/bin/r9-iso-builder"
              export R9_HARVEY_DIR_AMD64="${self.packages.${system}.harvey-tree-amd64}"
              export R9_MKPAQFS="${self.packages.${system}.harvey-mkpaqfs}/bin/mkpaqfs"
              export R9_AUTHDOMAIN_GEN="${self.packages.${system}.harvey-authdomain-gen}/bin/authdomain-gen"
            '' bootR9X64RuntimeInputs}/bin/boot-r9-churn-x64";
          };
          boot-r9-services-x64 = {
            type = "app";
            program = "${mkScriptApp pkgs "boot-r9-services-x64" ./nix/gates/services-r9-x64.sh ''
              export R9_KERNEL_X64_PKG="${self.packages.${system}.kernel-x64}"
              export R9_ISO_BUILDER="${self.packages.${system}.iso-builder}/bin/r9-iso-builder"
              export R9_HARVEY_DIR_AMD64="${self.packages.${system}.harvey-tree-amd64}"
              export R9_MKPAQFS="${self.packages.${system}.harvey-mkpaqfs}/bin/mkpaqfs"
              export R9_AUTHDOMAIN_GEN="${self.packages.${system}.harvey-authdomain-gen}/bin/authdomain-gen"
            '' bootR9X64RuntimeInputs}/bin/boot-r9-services-x64";
          };
        }
        # 01-24 (D-07/D-09/HARN-08): the net-doctor preflight app —
        # read-only host-state checks (qemu on PATH, ip/ss link+addr+port
        # state, sysctl, the netfilter read, the setuid helper, the ACL
        # file).  It consumes NO r9 attrs, so its own isLinux gate keeps
        # evaluation lazy everywhere the checks are meaningless (the
        # vmnet leg is HNET-04 documented-manual) — the 16-03 shape.
        // nixpkgs.lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
          net-doctor = {
            type = "app";
            program = "${netDoctorPkg}/bin/r9-net-doctor";
          };
        }
        # 01-24 (D-07/D-09/HARN-08): the moved x86_64 NET gate apps.
        # Each header exports r9's own supply: the kernel-x64 package,
        # r9's iso-builder and netdev emitter, the read-only amd64 tree
        # and the host mkpaqfs; the soak and stress headers add
        # R9_NET_DOCTOR (the preflight's ONE implementation),
        # HOST_ECHO_X64 (hostEchoX64Pkg's echo peer) and
        # HARVEY_NETADDR_SEAM (the seam file INSIDE the read-only tree —
        # D-19, never a path in Harvey's build source).  The
        # runtimeInputs are bootR9X64RuntimeInputs (the Rust toolchain
        # for `cargo xtask image`, qemu, expect, procps, iproute2 and
        # the text tools).  x86_64-linux only: the BIOS ISO machinery
        # (iso-builder) is the q35 channel's venue (X64-05) and the tap
        # venue is Linux's.
        // nixpkgs.lib.optionalAttrs (system == "x86_64-linux") {
          boot-r9-net-smoke-x64 = {
            type = "app";
            program = "${mkScriptApp pkgs "boot-r9-net-smoke-x64" ./nix/gates/smoke-r9-net-x64.sh ''
              export R9_KERNEL_X64_PKG="${self.packages.${system}.kernel-x64}"
              export R9_ISO_BUILDER="${self.packages.${system}.iso-builder}/bin/r9-iso-builder"
              export R9_NETDEV_EMITTER="${self.packages.${system}.netdev-args}/bin/r9-netdev-args"
              export R9_HARVEY_DIR_AMD64="${self.packages.${system}.harvey-tree-amd64}"
              export R9_MKPAQFS="${self.packages.${system}.harvey-mkpaqfs}/bin/mkpaqfs"
            '' bootR9X64RuntimeInputs}/bin/boot-r9-net-smoke-x64";
          };
          boot-r9-net-soak-x64 = {
            type = "app";
            program = "${mkScriptApp pkgs "boot-r9-net-soak-x64" ./nix/gates/soak-r9-net-x64.sh ''
              export R9_KERNEL_X64_PKG="${self.packages.${system}.kernel-x64}"
              export R9_ISO_BUILDER="${self.packages.${system}.iso-builder}/bin/r9-iso-builder"
              export R9_NETDEV_EMITTER="${self.packages.${system}.netdev-args}/bin/r9-netdev-args"
              export R9_HARVEY_DIR_AMD64="${self.packages.${system}.harvey-tree-amd64}"
              export R9_MKPAQFS="${self.packages.${system}.harvey-mkpaqfs}/bin/mkpaqfs"
              export R9_NET_DOCTOR="${netDoctorPkg}/bin/r9-net-doctor"
              export HOST_ECHO_X64="${hostEchoX64Pkg}/bin/host-echo"
              export HARVEY_NETADDR_SEAM="${self.packages.${system}.harvey-tree-amd64}/build/x86_64/root-tree/lib/netaddr.rc"
            '' bootR9X64RuntimeInputs}/bin/boot-r9-net-soak-x64";
          };
          boot-r9-stress-x64 = {
            type = "app";
            program = "${mkScriptApp pkgs "boot-r9-stress-x64" ./nix/gates/stress-r9-net-x64.sh ''
              export R9_KERNEL_X64_PKG="${self.packages.${system}.kernel-x64}"
              export R9_ISO_BUILDER="${self.packages.${system}.iso-builder}/bin/r9-iso-builder"
              export R9_NETDEV_EMITTER="${self.packages.${system}.netdev-args}/bin/r9-netdev-args"
              export R9_HARVEY_DIR_AMD64="${self.packages.${system}.harvey-tree-amd64}"
              export R9_MKPAQFS="${self.packages.${system}.harvey-mkpaqfs}/bin/mkpaqfs"
              export R9_NET_DOCTOR="${netDoctorPkg}/bin/r9-net-doctor"
              export HOST_ECHO_X64="${hostEchoX64Pkg}/bin/host-echo"
              export HARVEY_NETADDR_SEAM="${self.packages.${system}.harvey-tree-amd64}/build/x86_64/root-tree/lib/netaddr.rc"
            '' bootR9X64RuntimeInputs}/bin/boot-r9-stress-x64";
          };
        }
        # 01-31 (D-07/D-09/HARN-08): the last two moved x86_64 composed
        # apps — the plain boot (its tap mode gates on the 01-24
        # net-doctor, hence R9_NET_DOCTOR) and the auth smoke.  Each
        # header exports r9's own supply: the kernel-x64 package, r9's
        # iso-builder and netdev emitter, the read-only amd64 tree, the
        # host mkpaqfs and authdomain generator (the per-bake p9sk1
        # domain the cpu/auth images carry).  bootR9X64RuntimeInputs
        # carries the Rust toolchain (`cargo xtask image`), qemu,
        # expect, procps (the auth gate's survivor sweep), iproute2
        # (the auth driver's hostfwd observation) and the text tools;
        # Harvey's plan9port and namespace-transport tools are no
        # longer consumed (there is no in-checkout 9 build left) and
        # stdenv.cc is not carried (the authdomain generator is r9's
        # prebuilt store package).  x86_64-linux only: the BIOS ISO
        # machinery (iso-builder) is the q35 channel's venue (X64-05).
        // nixpkgs.lib.optionalAttrs (system == "x86_64-linux") {
          boot-r9-x64 = {
            type = "app";
            program = "${mkScriptApp pkgs "boot-r9-x64" ./nix/gates/boot-r9-x64.sh ''
              export R9_KERNEL_X64_PKG="${self.packages.${system}.kernel-x64}"
              export R9_ISO_BUILDER="${self.packages.${system}.iso-builder}/bin/r9-iso-builder"
              export R9_NETDEV_EMITTER="${self.packages.${system}.netdev-args}/bin/r9-netdev-args"
              export R9_NET_DOCTOR="${netDoctorPkg}/bin/r9-net-doctor"
              export R9_HARVEY_DIR_AMD64="${self.packages.${system}.harvey-tree-amd64}"
              export R9_MKPAQFS="${self.packages.${system}.harvey-mkpaqfs}/bin/mkpaqfs"
              export R9_AUTHDOMAIN_GEN="${self.packages.${system}.harvey-authdomain-gen}/bin/authdomain-gen"
            '' bootR9X64RuntimeInputs}/bin/boot-r9-x64";
          };
          boot-r9-auth-x64 = {
            type = "app";
            program = "${mkScriptApp pkgs "boot-r9-auth-x64" ./nix/gates/smoke-r9-auth-x64.sh ''
              export R9_KERNEL_X64_PKG="${self.packages.${system}.kernel-x64}"
              export R9_ISO_BUILDER="${self.packages.${system}.iso-builder}/bin/r9-iso-builder"
              export R9_NETDEV_EMITTER="${self.packages.${system}.netdev-args}/bin/r9-netdev-args"
              export R9_HARVEY_DIR_AMD64="${self.packages.${system}.harvey-tree-amd64}"
              export R9_MKPAQFS="${self.packages.${system}.harvey-mkpaqfs}/bin/mkpaqfs"
              export R9_AUTHDOMAIN_GEN="${self.packages.${system}.harvey-authdomain-gen}/bin/authdomain-gen"
            '' bootR9X64RuntimeInputs}/bin/boot-r9-auth-x64";
          };
        }
        # 01-31 (D-20/D-08/HARN-08): r9's own drawterm developer app —
        # the moved wrapper (Harvey 19-06's) rewired onto r9's cpu
        # image: DRAWTERM_BIN is Harvey's drawterm package (D-09), the
        # connection is the committed hostfwd pair's loopback side
        # (cpu=127.0.0.1, auth=tcp!127.0.0.1!5670, user=glenda), and
        # the creds FILE is r9's target/images/x86_64/cpu.creds (read
        # by path only — T-01-97).  Linux-only beside the package
        # itself (the X11 backend is the package's Linux build); it
        # consumes no other r9 attr, so it keeps its own isLinux gate
        # (the net-doctor shape) rather than riding the x64 block.
        // nixpkgs.lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
          drawterm = {
            type = "app";
            program = "${mkScriptApp pkgs "r9-drawterm" ./nix/gates/drawterm-x64.sh ''
              export DRAWTERM_BIN="${harvey.packages.${system}.drawterm}/bin/drawterm"
            '' [ pkgs.bash ]}/bin/r9-drawterm";
          };
        });
    };
}
