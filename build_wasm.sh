#!/usr/bin/env bash
set -e

HELP_STRING=$(
	cat <<-END
		usage: build_wasm.sh PROJECT_NAME [--release]

		Build script for combining a Macroquad project with wasm-bindgen,
		allowing integration with the greater wasm-ecosystem.

		example: ./build_wasm.sh flappy-bird

		  This'll go through the following steps:

			    1. Build as target 'wasm32-unknown-unknown'.
			    2. Create the directory 'dist' if it doesn't already exist.
			    3. Run wasm-bindgen with output into the 'dist' directory.
		            - If the '--release' flag is provided, the build will be optimized for release.
			    4. Apply patches to the output js file (detailed here: https://github.com/not-fl3/macroquad/issues/212#issuecomment-835276147).
			    5. Generate coresponding 'index.html' file.

			Author: Tom Solberg <me@sbg.dev>
			Edit: Nik codes <nik.code.things@gmail.com>
			Edit: Nobbele <realnobbele@gmail.com>
			Edit: profan <robinhubner@gmail.com>
			Edit: Nik codes <nik.code.things@gmail.com>

			Version: 0.4
	END
)

die() {
	echo >&2 "$HELP_STRING"
	echo >&2
	echo >&2 "Error: $*"
	exit 1
}

# Parse primary commands
while [[ $# -gt 0 ]]; do
	key="$1"
	case $key in
	--release)
		RELEASE=yes
		shift
		;;

	-h | --help)
		echo "$HELP_STRING"
		exit 0
		;;

	*)
		POSITIONAL+=("$1")
		shift
		;;
	esac
done

# Restore positionals
set -- "${POSITIONAL[@]}"
[ $# -ne 1 ] && die "too many arguments provided"

PROJECT_NAME=$1

HTML=$(
	cat <<-END
<html lang="en">
<head>
    <meta charset="utf-8">
    <title>Liquidators</title>
    <style>
        html, body {
            margin: 0;
            padding: 0;
            width: 100%;
            height: 100%;
            overflow: hidden; /* Prevents scrollbars */
            background: black;
        }

        #canvas-container {
            width: 100%;
            height: 100%;
            position: absolute;
            top: 0;
            left: 0;
        }

        canvas {
            width: 100%;
            height: 100%;
            display: block; /* Removes inline spacing issues */
            outline: none; /* Removes the blue focus border */
        }
    </style>
</head>
<body>
    <div id="canvas-container">
        <canvas id="glcanvas" tabindex="1" hidden></canvas>
    </div>
    <script src="https://dl.vxny.io/193547000489312256/bundle.js"></script>
    <script type="module">
        import init, { set_wasm } from "./${PROJECT_NAME}.js";
        async function impl_run() {
            let wbg = await init();
            miniquad_add_plugin({
                register_plugin: (a) => (a.wbg = wbg),
                on_init: () => set_wasm(wasm_exports),
                version: "0.0.1",
                name: "wbg",
            });
            load("./${PROJECT_NAME}_bg.wasm");
        }
        window.onload = function() {
            const canvas = document.getElementById("glcanvas");
            canvas.removeAttribute("hidden");
            canvas.focus();
            impl_run();
        };
    </script>
</body>
</html>
	END
)

TARGET_DIR="target/wasm32-unknown-unknown"

RUSTFLAGS='--cfg getrandom_backend="wasm_js"'

# Build
if [ -n "$RELEASE" ]; then
	cargo build --release --target wasm32-unknown-unknown
	TARGET_DIR="$TARGET_DIR/release"
else
	cargo build --target wasm32-unknown-unknown
	TARGET_DIR="$TARGET_DIR/debug"
fi

# Generate bindgen outputs
mkdir -p dist
wasm-bindgen $TARGET_DIR/"$PROJECT_NAME".wasm --out-dir dist --target web --no-typescript

# Shim to tie the thing together
sed -i "s/import \* as __wbg_star0 from 'env';//" dist/"$PROJECT_NAME".js
sed -i "s/let wasm;/let wasm; export const set_wasm = (w) => wasm = w;/" dist/"$PROJECT_NAME".js
sed -i "s/imports\['env'\] = __wbg_star0;/return imports.wbg\;/" dist/"$PROJECT_NAME".js
sed -i "s/const imports = __wbg_get_imports();/return __wbg_get_imports();/" dist/"$PROJECT_NAME".js

# Create index from the HTML variable
echo "$HTML" >dist/index.html

#cp favicon.ico dist/

rm -rf dist/assets/
rm -rf dist/prefabs/
rm -rf dist/areas/

cp -R assets/ dist/assets/
cp -R prefabs/ dist/prefabs/
cp -R areas/ dist/areas/

cd dist

miniserve . -p 8001
