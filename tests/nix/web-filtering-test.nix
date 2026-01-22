{ lib
, stdenv
, fetchFromGitHub
, nodejs_20
, playwright
, makeWrapper
, dotnet-runtime
}:

stdenv.mkDerivation rec {
  pname = "dots-family-web-filtering-test";
  version = "1.0.0";

  src = ../scripts/web-filtering-test;

  nativeBuildInputs = [
    nodejs_20
    makeWrapper
  ];

  buildInputs = [
    playwright
  ];

  dontBuild = true;
  dontConfigure = true;

  installPhase = ''
    mkdir -p $out/bin

    # Copy test files
    cp -r web-filtering-test.js package.json package-lock.json $out/
    cp run-web-filtering-test.sh $out/bin/run-web-filtering-test

    # Make script executable
    chmod +x $out/bin/run-web-filtering-test

    # Wrap the script to include Node.js and Playwright in PATH
    wrapProgram $out/bin/run-web-filtering-test \
      --prefix PATH : ${lib.makeBinPath [ nodejs_20 playwright ]}

    # Install Playwright browsers
    export PLAYWRIGHT_BROWSERS_PATH=$out/share/playwright-browsers
    mkdir -p $PLAYWRIGHT_BROWSERS_PATH
    ${playwright}/bin/playwright install chromium 2>/dev/null || true

    # Create evidence directory
    mkdir -p $out/share/evidence
  '';

  meta = with lib; {
    description = "DOTS Family Mode Web Filtering Test Suite";
    homepage = "https://github.com/dots-distro/dots-family-mode";
    license = licenses.mit;
    platforms = platforms.linux;
    maintainers = with maintainers; [ dots-distro ];
  };
}
