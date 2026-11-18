// Build the Hook receiver for the current target and stage it for Tauri's
// `bundle.externalBin`. Tauri expects the target triple in the file name and
// places the binary next to the installed application executable.

import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const root = path.resolve(import.meta.dirname, "..");

function hostTriple() {
  const verbose = execFileSync("rustc", ["-vV"], { cwd: root, encoding: "utf8" });
  const line = verbose.split(/\r?\n/).find((entry) => entry.startsWith("host: "));
  if (!line) throw new Error("rustc -vV did not report a host triple");
  return line.slice("host: ".length).trim();
}

const requestedTarget = process.argv[2] ?? process.env.TAURI_ENV_TARGET_TRIPLE;
const cargoTarget = requestedTarget ?? process.env.CARGO_BUILD_TARGET;
const target = cargoTarget ?? hostTriple();
const metadata = JSON.parse(
  execFileSync("cargo", ["metadata", "--locked", "--no-deps", "--format-version", "1"], {
    cwd: root,
    encoding: "utf8",
  }),
);
const desktop = metadata.packages.find((pkg) => pkg.name === "xushu-desktop");
const hook = metadata.packages.find((pkg) => pkg.name === "xushu-hook");
if (!desktop || !hook || desktop.version !== hook.version) {
  throw new Error("desktop and Hook receiver Cargo package versions must agree");
}

const staticMsvcRuntime = target.endsWith("-windows-msvc");
const args = [staticMsvcRuntime ? "rustc" : "build", "--locked", "--release", "-p", "xushu-hook"];
if (cargoTarget) args.push("--target", cargoTarget);
if (staticMsvcRuntime) args.push("--", "-C", "target-feature=+crt-static");

execFileSync("cargo", args, { cwd: root, stdio: "inherit" });

const extension = target.includes("-windows-") ? ".exe" : "";
const profileDirectory = cargoTarget
  ? path.join(metadata.target_directory, target, "release")
  : path.join(metadata.target_directory, "release");
const source = path.join(profileDirectory, `xushu-hook${extension}`);
if (!fs.existsSync(source)) throw new Error(`built sidecar not found: ${source}`);

const probe = execFileSync(source, ["--version"], {
  encoding: "utf8",
  timeout: 10_000,
  maxBuffer: 4096,
}).trim();
if (probe !== `xushu-hook ${desktop.version}`) {
  throw new Error(`built sidecar version mismatch: expected xushu-hook ${desktop.version}`);
}

const destinationDirectory = path.join(root, "apps", "desktop", "src-tauri", "binaries");
const destination = path.join(destinationDirectory, `xushu-hook-${target}${extension}`);
fs.mkdirSync(destinationDirectory, { recursive: true });
fs.copyFileSync(source, destination);

console.log(`sidecar: ${source} -> ${destination} (${probe})`);
