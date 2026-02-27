{pkgs, ...}: {
  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = true;

  documentation.nixos.enable = false;

  virtualisation.vmVariant = {
    virtualisation.memorySize = 2048;
  };

  nix = {
    settings = {
      trusted-users = [
        "root"
        "termsurf"
      ];
    };
    extraOptions = ''
      experimental-features = nix-command flakes
    '';
  };

  users.mutableUsers = false;

  users.groups.termsurf = {};

  users.users.termsurf = {
    isNormalUser = true;
    description = "TermSurf";
    group = "termsurf";
    extraGroups = ["wheel"];
    hashedPassword = "";
  };

  environment.systemPackages = [
    pkgs.kitty
    pkgs.fish
    pkgs.termsurf
    pkgs.helix
    pkgs.neovim
    pkgs.xterm
    pkgs.zsh
  ];

  security.polkit = {
    enable = true;
  };

  services.dbus = {
    enable = true;
  };

  services.displayManager = {
    autoLogin = {
      enable = true;
      user = "termsurf";
    };
  };

  services.libinput = {
    enable = true;
  };

  services.qemuGuest = {
    enable = true;
  };

  services.spice-vdagentd = {
    enable = true;
  };

  services.xserver = {
    enable = true;
  };
}
