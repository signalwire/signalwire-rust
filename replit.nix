{ pkgs }: {
  deps = [ pkgs.rustc pkgs.cargo pkgs.pkg-config pkgs.openssl ];
}
