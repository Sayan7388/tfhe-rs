import { runTestAttachedToButton } from "./common.mjs";

it("Compressed Compact Public Key Test Small 256 Bit", async () => {
  await runTestAttachedToButton("compressedCompactPublicKeyTest256BitSmall");
});

it("Compressed Compact Public Key Test Big 256 Bit", async () => {
  await runTestAttachedToButton("compressedCompactPublicKeyTest256BitBig");
});

it(
  "Compact Public Key Test Big 64 Bit With Zero Knowledge",
  async () => {
    await runTestAttachedToButton("compactPublicKeyZeroKnowledge");
  },
  1200 * 1000,
); // 20 minutes timeout
