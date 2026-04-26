{
  description = "The unofficial MyAsus alternative for Linux";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    flake-utils.lib.eachSystem [ "x86_64-linux" "aarch64-linux" ] (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        packages.ayuz = pkgs.rustPlatform.buildRustPackage {
          pname = "ayuz";
          version = "1.0.8-unstable-${pkgs.lib.substring 0 8 (self.lastModifiedDate or "00000000")}";

          src = pkgs.lib.cleanSource ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = with pkgs; [
            pkg-config
            wrapGAppsHook4
            gettext
          ];

          buildInputs = with pkgs; [
            gtk4
            libadwaita
            dbus
          ];

          preFixup = ''
            gappsWrapperArgs+=(
              --prefix PATH : ${
                pkgs.lib.makeBinPath (
                  with pkgs;
                  [
                    asusctl
                    kdePackages.qttools
                    kdePackages.libkscreen
                    kdePackages.kconfig
                    swayidle
                    wireplumber
                    easyeffects
                    procps
                    coreutils
                    which
                    xdg-utils
                    brightnessctl
                    playerctl
                    pulseaudio
                    iio-sensor-proxy
                    glib
                  ]
                )
              }
            )
          '';

          postInstall = ''
            install -Dm644 packaging/de.guido.ayuz.desktop -t $out/share/applications/
            install -Dm644 packaging/de.guido.ayuz.metainfo.xml -t $out/share/metainfo/
            install -Dm644 assets/trayicon.png $out/share/icons/hicolor/128x128/apps/de.guido.ayuz.png
          '';

          meta = with pkgs.lib; {
            description = "The unofficial MyAsus alternative for Linux";
            homepage = "https://github.com/Traciges/Ayuz";
            license = licenses.gpl3Plus;
            platforms = platforms.linux;
            mainProgram = "ayuz";
          };
        };

        packages.default = self.packages.${system}.ayuz;

        devShells.default = pkgs.mkShell {
          inputsFrom = [ self.packages.${system}.ayuz ];
          nativeBuildInputs = with pkgs; [
            rustc
            cargo
            rust-analyzer
            clippy
            rustfmt
          ];

          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (
            with pkgs;
            [
              gtk4
              libadwaita
              dbus
            ]
          );
        };
      }
    )
    // {
      nixosModules.default =
        {
          config,
          lib,
          pkgs,
          ...
        }:
        let
          cfg = config.services.ayuz;
          ayuz-pkg = self.packages.${pkgs.system}.default;
        in
        {
          options.services.ayuz = {
            enable = lib.mkEnableOption "Ayuz - The unofficial MyAsus alternative for Linux";
            supportMyAsusKey = lib.mkOption {
              type = lib.types.bool;
              default = true;
              description = "Whether to rebind the MyAsus/ROG key to launch Ayuz.";
            };
            fnKeyMode = lib.mkOption {
              type = lib.types.nullOr (lib.types.enum [ "function" "shortcut" ]);
              default = "shortcut";
              description = "Priority for the top row of keys: 'function' for F1-F12, 'shortcut' for media keys (volume, brightness, etc.). Sets asus_wmi.fnlock_default.";
            };
          };

          config = lib.mkIf cfg.enable {
            environment.systemPackages = [ ayuz-pkg ];
            boot.kernelParams = lib.optional (cfg.fnKeyMode != null) "asus_wmi.fnlock_default=${if cfg.fnKeyMode == "function" then "1" else "0"}";

            services.asusd.enable = lib.mkDefault true;
            # Force asusd to start on boot
            systemd.services.asusd.wantedBy = [ "multi-user.target" ];
            services.supergfxd.enable = lib.mkDefault true;
            hardware.sensor.iio.enable = lib.mkDefault true;
            security.polkit.enable = lib.mkDefault true;
            services.playerctld.enable = lib.mkDefault true;

            services.triggerhappy = lib.mkIf cfg.supportMyAsusKey {
              enable = true;
              user = "root";
              bindings = [
                {
                  keys = [ "PROG1" ];
                  event = "press";
                  cmd = "${pkgs.bash}/bin/bash -c '${pkgs.systemd}/bin/systemd-run --user --machine=$(${pkgs.systemd}/bin/loginctl list-users --no-legend | ${pkgs.coreutils}/bin/head -n1 | ${pkgs.gawk}/bin/awk \"{print \\$2}\")@.host ${ayuz-pkg}/bin/ayuz'";
                }
              ];
            };

            services.udev.extraRules = ''
              ACTION=="add|change", KERNEL=="event*", ATTRS{name}=="*Touchpad*", MODE="0660", GROUP="input"
            '';
          };
        };

      homeManagerModules.default =
        {
          config,
          lib,
          pkgs,
          ...
        }:
        let
          cfg = config.programs.ayuz;
          ayuz-pkg = self.packages.${pkgs.system}.default;
        in
        {
          options.programs.ayuz = {
            enable = lib.mkEnableOption "Ayuz";
            autostart = lib.mkOption {
              type = lib.types.bool;
              default = false;
              description = "Start Ayuz minimized on login.";
            };
            settings = lib.mkOption {
              type =
                with lib.types;
                nullOr (
                  either str (submodule {
                    freeformType = (pkgs.formats.json { }).type;
                  })
                );
              default = null;
              description = ''
                Configuration written to <filename>~/.config/ayuz/config.json</filename>.
                Accepts either a Nix attribute set or a raw JSON string.
                See the application for available options.
                If set to <literal>null</literal> (default), the configuration is not managed by Nix.
              '';
            };
          };

          config = lib.mkIf cfg.enable {
            home.packages = [ ayuz-pkg ];

            xdg.configFile."ayuz/config.json" = lib.mkIf (cfg.settings != null) (
              if lib.isString cfg.settings then
                {
                  text = cfg.settings;
                }
              else
                {
                  source = (pkgs.formats.json { }).generate "ayuz-config" cfg.settings;
                }
            );

            systemd.user.services.ayuz = lib.mkIf cfg.autostart {
              Unit = {
                Description = "Ayuz - The unofficial MyAsus alternative for Linux";
                After = [ "graphical-session.target" ];
                PartOf = [ "graphical-session.target" ];
              };
              Service = {
                ExecStart = "${ayuz-pkg}/bin/ayuz --hidden";
                Restart = "always";
              };
              Install.WantedBy = [ "graphical-session.target" ];
            };
          };
        };
    };
}
