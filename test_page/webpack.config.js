const path = require('path');
const CopyPlugin = require("copy-webpack-plugin");
const WasmPackPlugin = require("@wasm-tool/wasm-pack-plugin");

const crate_dir = path.resolve(__dirname, "../client_test");
const dist = path.resolve(__dirname, "dist");

module.exports = {
    entry: './src/index.js',
    output: {
        path: dist,
        filename: "index.js",
    },
    devServer: {
        contentBase: dist,
        port: 9000,
    },
    plugins: [
        new CopyPlugin({
            patterns: [
                path.resolve(__dirname, "static")
            ],
        }),
        new WasmPackPlugin({
            crateDirectory: crate_dir,
            outDir: "pkg",
            forceMode: "development",
        }),
    ],
    experiments: {
        syncWebAssembly: true,
    },
    devtool: 'eval-cheap-source-map',
};
