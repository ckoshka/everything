import $ from "https://deno.land/x/dax@0.9.0/mod.ts";

$.logLight(`Cloning directory...`);
await $`git clone --depth 1 https://github.com/ckoshka/everything`;
const rm = (folder: string) => $`rm -rf everything/${folder}`;
await Promise.all(
  [`ai_stuff`, `archived`, `experimental`, `fun`, `useful`, `wasm_libs`].map(
    rm,
  ),
);
$.logLight(`Done cloning and cleaning directory...`);

$.logLight(`Building targets...`);
await $`cd everything`;
await $`cargo build --release --workspace`;
await $`cd ..`;
$.logLight(`Done building targets...`);

await $`mkdir binaries`;
await $`mv everything/target/release/* binaries`;
await $`rm -rf everything`;
$.logLight(`Done!`);