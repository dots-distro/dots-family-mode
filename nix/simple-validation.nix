# Simple NixOS Testing Configuration

{ pkgs, lib, ... }:

{
  users.users = {
    test = {
      isNormalUser = true;
      description = "Test User";
      initialPassword = "test123";
    };
  };

  environment.systemPackages = with pkgs; [
    (pkgs.writeScriptBin "test-basic" "echo Success")
  ];

  system.stateVersion = "25.05";
  networking.hostName = "dots-simple-test";
}
